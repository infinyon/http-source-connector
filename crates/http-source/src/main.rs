mod config;
mod formatter;
mod http_streaming_source;
mod source;

use anyhow::Result;
use async_std::stream::StreamExt;
use config::HttpConfig;
use fluvio::{RecordKey, TopicProducer};
use fluvio_connector_common::{
    connector,
    tracing::{debug, info, trace},
    Source,
};

use http_streaming_source::HttpStreamingSource;
use source::HttpSource;

const SIGNATURES: &str = concat!("InfinyOn HTTP Source Connector ", env!("CARGO_PKG_VERSION"));

#[connector(source)]
#[allow(unreachable_code)]
async fn start(config: HttpConfig, producer: TopicProducer) -> Result<()> {
    debug!(?config);

    loop {
        let mut stream = if config.stream {
            match HttpStreamingSource::new(&config)?.connect(None).await {
                Ok(stream) => stream,
                Err(err) => {
                    info!("Error connecting to streaming source: {}", err);
                    //sleep 500ms before reconnecting
                    async_std::task::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
            }
        } else {
            HttpSource::new(&config)?.connect(None).await?
        };

        info!("Starting {SIGNATURES}");

        while let Some(item) = stream.next().await {
            trace!(?item);
            producer.send(RecordKey::NULL, item).await?;
        }

        info!("Consumer loop finished");
    }

    Ok(())
}
