use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use tide::prelude::*;
use tide::sse;
use tide::sse::Sender;
use tide::Request;
use tide_websockets::WebSocket;

#[derive(Clone)]
struct State {
    get_count: Arc<AtomicU32>,
    post_count: Arc<AtomicU32>,
}
impl State {
    fn new() -> Self {
        Self {
            get_count: Arc::new(AtomicU32::new(0)),
            post_count: Arc::new(AtomicU32::new(0)),
        }
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let mut app = tide::with_state(State::new());
    app.at("/get").get(get_request);
    app.at("/time").get(get_time_request);
    app.at("/post").post(post_request);
    app.at("/stream_count_updates")
        .get(sse::endpoint(stream_count_updates));
    app.at("/websocket")
        .get(WebSocket::new(|_request, stream| async move {
            for i in 1..11 {
                stream
                    .send_string(format!("Hello, Fluvio! - {}", i))
                    .await?;
            }
            Ok(())
        }));
    app.at("/websocket-auth")
        .get(WebSocket::new(|request, stream| async move {
            let header_values = request.header("x-secret-token");
            if header_values.is_none() || header_values.unwrap().last() != "abc123" {
                stream.send_string("Unauthorized".to_string()).await?;
                return Ok(());
            }

            for i in 1..11 {
                stream
                    .send_string(format!("Hello, Fluvio! - {}", i))
                    .await?;
            }
            Ok(())
        }));

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
async fn get_request(req: Request<State>) -> tide::Result {
    let state = req.state();
    let value = state.get_count.fetch_add(1, Ordering::Relaxed) + 1;
    Ok(format!("Hello, Fluvio! - {value}").into())
}

use std::time::{Duration, SystemTime, UNIX_EPOCH};

async fn get_time_request(_req: Request<State>) -> tide::Result {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    Ok(format!("{time}").into())
}

#[derive(Debug, Deserialize)]
struct HelloPostBody {
    name: String,
}

async fn post_request(mut req: Request<State>) -> tide::Result {
    let HelloPostBody { name } = req.body_json().await?;
    let state = req.state();
    let value = state.post_count.fetch_add(1, Ordering::Relaxed) + 1;
    Ok(format!("Hello, {name}! - {value}").into())
}

async fn stream_count_updates(req: Request<State>, sender: Sender) -> tide::Result<()> {
    let mut get_count = 0;
    let mut post_count = 0;
    let state = req.state();

    loop {
        let new_get_count = &state.get_count.load(Ordering::Relaxed);
        let new_post_count = &state.post_count.load(Ordering::Relaxed);

        let get_received = *new_get_count > get_count;
        let post_received = *new_post_count > post_count;

        let event = match (get_received, post_received) {
            (true, true) => Some("get and post requests"),
            (true, false) => Some("get request(s)"),
            (false, true) => Some("post request(s)"),
            (false, false) => None,
        };

        if let Some(event_name) = event {
            get_count = *new_get_count;
            post_count = *new_post_count;

            let counts = format!("{{ \"gets\": {}, \"posts\": {} }}", get_count, post_count,);

            sender.send(event_name, counts, None).await?;
        }

        sleep(Duration::from_millis(100));
    }
}
