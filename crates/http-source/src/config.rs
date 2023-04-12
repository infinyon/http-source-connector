use std::time::Duration;

use fluvio_connector_common::{connector, secret::SecretString};
use serde::Deserialize;

const DEFAULT_USER_AGENT: &str = "fluvio/http-source 0.5.0";
const DEFAULT_HTTP_METHOD: &str = "GET";
const DEFAULT_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Debug)]
#[connector(config, name = "http")]
pub(crate) struct HttpConfig {
    /// Endpoint for the http connector
    pub endpoint: SecretString,

    /// HTTP body for the request
    pub body: Option<String>,

    /// HTTP user-agent header for the request
    #[serde(default = "default_user_agent")]
    pub user_agent: String,

    /// HTTP method used in the request. Eg. GET, POST, PUT...
    #[serde(default = "default_http_method")]
    pub method: String,

    /// Time to wait before sending
    /// Ex: '150ms', '20s'
    #[serde(with = "humantime_serde", default = "default_interval")]
    pub interval: Duration,

    /// Headers to include in the HTTP request, in "Key=Value" format
    #[serde(default = "Vec::new")]
    pub headers: Vec<SecretString>,

    /// Response output parts: body | full
    #[serde(default = "Default::default")]
    pub output_parts: OutputParts,

    /// Response output type: text | json
    #[serde(default = "Default::default")]
    pub output_type: OutputType,
}

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OutputParts {
    #[default]
    Body,
    Full,
}

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OutputType {
    #[default]
    Text,
    Json,
}

fn default_user_agent() -> String {
    DEFAULT_USER_AGENT.into()
}

fn default_http_method() -> String {
    DEFAULT_HTTP_METHOD.into()
}

fn default_interval() -> Duration {
    DEFAULT_INTERVAL
}
