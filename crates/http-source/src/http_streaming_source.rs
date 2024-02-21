use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bytes::BytesMut;
use encoding_rs::{Encoding, UTF_8};
use fluvio::Offset;
use fluvio_connector_common::{
    tracing::{error, warn},
    Source,
};
use futures::{stream::BoxStream, stream::LocalBoxStream, StreamExt};
use reqwest::{Client, RequestBuilder, Url};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    config::HttpConfig,
    formatter::{formatter, Formatter, HttpResponseMetadata, HttpResponseRecord},
};

pub(crate) struct HttpStreamingSource {
    request: RequestBuilder,
    delimiter: Vec<u8>,
    formatter: Arc<dyn Formatter + Sync + Send>,
}

#[async_trait]
impl<'a> Source<'a, String> for HttpStreamingSource {
    async fn connect(mut self, _offset: Option<Offset>) -> Result<LocalBoxStream<'a, String>> {
        let request = self
            .request
            .try_clone()
            .context("request must be cloneable")?;

        let response = request.send().await.context("send request")?;

        let response_metadata = HttpResponseMetadata::new(&response)?;
        let encoding = transfer_encoding(&response);

        Ok(self.record_stream(response, response_metadata, encoding))
    }
}

impl HttpStreamingSource {
    pub(crate) fn new(config: &HttpConfig) -> Result<Self> {
        let client = Client::new();
        let method = config.method.parse()?;
        let url = Url::parse(&config.endpoint.resolve()?).context("parse http endpoint")?;
        let mut request = client.request(method, url);

        request = request.header(reqwest::header::USER_AGENT, config.user_agent.clone());
        let headers = config
            .headers
            .iter()
            .map(|h| h.resolve().unwrap_or_default())
            .collect::<Vec<_>>();

        for (key, value) in headers.iter().flat_map(|h| h.split_once(':')) {
            request = request.header(key, value);
        }

        if let Some(ref body) = config.body {
            request = request.body(body.clone());
        }

        let delimiter = config.delimiter.as_bytes().to_vec();

        let formatter = formatter(config.output_type, config.output_parts);

        Ok(Self {
            delimiter,
            request,
            formatter,
        })
    }

    pub(crate) fn record_stream(
        self,
        response: reqwest::Response,
        response_metadata: HttpResponseMetadata,
        encoding: &'static Encoding,
    ) -> LocalBoxStream<'static, String> {
        let (tx1, rx1) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            read_http_stream(
                response.bytes_stream().boxed(),
                tx1,
                self.delimiter,
                encoding,
            )
            .await;
        });

        let (tx2, rx2) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            write_to_output_stream(rx1, tx2, response_metadata, self.formatter).await;
        });

        Box::pin(UnboundedReceiverStream::new(rx2))
    }
}

async fn read_http_stream(
    mut stream: BoxStream<'_, Result<bytes::Bytes, reqwest::Error>>,
    tx: mpsc::UnboundedSender<String>,
    delimiter: Vec<u8>,
    encoding: &'static Encoding,
) {
    let mut buf = BytesMut::new();

    while let Some(bytes) = stream.next().await {
        match bytes {
            Ok(bytes) => {
                buf.extend_from_slice(bytes.as_ref());

                dequeue_and_forward_records(&mut buf, &tx, &delimiter, encoding)
            }
            Err(e) => {
                warn!("could not read data from http response stream: {}", e);
            }
        }
    }
}

fn dequeue_and_forward_records(
    buf: &mut BytesMut,
    tx: &mpsc::UnboundedSender<String>,
    delimiter: &[u8],
    encoding: &'static Encoding,
) {
    while let Some(index) = first_delim_index(buf, delimiter) {
        let next_record = dequeue_next_record(buf, index, delimiter);
        let decoded_record = decoded_record_body(next_record, encoding);

        let stream_result = tx.send(decoded_record);
        if let Err(e) = stream_result {
            error!("Couldn't send bytes to formatting task: {e}");
        }
    }
}

async fn write_to_output_stream(
    mut rx: mpsc::UnboundedReceiver<String>,
    tx: mpsc::UnboundedSender<String>,
    response_metadata: HttpResponseMetadata,
    formatter: Arc<dyn Formatter + Sync + Send>,
) {
    while let Some(record) = rx.recv().await {
        let res = format_record(record, response_metadata.clone(), &formatter);

        match res {
            Ok(record) => {
                let stream_result = tx.send(record);

                if let Err(e) = stream_result {
                    error!("Couldn't send records to output stream: {e}");
                }
            }
            Err(err) => {
                error!("Error formatting record: {err:?}");
            }
        }
    }
}

fn format_record(
    record: String,
    response_metadata: HttpResponseMetadata,
    formatter: &Arc<dyn Formatter + Sync + Send>,
) -> Result<String> {
    let formatter_input = HttpResponseRecord::new(response_metadata, record);

    let formatted_record = formatter.to_string(&formatter_input).map_err(|err| {
        anyhow!(
            "formatting failed, record: {:?}, reason: {:?}",
            formatter_input,
            err
        )
    })?;

    Ok(formatted_record)
}

fn first_delim_index(bytes: &[u8], delimiter: &[u8]) -> Option<usize> {
    if delimiter.is_empty() {
        return None;
    }

    if bytes.len() < delimiter.len() {
        return None;
    }

    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(delimiter) {
            return Some(i);
        }

        i += 1;
    }

    None
}

fn dequeue_next_record(buffer: &mut BytesMut, index: usize, delimiter: &[u8]) -> BytesMut {
    let mut next_record = buffer.split_to(index + delimiter.len());

    next_record.truncate(next_record.len() - delimiter.len());

    next_record
}

fn decoded_record_body(record_body: BytesMut, encoding: &'static Encoding) -> String {
    let (text, _, _) = encoding.decode(&record_body);

    text.into_owned()
}

// inspired by reqwest::Response::text()
fn transfer_encoding(response: &reqwest::Response) -> &'static Encoding {
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<mime::Mime>().ok());

    let encoding_name = content_type
        .as_ref()
        .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
        .unwrap_or("utf-8");

    Encoding::for_label(encoding_name.as_bytes()).unwrap_or(UTF_8)
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::StreamExt;
    use tokio::sync::mpsc;

    #[async_std::test]
    async fn test_read_http_stream_concatenates_chunks() {
        let inner_stream = futures::stream::iter(vec![
            Ok(bytes::Bytes::from("Hello")),
            Ok(bytes::Bytes::from(" world")),
            Ok(bytes::Bytes::from("!")),
            Ok(bytes::Bytes::from(" Welcome")),
            Ok(bytes::Bytes::from(" to ")),
            Ok(bytes::Bytes::from("NY!")),
        ]);
        let http_stream = inner_stream.boxed();

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            read_http_stream(http_stream, tx, "!".into(), encoding_rs::UTF_8).await
        });
        let mut chunked_stream = Box::pin(UnboundedReceiverStream::new(rx));

        let first_chunk = chunked_stream.next().await;
        assert_eq!(first_chunk.unwrap(), String::from("Hello world"));

        let second_chunk = chunked_stream.next().await;
        assert_eq!(second_chunk.unwrap(), String::from(" Welcome to NY"));
    }

    #[async_std::test]
    async fn test_read_http_stream_handles_remainders() {
        let inner_stream = futures::stream::iter(vec![
            Ok(bytes::Bytes::from("Hello wo")),
            Ok(bytes::Bytes::from("rld! Wel")),
            Ok(bytes::Bytes::from("come to NY!")),
        ]);
        let http_stream = inner_stream.boxed();

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            read_http_stream(http_stream, tx, "!".into(), encoding_rs::UTF_8).await
        });
        let mut chunked_stream = Box::pin(UnboundedReceiverStream::new(rx));

        let first_chunk = chunked_stream.next().await;
        assert_eq!(first_chunk.unwrap(), String::from("Hello world"));

        let second_chunk = chunked_stream.next().await;
        assert_eq!(second_chunk.unwrap(), String::from(" Welcome to NY"));
    }

    #[async_std::test]
    async fn test_read_http_stream_handles_multi_record_chunks() {
        let inner_stream = futures::stream::iter(vec![Ok(bytes::Bytes::from(
            "Hello world! Welcome to NY! Glad you coul",
        ))]);
        let http_stream = inner_stream.boxed();

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            read_http_stream(http_stream, tx, "!".into(), encoding_rs::UTF_8).await
        });
        let mut chunked_stream = Box::pin(UnboundedReceiverStream::new(rx));

        let first_chunk = chunked_stream.next().await;
        assert_eq!(first_chunk.unwrap(), String::from("Hello world"));

        let second_chunk = chunked_stream.next().await;
        assert_eq!(second_chunk.unwrap(), String::from(" Welcome to NY"));
    }

    #[async_std::test]
    async fn test_read_http_stream_handles_chunks_beginning_with_delimiter() {
        let inner_stream = futures::stream::iter(vec![
            Ok(bytes::Bytes::from("Hello world")),
            Ok(bytes::Bytes::from("! Welcome to NY!")),
        ]);
        let http_stream = inner_stream.boxed();

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            read_http_stream(http_stream, tx, "!".into(), encoding_rs::UTF_8).await
        });
        let mut chunked_stream = Box::pin(UnboundedReceiverStream::new(rx));

        let first_chunk = chunked_stream.next().await;
        assert_eq!(first_chunk.unwrap(), String::from("Hello world"));

        let second_chunk = chunked_stream.next().await;
        assert_eq!(second_chunk.unwrap(), String::from(" Welcome to NY"));
    }

    #[async_std::test]
    async fn test_read_http_stream_handles_chunks_ending_with_delimiter() {
        let inner_stream = futures::stream::iter(vec![
            Ok(bytes::Bytes::from("Hello wo")),
            Ok(bytes::Bytes::from("rld!")),
            Ok(bytes::Bytes::from(" Welcome to NY!")),
        ]);
        let http_stream = inner_stream.boxed();

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            read_http_stream(http_stream, tx, "!".into(), encoding_rs::UTF_8).await
        });
        let mut chunked_stream = Box::pin(UnboundedReceiverStream::new(rx));

        let first_chunk = chunked_stream.next().await;
        assert_eq!(first_chunk.unwrap(), String::from("Hello world"));

        let second_chunk = chunked_stream.next().await;
        assert_eq!(second_chunk.unwrap(), String::from(" Welcome to NY"));
    }

    #[test]
    fn test_first_delim_index_finds_single_byte_delimiters() {
        assert_eq!(super::first_delim_index(b"", b"\n"), None);
        assert_eq!(super::first_delim_index(b"0", b"\n"), None);
        assert_eq!(super::first_delim_index(b"\n", b"\n"), Some(0));
        assert_eq!(super::first_delim_index(b"0\n", b"\n"), Some(1));
        assert_eq!(super::first_delim_index(b"\n2", b"\n"), Some(0));
        assert_eq!(super::first_delim_index(b"\n2\n", b"\n"), Some(0));
        assert_eq!(super::first_delim_index(b"012345", b"\n"), None);
        assert_eq!(super::first_delim_index(b"0123\n6", b"\n"), Some(4));
        assert_eq!(super::first_delim_index(b"0123\n5\n", b"\n"), Some(4));
        assert_eq!(super::first_delim_index(b"0123\n56\n", b"\n"), Some(4));
        assert_eq!(super::first_delim_index(b"0123\n56\n8", b"\n"), Some(4));
    }

    #[test]
    fn test_first_delim_index_finds_multi_byte_delimiters() {
        assert_eq!(super::first_delim_index(b"", b",\n"), None);
        assert_eq!(super::first_delim_index(b"0", b",\n"), None);
        assert_eq!(super::first_delim_index(b",\n", b",\n"), Some(0));
        assert_eq!(super::first_delim_index(b"0,\n", b",\n"), Some(1));
        assert_eq!(super::first_delim_index(b",\n2", b",\n"), Some(0));
        assert_eq!(super::first_delim_index(b",\n2,\n", b",\n"), Some(0));
        assert_eq!(super::first_delim_index(b"012345", b",\n"), None);
        assert_eq!(super::first_delim_index(b"0123,\n6", b",\n"), Some(4));
        assert_eq!(super::first_delim_index(b"0123,\n6,\n", b",\n"), Some(4));
        assert_eq!(super::first_delim_index(b"0123,\n67,\n", b",\n"), Some(4));
        assert_eq!(super::first_delim_index(b"0123,\n67,\n8", b",\n"), Some(4));
    }
}
