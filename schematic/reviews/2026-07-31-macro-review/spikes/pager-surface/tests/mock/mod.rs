//! A raw HTTP/1.1 mock server on its own thread and runtime. Unlike wiremock
//! it observes the client closing a connection while a reply is delayed,
//! which the drop-cancel and deadline checks need.

#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, watch};

#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub delay: Duration,
}

impl Reply {
    pub fn json(body: Value) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body: body.to_string(),
            delay: Duration::ZERO,
        }
    }
    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
    /// Never answers; the server waits for the client to go away.
    pub fn hang() -> Self {
        Self::json(Value::Null).delay(Duration::from_secs(3600))
    }
}

#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    /// Path and query.
    pub target: String,
    pub body: Option<Value>,
    pub at: Instant,
}

impl Recorded {
    pub fn query(&self, name: &str) -> Option<String> {
        let url = url::Url::parse(&format!("http://x{}", self.target)).ok()?;
        url.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.into_owned())
    }
    pub fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or("")
    }
}

#[derive(Debug, Default, Clone)]
pub struct State {
    pub requests: Vec<Recorded>,
    /// Request indexes whose connection the client closed before the reply.
    pub aborted: Vec<(usize, Instant)>,
}

type Handler = Arc<dyn Fn(usize, &Recorded) -> Reply + Send + Sync>;

pub struct MockServer {
    pub addr: SocketAddr,
    state: watch::Receiver<State>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    /// `handler(index, request)`: `index` counts requests across connections.
    pub fn start(handler: impl Fn(usize, &Recorded) -> Reply + Send + Sync + 'static) -> Self {
        let handler: Handler = Arc::new(handler);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = watch::channel(State::default());
        let (stop_tx, stop_rx) = oneshot::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(async move {
                let listener = TcpListener::from_std(listener).unwrap();
                let tx = Arc::new(tx);
                let accept = async {
                    loop {
                        let Ok((stream, _)) = listener.accept().await else { continue };
                        tokio::spawn(serve(stream, handler.clone(), tx.clone()));
                    }
                };
                tokio::select! {
                    _ = accept => {}
                    _ = stop_rx => {}
                }
            });
        });
        Self {
            addr,
            state: rx,
            stop: Some(stop_tx),
            thread: Some(thread),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn state(&self) -> State {
        self.state.borrow().clone()
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.state.borrow().requests.clone()
    }

    /// Waits for a condition on the server's state, up to `limit`.
    pub async fn wait_until(&self, limit: Duration, predicate: impl FnMut(&State) -> bool) -> bool {
        let mut rx = self.state.clone();
        tokio::time::timeout(limit, rx.wait_for(predicate)).await.is_ok()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn read_request(stream: &mut TcpStream, buf: &mut Vec<u8>) -> Option<(String, String, Option<Value>)> {
    let head_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        let mut chunk = [0u8; 4096];
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_owned();
    let target = first.next()?.to_owned();
    let length: usize = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0);
    while buf.len() < head_end + length {
        let mut chunk = [0u8; 4096];
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = (length > 0).then(|| serde_json::from_slice(&buf[head_end..head_end + length]).unwrap_or(Value::Null));
    buf.drain(..head_end + length);
    Some((method, target, body))
}

async fn serve(mut stream: TcpStream, handler: Handler, tx: Arc<watch::Sender<State>>) {
    let mut buf = Vec::new();
    while let Some((method, target, body)) = read_request(&mut stream, &mut buf).await {
        let recorded = Recorded {
            method,
            target,
            body,
            at: Instant::now(),
        };
        let mut index = 0;
        tx.send_modify(|s| {
            index = s.requests.len();
            s.requests.push(recorded.clone());
        });
        let reply = handler(index, &recorded);
        if !reply.delay.is_zero() {
            let mut probe = [0u8; 1];
            tokio::select! {
                _ = tokio::time::sleep(reply.delay) => {}
                read = stream.read(&mut probe) => {
                    if matches!(read, Ok(0) | Err(_)) {
                        tx.send_modify(|s| s.aborted.push((index, Instant::now())));
                        return;
                    }
                }
            }
        }
        let reason = if reply.status < 400 { "OK" } else { "ERR" };
        let mut out = format!("HTTP/1.1 {} {reason}\r\ncontent-length: {}\r\n", reply.status, reply.body.len());
        for (k, v) in &reply.headers {
            out.push_str(&format!("{k}: {v}\r\n"));
        }
        out.push_str("\r\n");
        out.push_str(&reply.body);
        if stream.write_all(out.as_bytes()).await.is_err() {
            return;
        }
    }
}
