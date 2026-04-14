//! Output rendering for the `geo` CLI.
//!
//! Provides three modes:
//! - **text** (default): human-readable, optionally ANSI-styled
//! - **plain**: text with escape codes stripped
//! - **json**: machine-readable JSON via serde
//!
//! All public render functions return a `String` that the caller writes to
//! stdout. Errors are rendered separately by `render_error`.

use std::fmt::Write;
use std::io::IsTerminal;

use biscuit_location::{DistanceUnit, Location, LocationSource};
use serde::Serialize;

/// Which output contract the CLI is honoring for this invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Terminal-friendly text with optional ANSI styling.
    Text,
    /// Text with all ANSI escape codes stripped.
    Plain,
    /// Machine-readable JSON.
    Json,
}

impl OutputMode {
    /// Resolve the output mode from CLI flags and the runtime environment.
    ///
    /// Rules (highest precedence first):
    /// 1. `--json` wins if set.
    /// 2. `--plain` wins otherwise.
    /// 3. If `NO_COLOR` env var is set, treat as plain.
    /// 4. If `FORCE_COLOR` env var is set, keep styled text.
    /// 5. If stdout is not a TTY, treat as plain.
    /// 6. Default: styled text.
    pub fn resolve(json: bool, plain: bool) -> Self {
        if json {
            return Self::Json;
        }
        if plain {
            return Self::Plain;
        }
        if std::env::var_os("FORCE_COLOR").is_some() {
            return Self::Text;
        }
        if std::env::var_os("NO_COLOR").is_some() {
            return Self::Plain;
        }
        if !std::io::stdout().is_terminal() {
            return Self::Plain;
        }
        Self::Text
    }

    /// Returns `true` if ANSI escape codes should be emitted.
    pub fn allows_color(self) -> bool {
        matches!(self, Self::Text)
    }
}

// ---------------------------------------------------------------------------
// JSON response models
// ---------------------------------------------------------------------------

/// JSON-serializable view of a resolved location.
#[derive(Debug, Serialize)]
pub struct LocationResponse<'a> {
    pub coordinates: CoordinatesJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<&'a biscuit_location::Place>,
    pub source: SourceJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy_meters: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maps_url: Option<String>,
}

/// JSON-friendly coordinates (avoids the library's Display-based format).
#[derive(Debug, Serialize)]
pub struct CoordinatesJson {
    pub latitude: f64,
    pub longitude: f64,
}

/// JSON-friendly location source.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceJson {
    Gps,
    Ip { address: String },
    ReverseGeocode,
    CoordinatesLiteral,
}

impl From<&LocationSource> for SourceJson {
    fn from(src: &LocationSource) -> Self {
        match src {
            LocationSource::Gps => Self::Gps,
            LocationSource::Ip { ip } => Self::Ip {
                address: ip.to_string(),
            },
            LocationSource::ReverseGeocode => Self::ReverseGeocode,
            LocationSource::CoordinatesLiteral => Self::CoordinatesLiteral,
        }
    }
}

impl<'a> LocationResponse<'a> {
    pub fn from_location(location: &'a Location, maps_url: Option<String>) -> Self {
        Self {
            coordinates: CoordinatesJson {
                latitude: location.coordinates.latitude,
                longitude: location.coordinates.longitude,
            },
            place: location.place.as_ref(),
            source: SourceJson::from(&location.source),
            accuracy_meters: location.accuracy_meters,
            maps_url,
        }
    }
}

/// JSON-serializable distance result.
#[derive(Debug, Serialize)]
pub struct DistanceResponse {
    pub value: f64,
    pub unit: DistanceUnit,
}

/// JSON-serializable "no GPS fix" response.
#[derive(Debug, Serialize)]
pub struct NoGpsResponse {
    pub ok: bool,
    pub reason: &'static str,
}

impl NoGpsResponse {
    pub fn new() -> Self {
        Self {
            ok: false,
            reason: "no_gps_fix",
        }
    }
}

/// Error envelope used for `--json` error output on stderr.
#[derive(Debug, Serialize)]
pub struct ErrorResponse<'a> {
    pub error: bool,
    pub message: &'a str,
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render a location for the given output mode.
pub fn render_location(location: &Location, maps_url: Option<String>, mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => {
            let response = LocationResponse::from_location(location, maps_url);
            // Safe: our response types serialize to finite JSON with no maps
            // keyed by non-string values.
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".into())
        }
        OutputMode::Text | OutputMode::Plain => {
            render_location_text(location, maps_url.as_deref(), mode.allows_color())
        }
    }
}

/// Render a distance value for the given output mode.
pub fn render_distance(value: f64, unit: DistanceUnit, mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => {
            let response = DistanceResponse { value, unit };
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".into())
        }
        OutputMode::Text | OutputMode::Plain => render_distance_text(value, unit),
    }
}

/// Render the "no GPS fix" message for the given output mode.
pub fn render_no_gps(mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => {
            serde_json::to_string_pretty(&NoGpsResponse::new()).unwrap_or_else(|_| "{}".into())
        }
        OutputMode::Text | OutputMode::Plain => "No GPS fix available.".into(),
    }
}

/// Render an error message for the given output mode.
///
/// The returned string is meant for stderr. In JSON mode it contains a JSON
/// envelope on a single line; in text modes it contains a styled (or plain)
/// prose message.
pub fn render_error(message: &str, mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => {
            let response = ErrorResponse {
                error: true,
                message,
            };
            serde_json::to_string(&response).unwrap_or_else(|_| "{\"error\":true}".into())
        }
        OutputMode::Plain => format!("Error: {message}"),
        OutputMode::Text => format!("{BOLD}{RED}Error:{RESET} {message}"),
    }
}

/// Render an informational message for the given output mode.
pub fn render_info(message: &str, mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => {
            #[derive(Debug, Serialize)]
            struct InfoResponse<'a> {
                ok: bool,
                message: &'a str,
            }
            let response = InfoResponse { ok: true, message };
            serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":true}".into())
        }
        OutputMode::Plain => message.to_string(),
        OutputMode::Text => format!("{BOLD}✓{RESET} {message}"),
    }
}

// ---------------------------------------------------------------------------
// Text rendering helpers
// ---------------------------------------------------------------------------

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

fn render_location_text(location: &Location, maps_url: Option<&str>, color: bool) -> String {
    let mut out = String::new();

    // Line 1: place string (if any)
    if let Some(place) = &location.place {
        let display = place.to_string();
        if !display.is_empty() {
            let _ = writeln!(out, "{display}");
        }
    }

    // Line 2: coordinates
    let _ = writeln!(out, "{}", location.coordinates);

    // Line 3: source
    let source = match &location.source {
        LocationSource::Gps => "GPS".to_string(),
        LocationSource::Ip { ip } => format!("IP: {ip}"),
        LocationSource::ReverseGeocode => "Reverse geocode".to_string(),
        LocationSource::CoordinatesLiteral => "Coordinates".to_string(),
    };
    write_label(&mut out, "Source", &source, color);

    if let Some(acc) = location.accuracy_meters {
        write_label(&mut out, "Accuracy", &format!("{acc:.0} m"), color);
    }

    if let Some(url) = maps_url {
        write_label(&mut out, "Maps", url, color);
    }

    // Strip trailing newline so callers can println! without a double blank line.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

fn write_label(out: &mut String, label: &str, value: &str, color: bool) {
    if color {
        let _ = writeln!(out, "{DIM}{label}:{RESET} {value}");
    } else {
        let _ = writeln!(out, "{label}: {value}");
    }
}

fn render_distance_text(value: f64, unit: DistanceUnit) -> String {
    let unit_str = match unit {
        DistanceUnit::Meters => "meters",
        DistanceUnit::Kilometers => "km",
        DistanceUnit::Miles => "miles",
        DistanceUnit::NauticalMiles => "nautical miles",
    };
    format!("{value:.2} {unit_str}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_location::{Coordinates, Place};
    use std::net::IpAddr;

    fn sample_location() -> Location {
        Location {
            coordinates: Coordinates::new(34.0522, -118.2437).unwrap(),
            place: Some(Place {
                city: Some("Los Angeles".into()),
                region: Some("California".into()),
                country: Some("United States".into()),
                ..Default::default()
            }),
            source: LocationSource::Ip {
                ip: "8.8.8.8".parse::<IpAddr>().unwrap(),
            },
            accuracy_meters: Some(1000.0),
        }
    }

    #[test]
    fn text_mode_renders_basic_fields() {
        let loc = sample_location();
        let out = render_location(&loc, None, OutputMode::Plain);
        assert!(out.contains("Los Angeles, California, United States"));
        assert!(out.contains("34.0522, -118.2437"));
        assert!(out.contains("Source: IP: 8.8.8.8"));
        assert!(out.contains("Accuracy: 1000 m"));
        assert!(!out.contains("Maps:"));
    }

    #[test]
    fn text_mode_plain_has_no_escape_codes() {
        let loc = sample_location();
        let out = render_location(&loc, None, OutputMode::Plain);
        assert!(!out.contains('\x1b'), "plain output contained escape codes");
    }

    #[test]
    fn text_mode_styled_includes_escape_codes() {
        let loc = sample_location();
        let out = render_location(&loc, None, OutputMode::Text);
        assert!(out.contains('\x1b'), "styled output missing escape codes");
    }

    #[test]
    fn text_mode_trims_trailing_newline() {
        let loc = sample_location();
        let out = render_location(&loc, None, OutputMode::Plain);
        assert!(!out.ends_with('\n'));
    }

    #[test]
    fn text_mode_includes_maps_url_when_present() {
        let loc = sample_location();
        let out = render_location(
            &loc,
            Some("https://maps.google.com/x".into()),
            OutputMode::Plain,
        );
        assert!(out.contains("Maps: https://maps.google.com/x"));
    }

    #[test]
    fn text_mode_skips_place_when_empty() {
        let mut loc = sample_location();
        loc.place = None;
        let out = render_location(&loc, None, OutputMode::Plain);
        // First line should be coordinates, not a place.
        assert!(out.starts_with("34.0522, -118.2437"));
    }

    #[test]
    fn json_mode_produces_valid_json() {
        let loc = sample_location();
        let out = render_location(&loc, Some("https://g.co/x".into()), OutputMode::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(parsed["coordinates"]["latitude"], 34.0522);
        assert_eq!(parsed["coordinates"]["longitude"], -118.2437);
        assert_eq!(parsed["place"]["city"], "Los Angeles");
        assert_eq!(parsed["source"]["kind"], "ip");
        assert_eq!(parsed["source"]["address"], "8.8.8.8");
        assert_eq!(parsed["accuracy_meters"], 1000.0);
        assert_eq!(parsed["maps_url"], "https://g.co/x");
    }

    #[test]
    fn json_mode_omits_optional_fields_when_none() {
        let mut loc = sample_location();
        loc.place = None;
        loc.accuracy_meters = None;
        let out = render_location(&loc, None, OutputMode::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed.get("place").is_none());
        assert!(parsed.get("accuracy_meters").is_none());
        assert!(parsed.get("maps_url").is_none());
    }

    #[test]
    fn json_mode_gps_source_shape() {
        let mut loc = sample_location();
        loc.source = LocationSource::Gps;
        let out = render_location(&loc, None, OutputMode::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["source"]["kind"], "gps");
        assert!(parsed["source"].get("address").is_none());
    }

    #[test]
    fn distance_text_formats_with_two_decimals() {
        let out = render_distance(3936.1234, DistanceUnit::Kilometers, OutputMode::Plain);
        assert_eq!(out, "3936.12 km");
    }

    #[test]
    fn distance_json_roundtrips() {
        let out = render_distance(100.0, DistanceUnit::Miles, OutputMode::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["value"], 100.0);
        assert_eq!(parsed["unit"], "miles");
    }

    #[test]
    fn distance_json_unit_names_are_snake_case() {
        let out = render_distance(1.0, DistanceUnit::NauticalMiles, OutputMode::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["unit"], "nautical_miles");
    }

    #[test]
    fn no_gps_text_message() {
        assert_eq!(render_no_gps(OutputMode::Plain), "No GPS fix available.");
    }

    #[test]
    fn no_gps_json_shape() {
        let out = render_no_gps(OutputMode::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["ok"], false);
        assert_eq!(parsed["reason"], "no_gps_fix");
    }

    #[test]
    fn error_text_styled() {
        let out = render_error("boom", OutputMode::Text);
        assert!(out.contains('\x1b'));
        assert!(out.contains("boom"));
    }

    #[test]
    fn error_plain_unstyled() {
        let out = render_error("boom", OutputMode::Plain);
        assert_eq!(out, "Error: boom");
    }

    #[test]
    fn error_json_envelope() {
        let out = render_error("boom", OutputMode::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["error"], true);
        assert_eq!(parsed["message"], "boom");
    }

    #[test]
    fn output_mode_resolves_json_over_plain() {
        // Explicit flags win.
        assert_eq!(OutputMode::resolve(true, false), OutputMode::Json);
        assert_eq!(OutputMode::resolve(false, true), OutputMode::Plain);
        // NOTE: `resolve` with json=true, plain=true is prevented by clap conflicts_with.
    }
}
