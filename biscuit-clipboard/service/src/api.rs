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

    fn test_app_with_spill_threshold(threshold: usize) -> (Router, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf())
            .with_spill_threshold(threshold);
        let state = Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
        });
        (router(state), dir)
    }

    async fn send_request(app: &Router, req: Request<Body>) -> axum::http::Response<axum::body::Body> {
        app.clone().oneshot(req).await.unwrap()
    }

    async fn body_text(response: axum::http::Response<axum::body::Body>) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[allow(dead_code)]
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
                    .body(Body::from(r#"{"text":"hello world"}"#))
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_clipper_header(&resp);

            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
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

            let history: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            let entries = history["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["id"].as_str().unwrap(), entry_id);
            assert_eq!(entries[0]["content_type"].as_str().unwrap(), "text");
            assert_eq!(entries[0]["preview"].as_str().unwrap(), "hello world");

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(&format!("/history/{entry_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_clipper_header(&resp);

            let entry: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(entry["id"].as_str().unwrap(), entry_id);
            assert_eq!(entry["content_type"].as_str().unwrap(), "text");

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(&format!("/history/{entry_id}/content"))
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
                        .body(Body::from(format!(r#"{{"text":"entry-{i}"}}"#)))
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

            let history: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
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
                    .body(Body::from(r#"{"text":"duplicate"}"#))
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            let first_id = body["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"duplicate"}"#))
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::OK);
            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(body["id"].as_str().unwrap(), "duplicate");

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            let history: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
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
                    .body(Body::from(r#"{"text":"first"}"#))
                    .unwrap(),
            )
            .await;

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"second"}"#))
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

            let latest: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(latest["preview"].as_str().unwrap(), "second");
        }

        #[tokio::test]
        async fn test_current_reflects_latest() {
            let (app, _dir) = test_app();

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"current content"}"#))
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
            assert_clipper_header(&resp);

            let text = body_text(resp).await;
            assert_eq!(text, "current content");
        }

        #[tokio::test]
        async fn test_current_empty_returns_not_found() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri("/current")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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
                        .body(Body::from(format!(r#"{{"text":"item-{i}"}}"#)))
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
            let history: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
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
            let history: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
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
        async fn test_set_empty_body_returns_error() {
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
                    .body(Body::from(r#"{"text":"plain text content"}"#))
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            let entry_id = body["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(&format!("/history/{entry_id}/content"))
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
            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(body["entries"].as_u64().unwrap(), 0);

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"test"}"#))
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
            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(body["entries"].as_u64().unwrap(), 1);
        }

        #[tokio::test]
        async fn test_x_clipper_header_on_all_endpoints() {
            let (app, _dir) = test_app();

            let endpoints = vec![
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
                    .body(Body::from(r#"{"text":"header test"}"#))
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

            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(body["status"].as_str().unwrap(), "ok");
        }

        #[tokio::test]
        async fn test_disk_spill_for_large_image_content() {
            let (app, dir) = test_app_with_spill_threshold(64);

            let large_data: Vec<u8> = (0..=255u8).cycle().take(200).collect();
            let large_b64 = base64_engine_encode(&large_data);

            let body_str = format!(
                r#"{{"content_type":"image","data":"{large_b64}","width":10,"height":10}}"#
            );

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(body_str))
                    .unwrap(),
            )
            .await;

            if resp.status() == StatusCode::BAD_REQUEST {
                send_request(
                    &app,
                    Request::builder()
                        .method("POST")
                        .uri("/set")
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"text":"fallback text"}"#))
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
            let history: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            let entries = history["entries"].as_array().unwrap();
            assert!(!entries.is_empty());

            assert!(dir.path().exists());
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
                    .body(Body::from(r#"{"text":"small entry"}"#))
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
            let latest: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            assert_eq!(latest["preview"].as_str().unwrap(), "small entry");
            let entry_id = latest["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(&format!("/history/{entry_id}/content"))
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
                    .body(Body::from(r#"{"text":"consistent"}"#))
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
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
                    .body(Body::from(r#"{"text":"consistent"}"#))
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
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
                    .body(Body::from(r#"{"text":"alpha"}"#))
                    .unwrap(),
            )
            .await;

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"beta"}"#))
                    .unwrap(),
            )
            .await;

            send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"alpha"}"#))
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
            let history: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            let entries = history["entries"].as_array().unwrap();

            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0]["preview"].as_str().unwrap(), "alpha");
            assert_eq!(entries[1]["preview"].as_str().unwrap(), "beta");
        }

        #[tokio::test]
        async fn test_nonexistent_entry_content_returns_not_found() {
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
        }

        #[tokio::test]
        async fn test_nonexistent_entry_thumbnail_returns_not_found() {
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
        }

        #[tokio::test]
        async fn test_thumbnail_for_text_entry_returns_not_found() {
            let (app, _dir) = test_app();

            let resp = send_request(
                &app,
                Request::builder()
                    .method("POST")
                    .uri("/set")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"text only"}"#))
                    .unwrap(),
            )
            .await;
            let body: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();
            let entry_id = body["id"].as_str().unwrap();

            let resp = send_request(
                &app,
                Request::builder()
                    .uri(&format!("/history/{entry_id}/thumbnail"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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
                    .body(Body::from(r#"{"text":"spec test"}"#))
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
            let entry: serde_json::Value =
                serde_json::from_str(&body_text(resp).await).unwrap();

            assert!(entry["id"].is_string());
            assert!(entry["timestamp"].is_string());
            assert!(entry["content_type"].is_string());
            assert!(entry["preview"].is_string());
            assert!(entry["size_bytes"].is_u64());

            let ts = entry["timestamp"].as_str().unwrap();
            assert!(ts.contains('T'));
            assert!(ts.contains('Z') || ts.contains('+'));
        }

        fn base64_engine_encode(data: &[u8]) -> String {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(data)
        }
    }
}
