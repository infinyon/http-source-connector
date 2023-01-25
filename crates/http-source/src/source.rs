use std::sync::Arc;

use crate::{
    config::HttpConfig,
    formatter::{formatter, Formatter},
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use fluvio::Offset;
use fluvio_connector_common::{tracing::error, Source};
use futures::{stream::LocalBoxStream, StreamExt};
use reqwest::{Client, RequestBuilder};
use tokio::time::Interval;
use tokio_stream::wrappers::IntervalStream;

pub(crate) struct HttpSource {
    interval: Interval,
    request: RequestBuilder,
    formatter: Arc<dyn Formatter + Sync + Send>,
}

impl HttpSource {
    pub(crate) fn new(config: &HttpConfig) -> Result<Self> {
        let client = Client::new();
        let method = config.method.parse()?;
        let mut request = client.request(method, config.endpoint.clone());
        request = request.header(reqwest::header::USER_AGENT, config.user_agent.clone());
        let headers = config.headers.iter().flat_map(|h| h.split_once(':'));
        for (key, value) in headers {
            request = request.header(key, value);
        }
        if let Some(ref body) = config.body {
            request = request.body(body.clone());
        }

        let interval = tokio::time::interval(config.interval);
        let formatter = formatter(config.output_type, config.output_parts);
        Ok(Self {
            interval,
            request,
            formatter,
        })
    }
}

#[async_trait]
impl<'a> Source<'a, String> for HttpSource {
    async fn connect(self, _offset: Option<Offset>) -> Result<LocalBoxStream<'a, String>> {
        let stream = IntervalStream::new(self.interval).filter_map(move |_| {
            let builder = self.request.try_clone();
            let formatter = self.formatter.clone();
            async move {
                match request(builder, formatter.as_ref()).await {
                    Ok(res) => Some(res),
                    Err(err) => {
                        error!("Request execution failed: {}", err);
                        None
                    }
                }
            }
        });
        Ok(stream.boxed_local())
    }
}

async fn request(builder: Option<RequestBuilder>, formatter: &dyn Formatter) -> Result<String> {
    let request = builder.ok_or_else(|| anyhow!("Request must be cloneable"))?;
    let response = request.send().await.context("Request failed")?;
    formatter.to_string(response).await
}
