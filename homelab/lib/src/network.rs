use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

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
