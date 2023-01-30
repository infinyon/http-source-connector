use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Response;
use serde::Serialize;

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::config::{OutputParts, OutputType};

#[async_trait]
pub(crate) trait Formatter {
    async fn to_string(&self, response: Response) -> anyhow::Result<String>;
}

pub(crate) fn formatter(
    output_type: OutputType,
    output_parts: OutputParts,
) -> Arc<dyn Formatter + Sync + Send> {
    match output_type {
        OutputType::Text => Arc::new(TextFormatter(output_parts)),
        OutputType::Json => Arc::new(JsonFormatter(output_parts)),
    }
}

struct JsonFormatter(OutputParts);

struct TextFormatter(OutputParts);

#[async_trait]
impl Formatter for JsonFormatter {
    async fn to_string(&self, response: Response) -> anyhow::Result<String> {
        let record = record_from_response(response).await?;
        let json_record = match self.0 {
            OutputParts::Body => HttpJsonRecord::from(HttpResponseRecord {
                body: record.body,
                ..Default::default()
            }),
            OutputParts::Full => HttpJsonRecord::from(record),
        };

        Ok(serde_json::to_string(&json_record)?)
    }
}

#[async_trait]
impl Formatter for TextFormatter {
    async fn to_string(&self, response: Response) -> anyhow::Result<String> {
        let HttpResponseRecord {
            version,
            status_code,
            status_string,
            headers,
            body,
        } = record_from_response(response).await?;

        let mut record_out_parts: Vec<String> = Vec::new();
        if let OutputParts::Full = self.0 {
            // Status Line HTTP/X 200 CANONICAL
            let status_line: Vec<String> = vec![
                version.unwrap_or_default(),
                status_code.unwrap_or_default().to_string(),
                status_string.unwrap_or_default().to_string(),
            ];
            record_out_parts.push(status_line.join(" "));

            // Header lines foo: bar
            if let Some(headers) = headers {
                let hdr_out_parts: Vec<String> = headers
                    .into_iter()
                    .map(|hdr| format!("{}: {}", hdr.name, hdr.value))
                    .collect();

                record_out_parts.push(hdr_out_parts.join("\n"));
            }

            // Body with an empty line between
            if body.is_some() {
                record_out_parts.push(String::from(""));
            }
        };
        // Body with an empty line between
        if let Some(body) = body {
            record_out_parts.push(body);
        }

        Ok(record_out_parts.join("\n"))
    }
}

#[derive(Debug, Default)]
struct HttpResponseRecord {
    version: Option<String>,
    status_code: Option<u16>,
    status_string: Option<&'static str>,
    headers: Option<Vec<HttpHeader>>,
    body: Option<String>,
}

#[derive(Debug)]
struct HttpHeader {
    name: String,
    value: String,
}

impl TryFrom<&Response> for HttpResponseRecord {
    type Error = Error;

    fn try_from(response: &Response) -> Result<Self> {
        let status_code = Some(response.status().as_u16());
        let status_string = response.status().canonical_reason();
        let version = Some(format!("{:?}", response.version()));
        let headers = Some(
            response
                .headers()
                .iter()
                .map(|(key, value)| {
                    value.to_str().map(|value| HttpHeader {
                        name: key.to_string(),
                        value: value.to_string(),
                    })
                })
                .collect::<Result<Vec<HttpHeader>, _>>()?,
        );

        Ok(Self {
            version,
            status_code,
            status_string,
            headers,
            body: None,
        })
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HttpJsonStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    string: Option<&'static str>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HttpJsonRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<HttpJsonStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    header: Option<BTreeMap<String, JsonHeadersValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(untagged)]
enum JsonHeadersValue {
    One(String),
    Many(Vec<String>),
}

impl JsonHeadersValue {
    fn push(&mut self, value: String) {
        match self {
            JsonHeadersValue::One(_) => {
                let prev = std::mem::replace(self, JsonHeadersValue::Many(Vec::with_capacity(2)));
                if let (Self::One(prev_value), Self::Many(vec)) = (prev, self) {
                    vec.push(prev_value);
                    vec.push(value);
                }
            }
            JsonHeadersValue::Many(vec) => vec.push(value),
        }
    }
}

impl From<HttpResponseRecord> for HttpJsonRecord {
    fn from(resp_record: HttpResponseRecord) -> Self {
        let HttpResponseRecord {
            version,
            status_code,
            status_string,
            headers,
            body,
        } = resp_record;

        let header = headers.map(headers_to_json);

        let status = match (&version, &status_code, &status_string) {
            (None, None, None) => None,
            _ => Some(HttpJsonStatus {
                version,
                code: status_code,
                string: status_string,
            }),
        };

        HttpJsonRecord {
            status,
            header,
            body,
        }
    }
}

async fn record_from_response(response: Response) -> Result<HttpResponseRecord> {
    let mut response_record =
        HttpResponseRecord::try_from(&response).context("Failed to read response headers")?;
    let body = response
        .text()
        .await
        .context("Failed to read response body")?;
    response_record.body = Some(body);
    Ok(response_record)
}

fn headers_to_json(headers: Vec<HttpHeader>) -> BTreeMap<String, JsonHeadersValue> {
    let mut result: BTreeMap<String, JsonHeadersValue> = BTreeMap::new();
    for header in headers {
        match result.entry(header.name) {
            Entry::Occupied(mut entry) => entry.get_mut().push(header.value),
            Entry::Vacant(entry) => {
                entry.insert(JsonHeadersValue::One(header.value));
            }
        };
    }
    result
}

#[cfg(test)]
mod tests {
    use mockito::{mock, server_url, Mock};
    use reqwest::Client;

    use super::*;

    #[test]
    fn test_multiple_headers_to_json() {
        //given
        let headers = vec![
            HttpHeader {
                name: "name1".to_string(),
                value: "value1".to_string(),
            },
            HttpHeader {
                name: "name2".to_string(),
                value: "value21".to_string(),
            },
            HttpHeader {
                name: "name2".to_string(),
                value: "value22".to_string(),
            },
        ];

        //when
        let map = headers_to_json(headers);

        //then
        assert_eq!(
            map,
            BTreeMap::from([
                (
                    "name1".to_string(),
                    JsonHeadersValue::One("value1".to_string())
                ),
                (
                    "name2".to_string(),
                    JsonHeadersValue::Many(vec!["value21".to_string(), "value22".to_string()])
                )
            ])
        )
    }

    #[async_std::test]
    async fn test_full_text_output() -> Result<()> {
        //given
        let response = send_request().await?;
        let formatter = formatter(OutputType::Text, OutputParts::Full);

        //when
        let string = formatter.to_string(response).await?;

        //then
        assert_eq!(
            string,
            "HTTP/1.1 201 Created\nconnection: close\ncontent-type: text/plain\nx-api-key: 1234\nx-api-attribute: a1\nx-api-attribute: a2\ncontent-length: 5\n\nworld"
        );
        Ok(())
    }

    #[async_std::test]
    async fn test_body_text_output() -> Result<()> {
        //given
        let response = send_request().await?;
        let formatter = formatter(OutputType::Text, OutputParts::Body);

        //when
        let string = formatter.to_string(response).await?;

        //then
        assert_eq!(string, "world");
        Ok(())
    }

    #[async_std::test]
    async fn test_full_json_output() -> Result<()> {
        //given
        let response = send_request().await?;
        let formatter = formatter(OutputType::Json, OutputParts::Full);

        //when
        let string = formatter.to_string(response).await?;

        //then
        assert_eq!(
            string,
            r#"{"status":{"version":"HTTP/1.1","code":201,"string":"Created"},"header":{"connection":"close","content-length":"5","content-type":"text/plain","x-api-attribute":["a1","a2"],"x-api-key":"1234"},"body":"world"}"#
        );
        Ok(())
    }

    #[async_std::test]
    async fn test_body_json_output() -> Result<()> {
        //given
        let response = send_request().await?;
        let formatter = formatter(OutputType::Json, OutputParts::Body);

        //when
        let string = formatter.to_string(response).await?;

        //then
        assert_eq!(string, r#"{"body":"world"}"#);
        Ok(())
    }

    #[async_std::test]
    async fn test_unparsable_header() -> Result<()> {
        //given
        let mock = mock("GET", "/bad")
            .with_status(201)
            .with_header("bad-header", "🦄")
            .create();
        let client = Client::new();
        let response = client
            .request("GET".parse()?, format!("{}/bad", server_url()))
            .send()
            .await?;
        let formatter = formatter(OutputType::Json, OutputParts::Body);

        //when
        let res = formatter.to_string(response).await;

        //then
        mock.assert();
        assert_eq!(
            res.unwrap_err().to_string(),
            "Failed to read response headers"
        );
        Ok(())
    }

    async fn send_request() -> Result<Response> {
        let (url, mock) = create_mock();
        let client = Client::new();
        let request = client.request("GET".parse()?, format!("{url}/hello"));
        let response = request.send().await?;
        mock.assert();
        Ok(response)
    }

    fn create_mock() -> (String, Mock) {
        (
            server_url(),
            mock("GET", "/hello")
                .with_status(201)
                .with_header("content-type", "text/plain")
                .with_header("x-api-key", "1234")
                .with_header("x-api-attribute", "a1")
                .with_header("x-api-attribute", "a2")
                .with_body("world")
                .create(),
        )
    }
}
