//! A loopback HTTP listener that stands in for a Discord webhook.
//!
//! Claudine's inline webhook validator only accepts a production Discord
//! host, but the messenger's `parse_webhook_url` accepts any URL with a
//! `/webhooks/{id}/{token}` path. [`write_webhook_route`] therefore writes an
//! environment-backed route, and [`WebhookListener::apply_route_env`] points
//! that variable at this listener, so the production config validation stays
//! in the path under test.
//!
//! The token in [`WebhookListener::webhook_url`] is [`DUMMY_TOKEN`]. Redaction
//! only recognizes `https://discord.com/...` URLs, so a loopback URL in an
//! error string is printed verbatim; the dummy token keeps that harmless and
//! gives a test a needle to assert is absent.
//!
//! One thread serves every connection. It never blocks on a single client, so
//! several withheld or stalled deliveries can be held open at once, and a
//! watchdog ends it even if the test forgets to.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::ConfigurableCommand;

/// The route name [`write_webhook_route`] makes active.
pub const ROUTE_NAME: &str = "drain-test";

/// The environment variable the route reads its webhook URL from.
pub const WEBHOOK_URL_ENV: &str = "DRAIN_TEST_WEBHOOK_URL";

/// The secret segment of the listener's webhook URL.
pub const DUMMY_TOKEN: &str = "dummy-token";

/// How the listener answers each request it has read in full.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListenerMode {
    /// Answer immediately with this status.
    Reply(u16),
    /// Hold every connection open without a reply until
    /// [`WebhookListener::release`], then answer `200`.
    WithholdUntilReleased,
    /// Hold every connection open and never answer.
    NeverReply,
}

/// One request the listener read in full.
#[derive(Clone, Debug)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub body: String,
}

#[derive(Default)]
struct Shared {
    requests: Mutex<Vec<RecordedRequest>>,
    arrived: Condvar,
    released: AtomicBool,
    shutdown: AtomicBool,
}

/// A running listener; dropping it stops the serving thread.
pub struct WebhookListener {
    addr: SocketAddr,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

/// The listener's default lifetime ceiling.
pub const DEFAULT_WATCHDOG: Duration = Duration::from_secs(90);

impl WebhookListener {
    /// Start a listener with [`DEFAULT_WATCHDOG`].
    pub fn start(mode: ListenerMode) -> Self {
        Self::start_with_watchdog(mode, DEFAULT_WATCHDOG)
    }

    /// Start a listener that shuts itself down after `watchdog`, dropping any
    /// connection it still holds.
    pub fn start_with_watchdog(mode: ListenerMode, watchdog: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
        listener
            .set_nonblocking(true)
            .expect("non-blocking loopback listener");
        let addr = listener.local_addr().expect("listener address");
        let shared = Arc::new(Shared::default());
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("webhook-listener".into())
            .spawn(move || serve(listener, mode, thread_shared, Instant::now() + watchdog))
            .expect("spawn listener thread");
        Self {
            addr,
            shared,
            thread: Some(thread),
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The webhook URL a route should post to.
    pub fn webhook_url(&self) -> String {
        format!("http://{}/webhooks/1/{DUMMY_TOKEN}", self.addr)
    }

    /// Point [`WEBHOOK_URL_ENV`] at this listener on `command`, and keep any
    /// proxy the host exports from intercepting the loopback request.
    pub fn apply_route_env<C: ConfigurableCommand>(&self, command: &mut C) {
        command.set_variable(WEBHOOK_URL_ENV.as_ref(), self.webhook_url().as_ref());
        for proxy in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            command.remove_variable(proxy.as_ref());
        }
        command.set_variable("NO_PROXY".as_ref(), "127.0.0.1,localhost".as_ref());
    }

    /// Let every withheld connection receive its `200`.
    pub fn release(&self) {
        self.shared.released.store(true, Ordering::SeqCst);
    }

    /// Every request read so far, in arrival order.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.shared.requests.lock().unwrap().clone()
    }

    /// Wait until the first request has been read in full.
    pub fn wait_for_request(&self, timeout: Duration) -> Option<RecordedRequest> {
        self.wait_for_requests(1, timeout)
            .and_then(|requests| requests.into_iter().next())
    }

    /// Wait until at least `count` requests have been read in full.
    pub fn wait_for_requests(&self, count: usize, timeout: Duration) -> Option<Vec<RecordedRequest>> {
        let deadline = Instant::now() + timeout;
        let mut requests = self.shared.requests.lock().unwrap();
        while requests.len() < count {
            let remaining = deadline.checked_duration_since(Instant::now())?;
            requests = self
                .shared
                .arrived
                .wait_timeout(requests, remaining)
                .unwrap()
                .0;
        }
        Some(requests.clone())
    }
}

impl Drop for WebhookListener {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A connection whose request has not been read in full yet.
struct Reading {
    stream: TcpStream,
    buffer: Vec<u8>,
}

fn serve(listener: TcpListener, mode: ListenerMode, shared: Arc<Shared>, watchdog: Instant) {
    let mut reading: Vec<Reading> = Vec::new();
    let mut held: Vec<TcpStream> = Vec::new();

    while !shared.shutdown.load(Ordering::SeqCst) && Instant::now() < watchdog {
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(true);
                    reading.push(Reading {
                        stream,
                        buffer: Vec::new(),
                    });
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        let mut still_reading = Vec::new();
        for mut connection in reading.drain(..) {
            match read_available(&mut connection) {
                ReadState::Pending => still_reading.push(connection),
                ReadState::Closed => {}
                ReadState::Complete(request) => {
                    shared.requests.lock().unwrap().push(request);
                    shared.arrived.notify_all();
                    match mode {
                        ListenerMode::Reply(status) => respond(connection.stream, status),
                        ListenerMode::WithholdUntilReleased | ListenerMode::NeverReply => {
                            held.push(connection.stream);
                        }
                    }
                }
            }
        }
        reading = still_reading;

        if mode == ListenerMode::WithholdUntilReleased && shared.released.load(Ordering::SeqCst) {
            for stream in held.drain(..) {
                respond(stream, 200);
            }
        }

        std::thread::sleep(Duration::from_millis(5));
    }

    for stream in held {
        let _ = stream.shutdown(Shutdown::Both);
    }
}

enum ReadState {
    Pending,
    Closed,
    Complete(RecordedRequest),
}

fn read_available(connection: &mut Reading) -> ReadState {
    let mut chunk = [0_u8; 8192];
    loop {
        match connection.stream.read(&mut chunk) {
            Ok(0) => return ReadState::Closed,
            Ok(read) => connection.buffer.extend_from_slice(&chunk[..read]),
            Err(error) if error.kind() == ErrorKind::WouldBlock => break,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return ReadState::Closed,
        }
    }
    parse_request(&connection.buffer).map_or(ReadState::Pending, ReadState::Complete)
}

/// Parse a request once its headers and `Content-Length` body are buffered.
fn parse_request(buffer: &[u8]) -> Option<RecordedRequest> {
    let header_end = buffer.windows(4).position(|window| window == b"\r\n\r\n")? + 4;
    let head = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = head.lines();
    let mut request_line = lines.next()?.split_whitespace();
    let method = request_line.next()?.to_string();
    let path = request_line.next()?.to_string();
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let body = buffer.get(header_end..header_end + content_length)?;
    Some(RecordedRequest {
        method,
        path,
        body: String::from_utf8_lossy(body).into_owned(),
    })
}

/// Answer with `status`. A success carries the fields the messenger's
/// Discord webhook provider deserializes from a `wait=true` reply.
fn respond(mut stream: TcpStream, status: u16) {
    let body = if (200..300).contains(&status) {
        r#"{"id":"1","channel_id":"2","webhook_id":"1"}"#
    } else {
        r#"{"message":"rejected by test listener"}"#
    };
    let response = format!(
        "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.set_nonblocking(false);
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
}

/// Write an active, environment-backed Discord webhook route named
/// [`ROUTE_NAME`] into the fixture home's `~/.claudine/config.json`.
///
/// The route stores only [`WEBHOOK_URL_ENV`], never a URL, so the loopback
/// address never has to pass the inline Discord-host validator.
pub fn write_webhook_route(home: &Path) {
    write_config_with_webhook_route(home, serde_json::json!({}));
}

/// Write `config` plus the [`write_webhook_route`] route as the fixture home's
/// `~/.claudine/config.json`, for a test that also needs other keys such as
/// hook `actions`.
///
/// ## Panics
///
/// When `config` is not a JSON object or already has a `messenger` key.
pub fn write_config_with_webhook_route(home: &Path, mut config: serde_json::Value) {
    let object = config.as_object_mut().expect("config must be a JSON object");
    assert!(
        !object.contains_key("messenger"),
        "the webhook route owns the `messenger` key"
    );
    object.insert("messenger".to_string(), webhook_route_messenger());
    super::write_json(&home.join(".claudine").join("config.json"), &config);
}

fn webhook_route_messenger() -> serde_json::Value {
    serde_json::json!({
        "active_config": ROUTE_NAME,
        "configurations": {
            ROUTE_NAME: {
                "provider": "discord_webhook",
                "webhook_url_env": WEBHOOK_URL_ENV,
            }
        }
    })
}
