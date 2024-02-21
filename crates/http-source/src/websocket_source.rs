use anyhow::{Context, Result};
use async_trait::async_trait;
use fluvio::Offset;
use fluvio_connector_common::{
    tracing::{debug, error, info},
    Source,
};
use futures::{
    self,
    stream::{LocalBoxStream, SplitSink},
    SinkExt,
};
use tokio::net::TcpStream;
use tokio::time::Duration;
use tokio_stream::{wrappers::IntervalStream, StreamExt};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use url::Url;

use crate::config::HttpConfig;

pub(crate) struct WebSocketSource {
    request: WSRequest,
    ping_interval_ms: u64,
}

#[derive(Clone)]
struct WSRequest {
    url: Url,
    subscription_message: Option<String>,
}

type Transport = MaybeTlsStream<TcpStream>;

#[async_trait]
trait PingStream {
    async fn ping(&mut self) -> Result<()>;
}

struct WSPingOnlySink(SplitSink<WebSocketStream<Transport>, Message>);

#[async_trait]
impl PingStream for WSPingOnlySink {
    async fn ping(&mut self) -> Result<()> {
        self.0.send(Message::Ping(Vec::new())).await.map_err(|e| {
            error!("Failed to send ping: {}", e);
            anyhow::Error::new(e)
        })?;

        debug!("Ping sent");
        Ok(())
    }
}

async fn establish_connection(request: WSRequest) -> Result<WebSocketStream<Transport>> {
    match connect_async(&request.url).await {
        Ok((mut ws_stream, _)) => {
            info!("WebSocket connected to {}", &request.url);
            if let Some(message) = request.subscription_message.as_ref() {
                ws_stream.send(Message::Text(message.to_owned())).await?;
            }
            Ok(ws_stream)
        }
        Err(e) => {
            error!("WebSocket connection error: {}", e);
            Err(anyhow::Error::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            )))
        }
    }
}

async fn websocket_writer_and_stream<'a>(
    request: WSRequest,
) -> Result<(WSPingOnlySink, LocalBoxStream<'a, String>)> {
    let ws_stream = establish_connection(request)
        .await
        .context("Failed to establish WebSocket connection")?;

    let (write_half, read_half) = futures::stream::StreamExt::split(ws_stream);
    let stream = futures::stream::StreamExt::filter_map(read_half, |message_result| {
        async move {
            match message_result {
                Ok(message) => {
                    match message {
                        Message::Text(text) => Some(text),
                        Message::Binary(data) => {
                            if let Ok(text) = String::from_utf8(data) {
                                Some(text)
                            } else {
                                error!("Received binary data that could not be converted to UTF-8 text");
                                None
                            }
                        }

                        Message::Ping(_) | Message::Pong(_) => {
                            // upon receiving ping messages tungstenite queues pong replies automatically
                            debug!("Received ping/pong message, connection is alive");
                            None
                        }
                        Message::Close(_) => {
                            info!("Received WebSocket Close frame");
                            None
                        }
                        _ => {
                            // Ignore other message types
                            None
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket read error: {}", e);
                    // Depending on the error you may choose to stop and close or try to reconnect
                    None
                }
            }
        }
    });

    Ok((
        WSPingOnlySink(write_half),
        futures::stream::StreamExt::boxed_local(stream),
    ))
}

impl WebSocketSource {
    pub(crate) fn new(config: &HttpConfig) -> Result<Self> {
        let ws_config = config.websocket_config.as_ref();
        Ok(Self {
            request: WSRequest {
                url: Url::parse(&config.endpoint.resolve()?)
                    .context("unable to parse http endpoint")?,
                subscription_message: ws_config.and_then(|c| c.subscription_message.to_owned()),
            },
            ping_interval_ms: ws_config.and_then(|c| c.ping_interval_ms).unwrap_or(10_000),
        })
    }

    async fn connect_and_run<'a>(self) -> Result<LocalBoxStream<'a, String>> {
        enum StreamElement {
            Read(String),
            PingInterval,
        }

        let ws_stream_result = websocket_writer_and_stream(self.request.clone()).await?;

        let repeated_websocket = Box::pin(async_stream::stream! {
            let (mut ping_only, ws_stream) = ws_stream_result;

            let mut ws_stream = ws_stream
                .map(StreamElement::Read)
                .merge(IntervalStream::new(tokio::time::interval(Duration::from_millis(self.ping_interval_ms))).map(|_| StreamElement::PingInterval));

            while let Some(item) = ws_stream.next().await {
                match item {
                    StreamElement::Read(s) => yield s,
                    StreamElement::PingInterval => {
                        let ping_res = ping_only.ping().await;
                        if ping_res.is_err() { break; }
                    }
                }
            }
        });

        Ok(futures::stream::StreamExt::boxed_local(repeated_websocket))
    }
}

#[async_trait]
impl<'a> Source<'a, String> for WebSocketSource {
    async fn connect(self, _offset: Option<Offset>) -> Result<LocalBoxStream<'a, String>> {
        self.connect_and_run()
            .await
            .context("Failed to run WebSocket connection")
    }
}
