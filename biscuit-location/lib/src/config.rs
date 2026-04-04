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
        let prev = std::env::var(MAXMIND_ENV_VAR).ok();
        unsafe { std::env::set_var(MAXMIND_ENV_VAR, "/from/env/GeoLite2-City.mmdb") };
        let result = resolve_maxmind_path(None);
        assert_eq!(
            result,
            Some(PathBuf::from("/from/env/GeoLite2-City.mmdb"))
        );
        match prev {
            Some(v) => unsafe { std::env::set_var(MAXMIND_ENV_VAR, v) },
            None => unsafe { std::env::remove_var(MAXMIND_ENV_VAR) },
        }
    }

    #[test]
    fn os_default_used_as_fallback() {
        let prev = std::env::var(MAXMIND_ENV_VAR).ok();
        unsafe { std::env::remove_var(MAXMIND_ENV_VAR) };
        let result = resolve_maxmind_path(None);
        if let Some(data_dir) = dirs::data_dir() {
            assert_eq!(
                result,
                Some(data_dir.join(MAXMIND_APP_DIR).join(MAXMIND_FILENAME))
            );
        }
        if let Some(v) = prev {
            unsafe { std::env::set_var(MAXMIND_ENV_VAR, v) };
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
