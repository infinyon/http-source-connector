mod backoff;
mod config;
mod formatter;
mod http_streaming_source;
mod source;

use anyhow::Result;
use async_std::stream::StreamExt;
use backoff::Backoff;
use config::HttpConfig;
use fluvio::{RecordKey, TopicProducer};
use fluvio_connector_common::{
    connector,
    tracing::{debug, error, info, trace},
    Source,
};

use futures::Stream;
use http_streaming_source::HttpStreamingSource;
use source::HttpSource;

const SIGNATURES: &str = concat!("InfinyOn HTTP Source Connector ", env!("CARGO_PKG_VERSION"));

#[connector(source)]
#[allow(unreachable_code)]
async fn start(config: HttpConfig, producer: TopicProducer) -> Result<()> {
    debug!(?config);
    let mut backoff = Backoff::new();

    loop {
        let mut stream = if config.stream {
            match connect_streaming_source(&config, &mut backoff).await {
                Ok(stream) => stream,
                Err(_err) => {
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

async fn connect_streaming_source(
    config: &HttpConfig,
    backoff: &mut Backoff,
) -> Result<std::pin::Pin<Box<dyn Stream<Item = String>>>> {
    match HttpStreamingSource::new(config)?.connect(None).await {
        Ok(stream) => Ok(stream),
        Err(err) => {
            info!("Error connecting to streaming source: {}", err);

            let wait = backoff.next();

            if wait > 10000 {
                error!("Failed to connect to streaming source after 20 retries");
            }

            async_std::task::sleep(std::time::Duration::from_millis(100 * wait)).await;

            Err(err)
        }
    }
}
