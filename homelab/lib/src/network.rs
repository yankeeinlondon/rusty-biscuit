use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};
use thiserror::Error;

/// Network host identifier - can be IPv4, IPv6, or DNS name.
#[derive(Debug, Clone)]
pub enum Host {
    /// Using an IPv4 Address
    V4(Ipv4Addr),
    /// Using an IPv6 Address
    V6(Ipv6Addr),
    /// Using a DNS Address
    Dns(String),
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Host::V4(ip) => write!(f, "{ip}"),
            Host::V6(ip) => write!(f, "[{ip}]"), // IPv6 needs brackets in URLs
            Host::Dns(name) => write!(f, "{name}"),
        }
    }
}

/// Error parsing a host string
#[derive(Debug, Error)]
#[error("invalid host: {0}")]
pub struct ParseHostError(pub String);

/// Parse a host string into a Host enum.
///
/// Supports:
/// - IPv4: "192.168.1.100"
/// - IPv6: "::1" or "[::1]"
/// - DNS: "device.local" or "host.example.com"
pub fn parse_host(host_str: &str) -> Result<Host, ParseHostError> {
    // Try IPv4 first
    if let Ok(ipv4) = host_str.parse::<Ipv4Addr>() {
        return Ok(Host::V4(ipv4));
    }

    // Try IPv6 (strip brackets if present)
    let ipv6_str = host_str.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ipv6) = ipv6_str.parse::<Ipv6Addr>() {
        return Ok(Host::V6(ipv6));
    }

    // Must be DNS
    if host_str.is_empty() {
        return Err(ParseHostError("empty hostname".to_string()));
    }

    Ok(Host::Dns(host_str.to_string()))
}
