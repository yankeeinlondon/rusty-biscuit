//! Homelab REST server library
//!
//! This module exposes the router building function and types needed for testing.

pub mod config;
pub mod error;
pub mod handlers;
pub mod state;

use axum::response::IntoResponse;
use axum::{
    Json, Router,
    extract::State,
    response::Html,
    routing::{get, put},
};
use homelab::arcam::Arcam;
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
        .split_for_parts();

    router
        .route("/", get(index))
        .route("/status", get(status))
        .route("/arcam/timeout", get(arcam_timeout))
        .route("/arcam/auto-shutdown", get(arcam_auto_shutdown_get))
        .route("/arcam/auto-shutdown", put(arcam_auto_shutdown_set))
        .merge(Scalar::with_url("/explore", api))
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

/// Maps a semantic status to a CSS class for the initial HTML render.
fn status_to_css(status: &str) -> &'static str {
    match status {
        "active" => "green",
        "standby" => "amber",
        "error" => "red",
        _ => "grey",
    }
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
        if !map.contains_key(&n.category) {
            if let Some(uri) = category_to_logical_uri(&n.category) {
                map.insert(n.category.clone(), uri.to_string());
            }
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
    if base_lower == "hdmi" {
        if let Some(port) = query
            .split('&')
            .find_map(|p| p.strip_prefix("port="))
            .and_then(|p| p.parse::<u32>().ok())
        {
            if let Some(matched) = native
                .iter()
                .find(|n| extract_hdmi_port(&n.hdmi_assign) == Some(port))
            {
                return Some(matched.category.clone());
            }
        }
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
    let arcam = get_cached_or_fresh_arcam(&state).await;
    let sony = probe_sony(&state).await;
    Json(StatusResponse { sony, arcam })
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
        if let Some(ref detail) = fresh.detail {
            if let Some(auto_shutdown) = detail.get("auto_shutdown").and_then(|v| v.as_u64()) {
                state.update_arcam_poll_interval(auto_shutdown as u8).await;
            }
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

    // Get first device names from config for API routes
    let sony_device_name = {
        let receivers = state.sony_receivers.read().await;
        receivers.keys().next().cloned().unwrap_or_default()
    };
    let arcam_device_name = {
        let hosts = state.arcam_hosts.read().await;
        hosts.keys().next().cloned().unwrap_or_default()
    };

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
  .dot {{ width: 28px; height: 28px; border-radius: 50%; flex-shrink: 0; transition: background 0.3s, box-shadow 0.3s, transform 0.2s ease, filter 0.2s ease, border-color 0.2s ease; cursor: pointer; border: 2px solid transparent; }}
  .dot:not(.grey):hover {{ border-color: rgba(255,255,255,0.12); transform: scale(1.08); filter: brightness(1.15); }}
  .dot:not(.grey):active {{ transform: scale(0.92); }}
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
  .instruments {{ display: flex; flex-direction: column; gap: 2px; align-items: flex-end; flex-shrink: 0; justify-content: center; }}
  .badge-row {{ display: flex; gap: 6px; flex-wrap: wrap; }}
  .badge {{ background: #0f3460; border: 1px solid #1a4a7a; border-radius: 4px; padding: 4px 10px; font-size: 0.75em; color: #a8c8e8; white-space: nowrap; letter-spacing: 0.02em; transition: filter 0.2s ease, transform 0.2s ease; }}
  .badge[id]:hover {{ filter: brightness(1.2); transform: scale(1.04); }}
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
  .sources .badge[data-uri] {{ cursor: pointer; }}
  .sources .badge[data-uri]:hover:not(.badge-active) {{ background: rgba(15, 52, 96, 0.4); border-color: #1a5a8a; color: #7ab8e0; }}
  .sources .badge.badge-active {{ cursor: default; }}
  .badge-dim {{ background: rgba(15, 52, 96, 0.15); border-color: #1a3050; color: #4a6278; }}
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
  .heartbeat {{ display: inline-block; animation: pulse 2s ease-in-out infinite; line-height: 1; font-size: 0.9em; }}
  @keyframes pulse {{ 0%, 100% {{ opacity: 0.3; }} 50% {{ opacity: 1; }} }}
  .badge-auto-shutdown {{ background: #1a3a2e; border-color: #2a5a4e; color: #80c8a0; cursor: help; }}
  .badge-auto-shutdown-off {{ background: #2a2a2a; border-color: #3a3a3a; color: #888; cursor: help; }}
  .device-model {{ color: #556; font-size: 0.8em; font-weight: 500; letter-spacing: 0.03em; line-height: 1; }}
  #arcam-mode-badge {{ anchor-name: --arcam-mode; cursor: help; }}
  #pop-arcam-mode {{ position-anchor: --arcam-mode; }}
  #arcam-auto-shutdown-badge {{ anchor-name: --arcam-auto-shutdown; cursor: pointer; }}
  #pop-arcam-auto-shutdown {{ position-anchor: --arcam-auto-shutdown; margin-top: 10px; }}
  #auto-shutdown-select {{ margin: 0.5em 0 0; padding: 8px 12px; font-size: 0.9em; background: #0f1a2e; color: #c8d8e8; border: 1px solid #1a4a7a; border-radius: 6px; cursor: pointer; width: 100%; appearance: none; -webkit-appearance: none; background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8' viewBox='0 0 12 8'%3E%3Cpath fill='%237cc4ff' d='M1 1l5 5 5-5'/%3E%3C/svg%3E"); background-repeat: no-repeat; background-position: right 10px center; }}
  #auto-shutdown-select:hover {{ border-color: #2a6aaa; }}
  #auto-shutdown-select:focus {{ outline: none; border-color: #7cc4ff; box-shadow: 0 0 0 2px rgba(124,196,255,0.15); }}
  #auto-shutdown-select option {{ background: #0f1a2e; color: #c8d8e8; padding: 6px; }}
  .frost-overlay {{ position: fixed; inset: 0; z-index: 9000; backdrop-filter: blur(6px) saturate(0.3); -webkit-backdrop-filter: blur(6px) saturate(0.3); background: rgba(10, 10, 20, 0.55); display: flex; align-items: center; justify-content: center; opacity: 0; pointer-events: none; transition: opacity 0.6s ease; }}
  .frost-overlay.visible {{ opacity: 1; pointer-events: auto; }}
  .frost-modal {{ background: #16213e; border: 1px solid #1a4a7a; border-radius: 12px; padding: 28px 36px; text-align: center; box-shadow: 0 12px 48px rgba(0,0,0,0.6); max-width: 340px; }}
  .frost-modal h2 {{ color: #e8e8e8; font-size: 1.1em; margin: 0 0 10px; font-weight: 600; }}
  .frost-modal p {{ color: #778; font-size: 0.85em; margin: 0; line-height: 1.5; }}
  .frost-modal .spinner {{ display: inline-block; width: 18px; height: 18px; border: 2px solid #334; border-top-color: #7cc4ff; border-radius: 50%; animation: spin 0.8s linear infinite; margin-top: 14px; }}
  @keyframes spin {{ to {{ transform: rotate(360deg); }} }}
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

<div class="frost-overlay" id="frost-overlay">
  <div class="frost-modal">
    <h2>Homelab Server is unreachable</h2>
    <p>Waiting for connection&hellip;</p>
    <div class="spinner"></div>
  </div>
</div>

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

<div id="pop-arcam-auto-shutdown" popover="auto" class="info-popover">
  <h4>Auto Shutdown</h4>
  <p>When enabled, the amplifier monitors its audio input. If no signal is detected for the configured period, it enters low-power standby mode.</p>
  <select id="auto-shutdown-select">
    <option value="0">Off</option>
    <option value="1">20 min</option>
    <option value="2">30 min</option>
    <option value="3">1 hour</option>
    <option value="4">2 hours</option>
  </select>
</div>

<script>
(function() {{
  const SONY_DEVICE = '{}';
  const ARCAM_DEVICE = '{}';
  const CLASSES = ["green", "amber", "red", "grey"];
  const STATUS_CLASS = {{ active: 'green', standby: 'amber', error: 'red', not_configured: 'grey' }};
  function toClass(status) {{ return STATUS_CLASS[status] || 'grey'; }}

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

  let lastSonyDetail = null;
  let lastArcamOnDetail = null;
  let lastArcamModel = null;
  // Locks: hold dot+label at optimistic state until real data confirms transition
  let sonyLockClass = null;
  let sonyLockExpiry = 0;
  let arcamLockClass = null;
  let arcamLockExpiry = 0;
  // Lock: hold source badges at optimistic state until poll confirms
  let sonySourceLock = null; // category string
  let sonySourceLockExpiry = 0;
  // Lock: hold auto-shutdown badge at optimistic value until server confirms
  let arcamAutoShutdownLock = null; // {{ value, label, serverLabel }}
  let arcamAutoShutdownLockExpiry = 0;
  // Server reachability tracking
  let consecutiveFailures = 0;
  let serverUnreachable = false;
  const FAILURE_THRESHOLD = 2; // Show overlay after 2 consecutive failures

  function togglePower(dotEl, device) {{
    if (dotEl.classList.contains('grey')) return;
    const isOn = dotEl.classList.contains('green');
    // Animate press
    dotEl.classList.remove('pressing');
    void dotEl.offsetWidth;
    dotEl.classList.add('pressing');
    dotEl.addEventListener('animationend', () => dotEl.classList.remove('pressing'), {{ once: true }});
    // Optimistic UI — immediate
    const labelEl = document.getElementById(device === 'sony' ? 'sony-label' : 'arcam-label');
    const expectedClass = isOn ? 'amber' : 'green';
    setDot(dotEl, expectedClass);
    setText(labelEl, isOn ? 'Standby' : 'Active');
    if (device === 'sony') {{
      const srcEl = document.getElementById('sony-sources');
      const insEl = document.getElementById('sony-instruments');
      if (isOn) {{
        renderSony(insEl, srcEl, null);
      }} else if (lastSonyDetail) {{
        renderSony(insEl, srcEl, lastSonyDetail);
      }}
      sonyLockClass = expectedClass;
      sonyLockExpiry = Date.now() + 15000;
      fetch('/sony_receiver/' + encodeURIComponent(SONY_DEVICE) + '/power', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{ active: !isOn }})
      }}).catch(() => {{}});
    }} else if (device === 'arcam') {{
      const insEl = document.getElementById('arcam-instruments');
      if (isOn) {{
        if (lastArcamModel) {{
          insEl.innerHTML = '<span class="device-model">' + lastArcamModel + '</span>'
            + '<span class="heartbeat" title="Heartbeat keepalive active">\u2764\uFE0F</span>';
        }} else {{
          insEl.innerHTML = '';
        }}
      }} else {{
        if (lastArcamOnDetail) {{
          const badges = arcamBadges(lastArcamOnDetail);
          renderBadges(insEl, badges);
          setupModeBadgeHover();
          setupAutoShutdownBadgeHover();
        }}
      }}
      arcamLockClass = expectedClass;
      arcamLockExpiry = Date.now() + 15000;
      const action = isOn ? 'off' : 'on';
      fetch('/arcam_amp/' + encodeURIComponent(ARCAM_DEVICE) + '/power/' + action, {{ method: 'POST' }}).catch(() => {{}});
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
        '<span class="badge' + (s.active ? ' badge-active' : ' badge-dim') + '"'
        + ' data-category="' + (s.category || '') + '"'
        + (s.uri ? ' data-uri="' + s.uri + '"' : '')
        + '>' + s.name + '</span>'
      ).join('') + '</div>';
      if (sourcesEl.innerHTML !== srcHtml) {{
        sourcesEl.innerHTML = srcHtml;
        bindSourceClicks(sourcesEl);
      }}
      sourcesEl.classList.add('visible');
    }} else {{
      sourcesEl.innerHTML = '';
      sourcesEl.classList.remove('visible');
    }}
  }}

  function bindSourceClicks(container) {{
    container.querySelectorAll('.badge[data-uri]').forEach(badge => {{
      badge.addEventListener('click', function() {{
        const uri = this.dataset.uri;
        const cat = this.dataset.category;
        if (!uri || this.classList.contains('badge-active')) return;
        // Optimistic UI: activate this badge, dim all others
        container.querySelectorAll('.badge').forEach(b => {{
          b.classList.remove('badge-active');
          b.classList.add('badge-dim');
        }});
        this.classList.remove('badge-dim');
        this.classList.add('badge-active');
        // Update lastSonyDetail so poll() respects the lock
        if (lastSonyDetail && lastSonyDetail.sources) {{
          lastSonyDetail.sources.forEach(s => {{ s.active = s.category === cat; }});
        }}
        sonySourceLock = cat;
        sonySourceLockExpiry = Date.now() + 10000;
        // Fire API call using the known-good set_input endpoint
        fetch('/sony_receiver/' + encodeURIComponent(SONY_DEVICE) + '/input', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{ uri: uri }})
        }}).catch(() => {{}});
      }});
    }});
  }}

  function arcamBadges(detail) {{
    if (!detail) return [];
    const b = [];
    if (detail.mode) b.push({{ text: detail.mode, id: 'arcam-mode-badge' }});
    if (detail.muted) b.push({{ text: 'MUTED', cls: 'badge-muted' }});
    if (detail.auto_shutdown !== undefined) {{
      b.push({{
        text: 'Auto Shutdown: ' + detail.auto_shutdown_label,
        id: 'arcam-auto-shutdown-badge',
        cls: detail.auto_shutdown === 0 ? 'badge-auto-shutdown-off' : 'badge-auto-shutdown'
      }});
    }}
    return b;
  }}

  function hoverPopover(trigger, pop) {{
    let showT, hideT;
    const created = Date.now();
    trigger.addEventListener('mouseenter', () => {{
      // Skip synthetic mouseenter from innerHTML recreation under cursor
      if (Date.now() - created < 100) return;
      clearTimeout(hideT);
      showT = setTimeout(() => {{ try {{ pop.showPopover(); }} catch(_) {{}} }}, 1000);
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

  function setupAutoShutdownBadgeHover() {{
    const badge = document.getElementById('arcam-auto-shutdown-badge');
    const pop = document.getElementById('pop-arcam-auto-shutdown');
    const select = document.getElementById('auto-shutdown-select');
    
    // Make badge clickable instead of hover (dropdown now in popover)
    if (badge && !badge._clickBound) {{
      badge.style.cursor = 'pointer';
      badge.addEventListener('click', (e) => {{
        e.preventDefault();
        try {{
          if (pop.matches(':popover-open')) {{
            pop.hidePopover();
          }} else {{
            pop.showPopover();
          }}
        }} catch(_) {{}}
      }});
      badge._clickBound = true;
    }}
    
    // Handle dropdown change — optimistic instant update
    if (select && !select._changeBound) {{
      // Use same labels as server (arcam::auto_shutdown_label) for consistency
      const BADGE_LABELS = {{ '0': 'Off', '1': '20 min', '2': '30 min', '3': '1 hour', '4': '2 hours' }};
      select.addEventListener('change', () => {{
        const value = parseInt(select.value, 10);
        const label = BADGE_LABELS[select.value] || 'Unknown';

        // Re-query badge (poll() rebuilds innerHTML so the captured ref may be stale)
        const liveBadge = document.getElementById('arcam-auto-shutdown-badge');
        if (liveBadge) {{
          liveBadge.textContent = 'Auto Shutdown: ' + label;
          liveBadge.className = 'badge ' + (value === 0 ? 'badge-auto-shutdown-off' : 'badge-auto-shutdown');
        }}
        lastArcamOnDetail = lastArcamOnDetail || {{}};
        lastArcamOnDetail.auto_shutdown = value;
        lastArcamOnDetail.auto_shutdown_label = label;
        // Lock: prevent poll() from overwriting badge until server confirms
        arcamAutoShutdownLock = {{ value, label }};
        arcamAutoShutdownLockExpiry = Date.now() + 15000;
        try {{ pop.hidePopover(); }} catch(_) {{}}

        // Fire-and-forget API call (badge already updated)
        fetch('/arcam/auto-shutdown', {{
          method: 'PUT',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{ value }})
        }}).catch(() => {{}});
      }});
      select._changeBound = true;
    }}
    
    // Sync select with current value when popover opens
    if (badge && pop && !pop._syncBound) {{
      const originalShow = pop.showPopover;
      pop.showPopover = function() {{
        originalShow.call(this);
        // Fetch current value
        fetch('/arcam/auto-shutdown')
          .then(r => r.json())
          .then(data => {{
            if (!data.error && select) {{
              select.value = data.value.toString();
            }}
          }})
          .catch(() => {{}});
      }};
      pop._syncBound = true;
    }}
  }}

  async function poll() {{
    try {{
      const resp = await fetch("/status");
      if (!resp.ok) throw new Error('status ' + resp.status);
      const data = await resp.json();

      // Server is reachable — if recovering, reload for fresh state
      if (serverUnreachable) {{
        location.reload();
        return;
      }}
      consecutiveFailures = 0;

      // --- Sony (lock-based) ---
      if (data.sony.detail) lastSonyDetail = data.sony.detail;
      const sonyLocked = sonyLockClass && Date.now() < sonyLockExpiry;
      if (sonyLocked && toClass(data.sony.status) !== sonyLockClass) {{
        // Still transitioning — keep optimistic state
      }} else {{
        if (sonyLocked) sonyLockClass = null;
        // Source lock: override active source until server confirms the switch
        if (sonySourceLock && Date.now() < sonySourceLockExpiry && data.sony.detail && data.sony.detail.sources) {{
          const confirmed = data.sony.detail.sources.some(s => s.category === sonySourceLock && s.active);
          if (confirmed) {{
            sonySourceLock = null;
          }} else {{
            data.sony.detail.sources.forEach(s => {{ s.active = s.category === sonySourceLock; }});
          }}
        }} else {{
          sonySourceLock = null;
        }}
        setDot(document.getElementById("sony-dot"), toClass(data.sony.status));
        setText(document.getElementById("sony-label"), data.sony.label);
        renderSony(document.getElementById("sony-instruments"), document.getElementById("sony-sources"), data.sony.detail);
      }}

      // --- Arcam (lock-based) ---
      // Always cache detail for optimistic rendering
      if (data.arcam.detail && data.arcam.detail.model) lastArcamModel = data.arcam.detail.model;
      if (data.arcam.status === 'active' && data.arcam.detail) lastArcamOnDetail = data.arcam.detail;

      const locked = arcamLockClass && Date.now() < arcamLockExpiry;
      if (locked && toClass(data.arcam.status) !== arcamLockClass) {{
        // Still transitioning — keep optimistic state, skip update
      }} else {{
        // Either unlocked, or real data matches expected state — render real data
        if (locked) arcamLockClass = null; // confirmed, unlock
        setDot(document.getElementById("arcam-dot"), toClass(data.arcam.status));
        setText(document.getElementById("arcam-label"), data.arcam.label);
        const arcamInstruments = document.getElementById("arcam-instruments");
        if (data.arcam.status === 'standby' && data.arcam.detail && data.arcam.detail.model) {{
          const want = '<span class="device-model">' + data.arcam.detail.model + '</span>'
            + '<span class="heartbeat" title="Heartbeat keepalive active">\u2764\uFE0F</span>';
          if (arcamInstruments.innerHTML !== want) arcamInstruments.innerHTML = want;
        }} else if (data.arcam.status === 'active') {{
          // Auto-shutdown lock: override stale server data with optimistic value
          if (arcamAutoShutdownLock && Date.now() < arcamAutoShutdownLockExpiry) {{
            if (data.arcam.detail && data.arcam.detail.auto_shutdown !== arcamAutoShutdownLock.value) {{
              data.arcam.detail.auto_shutdown = arcamAutoShutdownLock.value;
              data.arcam.detail.auto_shutdown_label = arcamAutoShutdownLock.label;
            }} else {{
              arcamAutoShutdownLock = null; // server confirmed, unlock
            }}
          }} else {{
            arcamAutoShutdownLock = null;
          }}
          const badges = arcamBadges(data.arcam.detail);
          const want = badges.length ? '<div class="badge-row">' + badges.map(b =>
            '<span' + (b.id ? ' id="' + b.id + '"' : '') + ' class="badge' + (b.cls ? ' ' + b.cls : '') + '">' + b.text + '</span>'
          ).join('') + '</div>' : '';
          if (arcamInstruments.innerHTML !== want) {{
            arcamInstruments.innerHTML = want;
            setupModeBadgeHover();
            setupAutoShutdownBadgeHover();
          }}
        }} else {{
          if (arcamInstruments.innerHTML !== '') arcamInstruments.innerHTML = '';
        }}
      }}
    }} catch (_) {{
      consecutiveFailures++;
      if (consecutiveFailures >= FAILURE_THRESHOLD && !serverUnreachable) {{
        serverUnreachable = true;
        document.getElementById('frost-overlay').classList.add('visible');
      }}
    }}
  }}

  poll();
  setInterval(poll, 3000);

  // Separate timeout counter poll — independent TCP connection, no command chain contamination
  async function pollTimeout() {{
    try {{
      const resp = await fetch('/arcam/timeout');
      if (!resp.ok) return;
      const d = await resp.json();
      if (d.error) {{
        console.warn('[arcam] timeout_counter error:', d.error);
      }} else {{
        console.log('[arcam] timeout_counter raw:', {{
          answer_code: '0x' + d.answer_code.toString(16).padStart(2, '0'),
          data_len: d.data_len,
          data_hex: d.data_hex.join(' '),
          data_raw: d.data_raw,
          parsed_be: d.parsed_be + 's (' + Math.floor(d.parsed_be/60) + 'm ' + (d.parsed_be%60) + 's)',
          parsed_le: d.parsed_le + 's (' + Math.floor(d.parsed_le/60) + 'm ' + (d.parsed_le%60) + 's)',
          frame: d.full_frame_hex.join(' ')
        }});
      }}
    }} catch (_) {{}}
  }}
  pollTimeout();
  setInterval(pollTimeout, 30000);
}})();
</script>

</body>
</html>"#,
        status_to_css(sony_status.status),
        sony_status.label,
        sony_host,
        status_to_css(arcam_status.status),
        arcam_status.label,
        arcam_host,
        sony_device_name,
        arcam_device_name,
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
