# biscuit-location Dependencies

## Library (`biscuit-location`)

| Crate | Version | Purpose |
|-------|---------|---------|
| [dirs](https://crates.io/crates/dirs) | 6 | OS-specific data directory resolution |
| [geo](https://crates.io/crates/geo) | 0.29 | Geodesic distance calculation (WGS-84) |
| [maxminddb](https://crates.io/crates/maxminddb) | 0.27 | MaxMind GeoLite2/GeoIP2 database reader (mmap feature) |
| [reqwest](https://crates.io/crates/reqwest) | 0.12 | HTTP client for Nominatim reverse geocoding (rustls-tls + json) |
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

### Platform-Specific (Windows)

| Crate | Version | Purpose |
|-------|---------|---------|
| [windows](https://crates.io/crates/windows) | 0.62 | `Windows.Devices.Geolocation` bindings |

### Platform-Specific (Linux)

| Crate | Version | Purpose |
|-------|---------|---------|
| [zbus](https://crates.io/crates/zbus) | 5 | D-Bus client for GeoClue2 |
| [futures-util](https://crates.io/crates/futures-util) | 0.3 | `StreamExt` for D-Bus signal subscription |

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
