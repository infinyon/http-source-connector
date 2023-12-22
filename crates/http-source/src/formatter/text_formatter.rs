use crate::config::OutputParts;

use super::{http_response_record::HttpResponseRecord, Formatter, HttpResponseMetadata};

#[derive(Clone)]
pub(crate) struct TextFormatter(pub OutputParts);

impl Formatter for TextFormatter {
    fn to_string(&self, record: &HttpResponseRecord) -> anyhow::Result<String> {
        let HttpResponseMetadata {
            version,
            status_code,
            status_string,
            headers,
        } = &record.metadata;
        let body = &record.body;

        let mut record_out_parts: Vec<String> = Vec::new();
        if let OutputParts::Full = self.0 {
            // Status Line HTTP/X 200 CANONICAL
            let status_line: Vec<String> = vec![
                version.clone().unwrap_or_default(),
                status_code.unwrap_or_default().to_string(),
                status_string.unwrap_or_default().to_string(),
            ];
            record_out_parts.push(status_line.join(" "));

            // Header lines foo: bar
            if let Some(headers) = headers {
                let hdr_out_parts: Vec<String> = headers
                    .iter()
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
            record_out_parts.push(body.clone());
        }

        Ok(record_out_parts.join("\n"))
    }
}
