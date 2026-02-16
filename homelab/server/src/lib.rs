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
                    // Best-effort instrumentation: volume, current input, native sources
                    // JSON-RPC calls are sequential (Sony allows one TCP connection);
                    // native web API (port 80) is a separate connection.
                    let detail = timeout(state.request_timeout, async {
                        let volume = sony.get_volume().await.ok();
                        let input = sony.get_current_input().await.ok();
                        let native = match sony.get_native_inputs().await {
                            Ok(n) => Some(n),
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to fetch native inputs");
                                None
                            }
                        };
                        let mut d = serde_json::Map::new();
                        if let Some(v) = volume {
                            d.insert("volume".into(), json!(v.volume));
                            d.insert("max_volume".into(), json!(v.max_volume));
                            d.insert("muted".into(), json!(v.mute == "on"));
                        }
                        let current_cat = input.as_ref().and_then(|i| {
                            match_uri_to_category(&i.uri, native.as_deref())
                        });
                        if let Some(inputs) = &native {
                            let source_list: Vec<serde_json::Value> = inputs
                                .iter()
                                .map(|i| {
                                    json!({
                                        "name": format_source_name(&i.name),
                                        "active": current_cat.as_deref() == Some(i.category.as_str()),
                                    })
                                })
                                .collect();
                            d.insert("sources".into(), json!(source_list));
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

/// Formats a source name for display.
///
/// Handles two input formats from the Sony native API:
/// - ALL-CAPS words: title-case long simple words, keep abbreviations
///   (words with `/`, `-`, or ≤2 chars) as-is
/// - camelCase: split at lowercase→uppercase transitions
///
/// ## Examples
///
/// - `"MEDIA BOX"` → `"Media Box"`
/// - `"BD/DVD"` → `"BD/DVD"` (abbreviation kept)
/// - `"TV"` → `"TV"` (short word kept)
/// - `"mediaBox"` → `"Media Box"` (camelCase split)
fn format_source_name(s: &str) -> String {
    let has_lower = s.chars().any(|c| c.is_lowercase());

    // All-uppercase path: title-case simple words, keep abbreviations as-is
    if !has_lower && s.len() > 1 {
        return s
            .split(' ')
            .map(|word| {
                // Keep abbreviations: short words or words with / or -
                if word.len() <= 2 || word.contains('/') || word.contains('-') {
                    return word.to_string();
                }
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        let rest: String =
                            chars.flat_map(|c| c.to_lowercase()).collect();
                        format!("{first}{rest}")
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }

    // camelCase splitting
    let mut result = String::with_capacity(s.len() + 4);
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 && chars[i - 1].is_lowercase() {
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

/// Matches a Sony input URI to a native input category.
///
/// The receiver reports the current input as `extInput:{icon}` where the
/// icon value corresponds to the `icon` field from the native input
/// configuration. For example, `extInput:mediaBox` matches the native
/// input whose `icon == "mediabox"` (case-insensitive).
fn match_uri_to_category(
    uri: &str,
    native: Option<&[homelab::sony_receiver::NativeInputConfig]>,
) -> Option<String> {
    let rest = uri.strip_prefix("extInput:")?;
    let (base, _query) = rest.split_once('?').unwrap_or((rest, ""));
    let base_lower = base.to_lowercase();

    native?
        .iter()
        .find(|n| n.icon.to_lowercase() == base_lower)
        .map(|n| n.category.clone())
}

/// Formats a Sony input URI into a human-readable name.
///
/// Formats a Sony input URI into a human-readable name.
///
/// ## Examples
///
/// - `extInput:hdmi?port=1` -> `"HDMI 1"`
/// - `extInput:bd` -> `"BD/DVD"`
/// - `radio:fm?contentId=0` -> `"FM"`
/// - `extInput:mediaBox` -> `"Media Box"` (camelCase fallback)
#[cfg(test)]
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
        other => format_source_name(other),
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
    use super::{format_source_name, format_input_name, match_uri_to_category};

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
    fn uri_to_category_matches_icon() {
        use homelab::sony_receiver::NativeInputConfig;
        // The receiver reports current input as extInput:{icon}
        // where icon is the native config's icon field (case-insensitive)
        let inputs = vec![
            NativeInputConfig {
                category: "STB".to_string(),
                name: "MEDIA BOX".to_string(),
                hdmi_assign: "in2".to_string(),
                icon: "mediabox".to_string(),
                visible: false,
                sound_field: String::new(),
            },
            NativeInputConfig {
                category: "GAME".to_string(),
                name: "GAME".to_string(),
                hdmi_assign: "in1".to_string(),
                icon: "game".to_string(),
                visible: false,
                sound_field: String::new(),
            },
            NativeInputConfig {
                category: "TV".to_string(),
                name: "TV".to_string(),
                hdmi_assign: "none".to_string(),
                icon: "tv".to_string(),
                visible: false,
                sound_field: String::new(),
            },
        ];
        // extInput:mediaBox matches icon "mediabox" (case-insensitive)
        assert_eq!(
            match_uri_to_category("extInput:mediaBox", Some(&inputs)),
            Some("STB".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:game", Some(&inputs)),
            Some("GAME".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:tv", Some(&inputs)),
            Some("TV".to_string())
        );
        // No match
        assert_eq!(
            match_uri_to_category("extInput:unknown", Some(&inputs)),
            None
        );
        // Non-extInput prefix
        assert_eq!(match_uri_to_category("radio:fm", Some(&inputs)), None);
        // No native inputs
        assert_eq!(match_uri_to_category("extInput:game", None), None);
    }

    #[test]
    fn format_source_name_camel_case() {
        assert_eq!(format_source_name("mediaBox"), "Media Box");
        assert_eq!(format_source_name("appleTv"), "Apple Tv");
        assert_eq!(format_source_name("hdmi"), "Hdmi");
        assert_eq!(format_source_name(""), "");
        assert_eq!(format_source_name("a"), "A");
    }

    #[test]
    fn format_source_name_uppercase() {
        // Native API returns ALL-CAPS names — title-case long words,
        // keep abbreviations (short words, words with / or -)
        assert_eq!(format_source_name("GAME"), "Game");
        assert_eq!(format_source_name("MEDIA BOX"), "Media Box");
        assert_eq!(format_source_name("BD/DVD"), "BD/DVD");
        assert_eq!(format_source_name("SAT/CATV"), "SAT/CATV");
        assert_eq!(format_source_name("SA-CD/CD"), "SA-CD/CD");
        assert_eq!(format_source_name("TV"), "TV");
        assert_eq!(format_source_name("AUX"), "Aux");
        assert_eq!(format_source_name("VIDEO"), "Video");
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
        .map(|s| format!("{}<span class=\"port\">:{}</span>", s.host(), s.port()))
        .unwrap_or_default();
    let arcam_host = state
        .arcam_host
        .as_deref()
        .map(|h| format!("{h}<span class=\"port\">:50000</span>"))
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
  .device {{ background: #16213e; border-radius: 8px; padding: 16px 20px; margin-bottom: 12px; }}
  .device-row {{ display: flex; align-items: center; gap: 16px; }}
  .device-info {{ flex: 1; min-width: 0; }}
  .dot {{ width: 28px; height: 28px; border-radius: 50%; flex-shrink: 0; transition: background 0.3s, box-shadow 0.3s; cursor: pointer; border: 2px solid transparent; }}
  .dot:not(.grey):hover {{ border-color: rgba(255,255,255,0.15); }}
  .dot:not(.grey):active {{ transform: scale(0.9); }}
  .dot.pressing {{ animation: dot-press 0.4s ease-out; }}
  .green {{ background: #4caf50; box-shadow: 0 0 10px #4caf5088; }}
  .amber {{ background: #ff9800; box-shadow: 0 0 10px #ff980088; }}
  .red {{ background: #f44336; box-shadow: 0 0 10px #f4433688; }}
  .grey {{ background: #666; box-shadow: none; cursor: default; }}
  @keyframes dot-press {{ 0% {{ transform: scale(1); box-shadow: 0 0 10px currentColor; }} 30% {{ transform: scale(0.8); box-shadow: 0 0 20px currentColor; }} 100% {{ transform: scale(1); box-shadow: 0 0 10px currentColor; }} }}
  .device-name {{ font-weight: 600; }}
  .device-detail {{ color: #999; font-size: 0.85em; }}
  .host {{ color: #666; font-size: 0.8em; }}
  .port {{ opacity: 0.5; }}
  .instruments {{ display: flex; flex-direction: column; gap: 6px; align-items: flex-end; flex-shrink: 0; }}
  .badge-row {{ display: flex; gap: 6px; flex-wrap: wrap; }}
  .badge {{ background: #0f3460; border: 1px solid #1a4a7a; border-radius: 4px; padding: 4px 10px; font-size: 0.75em; color: #a8c8e8; white-space: nowrap; letter-spacing: 0.02em; }}
  .badge-muted {{ background: #4a2e0a; border-color: #7a5a1a; color: #f0a030; }}
  .volume-wrap {{ display: flex; align-items: center; gap: 8px; width: 140px; }}
  .volume-track {{ flex: 1; height: 5px; background: #0a1628; border-radius: 3px; overflow: hidden; }}
  .volume-fill {{ height: 100%; border-radius: 3px; background: linear-gradient(90deg, #1a6b5a, #4caf50); transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1); box-shadow: 0 0 6px #4caf5044; }}
  .volume-fill.muted {{ background: linear-gradient(90deg, #5a1a1a, #c44); box-shadow: 0 0 6px #c4444444; }}
  .volume-label {{ font-size: 0.7em; color: #556; min-width: 18px; text-align: right; font-variant-numeric: tabular-nums; }}
  .sources {{ display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.35s ease, margin-top 0.35s ease, padding-top 0.35s ease; overflow: hidden; margin-top: 0; padding-top: 0; }}
  .sources > .badge-row {{ min-height: 0; }}
  .sources.visible {{ grid-template-rows: 1fr; margin-top: 12px; padding-top: 12px; border-top: 1px solid #1a2a45; }}
  .sources .badge-row {{ justify-content: flex-end; }}
  .sources .badge {{ transition: background 0.2s, border-color 0.2s, color 0.2s; }}
  .badge-dim {{ background: transparent; border-color: #182540; color: #3a4a60; }}
  .explore {{ margin-top: 24px; color: #b5b5b5; }}
  .explore a {{ color: #7cc4ff; text-decoration: none; }}
  .explore a:hover {{ text-decoration: underline; }}
  .info-popover {{ margin: 0; border: 1px solid #1a4a7a; border-radius: 8px; padding: 14px 18px; background: #16213e; color: #c8d8e8; font-size: 0.85em; line-height: 1.6; max-width: 320px; box-shadow: 0 8px 32px rgba(0,0,0,0.5); position-area: bottom; position-try-fallbacks: flip-block; }}
  .info-popover, .info-popover:popover-open {{ opacity: 1; transform: translateY(0); transition: opacity 0.2s ease, transform 0.2s ease, overlay 0.2s allow-discrete, display 0.2s allow-discrete; }}
  .info-popover:not(:popover-open) {{ opacity: 0; transform: translateY(-4px); }}
  @starting-style {{ .info-popover:popover-open {{ opacity: 0; transform: translateY(-4px); }} }}
  .info-popover h4 {{ margin: 0 0 8px; color: #fff; font-size: 0.95em; }}
  .info-popover dl {{ margin: 0; display: grid; grid-template-columns: auto 1fr; gap: 4px 12px; }}
  .info-popover dt {{ color: #7cc4ff; font-weight: 600; white-space: nowrap; }}
  .info-popover dd {{ margin: 0; color: #99aabb; }}
  .info-popover .note {{ margin-top: 8px; padding-top: 8px; border-top: 1px solid #1a2a45; color: #667; font-size: 0.9em; font-style: italic; }}
  #sony-dot {{ anchor-name: --sony-dot; }}
  #arcam-dot {{ anchor-name: --arcam-dot; }}
  #pop-sony-power {{ position-anchor: --sony-dot; }}
  #pop-arcam-power {{ position-anchor: --arcam-dot; }}
  #arcam-mode-badge {{ anchor-name: --arcam-mode; cursor: help; }}
  #pop-arcam-mode {{ position-anchor: --arcam-mode; }}
</style>
</head>
<body>
<h1>Homelab Server</h1>
<p class="subtitle">AV Device Control</p>

<div class="device">
  <div class="device-row">
    <div class="dot {}" id="sony-dot"></div>
    <div class="device-info">
      <div class="device-name">Sony Receiver</div>
      <div class="device-detail" id="sony-label">{}</div>
      <div class="host">{}</div>
    </div>
    <div class="instruments" id="sony-instruments"></div>
  </div>
  <div class="sources" id="sony-sources"></div>
</div>

<div class="device">
  <div class="device-row">
    <div class="dot {}" id="arcam-dot"></div>
    <div class="device-info">
      <div class="device-name">Arcam Amplifier</div>
      <div class="device-detail" id="arcam-label">{}</div>
      <div class="host">{}</div>
    </div>
    <div class="instruments" id="arcam-instruments"></div>
  </div>
</div>

<p class="explore">Try interacting with the API by using the <a href="./explore">explore</a> UI.</p>

<div id="pop-sony-power" popover="manual" class="info-popover">
  <h4>Sony Receiver &mdash; Power States</h4>
  <dl>
    <dt>Active</dt><dd>Powered on and fully operational. All features and controls are available.</dd>
    <dt>Standby</dt><dd>Low-power network standby. Not truly &ldquo;off&rdquo;&hairsp;&mdash;&hairsp;the receiver maintains network connectivity so the API can wake it.</dd>
    <dt>Unreachable</dt><dd>Truly powered off or unplugged. The API cannot communicate with the device in this state.</dd>
  </dl>
  <p class="note">Turning &ldquo;off&rdquo; via this UI puts the receiver into Standby, not a full power-off.</p>
</div>

<div id="pop-arcam-power" popover="manual" class="info-popover">
  <h4>Arcam Amplifier &mdash; Power States</h4>
  <dl>
    <dt>On</dt><dd>Powered on and fully operational. All features and controls are available.</dd>
    <dt>Standby</dt><dd>Low-power network standby. The amplifier maintains network connectivity; a heartbeat signal is sent every 10&nbsp;minutes to keep the interface alive.</dd>
    <dt>Unreachable</dt><dd>Truly powered off or unplugged. The API cannot communicate with the device in this state.</dd>
  </dl>
  <p class="note">Turning &ldquo;off&rdquo; via this UI puts the amplifier into Standby, not a full power-off.</p>
</div>

<div id="pop-arcam-mode" popover="manual" class="info-popover">
  <h4>Arcam Amplifier &mdash; Mode</h4>
  <dl>
    <dt>Stereo</dt><dd>Standard two-channel operation. Left and right speakers receive independent audio signals.</dd>
    <dt>Bridged</dt><dd>Both amplifier channels are combined to drive a single speaker pair, roughly doubling the output wattage.</dd>
    <dt>Dual Mono</dt><dd>Both channels receive the same mono signal, amplified independently. Used for bi-amping a single speaker pair.</dd>
  </dl>
</div>

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

  let pollSuppressedUntil = 0;
  let lastSonyDetail = null;

  function togglePower(dotEl, device) {{
    if (dotEl.classList.contains('grey')) return;
    const isOn = dotEl.classList.contains('green');
    // Animate press
    dotEl.classList.remove('pressing');
    void dotEl.offsetWidth; // reflow to restart animation
    dotEl.classList.add('pressing');
    dotEl.addEventListener('animationend', () => dotEl.classList.remove('pressing'), {{ once: true }});
    // Optimistic UI update — immediate, no waiting
    const labelEl = document.getElementById(device === 'sony' ? 'sony-label' : 'arcam-label');
    setDot(dotEl, isOn ? 'amber' : 'green');
    setText(labelEl, isOn ? 'Power: off' : 'Power: on');
    if (device === 'sony') {{
      const srcEl = document.getElementById('sony-sources');
      const insEl = document.getElementById('sony-instruments');
      if (isOn) {{
        // Turning off: collapse sources and volume
        renderSony(insEl, srcEl, null);
      }} else if (lastSonyDetail) {{
        // Turning on: restore last known sources and volume
        renderSony(insEl, srcEl, lastSonyDetail);
      }}
    }}
    // Suppress polling while receiver transitions:
    // power-off is slow (~4s), power-on from standby is fast (~1s)
    const delay = isOn ? 4000 : 1000;
    pollSuppressedUntil = Date.now() + delay;
    // Send API call, then resume polling
    const done = () => setTimeout(poll, delay);
    if (device === 'sony') {{
      fetch('/sony/power', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{ active: !isOn }})
      }}).then(done, done);
    }} else if (device === 'arcam') {{
      const action = isOn ? 'off' : 'on';
      fetch('/arcam/power/' + action, {{ method: 'POST' }}).then(done, done);
    }}
  }}

  document.getElementById('sony-dot').addEventListener('click', function() {{
    togglePower(this, 'sony');
  }});
  document.getElementById('arcam-dot').addEventListener('click', function() {{
    togglePower(this, 'arcam');
  }});

  function renderBadges(el, badges) {{
    if (!badges.length) {{ el.innerHTML = ''; return; }}
    const prev = el.innerHTML;
    const html = '<div class="badge-row">' + badges.map(b =>
      '<span' + (b.id ? ' id="' + b.id + '"' : '') + ' class="badge' + (b.cls ? ' ' + b.cls : '') + '">' + b.text + '</span>'
    ).join('') + '</div>';
    if (prev !== html) el.innerHTML = html;
  }}

  function renderSony(instrumentsEl, sourcesEl, detail) {{
    if (!detail) {{
      instrumentsEl.innerHTML = '';
      sourcesEl.innerHTML = '';
      sourcesEl.classList.remove('visible');
      return;
    }}
    // Volume bar in instruments (right side)
    let volHtml = '';
    if (detail.volume !== undefined) {{
      const max = detail.max_volume || 100;
      const pct = Math.min(100, Math.round((detail.volume / max) * 100));
      const cls = detail.muted ? ' muted' : '';
      volHtml += '<div class="volume-wrap">';
      volHtml += '<div class="volume-track"><div class="volume-fill' + cls + '" style="width:' + pct + '%"></div></div>';
      volHtml += '<span class="volume-label">' + detail.volume + '</span>';
      volHtml += '</div>';
    }}
    if (instrumentsEl.innerHTML !== volHtml) instrumentsEl.innerHTML = volHtml;
    // Source badges in full-width row below
    if (detail.sources && detail.sources.length) {{
      const P = ['Media Box', 'Game'];
      const sorted = detail.sources.slice().sort((a, b) => {{
        const ai = P.indexOf(a.name); const bi = P.indexOf(b.name);
        if (ai !== -1 && bi !== -1) return ai - bi;
        if (ai !== -1) return -1;
        if (bi !== -1) return 1;
        return a.name.localeCompare(b.name);
      }});
      const srcHtml = '<div class="badge-row">' + sorted.map(s =>
        '<span class="badge' + (s.active ? '' : ' badge-dim') + '">' + s.name + '</span>'
      ).join('') + '</div>';
      if (sourcesEl.innerHTML !== srcHtml) sourcesEl.innerHTML = srcHtml;
      sourcesEl.classList.add('visible');
    }} else {{
      sourcesEl.innerHTML = '';
      sourcesEl.classList.remove('visible');
    }}
  }}

  function arcamBadges(detail) {{
    if (!detail) return [];
    const b = [];
    if (detail.mode) b.push({{ text: detail.mode, id: 'arcam-mode-badge' }});
    if (detail.muted) b.push({{ text: 'MUTED', cls: 'badge-muted' }});
    return b;
  }}

  function hoverPopover(trigger, pop) {{
    let showT, hideT;
    trigger.addEventListener('mouseenter', () => {{
      clearTimeout(hideT);
      showT = setTimeout(() => {{ try {{ pop.showPopover(); }} catch(_) {{}} }}, 300);
    }});
    trigger.addEventListener('mouseleave', () => {{
      clearTimeout(showT);
      hideT = setTimeout(() => {{ try {{ pop.hidePopover(); }} catch(_) {{}} }}, 150);
    }});
    pop.addEventListener('mouseenter', () => clearTimeout(hideT));
    pop.addEventListener('mouseleave', () => {{ try {{ pop.hidePopover(); }} catch(_) {{}} }});
  }}

  hoverPopover(document.getElementById('sony-dot'), document.getElementById('pop-sony-power'));
  hoverPopover(document.getElementById('arcam-dot'), document.getElementById('pop-arcam-power'));

  function setupModeBadgeHover() {{
    const badge = document.getElementById('arcam-mode-badge');
    const pop = document.getElementById('pop-arcam-mode');
    if (badge && !badge._hoverBound) {{
      hoverPopover(badge, pop);
      badge._hoverBound = true;
    }}
  }}
  setupModeBadgeHover();

  async function poll() {{
    if (Date.now() < pollSuppressedUntil) return;
    try {{
      const resp = await fetch("/status");
      if (!resp.ok) return;
      const data = await resp.json();

      if (data.sony.detail) lastSonyDetail = data.sony.detail;
      setDot(document.getElementById("sony-dot"), data.sony.css_class);
      setText(document.getElementById("sony-label"), data.sony.label);
      renderSony(document.getElementById("sony-instruments"), document.getElementById("sony-sources"), data.sony.detail);

      setDot(document.getElementById("arcam-dot"), data.arcam.css_class);
      setText(document.getElementById("arcam-label"), data.arcam.label);
      renderBadges(document.getElementById("arcam-instruments"), arcamBadges(data.arcam.detail));
      setupModeBadgeHover();
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
