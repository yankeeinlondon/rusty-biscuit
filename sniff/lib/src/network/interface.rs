use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};

/// Represents a network interface with its associated properties.
///
/// Each interface has a name, optional MAC address, IPv4 and IPv6 addresses,
/// and various flags indicating its state.
///
/// ## Examples
///
/// ```
/// use sniff_lib::network::NetworkInterface;
///
/// let iface = NetworkInterface::new("eth0".to_string());
/// assert_eq!(iface.name, "eth0");
/// assert!(iface.ipv4_addresses.is_empty());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name (e.g., "eth0", "wlan0", "lo")
    pub name: String,

    /// MAC address in hexadecimal format (e.g., "00:1A:2B:3C:4D:5E")
    pub mac_address: Option<String>,

    /// List of IPv4 addresses assigned to this interface
    pub ipv4_addresses: Vec<Ipv4Addr>,

    /// List of IPv6 addresses assigned to this interface
    pub ipv6_addresses: Vec<Ipv6Addr>,

    /// Interface state flags
    pub flags: InterfaceFlags,
}

impl NetworkInterface {
    /// Creates a new network interface with the given name.
    ///
    /// All other fields are initialized to their default values.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::network::NetworkInterface;
    ///
    /// let iface = NetworkInterface::new("eth0".to_string());
    /// assert_eq!(iface.name, "eth0");
    /// assert!(iface.mac_address.is_none());
    /// assert!(iface.ipv4_addresses.is_empty());
    /// ```
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

/// Flags indicating the state and properties of a network interface.
///
/// These flags correspond to the standard interface flags exposed by
/// the operating system (e.g., IFF_UP, IFF_LOOPBACK, IFF_RUNNING).
///
/// ## Examples
///
/// ```
/// use sniff_lib::network::InterfaceFlags;
///
/// let flags = InterfaceFlags {
///     is_up: true,
///     is_loopback: false,
///     is_running: true,
/// };
/// assert!(flags.is_up);
/// assert!(!flags.is_loopback);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InterfaceFlags {
    /// True if the interface is administratively up (IFF_UP)
    pub is_up: bool,

    /// True if this is a loopback interface (IFF_LOOPBACK)
    pub is_loopback: bool,

    /// True if the interface is running and can transmit/receive (IFF_RUNNING)
    pub is_running: bool,
}
