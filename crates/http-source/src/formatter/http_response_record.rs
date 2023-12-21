use super::http_response_metadata::HttpResponseMetadata;

#[derive(Debug, Default, Clone)]
pub(crate) struct HttpResponseRecord {
    pub metadata: HttpResponseMetadata,
    pub body: Option<String>,
}

impl HttpResponseRecord {
    pub fn new(response_metadata: HttpResponseMetadata, record_body: String) -> Self {
        Self {
            metadata: response_metadata,
            body: Some(record_body),
        }
    }
}
