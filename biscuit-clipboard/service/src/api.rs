use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use biscuit_clipboard::{ClipboardEntry, History, Storage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

const HEADER_CLIPPER: &str = "x-clipper";

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub history: Mutex<History>,
    pub storage: Storage,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub entries: usize,
}

#[derive(Serialize)]
pub struct HistoryResponse {
    pub entries: Vec<EntrySummary>,
}

#[derive(Serialize, Clone)]
pub struct EntrySummary {
    pub id: String,
    pub timestamp: String,
    pub content_type: String,
    pub preview: String,
    pub size_bytes: usize,
}

impl From<&ClipboardEntry> for EntrySummary {
    fn from(entry: &ClipboardEntry) -> Self {
        Self {
            id: entry.id.clone(),
            timestamp: entry.timestamp.to_rfc3339(),
            content_type: format!("{:?}", entry.primary_content_type()).to_lowercase(),
            preview: entry.preview(),
            size_bytes: entry.total_size_bytes(),
        }
    }
}

#[derive(Deserialize)]
pub struct SetRequest {
    pub text: Option<String>,
}

#[derive(Serialize)]
pub struct SetResponse {
    pub id: String,
}

fn clipper_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(HEADER_CLIPPER, HeaderValue::from_static("1"));
    headers
}

fn json_response(status: StatusCode, body: impl Serialize) -> Response {
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
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type),
    );
    (status, headers, data).into_response()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    json_response(
        status,
        serde_json::json!({"error": message}),
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
        .with_state(state)
}

async fn health(State(state): State<SharedState>) -> Response {
    let count = state.history.lock().await.len();
    json_response(
        StatusCode::OK,
        HealthResponse {
            status: "ok".to_string(),
            entries: count,
        },
    )
}

async fn get_history(State(state): State<SharedState>) -> Response {
    let history = state.history.lock().await;
    let entries: Vec<EntrySummary> = history.all().iter().map(EntrySummary::from).collect();
    json_response(StatusCode::OK, HistoryResponse { entries })
}

async fn delete_history(State(state): State<SharedState>) -> Response {
    state.history.lock().await.clear();
    let headers = clipper_headers();
    (StatusCode::NO_CONTENT, headers).into_response()
}

async fn get_latest(State(state): State<SharedState>) -> Response {
    let history = state.history.lock().await;
    match history.latest() {
        Some(entry) => json_response(StatusCode::OK, EntrySummary::from(entry)),
        None => error_response(StatusCode::NOT_FOUND, "no entries"),
    }
}

async fn get_entry(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Response {
    let history = state.history.lock().await;
    match history.get(&id) {
        Some(entry) => json_response(StatusCode::OK, EntrySummary::from(entry)),
        None => error_response(StatusCode::NOT_FOUND, "entry not found"),
    }
}

async fn get_content(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Response {
    let history = state.history.lock().await;
    match history.get(&id) {
        Some(entry) => {
            if let Some(text) = entry.find_text() {
                text_response(StatusCode::OK, text.to_string())
            } else if let Some(img) = entry.find_image() {
                match state.storage.load_spilled(img) {
                    Ok(data) => binary_response(StatusCode::OK, "image/png", data),
                    Err(e) => error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("Failed to load image: {e}"),
                    ),
                }
            } else {
                error_response(StatusCode::NOT_FOUND, "No accessible content")
            }
        }
        None => error_response(StatusCode::NOT_FOUND, "Entry not found"),
    }
}

async fn get_thumbnail(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Response {
    let history = state.history.lock().await;
    match history.get(&id) {
        Some(entry) => {
            if let Some(img) = entry.find_image() {
                match state.storage.load_spilled(img) {
                    Ok(data) => binary_response(StatusCode::OK, "image/png", data),
                    Err(e) => error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("Failed to load thumbnail: {e}"),
                    ),
                }
            } else {
                error_response(StatusCode::NOT_FOUND, "No image content")
            }
        }
        None => error_response(StatusCode::NOT_FOUND, "Entry not found"),
    }
}

async fn get_current(State(state): State<SharedState>) -> Response {
    let history = state.history.lock().await;
    match history.latest() {
        Some(entry) => {
            if let Some(text) = entry.find_text() {
                text_response(StatusCode::OK, text.to_string())
            } else {
                json_response(StatusCode::OK, EntrySummary::from(entry))
            }
        }
        None => error_response(StatusCode::NOT_FOUND, "no current content"),
    }
}

async fn set_content(
    State(state): State<SharedState>,
    Json(body): Json<SetRequest>,
) -> Response {
    if let Some(text) = body.text {
        let format = biscuit_clipboard::ClipboardFormat::Text(text);
        let mut history = state.history.lock().await;
        if let Some(entry) = history.insert(vec![format]) {
            json_response(
                StatusCode::OK,
                SetResponse { id: entry.id.clone() },
            )
        } else {
            json_response(
                StatusCode::OK,
                SetResponse {
                    id: "duplicate".to_string(),
                },
            )
        }
    } else {
        error_response(StatusCode::BAD_REQUEST, "no content provided")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn test_app() -> (Router, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
        });
        (router(state), dir)
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
        assert_eq!(response.headers().get("x-clipper").unwrap(), "1");
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
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let (app, _dir) = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
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
    }

    #[tokio::test]
    async fn test_entry_not_found() {
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
    }
}
