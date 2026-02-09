use homelab::{network::Host, sony_receiver::SonyReceiver};
use std::{net::Ipv4Addr, sync::Arc, time::Duration};

/// Default port for Sony receivers
const SONY_DEFAULT_PORT: u16 = 10000;

/// Default port for Arcam amplifiers
pub const ARCAM_DEFAULT_PORT: u16 = 50000;

/// Default request timeout in milliseconds
const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Sony receiver client (pre-configured, reusable)
    pub sony: Option<Arc<SonyReceiver>>,
    /// Arcam host (we create connections per-request)
    pub arcam_host: Option<String>,
    /// Request timeout for device operations
    pub request_timeout: Duration,
}

impl AppState {
    /// Creates application state from environment variables.
    ///
    /// ## Environment Variables
    ///
    /// - `SONY_RECEIVER` - Sony receiver hostname or IP (optional port: `host:port`, default port 10000)
    /// - `ARCAM_AMP` - Arcam amplifier hostname or IP (port always 50000)
    /// - `REQUEST_TIMEOUT_MS` - Request timeout in milliseconds (default 5000)
    ///
    /// ## Errors
    ///
    /// Returns an error if host parsing fails for configured devices.
    pub fn from_env() -> Result<Self, ConfigError> {
        let sony = match std::env::var("SONY_RECEIVER") {
            Ok(host_str) => Some(Arc::new(parse_sony_host(&host_str)?)),
            Err(_) => None,
        };

        let arcam_host = std::env::var("ARCAM_AMP").ok();

        let request_timeout = std::env::var("REQUEST_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_TIMEOUT_MS));

        Ok(Self {
            sony,
            arcam_host,
            request_timeout,
        })
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid host format: {0}")]
    InvalidHost(String),
}

/// Parse a Sony receiver host string (host or host:port)
fn parse_sony_host(host_str: &str) -> Result<SonyReceiver, ConfigError> {
    let (host, port) = if let Some(idx) = host_str.rfind(':') {
        // Check if this is IPv6 (contains multiple colons)
        if host_str.matches(':').count() > 1 && !host_str.contains('[') {
            // Plain IPv6 without port
            (host_str, SONY_DEFAULT_PORT)
        } else if host_str.starts_with('[') {
            // IPv6 with brackets: [::1]:port
            if let Some(bracket_idx) = host_str.find("]:") {
                let ip_part = &host_str[1..bracket_idx];
                let port_str = &host_str[bracket_idx + 2..];
                let port = port_str
                    .parse()
                    .map_err(|_| ConfigError::InvalidHost(format!("invalid port: {}", port_str)))?;
                (ip_part, port)
            } else {
                // Just [::1] without port
                let ip_part = host_str.trim_start_matches('[').trim_end_matches(']');
                (ip_part, SONY_DEFAULT_PORT)
            }
        } else {
            // host:port format
            let port_str = &host_str[idx + 1..];
            let port = port_str
                .parse()
                .map_err(|_| ConfigError::InvalidHost(format!("invalid port: {}", port_str)))?;
            (&host_str[..idx], port)
        }
    } else {
        (host_str, SONY_DEFAULT_PORT)
    };

    let host = parse_host(host)?;
    Ok(SonyReceiver::new(host, port))
}

/// Parse a host string into a Host enum
fn parse_host(host_str: &str) -> Result<Host, ConfigError> {
    // Try IPv4 first
    if let Ok(ipv4) = host_str.parse::<Ipv4Addr>() {
        return Ok(Host::V4(ipv4));
    }

    // Try IPv6
    if let Ok(ipv6) = host_str.parse() {
        return Ok(Host::V6(ipv6));
    }

    // Must be DNS
    if host_str.is_empty() {
        return Err(ConfigError::InvalidHost("empty hostname".to_string()));
    }

    Ok(Host::Dns(host_str.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sony_host_simple() {
        let receiver = parse_sony_host("192.168.1.100").unwrap();
        assert_eq!(receiver.port(), SONY_DEFAULT_PORT);
    }

    #[test]
    fn test_parse_sony_host_with_port() {
        let receiver = parse_sony_host("192.168.1.100:8080").unwrap();
        assert_eq!(receiver.port(), 8080);
    }

    #[test]
    fn test_parse_sony_host_dns() {
        let receiver = parse_sony_host("receiver.local").unwrap();
        assert_eq!(receiver.port(), SONY_DEFAULT_PORT);
    }

    #[test]
    fn test_parse_sony_host_dns_with_port() {
        let receiver = parse_sony_host("receiver.local:9000").unwrap();
        assert_eq!(receiver.port(), 9000);
    }

    #[test]
    fn test_parse_host_ipv4() {
        let host = parse_host("192.168.1.1").unwrap();
        assert!(matches!(host, Host::V4(_)));
    }

    #[test]
    fn test_parse_host_ipv6() {
        let host = parse_host("::1").unwrap();
        assert!(matches!(host, Host::V6(_)));
    }

    #[test]
    fn test_parse_host_dns() {
        let host = parse_host("mydevice.local").unwrap();
        assert!(matches!(host, Host::Dns(_)));
    }

    #[test]
    fn test_parse_host_empty_fails() {
        let result = parse_host("");
        assert!(result.is_err());
    }

    #[test]
    fn test_app_state_defaults() {
        // Clear env vars for this test
        // SAFETY: This is a single-threaded test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::remove_var("REQUEST_TIMEOUT_MS");
        }

        let state = AppState::from_env().unwrap();
        assert!(state.sony.is_none());
        assert!(state.arcam_host.is_none());
        assert_eq!(state.request_timeout, Duration::from_millis(5000));
    }
}
