//! A scriptable loopback WebSocket server for generated-client tests.
//!
//! [`MockWsServer::start`] binds `127.0.0.1:0` (never a fixed port, never a
//! non-loopback interface) and serves each accepted connection with the next
//! [`Script`]; once the scripts run out, the last one repeats. Every upgrade
//! request is recorded ([`RecordedRequest`]: path, query, headers) before
//! the script decides to accept, refuse, or redirect it, and every frame a
//! script receives is recorded with its connection index.
//!
//! [`MockWsServer::shutdown`] is deterministic: when it returns, the listener
//! is closed and every connection task has ended, so each client sees its
//! stream end. Dropping the server aborts the same tasks without waiting.
//!
//! [`FailingStream`] is the client-side half: a transport wrapper whose
//! writes fail once its [`FailSwitch`] is armed. A peer close does not make
//! the next local write fail reliably (on macOS the first write after a
//! peer's close returns `Ok`), so a test that needs a failed write injects
//! one instead.
//!
//! Nothing here needs a terminal or takes window focus; every test that uses
//! it is L1.

#![allow(dead_code, clippy::result_large_err)]

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, watch};
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::Message;

/// How one accepted TCP connection is served.
#[derive(Clone, Debug)]
pub enum Script {
    /// Complete the upgrade, then run `steps` in order. After the last step
    /// the connection stays open, reading and discarding frames, until the
    /// peer leaves or the server shuts down.
    Accept(Vec<Step>),
    /// Reject the upgrade with this HTTP status.
    Refuse(u16),
    /// Reject the upgrade with this 3xx status and a `Location` header.
    Redirect { status: u16, location: String },
    /// Reject the upgrade with this HTTP status and these response headers,
    /// appended in order (a name may repeat, e.g. `Set-Cookie`).
    RefuseWith { status: u16, headers: Vec<(String, String)> },
    /// Hold the TCP connection open without ever answering the upgrade.
    Stall,
    /// Close the TCP connection before reading the upgrade request.
    DropTcp,
}

impl Script {
    /// [`Script::Accept`] with no steps.
    pub fn accept() -> Self {
        Self::Accept(Vec::new())
    }
}

/// One action of an accepted connection's script.
#[derive(Clone, Debug)]
pub enum Step {
    /// Send this frame.
    Send(Message),
    /// Read and record this many data frames (text or binary); control
    /// frames are not counted.
    Receive(usize),
    /// Wait until the test calls [`Barrier::release`].
    Wait(Barrier),
    /// Do nothing until shutdown: no reads, no writes.
    Stall,
    /// Send a close frame and end the connection.
    Close,
    /// Drop the TCP connection without a close frame.
    Drop,
    /// Send a ping every `every` (the first at once) until `until` is
    /// released. Ping `n` (from 1) carries `n` as eight big-endian bytes.
    /// With `read`, frames are read meanwhile: pongs are recorded
    /// ([`MockWsServer::pongs`]) and data frames as [`Step::Receive`]
    /// records them. Without it nothing is read, so a peer's writes back
    /// up once the transport buffers fill.
    Heartbeat { every: Duration, until: Barrier, read: bool },
}

impl Step {
    /// [`Step::Send`] of a text frame.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Send(Message::Text(text.into().into()))
    }
}

/// A one-shot gate a script waits on until the test opens it.
///
/// Opening it before the script reaches it is not lost.
#[derive(Clone, Debug, Default)]
pub struct Barrier {
    opened: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl Barrier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the gate for every current and later waiter.
    pub fn release(&self) {
        self.opened.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    async fn wait(&self) {
        loop {
            let notified = self.notify.notified();
            if self.opened.load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}

/// The upgrade request a connection sent, recorded before its script ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedRequest {
    /// Zero-based index of the TCP connection.
    pub connection: usize,
    pub path: String,
    pub query: Option<String>,
    /// Header names lowercased, in received order.
    pub headers: Vec<(String, String)>,
}

impl RecordedRequest {
    /// The first value of header `name` (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Path plus `?query` when one was sent: the request target.
    pub fn target(&self) -> String {
        match &self.query {
            Some(query) => format!("{}?{query}", self.path),
            None => self.path.clone(),
        }
    }
}

#[derive(Default)]
struct Record {
    connections: usize,
    requests: Vec<RecordedRequest>,
    frames: Vec<(usize, Message)>,
    pongs: Vec<(usize, Vec<u8>)>,
}

struct Shared {
    scripts: Vec<Script>,
    record: Mutex<Record>,
    changed: watch::Sender<u64>,
}

impl Shared {
    fn update(&self, edit: impl FnOnce(&mut Record)) {
        edit(&mut self.record.lock().unwrap());
        self.changed.send_modify(|version| *version += 1);
    }

    fn script_for(&self, connection: usize) -> Script {
        let index = connection.min(self.scripts.len() - 1);
        self.scripts[index].clone()
    }
}

/// A running mock server; see the module docs.
pub struct MockWsServer {
    addr: SocketAddr,
    shared: Arc<Shared>,
    shutdown: watch::Sender<bool>,
    accept_task: Option<tokio::task::JoinHandle<()>>,
}

impl MockWsServer {
    /// Binds `127.0.0.1:0` and serves connection `n` with `scripts[n]`, or
    /// with the last script once they run out.
    ///
    /// ## Panics
    ///
    /// When `scripts` is empty or the loopback bind fails.
    pub async fn start(scripts: Vec<Script>) -> Self {
        assert!(!scripts.is_empty(), "a mock server needs at least one script");
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let (changed, _) = watch::channel(0);
        let shared = Arc::new(Shared {
            scripts,
            record: Mutex::new(Record::default()),
            changed,
        });
        let (shutdown, shutdown_rx) = watch::channel(false);
        let accept_task = tokio::spawn(accept_loop(listener, Arc::clone(&shared), shutdown_rx));
        Self {
            addr,
            shared,
            shutdown,
            accept_task: Some(accept_task),
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// `ws://127.0.0.1:<port>`, with no trailing slash.
    pub fn ws_origin(&self) -> String {
        format!("ws://{}", self.addr)
    }

    /// TCP connections accepted so far.
    pub fn connections(&self) -> usize {
        self.shared.record.lock().unwrap().connections
    }

    /// Upgrade requests recorded so far, in arrival order.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.shared.record.lock().unwrap().requests.clone()
    }

    /// Frames scripts received so far, with their connection index.
    pub fn frames(&self) -> Vec<(usize, Message)> {
        self.shared.record.lock().unwrap().frames.clone()
    }

    /// Pong payloads [`Step::Heartbeat`] received so far, with their
    /// connection index.
    pub fn pongs(&self) -> Vec<(usize, Vec<u8>)> {
        self.shared.record.lock().unwrap().pongs.clone()
    }

    /// Waits until `ready` holds for the record, or panics after `within`.
    async fn wait_until(&self, within: Duration, what: &str, ready: impl Fn(&Record) -> bool) {
        let mut changed = self.shared.changed.subscribe();
        let wait = async {
            loop {
                if ready(&self.shared.record.lock().unwrap()) {
                    return;
                }
                if changed.changed().await.is_err() {
                    return;
                }
            }
        };
        if tokio::time::timeout(within, wait).await.is_err() {
            panic!("mock server: timed out after {within:?} waiting for {what}");
        }
    }

    /// Waits for at least `count` accepted connections.
    pub async fn wait_for_connections(&self, count: usize, within: Duration) {
        self.wait_until(within, &format!("{count} connection(s)"), |r| r.connections >= count)
            .await;
    }

    /// Waits for at least `count` recorded upgrade requests.
    pub async fn wait_for_requests(&self, count: usize, within: Duration) {
        self.wait_until(within, &format!("{count} request(s)"), |r| r.requests.len() >= count)
            .await;
    }

    /// Waits for at least `count` recorded frames.
    pub async fn wait_for_frames(&self, count: usize, within: Duration) {
        self.wait_until(within, &format!("{count} frame(s)"), |r| r.frames.len() >= count)
            .await;
    }

    /// Waits for at least `count` recorded pongs.
    pub async fn wait_for_pongs(&self, count: usize, within: Duration) {
        self.wait_until(within, &format!("{count} pong(s)"), |r| r.pongs.len() >= count)
            .await;
    }

    /// Stops accepting, ends every connection, and returns once the listener
    /// is closed and every connection task has finished.
    pub async fn shutdown(mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.accept_task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for MockWsServer {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.accept_task.take() {
            task.abort();
        }
    }
}

async fn accept_loop(listener: TcpListener, shared: Arc<Shared>, mut shutdown: watch::Receiver<bool>) {
    // Owns every connection task: aborting or finishing this loop drops the
    // set, which aborts whatever is still running.
    let mut connections = JoinSet::new();
    let connection_shutdown = shutdown.clone();
    loop {
        tokio::select! {
            _ = shutdown.wait_for(|stop| *stop) => break,
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else { continue };
                let mut index = 0;
                shared.update(|record| {
                    index = record.connections;
                    record.connections += 1;
                });
                let shared = Arc::clone(&shared);
                connections.spawn(serve_connection(stream, index, shared, connection_shutdown.clone()));
            }
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
        }
    }
    drop(listener);
    // Connection tasks end on the same shutdown signal; wait for each.
    while connections.join_next().await.is_some() {}
}

async fn serve_connection(
    stream: TcpStream,
    index: usize,
    shared: Arc<Shared>,
    mut shutdown: watch::Receiver<bool>,
) {
    let script = shared.script_for(index);
    tokio::select! {
        _ = shutdown.wait_for(|stop| *stop) => {}
        () = run_script(stream, index, &shared, script) => {}
    }
}

async fn run_script(stream: TcpStream, index: usize, shared: &Shared, script: Script) {
    let (steps, rejection) = match script {
        Script::DropTcp => return,
        Script::Stall => {
            // Keep the socket open and unanswered until shutdown.
            let _stream = stream;
            std::future::pending::<()>().await;
            return;
        }
        Script::Accept(steps) => (steps, None),
        Script::Refuse(status) => (Vec::new(), Some((status, Vec::new()))),
        Script::Redirect { status, location } => {
            (Vec::new(), Some((status, vec![("location".to_string(), location)])))
        }
        Script::RefuseWith { status, headers } => (Vec::new(), Some((status, headers))),
    };

    let callback = |request: &Request, response: Response| {
        let uri = request.uri();
        let recorded = RecordedRequest {
            connection: index,
            path: uri.path().to_string(),
            query: uri.query().map(str::to_string),
            headers: request
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_ascii_lowercase(),
                        String::from_utf8_lossy(value.as_bytes()).into_owned(),
                    )
                })
                .collect(),
        };
        shared.update(|record| record.requests.push(recorded));
        match &rejection {
            None => Ok(response),
            Some((status, headers)) => Err(rejection_response(*status, headers)),
        }
    };

    // A rejected upgrade ends here: the callback's error response was sent.
    let Ok(mut ws) = tokio_tungstenite::accept_hdr_async(stream, callback).await else {
        return;
    };

    for step in steps {
        match step {
            Step::Send(message) => {
                if ws.send(message).await.is_err() {
                    return;
                }
            }
            Step::Receive(count) => {
                let mut received = 0;
                while received < count {
                    match ws.next().await {
                        Some(Ok(message @ (Message::Text(_) | Message::Binary(_)))) => {
                            shared.update(|record| record.frames.push((index, message)));
                            received += 1;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(_)) | None => return,
                    }
                }
            }
            Step::Wait(barrier) => barrier.wait().await,
            Step::Stall => std::future::pending::<()>().await,
            Step::Close => {
                let _ = ws.close(None).await;
                return;
            }
            Step::Drop => return,
            Step::Heartbeat { every, until, read } => {
                if !heartbeat(&mut ws, index, shared, every, &until, read).await {
                    return;
                }
            }
        }
    }

    while let Some(Ok(_)) = ws.next().await {}
}

/// Runs [`Step::Heartbeat`]; `false` once the connection has ended.
async fn heartbeat<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    index: usize,
    shared: &Shared,
    every: Duration,
    until: &Barrier,
    read: bool,
) -> bool
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let stop = until.wait();
    tokio::pin!(stop);
    let mut ticker = tokio::time::interval(every);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut sent: u64 = 0;
    loop {
        tokio::select! {
            () = &mut stop => return true,
            _ = ticker.tick() => {
                sent += 1;
                let payload = sent.to_be_bytes().to_vec();
                if ws.send(Message::Ping(payload.into())).await.is_err() {
                    return false;
                }
            }
            message = ws.next(), if read => match message {
                Some(Ok(Message::Pong(payload))) => {
                    shared.update(|record| record.pongs.push((index, payload.to_vec())));
                }
                Some(Ok(message @ (Message::Text(_) | Message::Binary(_)))) => {
                    shared.update(|record| record.frames.push((index, message)));
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return false,
            },
        }
    }
}

fn rejection_response(status: u16, headers: &[(String, String)]) -> ErrorResponse {
    let mut response = ErrorResponse::new(Some(format!("mock rejection {status}")));
    *response.status_mut() = StatusCode::from_u16(status).expect("valid HTTP status");
    for (name, value) in headers {
        response.headers_mut().append(
            HeaderName::from_bytes(name.as_bytes()).expect("valid header name"),
            HeaderValue::from_str(value).expect("valid header value"),
        );
    }
    response
}

/// Arms a [`FailingStream`]: once armed, every write fails with
/// [`io::ErrorKind::BrokenPipe`].
#[derive(Clone, Debug, Default)]
pub struct FailSwitch(Arc<AtomicBool>);

impl FailSwitch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arm(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn armed(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// A transport whose reads pass through and whose writes fail once its
/// [`FailSwitch`] is armed; see the module docs.
pub struct FailingStream<S> {
    inner: S,
    switch: FailSwitch,
}

impl<S> FailingStream<S> {
    pub fn new(inner: S, switch: FailSwitch) -> Self {
        Self { inner, switch }
    }
}

fn injected_failure() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "injected write failure")
}

impl<S: AsyncRead + Unpin> AsyncRead for FailingStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for FailingStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.switch.armed() {
            return Poll::Ready(Err(injected_failure()));
        }
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.switch.armed() {
            return Poll::Ready(Err(injected_failure()));
        }
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// The explicit "already authenticated" setup callback for a challenge
/// endpoint, whose only connect requires one. Tests that exercise transport
/// behavior rather than the challenge exchange pass it.
pub async fn no_setup<S>(_session: S) -> Result<(), schematic_schema::ws_shared::WsSetupError> {
    Ok(())
}
