use axum::{
    extract::Path,
    http::{StatusCode, Uri, header},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

/// Serves `index.html` for the root route.
pub async fn index_handler() -> impl IntoResponse {
    serve_embedded("index.html")
}

/// Serves static assets from the embedded `dist/assets/` directory.
pub async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    serve_embedded(&format!("assets/{path}"))
}

/// SPA fallback: tries the exact path first (for PWA files like sw.js),
/// then falls back to index.html for client-side routing.
pub async fn fallback_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    serve_embedded(path)
}

/// Serves any embedded file by path, with SPA fallback to `index.html`.
fn serve_embedded(path: &str) -> Response {
    match FrontendAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime)],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => {
            // SPA fallback: serve index.html for unmatched routes
            match FrontendAssets::get("index.html") {
                Some(index) => {
                    Html(String::from_utf8_lossy(&index.data).to_string()).into_response()
                }
                None => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Frontend not built. Run `pnpm build` in homelab/server/frontend/",
                )
                    .into_response(),
            }
        }
    }
}
