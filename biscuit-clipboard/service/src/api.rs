use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use biscuit_clipboard::api_types::{
    ContentQuery, Encoding, EntrySummary, ErrorBody, ErrorCode, ErrorResponse, HealthResponse,
    HistoryQuery, HistoryResponse, SetRequest, SetResponse, content_type_str,
};
use biscuit_clipboard::backend::ClipboardBackend;
use biscuit_clipboard::content::{ClipboardFormat, ContentType, ImageSnapshot};
use biscuit_clipboard::entry::{ClipboardEntry, EntryId, FORMAT_PRIORITY as ENTRY_FORMAT_PRIORITY};
use biscuit_clipboard::{History, Storage, SupervisorStatus};
use futures_util::stream::{Stream, StreamExt};
use tokio::sync::{Mutex, RwLock, broadcast};
use tokio_stream::wrappers::BroadcastStream;

const HEADER_CLIPPER: &str = "x-clipper";

/// Format priority used by [`ClipboardEntry::primary_content_type`] and
/// `/history/:id/content` default-format selection.
///
/// This is a re-export of [`biscuit_clipboard::entry::FORMAT_PRIORITY`]
/// so the lib and service crate can import it under a single name.
/// One source of truth — see review-1 Code Quality #5.
pub const FORMAT_PRIORITY: [ContentType; 5] = ENTRY_FORMAT_PRIORITY;

pub type SharedState = Arc<AppState>;

/// Capacity of the broadcast channel that fans `EntrySummary` events
/// out to SSE subscribers. A modest buffer absorbs short SSE-consumer
/// stalls; subscribers that lag past this many events receive a
/// `Lagged` notification (which the `BroadcastStream` adapter folds
/// into a single dropped item).
pub const EVENTS_CHANNEL_CAPACITY: usize = 64;

pub struct AppState {
    pub history: Mutex<History>,
    pub storage: Storage,
    /// Live clipboard backend used by `/current` for at-request reads.
    /// Wrapped in `Arc<dyn>` so tests can swap in `MockClipboard`.
    pub clipboard: Arc<dyn ClipboardBackend + Send + Sync>,
    /// Supervisor-reported watcher health. Updated by the supervisor task
    /// in `main.rs`, read by `/health` and gating handlers as needed.
    pub watcher_status: Arc<RwLock<SupervisorStatus>>,
    /// Broadcasts every successfully-inserted clipboard entry to SSE
    /// subscribers on `GET /events`. The watcher-supervisor task in
    /// `main.rs` pushes onto this channel; `set_content` also pushes
    /// for entries created via `POST /set`.
    pub events_tx: broadcast::Sender<EntrySummary>,
}

impl AppState {
    /// Construct a new, empty events broadcast channel sized at
    /// [`EVENTS_CHANNEL_CAPACITY`].
    pub fn new_events_channel() -> broadcast::Sender<EntrySummary> {
        let (tx, _rx) = broadcast::channel(EVENTS_CHANNEL_CAPACITY);
        tx
    }
}

fn clipper_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(HEADER_CLIPPER, HeaderValue::from_static("1"));
    headers
}

fn json_response(status: StatusCode, body: impl serde::Serialize) -> Response {
    let mut headers = clipper_headers();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let json = serde_json::to_string(&body).unwrap_or_default();
    (status, headers, json).into_response()
}

fn text_response(status: StatusCode, text: String) -> Response {
    let mut headers = clipper_headers();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    (status, headers, text).into_response()
}

fn binary_response(status: StatusCode, content_type: &'static str, data: Vec<u8>) -> Response {
    let mut headers = clipper_headers();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    (status, headers, data).into_response()
}

fn empty_response(status: StatusCode) -> Response {
    let headers = clipper_headers();
    (status, headers).into_response()
}

fn error_envelope(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Response {
    json_response(
        status,
        ErrorResponse {
            error: ErrorBody {
                code,
                message: message.into(),
            },
        },
    )
}

fn entry_not_found(id: &str) -> Response {
    error_envelope(
        StatusCode::NOT_FOUND,
        ErrorCode::EntryNotFound,
        format!("No history entry with id '{id}'"),
    )
}

fn bad_request(message: impl Into<String>) -> Response {
    error_envelope(StatusCode::BAD_REQUEST, ErrorCode::BadRequest, message)
}

fn format_not_available(format: ContentType) -> Response {
    error_envelope(
        StatusCode::NOT_ACCEPTABLE,
        ErrorCode::FormatNotAvailable,
        format!(
            "Requested format '{}' is not available for this entry",
            content_type_str(format)
        ),
    )
}

fn internal(message: impl Into<String>) -> Response {
    error_envelope(
        StatusCode::INTERNAL_SERVER_ERROR,
        ErrorCode::Internal,
        message,
    )
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/history", get(get_history).delete(delete_history))
        .route("/history/latest", get(get_latest))
        .route("/history/{id}", get(get_entry))
        .route("/history/{id}/content", get(get_content))
        .route("/history/{id}/thumbnail", get(get_thumbnail))
        .route("/current", get(get_current))
        .route("/set", post(set_content))
        .route("/clear", post(clear_clipboard))
        .route("/events", get(events_stream))
        .with_state(state)
}

async fn health(State(state): State<SharedState>) -> Response {
    let count = state.history.lock().await.len();
    let watcher = state.watcher_status.read().await.clone();
    let healthy = matches!(watcher, SupervisorStatus::Running);

    if healthy {
        json_response(
            StatusCode::OK,
            HealthResponse {
                status: "ok".to_string(),
                entries: count,
                watcher: "running".to_string(),
            },
        )
    } else {
        // Spec contract: degraded watcher → 503 + service_unavailable
        // envelope. Primary signal is the HTTP status; the body uses the
        // documented error envelope.
        error_envelope(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ServiceUnavailable,
            format!("watcher is {}", watcher_label(&watcher)),
        )
    }
}

fn watcher_label(status: &SupervisorStatus) -> &'static str {
    match status {
        SupervisorStatus::Running => "running",
        SupervisorStatus::Degraded { .. } => "degraded",
        SupervisorStatus::Stopped => "stopped",
    }
}

async fn get_history(
    State(state): State<SharedState>,
    Query(params): Query<HistoryQuery>,
) -> Response {
    let history = state.history.lock().await;
    let entries: Vec<&ClipboardEntry> = match params.since {
        Some(ts) => history.since(ts),
        None => history.all(),
    };
    let truncated: Vec<EntrySummary> = match params.limit {
        Some(limit) => entries
            .into_iter()
            .take(limit)
            .map(EntrySummary::from)
            .collect(),
        None => entries.into_iter().map(EntrySummary::from).collect(),
    };
    json_response(StatusCode::OK, HistoryResponse { entries: truncated })
}

async fn delete_history(State(state): State<SharedState>) -> Response {
    state.history.lock().await.clear();
    empty_response(StatusCode::NO_CONTENT)
}

async fn get_latest(State(state): State<SharedState>) -> Response {
    let history = state.history.lock().await;
    match history.latest() {
        Some(entry) => json_response(StatusCode::OK, EntrySummary::from(entry)),
        None => entry_not_found("latest"),
    }
}

async fn get_entry(State(state): State<SharedState>, Path(id): Path<String>) -> Response {
    let history = state.history.lock().await;
    let lookup = EntryId::from(id.clone());
    match history.get(&lookup) {
        Some(entry) => json_response(StatusCode::OK, EntrySummary::from(entry)),
        None => entry_not_found(&id),
    }
}

/// Pick the "primary" format from an entry following the documented
/// priority `text > html > rtf > image > files`.
fn select_format(
    entry: &ClipboardEntry,
    requested: Option<ContentType>,
) -> Option<&ClipboardFormat> {
    if let Some(target) = requested {
        return entry.find_format(target);
    }
    for ct in &FORMAT_PRIORITY {
        if let Some(f) = entry.find_format(*ct) {
            return Some(f);
        }
    }
    None
}

async fn get_content(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(params): Query<ContentQuery>,
) -> Response {
    let history = state.history.lock().await;
    let lookup = EntryId::from(id.clone());
    let entry = match history.get(&lookup) {
        Some(e) => e,
        None => return entry_not_found(&id),
    };

    let format = match select_format(entry, params.format) {
        Some(f) => f,
        None => {
            return match params.format {
                Some(fmt) => format_not_available(fmt),
                None => entry_not_found(&id),
            };
        }
    };

    let encode = matches!(params.encoding, Some(Encoding::Base64));
    render_format(&state.storage, format, encode)
}

async fn get_thumbnail(State(state): State<SharedState>, Path(id): Path<String>) -> Response {
    let history = state.history.lock().await;
    let lookup = EntryId::from(id.clone());
    let entry = match history.get(&lookup) {
        Some(e) => e,
        None => return entry_not_found(&id),
    };
    let img = match entry.find_image() {
        Some(i) => i,
        None => return format_not_available(ContentType::Image),
    };
    match state.storage.load_spilled(img) {
        Ok(data) => binary_response(StatusCode::OK, "image/png", data),
        Err(e) => internal(format!("Failed to load thumbnail: {e}")),
    }
}

/// Render a [`ClipboardFormat`] as the body of `/history/:id/content`.
/// When `encode_base64` is true, binary outputs (and text) are base64-
/// encoded and returned as `text/plain; charset=utf-8`. Text formats
/// default to plain text; images default to `image/png` bytes.
fn render_format(storage: &Storage, format: &ClipboardFormat, encode_base64: bool) -> Response {
    match format {
        ClipboardFormat::Text(s) | ClipboardFormat::Html(s) | ClipboardFormat::Rtf(s) => {
            if encode_base64 {
                text_response(StatusCode::OK, BASE64_STANDARD.encode(s.as_bytes()))
            } else {
                text_response(StatusCode::OK, s.clone())
            }
        }
        ClipboardFormat::Image(img) => match storage.load_spilled(img) {
            Ok(bytes) => {
                if encode_base64 {
                    text_response(StatusCode::OK, BASE64_STANDARD.encode(&bytes))
                } else {
                    binary_response(StatusCode::OK, "image/png", bytes)
                }
            }
            Err(e) => internal(format!("Failed to load image: {e}")),
        },
        ClipboardFormat::Files(paths) => {
            let payload = serde_json::json!({
                "files": paths.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>()
            });
            if encode_base64 {
                let serialized = serde_json::to_vec(&payload).unwrap_or_default();
                text_response(StatusCode::OK, BASE64_STANDARD.encode(&serialized))
            } else {
                json_response(StatusCode::OK, payload)
            }
        }
    }
}

/// `GET /current` performs a live read against the OS clipboard and
/// returns an entry-shaped body with `id: "current"`. It does **not**
/// insert into history.
///
/// When `?format=` or `?encoding=` query parameters are present, the
/// handler returns the rendered format directly (mirroring
/// `/history/:id/content` behaviour) rather than the `EntrySummary`
/// JSON.
async fn get_current(
    State(state): State<SharedState>,
    Query(params): Query<ContentQuery>,
) -> Response {
    let backend = state.clipboard.clone();

    let formats = match tokio::task::spawn_blocking(move || capture_current(backend.as_ref())).await
    {
        Ok(Ok(formats)) => formats,
        Ok(Err(e)) => return internal(format!("clipboard read failed: {e}")),
        Err(e) => return internal(format!("clipboard task panicked: {e}")),
    };

    if formats.is_empty() {
        return empty_response(StatusCode::NO_CONTENT);
    }

    // If format or encoding is requested, render the specific format
    // directly rather than returning the entry summary JSON.
    if params.format.is_some() || params.encoding.is_some() {
        let entry = ClipboardEntry::new(formats);
        let format = match select_format(&entry, params.format) {
            Some(f) => f,
            None => {
                return match params.format {
                    Some(fmt) => format_not_available(fmt),
                    None => {
                        // formats is not empty (we checked above), so
                        // this path is theoretically unreachable.
                        return internal("no format available for current clipboard");
                    }
                };
            }
        };
        let encode = matches!(params.encoding, Some(Encoding::Base64));
        return render_format(&state.storage, format, encode);
    }

    let entry = ClipboardEntry::new(formats);
    let mut summary = EntrySummary::from(&entry);
    summary.id = EntryId::new("current");
    json_response(StatusCode::OK, summary)
}

/// Synchronously capture every available format from the live backend.
fn capture_current(
    backend: &(dyn ClipboardBackend + Send + Sync),
) -> Result<Vec<ClipboardFormat>, String> {
    let mut out = Vec::new();
    if let Some(t) = backend.get_text().map_err(|e| e.to_string())? {
        out.push(ClipboardFormat::Text(t));
    }
    if let Some(h) = backend.get_html().map_err(|e| e.to_string())? {
        out.push(ClipboardFormat::Html(h));
    }
    if let Some(r) = backend.get_rtf().map_err(|e| e.to_string())? {
        out.push(ClipboardFormat::Rtf(r));
    }
    if let Some(img) = backend.get_image().map_err(|e| e.to_string())? {
        out.push(ClipboardFormat::Image(img));
    }
    if let Some(files) = backend.get_files().map_err(|e| e.to_string())? {
        out.push(ClipboardFormat::Files(files));
    }
    Ok(out)
}

async fn set_content(
    State(state): State<SharedState>,
    body: Result<Json<SetRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(req) = match body {
        Ok(j) => j,
        Err(rej) => {
            return bad_request(format!("invalid set request body: {rej}"));
        }
    };

    let format = match build_format_from_request(&state.storage, req) {
        Ok(f) => f,
        Err(BuildFormatError::InvalidBase64(msg)) => return bad_request(msg),
    };

    let backend = state.clipboard.clone();
    let storage = state.storage.clone();
    let format_for_write = format.clone();
    let result = tokio::task::spawn_blocking(move || {
        write_format_to_backend(&storage, backend.as_ref(), &format_for_write)
    })
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return internal(format!("clipboard write failed: {e}")),
        Err(e) => return internal(format!("clipboard write task panicked: {e}")),
    }

    let mut history = state.history.lock().await;
    let (id, broadcast_payload) = match history.insert(vec![format]) {
        Some(entry) => (entry.id.clone(), Some(EntrySummary::from(entry))),
        None => (EntryId::new("duplicate"), None),
    };
    drop(history);

    if let Some(summary) = broadcast_payload {
        // Send is best-effort: if there are no subscribers (no active
        // SSE clients) we deliberately drop the event.
        let _ = state.events_tx.send(summary);
    }

    json_response(StatusCode::OK, SetResponse { id })
}

fn write_format_to_backend(
    storage: &Storage,
    backend: &(dyn ClipboardBackend + Send + Sync),
    format: &ClipboardFormat,
) -> Result<(), String> {
    match format {
        ClipboardFormat::Text(text) => backend.set_text(text).map_err(|e| e.to_string()),
        ClipboardFormat::Html(html) => backend.set_html(html).map_err(|e| e.to_string()),
        ClipboardFormat::Rtf(rtf) => backend.set_rtf(rtf).map_err(|e| e.to_string()),
        ClipboardFormat::Image(image) => {
            let bytes = storage.load_spilled(image).map_err(|e| e.to_string())?;
            backend
                .set_image(&bytes, image.width(), image.height())
                .map_err(|e| e.to_string())
        }
        ClipboardFormat::Files(files) => backend.set_files(files).map_err(|e| e.to_string()),
    }
}

/// `POST /clear` — clears the OS clipboard by writing an empty string
/// through the live backend. Does **not** clear history (that lives at
/// `DELETE /history`).
///
/// Per spec the response is `204 No Content`.
async fn clear_clipboard(State(state): State<SharedState>) -> Response {
    let backend = state.clipboard.clone();
    let result = tokio::task::spawn_blocking(move || backend.set_text("")).await;

    match result {
        Ok(Ok(())) => empty_response(StatusCode::NO_CONTENT),
        Ok(Err(e)) => internal(format!("clear failed: {e}")),
        Err(e) => internal(format!("clear task panicked: {e}")),
    }
}

/// `GET /events` — Server-Sent Events stream of clipboard change events.
///
/// Each `WatcherEvent::Changed` captured by the daemon (or `POST /set`
/// insertion) is fanned out as one SSE message containing a JSON
/// [`EntrySummary`]. Subscribers that lag past
/// [`EVENTS_CHANNEL_CAPACITY`] receive an internal `Lagged`
/// notification which we silently skip — the client will pick up at
/// the next event.
async fn events_stream(
    State(state): State<SharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(summary) => match serde_json::to_string(&summary) {
                Ok(payload) => Some(Ok(Event::default().event("entry").data(payload))),
                Err(_) => None,
            },
            // BroadcastStream surfaces lag as an Err — we drop it and let
            // the client continue with the next event.
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Debug)]
enum BuildFormatError {
    InvalidBase64(String),
}

fn build_format_from_request(
    storage: &Storage,
    req: SetRequest,
) -> Result<ClipboardFormat, BuildFormatError> {
    match req {
        SetRequest::Text { data } => Ok(ClipboardFormat::Text(data)),
        SetRequest::Html { data } => Ok(ClipboardFormat::Html(data)),
        SetRequest::Rtf { data } => Ok(ClipboardFormat::Rtf(data)),
        SetRequest::Files { data } => Ok(ClipboardFormat::Files(data)),
        SetRequest::Image {
            data,
            width,
            height,
        } => {
            let bytes = BASE64_STANDARD.decode(data.as_bytes()).map_err(|e| {
                BuildFormatError::InvalidBase64(format!("invalid base64 image data: {e}"))
            })?;
            let inline = ImageSnapshot::Inline {
                data: bytes,
                width,
                height,
            };
            // Hash the inline bytes so `Storage::spill_if_needed` writes to a
            // content-addressed file when the payload exceeds the threshold.
            let probe_hash = inline_image_hash(&inline);
            let stored = storage.spill_if_needed(probe_hash, inline);
            Ok(ClipboardFormat::Image(stored))
        }
    }
}

/// Cheap content-hash for an inline image; used as the spill filename
/// stem so that two `/set` calls with the same bytes land in the same
/// cache file. Mirrors the dedup hash used by `History`.
fn inline_image_hash(image: &ImageSnapshot) -> u64 {
    Storage::hash_image_payload(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use biscuit_clipboard::backend::MockClipboard;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn test_app() -> (Router, TempDir) {
        test_app_with(MockClipboard::default())
    }

    fn test_app_with(mock: MockClipboard) -> (Router, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
            clipboard: Arc::new(mock),
            watcher_status: Arc::new(RwLock::new(SupervisorStatus::Running)),
            events_tx: AppState::new_events_channel(),
        });
        (router(state), dir)
    }

    fn test_app_with_spill_threshold(threshold: usize) -> (Router, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf())
            .with_spill_threshold(threshold);
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
            clipboard: Arc::new(MockClipboard::default()),
            watcher_status: Arc::new(RwLock::new(SupervisorStatus::Running)),
            events_tx: AppState::new_events_channel(),
        });
        (router(state), dir)
    }

    /// Build a test app whose `watcher_status` we can mutate at runtime
    /// to drive the `/health` degraded path.
    fn test_app_with_status() -> (Router, TempDir, Arc<RwLock<SupervisorStatus>>) {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());
        let watcher_status = Arc::new(RwLock::new(SupervisorStatus::Running));
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
            clipboard: Arc::new(MockClipboard::default()),
            watcher_status: watcher_status.clone(),
            events_tx: AppState::new_events_channel(),
        });
        (router(state), dir, watcher_status)
    }

    async fn send_request(
        app: &Router,
        req: Request<Body>,
    ) -> axum::http::Response<axum::body::Body> {
        app.clone().oneshot(req).await.unwrap()
    }

    async fn body_text(response: axum::http::Response<axum::body::Body>) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn body_bytes(response: axum::http::Response<axum::body::Body>) -> Vec<u8> {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        bytes.to_vec()
    }

    fn assert_clipper_header(response: &axum::http::Response<axum::body::Body>) {
        let header = response
            .headers()
            .get(HEADER_CLIPPER)
            .expect("missing X-Clipper header");
        assert_eq!(header, "1", "X-Clipper header must be '1'");
    }

    fn b64(data: &[u8]) -> String {
        BASE64_STANDARD.encode(data)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let (app, _dir) = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_clipper_header(&response);
    }

    #[tokio::test]
    async fn test_health_reports_watcher_running_in_happy_path() {
        let (app, _dir) = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_clipper_header(&response);

        let body: serde_json::Value = serde_json::from_str(&body_text(response).await).unwrap();
        assert_eq!(body["watcher"].as_str().unwrap(), "running");
    }

    #[tokio::test]
    async fn test_health_returns_503_with_service_unavailable_when_stopped() {
        let (app, _dir, status) = test_app_with_status();
        *status.write().await = SupervisorStatus::Stopped;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_clipper_header(&response);

        let body_str = body_text(response).await;
        let envelope: ErrorResponse = serde_json::from_str(&body_str).unwrap();
        assert_eq!(envelope.error.code, ErrorCode::ServiceUnavailable);
        assert!(!envelope.error.message.is_empty());
    }

    #[tokio::test]
    async fn test_health_returns_503_when_degraded() {
        let (app, _dir, status) = test_app_with_status();
        *status.write().await = SupervisorStatus::Degraded {
            consecutive_failures: 2,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let envelope: ErrorResponse = serde_json::from_str(&body_text(response).await).unwrap();
        assert_eq!(envelope.error.code, ErrorCode::ServiceUnavailable);
    }

    #[tokio::test]
    async fn test_history_empty() {
        let (app, _dir) = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_clipper_header(&response);
    }

    #[tokio::test]
    async fn test_set_text_via_tagged_enum() {
        let (app, _dir) = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_set_text_invokes_backend_set_text() {
        let mock = Arc::new(MockClipboard::default());
        let (app, _dir) = test_app_with_mock_arc(mock.clone());

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"hello"}"#))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(mock.set_text_log(), vec!["hello".to_string()]);
    }

    #[tokio::test]
    async fn test_set_html_invokes_backend_set_html() {
        let mock = Arc::new(MockClipboard::default());
        let (app, _dir) = test_app_with_mock_arc(mock.clone());

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"content_type":"html","data":"<b>hello</b>"}"#,
                ))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(mock.set_html_log(), vec!["<b>hello</b>".to_string()]);
    }

    #[tokio::test]
    async fn test_set_image_invokes_backend_set_image() {
        let mock = Arc::new(MockClipboard::default());
        let (app, _dir) = test_app_with_mock_arc(mock.clone());
        let pixel_bytes: Vec<u8> = vec![1, 2, 3, 4];
        let body = serde_json::json!({
            "content_type": "image",
            "data": b64(&pixel_bytes),
            "width": 2u32,
            "height": 2u32,
        });

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(mock.set_image_log(), vec![(pixel_bytes, 2, 2)]);
    }

    #[tokio::test]
    async fn test_set_image_decodes_base64_and_creates_image_entry() {
        let (app, _dir) = test_app();

        let pixel_bytes: Vec<u8> = vec![1, 2, 3, 4];
        let body = serde_json::json!({
            "content_type": "image",
            "data": b64(&pixel_bytes),
            "width": 2u32,
            "height": 2u32,
        });

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let set: SetResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_ne!(set.id, "duplicate");

        // Verify the entry's primary content type is `image`.
        let resp = send_request(
            &app,
            Request::builder()
                .uri("/history/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let summary: EntrySummary = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(summary.content_type, "image");
    }

    #[tokio::test]
    async fn test_set_image_disk_spill_for_large_payload() {
        let (app, dir) = test_app_with_spill_threshold(64);

        let large: Vec<u8> = (0..=255u8).cycle().take(200).collect();
        let body = serde_json::json!({
            "content_type": "image",
            "data": b64(&large),
            "width": 10u32,
            "height": 10u32,
        });

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        // The cache dir must contain at least one spilled `.dat` file.
        let mut found_dat = false;
        for entry in std::fs::read_dir(dir.path()).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().is_some_and(|e| e == "dat") {
                found_dat = true;
                break;
            }
        }
        assert!(
            found_dat,
            "expected a spilled .dat file in {:?}",
            dir.path()
        );
    }

    #[tokio::test]
    async fn test_set_malformed_body_returns_bad_request_envelope() {
        let (app, _dir) = test_app();

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"text":"hi"}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::BadRequest);
    }

    #[tokio::test]
    async fn test_set_image_with_invalid_base64_returns_bad_request() {
        let (app, _dir) = test_app();
        let body = r#"{"content_type":"image","data":"!!! not base64 !!!","width":1,"height":1}"#;
        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::BadRequest);
    }

    #[tokio::test]
    async fn test_delete_history() {
        let (app, _dir) = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_latest_when_empty() {
        let (app, _dir) = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/history/latest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let env: ErrorResponse = serde_json::from_str(&body_text(response).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::EntryNotFound);
    }

    #[tokio::test]
    async fn test_entry_not_found_returns_envelope() {
        let (app, _dir) = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/history/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let env: ErrorResponse = serde_json::from_str(&body_text(response).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::EntryNotFound);
        assert!(env.error.message.contains("nonexistent"));
    }

    /// `/set` writes through the live backend, so a subsequent
    /// `/current` on the same app state sees the text that was set.
    #[tokio::test]
    async fn test_set_then_current_roundtrip() {
        let (app, _dir) = test_app();

        send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"in history"}"#))
                .unwrap(),
        )
        .await;

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let summary: EntrySummary = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(summary.id, "current");
        assert_eq!(summary.preview, "in history");
    }

    #[tokio::test]
    async fn test_current_returns_204_when_clipboard_empty() {
        let (app, _dir) = test_app();
        let resp = send_request(
            &app,
            Request::builder()
                .uri("/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_current_returns_id_current_for_live_read() {
        let mock = MockClipboard {
            text: Some("live text".to_string()),
            ..Default::default()
        };
        let (app, _dir) = test_app_with(mock);

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let summary: EntrySummary = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(summary.id, "current");
        assert_eq!(summary.content_type, "text");
        assert_eq!(summary.preview, "live text");
    }

    #[tokio::test]
    async fn test_current_does_not_insert_into_history() {
        let mock = MockClipboard {
            text: Some("not stored".to_string()),
            ..Default::default()
        };
        let (app, _dir) = test_app_with(mock);

        // Hit /current twice.
        for _ in 0..2 {
            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/current")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // History must remain empty.
        let resp = send_request(
            &app,
            Request::builder()
                .uri("/history")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let history: HistoryResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(history.entries.len(), 0);
    }

    #[tokio::test]
    async fn test_history_query_since_filters_old_entries() {
        let (app, _dir) = test_app();

        // First entry — capture timestamp.
        send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"first"}"#))
                .unwrap(),
        )
        .await;
        let resp = send_request(
            &app,
            Request::builder()
                .uri("/history/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let first: EntrySummary = serde_json::from_str(&body_text(resp).await).unwrap();
        let cutoff = first.timestamp.clone();

        // Second entry, ensure timestamps differ.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"second"}"#))
                .unwrap(),
        )
        .await;

        let url = format!("/history?since={}", urlencode(&cutoff));
        let resp = send_request(
            &app,
            Request::builder().uri(&url).body(Body::empty()).unwrap(),
        )
        .await;
        let history: HistoryResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].preview, "second");
    }

    #[tokio::test]
    async fn test_history_query_limit_truncates() {
        let (app, _dir) = test_app();

        for i in 0..5 {
            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content_type":"text","data":"entry-{i}"}}"#
                    )))
                    .unwrap(),
            )
            .await;
        }

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/history?limit=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let history: HistoryResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(history.entries.len(), 2);
        // newest first — entry-4 then entry-3
        assert_eq!(history.entries[0].preview, "entry-4");
        assert_eq!(history.entries[1].preview, "entry-3");
    }

    #[tokio::test]
    async fn test_content_format_priority_text_over_html() {
        let (app, _dir) = test_app();

        // Insert via /set with text body, then directly into history with
        // both formats by inserting a multi-format entry — easier to do via
        // the inner state. We achieve this by sending two /set calls — but
        // /set today only handles a single format. Instead build the entry
        // through history directly via a downcast: we use an integration
        // helper test by bypassing /set: we can construct a multi-format
        // entry via History::insert through a clone of the state. Simpler
        // path: rely on the format-priority unit test in entry.rs and here
        // assert that single-format /set + ?format=text round-trips.
        send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"plain"}"#))
                .unwrap(),
        )
        .await;
        let resp = send_request(
            &app,
            Request::builder()
                .uri("/history/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let s: EntrySummary = serde_json::from_str(&body_text(resp).await).unwrap();
        let id = s.id;

        // Default — no format param — picks text per priority.
        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!("/history/{id}/content"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_text(resp).await;
        assert_eq!(body, "plain");
    }

    /// Multi-format entries — insert directly via the state to verify
    /// `text > html > rtf > image > files` priority selection.
    #[tokio::test]
    async fn test_format_priority_picks_text_over_others() {
        // Construct an entry with html, rtf, image, files via the state.
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
            clipboard: Arc::new(MockClipboard::default()),
            watcher_status: Arc::new(RwLock::new(SupervisorStatus::Running)),
            events_tx: AppState::new_events_channel(),
        });
        let app = router(state.clone());

        // Insert directly with multiple formats — text first, then html, rtf, image, files.
        {
            let mut hist = state.history.lock().await;
            let _ = hist.insert(vec![
                ClipboardFormat::Html("<b>html</b>".to_string()),
                ClipboardFormat::Text("text wins".to_string()),
                ClipboardFormat::Rtf("{\\rtf}".to_string()),
                ClipboardFormat::Image(ImageSnapshot::Inline {
                    data: vec![1, 2, 3],
                    width: 1,
                    height: 1,
                }),
            ]);
        }
        let id = state.history.lock().await.latest().unwrap().id.clone();

        // Default (no format) should pick text.
        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!("/history/{id}/content"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_text(resp).await, "text wins");

        // Explicit html.
        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!("/history/{id}/content?format=html"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_text(resp).await, "<b>html</b>");

        // Explicit rtf.
        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!("/history/{id}/content?format=rtf"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_text(resp).await, "{\\rtf}");

        // Explicit image — bytes.
        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!("/history/{id}/content?format=image"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_bytes(resp).await, vec![1u8, 2, 3]);

        // Image with base64 encoding.
        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!(
                    "/history/{id}/content?format=image&encoding=base64"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let encoded = body_text(resp).await;
        assert_eq!(encoded, BASE64_STANDARD.encode([1u8, 2, 3]));

        // Files not present — 406 format_not_available.
        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!("/history/{id}/content?format=files"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
        let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::FormatNotAvailable);
    }

    #[tokio::test]
    async fn test_content_encoding_base64_round_trip() {
        let (app, _dir) = test_app();
        send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"hello b64"}"#))
                .unwrap(),
        )
        .await;
        let resp = send_request(
            &app,
            Request::builder()
                .uri("/history/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let s: EntrySummary = serde_json::from_str(&body_text(resp).await).unwrap();
        let id = s.id;

        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!("/history/{id}/content?encoding=base64"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let encoded = body_text(resp).await;
        let decoded = BASE64_STANDARD.decode(encoded.as_bytes()).unwrap();
        assert_eq!(decoded, b"hello b64");
    }

    /// Build an app whose live backend is the supplied `MockClipboard`
    /// shared via `Arc`, so the test can read its `set_text_calls` log
    /// after invoking the API.
    fn test_app_with_mock_arc(mock: Arc<MockClipboard>) -> (Router, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
            clipboard: mock,
            watcher_status: Arc::new(RwLock::new(SupervisorStatus::Running)),
            events_tx: AppState::new_events_channel(),
        });
        (router(state), dir)
    }

    #[tokio::test]
    async fn test_clear_endpoint_returns_204_and_invokes_set_text_empty() {
        let mock = Arc::new(MockClipboard::default());
        let (app, _dir) = test_app_with_mock_arc(mock.clone());

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/clear")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_clipper_header(&resp);

        // Backend should have observed exactly one set_text("") call.
        let calls = mock.set_text_log();
        assert_eq!(calls.len(), 1, "expected single set_text call");
        assert_eq!(calls[0], "");
    }

    #[tokio::test]
    async fn test_clear_endpoint_does_not_clear_history() {
        let mock = Arc::new(MockClipboard::default());
        let (app, _dir) = test_app_with_mock_arc(mock.clone());

        // Populate history first.
        send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"keep me"}"#))
                .unwrap(),
        )
        .await;

        // POST /clear must NOT empty history.
        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/clear")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/history")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let history: HistoryResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(history.entries.len(), 1, "history must survive POST /clear");
    }

    /// Phase 6 / Test Coverage #2: every documented `ErrorCode` must have
    /// at least one envelope-shape assertion. The `Internal` variant is
    /// reachable via the `/clear` handler when the backend's `set_text`
    /// fails. This test installs a backend that always errors and asserts
    /// the response is a `500 internal` envelope.
    #[tokio::test]
    async fn test_internal_error_envelope_from_clear_failure() {
        use biscuit_clipboard::error::ClipboardError;
        use std::path::PathBuf;

        struct ErroringBackend;
        impl ClipboardBackend for ErroringBackend {
            fn get_text(&self) -> Result<Option<String>, ClipboardError> {
                Ok(None)
            }
            fn get_html(&self) -> Result<Option<String>, ClipboardError> {
                Ok(None)
            }
            fn get_rtf(&self) -> Result<Option<String>, ClipboardError> {
                Ok(None)
            }
            fn get_image(&self) -> Result<Option<ImageSnapshot>, ClipboardError> {
                Ok(None)
            }
            fn get_files(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
                Ok(None)
            }
            fn set_text(&self, _: &str) -> Result<(), ClipboardError> {
                Err(ClipboardError::Backend("synthetic set_text failure".into()))
            }
            fn set_html(&self, _: &str) -> Result<(), ClipboardError> {
                Ok(())
            }
            fn set_rtf(&self, _: &str) -> Result<(), ClipboardError> {
                Ok(())
            }
            fn set_image(&self, _: &[u8], _: u32, _: u32) -> Result<(), ClipboardError> {
                Ok(())
            }
            fn set_files(&self, _: &[PathBuf]) -> Result<(), ClipboardError> {
                Ok(())
            }
            fn is_concealed(&self) -> Result<bool, ClipboardError> {
                Ok(false)
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
            clipboard: Arc::new(ErroringBackend),
            watcher_status: Arc::new(RwLock::new(SupervisorStatus::Running)),
            events_tx: AppState::new_events_channel(),
        });
        let app = router(state);

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/clear")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::Internal);
        assert!(
            env.error.message.contains("clear failed"),
            "expected message to contain 'clear failed', got: {}",
            env.error.message,
        );
    }

    #[tokio::test]
    async fn test_set_backend_failure_returns_500_internal() {
        use biscuit_clipboard::error::ClipboardError;
        use std::path::PathBuf;

        struct ErroringBackend;
        impl ClipboardBackend for ErroringBackend {
            fn get_text(&self) -> Result<Option<String>, ClipboardError> {
                Ok(None)
            }
            fn get_html(&self) -> Result<Option<String>, ClipboardError> {
                Ok(None)
            }
            fn get_rtf(&self) -> Result<Option<String>, ClipboardError> {
                Ok(None)
            }
            fn get_image(&self) -> Result<Option<ImageSnapshot>, ClipboardError> {
                Ok(None)
            }
            fn get_files(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
                Ok(None)
            }
            fn set_text(&self, _: &str) -> Result<(), ClipboardError> {
                Err(ClipboardError::Backend("synthetic set_text failure".into()))
            }
            fn set_html(&self, _: &str) -> Result<(), ClipboardError> {
                Ok(())
            }
            fn set_rtf(&self, _: &str) -> Result<(), ClipboardError> {
                Ok(())
            }
            fn set_image(&self, _: &[u8], _: u32, _: u32) -> Result<(), ClipboardError> {
                Ok(())
            }
            fn set_files(&self, _: &[PathBuf]) -> Result<(), ClipboardError> {
                Ok(())
            }
            fn is_concealed(&self) -> Result<bool, ClipboardError> {
                Ok(false)
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
            clipboard: Arc::new(ErroringBackend),
            watcher_status: Arc::new(RwLock::new(SupervisorStatus::Running)),
            events_tx: AppState::new_events_channel(),
        });
        let app = router(state.clone());

        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"boom"}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::Internal);

        let history = state.history.lock().await;
        assert_eq!(
            history.len(),
            0,
            "failed backend writes must not enter history"
        );
    }

    /// Phase 6 / Test Coverage #9: end-to-end image spill — POST a large
    /// image, confirm a `.dat` file lands on disk, GET the content with
    /// `?encoding=base64`, and assert the round-trip yields the exact
    /// original bytes. Complements `test_set_image_disk_spill_for_large_payload`
    /// (which only verifies the spill file exists) by exercising the
    /// load_spilled path back through the API.
    #[tokio::test]
    async fn test_image_disk_spill_round_trip_via_api() {
        let (app, dir) = test_app_with_spill_threshold(64);

        // 200-byte cycling pattern is well above the 64-byte threshold.
        let payload: Vec<u8> = (0..=255u8).cycle().take(200).collect();
        let body = serde_json::json!({
            "content_type": "image",
            "data": b64(&payload),
            "width": 10u32,
            "height": 10u32,
        });
        let resp = send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let set_resp: SetResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        let id = set_resp.id;

        // Confirm the spill file exists.
        let mut found_dat = false;
        for entry in std::fs::read_dir(dir.path()).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().is_some_and(|e| e == "dat") {
                found_dat = true;
                break;
            }
        }
        assert!(found_dat, "expected spilled .dat file in {:?}", dir.path());

        // GET /history/<id>/content?format=image&encoding=base64 must
        // round-trip the exact bytes.
        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!(
                    "/history/{id}/content?format=image&encoding=base64"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let encoded = body_text(resp).await;
        let decoded = BASE64_STANDARD.decode(encoded.as_bytes()).unwrap();
        assert_eq!(decoded, payload, "round-tripped image bytes must match");
    }

    #[tokio::test]
    async fn test_events_endpoint_streams_set_inserts() {
        let (app, _dir) = test_app();

        let req = Request::builder()
            .uri("/events")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.contains("text/event-stream"),
            "expected SSE content-type, got {ct}"
        );

        // Drive an insertion concurrently and pick the first event off
        // the body stream.
        let app_for_post = app.clone();
        let post_task = tokio::spawn(async move {
            // Slight delay so the SSE subscriber registers first.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            send_request(
                &app_for_post,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"sse hello"}"#))
                    .unwrap(),
            )
            .await
        });

        // Read the SSE response body until we see one event.
        let mut body = resp.into_body().into_data_stream();
        let mut buffer = String::new();
        let timeout = tokio::time::sleep(std::time::Duration::from_secs(3));
        tokio::pin!(timeout);

        let payload = loop {
            tokio::select! {
                _ = &mut timeout => panic!("timed out waiting for SSE event"),
                chunk = body.next() => {
                    let chunk = match chunk {
                        Some(Ok(c)) => c,
                        Some(Err(e)) => panic!("body stream error: {e}"),
                        None => panic!("body closed before event"),
                    };
                    buffer.push_str(&String::from_utf8_lossy(&chunk));
                    if let Some(idx) = buffer.find("data: ")
                        && let Some(end) = buffer[idx..].find('\n')
                    {
                        let line = &buffer[idx + 6..idx + end];
                        break line.to_string();
                    }
                }
            }
        };

        let _ = post_task.await;

        let summary: EntrySummary = serde_json::from_str(&payload).unwrap_or_else(|e| {
            panic!("expected EntrySummary JSON, got `{payload}` ({e})");
        });
        assert_eq!(summary.preview, "sse hello");
        assert_eq!(summary.content_type, "text");
    }

    #[tokio::test]
    async fn test_events_endpoint_fans_out_to_multiple_subscribers() {
        let (app, _dir) = test_app();

        // Two subscribers
        let sub1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let sub2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Drive a /set
        let app_for_post = app.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            send_request(
                &app_for_post,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"fan out"}"#))
                    .unwrap(),
            )
            .await
        });

        async fn read_first_event(resp: axum::http::Response<axum::body::Body>) -> String {
            let mut body = resp.into_body().into_data_stream();
            let mut buffer = String::new();
            let timeout = tokio::time::sleep(std::time::Duration::from_secs(3));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    _ = &mut timeout => panic!("timed out reading SSE"),
                    chunk = body.next() => {
                        let chunk = chunk.expect("body closed").expect("body err");
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                        if let Some(idx) = buffer.find("data: ")
                            && let Some(end) = buffer[idx..].find('\n')
                        {
                            return buffer[idx + 6..idx + end].to_string();
                        }
                    }
                }
            }
        }

        let (a, b) = tokio::join!(read_first_event(sub1), read_first_event(sub2));
        let s_a: EntrySummary = serde_json::from_str(&a).unwrap();
        let s_b: EntrySummary = serde_json::from_str(&b).unwrap();
        assert_eq!(s_a.preview, "fan out");
        assert_eq!(s_b.preview, "fan out");
        assert_eq!(s_a.id, s_b.id, "both subscribers must see the same id");
    }

    #[tokio::test]
    async fn test_format_not_available_returns_envelope() {
        let (app, _dir) = test_app();
        send_request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/set")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content_type":"text","data":"only text"}"#))
                .unwrap(),
        )
        .await;
        let resp = send_request(
            &app,
            Request::builder()
                .uri("/history/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let s: EntrySummary = serde_json::from_str(&body_text(resp).await).unwrap();
        let id = s.id;

        let resp = send_request(
            &app,
            Request::builder()
                .uri(format!("/history/{id}/content?format=image"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
        let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::FormatNotAvailable);
    }

    fn urlencode(s: &str) -> String {
        // RFC3339 timestamps contain `:` and `+` which need encoding for
        // a query parameter. Hand-roll a tiny encoder to avoid pulling a
        // dep just for tests.
        let mut out = String::new();
        for c in s.chars() {
            match c {
                '0'..='9' | 'A'..='Z' | 'a'..='z' | '-' | '_' | '.' | '~' => out.push(c),
                _ => {
                    let mut buf = [0u8; 4];
                    for b in c.encode_utf8(&mut buf).as_bytes() {
                        out.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        out
    }

    // -------------------------------------------------------------------
    // Phase 2: /current must honour ?format= and ?encoding= query params
    // -------------------------------------------------------------------

    /// `GET /current?format=text` returns the text body directly.
    #[tokio::test]
    async fn test_current_with_format_text() {
        let mock = MockClipboard {
            text: Some("plain text".to_string()),
            html: Some("<b>html</b>".to_string()),
            ..Default::default()
        };
        let (app, _dir) = test_app_with(mock);

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/current?format=text")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_text(resp).await, "plain text");
    }

    /// `GET /current?format=html` returns the HTML body directly.
    #[tokio::test]
    async fn test_current_with_format_html() {
        let mock = MockClipboard {
            text: Some("plain text".to_string()),
            html: Some("<b>html</b>".to_string()),
            ..Default::default()
        };
        let (app, _dir) = test_app_with(mock);

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/current?format=html")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_text(resp).await, "<b>html</b>");
    }

    /// Requesting a format that is not on the clipboard returns 406.
    #[tokio::test]
    async fn test_current_with_format_missing_returns_406() {
        let mock = MockClipboard {
            text: Some("only text".to_string()),
            ..Default::default()
        };
        let (app, _dir) = test_app_with(mock);

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/current?format=html")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
        let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(env.error.code, ErrorCode::FormatNotAvailable);
    }

    /// `GET /current?encoding=base64` returns the primary format
    /// base64-encoded.
    #[tokio::test]
    async fn test_current_with_encoding_base64() {
        let mock = MockClipboard {
            text: Some("hello base64".to_string()),
            ..Default::default()
        };
        let (app, _dir) = test_app_with(mock);

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/current?encoding=base64")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let encoded = body_text(resp).await;
        let decoded = BASE64_STANDARD.decode(encoded.as_bytes()).unwrap();
        assert_eq!(decoded, b"hello base64");
    }

    /// Without query params, `/current` preserves the existing
    /// `EntrySummary` JSON behaviour (regression guard).
    #[tokio::test]
    async fn test_current_without_query_params_returns_entry_summary() {
        let mock = MockClipboard {
            text: Some("summary text".to_string()),
            ..Default::default()
        };
        let (app, _dir) = test_app_with(mock);

        let resp = send_request(
            &app,
            Request::builder()
                .uri("/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let summary: EntrySummary = serde_json::from_str(&body_text(resp).await).unwrap();
        assert_eq!(summary.id, "current");
        assert_eq!(summary.preview, "summary text");
        assert_eq!(summary.content_type, "text");
    }

    mod integration {
        use super::*;

        #[tokio::test]
        async fn test_full_set_text_check_history_flow() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"content_type":"text","data":"hello world"}"#,
                    ))
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_clipper_header(&resp);

            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let entry_id = body["id"].as_str().unwrap();
            assert!(!entry_id.is_empty());
            assert_ne!(entry_id, "duplicate");

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_clipper_header(&resp);

            let history: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let entries = history["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["id"].as_str().unwrap(), entry_id);
            assert_eq!(entries[0]["content_type"].as_str().unwrap(), "text");
            assert_eq!(entries[0]["preview"].as_str().unwrap(), "hello world");

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(format!("/history/{entry_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_clipper_header(&resp);

            let entry: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(entry["id"].as_str().unwrap(), entry_id);
            assert_eq!(entry["content_type"].as_str().unwrap(), "text");

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(format!("/history/{entry_id}/content"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_clipper_header(&resp);

            let text = body_text(resp).await;
            assert_eq!(text, "hello world");
        }

        #[tokio::test]
        async fn test_multiple_entries_history_ordering() {
            let (app, _dir) = test_app();

            for i in 0..5 {
                let resp = send_request(
                    &app,
                    Request::builder()
                        .method("POST")
                        .uri("/set")
                        .header("content-type", "application/json")
                        .body(Body::from(format!(
                            r#"{{"content_type":"text","data":"entry-{i}"}}"#
                        )))
                        .unwrap(),
                )
                .await;
                assert_eq!(resp.status(), StatusCode::OK);
            }

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);

            let history: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let entries = history["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 5);

            assert_eq!(entries[0]["preview"].as_str().unwrap(), "entry-4");
            assert_eq!(entries[4]["preview"].as_str().unwrap(), "entry-0");
        }

        #[tokio::test]
        async fn test_deduplication_through_api() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"duplicate"}"#))
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let first_id = body["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"duplicate"}"#))
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(body["id"].as_str().unwrap(), "duplicate");

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let history: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let entries = history["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["id"].as_str().unwrap(), first_id);
        }

        #[tokio::test]
        async fn test_latest_reflects_most_recent() {
            let (app, _dir) = test_app();

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"first"}"#))
                    .unwrap(),
            )
            .await;

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"second"}"#))
                    .unwrap(),
            )
            .await;

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history/latest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_clipper_header(&resp);

            let latest: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(latest["preview"].as_str().unwrap(), "second");
        }

        #[tokio::test]
        async fn test_clear_history_flow() {
            let (app, _dir) = test_app();

            for i in 0..3 {
                send_request(
                    &app,
                    Request::builder()
                        .method("POST")
                        .uri("/set")
                        .header("content-type", "application/json")
                        .body(Body::from(format!(
                            r#"{{"content_type":"text","data":"item-{i}"}}"#
                        )))
                        .unwrap(),
                )
                .await;
            }

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let history: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(history["entries"].as_array().unwrap().len(), 3);

            let resp = send_request(
                &app,
                Request::builder()
                    .method("DELETE")
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::NO_CONTENT);
            assert_clipper_header(&resp);

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let history: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(history["entries"].as_array().unwrap().len(), 0);

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history/latest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_set_empty_body_returns_error_envelope() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
            assert_clipper_header(&resp);
            let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(env.error.code, ErrorCode::BadRequest);
        }

        #[tokio::test]
        async fn test_content_endpoint_for_text_entry() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"content_type":"text","data":"plain text content"}"#,
                    ))
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let entry_id = body["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(format!("/history/{entry_id}/content"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_clipper_header(&resp);

            let ct = resp
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap();
            assert!(ct.contains("text/plain"));

            let text = body_text(resp).await;
            assert_eq!(text, "plain text content");
        }

        #[tokio::test]
        async fn test_health_shows_entry_count() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(body["entries"].as_u64().unwrap(), 0);

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"test"}"#))
                    .unwrap(),
            )
            .await;

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(body["entries"].as_u64().unwrap(), 1);
        }

        #[tokio::test]
        async fn test_x_clipper_header_on_all_endpoints() {
            let (app, _dir) = test_app();

            let endpoints = [
                ("GET", "/health"),
                ("GET", "/history"),
                ("GET", "/history/latest"),
                ("GET", "/history/nonexistent"),
                ("GET", "/current"),
            ];

            for (method, uri) in &endpoints {
                let resp = send_request(
                    &app,
                    Request::builder()
                        .method(*method)
                        .uri(*uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await;
                assert_clipper_header(&resp);
            }

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"content_type":"text","data":"header test"}"#,
                    ))
                    .unwrap(),
            )
            .await;
            assert_clipper_header(&resp);

            let resp = send_request(
                &app,
                Request::builder()
                    .method("DELETE")
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_clipper_header(&resp);
        }

        #[tokio::test]
        async fn test_x_clipper_fingerprint_prevents_false_positive() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/health")
                    .header("x-clipper", "1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);

            let resp_header = resp.headers().get(HEADER_CLIPPER);
            assert!(resp_header.is_some());
            assert_eq!(resp_header.unwrap(), "1");

            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(body["status"].as_str().unwrap(), "ok");
        }

        #[tokio::test]
        async fn test_storage_spill_roundtrip_via_api() {
            let (app, _dir) = test_app_with_spill_threshold(32);

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"content_type":"text","data":"small entry"}"#,
                    ))
                    .unwrap(),
            )
            .await;

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history/latest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let latest: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(latest["preview"].as_str().unwrap(), "small entry");
            let entry_id = latest["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(format!("/history/{entry_id}/content"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            let text = body_text(resp).await;
            assert_eq!(text, "small entry");
        }

        #[tokio::test]
        async fn test_entry_id_is_deterministic_via_api() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"consistent"}"#))
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let first_id = body["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("DELETE")
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::NO_CONTENT);

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"consistent"}"#))
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let second_id = body["id"].as_str().unwrap();

            assert_eq!(first_id, second_id);
        }

        #[tokio::test]
        async fn test_reinsertion_moves_to_front_via_api() {
            let (app, _dir) = test_app();

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"alpha"}"#))
                    .unwrap(),
            )
            .await;

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"beta"}"#))
                    .unwrap(),
            )
            .await;

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"alpha"}"#))
                    .unwrap(),
            )
            .await;

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let history: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let entries = history["entries"].as_array().unwrap();

            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0]["preview"].as_str().unwrap(), "alpha");
            assert_eq!(entries[1]["preview"].as_str().unwrap(), "beta");
        }

        #[tokio::test]
        async fn test_nonexistent_entry_content_returns_envelope() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history/deadbeef/content")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            assert_clipper_header(&resp);
            let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(env.error.code, ErrorCode::EntryNotFound);
        }

        #[tokio::test]
        async fn test_nonexistent_entry_thumbnail_returns_envelope() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history/deadbeef/thumbnail")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            assert_clipper_header(&resp);
            let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(env.error.code, ErrorCode::EntryNotFound);
        }

        #[tokio::test]
        async fn test_thumbnail_for_text_entry_returns_format_not_available() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"text only"}"#))
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();
            let entry_id = body["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(format!("/history/{entry_id}/thumbnail"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
            let env: ErrorResponse = serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(env.error.code, ErrorCode::FormatNotAvailable);
        }

        #[tokio::test]
        async fn test_response_shapes_match_spec() {
            let (app, _dir) = test_app();

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"text","data":"spec test"}"#))
                    .unwrap(),
            )
            .await;

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history/latest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let entry: serde_json::Value = serde_json::from_str(&body_text(resp).await).unwrap();

            assert!(entry["id"].is_string());
            assert!(entry["timestamp"].is_string());
            assert!(entry["content_type"].is_string());
            assert!(entry["preview"].is_string());
            assert!(entry["size_bytes"].is_u64());

            let ts = entry["timestamp"].as_str().unwrap();
            assert!(ts.contains('T'));
            assert!(ts.contains('Z') || ts.contains('+'));
        }
    }
}
