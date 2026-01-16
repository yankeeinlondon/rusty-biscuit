use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod interface;
pub use interface::{InterfaceFlags, IpAddresses, Ipv4Address, Ipv6Address, NetworkInterface};

/// Network information for the system.
///
/// Contains a list of all network interfaces, the primary interface name,
/// aggregated IP addresses across all interfaces, and a flag indicating
/// if permission was denied during detection.
///
/// ## Examples
///
/// ```
/// use sniff_lib::network::detect_network;
///
/// let info = detect_network().unwrap();
/// if !info.permission_denied {
///     assert!(!info.interfaces.is_empty());
///     // ip_addresses aggregates all addresses with interface attribution
///     let total_v4 = info.ip_addresses.v4.len();
///     let per_iface_v4: usize = info.interfaces.iter()
///         .map(|i| i.ipv4_addresses.len())
///         .sum();
///     assert_eq!(total_v4, per_iface_v4);
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// All detected network interfaces
    pub interfaces: Vec<NetworkInterface>,

    /// Name of the primary interface (first non-loopback with IPv4 address)
    pub primary_interface: Option<String>,

    /// Aggregated IP addresses from all interfaces with interface attribution.
    ///
    /// This is a convenience field that collects all IPv4 and IPv6 addresses
    /// in one place, allowing quick lookups without iterating through interfaces.
    pub ip_addresses: IpAddresses,

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
                ip_addresses: IpAddresses::default(),
                permission_denied: true,
            });
        }
        Err(e) => return Err(e.into()),
    };

    let mut interfaces: HashMap<String, NetworkInterface> = HashMap::new();
    let mut ip_addresses = IpAddresses::default();

    for ifaddr in addrs {
        let interface_name = ifaddr.name.clone();
        let entry = interfaces
            .entry(interface_name.clone())
            .or_insert_with(|| NetworkInterface::new(interface_name.clone()));

        // Update flags using bitflags contains() method
        entry.flags.is_up = ifaddr.flags.contains(getifaddrs::InterfaceFlags::UP);
        entry.flags.is_loopback = ifaddr.flags.contains(getifaddrs::InterfaceFlags::LOOPBACK);
        entry.flags.is_running = ifaddr.flags.contains(getifaddrs::InterfaceFlags::RUNNING);

        // Add addresses based on type
        match ifaddr.address {
            getifaddrs::Address::V4(v4) => {
                if !entry.ipv4_addresses.contains(&v4.address) {
                    entry.ipv4_addresses.push(v4.address);
                    // Also add to aggregated addresses
                    ip_addresses.v4.push(Ipv4Address {
                        address: v4.address.to_string(),
                        interface: interface_name,
                    });
                }
            }
            getifaddrs::Address::V6(v6) => {
                if !entry.ipv6_addresses.contains(&v6.address) {
                    entry.ipv6_addresses.push(v6.address);
                    // Also add to aggregated addresses
                    ip_addresses.v6.push(Ipv6Address {
                        address: v6.address.to_string(),
                        interface: interface_name,
                    });
                }
            }
            getifaddrs::Address::Mac(mac) => {
                // Only set MAC if not already set (first non-zero wins)
                if entry.mac_address.is_none() && mac != [0u8; 6] {
                    entry.mac_address = Some(format_mac_address(&mac));
                }
            }
        }
    }

    let mut interfaces: Vec<_> = interfaces.into_values().collect();
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));

    let primary = find_primary_interface(&interfaces);

    Ok(NetworkInfo {
        interfaces,
        primary_interface: primary,
        ip_addresses,
        permission_denied: false,
    })
}

/// Formats a MAC address as a colon-separated hex string.
///
/// ## Examples
///
/// ```ignore
/// let mac = [0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E];
/// assert_eq!(format_mac_address(&mac), "00:1a:2b:3c:4d:5e");
/// ```
fn format_mac_address(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

/// Finds the primary network interface.
///
/// The primary interface is selected using the following priority:
/// 1. Physical interfaces (en*, eth*, wlan*) over virtual ones (bridge*, utun*, docker*, veth*, etc.)
/// 2. Interfaces with RUNNING flag (actively transmitting)
/// 3. First interface alphabetically within the same priority tier
///
/// All candidates must be non-loopback, up, and have at least one IPv4 address.
///
/// ## Returns
///
/// Returns the name of the primary interface, or None if no suitable interface exists.
fn find_primary_interface(interfaces: &[NetworkInterface]) -> Option<String> {
    /// Checks if an interface name looks like a physical interface
    fn is_physical_interface(name: &str) -> bool {
        // Physical interface patterns:
        // - en* (macOS/BSD Ethernet/WiFi)
        // - eth* (Linux Ethernet)
        // - wlan* (Linux WiFi)
        // - wlp* (Linux WiFi with PCI naming)
        // - enp* (Linux with predictable naming)
        name.starts_with("en") && !name.starts_with("enx") // enx* are USB adapters, less preferred
            || name.starts_with("eth")
            || name.starts_with("wlan")
            || name.starts_with("wlp")
            || name.starts_with("enp")
    }

    /// Checks if an interface name looks like a virtual/bridge interface
    fn is_virtual_interface(name: &str) -> bool {
        // Common virtual interface patterns:
        // - bridge* (network bridges)
        // - utun* (macOS VPN tunnels)
        // - tun*, tap* (VPN/virtual network devices)
        // - veth* (Linux virtual Ethernet)
        // - docker*, br-* (Docker)
        // - vmnet* (VM networking)
        // - awdl0 (Apple Wireless Direct Link)
        // - llw* (Low Latency WLAN)
        name.starts_with("bridge")
            || name.starts_with("utun")
            || name.starts_with("tun")
            || name.starts_with("tap")
            || name.starts_with("veth")
            || name.starts_with("docker")
            || name.starts_with("br-")
            || name.starts_with("vmnet")
            || name.starts_with("awdl")
            || name.starts_with("llw")
    }

    // Filter to candidates: non-loopback, up, has IPv4
    let candidates: Vec<_> = interfaces
        .iter()
        .filter(|i| !i.flags.is_loopback && !i.ipv4_addresses.is_empty() && i.flags.is_up)
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Priority 1: Physical + Running
    if let Some(iface) = candidates
        .iter()
        .find(|i| is_physical_interface(&i.name) && i.flags.is_running)
    {
        return Some(iface.name.clone());
    }

    // Priority 2: Physical (even if not running)
    if let Some(iface) = candidates.iter().find(|i| is_physical_interface(&i.name)) {
        return Some(iface.name.clone());
    }

    // Priority 3: Non-virtual + Running
    if let Some(iface) = candidates
        .iter()
        .find(|i| !is_virtual_interface(&i.name) && i.flags.is_running)
    {
        return Some(iface.name.clone());
    }

    // Priority 4: Any non-virtual
    if let Some(iface) = candidates.iter().find(|i| !is_virtual_interface(&i.name)) {
        return Some(iface.name.clone());
    }

    // Fallback: First candidate (even if virtual)
    candidates.first().map(|i| i.name.clone())
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

    // Collect retained interface names for filtering ip_addresses
    let retained_names: std::collections::HashSet<&str> =
        info.interfaces.iter().map(|i| i.name.as_str()).collect();

    // Filter ip_addresses to only include addresses from retained interfaces
    info.ip_addresses
        .v4
        .retain(|addr| retained_names.contains(addr.interface.as_str()));
    info.ip_addresses
        .v6
        .retain(|addr| retained_names.contains(addr.interface.as_str()));

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

    #[test]
    fn test_format_mac_address() {
        let mac = [0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e];
        assert_eq!(format_mac_address(&mac), "00:1a:2b:3c:4d:5e");
    }

    #[test]
    fn test_format_mac_address_all_zeros() {
        let mac = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(format_mac_address(&mac), "00:00:00:00:00:00");
    }

    #[test]
    fn test_format_mac_address_all_ff() {
        let mac = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        assert_eq!(format_mac_address(&mac), "ff:ff:ff:ff:ff:ff");
    }

    #[test]
    fn test_non_loopback_has_mac_address() {
        let info = detect_network().unwrap();
        if !info.permission_denied {
            // At least one non-loopback interface should have a MAC address
            // (on most systems with physical or virtual network hardware)
            let has_mac = info
                .interfaces
                .iter()
                .filter(|i| !i.flags.is_loopback)
                .any(|i| i.mac_address.is_some());

            // This is a soft assertion - not all systems will have MAC addresses visible
            // (e.g., containers, minimal VMs). We just ensure we don't crash.
            if has_mac {
                let iface_with_mac = info
                    .interfaces
                    .iter()
                    .find(|i| i.mac_address.is_some())
                    .unwrap();
                let mac = iface_with_mac.mac_address.as_ref().unwrap();
                // MAC should be formatted as xx:xx:xx:xx:xx:xx
                assert_eq!(mac.len(), 17, "MAC address should be 17 chars: {mac}");
                assert_eq!(
                    mac.matches(':').count(),
                    5,
                    "MAC should have 5 colons: {mac}"
                );
            }
        }
    }

    // ============================================================================
    // ip_addresses Field Tests
    // ============================================================================

    #[test]
    fn test_ip_addresses_count_matches_per_interface_sum() {
        let info = detect_network().unwrap();
        if !info.permission_denied {
            // Sum of per-interface IPv4 addresses should equal aggregated v4 count
            let per_iface_v4: usize = info.interfaces.iter().map(|i| i.ipv4_addresses.len()).sum();
            assert_eq!(
                info.ip_addresses.v4.len(),
                per_iface_v4,
                "Aggregated IPv4 count should match sum of per-interface counts"
            );

            // Sum of per-interface IPv6 addresses should equal aggregated v6 count
            let per_iface_v6: usize = info.interfaces.iter().map(|i| i.ipv6_addresses.len()).sum();
            assert_eq!(
                info.ip_addresses.v6.len(),
                per_iface_v6,
                "Aggregated IPv6 count should match sum of per-interface counts"
            );
        }
    }

    #[test]
    fn test_ip_addresses_interface_attribution_is_valid() {
        let info = detect_network().unwrap();
        if !info.permission_denied {
            // Collect all interface names
            let interface_names: std::collections::HashSet<&str> =
                info.interfaces.iter().map(|i| i.name.as_str()).collect();

            // Every IPv4 address should have a valid interface attribution
            for addr in &info.ip_addresses.v4 {
                assert!(
                    interface_names.contains(addr.interface.as_str()),
                    "IPv4 address {} has invalid interface '{}' - not found in interfaces list",
                    addr.address,
                    addr.interface
                );
            }

            // Every IPv6 address should have a valid interface attribution
            for addr in &info.ip_addresses.v6 {
                assert!(
                    interface_names.contains(addr.interface.as_str()),
                    "IPv6 address {} has invalid interface '{}' - not found in interfaces list",
                    addr.address,
                    addr.interface
                );
            }
        }
    }

    #[test]
    fn test_ip_addresses_are_valid_ip_strings() {
        use std::net::{Ipv4Addr, Ipv6Addr};

        let info = detect_network().unwrap();
        if !info.permission_denied {
            // Every IPv4 address should parse as valid Ipv4Addr
            for addr in &info.ip_addresses.v4 {
                let parsed = addr.address.parse::<Ipv4Addr>();
                assert!(
                    parsed.is_ok(),
                    "IPv4 address '{}' should be a valid IPv4 string",
                    addr.address
                );
            }

            // Every IPv6 address should parse as valid Ipv6Addr
            for addr in &info.ip_addresses.v6 {
                let parsed = addr.address.parse::<Ipv6Addr>();
                assert!(
                    parsed.is_ok(),
                    "IPv6 address '{}' should be a valid IPv6 string",
                    addr.address
                );
            }
        }
    }

    #[test]
    fn test_filtered_ip_addresses_match_retained_interfaces() {
        let info = detect_network_filtered().unwrap();
        if !info.permission_denied {
            // Collect retained interface names (non-loopback, up)
            let retained_names: std::collections::HashSet<&str> =
                info.interfaces.iter().map(|i| i.name.as_str()).collect();

            // All IPv4 addresses in filtered result should be from retained interfaces
            for addr in &info.ip_addresses.v4 {
                assert!(
                    retained_names.contains(addr.interface.as_str()),
                    "Filtered IPv4 address {} should only be from retained interfaces, got '{}'",
                    addr.address,
                    addr.interface
                );
            }

            // All IPv6 addresses in filtered result should be from retained interfaces
            for addr in &info.ip_addresses.v6 {
                assert!(
                    retained_names.contains(addr.interface.as_str()),
                    "Filtered IPv6 address {} should only be from retained interfaces, got '{}'",
                    addr.address,
                    addr.interface
                );
            }

            // Verify no loopback addresses appear in filtered result
            // (loopback interfaces are filtered out, so their addresses should be too)
            let unfiltered = detect_network().unwrap();
            let loopback_names: std::collections::HashSet<&str> = unfiltered
                .interfaces
                .iter()
                .filter(|i| i.flags.is_loopback)
                .map(|i| i.name.as_str())
                .collect();

            for addr in &info.ip_addresses.v4 {
                assert!(
                    !loopback_names.contains(addr.interface.as_str()),
                    "Filtered result should not contain loopback IPv4 address {}",
                    addr.address
                );
            }

            for addr in &info.ip_addresses.v6 {
                assert!(
                    !loopback_names.contains(addr.interface.as_str()),
                    "Filtered result should not contain loopback IPv6 address {}",
                    addr.address
                );
            }
        }
    }

    #[test]
    fn test_ip_addresses_json_serialization() {
        let info = detect_network().unwrap();
        let json = serde_json::to_string(&info).expect("NetworkInfo should serialize to JSON");

        // JSON should contain ip_addresses field with v4 and v6 arrays
        assert!(
            json.contains("\"ip_addresses\""),
            "JSON should contain ip_addresses field"
        );
        assert!(
            json.contains("\"v4\""),
            "JSON should contain v4 field in ip_addresses"
        );
        assert!(
            json.contains("\"v6\""),
            "JSON should contain v6 field in ip_addresses"
        );

        // Deserialize and verify roundtrip
        let parsed: NetworkInfo =
            serde_json::from_str(&json).expect("JSON should deserialize back to NetworkInfo");
        assert_eq!(
            info.ip_addresses.v4.len(),
            parsed.ip_addresses.v4.len(),
            "Roundtrip should preserve v4 count"
        );
        assert_eq!(
            info.ip_addresses.v6.len(),
            parsed.ip_addresses.v6.len(),
            "Roundtrip should preserve v6 count"
        );
    }

    #[test]
    fn test_ip_addresses_empty_arrays_serialize_as_empty() {
        // Test that empty IpAddresses serializes with empty arrays, not null
        let empty = IpAddresses::default();
        let json = serde_json::to_string(&empty).expect("Empty IpAddresses should serialize");
        assert!(
            json.contains("\"v4\":[]"),
            "Empty v4 should serialize as [], got: {}",
            json
        );
        assert!(
            json.contains("\"v6\":[]"),
            "Empty v6 should serialize as [], got: {}",
            json
        );
    }

    // ============================================================================
    // Primary Interface Selection Tests (Regression tests for physical vs virtual)
    // ============================================================================

    /// Helper to create a test interface
    fn create_test_interface(name: &str, has_ipv4: bool, is_running: bool) -> NetworkInterface {
        let mut iface = NetworkInterface::new(name.to_string());
        iface.flags.is_up = true;
        iface.flags.is_running = is_running;
        if has_ipv4 {
            iface.ipv4_addresses.push("192.168.1.100".parse().unwrap());
        }
        iface
    }

    #[test]
    fn test_primary_interface_prefers_physical_over_virtual() {
        // Regression test: Physical interfaces (en*, eth*) should be preferred over
        // virtual/bridge interfaces (bridge*, utun*, docker*)
        //
        // Bug: bridge100 was selected over en0 because it came first alphabetically
        let interfaces = vec![
            create_test_interface("bridge100", true, true), // Virtual bridge
            create_test_interface("en0", true, true),       // Physical WiFi/Ethernet
            create_test_interface("utun0", true, true),     // VPN tunnel
        ];

        let primary = find_primary_interface(&interfaces);
        assert_eq!(
            primary,
            Some("en0".to_string()),
            "Should prefer physical interface en0 over virtual bridge100"
        );
    }

    #[test]
    fn test_primary_interface_prefers_running_physical() {
        // Among physical interfaces, prefer ones with RUNNING flag
        let interfaces = vec![
            create_test_interface("en0", true, false), // Physical but not running
            create_test_interface("en1", true, true),  // Physical and running
        ];

        let primary = find_primary_interface(&interfaces);
        assert_eq!(
            primary,
            Some("en1".to_string()),
            "Should prefer running physical interface"
        );
    }

    #[test]
    fn test_primary_interface_picks_any_physical_if_none_running() {
        // If no physical interfaces are running, pick any physical
        let interfaces = vec![
            create_test_interface("bridge100", true, true), // Virtual
            create_test_interface("en0", true, false),      // Physical but not running
        ];

        let primary = find_primary_interface(&interfaces);
        assert_eq!(
            primary,
            Some("en0".to_string()),
            "Should prefer physical interface even if not running over virtual"
        );
    }

    #[test]
    fn test_primary_interface_fallback_to_virtual_when_no_physical() {
        // When only virtual interfaces exist, select the first virtual
        let interfaces = vec![
            create_test_interface("bridge100", true, true),
            create_test_interface("utun0", true, true),
        ];

        let primary = find_primary_interface(&interfaces);
        assert_eq!(
            primary,
            Some("bridge100".to_string()),
            "Should fallback to virtual interface when no physical exists"
        );
    }

    #[test]
    fn test_primary_interface_returns_none_when_no_ipv4() {
        // Interfaces without IPv4 addresses should not be selected
        let interfaces = vec![
            create_test_interface("en0", false, true), // No IPv4
            create_test_interface("en1", false, true), // No IPv4
        ];

        let primary = find_primary_interface(&interfaces);
        assert_eq!(
            primary, None,
            "Should return None when no interfaces have IPv4 addresses"
        );
    }

    #[test]
    fn test_primary_interface_returns_none_when_only_loopback() {
        // Loopback interfaces should never be selected as primary
        let mut loopback = create_test_interface("lo0", true, true);
        loopback.flags.is_loopback = true;

        let interfaces = vec![loopback];

        let primary = find_primary_interface(&interfaces);
        assert_eq!(
            primary, None,
            "Should return None when only loopback interface exists"
        );
    }

    #[test]
    fn test_primary_interface_excludes_common_virtual_patterns() {
        // Test that common virtual interface patterns are correctly identified
        let virtual_patterns = vec![
            "bridge100",
            "utun0",
            "tun0",
            "tap0",
            "veth123abc",
            "docker0",
            "br-abc123",
            "vmnet1",
            "awdl0",
            "llw0",
        ];

        for virtual_name in virtual_patterns {
            let interfaces = vec![
                create_test_interface(virtual_name, true, true),
                create_test_interface("en0", true, true),
            ];

            let primary = find_primary_interface(&interfaces);
            assert_eq!(
                primary,
                Some("en0".to_string()),
                "Should prefer en0 over virtual interface {}",
                virtual_name
            );
        }
    }

    #[test]
    fn test_primary_interface_recognizes_physical_patterns() {
        // Test that common physical interface patterns are correctly identified
        let physical_patterns = vec![
            "en0",     // macOS/BSD
            "en1",     // macOS/BSD
            "eth0",    // Linux Ethernet
            "eth1",    // Linux Ethernet
            "wlan0",   // Linux WiFi
            "wlp3s0",  // Linux WiFi with PCI naming
            "enp0s31", // Linux with predictable naming
        ];

        for physical_name in physical_patterns {
            let interfaces = vec![
                create_test_interface("bridge100", true, true),
                create_test_interface(physical_name, true, true),
            ];

            let primary = find_primary_interface(&interfaces);
            assert_eq!(
                primary,
                Some(physical_name.to_string()),
                "Should recognize {} as physical interface",
                physical_name
            );
        }
    }

    #[test]
    fn test_primary_interface_prefers_builtin_over_usb() {
        // enx* interfaces are USB adapters, should be less preferred than built-in en*
        let interfaces = vec![
            create_test_interface("enx00e04c123456", true, true), // USB adapter
            create_test_interface("en0", true, true),             // Built-in
        ];

        let primary = find_primary_interface(&interfaces);
        assert_eq!(
            primary,
            Some("en0".to_string()),
            "Should prefer built-in en0 over USB adapter enx*"
        );
    }

    #[test]
    fn test_primary_interface_with_real_system_interfaces() {
        // Integration test: verify the fix works with actual detected interfaces
        let info = detect_network().unwrap();
        if !info.permission_denied && info.interfaces.len() > 1 {
            // If we have multiple interfaces, primary should not be a known virtual pattern
            if let Some(ref primary) = info.primary_interface {
                let is_common_virtual = primary.starts_with("bridge")
                    || primary.starts_with("utun")
                    || primary.starts_with("docker")
                    || primary.starts_with("veth");

                // Check if there's a physical interface available
                let has_physical = info.interfaces.iter().any(|i| {
                    !i.flags.is_loopback
                        && !i.ipv4_addresses.is_empty()
                        && i.flags.is_up
                        && (i.name.starts_with("en") && !i.name.starts_with("enx")
                            || i.name.starts_with("eth")
                            || i.name.starts_with("wlan"))
                });

                if has_physical {
                    assert!(
                        !is_common_virtual,
                        "Primary interface should not be a virtual interface ({}) when physical interfaces are available",
                        primary
                    );
                }
            }
        }
    }
}
