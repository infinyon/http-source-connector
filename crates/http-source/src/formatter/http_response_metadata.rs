use anyhow::Result;
use reqwest::Response;

#[derive(Debug, Default, Clone)]
pub(crate) struct HttpResponseMetadata {
    pub version: Option<String>,
    pub status_code: Option<u16>,
    pub status_string: Option<&'static str>,
    pub headers: Option<Vec<HttpHeader>>,
}

#[derive(Debug, Clone)]
pub(crate) struct HttpHeader {
    pub name: String,
    pub value: String,
}

impl HttpResponseMetadata {
    pub fn new(response: &Response) -> Result<Self> {
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
        })
    }
}
