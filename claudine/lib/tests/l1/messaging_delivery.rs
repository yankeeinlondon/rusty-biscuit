//! The embedder's drain contract, through the public API only.
//!
//! A program that embeds `claudine` sends with `execute_resolved_message`,
//! which returns at once, and opts in to delivery before exit by awaiting
//! `drain_deliveries`. These tests pin that contract against a loopback
//! webhook listener: a drained send has reached the listener and received its
//! reply, and a stalled one comes back as pending under its route name alone.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use claudine::messaging::{
    DELIVERY_DRAIN_BUDGET, DeliveryLabel, MessagingRouteConfig, RuntimeMessagingSettings,
    ScopedMessagingSettings, drain_deliveries, execute_resolved_message,
};
use tokio::time::Instant;

const ROUTE: &str = "alerts";
const BODY_TEXT: &str = "Nightly build finished";
const TOKEN: &str = "dummy-token";

struct Request {
    method: String,
    path: String,
    body: String,
}

enum Reply {
    /// Hold the reply this long after the request arrives, then answer `200`.
    OkAfter(Duration),
    /// Read the request and never answer.
    Never,
}

/// Serve one webhook request on loopback. The receiver yields the request as
/// soon as it is read; the second yields once the reply has been written.
fn serve_one(reply: Reply) -> (u16, mpsc::Receiver<Request>, mpsc::Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let (request_tx, request_rx) = mpsc::channel();
    let (replied_tx, replied_rx) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept the webhook post");
        let request = read_request(&mut stream);
        let _ = request_tx.send(request);
        match reply {
            Reply::OkAfter(delay) => {
                thread::sleep(delay);
                let body = r#"{"id":"1","channel_id":"2","webhook_id":"1"}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write reply");
                stream.flush().expect("flush reply");
                let _ = replied_tx.send(());
            }
            // Keep the connection open for as long as the process lives.
            Reply::Never => thread::sleep(Duration::from_secs(600)),
        }
    });
    (port, request_rx, replied_rx)
}

fn read_request(stream: &mut TcpStream) -> Request {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("read timeout");
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stream.read(&mut chunk).expect("read the webhook post");
        assert!(read > 0, "the client closed before sending a whole request");
        buffer.extend_from_slice(&chunk[..read]);
        let Some(header_end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let head = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
        let content_length = head
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
            .map_or(0, |(_, value)| value.trim().parse::<usize>().expect("content length"));
        let body_start = header_end + 4;
        if buffer.len() < body_start + content_length {
            continue;
        }
        let mut request_line = head.lines().next().unwrap_or_default().split(' ');
        return Request {
            method: request_line.next().unwrap_or_default().to_string(),
            path: request_line.next().unwrap_or_default().to_string(),
            body: String::from_utf8_lossy(&buffer[body_start..body_start + content_length])
                .into_owned(),
        };
    }
}

/// A user-scoped Discord webhook route named [`ROUTE`] pointing at `port`.
fn route_to(port: u16) -> RuntimeMessagingSettings {
    let config = MessagingRouteConfig::DiscordWebhook {
        webhook_url: Some(format!("http://127.0.0.1:{port}/webhooks/1/{TOKEN}")),
        webhook_url_env: "CLAUDINE_TEST_UNUSED_WEBHOOK_URL".to_string(),
    };
    RuntimeMessagingSettings {
        user: Some(ScopedMessagingSettings {
            active: Some(ROUTE.to_string()),
            configs: HashMap::from([(ROUTE.to_string(), config)]),
        }),
        repo: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_drained_send_has_been_delivered_when_the_drain_returns() {
    let (port, requests, replied) = serve_one(Reply::OkAfter(Duration::from_millis(300)));

    execute_resolved_message(BODY_TEXT, None, None, None, &route_to(port));
    let outcome = drain_deliveries(Instant::now() + DELIVERY_DRAIN_BUDGET).await;

    // The reply was held back after the request arrived, so a drain that did
    // not wait for the task would return before it was written.
    replied
        .try_recv()
        .expect("the drain returned before the listener replied");
    let request = requests.try_recv().expect("the listener received the post");
    assert_eq!(request.method, "POST");
    assert!(
        request.path.starts_with(&format!("/webhooks/1/{TOKEN}")),
        "unexpected path {}",
        request.path
    );
    assert!(request.body.contains(BODY_TEXT), "body was {}", request.body);
    assert!(requests.try_recv().is_err(), "more than one post arrived");
    assert!(outcome.pending.is_empty());
    assert!(outcome.panicked.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stalled_send_is_pending_under_its_route_name_alone() {
    let (port, requests, _replied) = serve_one(Reply::Never);

    execute_resolved_message(BODY_TEXT, None, None, None, &route_to(port));
    let start = Instant::now();
    let outcome = drain_deliveries(start + Duration::from_millis(750)).await;

    assert!(
        start.elapsed() < Duration::from_secs(5),
        "the drain overran its deadline: {:?}",
        start.elapsed()
    );
    requests
        .try_recv()
        .expect("the stalled post reached the listener before the deadline");
    assert_eq!(outcome.pending, vec![DeliveryLabel::Route(ROUTE.to_string())]);
    let shown = outcome.pending[0].to_string();
    for secret in [TOKEN, "127.0.0.1", BODY_TEXT] {
        assert!(!shown.contains(secret), "the label {shown:?} leaks {secret:?}");
    }

    // The aborted task is gone: a second drain finds nothing to wait for.
    let again = drain_deliveries(Instant::now() + DELIVERY_DRAIN_BUDGET).await;
    assert!(again.pending.is_empty());
}
