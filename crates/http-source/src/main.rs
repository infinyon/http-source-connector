mod backoff;
mod config;
mod formatter;
mod http_streaming_source;
mod source;

use std::time::Duration;

use anyhow::Result;
use async_std::stream::StreamExt;
use backoff::Backoff;
use config::HttpConfig;
use fluvio::{RecordKey, TopicProducer};
use fluvio_connector_common::{
    connector,
    tracing::{debug, info, trace, warn},
    Source,
};

use crate::http_streaming_source::reconnect_stream_with_backoff;
use source::HttpSource;

const SIGNATURES: &str = concat!("InfinyOn HTTP Source Connector ", env!("CARGO_PKG_VERSION"));
const BACKOFF_LIMIT: Duration = Duration::from_secs(1000);

#[allow(unreachable_code)]
#[connector(source)]
async fn start(config: HttpConfig, producer: TopicProducer) -> Result<()> {
    debug!(?config);
    let mut backoff = Backoff::new();

    loop {
        let mut stream = if config.stream {
            match reconnect_stream_with_backoff(&config, &mut backoff).await {
                Ok(stream) => stream,
                Err(_err) => {
                    continue;
                }
            }
        } else {
            HttpSource::new(&config)?.connect(None).await?
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
