mod http_json_record;
mod http_response_metadata;
mod http_response_record;
mod json_formatter;
mod text_formatter;

use std::sync::Arc;

pub(crate) use http_response_metadata::HttpResponseMetadata;
pub(crate) use http_response_record::HttpResponseRecord;
use json_formatter::JsonFormatter;
use text_formatter::TextFormatter;

use crate::config::{OutputParts, OutputType};

pub(crate) trait Formatter {
    fn to_string(&self, response: &HttpResponseRecord) -> anyhow::Result<String>;
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
