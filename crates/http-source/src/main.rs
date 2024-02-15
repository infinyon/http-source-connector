mod config;
mod formatter;
mod websocket_source;
mod http_streaming_source;
mod source;

use anyhow::Result;
use async_std::stream::StreamExt;
use config::HttpConfig;
use url::Url;
use fluvio::{RecordKey, TopicProducer};
use fluvio_connector_common::{
    connector,
    tracing::{debug, info, trace},
    Source,
};

use http_streaming_source::HttpStreamingSource;
use source::HttpSource;
use websocket_source::WebSocketSource;

const SIGNATURES: &str = concat!("InfinyOn HTTP Source Connector ", env!("CARGO_PKG_VERSION"));

#[connector(source)]
async fn start(config: HttpConfig, producer: TopicProducer) -> Result<()> {
    debug!(?config);

    let url = Url::parse(&config.endpoint.resolve()?)?;
    let mut stream = if url.scheme() == "ws" || url.scheme() == "wss" {
        WebSocketSource::new(&config)?.connect(None).await?
    } else if config.stream {
        HttpStreamingSource::new(&config)?.connect(None).await?
    } else {
        HttpSource::new(&config)?.connect(None).await?
    };

    info!("Starting {SIGNATURES}");

    while let Some(item) = stream.next().await {
        trace!(?item);
        producer.send(RecordKey::NULL, item).await?;
    }

    info!("Consumer loop finished");

    Ok(())
}
