//! Homelab REST server library
//!
//! This module exposes the router building function and types needed for testing.

pub mod config;
pub mod error;
pub mod frontend;
pub mod handlers;
pub mod state;

use axum::response::IntoResponse;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, put},
};
use homelab::arcam::Arcam;
use homelab::eversolo::Eversolo;
use homelab::samsung_tv::SamsungTv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
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
        (name = "eversolo", description = "Eversolo DMP-A8 music streamer management and control"),
        (name = "samsung_tv", description = "Samsung Smart TV management and control"),
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
        .nest(
            "/eversolo",
            handlers::crud::eversolo_crud_routes()
                .merge(handlers::eversolo::routes_with_name()),
        )
        .nest(
            "/samsung_tv",
            handlers::crud::samsung_tv_crud_routes()
                .merge(handlers::samsung_tv::routes_with_name()),
        )
        .split_for_parts();

    router
        .route("/", get(frontend::index_handler))
        .route("/assets/{*file}", get(frontend::static_handler))
        .route("/status", get(status))
        .route("/arcam/timeout", get(arcam_timeout))
        .route("/arcam/auto-shutdown", get(arcam_auto_shutdown_get))
        .route("/arcam/auto-shutdown", put(arcam_auto_shutdown_set))
        .merge(Scalar::with_url("/explore", api))
        .fallback(get(frontend::fallback_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// --- Index Page ---

/// Device status for a single device (serializable for the `/status` endpoint).
#[derive(Serialize, Clone)]
pub struct DeviceStatusJson {
    /// Semantic status: `"active"`, `"standby"`, `"error"`, `"not_configured"`
    status: &'static str,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<serde_json::Value>,
}

/// Combined status for all devices.
#[derive(Serialize)]
struct StatusResponse {
    sony: DeviceStatusJson,
    arcam: DeviceStatusJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    eversolo: Option<DeviceStatusJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    samsung_tv: Option<DeviceStatusJson>,
}

/// Probe the Sony receiver and return its status.
///
/// When the receiver is active, fetches volume and current input
/// for dashboard instrumentation (best-effort — errors are silenced).
async fn probe_sony(state: &AppState) -> DeviceStatusJson {
    match &state.sony {
        None => DeviceStatusJson {
            status: "not_configured",
            label: "Not configured".to_string(),
            detail: None,
        },
        Some(sony) => match timeout(state.request_timeout, sony.get_power_status()).await {
            Ok(Ok(status)) => {
                let mut last = state.sony_last_power_status.write().await;
                if last.as_deref() != Some(status.as_str()) {
                    tracing::info!(status = %status, "Sony power status changed");
                    *last = Some(status.clone());
                } else {
                    tracing::trace!(status = %status, "Sony power status unchanged");
                }
                drop(last);
                match status.as_str() {
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
                        // Fetch real terminal URIs for source switching
                        let terminals = sony.list_inputs().await.ok();
                        let mut d = serde_json::Map::new();
                        if let Some(v) = volume {
                            d.insert("volume".into(), json!(v.volume));
                            d.insert("max_volume".into(), json!(v.max_volume));
                            d.insert("muted".into(), json!(v.mute == "on"));
                        }
                        let current_cat = input.as_ref().and_then(|i| {
                            let cat = match_uri_to_category(&i.uri, native.as_deref());
                            if cat.is_none() {
                                let icons: Vec<&str> = native.as_deref()
                                    .map(|n| n.iter().map(|x| x.icon.as_str()).collect())
                                    .unwrap_or_default();
                                tracing::warn!(
                                    uri = %i.uri,
                                    ?icons,
                                    "No source matched current input URI"
                                );
                            }
                            cat
                        });
                        if let Some(native_inputs) = &native {
                            // Build category -> terminal URI map
                            let cat_to_uri = build_category_uri_map(native_inputs, terminals.as_deref());
                            let source_list: Vec<serde_json::Value> = native_inputs
                                .iter()
                                .filter(|i| i.visible)
                                .map(|i| {
                                    let name = if i.name.is_empty() {
                                        display_name_for_category(&i.category)
                                    } else {
                                        i.name.clone()
                                    };
                                    let uri = cat_to_uri.get(i.category.as_str());
                                    json!({
                                        "category": i.category,
                                        "name": name,
                                        "active": current_cat.as_deref() == Some(i.category.as_str()),
                                        "uri": uri,
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
                            status: "active",
                            label: "Active".to_string(),
                            detail,
                        }
                    }
                    "standby" => DeviceStatusJson {
                        status: "standby",
                        label: "Standby".to_string(),
                        detail: None,
                    },
                    other => DeviceStatusJson {
                        status: "standby",
                        label: format!("Power: {other}"),
                        detail: None,
                    },
                }
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "Sony power status error");
                DeviceStatusJson {
                    status: "error",
                    label: format!("Error: {e}"),
                    detail: None,
                }
            }
            Err(_) => {
                tracing::warn!("Sony power status timeout");
                DeviceStatusJson {
                    status: "error",
                    label: "Timeout".to_string(),
                    detail: None,
                }
            }
        },
    }
}

/// Probe the first configured Eversolo device and return its status.
///
/// Returns `None` if no Eversolo devices are configured.
/// Calls `get_state()` to determine reachability and playback state.
async fn probe_eversolo(state: &AppState) -> Option<DeviceStatusJson> {
    let devices = state.eversolo_devices.read().await;
    let (name, service) = devices.iter().next()?;
    let eversolo = Eversolo::new(service.host.clone(), service.port);
    let _ = name; // used only for iteration

    Some(
        match timeout(state.request_timeout, eversolo.get_state()).await {
            Ok(Ok(resp)) => {
                let label = match resp.state {
                    1 => "Playing",
                    2 => "Paused",
                    _ => "Active",
                };
                let mut detail = serde_json::Map::new();
                if let Some(vol) = &resp.volume_data {
                    detail.insert("volume".into(), json!(vol.current_volume));
                    detail.insert("max_volume".into(), json!(vol.max_volume));
                    detail.insert("muted".into(), json!(vol.is_mute));
                }
                if let Some(music) = &resp.playing_music {
                    if let Some(title) = &music.title {
                        detail.insert("title".into(), json!(title));
                    }
                    if let Some(artist) = &music.artist {
                        detail.insert("artist".into(), json!(artist));
                    }
                }
                DeviceStatusJson {
                    status: "active",
                    label: label.to_string(),
                    detail: if detail.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(detail))
                    },
                }
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "Eversolo probe error");
                DeviceStatusJson {
                    status: "error",
                    label: "Unreachable".to_string(),
                    detail: None,
                }
            }
            Err(_) => {
                tracing::warn!("Eversolo probe timeout");
                DeviceStatusJson {
                    status: "error",
                    label: "Timeout".to_string(),
                    detail: None,
                }
            }
        },
    )
}

/// Probe the first configured Samsung TV and return its status.
///
/// Returns `None` if no Samsung TVs are configured.
/// Calls `get_device_info()` to determine reachability.
async fn probe_samsung_tv(state: &AppState) -> Option<DeviceStatusJson> {
    let tvs = state.samsung_tvs.read().await;
    let (_name, service) = tvs.iter().next()?;
    let tv = SamsungTv::new(service.host.clone(), service.rest_port, service.ws_port);

    Some(
        match timeout(state.request_timeout, tv.get_device_info()).await {
            Ok(Ok(info)) => {
                let mut detail = serde_json::Map::new();
                if let Some(name) = &info.name {
                    detail.insert("name".into(), json!(name));
                }
                if let Some(device) = &info.device {
                    if let Some(model) = &device.model_name {
                        detail.insert("model".into(), json!(model));
                    }
                }
                DeviceStatusJson {
                    status: "active",
                    label: "On".to_string(),
                    detail: if detail.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(detail))
                    },
                }
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "Samsung TV probe error");
                DeviceStatusJson {
                    status: "error",
                    label: "Unreachable".to_string(),
                    detail: None,
                }
            }
            Err(_) => {
                tracing::warn!("Samsung TV probe timeout");
                DeviceStatusJson {
                    status: "error",
                    label: "Timeout".to_string(),
                    detail: None,
                }
            }
        },
    )
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
                        let rest: String = chars.flat_map(|c| c.to_lowercase()).collect();
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

/// Returns a human-friendly display name for a Sony input category.
///
/// Used as a fallback when the native API doesn't provide a user-defined
/// input name (i.e. the `inputname` field is empty).
fn display_name_for_category(category: &str) -> String {
    match category {
        "GAME" => "Game",
        "STB" => "Media Box",
        "BD" => "Blu-ray",
        "SAT" => "Satellite",
        "VIDEO" => "Video",
        "AUX" => "Aux",
        "TV" => "TV",
        "CD" => "CD",
        other => return format_source_name(other),
    }
    .to_string()
}

/// Builds a map from native input category to the URI for `setPlayContent`.
///
/// Uses a two-phase strategy:
///
/// 1. **Terminal matching**: For each URI from `getCurrentExternalTerminalsStatus`,
///    resolves which category it belongs to via `match_uri_to_category`.
///    Produces real `extInput:hdmi?port=N` URIs when native fields are populated.
///
/// 2. **Known URI fallback**: For categories not resolved in phase 1, falls back
///    to the standard Sony logical URIs (e.g. `extInput:game`, `extInput:mediaBox`).
///    These are the same URIs `getPlayingContentInfo` reports and are accepted by
///    `setPlayContent` on Sony ES receivers.
fn build_category_uri_map(
    native: &[homelab::sony_receiver::NativeInputConfig],
    terminals: Option<&[homelab::sony_receiver::InputSource]>,
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    // Phase 1: resolve from real terminal URIs
    if let Some(terminals) = terminals {
        for t in terminals {
            if let Some(cat) = match_uri_to_category(&t.uri, Some(native)) {
                map.entry(cat).or_insert_with(|| t.uri.clone());
            }
        }
    }

    // Phase 2: fill gaps with known logical URIs
    for n in native {
        if !map.contains_key(&n.category)
            && let Some(uri) = category_to_logical_uri(&n.category)
        {
            map.insert(n.category.clone(), uri.to_string());
        }
    }

    map
}

/// Maps a Sony input category to its standard logical URI.
///
/// These are the URIs reported by `getPlayingContentInfo` and accepted
/// by `setPlayContent` on Sony ES receivers.
fn category_to_logical_uri(category: &str) -> Option<&'static str> {
    match category {
        "GAME" => Some("extInput:game"),
        "STB" => Some("extInput:mediaBox"),
        "BD" => Some("extInput:bd"),
        "SAT" => Some("extInput:sat_catv"),
        "VIDEO" => Some("extInput:video"),
        "AUX" => Some("extInput:line"),
        "TV" => Some("extInput:tv"),
        "CD" => Some("extInput:sacd_cd"),
        _ => None,
    }
}

/// Matches a Sony input URI to a native input category.
///
/// Uses a three-phase matching strategy against `getPlayingContentInfo` URIs:
///
/// 1. **Icon-based**: `extInput:game` matches native input with `icon == "game"`
/// 2. **HDMI port-based**: `extInput:hdmi?port=2` matches the native input assigned
///    to HDMI port 2 (handles `hdmi_assign` formats like `"IN 2"`, `"in2"`, etc.)
/// 3. **Known URI mapping**: Falls back to a hardcoded mapping of Sony URI bases
///    to input categories (e.g. `mediaBox` -> `STB`). This handles receivers
///    whose native API returns empty icon fields.
pub(crate) fn match_uri_to_category(
    uri: &str,
    native: Option<&[homelab::sony_receiver::NativeInputConfig]>,
) -> Option<String> {
    let rest = uri.strip_prefix("extInput:")?;
    let (base, query) = rest.split_once('?').unwrap_or((rest, ""));
    let base_lower = base.to_lowercase();
    let native = native?;

    // Phase 1: icon-based matching (e.g. extInput:game -> icon "game")
    if let Some(matched) = native.iter().find(|n| n.icon.to_lowercase() == base_lower) {
        return Some(matched.category.clone());
    }

    // Phase 2: HDMI port matching (e.g. extInput:hdmi?port=2)
    if base_lower == "hdmi"
        && let Some(port) = query
            .split('&')
            .find_map(|p| p.strip_prefix("port="))
            .and_then(|p| p.parse::<u32>().ok())
        && let Some(matched) = native
            .iter()
            .find(|n| extract_hdmi_port(&n.hdmi_assign) == Some(port))
    {
        return Some(matched.category.clone());
    }

    // Phase 3: known URI-to-category mapping (fallback for empty icons)
    let mapped_category = match base {
        "game" => "GAME",
        "mediaBox" => "STB",
        "bd" => "BD",
        "sat" | "sat_catv" => "SAT",
        "video" => "VIDEO",
        "aux" => "AUX",
        "tv" => "TV",
        "cd" | "sacd_cd" => "CD",
        _ => return None,
    };
    // Only return if the category exists in native inputs
    if native.iter().any(|n| n.category == mapped_category) {
        return Some(mapped_category.to_string());
    }

    None
}

/// Extracts the HDMI port number from a native API `hdmi_assign` value.
///
/// Handles formats like `"IN 1"`, `"in1"`, `"IN 7"`, `"HDMI 2"`.
/// Returns `None` for non-HDMI assignments like `"none"`, `"eARC/OUT A"`, `""`.
pub(crate) fn extract_hdmi_port(hdmi_assign: &str) -> Option<u32> {
    let trimmed = hdmi_assign.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return None;
    }
    // Extract trailing digits (handles "IN 1", "in1", "HDMI 2", "IN 7")
    let digits: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.chars().rev().collect::<String>().parse().ok()
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
    let port = query.split('&').find_map(|kv| kv.strip_prefix("port="));

    match port {
        Some(p) => format!("{name} {p}"),
        None => name,
    }
}

#[cfg(test)]
mod format_tests {
    use super::{
        build_category_uri_map, category_to_logical_uri, display_name_for_category,
        extract_hdmi_port, format_input_name, format_source_name, match_uri_to_category,
    };

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

    fn make_source(
        category: &str,
        name: &str,
        hdmi_assign: &str,
        icon: &str,
    ) -> homelab::sony_receiver::NativeInputConfig {
        homelab::sony_receiver::NativeInputConfig {
            category: category.to_string(),
            name: name.to_string(),
            hdmi_assign: hdmi_assign.to_string(),
            icon: icon.to_string(),
            visible: false,
            sound_field: String::new(),
            digital_assign: String::new(),
            input_mode: String::new(),
            subwoofer_level: String::new(),
            subwoofer_lpf: String::new(),
            in_ceiling_mode: false,
            trigger_1: false,
            trigger_2: false,
            trigger_3: false,
            preset_gain: String::new(),
            av_sync: String::new(),
        }
    }

    #[test]
    fn uri_to_category_matches_icon() {
        let inputs = vec![
            make_source("STB", "MEDIA BOX", "in2", "MEDIA BOX"),
            make_source("GAME", "GAME", "in1", "GAME"),
            make_source("TV", "TV", "eARC/OUT A", "TV"),
        ];
        // Icon-based matching (case-insensitive)
        assert_eq!(
            match_uri_to_category("extInput:MEDIA BOX", Some(&inputs)),
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
    fn uri_to_category_matches_hdmi_port() {
        // Realistic hdmi_assign values from Sony native API: "in1", "in2", etc.
        let inputs = vec![
            make_source("GAME", "GAME", "in1", "GAME"),
            make_source("STB", "MEDIA BOX", "in2", "MEDIA BOX"),
            make_source("BD", "BD/DVD", "in3", "BD Player"),
            make_source("SAT", "SAT/CATV", "in4", "Tuner"),
            make_source("VIDEO", "VIDEO", "in7", "VIDEO"),
            make_source("AUX", "AUX", "in5", "Tuner"),
            make_source("TV", "TV", "eARC/OUT A", "TV"),
            make_source("CD", "SA-CD/CD", "in6", "CD Player"),
        ];
        // HDMI port matching: extInput:hdmi?port=N -> hdmi_assign with trailing N
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=1", Some(&inputs)),
            Some("GAME".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=2", Some(&inputs)),
            Some("STB".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=3", Some(&inputs)),
            Some("BD".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=7", Some(&inputs)),
            Some("VIDEO".to_string())
        );
        // No HDMI port 9 assigned
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=9", Some(&inputs)),
            None
        );
        // eARC/OUT A has no trailing number — won't match any port
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=0", Some(&inputs)),
            None
        );
    }

    #[test]
    fn uri_to_category_hdmi_alternate_formats() {
        // Handle various hdmi_assign formats from different firmware versions
        let inputs = vec![
            make_source("GAME", "GAME", "IN 1", "GAME"),
            make_source("STB", "MEDIA BOX", "HDMI 2", "MEDIA BOX"),
        ];
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=1", Some(&inputs)),
            Some("GAME".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=2", Some(&inputs)),
            Some("STB".to_string())
        );
    }

    #[test]
    fn uri_to_category_known_uri_fallback() {
        // When icons are empty (some receiver models), fall back to known URI mapping
        let inputs = vec![
            make_source("GAME", "", "in1", ""),
            make_source("STB", "", "in2", ""),
            make_source("BD", "", "in3", ""),
            make_source("SAT", "", "in4", ""),
            make_source("VIDEO", "", "in7", ""),
            make_source("AUX", "", "in5", ""),
            make_source("TV", "", "eARC/OUT A", ""),
            make_source("CD", "", "in6", ""),
        ];
        // Direct URI-to-category mapping
        assert_eq!(
            match_uri_to_category("extInput:mediaBox", Some(&inputs)),
            Some("STB".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:game", Some(&inputs)),
            Some("GAME".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:bd", Some(&inputs)),
            Some("BD".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:sat", Some(&inputs)),
            Some("SAT".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:sat_catv", Some(&inputs)),
            Some("SAT".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:video", Some(&inputs)),
            Some("VIDEO".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:aux", Some(&inputs)),
            Some("AUX".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:tv", Some(&inputs)),
            Some("TV".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:cd", Some(&inputs)),
            Some("CD".to_string())
        );
        assert_eq!(
            match_uri_to_category("extInput:sacd_cd", Some(&inputs)),
            Some("CD".to_string())
        );
        // Unknown URI still returns None
        assert_eq!(
            match_uri_to_category("extInput:unknown", Some(&inputs)),
            None
        );
        // HDMI port matching still works with empty icons
        assert_eq!(
            match_uri_to_category("extInput:hdmi?port=1", Some(&inputs)),
            Some("GAME".to_string())
        );
    }

    #[test]
    fn extract_hdmi_port_formats() {
        // Native API formats
        assert_eq!(extract_hdmi_port("in1"), Some(1));
        assert_eq!(extract_hdmi_port("in7"), Some(7));
        // UI display formats
        assert_eq!(extract_hdmi_port("IN 1"), Some(1));
        assert_eq!(extract_hdmi_port("IN 5"), Some(5));
        assert_eq!(extract_hdmi_port("HDMI 2"), Some(2));
        // Non-HDMI
        assert_eq!(extract_hdmi_port("none"), None);
        assert_eq!(extract_hdmi_port(""), None);
        assert_eq!(extract_hdmi_port("eARC/OUT A"), None);
        assert_eq!(extract_hdmi_port("OPT 1"), Some(1)); // Won't cause issues — only checked for HDMI URIs
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

    #[test]
    fn display_name_for_category_all() {
        assert_eq!(display_name_for_category("GAME"), "Game");
        assert_eq!(display_name_for_category("STB"), "Media Box");
        assert_eq!(display_name_for_category("BD"), "Blu-ray");
        assert_eq!(display_name_for_category("SAT"), "Satellite");
        assert_eq!(display_name_for_category("VIDEO"), "Video");
        assert_eq!(display_name_for_category("AUX"), "Aux");
        assert_eq!(display_name_for_category("TV"), "TV");
        assert_eq!(display_name_for_category("CD"), "CD");
        // Unknown category falls back to format_source_name
        assert_eq!(display_name_for_category("PHONO"), "Phono");
    }

    #[test]
    fn build_category_uri_map_with_empty_native_fields() {
        use homelab::sony_receiver::InputSource;

        // Reproduce real-world scenario: native API returns empty icons AND
        // empty hdmi_assign values. Terminal URIs can't be matched to categories
        // via icon or HDMI port, so the fallback logical URIs are used.
        let native = vec![
            make_source("GAME", "", "", ""),
            make_source("STB", "", "", ""),
            make_source("BD", "", "", ""),
            make_source("SAT", "", "", ""),
            make_source("VIDEO", "", "", ""),
            make_source("AUX", "", "", ""),
            make_source("TV", "", "", ""),
            make_source("CD", "", "", ""),
        ];

        let terminals = vec![
            InputSource {
                title: "HDMI 1".into(),
                uri: "extInput:hdmi?port=1".into(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
            InputSource {
                title: "HDMI 2".into(),
                uri: "extInput:hdmi?port=2".into(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
        ];

        let map = build_category_uri_map(&native, Some(&terminals));

        // Phase 1 can't match HDMI terminals to categories (empty native fields),
        // but Phase 2 fills ALL categories with logical URIs.
        assert_eq!(map.len(), 8, "all 8 categories should have URIs: {:?}", map);
        assert_eq!(map.get("GAME"), Some(&"extInput:game".to_string()));
        assert_eq!(map.get("STB"), Some(&"extInput:mediaBox".to_string()));
        assert_eq!(map.get("BD"), Some(&"extInput:bd".to_string()));
        assert_eq!(map.get("SAT"), Some(&"extInput:sat_catv".to_string()));
        assert_eq!(map.get("VIDEO"), Some(&"extInput:video".to_string()));
        assert_eq!(map.get("AUX"), Some(&"extInput:line".to_string()));
        assert_eq!(map.get("TV"), Some(&"extInput:tv".to_string()));
        assert_eq!(map.get("CD"), Some(&"extInput:sacd_cd".to_string()));
    }

    #[test]
    fn build_category_uri_map_with_populated_hdmi_assign() {
        use homelab::sony_receiver::InputSource;

        // Normal scenario: native API returns hdmi_assign values
        let native = vec![
            make_source("GAME", "", "in1", ""),
            make_source("STB", "", "in2", ""),
            make_source("BD", "", "in3", ""),
            make_source("TV", "", "eARC/OUT A", ""),
        ];

        let terminals = vec![
            InputSource {
                title: "HDMI 1".into(),
                uri: "extInput:hdmi?port=1".into(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
            InputSource {
                title: "HDMI 2".into(),
                uri: "extInput:hdmi?port=2".into(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
            InputSource {
                title: "HDMI 3".into(),
                uri: "extInput:hdmi?port=3".into(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
        ];

        let map = build_category_uri_map(&native, Some(&terminals));

        assert_eq!(map.get("GAME"), Some(&"extInput:hdmi?port=1".to_string()));
        assert_eq!(map.get("STB"), Some(&"extInput:hdmi?port=2".to_string()));
        assert_eq!(map.get("BD"), Some(&"extInput:hdmi?port=3".to_string()));
        // TV on eARC has no matching terminal, gets logical URI fallback
        assert_eq!(map.get("TV"), Some(&"extInput:tv".to_string()));
    }

    #[test]
    fn build_category_uri_map_with_icon_matching() {
        use homelab::sony_receiver::InputSource;

        // Scenario: native API returns icons (some receiver models)
        let native = vec![
            make_source("GAME", "", "in1", "game"),
            make_source("STB", "", "in2", "mediaBox"),
        ];

        // Terminals with non-HDMI URIs that match by icon
        let terminals = vec![
            InputSource {
                title: "Game".into(),
                uri: "extInput:game".into(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
            InputSource {
                title: "Media Box".into(),
                uri: "extInput:mediaBox".into(),
                icon_url: None,
                connection: None,
                label: None,
                active: None,
            },
        ];

        let map = build_category_uri_map(&native, Some(&terminals));

        assert_eq!(map.get("GAME"), Some(&"extInput:game".to_string()));
        assert_eq!(map.get("STB"), Some(&"extInput:mediaBox".to_string()));
    }

    #[test]
    fn build_category_uri_map_no_terminals() {
        // Even without terminal data, Phase 2 fallback provides logical URIs
        let native = vec![make_source("GAME", "", "in1", "")];
        let map = build_category_uri_map(&native, None);
        assert_eq!(map.get("GAME"), Some(&"extInput:game".to_string()));
    }

    #[test]
    fn build_category_uri_map_phase1_takes_priority() {
        use homelab::sony_receiver::InputSource;

        // When Phase 1 resolves a real HDMI URI, Phase 2 fallback is NOT used
        let native = vec![
            make_source("GAME", "", "in1", ""),
            make_source("STB", "", "", ""), // empty hdmi_assign
        ];
        let terminals = vec![InputSource {
            title: "HDMI 1".into(),
            uri: "extInput:hdmi?port=1".into(),
            icon_url: None,
            connection: None,
            label: None,
            active: None,
        }];

        let map = build_category_uri_map(&native, Some(&terminals));

        // GAME resolved from terminal via HDMI port matching
        assert_eq!(map.get("GAME"), Some(&"extInput:hdmi?port=1".to_string()));
        // STB not matched by terminals, gets logical URI fallback
        assert_eq!(map.get("STB"), Some(&"extInput:mediaBox".to_string()));
    }

    #[test]
    fn category_to_logical_uri_all() {
        assert_eq!(category_to_logical_uri("GAME"), Some("extInput:game"));
        assert_eq!(category_to_logical_uri("STB"), Some("extInput:mediaBox"));
        assert_eq!(category_to_logical_uri("BD"), Some("extInput:bd"));
        assert_eq!(category_to_logical_uri("SAT"), Some("extInput:sat_catv"));
        assert_eq!(category_to_logical_uri("VIDEO"), Some("extInput:video"));
        assert_eq!(category_to_logical_uri("AUX"), Some("extInput:line"));
        assert_eq!(category_to_logical_uri("TV"), Some("extInput:tv"));
        assert_eq!(category_to_logical_uri("CD"), Some("extInput:sacd_cd"));
        assert_eq!(category_to_logical_uri("UNKNOWN"), None);
    }
}

/// Probe the Arcam amplifier and return its status.
///
/// When the amplifier is on, fetches mute status, amplifier mode (PA240 only),
/// auto-shutdown setting, and timeout counter for dashboard instrumentation.
/// When off, includes cached model name and heartbeat status.
async fn probe_arcam(state: &AppState) -> DeviceStatusJson {
    match &state.arcam_host {
        None => DeviceStatusJson {
            status: "not_configured",
            label: "Not configured".to_string(),
            detail: None,
        },
        Some(host) => {
            let arcam = Arcam::from(host.as_str());
            match timeout(state.request_timeout, arcam.request_power_state()).await {
                Ok(Ok(true)) => {
                    let mut last = state.arcam_last_power_state.write().await;
                    if *last != Some(true) {
                        tracing::info!("Arcam power: on");
                        *last = Some(true);
                    } else {
                        tracing::trace!("Arcam power: on");
                    }
                    drop(last);
                    let model = state.arcam_model.read().await.clone();
                    // Best-effort instrumentation: mute + mode + auto-shutdown + timeout
                    // Each call opens a new TCP connection (Arcam is single-connection)
                    let detail = timeout(state.request_timeout, async {
                        let mut d = serde_json::Map::new();
                        if let Some(m) = &model {
                            d.insert("model".into(), json!(m));
                        }

                        let arcam = Arcam::from(host.as_str());
                        if let Ok(m) = arcam.get_mute_status().await {
                            d.insert("muted".into(), json!(m));
                        }

                        // Only query amp mode for PA240.
                        // Firmware returns 1-indexed values (not 0-indexed as
                        // documented in SH305E Issue 3): ST=1, BR=2, DM=3.
                        if model.as_deref() == Some("PA240") {
                            let arcam = Arcam::from(host.as_str());
                            if let Ok(mb) = arcam.get_amplifier_mode().await {
                                let mode = match mb {
                                    1 => "Stereo",
                                    2 => "Bridged",
                                    3 => "Dual Mono",
                                    _ => "Unknown",
                                };
                                d.insert("mode".into(), json!(mode));
                                d.insert("mode_raw".into(), json!(mb));
                            }
                        }

                        let arcam = Arcam::from(host.as_str());
                        if let Ok(asd) = arcam.get_auto_shutdown().await {
                            d.insert("auto_shutdown".into(), json!(asd));
                            d.insert(
                                "auto_shutdown_label".into(),
                                json!(homelab::arcam::auto_shutdown_label(asd)),
                            );

                            // NOTE: timeout counter is NOT queried here because each
                            // preceding TCP command resets the amp's EuP idle timer,
                            // making the reading meaningless. It has a dedicated
                            // endpoint (/arcam/timeout) queried independently.
                        }

                        serde_json::Value::Object(d)
                    })
                    .await
                    .ok();

                    DeviceStatusJson {
                        status: "active",
                        label: "Active".to_string(),
                        detail,
                    }
                }
                Ok(Ok(false)) => {
                    let mut last = state.arcam_last_power_state.write().await;
                    if *last != Some(false) {
                        tracing::info!("Arcam power: standby");
                        *last = Some(false);
                    } else {
                        tracing::trace!("Arcam power: standby");
                    }
                    drop(last);
                    let model = state.arcam_model.read().await.clone();
                    let heartbeat = state
                        .arcam_heartbeat_alive
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let detail = if model.is_some() || heartbeat {
                        let mut d = serde_json::Map::new();
                        if let Some(m) = &model {
                            d.insert("model".into(), json!(m));
                        }
                        d.insert("heartbeat".into(), json!(heartbeat));
                        Some(serde_json::Value::Object(d))
                    } else {
                        None
                    };
                    DeviceStatusJson {
                        status: "standby",
                        label: "Standby".to_string(),
                        detail,
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "Arcam power error");
                    DeviceStatusJson {
                        status: "error",
                        label: format!("Error: {e}"),
                        detail: None,
                    }
                }
                Err(_) => {
                    tracing::warn!("Arcam power timeout");
                    DeviceStatusJson {
                        status: "error",
                        label: "Timeout".to_string(),
                        detail: None,
                    }
                }
            }
        }
    }
}

/// JSON status endpoint polled by the index page.
async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let (sony, arcam, eversolo, samsung_tv) = tokio::join!(
        probe_sony(&state),
        get_cached_or_fresh_arcam(&state),
        probe_eversolo(&state),
        probe_samsung_tv(&state),
    );
    Json(StatusResponse {
        sony,
        arcam,
        eversolo,
        samsung_tv,
    })
}

/// Get Arcam status with rate limiting based on Auto Shutdown setting.
///
/// Only queries the amp if enough time has passed since the last query.
/// Otherwise returns cached status.
async fn get_cached_or_fresh_arcam(state: &AppState) -> DeviceStatusJson {
    let interval_secs = *state.arcam_poll_interval_secs.read().await;
    let now = Instant::now();

    // Check if we should query fresh data
    let should_query = {
        let last_query = state.arcam_last_query.read().await;
        match *last_query {
            Some(last) => now.duration_since(last).as_secs() >= interval_secs,
            None => true,
        }
    };

    if should_query {
        // Query fresh data
        let fresh = probe_arcam(state).await;

        // Cache it
        *state.arcam_cached_status.write().await = Some(fresh.clone());
        *state.arcam_last_query.write().await = Some(now);

        // Update polling interval based on Auto Shutdown setting
        // This is a best-effort update - we don't want to block on this
        if let Some(ref detail) = fresh.detail
            && let Some(auto_shutdown) = detail.get("auto_shutdown").and_then(|v| v.as_u64())
        {
            state.update_arcam_poll_interval(auto_shutdown as u8).await;
        }

        fresh
    } else {
        // Return cached data
        state
            .arcam_cached_status
            .read()
            .await
            .clone()
            .unwrap_or_else(|| {
                // If no cache yet, return "unknown" state
                DeviceStatusJson {
                    status: "not_configured",
                    label: "Unknown".to_string(),
                    detail: None,
                }
            })
    }
}

/// Dedicated timeout counter query — sends ONLY the timeout counter command
/// on a fresh TCP connection so the reading isn't contaminated by other commands.
/// Returns raw response bytes for debugging.
async fn arcam_timeout(State(state): State<AppState>) -> impl IntoResponse {
    let host = state.arcam_host.as_deref().unwrap_or("");
    if host.is_empty() {
        return Json(json!({ "error": "not configured" }));
    }
    let arcam = Arcam::from(host);
    // Send raw command and return full response details
    let cmd = [0x21, 0x01, 0x55, 0x01, 0xF0, 0x0D];
    match arcam.send_command(&cmd).await {
        Ok(resp) => {
            let hex_data: Vec<String> = resp.data.iter().map(|b| format!("{b:02X}")).collect();
            let parsed_be = if resp.data.len() >= 2 {
                u16::from_be_bytes([resp.data[0], resp.data[1]])
            } else if resp.data.len() == 1 {
                resp.data[0] as u16
            } else {
                0
            };
            let parsed_le = if resp.data.len() >= 2 {
                u16::from_le_bytes([resp.data[0], resp.data[1]])
            } else if resp.data.len() == 1 {
                resp.data[0] as u16
            } else {
                0
            };
            Json(json!({
                "answer_code": resp.answer_code,
                "data_len": resp.data.len(),
                "data_hex": hex_data,
                "data_raw": resp.data,
                "parsed_be": parsed_be,
                "parsed_le": parsed_le,
                "full_frame_hex": resp.raw.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>()
            }))
        }
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Get auto-shutdown setting (no name required - uses legacy config)
async fn arcam_auto_shutdown_get(State(state): State<AppState>) -> impl IntoResponse {
    let host = match &state.arcam_host {
        Some(h) => h.clone(),
        None => return Json(json!({ "error": "not configured" })),
    };
    let arcam = Arcam::from(host.as_str());
    match arcam.get_auto_shutdown().await {
        Ok(value) => {
            let label = homelab::arcam::auto_shutdown_label(value).to_string();
            Json(json!({ "value": value, "label": label }))
        }
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Set auto-shutdown setting (no name required - uses legacy config)
async fn arcam_auto_shutdown_set(
    State(state): State<AppState>,
    Json(req): Json<SetAutoShutdownRequest>,
) -> impl IntoResponse {
    if req.value > 4 {
        return Json(json!({ "error": "value must be 0-4" }));
    }
    let host = match &state.arcam_host {
        Some(h) => h.clone(),
        None => return Json(json!({ "error": "not configured" })),
    };
    let arcam = Arcam::from(host.as_str());
    match arcam.set_auto_shutdown(req.value).await {
        Ok(value) => {
            // Update polling interval and invalidate cache so the next poll
            // fetches fresh data instead of returning stale cached status
            state.update_arcam_poll_interval(value).await;
            state.invalidate_arcam_cache().await;
            let label = homelab::arcam::auto_shutdown_label(value).to_string();
            Json(json!({ "value": value, "label": label }))
        }
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

#[derive(Deserialize)]
struct SetAutoShutdownRequest {
    value: u8,
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
