use crate::config::OutputParts;

use super::{
    http_json_record::HttpJsonRecord, http_response_record::HttpResponseRecord, Formatter,
};

#[derive(Clone)]
pub(crate) struct JsonFormatter(pub OutputParts);

impl Formatter for JsonFormatter {
    fn to_string(&self, record: &HttpResponseRecord) -> anyhow::Result<String> {
        let json_record = match self.0 {
            OutputParts::Body => HttpJsonRecord::from(&HttpResponseRecord {
                body: record.body.clone(),
                ..Default::default()
            }),
            OutputParts::Full => HttpJsonRecord::from(record),
        };

        Ok(serde_json::to_string(&json_record)?)
    }
}
