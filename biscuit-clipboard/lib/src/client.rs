//! REST client for the running `clipper` service.
//!
//! [`ClipperClient`] is the single typed client used by `clip` and by
//! external integrators. It speaks the spec'd JSON contract over
//! `127.0.0.1:<port>` and surfaces typed errors via [`ClientError`].
//!
//! ## Examples
//!
//! ```no_run
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use biscuit_clipboard::ClipperClient;
//!
//! let mut client = ClipperClient::new()?;
//! client.ensure_running().await?;
//! let id = client.set_text("hello").await?;
//! println!("inserted: {id}");
//! # Ok(()) }
//! ```

use std::time::{Duration, Instant};

use eventsource_stream::Eventsource;
use futures_core::Stream;
use futures_util::StreamExt;
use tracing::debug;

use crate::api_types::{EntrySummary, HistoryResponse, SetResponse};
use crate::config;
use crate::entry::EntryId;

const HEADER_CLIPPER: &str = "x-clipper";

/// Errors surfaced by [`ClipperClient`] operations.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The local `clipper` service is not running and could not be
    /// auto-started within the spec'd timeout.
    #[error("service not running")]
    ServiceNotRunning,

    /// Network-level failure reaching the local service.
    #[error("connection failed: {0}")]
    Connection(String),

    /// I/O failure (file system, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Underlying HTTP error from `reqwest`.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Service responded with a non-success status or unexpected body.
    #[error("service error: {0}")]
    Service(String),

    /// Failure constructing the HTTP client itself (TLS init, etc.).
    /// Surfaced from [`ClipperClient::new`] so callers can observe it
    /// instead of crashing the process.
    #[error("client init failed: {0}")]
    ClientBuild(String),
}

/// Coarse health/state summary for the local `clipper` service.
#[derive(Debug)]
pub enum ServiceStatus {
    Running { pid: u32, port: u16 },
    Stopped,
}

/// Typed REST client for the local `clipper` service.
pub struct ClipperClient {
    client: reqwest::Client,
    base_url: Option<String>,
}

impl ClipperClient {
    /// Build a new client with the standard 10-second timeout.
    ///
    /// ## Errors
    ///
    /// Returns [`ClientError::ClientBuild`] when the underlying
    /// `reqwest::Client` builder fails (typically a TLS backend init
    /// failure). Replaces the previous panic'ing constructor — see
    /// review-1 Code Quality #13.
    pub fn new() -> Result<Self, ClientError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ClientError::ClientBuild(e.to_string()))?;
        Ok(Self {
            client,
            base_url: None,
        })
    }

    pub async fn ensure_running(&mut self) -> Result<(), ClientError> {
        if self.try_connect().await.is_ok() {
            return Ok(());
        }

        self.spawn_service()?;
        self.poll_until_ready().await
    }

    pub async fn get_current(&self) -> Result<Option<String>, ClientError> {
        self.get_current_with_format(None, None).await
    }

    /// Read the current clipboard content honouring an optional format
    /// and encoding query, mirroring the spec's `GET /current` query
    /// parameters.
    ///
    /// ## Notes
    ///
    /// `/current` returns `204 No Content` when the OS clipboard is
    /// empty — mapped to `Ok(None)` here.
    pub async fn get_current_with_format(
        &self,
        format: Option<&str>,
        encoding: Option<&str>,
    ) -> Result<Option<String>, ClientError> {
        let mut url = format!("{}/current", self.base_url());
        let mut sep = '?';
        if let Some(fmt) = format {
            url.push(sep);
            url.push_str(&format!("format={fmt}"));
            sep = '&';
        }
        if let Some(enc) = encoding {
            url.push(sep);
            url.push_str(&format!("encoding={enc}"));
        }
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NO_CONTENT
            || resp.status() == reqwest::StatusCode::NOT_FOUND
        {
            return Ok(None);
        }

        if resp.status() == reqwest::StatusCode::NOT_ACCEPTABLE {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Service(format!(
                "format not available: {body}"
            )));
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("text/plain") {
            let text = resp.text().await?;
            Ok(Some(text))
        } else if content_type.contains("application/json") {
            let summary: EntrySummary = resp.json().await?;
            Ok(Some(summary.preview))
        } else {
            // Binary path — return the count as a placeholder; callers
            // expecting bytes should call get_current_bytes instead.
            let bytes = resp.bytes().await?;
            Ok(Some(format!("<{} bytes>", bytes.len())))
        }
    }

    /// Fetch a specific entry's content with optional format/encoding
    /// query parameters. Returns `None` for 404 / 204; surfaces
    /// `format_not_available` (406) as a `Service` error.
    pub async fn get_content_with_format(
        &self,
        id: &str,
        format: Option<&str>,
        encoding: Option<&str>,
    ) -> Result<Option<String>, ClientError> {
        let mut url = format!("{}/history/{}/content", self.base_url(), id);
        let mut sep = '?';
        if let Some(fmt) = format {
            url.push(sep);
            url.push_str(&format!("format={fmt}"));
            sep = '&';
        }
        if let Some(enc) = encoding {
            url.push(sep);
            url.push_str(&format!("encoding={enc}"));
        }
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND
            || resp.status() == reqwest::StatusCode::NO_CONTENT
        {
            return Ok(None);
        }
        if resp.status() == reqwest::StatusCode::NOT_ACCEPTABLE {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Service(format!(
                "format not available: {body}"
            )));
        }
        let text = resp.text().await?;
        Ok(Some(text))
    }

    pub async fn set_text(&self, text: &str) -> Result<EntryId, ClientError> {
        let url = format!("{}/set", self.base_url());
        let body = serde_json::json!({ "content_type": "text", "data": text });
        let resp = self
            .client
            .post(&url)
            .header(HEADER_CLIPPER, "1")
            .json(&body)
            .send()
            .await?;

        let result: SetResponse = resp.json().await?;
        Ok(result.id)
    }

    pub async fn get_history(&self) -> Result<Vec<EntrySummary>, ClientError> {
        let url = format!("{}/history", self.base_url());
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;
        let history: HistoryResponse = resp.json().await?;
        Ok(history.entries)
    }

    pub async fn get_latest(&self) -> Result<EntrySummary, ClientError> {
        let url = format!("{}/history/latest", self.base_url());
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ClientError::Service("no entries".to_string()));
        }

        resp.json().await.map_err(ClientError::Http)
    }

    pub async fn clear_history(&self) -> Result<(), ClientError> {
        let url = format!("{}/history", self.base_url());
        self.client
            .delete(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;
        Ok(())
    }

    /// Empty the OS clipboard via `POST /clear`.
    ///
    /// ## Errors
    ///
    /// Returns [`ClientError::Service`] when the server responds with a
    /// non-2xx status.
    pub async fn clear_clipboard(&self) -> Result<(), ClientError> {
        let url = format!("{}/clear", self.base_url());
        let resp = self
            .client
            .post(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Service(format!(
                "clear_clipboard failed: {status} {body}"
            )));
        }
        Ok(())
    }

    /// Subscribe to clipboard change events via `GET /events` (SSE).
    ///
    /// Each yielded item is the JSON-decoded [`EntrySummary`] of a new
    /// clipboard entry. The stream terminates when the underlying HTTP
    /// connection closes (server shutdown, network drop, etc.).
    ///
    /// ## Errors
    ///
    /// Returns [`ClientError::Http`] if the initial HTTP handshake
    /// fails. Per-message decode failures are surfaced as stream items
    /// of type `Result<EntrySummary, ClientError>`.
    pub async fn events_stream(
        &self,
    ) -> Result<impl Stream<Item = Result<EntrySummary, ClientError>>, ClientError> {
        let url = format!("{}/events", self.base_url());
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .header("accept", "text/event-stream")
            .send()
            .await?;
        let stream = resp.bytes_stream().eventsource().map(|item| match item {
            Ok(event) => serde_json::from_str::<EntrySummary>(&event.data)
                .map_err(|e| ClientError::Service(format!("malformed SSE payload: {e}"))),
            Err(e) => Err(ClientError::Service(format!("SSE error: {e}"))),
        });
        Ok(stream)
    }

    pub async fn stop_service(&self) -> Result<(), ClientError> {
        let pid = config::read_pid_file()
            .ok_or_else(|| ClientError::Service("no PID file found".to_string()))?;

        if !config::is_pid_alive(pid) {
            return Err(ClientError::Service("service not running".to_string()));
        }

        #[cfg(unix)]
        {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }

        #[cfg(not(unix))]
        {
            let _ = pid;
        }

        if let Ok(dir) = config::runtime_dir() {
            let _ = std::fs::remove_file(dir.join(config::PID_FILENAME));
            let _ = std::fs::remove_file(dir.join(config::PORT_FILENAME));
        }

        Ok(())
    }

    pub fn service_status(&self) -> ServiceStatus {
        match (config::read_pid_file(), config::read_port_file()) {
            (Some(pid), Some(port)) => {
                if config::is_pid_alive(pid) {
                    ServiceStatus::Running { pid, port }
                } else {
                    ServiceStatus::Stopped
                }
            }
            _ => ServiceStatus::Stopped,
        }
    }

    async fn try_connect(&mut self) -> Result<(), ClientError> {
        let port = config::read_port_file().ok_or(ClientError::ServiceNotRunning)?;

        let pid = config::read_pid_file().ok_or(ClientError::ServiceNotRunning)?;

        if !config::is_pid_alive(pid) {
            return Err(ClientError::ServiceNotRunning);
        }

        self.base_url = Some(format!("http://127.0.0.1:{port}"));
        self.health_check().await
    }

    async fn health_check(&self) -> Result<(), ClientError> {
        let url = format!("{}/health", self.base_url());
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await
            .map_err(|e| ClientError::Connection(e.to_string()))?;

        let has_fingerprint = resp.headers().get(HEADER_CLIPPER).is_some_and(|v| v == "1");

        if !has_fingerprint {
            return Err(ClientError::ServiceNotRunning);
        }

        resp.error_for_status_ref().map_err(ClientError::Http)?;

        Ok(())
    }

    fn spawn_service(&self) -> Result<(), ClientError> {
        let port = config::configured_port();

        let binary = find_clipper_binary();

        std::process::Command::new(&binary)
            .arg("--port")
            .arg(port.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                ClientError::Service(format!("Failed to start clipper ({binary}): {e}"))
            })?;

        Ok(())
    }

    async fn poll_until_ready(&mut self) -> Result<(), ClientError> {
        let start = Instant::now();
        let mut delay = Duration::from_millis(50);
        let max_delay = Duration::from_millis(500);
        let timeout = Duration::from_secs(5);

        while start.elapsed() < timeout {
            tokio::time::sleep(delay).await;
            delay = delay.saturating_mul(2).min(max_delay);

            if let Some(port) = config::read_port_file() {
                self.base_url = Some(format!("http://127.0.0.1:{port}"));
                if self.health_check().await.is_ok() {
                    return Ok(());
                }
            }
        }

        Err(ClientError::ServiceNotRunning)
    }

    fn base_url(&self) -> &str {
        self.base_url.as_deref().unwrap_or("")
    }
}

/// Locate the `clipper` binary used to auto-start the service.
///
/// ## Notes
///
/// Searches the directory of the current executable first (so that
/// `clip` paired with a sibling `clipper` works without a `PATH`
/// dependency), then falls back to the bare string `"clipper"` for
/// `PATH` lookup. Logs the chosen path at `debug` level so operators
/// running with `RUST_LOG=biscuit_clipboard=debug` can see which binary
/// will be invoked — review-1 Code Quality #12.
pub fn find_clipper_binary() -> String {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let clipper = dir.join("clipper");
        if clipper.exists() {
            let path = clipper.to_string_lossy().into_owned();
            debug!(path = %path, "find_clipper_binary: using sibling binary");
            return path;
        }
    }
    debug!("find_clipper_binary: falling back to PATH lookup for 'clipper'");
    "clipper".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let _client = ClipperClient::new().expect("client must build");
    }

    #[test]
    fn test_service_status_stopped() {
        let client = ClipperClient::new().expect("client must build");
        // Tests run with no clipper service installed, so we expect
        // `Stopped`. The override env var lets the assert hold even on
        // a developer machine running a real `clipper`.
        let tmp = tempfile::tempdir().unwrap();
        let prev = std::env::var_os(config::CLIP_RUNTIME_DIR_ENV);
        // SAFETY: scoped env mutation; restored below.
        unsafe { std::env::set_var(config::CLIP_RUNTIME_DIR_ENV, tmp.path()) };
        let status = client.service_status();
        unsafe {
            match prev {
                Some(v) => std::env::set_var(config::CLIP_RUNTIME_DIR_ENV, v),
                None => std::env::remove_var(config::CLIP_RUNTIME_DIR_ENV),
            }
        }
        assert!(matches!(status, ServiceStatus::Stopped));
    }

    #[test]
    fn test_client_error_construction() {
        // Smoke test that ClientError::ClientBuild is constructable from
        // a plain error message — exercising the typed error path that
        // replaced the old `expect("...")` panic.
        let err = ClientError::ClientBuild("synthetic".to_string());
        let rendered = format!("{err}");
        assert!(rendered.contains("synthetic"));
    }

    #[test]
    fn test_entry_summary_serialization() {
        let entry = EntrySummary {
            id: EntryId::new("a1b2c3d4e5f60718"),
            timestamp: "2026-05-02T14:32:01Z".to_string(),
            content_type: "text".to_string(),
            preview: "Hello world".to_string(),
            size_bytes: 42,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: EntrySummary = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, deserialized.id);
        assert_eq!(entry.preview, deserialized.preview);
    }

    #[test]
    fn test_find_clipper_binary_returns_string() {
        let binary = find_clipper_binary();
        assert!(!binary.is_empty());
    }

    /// Spin up a tiny `hyper` server that records the path of the most
    /// recent request, so we can assert the client wired the
    /// `?format=...` and `?encoding=...` query parameters through.
    async fn echo_path_server() -> (
        std::net::SocketAddr,
        std::sync::Arc<std::sync::Mutex<Vec<String>>>,
        tokio::task::JoinHandle<()>,
    ) {
        use std::sync::{Arc, Mutex};

        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let recorded_clone = recorded.clone();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let recorded = recorded_clone.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = vec![0u8; 4096];
                    let n = match socket.read(&mut buf).await {
                        Ok(n) => n,
                        Err(_) => return,
                    };
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    if let Some(line) = req.lines().next() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            recorded.lock().unwrap().push(parts[1].to_string());
                        }
                    }
                    let body = "no content";
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; \
                         charset=utf-8\r\nContent-Length: {}\r\nx-clipper: 1\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                });
            }
        });

        (addr, recorded, handle)
    }

    #[tokio::test]
    async fn test_get_current_with_format_forwards_query_params() {
        let (addr, recorded, handle) = echo_path_server().await;
        let mut client = ClipperClient::new().unwrap();
        client.base_url = Some(format!("http://{addr}"));

        let _ = client
            .get_current_with_format(Some("html"), Some("base64"))
            .await
            .unwrap();

        // Give the server a moment to record before we assert.
        tokio::time::sleep(Duration::from_millis(20)).await;
        let paths = recorded.lock().unwrap().clone();
        assert!(
            paths.iter().any(|p| p.contains("format=html")),
            "expected format=html in recorded paths, got {paths:?}"
        );
        assert!(
            paths.iter().any(|p| p.contains("encoding=base64")),
            "expected encoding=base64 in recorded paths, got {paths:?}"
        );
        handle.abort();
    }

    #[tokio::test]
    async fn test_get_content_with_format_forwards_query_params() {
        let (addr, recorded, handle) = echo_path_server().await;
        let mut client = ClipperClient::new().unwrap();
        client.base_url = Some(format!("http://{addr}"));

        let _ = client
            .get_content_with_format("abc123", Some("rtf"), None)
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;
        let paths = recorded.lock().unwrap().clone();
        assert!(
            paths
                .iter()
                .any(|p| p.contains("/history/abc123/content") && p.contains("format=rtf")),
            "expected /history/abc123/content?format=rtf, got {paths:?}"
        );
        handle.abort();
    }

    #[tokio::test]
    async fn test_clear_history_calls_delete_history() {
        let (addr, recorded, handle) = echo_path_server().await;
        let client = ClipperClient {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            base_url: Some(format!("http://{addr}")),
        };

        client.clear_history().await.unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;
        let paths = recorded.lock().unwrap().clone();
        assert!(
            paths.iter().any(|p| p == "/history"),
            "expected DELETE /history, got {paths:?}"
        );
        handle.abort();
    }

    #[tokio::test]
    async fn test_clear_clipboard_calls_post_clear() {
        let (addr, recorded, handle) = echo_path_server().await;
        let client = ClipperClient {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            base_url: Some(format!("http://{addr}")),
        };

        client.clear_clipboard().await.unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;
        let paths = recorded.lock().unwrap().clone();
        assert!(
            paths.iter().any(|p| p == "/clear"),
            "expected POST /clear, got {paths:?}"
        );
        handle.abort();
    }

    // -------------------------------------------------------------------
    // Phase 6 / Test Coverage #7: ClipperClient HTTP behavior with wiremock.
    //
    // These tests verify the HTTP behaviors documented in the spec —
    // X-Clipper fingerprint enforcement, error-envelope handling, and
    // the auto-start retry loop — using wiremock as a mock server.
    // -------------------------------------------------------------------

    /// `health_check` happy path: when the mock server responds `200`
    /// with `X-Clipper: 1`, the call returns Ok.
    #[tokio::test]
    async fn test_health_check_succeeds_with_x_clipper_header() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .and(header("x-clipper", "1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-clipper", "1")
                    .set_body_json(serde_json::json!({
                        "status": "ok",
                        "entries": 0,
                        "watcher": "running",
                    })),
            )
            .mount(&server)
            .await;

        let mut client = ClipperClient::new().expect("client must build");
        client.base_url = Some(server.uri());
        let result = client.health_check().await;
        assert!(
            result.is_ok(),
            "health_check should succeed with X-Clipper header, got: {result:?}",
        );
    }

    /// Fingerprint enforcement: when the mock server's response omits
    /// `X-Clipper`, the client must reject the connection with
    /// `ServiceNotRunning` — preventing accidental cross-talk with an
    /// unrelated process listening on the configured port.
    #[tokio::test]
    async fn test_health_check_rejects_missing_x_clipper_header() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "entries": 0,
                "watcher": "running",
            })))
            .mount(&server)
            .await;

        let mut client = ClipperClient::new().expect("client must build");
        client.base_url = Some(server.uri());
        let result = client.health_check().await;
        assert!(
            matches!(result, Err(ClientError::ServiceNotRunning)),
            "expected ServiceNotRunning when X-Clipper missing, got: {result:?}",
        );
    }

    /// Error-envelope deserialization: a 404 with the spec'd error body
    /// surfaces as a typed `ClientError::Service`. The client does not
    /// silently swallow non-2xx responses on `/history/latest`.
    #[tokio::test]
    async fn test_get_latest_surfaces_404_error_envelope() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/history/latest"))
            .respond_with(
                ResponseTemplate::new(404)
                    .insert_header("x-clipper", "1")
                    .set_body_json(serde_json::json!({
                        "error": {
                            "code": "entry_not_found",
                            "message": "No history entry with id 'latest'",
                        },
                    })),
            )
            .mount(&server)
            .await;

        let mut client = ClipperClient::new().expect("client must build");
        client.base_url = Some(server.uri());
        let result = client.get_latest().await;
        match result {
            Err(ClientError::Service(msg)) => {
                assert!(
                    msg.contains("no entries") || msg.contains("entry_not_found"),
                    "unexpected message: {msg}",
                );
            }
            other => panic!("expected Service error, got {other:?}"),
        }
    }

    /// Auto-start retry/backoff: simulate a server that 404s the first
    /// few `/health` probes, then succeeds. `poll_until_ready` must keep
    /// polling and eventually return Ok within its 5s budget.
    #[tokio::test]
    #[serial_test::serial]
    async fn test_poll_until_ready_retries_until_health_succeeds() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

        struct FlakyHealth {
            counter: AtomicUsize,
        }
        impl Respond for FlakyHealth {
            fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                // First two probes: 503 with no fingerprint. Third: success.
                if n < 2 {
                    ResponseTemplate::new(503)
                } else {
                    ResponseTemplate::new(200)
                        .insert_header("x-clipper", "1")
                        .set_body_json(serde_json::json!({
                            "status": "ok",
                            "entries": 0,
                            "watcher": "running",
                        }))
                }
            }
        }

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(FlakyHealth {
                counter: AtomicUsize::new(0),
            })
            .mount(&server)
            .await;

        // Write the port file so `poll_until_ready` finds the mock server.
        let tmp = tempfile::tempdir().unwrap();
        let port = server.address().port();
        std::fs::write(tmp.path().join(config::PORT_FILENAME), port.to_string()).unwrap();

        let prev_runtime = std::env::var_os(config::CLIP_RUNTIME_DIR_ENV);
        // SAFETY: env mutation serialized via serial_test.
        unsafe { std::env::set_var(config::CLIP_RUNTIME_DIR_ENV, tmp.path()) };

        // Replace 127.0.0.1 binding because wiremock binds to 127.0.0.1 by default.
        let mut client = ClipperClient::new().expect("client must build");
        let result = client.poll_until_ready().await;

        unsafe {
            match prev_runtime {
                Some(v) => std::env::set_var(config::CLIP_RUNTIME_DIR_ENV, v),
                None => std::env::remove_var(config::CLIP_RUNTIME_DIR_ENV),
            }
        }

        assert!(
            result.is_ok(),
            "poll_until_ready should retry until success, got: {result:?}",
        );
    }
}
