//! Wake-on-LAN magic packet utility.

use std::net::UdpSocket;
use thiserror::Error;

/// Errors that can occur when sending a WoL magic packet.
#[derive(Debug, Error)]
pub enum WolError {
    /// Invalid MAC address format
    #[error("invalid MAC address: {0}")]
    InvalidMac(String),

    /// UDP socket error
    #[error("network error: {0}")]
    Network(#[from] std::io::Error),
}

/// Sends a Wake-on-LAN magic packet to the specified MAC address.
///
/// ## Examples
///
/// ```no_run
/// # use homelab::wol::send_magic_packet;
/// // Standard WoL (port 9, broadcast)
/// send_magic_packet("AA:BB:CC:DD:EE:FF", "255.255.255.255", 9).unwrap();
///
/// // Eversolo uses port 9517
/// send_magic_packet("AA:BB:CC:DD:EE:FF", "255.255.255.255", 9517).unwrap();
/// ```
///
/// ## Errors
///
/// Returns an error if the MAC address is invalid or the UDP packet cannot be sent.
pub fn send_magic_packet(mac: &str, broadcast_addr: &str, port: u16) -> Result<(), WolError> {
    let mac_bytes = parse_mac(mac)?;

    // Magic packet: 6 bytes of 0xFF followed by the target MAC repeated 16 times
    let mut packet = [0u8; 102];
    packet[..6].fill(0xFF);
    for i in 0..16 {
        let offset = 6 + i * 6;
        packet[offset..offset + 6].copy_from_slice(&mac_bytes);
    }

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;
    socket.send_to(&packet, (broadcast_addr, port))?;

    Ok(())
}

/// Parses a MAC address string into 6 bytes.
///
/// Accepts colon-separated (`AA:BB:CC:DD:EE:FF`) or hyphen-separated (`AA-BB-CC-DD-EE-FF`) formats.
fn parse_mac(mac: &str) -> Result<[u8; 6], WolError> {
    let parts: Vec<&str> = if mac.contains(':') {
        mac.split(':').collect()
    } else if mac.contains('-') {
        mac.split('-').collect()
    } else {
        return Err(WolError::InvalidMac(mac.to_string()));
    };

    if parts.len() != 6 {
        return Err(WolError::InvalidMac(mac.to_string()));
    }

    let mut bytes = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        bytes[i] =
            u8::from_str_radix(part, 16).map_err(|_| WolError::InvalidMac(mac.to_string()))?;
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mac_colon() {
        let bytes = parse_mac("AA:BB:CC:DD:EE:FF").unwrap();
        assert_eq!(bytes, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn test_parse_mac_hyphen() {
        let bytes = parse_mac("aa-bb-cc-dd-ee-ff").unwrap();
        assert_eq!(bytes, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn test_parse_mac_invalid_length() {
        assert!(parse_mac("AA:BB:CC").is_err());
    }

    #[test]
    fn test_parse_mac_invalid_hex() {
        assert!(parse_mac("GG:BB:CC:DD:EE:FF").is_err());
    }

    #[test]
    fn test_parse_mac_no_separator() {
        assert!(parse_mac("AABBCCDDEEFF").is_err());
    }
}
