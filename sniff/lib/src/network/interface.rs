use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};

/// Represents a network interface with its associated properties.
///
/// Each interface has a name, optional MAC address, IPv4 and IPv6 addresses
/// (each paired with its CIDR prefix length), and various flags indicating
/// its state.
///
/// ## Examples
///
/// ```
/// use sniff::network::NetworkInterface;
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

    /// List of IPv4 addresses assigned to this interface, each paired with
    /// its CIDR prefix length when the kernel reported a netmask.
    pub ipv4_addresses: Vec<Ipv4Cidr>,

    /// List of IPv6 addresses assigned to this interface, each paired with
    /// its CIDR prefix length when the kernel reported a netmask.
    pub ipv6_addresses: Vec<Ipv6Cidr>,

    /// Interface state flags
    pub flags: InterfaceFlags,
}

/// An IPv4 address paired with its CIDR prefix length.
///
/// The `prefix_len` is derived from the kernel-supplied netmask via
/// `netmask.to_bits().count_ones()` and is therefore exact for the
/// (overwhelmingly common) contiguous-mask case. It is `None` when the
/// platform did not report a netmask for this address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ipv4Cidr {
    /// The IPv4 address.
    pub address: Ipv4Addr,
    /// CIDR prefix length in bits (0-32), if a netmask was reported.
    pub prefix_len: Option<u8>,
}

impl Ipv4Cidr {
    /// Construct a new CIDR pair.
    pub fn new(address: Ipv4Addr, prefix_len: Option<u8>) -> Self {
        Self {
            address,
            prefix_len,
        }
    }
}

/// An IPv6 address paired with its CIDR prefix length.
///
/// The `prefix_len` is derived from the kernel-supplied netmask via
/// `netmask.to_bits().count_ones()` and is therefore exact for the
/// (overwhelmingly common) contiguous-mask case. It is `None` when the
/// platform did not report a netmask for this address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ipv6Cidr {
    /// The IPv6 address.
    pub address: Ipv6Addr,
    /// CIDR prefix length in bits (0-128), if a netmask was reported.
    pub prefix_len: Option<u8>,
}

impl Ipv6Cidr {
    /// Construct a new CIDR pair.
    pub fn new(address: Ipv6Addr, prefix_len: Option<u8>) -> Self {
        Self {
            address,
            prefix_len,
        }
    }
}

impl NetworkInterface {
    /// Creates a new network interface with the given name.
    ///
    /// All other fields are initialized to their default values.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff::network::NetworkInterface;
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

/// An IPv4 address with interface attribution.
///
/// Stores the address as a formatted string for easy serialization
/// and display, along with the interface name that provides this address.
///
/// ## Examples
///
/// ```
/// use sniff::network::Ipv4Address;
///
/// let addr = Ipv4Address {
///     address: "192.168.1.100".to_string(),
///     interface: "en0".to_string(),
///     prefix_len: Some(24),
/// };
/// assert_eq!(addr.address, "192.168.1.100");
/// assert_eq!(addr.interface, "en0");
/// assert_eq!(addr.prefix_len, Some(24));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv4Address {
    /// The IPv4 address as a formatted string (e.g., "192.168.1.100")
    pub address: String,
    /// The interface name providing this address (e.g., "en0")
    pub interface: String,
    /// CIDR prefix length in bits (0-32), if a netmask was reported.
    pub prefix_len: Option<u8>,
}

/// An IPv6 address with interface attribution.
///
/// Stores the address as a formatted string for easy serialization
/// and display, along with the interface name that provides this address.
///
/// ## Examples
///
/// ```
/// use sniff::network::Ipv6Address;
///
/// let addr = Ipv6Address {
///     address: "fe80::1".to_string(),
///     interface: "en0".to_string(),
///     prefix_len: Some(64),
/// };
/// assert_eq!(addr.address, "fe80::1");
/// assert_eq!(addr.interface, "en0");
/// assert_eq!(addr.prefix_len, Some(64));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv6Address {
    /// The IPv6 address as a formatted string (e.g., "fe80::1")
    pub address: String,
    /// The interface name providing this address (e.g., "en0")
    pub interface: String,
    /// CIDR prefix length in bits (0-128), if a netmask was reported.
    pub prefix_len: Option<u8>,
}

/// Aggregated IP addresses across all network interfaces.
///
/// This is a convenience structure that collects all IPv4 and IPv6 addresses
/// from all interfaces in one place, with interface attribution for each address.
/// This allows quick lookups without iterating through individual interfaces.
///
/// ## Examples
///
/// ```
/// use sniff::network::{IpAddresses, Ipv4Address, Ipv6Address};
///
/// let mut addrs = IpAddresses::default();
/// addrs.v4.push(Ipv4Address {
///     address: "192.168.1.100".to_string(),
///     interface: "en0".to_string(),
///     prefix_len: Some(24),
/// });
/// addrs.v6.push(Ipv6Address {
///     address: "fe80::1".to_string(),
///     interface: "en0".to_string(),
///     prefix_len: Some(64),
/// });
/// assert_eq!(addrs.v4.len(), 1);
/// assert_eq!(addrs.v6.len(), 1);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IpAddresses {
    /// All IPv4 addresses with interface attribution
    pub v4: Vec<Ipv4Address>,
    /// All IPv6 addresses with interface attribution
    pub v6: Vec<Ipv6Address>,
}

/// Flags indicating the state and properties of a network interface.
///
/// These flags correspond to the standard interface flags exposed by
/// the operating system (e.g., IFF_UP, IFF_LOOPBACK, IFF_RUNNING).
///
/// ## Examples
///
/// ```
/// use sniff::network::InterfaceFlags;
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
