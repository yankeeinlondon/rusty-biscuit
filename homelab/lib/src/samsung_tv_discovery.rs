//! Samsung Smart TV SSDP discovery.
//!
//! Discovers Samsung TVs on the local network using UPnP/SSDP M-SEARCH
//! multicast queries. Sends to `239.255.255.250:1900` and filters responses
//! for Samsung remote control receiver devices.

use std::net::UdpSocket;
use std::time::Duration;

/// A Samsung TV discovered via SSDP on the local network.
#[derive(Debug, Clone)]
pub struct DiscoveredSamsungTv {
    /// IP address of the TV.
    pub host: String,
    /// SSDP LOCATION header URL (device description XML).
    pub location: String,
    /// SSDP SERVER header (firmware/platform identifier).
    pub server: Option<String>,
    /// SSDP USN (Unique Service Name).
    pub usn: Option<String>,
}

/// SSDP discovery errors.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// UDP socket error
    #[error("SSDP socket error: {0}")]
    Io(#[from] std::io::Error),
}

/// Discover Samsung Smart TVs on the local network via SSDP.
///
/// Sends M-SEARCH requests and listens for responses for up to `timeout_secs`.
/// Tries Samsung-specific search target first, then falls back to `ssdp:all`
/// and filters by Samsung server strings.
pub fn discover(timeout_secs: u64) -> Result<Vec<DiscoveredSamsungTv>, DiscoveryError> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(timeout_secs)))?;

    let multicast_addr = "239.255.255.250:1900";

    // Samsung-specific search target
    let samsung_st = "urn:samsung.com:device:RemoteControlReceiver:1";
    let m_search_samsung = format!(
        "M-SEARCH * HTTP/1.1\r\n\
         HOST: 239.255.255.250:1900\r\n\
         MAN: \"ssdp:discover\"\r\n\
         MX: 3\r\n\
         ST: {samsung_st}\r\n\
         \r\n"
    );

    // Fallback: discover all and filter
    let m_search_all = "M-SEARCH * HTTP/1.1\r\n\
         HOST: 239.255.255.250:1900\r\n\
         MAN: \"ssdp:discover\"\r\n\
         MX: 3\r\n\
         ST: ssdp:all\r\n\
         \r\n";

    // Send both queries
    socket.send_to(m_search_samsung.as_bytes(), multicast_addr)?;
    socket.send_to(m_search_all.as_bytes(), multicast_addr)?;

    let mut results = Vec::new();
    let mut seen_hosts = std::collections::HashSet::new();
    let mut buf = [0u8; 4096];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, addr)) => {
                let response = String::from_utf8_lossy(&buf[..len]);
                if let Some(tv) = parse_ssdp_response(&response, &addr.ip().to_string())
                    && seen_hosts.insert(tv.host.clone())
                {
                    results.push(tv);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(DiscoveryError::Io(e)),
        }
    }

    Ok(results)
}

/// Parse an SSDP response into a discovered Samsung TV.
///
/// Returns `Some` if the response appears to be from a Samsung TV,
/// based on the LOCATION, SERVER, or ST headers.
pub fn parse_ssdp_response(response: &str, source_ip: &str) -> Option<DiscoveredSamsungTv> {
    let lower = response.to_lowercase();

    // Must contain Samsung indicators
    let is_samsung = lower.contains("samsung")
        || lower.contains("remotecontrolreceiver")
        || lower.contains("dialserver");

    if !is_samsung {
        return None;
    }

    let location = extract_header(response, "LOCATION")?;
    let server = extract_header(response, "SERVER");
    let usn = extract_header(response, "USN");

    // Extract host from LOCATION URL or fall back to source IP
    let host = extract_host_from_url(&location).unwrap_or_else(|| source_ip.to_string());

    Some(DiscoveredSamsungTv {
        host,
        location,
        server,
        usn,
    })
}

/// Extract a header value from an SSDP response.
fn extract_header(response: &str, header: &str) -> Option<String> {
    let header_lower = header.to_lowercase();
    for line in response.lines() {
        let line_lower = line.to_lowercase();
        if let Some(rest) = line_lower.strip_prefix(&header_lower)
            && rest.starts_with(':')
        {
            let original_value = &line[header.len() + 1..];
            return Some(original_value.trim().to_string());
        }
    }
    None
}

/// Extract the host from a URL like `http://192.168.1.42:8001/api/v2/`.
fn extract_host_from_url(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let host_port = without_scheme.split('/').next()?;
    let host = host_port.split(':').next()?;
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_samsung_ssdp_response() {
        let response = "\
HTTP/1.1 200 OK\r\n\
CACHE-CONTROL: max-age=1800\r\n\
LOCATION: http://192.168.1.42:8001/api/v2/\r\n\
SERVER: Samsung/Tizen UPnP/1.0\r\n\
ST: urn:samsung.com:device:RemoteControlReceiver:1\r\n\
USN: uuid:12345678-1234-1234-1234-123456789012\r\n\
\r\n";

        let tv = parse_ssdp_response(response, "192.168.1.42").unwrap();
        assert_eq!(tv.host, "192.168.1.42");
        assert_eq!(tv.location, "http://192.168.1.42:8001/api/v2/");
        assert_eq!(tv.server.as_deref(), Some("Samsung/Tizen UPnP/1.0"));
        assert_eq!(
            tv.usn.as_deref(),
            Some("uuid:12345678-1234-1234-1234-123456789012")
        );
    }

    #[test]
    fn parse_non_samsung_response_returns_none() {
        let response = "\
HTTP/1.1 200 OK\r\n\
CACHE-CONTROL: max-age=1800\r\n\
LOCATION: http://192.168.1.1:80/description.xml\r\n\
SERVER: Linux/3.10 UPnP/1.0\r\n\
ST: upnp:rootdevice\r\n\
\r\n";

        assert!(parse_ssdp_response(response, "192.168.1.1").is_none());
    }

    #[test]
    fn parse_samsung_dial_response() {
        let response = "\
HTTP/1.1 200 OK\r\n\
LOCATION: http://192.168.1.50:8001/api/v2/\r\n\
SERVER: Samsung DIALServer/1.0\r\n\
ST: urn:dial-multiscreen-org:device:dialreceiver:1\r\n\
\r\n";

        let tv = parse_ssdp_response(response, "192.168.1.50").unwrap();
        assert_eq!(tv.host, "192.168.1.50");
        assert!(tv.server.unwrap().contains("Samsung"));
    }

    #[test]
    fn extract_host_from_various_urls() {
        assert_eq!(
            extract_host_from_url("http://192.168.1.42:8001/api/v2/"),
            Some("192.168.1.42".to_string())
        );
        assert_eq!(
            extract_host_from_url("https://tv.local:8002/"),
            Some("tv.local".to_string())
        );
        assert_eq!(
            extract_host_from_url("http://10.0.0.1/description.xml"),
            Some("10.0.0.1".to_string())
        );
    }

    #[test]
    fn extract_header_case_insensitive() {
        let response = "HTTP/1.1 200 OK\r\nlocation: http://example.com\r\n";
        assert_eq!(
            extract_header(response, "LOCATION"),
            Some("http://example.com".to_string())
        );
    }
}
