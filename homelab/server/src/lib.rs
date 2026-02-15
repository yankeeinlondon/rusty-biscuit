//! Homelab REST server library
//!
//! This module exposes the router building function and types needed for testing.

pub mod config;
pub mod error;
pub mod handlers;
pub mod state;

use axum::{Json, Router, extract::State, response::Html, routing::get};
use axum::response::IntoResponse;
use homelab::arcam::Arcam;
use serde::Serialize;
use serde_json::json;
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
        .route("/status", get(status))
        .merge(Scalar::with_url("/explore", api))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// --- Index Page ---

/// Device status for a single device (serializable for the `/status` endpoint).
#[derive(Serialize, Clone)]
struct DeviceStatusJson {
    css_class: &'static str,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<serde_json::Value>,
}

/// Combined status for all devices.
#[derive(Serialize)]
struct StatusResponse {
    sony: DeviceStatusJson,
    arcam: DeviceStatusJson,
}

/// Probe the Sony receiver and return its status.
///
/// When the receiver is active, fetches volume and current input
/// for dashboard instrumentation (best-effort — errors are silenced).
async fn probe_sony(state: &AppState) -> DeviceStatusJson {
    match &state.sony {
        None => DeviceStatusJson {
            css_class: "grey",
            label: "Not configured".to_string(),
            detail: None,
        },
        Some(sony) => match timeout(state.request_timeout, sony.get_power_status()).await {
            Ok(Ok(status)) => match status.as_str() {
                "active" => {
                    // Best-effort instrumentation: volume + current input
                    // Sequential calls required (Sony allows one TCP connection)
                    let detail = timeout(state.request_timeout, async {
                        let volume = sony.get_volume().await.ok();
                        let input = sony.get_current_input().await.ok();
                        let mut d = serde_json::Map::new();
                        if let Some(v) = volume {
                            d.insert("volume".into(), json!(v.volume));
                            d.insert("max_volume".into(), json!(v.max_volume));
                            d.insert("muted".into(), json!(v.mute == "on"));
                        }
                        if let Some(i) = input {
                            d.insert("input".into(), json!(format_input_name(&i.uri)));
                        }
                        serde_json::Value::Object(d)
                    })
                    .await
                    .ok();

                    DeviceStatusJson {
                        css_class: "green",
                        label: "Power: on".to_string(),
                        detail,
                    }
                }
                "standby" => DeviceStatusJson {
                    css_class: "amber",
                    label: "Power: off".to_string(),
                    detail: None,
                },
                other => DeviceStatusJson {
                    css_class: "amber",
                    label: format!("Power: {other}"),
                    detail: None,
                },
            },
            Ok(Err(e)) => DeviceStatusJson {
                css_class: "red",
                label: format!("Error: {e}"),
                detail: None,
            },
            Err(_) => DeviceStatusJson {
                css_class: "red",
                label: "Timeout".to_string(),
                detail: None,
            },
        },
    }
}

/// Converts a camelCase string into Title Case with spaces.
///
/// ## Examples
///
/// - `"mediaBox"` -> `"Media Box"`
/// - `"appleTv"` -> `"Apple Tv"`
/// - `"hdmi"` -> `"Hdmi"`
fn camel_to_title_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push(' ');
        }
        if i == 0 {
            result.extend(c.to_uppercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Formats a Sony input URI into a human-readable name.
///
/// ## Examples
///
/// - `extInput:hdmi?port=1` -> `"HDMI 1"`
/// - `extInput:bd` -> `"BD/DVD"`
/// - `radio:fm?contentId=0` -> `"FM"`
/// - `extInput:mediaBox` -> `"Media Box"` (camelCase fallback)
fn format_input_name(uri: &str) -> String {
    // Strip known scheme prefixes
    let rest = uri
        .strip_prefix("extInput:")
        .or_else(|| uri.strip_prefix("radio:"))
        .or_else(|| uri.strip_prefix("dlna:"))
        .or_else(|| uri.strip_prefix("storage:"))
        .unwrap_or(uri);

    // Split base from query params
    let (base, query) = rest.split_once('?').unwrap_or((rest, ""));

    let name = match base {
        "hdmi" => "HDMI".to_string(),
        "bd" => "BD/DVD".to_string(),
        "video" => "Video".to_string(),
        "game" => "Game".to_string(),
        "sacd_cd" => "SA-CD/CD".to_string(),
        "tv" => "TV".to_string(),
        "sat_catv" => "SAT/CATV".to_string(),
        "fm" => "FM".to_string(),
        "am" => "AM".to_string(),
        "music" => "DLNA".to_string(),
        other => camel_to_title_case(other),
    };

    // Extract port number from query (e.g. "port=1")
    let port = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("port="));

    match port {
        Some(p) => format!("{name} {p}"),
        None => name,
    }
}

#[cfg(test)]
mod format_tests {
    use super::{camel_to_title_case, format_input_name};

    #[test]
    fn hdmi_with_port() {
        assert_eq!(format_input_name("extInput:hdmi?port=1"), "HDMI 1");
        assert_eq!(format_input_name("extInput:hdmi?port=3"), "HDMI 3");
    }

    #[test]
    fn named_inputs() {
        assert_eq!(format_input_name("extInput:bd"), "BD/DVD");
        assert_eq!(format_input_name("extInput:tv"), "TV");
        assert_eq!(format_input_name("extInput:sat_catv"), "SAT/CATV");
        assert_eq!(format_input_name("extInput:sacd_cd"), "SA-CD/CD");
    }

    #[test]
    fn radio() {
        assert_eq!(format_input_name("radio:fm?contentId=0"), "FM");
        assert_eq!(format_input_name("radio:am"), "AM");
    }

    #[test]
    fn unknown_passthrough() {
        assert_eq!(format_input_name("something:else"), "Something:else");
    }

    #[test]
    fn camel_case_input() {
        assert_eq!(format_input_name("extInput:mediaBox"), "Media Box");
    }

    #[test]
    fn camel_to_title_basics() {
        assert_eq!(camel_to_title_case("mediaBox"), "Media Box");
        assert_eq!(camel_to_title_case("appleTv"), "Apple Tv");
        assert_eq!(camel_to_title_case("hdmi"), "Hdmi");
        assert_eq!(camel_to_title_case(""), "");
        assert_eq!(camel_to_title_case("a"), "A");
    }
}

/// Probe the Arcam amplifier and return its status.
///
/// When the amplifier is on, fetches mute status and amplifier mode
/// for dashboard instrumentation (best-effort — errors are silenced).
async fn probe_arcam(state: &AppState) -> DeviceStatusJson {
    match &state.arcam_host {
        None => DeviceStatusJson {
            css_class: "grey",
            label: "Not configured".to_string(),
            detail: None,
        },
        Some(host) => {
            let arcam = Arcam::from(host.as_str());
            match timeout(state.request_timeout, arcam.request_power_state()).await {
                Ok(Ok(true)) => {
                    // Best-effort instrumentation: mute + mode
                    // Each call opens a new TCP connection (Arcam is single-connection)
                    let detail = timeout(state.request_timeout, async {
                        let arcam = Arcam::from(host.as_str());
                        let muted = arcam.get_mute_status().await.ok();
                        let arcam = Arcam::from(host.as_str());
                        let mode_byte = arcam.get_amplifier_mode().await.ok();
                        let mut d = serde_json::Map::new();
                        if let Some(m) = muted {
                            d.insert("muted".into(), json!(m));
                        }
                        if let Some(mb) = mode_byte {
                            let mode = match mb {
                                0 => "Stereo",
                                1 => "Bridged",
                                2 => "Dual Mono",
                                _ => "Unknown",
                            };
                            d.insert("mode".into(), json!(mode));
                        }
                        serde_json::Value::Object(d)
                    })
                    .await
                    .ok();

                    DeviceStatusJson {
                        css_class: "green",
                        label: "Power: on".to_string(),
                        detail,
                    }
                }
                Ok(Ok(false)) => DeviceStatusJson {
                    css_class: "amber",
                    label: "Power: off".to_string(),
                    detail: None,
                },
                Ok(Err(e)) => DeviceStatusJson {
                    css_class: "red",
                    label: format!("Error: {e}"),
                    detail: None,
                },
                Err(_) => DeviceStatusJson {
                    css_class: "red",
                    label: "Timeout".to_string(),
                    detail: None,
                },
            }
        }
    }
}

/// JSON status endpoint polled by the index page.
async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let (sony, arcam) = tokio::join!(probe_sony(&state), probe_arcam(&state));
    Json(StatusResponse { sony, arcam })
}

async fn index(State(state): State<AppState>) -> Html<String> {
    let (sony_status, arcam_status) = tokio::join!(probe_sony(&state), probe_arcam(&state));

    let sony_host = state
        .sony
        .as_ref()
        .map(|s| format!("{}:{}", s.host(), s.port()))
        .unwrap_or_default();
    let arcam_host = state
        .arcam_host
        .as_deref()
        .map(|h| format!("{h}:50000"))
        .unwrap_or_default();

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Homelab Server</title>
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 700px; margin: 40px auto; padding: 0 20px; background: #1a1a2e; color: #e0e0e0; }}
  h1 {{ color: #fff; margin-bottom: 8px; }}
  .subtitle {{ color: #888; margin-bottom: 32px; }}
  .device {{ background: #16213e; border-radius: 8px; padding: 16px 20px; margin-bottom: 12px; display: flex; align-items: center; gap: 16px; }}
  .device-info {{ flex: 1; min-width: 0; }}
  .dot {{ width: 14px; height: 14px; border-radius: 50%; flex-shrink: 0; align-self: flex-start; margin-top: 3px; transition: background 0.3s, box-shadow 0.3s; }}
  .green {{ background: #4caf50; box-shadow: 0 0 8px #4caf5088; }}
  .amber {{ background: #ff9800; box-shadow: 0 0 8px #ff980088; }}
  .red {{ background: #f44336; box-shadow: 0 0 8px #f4433688; }}
  .grey {{ background: #666; box-shadow: none; }}
  .device-name {{ font-weight: 600; }}
  .device-detail {{ color: #999; font-size: 0.85em; }}
  .host {{ color: #666; font-size: 0.8em; }}
  .instruments {{ display: flex; flex-direction: column; gap: 6px; align-items: flex-end; align-self: flex-start; margin-top: 2px; flex-shrink: 0; }}
  .badge-row {{ display: flex; gap: 6px; flex-wrap: wrap; justify-content: flex-end; }}
  .badge {{ background: #0f3460; border: 1px solid #1a4a7a; border-radius: 4px; padding: 4px 10px; font-size: 0.75em; color: #a8c8e8; white-space: nowrap; letter-spacing: 0.02em; }}
  .badge-muted {{ background: #4a1a1a; border-color: #7a2a2a; color: #e88; }}
  .volume-wrap {{ display: flex; align-items: center; gap: 8px; width: 140px; }}
  .volume-track {{ flex: 1; height: 5px; background: #0a1628; border-radius: 3px; overflow: hidden; }}
  .volume-fill {{ height: 100%; border-radius: 3px; background: linear-gradient(90deg, #1a6b5a, #4caf50); transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1); box-shadow: 0 0 6px #4caf5044; }}
  .volume-fill.muted {{ background: linear-gradient(90deg, #5a1a1a, #c44); box-shadow: 0 0 6px #c4444444; }}
  .volume-label {{ font-size: 0.7em; color: #556; min-width: 18px; text-align: right; font-variant-numeric: tabular-nums; }}
  .explore {{ margin-top: 24px; color: #b5b5b5; }}
  .explore a {{ color: #7cc4ff; text-decoration: none; }}
  .explore a:hover {{ text-decoration: underline; }}
</style>
</head>
<body>
<h1>Homelab Server</h1>
<p class="subtitle">AV Device Control</p>

<div class="device">
  <div class="dot {}" id="sony-dot"></div>
  <div class="device-info">
    <div class="device-name">Sony Receiver</div>
    <div class="device-detail" id="sony-label">{}</div>
    <div class="host">{}</div>
  </div>
  <div class="instruments" id="sony-instruments"></div>
</div>

<div class="device">
  <div class="dot {}" id="arcam-dot"></div>
  <div class="device-info">
    <div class="device-name">Arcam Amplifier</div>
    <div class="device-detail" id="arcam-label">{}</div>
    <div class="host">{}</div>
  </div>
  <div class="instruments" id="arcam-instruments"></div>
</div>

<p class="explore">Try interacting with the API by using the <a href="./explore">explore</a> UI.</p>

<script>
(function() {{
  const CLASSES = ["green", "amber", "red", "grey"];

  function setDot(el, cls) {{
    if (!el.classList.contains(cls)) {{
      CLASSES.forEach(c => el.classList.remove(c));
      el.classList.add(cls);
    }}
  }}

  function setText(el, text) {{
    if (el.textContent !== text) {{
      el.textContent = text;
    }}
  }}

  function renderBadges(el, badges) {{
    if (!badges.length) {{ el.innerHTML = ''; return; }}
    const html = '<div class="badge-row">' + badges.map(b =>
      '<span class="badge' + (b.cls ? ' ' + b.cls : '') + '">' + b.text + '</span>'
    ).join('') + '</div>';
    if (el.innerHTML !== html) el.innerHTML = html;
  }}

  function renderSony(el, detail) {{
    if (!detail) {{ el.innerHTML = ''; return; }}
    let html = '<div class="badge-row">';
    if (detail.input) html += '<span class="badge">' + detail.input + '</span>';
    if (detail.muted) html += '<span class="badge badge-muted">MUTED</span>';
    html += '</div>';
    if (detail.volume !== undefined) {{
      const max = detail.max_volume || 100;
      const pct = Math.min(100, Math.round((detail.volume / max) * 100));
      const cls = detail.muted ? ' muted' : '';
      html += '<div class="volume-wrap">';
      html += '<div class="volume-track"><div class="volume-fill' + cls + '" style="width:' + pct + '%"></div></div>';
      html += '<span class="volume-label">' + detail.volume + '</span>';
      html += '</div>';
    }}
    if (el.innerHTML !== html) el.innerHTML = html;
  }}

  function arcamBadges(detail) {{
    if (!detail) return [];
    const b = [];
    if (detail.mode) b.push({{ text: detail.mode }});
    if (detail.muted) b.push({{ text: 'MUTED', cls: 'badge-muted' }});
    return b;
  }}

  async function poll() {{
    try {{
      const resp = await fetch("/status");
      if (!resp.ok) return;
      const data = await resp.json();

      setDot(document.getElementById("sony-dot"), data.sony.css_class);
      setText(document.getElementById("sony-label"), data.sony.label);
      renderSony(document.getElementById("sony-instruments"), data.sony.detail);

      setDot(document.getElementById("arcam-dot"), data.arcam.css_class);
      setText(document.getElementById("arcam-label"), data.arcam.label);
      renderBadges(document.getElementById("arcam-instruments"), arcamBadges(data.arcam.detail));
    }} catch (_) {{
      // Network error — leave current state, retry next tick
    }}
  }}

  poll();
  setInterval(poll, 3000);
}})();
</script>

</body>
</html>"#,
        sony_status.css_class,
        sony_status.label,
        sony_host,
        arcam_status.css_class,
        arcam_status.label,
        arcam_host,
    ))
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
