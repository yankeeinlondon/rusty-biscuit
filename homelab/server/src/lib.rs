//! Homelab REST server library
//!
//! This module exposes the router building function and types needed for testing.

pub mod config;
pub mod error;
pub mod handlers;
pub mod state;

use axum::{Json, Router, extract::State, response::Html, routing::get};
use homelab::arcam::Arcam;
use serde::Serialize;
use tokio::time::timeout;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

use state::AppState;

/// OpenAPI documentation metadata
#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "sony_receiver", description = "Sony ES receiver management and control"),
        (name = "arcam_amp", description = "Arcam amplifier management and control"),
    )
)]
struct ApiDoc;

/// Build the router with the given state.
///
/// This function is public to allow integration tests to build the router.
pub fn build_router(state: AppState) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(health))
        .routes(utoipa_axum::routes!(health_devices))
        // New routes (multi-device via config)
        .nest(
            "/sony_receiver",
            handlers::crud::sony_receiver_crud_routes().merge(handlers::sony::routes_with_name()),
        )
        .nest(
            "/arcam_amp",
            handlers::crud::arcam_amp_crud_routes().merge(handlers::arcam::routes_with_name()),
        )
        .split_for_parts();

    router
        // Legacy routes (not advertised in API docs)
        .nest("/sony", handlers::sony::routes().split_for_parts().0)
        .nest("/arcam", handlers::arcam::routes().split_for_parts().0)
        .route("/", get(index))
        .merge(Scalar::with_url("/explore", api))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// --- Index Page ---

async fn index(State(state): State<AppState>) -> Html<String> {
    // Probe Sony
    let sony_status = match &state.sony {
        None => DeviceStatus::NotConfigured,
        Some(sony) => match timeout(state.request_timeout, sony.get_power_status()).await {
            Ok(Ok(status)) => match status.as_str() {
                "active" => DeviceStatus::Green(format!("Power: {status}")),
                "standby" => DeviceStatus::Amber(format!("Power: {status}")),
                other => DeviceStatus::Amber(format!("Power: {other}")),
            },
            Ok(Err(e)) => DeviceStatus::Red(format!("Error: {e}")),
            Err(_) => DeviceStatus::Red("Timeout".to_string()),
        },
    };

    // Probe Arcam
    let arcam_status = match &state.arcam_host {
        None => DeviceStatus::NotConfigured,
        Some(host) => {
            let arcam = Arcam::from(host.as_str());
            match timeout(state.request_timeout, arcam.request_power_state()).await {
                Ok(Ok(on)) => {
                    if on {
                        DeviceStatus::Green("Power: on".to_string())
                    } else {
                        DeviceStatus::Amber("Power: standby".to_string())
                    }
                }
                Ok(Err(e)) => DeviceStatus::Red(format!("Error: {e}")),
                Err(_) => DeviceStatus::Red("Timeout".to_string()),
            }
        }
    };

    let sony_host = state
        .sony
        .as_ref()
        .map(|s| format!("{}:{}", s.host(), s.port()))
        .unwrap_or_default();
    let arcam_host = state.arcam_host.clone().unwrap_or_default();

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Homelab Server</title>
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 600px; margin: 40px auto; padding: 0 20px; background: #1a1a2e; color: #e0e0e0; }}
  h1 {{ color: #fff; margin-bottom: 8px; }}
  .subtitle {{ color: #888; margin-bottom: 32px; }}
  .device {{ background: #16213e; border-radius: 8px; padding: 16px 20px; margin-bottom: 12px; display: flex; align-items: center; gap: 16px; }}
  .dot {{ width: 14px; height: 14px; border-radius: 50%; flex-shrink: 0; }}
  .green {{ background: #4caf50; box-shadow: 0 0 8px #4caf5088; }}
  .amber {{ background: #ff9800; box-shadow: 0 0 8px #ff980088; }}
  .red {{ background: #f44336; box-shadow: 0 0 8px #f4433688; }}
  .grey {{ background: #666; }}
  .device-name {{ font-weight: 600; }}
  .device-detail {{ color: #999; font-size: 0.85em; }}
  .host {{ color: #666; font-size: 0.8em; }}
  .explore {{ margin-top: 24px; color: #b5b5b5; }}
  .explore a {{ color: #7cc4ff; text-decoration: none; }}
  .explore a:hover {{ text-decoration: underline; }}
</style>
</head>
<body>
<h1>Homelab Server</h1>
<p class="subtitle">AV Device Control</p>

<div class="device">
  <div class="dot {}"></div>
  <div>
    <div class="device-name">Sony Receiver</div>
    <div class="device-detail">{}</div>
    <div class="host">{}</div>
  </div>
</div>

<div class="device">
  <div class="dot {}"></div>
  <div>
    <div class="device-name">Arcam Amplifier</div>
    <div class="device-detail">{}</div>
    <div class="host">{}</div>
  </div>
</div>

<p class="explore">Try interacting with the API by using the <a href="./explore">explore</a> UI.</p>

</body>
</html>"#,
        sony_status.css_class(),
        sony_status.label(),
        sony_host,
        arcam_status.css_class(),
        arcam_status.label(),
        arcam_host,
    ))
}

enum DeviceStatus {
    Green(String),
    Amber(String),
    Red(String),
    NotConfigured,
}

impl DeviceStatus {
    fn css_class(&self) -> &'static str {
        match self {
            Self::Green(_) => "green",
            Self::Amber(_) => "amber",
            Self::Red(_) => "red",
            Self::NotConfigured => "grey",
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Green(s) | Self::Amber(s) | Self::Red(s) => s,
            Self::NotConfigured => "Not configured",
        }
    }
}

// --- Health Endpoints ---

#[derive(Serialize, ToSchema)]
struct HealthResponse {
    /// Server health status
    status: &'static str,
}

#[derive(Serialize, ToSchema)]
struct DeviceHealth {
    /// Whether the device is configured
    configured: bool,
    /// Device host address (if configured)
    host: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct DevicesHealthResponse {
    /// Sony receiver health
    sony: DeviceHealth,
    /// Arcam amplifier health
    arcam: DeviceHealth,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Server is healthy", body = HealthResponse),
    )
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "healthy" })
}

#[utoipa::path(
    get,
    path = "/health/devices",
    tag = "health",
    responses(
        (status = 200, description = "Device configuration status", body = DevicesHealthResponse),
    )
)]
async fn health_devices(State(state): State<AppState>) -> Json<DevicesHealthResponse> {
    Json(DevicesHealthResponse {
        sony: DeviceHealth {
            configured: state.sony.is_some(),
            host: state
                .sony
                .as_ref()
                .map(|s| format!("{}:{}", s.host(), s.port())),
        },
        arcam: DeviceHealth {
            configured: state.arcam_host.is_some(),
            host: state.arcam_host.clone(),
        },
    })
}
