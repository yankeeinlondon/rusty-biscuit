# biscuit-location Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a new `biscuit-location` package area (lib + CLI) providing host GPS lookup, IP-to-location, reverse geocoding, distance calculation, and Google Maps link generation.

**Architecture:** A `LocationService` facade holds configuration and adapters for each capability. Internal modules (`ip`, `reverse`, `distance`, `maps`, `gps`) are private; types and the service are re-exported at the crate root. The CLI binary (`where`) dispatches subcommands through the service.

**Tech Stack:** Rust 2024 edition, `maxminddb` for IP lookup, `geo` for distance, `reqwest` for Nominatim reverse geocoding, `objc2-core-location` for macOS GPS, `clap` for CLI, `tokio` async runtime, `wiremock` for HTTP mocking in tests.

**Design deviation:** The tech-design specifies the `geocoding` crate for reverse geocoding. This plan uses `reqwest` directly instead, because `geocoding`'s `Reverse` trait returns only a display-name string (`Option<String>`), not the structured address components (`city`, `region`, `country`, `postal_code`, `timezone`) that the `Place` struct requires. Using `reqwest` with Nominatim's `addressdetails=1` JSON response gives us all fields, plus full control over endpoint URL, user-agent, timeout, and rate limiting — exactly what the tech-design's wrapper module demands.

---

## File Map

### Files to Create

| File | Responsibility |
|------|---------------|
| `biscuit-location/justfile` | Package area build/test/lint recipes |
| `biscuit-location/lib/Cargo.toml` | Library crate manifest |
| `biscuit-location/lib/src/lib.rs` | Crate root, module declarations, re-exports |
| `biscuit-location/lib/src/types.rs` | `Coordinates`, `Place`, `Location`, `LocationSource`, `Distance`, `DistanceUnit`, `LocationInput` |
| `biscuit-location/lib/src/error.rs` | `LocationError` enum with `thiserror` |
| `biscuit-location/lib/src/config.rs` | `LocationConfig`, `ReverseGeocodeConfig`, MaxMind path resolution |
| `biscuit-location/lib/src/maps.rs` | Google Maps URL generation |
| `biscuit-location/lib/src/distance.rs` | Geodesic distance via `geo` crate |
| `biscuit-location/lib/src/ip.rs` | MaxMind IP-to-location lookup |
| `biscuit-location/lib/src/reverse.rs` | Nominatim reverse geocoding via `reqwest` |
| `biscuit-location/lib/src/gps/mod.rs` | GPS facade dispatching to platform backends |
| `biscuit-location/lib/src/gps/macos.rs` | macOS CoreLocation backend |
| `biscuit-location/lib/src/gps/windows.rs` | Windows Geolocation stub |
| `biscuit-location/lib/src/gps/linux.rs` | Linux GeoClue stub |
| `biscuit-location/lib/src/service.rs` | `LocationService` — configured facade over all modules |
| `biscuit-location/cli/Cargo.toml` | CLI crate manifest |
| `biscuit-location/cli/src/main.rs` | Entry point (`#[tokio::main]`) |
| `biscuit-location/cli/src/args.rs` | Clap derive structs and `LocationInput` parsing |
| `biscuit-location/cli/src/commands.rs` | Subcommand dispatch (`gps`, `ip`, `reverse`, `distance`) |
| `biscuit-location/cli/src/output.rs` | Human-readable output formatting |
| `biscuit-location/cli/tests/cli_tests.rs` | Integration tests with `assert_cmd` |
| `biscuit-location/docs/dependencies.md` | Package-level dependency documentation |

### Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` (root) | Add `biscuit-location/lib` and `biscuit-location/cli` to workspace members |
| `justfile` (root) | Add `biscuit-location` to the `areas` list |
| `biscuit-location/README.md` | Update with usage examples and feature overview |
| `biscuit-location/lib/README.md` | Fill with library API documentation |
| `biscuit-location/cli/README.md` | Fill with CLI usage documentation |

---

## Task 1: Project Scaffolding

**Files:**
- Create: `biscuit-location/lib/Cargo.toml`
- Create: `biscuit-location/cli/Cargo.toml`
- Create: `biscuit-location/justfile`
- Create: `biscuit-location/lib/src/lib.rs`
- Create: `biscuit-location/cli/src/main.rs`
- Modify: `Cargo.toml` (root workspace)
- Modify: `justfile` (root)

- [ ] **Step 1: Create `biscuit-location/lib/Cargo.toml`**

```toml
[package]
name = "biscuit-location"
version = "0.1.0"
edition = "2024"

[dependencies]
dirs = "6"
geo = "0.29"
maxminddb = "0.27"
reqwest = { version = "0.12", features = ["json"], default-features = false, optional = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["rt", "macros", "time", "sync"] }
url = "2"

[target.'cfg(target_os = "macos")'.dependencies]
block2 = "0.6"
objc2 = "0.6"
objc2-core-location = "0.3"
objc2-foundation = "0.3"

[features]
default = ["reverse"]
reverse = ["dep:reqwest"]

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
wiremock = "0.6"
```

- [ ] **Step 2: Create `biscuit-location/cli/Cargo.toml`**

```toml
[package]
name = "biscuit-location-cli"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "where"
path = "src/main.rs"

[dependencies]
biscuit-location = { path = "../lib" }
clap = { version = "4.5", features = ["derive", "wrap_help"] }
color-eyre = "0.6"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 3: Create minimal `biscuit-location/lib/src/lib.rs`**

```rust
//! Location services for the biscuit ecosystem.
//!
//! Provides host GPS lookup, IP-to-location resolution, reverse geocoding,
//! distance calculation, and Google Maps link generation.
```

- [ ] **Step 4: Create minimal `biscuit-location/cli/src/main.rs`**

```rust
fn main() {
    println!("where CLI — coming soon");
}
```

- [ ] **Step 5: Create `biscuit-location/justfile`**

```just
BOLD := '\033[1m'
DIM := '\033[2m'
ITALIC := '\033[3m'
RESET := '\033[0m'
RED := '\033[31m'

default:
    @echo "biscuit-location Library & CLI"
    @echo "=============================="
    @echo
    @just --list | grep -v 'default'

# build library and CLI
build *args="":
    @echo "Building {{BOLD}}biscuit-location{{RESET}} library..."
    @cargo build -p biscuit-location {{args}}
    @echo ""
    @echo "Building {{BOLD}}biscuit-location-cli{{RESET}}..."
    @cargo build -p biscuit-location-cli {{args}}

# test library and CLI
test *args="":
    @echo "Testing {{BOLD}}biscuit-location{{RESET}} library"
    @cargo test -p biscuit-location --all-features -- {{args}}
    @echo ""
    @echo "Testing {{BOLD}}biscuit-location-cli{{RESET}}"
    @cargo test -p biscuit-location-cli -- {{args}}

# lint with clippy
lint *args="":
    @cargo clippy -p biscuit-location -p biscuit-location-cli -- {{args}}

# install CLI binary
install *args="":
    @cargo install --path cli {{args}}

# build and show the docs for the library code
docs:
    @echo "Build and then show latest docs for the biscuit-location library code"
    @echo ""
    @cargo doc -p biscuit-location --open

# run CLI in development mode
cli *args="":
    @cargo run -p biscuit-location-cli -- {{args}}
```

- [ ] **Step 6: Add workspace members to root `Cargo.toml`**

Add these two entries to the `[workspace] members` array (alphabetical placement near other biscuit- entries):

```toml
    "biscuit-location/cli",
    "biscuit-location/lib",
```

- [ ] **Step 7: Add to root `justfile` areas**

In the root `justfile`, add `biscuit-location` to the `areas` variable:

```just
areas := "biscuit-hash biscuit-location biscuit-speaks biscuit-terminal schematic biscuit-file unchained-ai playa so-you-say tree-hugger darkmatter sniff model-citizen claudine research queue homelab"
```

- [ ] **Step 8: Verify the scaffolding compiles**

Run: `cargo build -p biscuit-location -p biscuit-location-cli`
Expected: Compiles with no errors (warnings about unused code are OK at this stage).

- [ ] **Step 9: Commit scaffolding**

```bash
git add biscuit-location/lib/Cargo.toml biscuit-location/cli/Cargo.toml \
  biscuit-location/justfile biscuit-location/lib/src/lib.rs \
  biscuit-location/cli/src/main.rs Cargo.toml justfile Cargo.lock
git commit -m "feat(biscuit-location): add workspace scaffolding for lib and CLI"
```

---

## Task 2: Core Types

**Files:**
- Create: `biscuit-location/lib/src/types.rs`
- Create: `biscuit-location/lib/src/error.rs`
- Modify: `biscuit-location/lib/src/lib.rs`

- [ ] **Step 1: Write failing tests for `Coordinates` validation**

Create `biscuit-location/lib/src/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;

use crate::error::LocationError;

/// A validated latitude/longitude pair.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_coordinates() {
        let c = Coordinates::new(37.7749, -122.4194).unwrap();
        assert_eq!(c.latitude, 37.7749);
        assert_eq!(c.longitude, -122.4194);
    }

    #[test]
    fn boundary_coordinates() {
        assert!(Coordinates::new(90.0, 180.0).is_ok());
        assert!(Coordinates::new(-90.0, -180.0).is_ok());
        assert!(Coordinates::new(0.0, 0.0).is_ok());
    }

    #[test]
    fn invalid_latitude() {
        assert!(Coordinates::new(91.0, 0.0).is_err());
        assert!(Coordinates::new(-91.0, 0.0).is_err());
    }

    #[test]
    fn invalid_longitude() {
        assert!(Coordinates::new(0.0, 181.0).is_err());
        assert!(Coordinates::new(0.0, -181.0).is_err());
    }
}
```

- [ ] **Step 2: Create `error.rs` with `LocationError`**

Create `biscuit-location/lib/src/error.rs`:

```rust
use std::net::IpAddr;

/// Alias for `Result<T, LocationError>`.
pub type Result<T> = std::result::Result<T, LocationError>;

/// Errors produced by biscuit-location operations.
#[derive(Debug, thiserror::Error)]
pub enum LocationError {
    #[error("invalid coordinates: latitude must be in [-90, 90] and longitude in [-180, 180], got ({latitude}, {longitude})")]
    InvalidCoordinates { latitude: f64, longitude: f64 },

    #[error("invalid location input: {0}")]
    InvalidLocationInput(String),

    #[error("MaxMind database not found at {0}")]
    DatabasePathNotFound(String),

    #[error("failed to open MaxMind database: {0}")]
    DatabaseOpen(String),

    #[error("IP lookup failed: {0}")]
    IpLookup(String),

    #[error("no location data found for IP {0}")]
    IpNotFound(IpAddr),

    #[error("reverse geocoding failed: {0}")]
    ReverseGeocode(String),

    #[error("failed to build Google Maps URL: {0}")]
    GoogleMapsUrl(String),

    #[error("GPS not supported on this platform")]
    UnsupportedPlatform,

    #[error("internal error: {0}")]
    Internal(String),
}
```

- [ ] **Step 3: Wire modules into `lib.rs` and run tests to verify failure**

Update `biscuit-location/lib/src/lib.rs`:

```rust
//! Location services for the biscuit ecosystem.
//!
//! Provides host GPS lookup, IP-to-location resolution, reverse geocoding,
//! distance calculation, and Google Maps link generation.

mod error;
mod types;

pub use error::{LocationError, Result};
pub use types::*;
```

Run: `cargo test -p biscuit-location`
Expected: FAIL — `Coordinates::new` does not exist yet.

- [ ] **Step 4: Implement `Coordinates::new` and remaining types**

Update `biscuit-location/lib/src/types.rs` — add the implementation above the `#[cfg(test)]` block:

```rust
impl Coordinates {
    /// Create validated coordinates.
    ///
    /// ## Errors
    ///
    /// Returns `InvalidCoordinates` if latitude is outside `[-90, 90]`
    /// or longitude is outside `[-180, 180]`.
    pub fn new(latitude: f64, longitude: f64) -> crate::Result<Self> {
        if !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) {
            return Err(LocationError::InvalidCoordinates {
                latitude,
                longitude,
            });
        }
        Ok(Self {
            latitude,
            longitude,
        })
    }
}

impl fmt::Display for Coordinates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.latitude, self.longitude)
    }
}

/// City-level place metadata from geocoding or IP lookup.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub city: Option<String>,
    pub region: Option<String>,
    pub region_code: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub postal_code: Option<String>,
    pub timezone: Option<String>,
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<&str> = [
            self.city.as_deref(),
            self.region.as_deref(),
            self.country.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();
        write!(f, "{}", parts.join(", "))
    }
}

/// How a location was obtained.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LocationSource {
    Gps,
    Ip { ip: IpAddr },
    ReverseGeocode,
    CoordinatesLiteral,
}

/// A resolved geographic location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub coordinates: Coordinates,
    pub place: Option<Place>,
    pub source: LocationSource,
    pub accuracy_meters: Option<f64>,
}

/// A distance stored canonically in meters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Distance {
    pub meters: f64,
}

/// Units for displaying distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceUnit {
    Meters,
    Kilometers,
    Miles,
    NauticalMiles,
}

impl Distance {
    /// Convert the canonical meter value to the requested unit.
    pub fn as_unit(&self, unit: DistanceUnit) -> f64 {
        match unit {
            DistanceUnit::Meters => self.meters,
            DistanceUnit::Kilometers => self.meters / 1_000.0,
            DistanceUnit::Miles => self.meters / 1_609.344,
            DistanceUnit::NauticalMiles => self.meters / 1_852.0,
        }
    }
}

/// A user-supplied location reference (CLI input grammar).
///
/// Parses from strings:
/// - `"gps"` — host GPS fix
/// - `"ip:1.2.3.4"` — IP lookup
/// - `"37.77,-122.41"` — coordinate literal
#[derive(Debug, Clone, PartialEq)]
pub enum LocationInput {
    Gps,
    Ip(IpAddr),
    Coordinates(Coordinates),
}

impl FromStr for LocationInput {
    type Err = LocationError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("gps") {
            return Ok(Self::Gps);
        }
        if let Some(ip_str) = s.strip_prefix("ip:") {
            let ip: IpAddr = ip_str
                .parse()
                .map_err(|_| LocationError::InvalidLocationInput(s.to_string()))?;
            return Ok(Self::Ip(ip));
        }
        // Try lat,lon
        if let Some((lat_str, lon_str)) = s.split_once(',') {
            let lat: f64 = lat_str
                .trim()
                .parse()
                .map_err(|_| LocationError::InvalidLocationInput(s.to_string()))?;
            let lon: f64 = lon_str
                .trim()
                .parse()
                .map_err(|_| LocationError::InvalidLocationInput(s.to_string()))?;
            let coords = Coordinates::new(lat, lon)?;
            return Ok(Self::Coordinates(coords));
        }
        Err(LocationError::InvalidLocationInput(s.to_string()))
    }
}
```

- [ ] **Step 5: Add `Distance` and `LocationInput` tests**

Append these tests inside the existing `mod tests` block in `types.rs`:

```rust
    #[test]
    fn distance_unit_conversion() {
        let d = Distance { meters: 1_609.344 };
        assert!((d.as_unit(DistanceUnit::Miles) - 1.0).abs() < 1e-9);
        assert!((d.as_unit(DistanceUnit::Kilometers) - 1.609344).abs() < 1e-6);
        assert!((d.as_unit(DistanceUnit::Meters) - 1_609.344).abs() < 1e-9);
    }

    #[test]
    fn distance_nautical_miles() {
        let d = Distance { meters: 1_852.0 };
        assert!((d.as_unit(DistanceUnit::NauticalMiles) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn location_input_gps() {
        assert_eq!("gps".parse::<LocationInput>().unwrap(), LocationInput::Gps);
        assert_eq!("GPS".parse::<LocationInput>().unwrap(), LocationInput::Gps);
    }

    #[test]
    fn location_input_ip() {
        let input: LocationInput = "ip:8.8.8.8".parse().unwrap();
        assert_eq!(
            input,
            LocationInput::Ip("8.8.8.8".parse().unwrap())
        );
    }

    #[test]
    fn location_input_ip_v6() {
        let input: LocationInput = "ip:2001:4860:4860::8888".parse().unwrap();
        assert_eq!(
            input,
            LocationInput::Ip("2001:4860:4860::8888".parse().unwrap())
        );
    }

    #[test]
    fn location_input_coordinates() {
        let input: LocationInput = "37.7749,-122.4194".parse().unwrap();
        assert_eq!(
            input,
            LocationInput::Coordinates(Coordinates::new(37.7749, -122.4194).unwrap())
        );
    }

    #[test]
    fn location_input_coordinates_with_spaces() {
        let input: LocationInput = "37.7749, -122.4194".parse().unwrap();
        assert_eq!(
            input,
            LocationInput::Coordinates(Coordinates::new(37.7749, -122.4194).unwrap())
        );
    }

    #[test]
    fn location_input_invalid() {
        assert!("not-a-location".parse::<LocationInput>().is_err());
        assert!("ip:not-an-ip".parse::<LocationInput>().is_err());
        assert!("999,0".parse::<LocationInput>().is_err()); // invalid latitude
    }

    #[test]
    fn place_display() {
        let place = Place {
            city: Some("Los Angeles".into()),
            region: Some("California".into()),
            country: Some("United States".into()),
            ..Default::default()
        };
        assert_eq!(place.to_string(), "Los Angeles, California, United States");
    }

    #[test]
    fn place_display_partial() {
        let place = Place {
            country: Some("Japan".into()),
            ..Default::default()
        };
        assert_eq!(place.to_string(), "Japan");
    }

    #[test]
    fn coordinates_display() {
        let c = Coordinates::new(34.0522, -118.2437).unwrap();
        assert_eq!(c.to_string(), "34.0522, -118.2437");
    }
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p biscuit-location`
Expected: All tests PASS.

- [ ] **Step 7: Commit core types and errors**

```bash
git add biscuit-location/lib/src/types.rs biscuit-location/lib/src/error.rs \
  biscuit-location/lib/src/lib.rs
git commit -m "feat(biscuit-location): add core types, errors, and LocationInput parsing"
```

---

## Task 3: Configuration

**Files:**
- Create: `biscuit-location/lib/src/config.rs`
- Modify: `biscuit-location/lib/src/lib.rs`

- [ ] **Step 1: Write failing tests for MaxMind path resolution**

Create `biscuit-location/lib/src/config.rs`:

```rust
use std::path::PathBuf;
use std::time::Duration;

use url::Url;

/// Top-level configuration for `LocationService`.
#[derive(Debug, Clone)]
pub struct LocationConfig {
    /// Explicit path to GeoLite2-City.mmdb. If `None`, uses env var or OS default.
    pub maxmind_db_path: Option<PathBuf>,
    /// GPS fix timeout.
    pub gps_timeout: Duration,
    /// Reverse geocoding configuration.
    pub reverse: ReverseGeocodeConfig,
}

impl Default for LocationConfig {
    fn default() -> Self {
        Self {
            maxmind_db_path: None,
            gps_timeout: Duration::from_secs(10),
            reverse: ReverseGeocodeConfig::default(),
        }
    }
}

/// Configuration for the Nominatim reverse geocoder.
#[derive(Debug, Clone)]
pub struct ReverseGeocodeConfig {
    /// Nominatim-compatible API base URL.
    pub endpoint: Url,
    /// HTTP User-Agent header value.
    pub user_agent: String,
    /// HTTP request timeout.
    pub timeout: Duration,
    /// Minimum interval between consecutive requests (rate limiting).
    pub min_interval: Duration,
}

impl Default for ReverseGeocodeConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://nominatim.openstreetmap.org/").unwrap(),
            user_agent: format!("where/{}", env!("CARGO_PKG_VERSION")),
            timeout: Duration::from_secs(10),
            min_interval: Duration::from_secs(1),
        }
    }
}

const MAXMIND_ENV_VAR: &str = "BISCUIT_LOCATION_MAXMIND_DB";
const MAXMIND_FILENAME: &str = "GeoLite2-City.mmdb";
const MAXMIND_APP_DIR: &str = "biscuit-location";

/// Resolve the MaxMind database path using the precedence chain:
///
/// 1. Explicit path from config
/// 2. `BISCUIT_LOCATION_MAXMIND_DB` environment variable
/// 3. OS-specific data directory default
pub fn resolve_maxmind_path(explicit: Option<&PathBuf>) -> Option<PathBuf> {
    // 1. Explicit config
    if let Some(path) = explicit {
        return Some(path.clone());
    }
    // 2. Environment variable
    if let Ok(env_path) = std::env::var(MAXMIND_ENV_VAR) {
        if !env_path.is_empty() {
            return Some(PathBuf::from(env_path));
        }
    }
    // 3. OS default
    dirs::data_dir().map(|d| d.join(MAXMIND_APP_DIR).join(MAXMIND_FILENAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_path_takes_precedence() {
        let explicit = PathBuf::from("/custom/path/GeoLite2-City.mmdb");
        let result = resolve_maxmind_path(Some(&explicit));
        assert_eq!(result, Some(explicit));
    }

    #[test]
    fn env_var_used_when_no_explicit_path() {
        // Save and restore env
        let prev = std::env::var(MAXMIND_ENV_VAR).ok();
        std::env::set_var(MAXMIND_ENV_VAR, "/from/env/GeoLite2-City.mmdb");
        let result = resolve_maxmind_path(None);
        assert_eq!(
            result,
            Some(PathBuf::from("/from/env/GeoLite2-City.mmdb"))
        );
        // Restore
        match prev {
            Some(v) => std::env::set_var(MAXMIND_ENV_VAR, v),
            None => std::env::remove_var(MAXMIND_ENV_VAR),
        }
    }

    #[test]
    fn os_default_used_as_fallback() {
        let prev = std::env::var(MAXMIND_ENV_VAR).ok();
        std::env::remove_var(MAXMIND_ENV_VAR);
        let result = resolve_maxmind_path(None);
        // Should resolve to the OS data dir
        if let Some(data_dir) = dirs::data_dir() {
            assert_eq!(
                result,
                Some(data_dir.join(MAXMIND_APP_DIR).join(MAXMIND_FILENAME))
            );
        }
        // Restore
        if let Some(v) = prev {
            std::env::set_var(MAXMIND_ENV_VAR, v);
        }
    }

    #[test]
    fn default_config_values() {
        let config = LocationConfig::default();
        assert!(config.maxmind_db_path.is_none());
        assert_eq!(config.gps_timeout, Duration::from_secs(10));
        assert_eq!(config.reverse.min_interval, Duration::from_secs(1));
        assert_eq!(config.reverse.timeout, Duration::from_secs(10));
    }

    #[test]
    fn reverse_config_default_endpoint() {
        let config = ReverseGeocodeConfig::default();
        assert_eq!(
            config.endpoint.as_str(),
            "https://nominatim.openstreetmap.org/"
        );
    }
}
```

- [ ] **Step 2: Wire config module into `lib.rs`**

Add to `biscuit-location/lib/src/lib.rs`:

```rust
mod config;

pub use config::{LocationConfig, ReverseGeocodeConfig, resolve_maxmind_path};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p biscuit-location`
Expected: All tests PASS.

- [ ] **Step 4: Commit configuration**

```bash
git add biscuit-location/lib/src/config.rs biscuit-location/lib/src/lib.rs
git commit -m "feat(biscuit-location): add configuration with MaxMind path resolution"
```

---

## Task 4: Google Maps URL Module

**Files:**
- Create: `biscuit-location/lib/src/maps.rs`
- Modify: `biscuit-location/lib/src/lib.rs`

- [ ] **Step 1: Write failing tests for URL generation**

Create `biscuit-location/lib/src/maps.rs`:

```rust
use url::Url;

use crate::types::Coordinates;

/// Generate a Google Maps URL that shows the given coordinates.
///
/// Uses the Maps search URL format which requires no API key:
/// `https://www.google.com/maps/search/?api=1&query={lat},{lon}`
pub fn google_maps_url(coords: &Coordinates) -> crate::Result<Url> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_url() {
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let url = google_maps_url(&coords).unwrap();
        assert_eq!(
            url.as_str(),
            "https://www.google.com/maps/search/?api=1&query=34.0522%2C-118.2437"
        );
    }

    #[test]
    fn url_contains_coordinates() {
        let coords = Coordinates::new(51.5074, -0.1278).unwrap();
        let url = google_maps_url(&coords).unwrap();
        let url_str = url.as_str();
        assert!(url_str.contains("51.5074"));
        assert!(url_str.contains("-0.1278"));
    }

    #[test]
    fn url_starts_with_google_maps() {
        let coords = Coordinates::new(0.0, 0.0).unwrap();
        let url = google_maps_url(&coords).unwrap();
        assert!(url.as_str().starts_with("https://www.google.com/maps/"));
    }
}
```

- [ ] **Step 2: Wire module and run tests to verify failure**

Add to `biscuit-location/lib/src/lib.rs`:

```rust
mod maps;

pub use maps::google_maps_url;
```

Run: `cargo test -p biscuit-location maps`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement `google_maps_url`**

Replace the `todo!()` in `maps.rs`:

```rust
pub fn google_maps_url(coords: &Coordinates) -> crate::Result<Url> {
    let mut url = Url::parse("https://www.google.com/maps/search/")
        .map_err(|e| crate::LocationError::GoogleMapsUrl(e.to_string()))?;
    url.query_pairs_mut()
        .append_pair("api", "1")
        .append_pair("query", &format!("{},{}", coords.latitude, coords.longitude));
    Ok(url)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p biscuit-location maps`
Expected: All tests PASS.

- [ ] **Step 5: Commit maps module**

```bash
git add biscuit-location/lib/src/maps.rs biscuit-location/lib/src/lib.rs
git commit -m "feat(biscuit-location): add Google Maps URL generation"
```

---

## Task 5: Distance Module

**Files:**
- Create: `biscuit-location/lib/src/distance.rs`
- Modify: `biscuit-location/lib/src/lib.rs`

- [ ] **Step 1: Write failing tests for distance calculation**

Create `biscuit-location/lib/src/distance.rs`:

```rust
use geo::algorithm::GeodesicDistance;
use geo::Point;

use crate::types::{Coordinates, Distance};

/// Compute the geodesic (ellipsoidal) distance between two coordinates.
///
/// Uses the Karney algorithm via the `geo` crate for high accuracy on WGS-84.
pub fn distance(from: &Coordinates, to: &Coordinates) -> crate::Result<Distance> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_point_is_zero() {
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let d = distance(&coords, &coords).unwrap();
        assert!(d.meters.abs() < 1.0);
    }

    #[test]
    fn la_to_new_york() {
        // LA to NYC is approximately 3,944 km
        let la = Coordinates::new(34.0522, -118.2437).unwrap();
        let nyc = Coordinates::new(40.7128, -74.0060).unwrap();
        let d = distance(&la, &nyc).unwrap();
        let km = d.meters / 1000.0;
        assert!(km > 3900.0 && km < 4000.0, "Expected ~3944 km, got {km}");
    }

    #[test]
    fn la_to_london() {
        // LA to London is approximately 8,757 km
        let la = Coordinates::new(34.0522, -118.2437).unwrap();
        let london = Coordinates::new(51.5074, -0.1278).unwrap();
        let d = distance(&la, &london).unwrap();
        let km = d.meters / 1000.0;
        assert!(km > 8700.0 && km < 8800.0, "Expected ~8757 km, got {km}");
    }

    #[test]
    fn distance_is_symmetric() {
        let a = Coordinates::new(34.0522, -118.2437).unwrap();
        let b = Coordinates::new(51.5074, -0.1278).unwrap();
        let d1 = distance(&a, &b).unwrap();
        let d2 = distance(&b, &a).unwrap();
        assert!((d1.meters - d2.meters).abs() < 1.0);
    }
}
```

- [ ] **Step 2: Wire module and run tests to verify failure**

Add to `biscuit-location/lib/src/lib.rs`:

```rust
mod distance;

pub use distance::distance;
```

Run: `cargo test -p biscuit-location distance`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement `distance`**

Replace the `todo!()` in `distance.rs`:

```rust
pub fn distance(from: &Coordinates, to: &Coordinates) -> crate::Result<Distance> {
    // geo::Point uses (x=longitude, y=latitude) convention
    let p1 = Point::new(from.longitude, from.latitude);
    let p2 = Point::new(to.longitude, to.latitude);
    let meters = p1.geodesic_distance(&p2);
    Ok(Distance { meters })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p biscuit-location distance`
Expected: All tests PASS.

- [ ] **Step 5: Commit distance module**

```bash
git add biscuit-location/lib/src/distance.rs biscuit-location/lib/src/lib.rs
git commit -m "feat(biscuit-location): add geodesic distance calculation"
```

---

## Task 6: IP Lookup Module

**Files:**
- Create: `biscuit-location/lib/src/ip.rs`
- Modify: `biscuit-location/lib/src/lib.rs`

- [ ] **Step 1: Write the IP lookup struct and record mapping tests**

Create `biscuit-location/lib/src/ip.rs`:

```rust
use std::net::IpAddr;
use std::path::Path;

use maxminddb::{geoip2, Reader};

use crate::error::LocationError;
use crate::types::{Coordinates, Location, LocationSource, Place};

/// Wraps a MaxMind GeoLite2-City reader for IP-to-location lookups.
pub struct IpLookup {
    reader: Reader<Vec<u8>>,
}

impl IpLookup {
    /// Open a MaxMind `.mmdb` file for reading.
    ///
    /// ## Errors
    ///
    /// Returns `DatabaseOpen` if the file cannot be read or parsed.
    pub fn open(path: &Path) -> crate::Result<Self> {
        let reader = Reader::open_readfile(path)
            .map_err(|e| LocationError::DatabaseOpen(format!("{path}: {e}", path = path.display())))?;
        Ok(Self { reader })
    }

    /// Look up an IP address and return a `Location` with city-level place data.
    ///
    /// ## Errors
    ///
    /// Returns `IpNotFound` if the database has no record for the address.
    /// Returns `IpLookup` for other database errors.
    pub fn lookup(&self, ip: IpAddr) -> crate::Result<Location> {
        let city: geoip2::City = self
            .reader
            .lookup(ip)
            .map_err(|e| match e {
                maxminddb::MaxMindDBError::AddressNotFoundError(_) => LocationError::IpNotFound(ip),
                other => LocationError::IpLookup(other.to_string()),
            })?;
        city_to_location(city, ip)
    }
}

/// Map a MaxMind City record into our domain `Location`.
///
/// Tolerates missing fields because GeoLite2 entries are often sparse.
fn city_to_location(city: geoip2::City, ip: IpAddr) -> crate::Result<Location> {
    let location = city.location.as_ref();
    let lat = location.and_then(|l| l.latitude);
    let lon = location.and_then(|l| l.longitude);

    let (latitude, longitude) = match (lat, lon) {
        (Some(lat), Some(lon)) => (lat, lon),
        _ => return Err(LocationError::IpNotFound(ip)),
    };

    let coordinates = Coordinates::new(latitude, longitude)?;

    let city_name = city
        .city
        .as_ref()
        .and_then(|c| c.names.as_ref())
        .and_then(|n| n.get("en"))
        .map(|s| s.to_string());

    let subdivision = city.subdivisions.as_ref().and_then(|s| s.first());

    let region = subdivision
        .and_then(|s| s.names.as_ref())
        .and_then(|n| n.get("en"))
        .map(|s| s.to_string());

    let region_code = subdivision
        .and_then(|s| s.iso_code)
        .map(|s| s.to_string());

    let country = city
        .country
        .as_ref()
        .and_then(|c| c.names.as_ref())
        .and_then(|n| n.get("en"))
        .map(|s| s.to_string());

    let country_code = city
        .country
        .as_ref()
        .and_then(|c| c.iso_code)
        .map(|s| s.to_string());

    let postal_code = city
        .postal
        .as_ref()
        .and_then(|p| p.code)
        .map(|s| s.to_string());

    let timezone = location
        .and_then(|l| l.time_zone)
        .map(|s| s.to_string());

    let place = Place {
        city: city_name,
        region,
        region_code,
        country,
        country_code,
        postal_code,
        timezone,
    };

    Ok(Location {
        coordinates,
        place: Some(place),
        source: LocationSource::Ip { ip },
        accuracy_meters: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test that requires a real MaxMind database.
    /// Set `BISCUIT_LOCATION_TEST_MMDB` to the path of a GeoLite2-City.mmdb file.
    #[test]
    fn lookup_real_db() {
        let db_path = match std::env::var("BISCUIT_LOCATION_TEST_MMDB") {
            Ok(p) => p,
            Err(_) => {
                eprintln!("Skipping: BISCUIT_LOCATION_TEST_MMDB not set");
                return;
            }
        };
        let lookup = IpLookup::open(Path::new(&db_path)).unwrap();

        // Google DNS — should resolve to a US location
        let loc = lookup.lookup("8.8.8.8".parse().unwrap()).unwrap();
        assert!(loc.place.is_some());
        let place = loc.place.unwrap();
        assert_eq!(place.country_code.as_deref(), Some("US"));
    }

    #[test]
    fn open_nonexistent_db() {
        let result = IpLookup::open(Path::new("/nonexistent/path.mmdb"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LocationError::DatabaseOpen(_)));
    }
}
```

- [ ] **Step 2: Wire module into `lib.rs`**

Add to `biscuit-location/lib/src/lib.rs`:

```rust
mod ip;
```

Note: `IpLookup` is not re-exported at the crate root — it's used internally by `LocationService`.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p biscuit-location ip`
Expected: `open_nonexistent_db` PASSES; `lookup_real_db` either passes (if env var set) or prints skip message.

- [ ] **Step 4: Commit IP lookup module**

```bash
git add biscuit-location/lib/src/ip.rs biscuit-location/lib/src/lib.rs
git commit -m "feat(biscuit-location): add MaxMind IP-to-location lookup"
```

---

## Task 7: Reverse Geocoding Module

**Files:**
- Create: `biscuit-location/lib/src/reverse.rs`
- Modify: `biscuit-location/lib/src/lib.rs`

- [ ] **Step 1: Write the reverse geocoder struct and Nominatim response types**

Create `biscuit-location/lib/src/reverse.rs`:

```rust
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::config::ReverseGeocodeConfig;
use crate::error::LocationError;
use crate::types::{Coordinates, Location, LocationSource, Place};

/// Reverse geocoder that calls a Nominatim-compatible API.
pub struct ReverseGeocoder {
    client: reqwest::Client,
    config: ReverseGeocodeConfig,
    last_request: Mutex<Option<Instant>>,
}

impl ReverseGeocoder {
    /// Create a new reverse geocoder with the given configuration.
    pub fn new(config: ReverseGeocodeConfig) -> crate::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(config.timeout)
            .build()
            .map_err(|e| LocationError::Internal(e.to_string()))?;
        Ok(Self {
            client,
            config,
            last_request: Mutex::new(None),
        })
    }

    /// Reverse geocode coordinates into a `Location` with place metadata.
    ///
    /// Enforces a minimum interval between requests to respect Nominatim rate limits.
    pub async fn reverse(&self, coords: &Coordinates) -> crate::Result<Location> {
        self.enforce_rate_limit().await;

        let url = format!(
            "{}reverse?lat={}&lon={}&format=json&addressdetails=1",
            self.config.endpoint, coords.latitude, coords.longitude
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LocationError::ReverseGeocode(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(LocationError::ReverseGeocode(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        let body: NominatimResponse = resp
            .json()
            .await
            .map_err(|e| LocationError::ReverseGeocode(e.to_string()))?;

        Ok(body.into_location(coords))
    }

    /// Sleep if the previous request was less than `min_interval` ago.
    async fn enforce_rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(prev) = *last {
            let elapsed = prev.elapsed();
            if elapsed < self.config.min_interval {
                tokio::time::sleep(self.config.min_interval - elapsed).await;
            }
        }
        *last = Some(Instant::now());
    }
}

/// Raw Nominatim JSON response.
#[derive(Debug, Deserialize)]
struct NominatimResponse {
    #[allow(dead_code)]
    display_name: Option<String>,
    address: Option<NominatimAddress>,
}

#[derive(Debug, Deserialize)]
struct NominatimAddress {
    city: Option<String>,
    town: Option<String>,
    village: Option<String>,
    state: Option<String>,
    #[serde(rename = "ISO3166-2-lvl4")]
    state_code: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
    postcode: Option<String>,
}

impl NominatimResponse {
    fn into_location(self, coords: &Coordinates) -> Location {
        let place = self.address.map(|addr| {
            // Nominatim uses city/town/village for different settlement sizes
            let city = addr.city.or(addr.town).or(addr.village);

            // Extract region code from ISO3166-2 format (e.g., "US-CA" -> "CA")
            let region_code = addr
                .state_code
                .as_deref()
                .and_then(|s| s.split('-').nth(1))
                .map(|s| s.to_string());

            Place {
                city,
                region: addr.state,
                region_code,
                country: addr.country,
                country_code: addr.country_code.map(|c| c.to_uppercase()),
                postal_code: addr.postcode,
                timezone: None, // Nominatim does not include timezone
            }
        });

        Location {
            coordinates: *coords,
            place,
            source: LocationSource::ReverseGeocode,
            accuracy_meters: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use url::Url;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config(endpoint: Url) -> ReverseGeocodeConfig {
        ReverseGeocodeConfig {
            endpoint,
            user_agent: "test/0.1.0".to_string(),
            timeout: Duration::from_secs(5),
            min_interval: Duration::from_millis(0), // No rate limit in tests
        }
    }

    const NOMINATIM_LA_RESPONSE: &str = r#"{
        "place_id": 282305978,
        "display_name": "Los Angeles, Los Angeles County, California, United States",
        "address": {
            "city": "Los Angeles",
            "county": "Los Angeles County",
            "state": "California",
            "ISO3166-2-lvl4": "US-CA",
            "country": "United States",
            "country_code": "us",
            "postcode": "90012"
        }
    }"#;

    #[tokio::test]
    async fn reverse_maps_nominatim_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .and(query_param("format", "json"))
            .and(query_param("addressdetails", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(NOMINATIM_LA_RESPONSE))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let location = geocoder.reverse(&coords).await.unwrap();

        assert_eq!(location.source, LocationSource::ReverseGeocode);
        let place = location.place.unwrap();
        assert_eq!(place.city.as_deref(), Some("Los Angeles"));
        assert_eq!(place.region.as_deref(), Some("California"));
        assert_eq!(place.region_code.as_deref(), Some("CA"));
        assert_eq!(place.country.as_deref(), Some("United States"));
        assert_eq!(place.country_code.as_deref(), Some("US"));
        assert_eq!(place.postal_code.as_deref(), Some("90012"));
    }

    #[tokio::test]
    async fn reverse_handles_town_instead_of_city() {
        let mock_server = MockServer::start().await;

        let response = r#"{
            "display_name": "Smallville, Kansas, United States",
            "address": {
                "town": "Smallville",
                "state": "Kansas",
                "country": "United States",
                "country_code": "us"
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(39.0, -97.0).unwrap();
        let location = geocoder.reverse(&coords).await.unwrap();

        let place = location.place.unwrap();
        assert_eq!(place.city.as_deref(), Some("Smallville"));
    }

    #[tokio::test]
    async fn reverse_handles_http_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(0.0, 0.0).unwrap();
        let result = geocoder.reverse(&coords).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LocationError::ReverseGeocode(_)));
    }

    #[tokio::test]
    async fn reverse_handles_empty_address() {
        let mock_server = MockServer::start().await;

        let response = r#"{"display_name": "Ocean"}"#;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(0.0, 0.0).unwrap();
        let location = geocoder.reverse(&coords).await.unwrap();

        assert!(location.place.is_none());
    }
}
```

- [ ] **Step 2: Wire module into `lib.rs`**

Add to `biscuit-location/lib/src/lib.rs`:

```rust
#[cfg(feature = "reverse")]
mod reverse;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p biscuit-location reverse`
Expected: All 4 tests PASS.

- [ ] **Step 4: Commit reverse geocoding module**

```bash
git add biscuit-location/lib/src/reverse.rs biscuit-location/lib/src/lib.rs
git commit -m "feat(biscuit-location): add Nominatim reverse geocoding with rate limiting"
```

---

## Task 8: GPS Module

**Files:**
- Create: `biscuit-location/lib/src/gps/mod.rs`
- Create: `biscuit-location/lib/src/gps/macos.rs`
- Create: `biscuit-location/lib/src/gps/windows.rs`
- Create: `biscuit-location/lib/src/gps/linux.rs`
- Modify: `biscuit-location/lib/src/lib.rs`

- [ ] **Step 1: Create the GPS facade (`gps/mod.rs`)**

Create `biscuit-location/lib/src/gps/mod.rs`:

```rust
use std::time::Duration;

use crate::types::{Coordinates, Location, LocationSource};

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

/// Request a one-shot GPS fix from the host device.
///
/// Returns `Ok(None)` when:
/// - Location services are disabled
/// - Permission is denied
/// - No fix is available before timeout
/// - The platform has no GPS provider
///
/// ## Errors
///
/// Returns `UnsupportedPlatform` on targets without a GPS backend.
pub async fn current_fix(timeout: Duration) -> crate::Result<Option<Location>> {
    #[cfg(target_os = "macos")]
    {
        macos::current_fix(timeout).await
    }
    #[cfg(target_os = "windows")]
    {
        windows::current_fix(timeout).await
    }
    #[cfg(target_os = "linux")]
    {
        linux::current_fix(timeout).await
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = timeout;
        Err(crate::LocationError::UnsupportedPlatform)
    }
}

/// Build a `Location` from raw GPS coordinates and optional accuracy.
fn gps_location(latitude: f64, longitude: f64, accuracy: Option<f64>) -> crate::Result<Location> {
    let coordinates = Coordinates::new(latitude, longitude)?;
    Ok(Location {
        coordinates,
        place: None,
        source: LocationSource::Gps,
        accuracy_meters: accuracy,
    })
}
```

- [ ] **Step 2: Create the macOS backend (`gps/macos.rs`)**

Create `biscuit-location/lib/src/gps/macos.rs`:

```rust
use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, AllocAnyThread, DeclaredClass, MainThreadMarker, msg_send};
use objc2_core_location::{
    CLAuthorizationStatus, CLLocation, CLLocationManager, CLLocationManagerDelegate,
};
use objc2_foundation::{NSArray, NSDate, NSError, NSObject, NSObjectProtocol, NSRunLoop};

use crate::types::Location;

/// One-shot location result: `(latitude, longitude, horizontal_accuracy)`.
type GpsFix = (f64, f64, Option<f64>);

struct DelegateIvars {
    sender: std::sync::Mutex<Option<mpsc::Sender<Option<GpsFix>>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "BiscuitLocationDelegate"]
    #[ivars = DelegateIvars]
    struct LocationDelegate;

    unsafe impl NSObjectProtocol for LocationDelegate {}

    unsafe impl CLLocationManagerDelegate for LocationDelegate {
        #[unsafe(method(locationManager:didUpdateLocations:))]
        fn did_update_locations(
            &self,
            _manager: &CLLocationManager,
            locations: &NSArray<CLLocation>,
        ) {
            if let Some(location) = locations.lastObject() {
                let coord = unsafe { location.coordinate() };
                let accuracy = unsafe { location.horizontalAccuracy() };
                let acc = if accuracy >= 0.0 {
                    Some(accuracy)
                } else {
                    None
                };
                if let Some(sender) = self.ivars().sender.lock().unwrap().take() {
                    let _ = sender.send(Some((coord.latitude, coord.longitude, acc)));
                }
            }
        }

        #[unsafe(method(locationManager:didFailWithError:))]
        fn did_fail_with_error(
            &self,
            _manager: &CLLocationManager,
            _error: &NSError,
        ) {
            if let Some(sender) = self.ivars().sender.lock().unwrap().take() {
                let _ = sender.send(None);
            }
        }
    }
);

impl LocationDelegate {
    fn new(sender: mpsc::Sender<Option<GpsFix>>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DelegateIvars {
            sender: std::sync::Mutex::new(Some(sender)),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub async fn current_fix(timeout: Duration) -> crate::Result<Option<Location>> {
    let (tx, rx) = mpsc::channel();

    // CoreLocation requires a run loop, so we run it on a dedicated thread.
    std::thread::spawn(move || {
        let manager = unsafe { CLLocationManager::new() };
        let delegate = LocationDelegate::new(tx);
        let delegate_proto: &ProtocolObject<dyn CLLocationManagerDelegate> =
            ProtocolObject::from_ref(&*delegate);
        unsafe { manager.setDelegate(Some(delegate_proto)) };

        // Request location — this triggers the permission prompt if needed.
        unsafe { manager.requestLocation() };

        // Pump the run loop until timeout. Callbacks fire during this loop.
        let deadline = unsafe { NSDate::dateWithTimeIntervalSinceNow(timeout.as_secs_f64()) };
        unsafe {
            NSRunLoop::currentRunLoop().runUntilDate(&deadline);
        }
    });

    // Wait for the thread's result
    match rx.recv_timeout(timeout + Duration::from_secs(1)) {
        Ok(Some((lat, lon, acc))) => super::gps_location(lat, lon, acc).map(Some),
        Ok(None) | Err(_) => Ok(None),
    }
}
```

**Note:** The exact `objc2`/`objc2-core-location` API surface may require minor adjustments based on the installed crate versions. The delegate pattern and run-loop approach are stable; specific macro syntax or method signatures may vary. Run `cargo doc -p objc2-core-location --open` to verify available APIs.

- [ ] **Step 3: Create the Windows stub (`gps/windows.rs`)**

Create `biscuit-location/lib/src/gps/windows.rs`:

```rust
use std::time::Duration;

use crate::types::Location;

/// Windows GPS backend using Windows.Devices.Geolocation.
///
/// TODO: Implement using the `windows` crate when Windows support is needed.
/// For now, returns `None` (no GPS fix available) on all Windows hosts.
pub async fn current_fix(_timeout: Duration) -> crate::Result<Option<Location>> {
    Ok(None)
}
```

- [ ] **Step 4: Create the Linux stub (`gps/linux.rs`)**

Create `biscuit-location/lib/src/gps/linux.rs`:

```rust
use std::time::Duration;

use crate::types::Location;

/// Linux GPS backend using GeoClue2 via D-Bus.
///
/// TODO: Implement using `zbus` when Linux support is needed.
/// For now, returns `None` (no GPS fix available) on all Linux hosts.
pub async fn current_fix(_timeout: Duration) -> crate::Result<Option<Location>> {
    Ok(None)
}
```

- [ ] **Step 5: Wire GPS module into `lib.rs`**

Add to `biscuit-location/lib/src/lib.rs`:

```rust
mod gps;
```

- [ ] **Step 6: Verify compilation**

Run: `cargo build -p biscuit-location`
Expected: Compiles without errors. GPS module is only testable manually — automated tests would require mocking OS services.

- [ ] **Step 7: Commit GPS module**

```bash
git add biscuit-location/lib/src/gps/mod.rs biscuit-location/lib/src/gps/macos.rs \
  biscuit-location/lib/src/gps/windows.rs biscuit-location/lib/src/gps/linux.rs \
  biscuit-location/lib/src/lib.rs
git commit -m "feat(biscuit-location): add GPS module with macOS CoreLocation backend"
```

---

## Task 9: LocationService

**Files:**
- Create: `biscuit-location/lib/src/service.rs`
- Modify: `biscuit-location/lib/src/lib.rs`

- [ ] **Step 1: Implement `LocationService`**

Create `biscuit-location/lib/src/service.rs`:

```rust
use std::net::IpAddr;

use url::Url;

use crate::config::{LocationConfig, resolve_maxmind_path};
use crate::distance;
use crate::error::LocationError;
use crate::gps;
use crate::ip::IpLookup;
use crate::maps;
use crate::types::{Coordinates, Distance, DistanceUnit, Location, LocationInput, LocationSource};

/// Configured facade for all location services.
///
/// Holds the MaxMind reader, reverse geocoder configuration, and GPS settings.
/// Construct via `LocationService::new()` and use the methods to perform lookups.
pub struct LocationService {
    ip_lookup: Option<IpLookup>,
    config: LocationConfig,
    #[cfg(feature = "reverse")]
    reverse_geocoder: Option<crate::reverse::ReverseGeocoder>,
}

impl LocationService {
    /// Create a new location service from configuration.
    ///
    /// If a MaxMind database path resolves successfully, the IP lookup reader
    /// is opened eagerly. If the path does not exist, IP lookups will return
    /// `DatabasePathNotFound` at call time.
    pub fn new(config: LocationConfig) -> crate::Result<Self> {
        let db_path = resolve_maxmind_path(config.maxmind_db_path.as_ref());
        let ip_lookup = match db_path {
            Some(ref p) if p.exists() => Some(IpLookup::open(p)?),
            _ => None,
        };

        #[cfg(feature = "reverse")]
        let reverse_geocoder = Some(crate::reverse::ReverseGeocoder::new(
            config.reverse.clone(),
        )?);

        Ok(Self {
            ip_lookup,
            config,
            #[cfg(feature = "reverse")]
            reverse_geocoder,
        })
    }

    /// Request a one-shot GPS fix from the host device.
    pub async fn gps(&self) -> crate::Result<Option<Location>> {
        gps::current_fix(self.config.gps_timeout).await
    }

    /// Look up the geographic location of an IP address.
    ///
    /// ## Errors
    ///
    /// Returns `DatabasePathNotFound` if no MaxMind database was configured or found.
    pub fn ip(&self, ip: IpAddr) -> crate::Result<Location> {
        match &self.ip_lookup {
            Some(lookup) => lookup.lookup(ip),
            None => Err(LocationError::DatabasePathNotFound(
                resolve_maxmind_path(self.config.maxmind_db_path.as_ref())
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "no path configured".to_string()),
            )),
        }
    }

    /// Reverse geocode coordinates into a location with place metadata.
    #[cfg(feature = "reverse")]
    pub async fn reverse(&self, coordinates: Coordinates) -> crate::Result<Location> {
        match &self.reverse_geocoder {
            Some(geocoder) => geocoder.reverse(&coordinates).await,
            None => Err(LocationError::Internal(
                "reverse geocoder not configured".to_string(),
            )),
        }
    }

    /// Calculate the distance between two coordinates.
    pub fn distance(
        &self,
        from: Coordinates,
        to: Coordinates,
        unit: DistanceUnit,
    ) -> crate::Result<f64> {
        let d = distance::distance(&from, &to)?;
        Ok(d.as_unit(unit))
    }

    /// Generate a Google Maps URL for the given coordinates.
    pub fn google_maps_url(&self, coordinates: Coordinates) -> crate::Result<Url> {
        maps::google_maps_url(&coordinates)
    }

    /// Resolve a `LocationInput` into a `Location`.
    ///
    /// This dispatches to GPS, IP lookup, or returns a literal coordinate location
    /// depending on the input variant.
    pub async fn resolve_input(&self, input: LocationInput) -> crate::Result<Location> {
        match input {
            LocationInput::Gps => self
                .gps()
                .await?
                .ok_or_else(|| LocationError::Internal("no GPS fix available".to_string())),
            LocationInput::Ip(ip) => self.ip(ip),
            LocationInput::Coordinates(coords) => Ok(Location {
                coordinates: coords,
                place: None,
                source: LocationSource::CoordinatesLiteral,
                accuracy_meters: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_without_maxmind_db() {
        let config = LocationConfig::default();
        let svc = LocationService::new(config).unwrap();
        let result = svc.ip("8.8.8.8".parse().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn service_distance() {
        let config = LocationConfig::default();
        let svc = LocationService::new(config).unwrap();
        let la = Coordinates::new(34.0522, -118.2437).unwrap();
        let nyc = Coordinates::new(40.7128, -74.0060).unwrap();
        let km = svc.distance(la, nyc, DistanceUnit::Kilometers).unwrap();
        assert!(km > 3900.0 && km < 4000.0);
    }

    #[test]
    fn service_google_maps_url() {
        let config = LocationConfig::default();
        let svc = LocationService::new(config).unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let url = svc.google_maps_url(coords).unwrap();
        assert!(url.as_str().contains("google.com/maps"));
    }

    #[tokio::test]
    async fn resolve_coordinate_literal() {
        let config = LocationConfig::default();
        let svc = LocationService::new(config).unwrap();
        let coords = Coordinates::new(51.5074, -0.1278).unwrap();
        let loc = svc
            .resolve_input(LocationInput::Coordinates(coords))
            .await
            .unwrap();
        assert_eq!(loc.source, LocationSource::CoordinatesLiteral);
        assert_eq!(loc.coordinates.latitude, 51.5074);
    }
}
```

- [ ] **Step 2: Update `lib.rs` with complete module list and re-exports**

Replace `biscuit-location/lib/src/lib.rs` entirely:

```rust
//! Location services for the biscuit ecosystem.
//!
//! Provides host GPS lookup, IP-to-location resolution, reverse geocoding,
//! distance calculation, and Google Maps link generation.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use biscuit_location::{LocationConfig, LocationService, Coordinates, DistanceUnit};
//!
//! # fn main() -> biscuit_location::Result<()> {
//! let svc = LocationService::new(LocationConfig::default())?;
//!
//! // Distance between two points
//! let la = Coordinates::new(34.0522, -118.2437)?;
//! let nyc = Coordinates::new(40.7128, -74.0060)?;
//! let km = svc.distance(la, nyc, DistanceUnit::Kilometers)?;
//! println!("{km:.1} km");
//!
//! // Google Maps link
//! let url = svc.google_maps_url(la)?;
//! println!("{url}");
//! # Ok(())
//! # }
//! ```

mod config;
mod distance;
mod error;
mod gps;
mod ip;
mod maps;
#[cfg(feature = "reverse")]
mod reverse;
mod service;
mod types;

pub use config::{LocationConfig, ReverseGeocodeConfig, resolve_maxmind_path};
pub use distance::distance;
pub use error::{LocationError, Result};
pub use maps::google_maps_url;
pub use service::LocationService;
pub use types::{
    Coordinates, Distance, DistanceUnit, Location, LocationInput, LocationSource, Place,
};
```

- [ ] **Step 3: Run all tests**

Run: `cargo test -p biscuit-location`
Expected: All tests PASS.

- [ ] **Step 4: Commit LocationService**

```bash
git add biscuit-location/lib/src/service.rs biscuit-location/lib/src/lib.rs
git commit -m "feat(biscuit-location): add LocationService facade"
```

---

## Task 10: CLI Implementation

**Files:**
- Create: `biscuit-location/cli/src/args.rs`
- Create: `biscuit-location/cli/src/commands.rs`
- Create: `biscuit-location/cli/src/output.rs`
- Modify: `biscuit-location/cli/src/main.rs`

- [ ] **Step 1: Create CLI argument definitions (`args.rs`)**

Create `biscuit-location/cli/src/args.rs`:

```rust
use std::net::IpAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueHint};

/// Location services: GPS, IP lookup, reverse geocoding, and distance.
#[derive(Parser)]
#[command(name = "where", version, about, long_about = None)]
#[command(after_help = AFTER_HELP)]
pub struct Cli {
    /// Override MaxMind database path
    #[arg(long, global = true, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub db_path: Option<PathBuf>,

    /// Include a Google Maps link in location output
    #[arg(long, global = true)]
    pub maps: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Get current location from host GPS
    Gps {
        /// GPS fix timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,
    },

    /// Look up geographic location of an IP address
    Ip {
        /// IPv4 or IPv6 address to look up
        address: IpAddr,
    },

    /// Reverse geocode coordinates to a place name
    Reverse {
        /// Latitude (-90 to 90)
        lat: f64,
        /// Longitude (-180 to 180)
        lon: f64,
    },

    /// Calculate distance between two locations
    Distance {
        /// First location: gps, ip:<addr>, or lat,lon
        from: String,
        /// Second location: gps, ip:<addr>, or lat,lon
        to: String,
        /// Distance unit
        #[arg(long, default_value = "kilometers", value_parser = parse_unit)]
        unit: biscuit_location::DistanceUnit,
    },
}

fn parse_unit(s: &str) -> Result<biscuit_location::DistanceUnit, String> {
    match s.to_lowercase().as_str() {
        "m" | "meters" => Ok(biscuit_location::DistanceUnit::Meters),
        "km" | "kilometers" => Ok(biscuit_location::DistanceUnit::Kilometers),
        "mi" | "miles" => Ok(biscuit_location::DistanceUnit::Miles),
        "nm" | "nautical" | "nauticalmiles" => Ok(biscuit_location::DistanceUnit::NauticalMiles),
        _ => Err(format!(
            "unknown unit '{s}': expected meters, kilometers, miles, or nauticalmiles"
        )),
    }
}

const AFTER_HELP: &str = "\
EXAMPLES:
  where gps                              Get location from GPS
  where ip 8.8.8.8                       Look up Google DNS location
  where reverse 34.0522 -118.2437        Reverse geocode LA coordinates
  where distance 34.05,-118.24 40.71,-74.01   LA to NYC distance
  where distance gps ip:8.8.8.8         GPS to IP-based location
  where ip 8.8.8.8 --maps               Include Google Maps link
";
```

- [ ] **Step 2: Create output formatting (`output.rs`)**

Create `biscuit-location/cli/src/output.rs`:

```rust
use biscuit_location::{Coordinates, DistanceUnit, Location};

/// Format a location for human-readable output.
pub fn format_location(location: &Location, maps_url: Option<&str>) -> String {
    let mut lines = Vec::new();

    // Line 1: best available place string, else coordinates
    match &location.place {
        Some(place) if !place.to_string().is_empty() => {
            lines.push(place.to_string());
        }
        _ => {}
    }

    // Line 2: raw coordinates
    lines.push(format!("{}", location.coordinates));

    // Line 3: source
    let source = match &location.source {
        biscuit_location::LocationSource::Gps => "GPS".to_string(),
        biscuit_location::LocationSource::Ip { ip } => format!("IP: {ip}"),
        biscuit_location::LocationSource::ReverseGeocode => "Reverse geocode".to_string(),
        biscuit_location::LocationSource::CoordinatesLiteral => "Coordinates".to_string(),
    };
    lines.push(format!("Source: {source}"));

    // Optional accuracy
    if let Some(acc) = location.accuracy_meters {
        lines.push(format!("Accuracy: {acc:.0} m"));
    }

    // Optional maps link
    if let Some(url) = maps_url {
        lines.push(format!("Maps: {url}"));
    }

    lines.join("\n")
}

/// Format a distance value for human-readable output.
pub fn format_distance(value: f64, unit: DistanceUnit) -> String {
    let unit_str = match unit {
        DistanceUnit::Meters => "meters",
        DistanceUnit::Kilometers => "km",
        DistanceUnit::Miles => "miles",
        DistanceUnit::NauticalMiles => "nautical miles",
    };
    format!("{value:.2} {unit_str}")
}

/// Format a "no GPS fix" message.
pub fn format_no_gps() -> &'static str {
    "No GPS fix available."
}
```

- [ ] **Step 3: Create command dispatch (`commands.rs`)**

Create `biscuit-location/cli/src/commands.rs`:

```rust
use std::time::Duration;

use biscuit_location::{
    Coordinates, LocationConfig, LocationInput, LocationService,
};

use crate::args::{Cli, Commands};
use crate::output;

/// Execute the CLI command and print output.
pub async fn run(cli: Cli) -> color_eyre::Result<()> {
    let config = LocationConfig {
        maxmind_db_path: cli.db_path.clone(),
        ..LocationConfig::default()
    };

    let svc = LocationService::new(config)?;

    match cli.command {
        Commands::Gps { timeout } => {
            let config = LocationConfig {
                gps_timeout: Duration::from_secs(timeout),
                maxmind_db_path: cli.db_path,
                ..LocationConfig::default()
            };
            let svc = LocationService::new(config)?;
            match svc.gps().await? {
                Some(location) => {
                    let maps_url = if cli.maps {
                        Some(svc.google_maps_url(location.coordinates)?.to_string())
                    } else {
                        None
                    };
                    println!(
                        "{}",
                        output::format_location(&location, maps_url.as_deref())
                    );
                }
                None => {
                    println!("{}", output::format_no_gps());
                }
            }
        }

        Commands::Ip { address } => {
            let location = svc.ip(address)?;
            let maps_url = if cli.maps {
                Some(svc.google_maps_url(location.coordinates)?.to_string())
            } else {
                None
            };
            println!(
                "{}",
                output::format_location(&location, maps_url.as_deref())
            );
        }

        Commands::Reverse { lat, lon } => {
            let coords = Coordinates::new(lat, lon)?;
            let location = svc.reverse(coords).await?;
            let maps_url = if cli.maps {
                Some(svc.google_maps_url(location.coordinates)?.to_string())
            } else {
                None
            };
            println!(
                "{}",
                output::format_location(&location, maps_url.as_deref())
            );
        }

        Commands::Distance { from, to, unit } => {
            let from_input: LocationInput = from.parse()?;
            let to_input: LocationInput = to.parse()?;
            let from_loc = svc.resolve_input(from_input).await?;
            let to_loc = svc.resolve_input(to_input).await?;
            let value = svc.distance(from_loc.coordinates, to_loc.coordinates, unit)?;
            println!("{}", output::format_distance(value, unit));
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Update `main.rs` entry point**

Replace `biscuit-location/cli/src/main.rs`:

```rust
//! CLI for location services: GPS, IP lookup, reverse geocoding, and distance.
//!
//! ## Usage
//!
//! ```bash
//! where gps                              # Get location from GPS
//! where ip 8.8.8.8                       # Look up IP location
//! where reverse 34.0522 -118.2437        # Reverse geocode coordinates
//! where distance 34.05,-118.24 40.71,-74.01  # Distance between points
//! ```

mod args;
mod commands;
mod output;

use clap::Parser;

use args::Cli;

#[tokio::main]
async fn main() {
    color_eyre::install().ok();

    let cli = Cli::parse();

    if let Err(err) = commands::run(cli).await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 5: Verify the CLI compiles and shows help**

Run: `cargo build -p biscuit-location-cli`
Expected: Compiles without errors.

Run: `cargo run -p biscuit-location-cli -- --help`
Expected: Shows help text with subcommands (gps, ip, reverse, distance).

- [ ] **Step 6: Commit CLI implementation**

```bash
git add biscuit-location/cli/src/args.rs biscuit-location/cli/src/commands.rs \
  biscuit-location/cli/src/output.rs biscuit-location/cli/src/main.rs
git commit -m "feat(biscuit-location): add where CLI with gps, ip, reverse, distance commands"
```

---

## Task 11: CLI Integration Tests

**Files:**
- Create: `biscuit-location/cli/tests/cli_tests.rs`

- [ ] **Step 1: Write CLI integration tests**

Create `biscuit-location/cli/tests/cli_tests.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("where").unwrap()
}

#[test]
fn shows_help() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("gps"))
        .stdout(predicate::str::contains("ip"))
        .stdout(predicate::str::contains("reverse"))
        .stdout(predicate::str::contains("distance"));
}

#[test]
fn shows_version() {
    cmd().arg("--version").assert().success();
}

#[test]
fn distance_between_coordinates() {
    cmd()
        .args(["distance", "34.0522,-118.2437", "40.7128,-74.0060"])
        .assert()
        .success()
        .stdout(predicate::str::contains("km"));
}

#[test]
fn distance_with_miles() {
    cmd()
        .args([
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
            "--unit",
            "miles",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("miles"));
}

#[test]
fn invalid_coordinates_rejected() {
    cmd()
        .args(["reverse", "999", "0"])
        .assert()
        .failure();
}

#[test]
fn invalid_ip_rejected() {
    cmd()
        .args(["ip", "not-an-ip"])
        .assert()
        .failure();
}

#[test]
fn distance_invalid_input() {
    cmd()
        .args(["distance", "not-a-location", "40.71,-74.01"])
        .assert()
        .failure();
}

#[test]
fn no_subcommand_shows_help() {
    cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}
```

- [ ] **Step 2: Run CLI tests**

Run: `cargo test -p biscuit-location-cli`
Expected: All tests PASS.

- [ ] **Step 3: Commit CLI tests**

```bash
git add biscuit-location/cli/tests/cli_tests.rs
git commit -m "test(biscuit-location): add CLI integration tests"
```

---

## Task 12: Run All Tests and Lint

- [ ] **Step 1: Run all library tests**

Run: `cargo test -p biscuit-location --all-features`
Expected: All tests PASS.

- [ ] **Step 2: Run all CLI tests**

Run: `cargo test -p biscuit-location-cli`
Expected: All tests PASS.

- [ ] **Step 3: Run clippy on both packages**

Run: `cargo clippy -p biscuit-location -p biscuit-location-cli -- -D warnings`
Expected: No warnings. If there are warnings, fix them before proceeding.

- [ ] **Step 4: Run the justfile recipes**

Run from `biscuit-location/`:

```bash
cd biscuit-location && just test && just lint
```

Expected: Both recipes pass.

- [ ] **Step 5: Fix any issues and commit**

If any fixes were needed:

```bash
git add -A biscuit-location/
git commit -m "fix(biscuit-location): address clippy warnings and test fixes"
```

---

## Task 13: Documentation

**Files:**
- Modify: `biscuit-location/README.md`
- Modify: `biscuit-location/lib/README.md`
- Modify: `biscuit-location/cli/README.md`
- Create: `biscuit-location/docs/dependencies.md`

- [ ] **Step 1: Write `biscuit-location/lib/README.md`**

```markdown
# biscuit-location

Location services library for the biscuit ecosystem.

## Features

- **Host GPS** — one-shot GPS fix from macOS CoreLocation (Windows/Linux stubs)
- **IP to Location** — resolve IP addresses via local MaxMind GeoLite2 database
- **Reverse Geocoding** — coordinates to city-level place name via Nominatim
- **Distance** — geodesic distance between two points (WGS-84 ellipsoid)
- **Google Maps Link** — generate a Maps URL for any coordinates

## Usage

```rust
use biscuit_location::{LocationConfig, LocationService, Coordinates, DistanceUnit};

let svc = LocationService::new(LocationConfig::default())?;

// Distance
let la = Coordinates::new(34.0522, -118.2437)?;
let nyc = Coordinates::new(40.7128, -74.0060)?;
let km = svc.distance(la, nyc, DistanceUnit::Kilometers)?;

// Google Maps
let url = svc.google_maps_url(la)?;
```

## MaxMind Database

IP lookup requires a GeoLite2-City database file (not bundled):

1. Create a free account at [maxmind.com](https://www.maxmind.com)
2. Download `GeoLite2-City.mmdb`
3. Place it at the OS default path or set `BISCUIT_LOCATION_MAXMIND_DB`

### Path Resolution

1. Explicit `LocationConfig.maxmind_db_path`
2. `BISCUIT_LOCATION_MAXMIND_DB` environment variable
3. OS default:
   - macOS: `~/Library/Application Support/biscuit-location/GeoLite2-City.mmdb`
   - Linux: `~/.local/share/biscuit-location/GeoLite2-City.mmdb`
   - Windows: `%LOCALAPPDATA%\biscuit-location\GeoLite2-City.mmdb`
```

- [ ] **Step 2: Write `biscuit-location/cli/README.md`**

```markdown
# biscuit-location-cli

CLI for location services. Installed as the `where` binary.

## Commands

```bash
where gps                                    # GPS fix from host
where ip 8.8.8.8                             # IP to location
where reverse 34.0522 -118.2437              # Coordinates to place
where distance 34.05,-118.24 40.71,-74.01    # Distance between points
```

## Flags

- `--maps` — include a Google Maps link in output
- `--db-path <PATH>` — override MaxMind database path
- `--unit <UNIT>` — distance unit: meters, kilometers (default), miles, nauticalmiles

## Distance Inputs

The `distance` command accepts three input forms:

- `gps` — resolve via host GPS
- `ip:<address>` — resolve via IP lookup
- `<lat>,<lon>` — literal coordinates

```bash
where distance gps 37.7749,-122.4194
where distance ip:8.8.8.8 ip:1.1.1.1
where distance 51.5074,-0.1278 48.8566,2.3522
```
```

- [ ] **Step 3: Update `biscuit-location/README.md`**

Update the existing README with a concise overview linking to the sub-crate READMEs.

- [ ] **Step 4: Create `biscuit-location/docs/dependencies.md`**

```markdown
# biscuit-location Dependencies

## Library (`biscuit-location`)

| Crate | Version | Purpose |
|-------|---------|---------|
| [dirs](https://crates.io/crates/dirs) | 6 | OS-specific data directory resolution |
| [geo](https://crates.io/crates/geo) | 0.29 | Geodesic distance calculation (WGS-84) |
| [maxminddb](https://crates.io/crates/maxminddb) | 0.27 | MaxMind GeoLite2/GeoIP2 database reader |
| [reqwest](https://crates.io/crates/reqwest) | 0.12 | HTTP client for Nominatim reverse geocoding |
| [serde](https://crates.io/crates/serde) | 1 | Serialization/deserialization |
| [serde_json](https://crates.io/crates/serde_json) | 1 | JSON parsing for Nominatim responses |
| [thiserror](https://crates.io/crates/thiserror) | 2 | Error type derivation |
| [tokio](https://crates.io/crates/tokio) | 1 | Async runtime (GPS, reverse geocoding) |
| [url](https://crates.io/crates/url) | 2 | URL construction for Google Maps |

### Platform-Specific (macOS)

| Crate | Version | Purpose |
|-------|---------|---------|
| [block2](https://crates.io/crates/block2) | 0.6 | Objective-C block support |
| [objc2](https://crates.io/crates/objc2) | 0.6 | Objective-C runtime bindings |
| [objc2-core-location](https://crates.io/crates/objc2-core-location) | 0.3 | CoreLocation GPS bindings |
| [objc2-foundation](https://crates.io/crates/objc2-foundation) | 0.3 | Foundation framework types |

### Dev Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [wiremock](https://crates.io/crates/wiremock) | 0.6 | HTTP mocking for reverse geocoding tests |

## CLI (`biscuit-location-cli`)

| Crate | Version | Purpose |
|-------|---------|---------|
| [clap](https://crates.io/crates/clap) | 4.5 | CLI argument parsing |
| [color-eyre](https://crates.io/crates/color-eyre) | 0.6 | Error reporting |
| [tokio](https://crates.io/crates/tokio) | 1 | Async runtime |

### Dev Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [assert_cmd](https://crates.io/crates/assert_cmd) | 2 | CLI integration testing |
| [predicates](https://crates.io/crates/predicates) | 3 | Test assertion matchers |
```

- [ ] **Step 5: Commit documentation**

```bash
git add biscuit-location/README.md biscuit-location/lib/README.md \
  biscuit-location/cli/README.md biscuit-location/docs/dependencies.md
git commit -m "docs(biscuit-location): add READMEs and dependency documentation"
```
