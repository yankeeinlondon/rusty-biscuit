use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod interface;
pub use interface::{InterfaceFlags, NetworkInterface};

/// Network information for the system.
///
/// Contains a list of all network interfaces, the primary interface name,
/// and a flag indicating if permission was denied during detection.
///
/// ## Examples
///
/// ```
/// use sniff_lib::network::detect_network;
///
/// let info = detect_network().unwrap();
/// if !info.permission_denied {
///     assert!(!info.interfaces.is_empty());
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// All detected network interfaces
    pub interfaces: Vec<NetworkInterface>,

    /// Name of the primary interface (first non-loopback with IPv4 address)
    pub primary_interface: Option<String>,

    /// True if permission was denied during interface enumeration
    pub permission_denied: bool,
}

/// Detects all network interfaces on the system.
///
/// This function enumerates all network interfaces using the `getifaddrs` system call.
/// It handles permission denied errors gracefully by returning a NetworkInfo with
/// `permission_denied` set to true.
///
/// ## Returns
///
/// Returns a `Result<NetworkInfo>` containing:
/// - All detected interfaces with their addresses and flags
/// - The primary interface name (first non-loopback with IPv4)
/// - Permission denied flag
///
/// ## Errors
///
/// Returns an error if:
/// - The `getifaddrs` call fails for reasons other than permission denied
///
/// ## Examples
///
/// ```
/// use sniff_lib::network::detect_network;
///
/// let info = detect_network().unwrap();
/// if !info.permission_denied {
///     // Should have at least loopback
///     assert!(!info.interfaces.is_empty());
/// }
/// ```
pub fn detect_network() -> Result<NetworkInfo> {
    let addrs = match getifaddrs::getifaddrs() {
        Ok(addrs) => addrs,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            return Ok(NetworkInfo {
                interfaces: vec![],
                primary_interface: None,
                permission_denied: true,
            });
        }
        Err(e) => return Err(e.into()),
    };

    let mut interfaces: HashMap<String, NetworkInterface> = HashMap::new();

    for ifaddr in addrs {
        let entry = interfaces
            .entry(ifaddr.name.clone())
            .or_insert_with(|| NetworkInterface::new(ifaddr.name.clone()));

        // Update flags using bitflags contains() method
        entry.flags.is_up = ifaddr.flags.contains(getifaddrs::InterfaceFlags::UP);
        entry.flags.is_loopback = ifaddr.flags.contains(getifaddrs::InterfaceFlags::LOOPBACK);
        entry.flags.is_running = ifaddr.flags.contains(getifaddrs::InterfaceFlags::RUNNING);

        // Add addresses based on type
        match ifaddr.address {
            getifaddrs::Address::V4(v4) => {
                if !entry.ipv4_addresses.contains(&v4.address) {
                    entry.ipv4_addresses.push(v4.address);
                }
            }
            getifaddrs::Address::V6(v6) => {
                if !entry.ipv6_addresses.contains(&v6.address) {
                    entry.ipv6_addresses.push(v6.address);
                }
            }
            _ => {}
        }
    }

    let mut interfaces: Vec<_> = interfaces.into_values().collect();
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));

    let primary = find_primary_interface(&interfaces);

    Ok(NetworkInfo {
        interfaces,
        primary_interface: primary,
        permission_denied: false,
    })
}

/// Finds the primary network interface.
///
/// The primary interface is defined as the first non-loopback interface
/// that is up and has at least one IPv4 address.
///
/// ## Returns
///
/// Returns the name of the primary interface, or None if no suitable interface exists.
fn find_primary_interface(interfaces: &[NetworkInterface]) -> Option<String> {
    interfaces
        .iter()
        .find(|i| !i.flags.is_loopback && !i.ipv4_addresses.is_empty() && i.flags.is_up)
        .map(|i| i.name.clone())
}

/// Detects network interfaces, excluding loopback and down interfaces.
///
/// This is a convenience function that calls `detect_network()` and then
/// filters out loopback interfaces and interfaces that are not up.
///
/// ## Returns
///
/// Returns a `Result<NetworkInfo>` with filtered interfaces.
///
/// ## Errors
///
/// Returns an error if `detect_network()` fails.
///
/// ## Examples
///
/// ```
/// use sniff_lib::network::detect_network_filtered;
///
/// let info = detect_network_filtered().unwrap();
/// // All interfaces should be up and not loopback
/// for iface in &info.interfaces {
///     assert!(iface.flags.is_up);
///     assert!(!iface.flags.is_loopback);
/// }
/// ```
pub fn detect_network_filtered() -> Result<NetworkInfo> {
    let mut info = detect_network()?;
    info.interfaces
        .retain(|i| !i.flags.is_loopback && i.flags.is_up);
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_network_returns_info() {
        let info = detect_network().unwrap();
        // Should have at least loopback, unless permission denied
        assert!(!info.interfaces.is_empty() || info.permission_denied);
    }

    #[test]
    fn test_loopback_is_detected() {
        let info = detect_network().unwrap();
        if !info.permission_denied {
            let has_loopback = info.interfaces.iter().any(|i| i.flags.is_loopback);
            assert!(has_loopback, "Should detect loopback interface");
        }
    }

    #[test]
    fn test_primary_interface_not_loopback() {
        let info = detect_network().unwrap();
        if let Some(ref primary) = info.primary_interface {
            let iface = info.interfaces.iter().find(|i| &i.name == primary);
            assert!(iface.is_some());
            assert!(!iface.unwrap().flags.is_loopback);
        }
    }

    #[test]
    fn test_filtered_excludes_loopback() {
        let info = detect_network_filtered().unwrap();
        for iface in &info.interfaces {
            assert!(!iface.flags.is_loopback);
        }
    }

    #[test]
    fn test_filtered_excludes_down_interfaces() {
        let info = detect_network_filtered().unwrap();
        for iface in &info.interfaces {
            assert!(iface.flags.is_up);
        }
    }

    #[test]
    fn test_interface_has_name() {
        let info = detect_network().unwrap();
        if !info.permission_denied {
            for iface in &info.interfaces {
                assert!(!iface.name.is_empty());
            }
        }
    }
}
