mod config;
mod formatter;
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

use crate::source::HttpSource;

const SIGNATURES: &str = concat!("InfinyOn HTTP Source Connector ", env!("CARGO_PKG_VERSION"));

#[connector(source)]
async fn start(config: HttpConfig, producer: TopicProducer) -> Result<()> {
    debug!(?config);
    let source = HttpSource::new(&config)?;
    let mut stream = source.connect(None).await?;

    info!("Starting {SIGNATURES}");

    while let Some(item) = stream.next().await {
        trace!(?item);
        producer.send(RecordKey::NULL, item).await?;
    }

    info!("Consumer loop finished");

    Ok(())
}
