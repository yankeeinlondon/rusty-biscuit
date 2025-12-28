# Web Service Integration

Examples of integrating resvg with web frameworks for server-side SVG rendering.

## Actix-Web Example

Basic endpoint for rendering SVGs with customizable dimensions:

```rust
use actix_web::{web, App, HttpResponse, HttpServer};
use resvg::{tiny_skia, usvg};
use serde::Deserialize;

#[derive(Deserialize)]
struct RenderParams {
    path: String,
    width: Option<u32>,
    height: Option<u32>,
}

async fn render_svg(
    query: web::Query<RenderParams>,
) -> Result<HttpResponse, actix_web::Error> {
    // Parse query parameters
    let width = query.width.unwrap_or(800);
    let height = query.height.unwrap_or(600);

    // Load SVG
    let svg_data = std::fs::read(&query.path)
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Parse and render
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Failed to create pixmap"))?;

    resvg::render(&tree,
                  usvg::FitTo::Size(width, height),
                  tiny_skia::Transform::identity(),
                  &mut pixmap.as_mut());

    // Return PNG response
    let png_data = pixmap.encode_png()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(png_data))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/render", web::get().to(render_svg))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

## Axum Example

Similar functionality with Axum framework:

```rust
use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use resvg::{tiny_skia, usvg};
use serde::Deserialize;

#[derive(Deserialize)]
struct RenderParams {
    path: String,
    #[serde(default = "default_width")]
    width: u32,
    #[serde(default = "default_height")]
    height: u32,
}

fn default_width() -> u32 { 800 }
fn default_height() -> u32 { 600 }

async fn render_svg(
    Query(params): Query<RenderParams>,
) -> Result<impl IntoResponse, StatusCode> {
    // Load and parse SVG
    let svg_data = std::fs::read(&params.path)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Create pixmap and render
    let mut pixmap = tiny_skia::Pixmap::new(params.width, params.height)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    resvg::render(&tree,
                  usvg::FitTo::Size(params.width, params.height),
                  tiny_skia::Transform::identity(),
                  &mut pixmap.as_mut());

    // Return PNG
    let png_data = pixmap.encode_png()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        png_data,
    ))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/render", get(render_svg));

    axum::Server::bind(&"127.0.0.1:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## Dynamic Open Graph Images

Generate social share images with dynamic content:

```rust
async fn generate_og_image(
    title: String,
    author: String,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Load SVG template
    let mut svg_template = std::fs::read_to_string("templates/og-image.svg")?;

    // Inject dynamic data (escape HTML entities!)
    svg_template = svg_template
        .replace("{{TITLE}}", &html_escape::encode_text(&title))
        .replace("{{AUTHOR}}", &html_escape::encode_text(&author));

    // Set up font database with embedded fonts
    let mut fontdb = usvg::fontdb::Database::new();
    let font_data = include_bytes!("../assets/Roboto-Regular.ttf");
    fontdb.load_font_data(font_data.to_vec());

    let opts = usvg::Options {
        fontdb,
        ..Default::default()
    };

    // Parse with custom options
    let tree = usvg::Tree::from_data(svg_template.as_bytes(), &opts)?;

    // Render at Open Graph dimensions (1200x630)
    let mut pixmap = tiny_skia::Pixmap::new(1200, 630)?;

    resvg::render(&tree, usvg::FitTo::Size(1200, 630),
                  tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    Ok(pixmap.encode_png()?)
}

// Actix-web route
async fn og_image(
    Query(params): Query<OgParams>,
) -> Result<HttpResponse, actix_web::Error> {
    let png_data = generate_og_image(params.title, params.author)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(png_data))
}
```

## Caching Strategy for Production

Implement caching to avoid re-rendering identical SVGs:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

struct SvgRenderer {
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    fontdb: usvg::fontdb::Database,
}

impl SvgRenderer {
    fn new() -> Self {
        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_system_fonts();

        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            fontdb,
        }
    }

    async fn render(&self, svg_path: &str, width: u32, height: u32)
        -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let cache_key = format!("{}_{}x{}", svg_path, width, height);

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(png_data) = cache.get(&cache_key) {
                return Ok(png_data.clone());
            }
        }

        // Render if not cached
        let svg_data = std::fs::read(svg_path)?;
        let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

        let mut pixmap = tiny_skia::Pixmap::new(width, height)
            .ok_or("Failed to create pixmap")?;

        resvg::render(&tree, usvg::FitTo::Size(width, height),
                      tiny_skia::Transform::identity(), &mut pixmap.as_mut());

        let png_data = pixmap.encode_png()?;

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, png_data.clone());
        }

        Ok(png_data)
    }
}

// Usage in Axum
#[derive(Clone)]
struct AppState {
    renderer: Arc<SvgRenderer>,
}

async fn render_svg_cached(
    State(state): State<AppState>,
    Query(params): Query<RenderParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let png_data = state.renderer
        .render(&params.path, params.width, params.height)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        png_data,
    ))
}
```

## Rate Limiting

Protect your service from abuse:

```rust
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

#[tokio::main]
async fn main() {
    // Configure rate limiting (10 requests per minute)
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(10)
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    let governor_layer = GovernorLayer {
        config: Box::leak(governor_conf),
    };

    let app = Router::new()
        .route("/render", get(render_svg))
        .layer(ServiceBuilder::new().layer(governor_layer));

    axum::Server::bind(&"127.0.0.1:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## Error Handling Best Practices

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum SvgRenderError {
    #[error("Failed to read SVG file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Failed to parse SVG: {0}")]
    Parse(String),

    #[error("Failed to create pixmap")]
    PixmapCreation,

    #[error("Failed to encode PNG: {0}")]
    PngEncode(String),
}

impl IntoResponse for SvgRenderError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            SvgRenderError::FileRead(_) => (StatusCode::NOT_FOUND, self.to_string()),
            SvgRenderError::Parse(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            SvgRenderError::PixmapCreation => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            SvgRenderError::PngEncode(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, error_message).into_response()
    }
}
```
