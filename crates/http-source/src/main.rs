mod backoff;
mod config;
mod formatter;
mod http_streaming_source;
mod source;
mod websocket_source;

use std::time::Duration;

use anyhow::Result;
use async_std::stream::StreamExt;
use backoff::Backoff;
use config::HttpConfig;
use fluvio::{RecordKey, TopicProducer};
use fluvio_connector_common::{
    connector,
    tracing::{debug, error, info, trace, warn},
    Source,
};
use futures::stream::LocalBoxStream;
use url::Url;

use crate::http_streaming_source::HttpStreamingSource;
use source::HttpSource;
use websocket_source::WebSocketSource;

const SIGNATURES: &str = concat!("InfinyOn HTTP Source Connector ", env!("CARGO_PKG_VERSION"));
const BACKOFF_LIMIT: Duration = Duration::from_secs(1000);

#[allow(unreachable_code)]
#[connector(source)]
async fn start(config: HttpConfig, producer: TopicProducer) -> Result<()> {
    debug!(?config);

    let url = Url::parse(&config.endpoint.resolve()?)?;
    let mut backoff = Backoff::new();

    loop {
        let stream = if url.scheme() == "ws" || url.scheme() == "wss" {
            with_backoff(&config, &mut backoff, WebSocketSource::new).await
        } else if config.stream {
            with_backoff(&config, &mut backoff, HttpStreamingSource::new).await
        } else {
            with_backoff(&config, &mut backoff, HttpSource::new).await
        };

        let mut stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };

        info!("Connected to source endpoint! Starting {SIGNATURES}");

        while let Some(item) = stream.next().await {
            trace!(?item);
            producer.send(RecordKey::NULL, item).await?;
        }

        warn!("Disconnected from source endpoint, attempting reconnect...");
        backoff = Backoff::new();
    }

    Ok(())
}

async fn with_backoff<'a, F, C>(
    config: &HttpConfig,
    backoff: &mut Backoff,
    new: F,
) -> Result<LocalBoxStream<'a, String>>
where
    F: FnOnce(&HttpConfig) -> Result<C>,
    C: Source<'a, String>,
{
    let wait = backoff.next();

    if wait > BACKOFF_LIMIT {
        error!("Max retry reached, exiting");
    }

    match new(config)?.connect(None).await {
        Ok(stream) => Ok(stream),
        Err(err) => {
            warn!(
                "Error connecting to streaming source: \"{}\", reconnecting in {}.",
                err,
                humantime::format_duration(wait)
            );

            async_std::task::sleep(wait).await;

            Err(err)
        }
    }
}
