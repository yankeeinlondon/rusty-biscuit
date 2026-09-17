//! Scope-preserving IP addresses and address-bit matching.
//!
//! Interface enumeration, default gateways, and ICMP targets share one address
//! spelling: an IPv4 or IPv6 literal, optionally followed by an IPv6 zone
//! (`fe80::1%en0`). The zone is what makes a link-local address routable, so it
//! is carried through rather than dropped by `std::net::IpAddr`. Network
//! membership ([`ScopedIpAddr::is_within`]) compares address bits only and never
//! consults the zone.

use std::collections::BTreeSet;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use ipnet::{IpNet, Ipv4Net};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::NetworkInterface;

/// Why a string is not a [`ScopedIpAddr`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AddressParseError {
    /// The address part is not an IPv4 or IPv6 literal. No DNS lookup is made.
    #[error("`{input}` is not an IPv4 or IPv6 address")]
    InvalidAddress {
        /// The rejected input.
        input: String,
    },
    /// A zone was attached to an IPv4 address; zones exist only for IPv6.
    #[error("`{input}` has a scope suffix, but only IPv6 addresses can be scoped")]
    ScopeOnIpv4 {
        /// The rejected input.
        input: String,
    },
    /// The zone after `%` is empty or contains whitespace, `%`, or `/`.
    #[error("`{input}` has an invalid IPv6 scope suffix")]
    InvalidScope {
        /// The rejected input.
        input: String,
    },
}

/// An IP address with an optional IPv6 zone (scope) identifier.
///
/// The zone is an interface name (`en0`) or a numeric interface index (`12`),
/// kept exactly as given. Ordering is by address (every IPv4 address before
/// every IPv6 address, then numerically), then by zone, so a sorted list is
/// stable across enumerations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopedIpAddr {
    address: IpAddr,
    scope: Option<String>,
}

impl ScopedIpAddr {
    /// An address without a zone.
    pub fn new(address: IpAddr) -> Self {
        Self {
            address,
            scope: None,
        }
    }

    /// An IPv6 address with a zone.
    ///
    /// ## Errors
    ///
    /// Returns [`AddressParseError::InvalidScope`] when `scope` is empty or
    /// contains whitespace, `%`, or `/`.
    pub fn with_scope(
        address: Ipv6Addr,
        scope: impl Into<String>,
    ) -> Result<Self, AddressParseError> {
        let scope = scope.into();
        if !is_valid_scope(&scope) {
            return Err(AddressParseError::InvalidScope {
                input: format!("{address}%{scope}"),
            });
        }
        Ok(Self {
            address: IpAddr::V6(address),
            scope: Some(scope),
        })
    }

    #[allow(missing_docs)]
    pub fn address(&self) -> IpAddr {
        self.address
    }

    #[allow(missing_docs)]
    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    /// True when the address bits fall inside `network`. The zone is ignored and
    /// a network of the other family never matches.
    pub fn is_within(&self, network: &IpNet) -> bool {
        match (self.address, network) {
            (IpAddr::V4(address), IpNet::V4(network)) => network.contains(&address),
            (IpAddr::V6(address), IpNet::V6(network)) => network.contains(&address),
            _ => false,
        }
    }

    /// True for `127.0.0.0/8` and `::1`.
    pub fn is_loopback(&self) -> bool {
        self.address.is_loopback()
    }

    /// True for `169.254.0.0/16` and `fe80::/10`.
    pub fn is_link_local(&self) -> bool {
        match self.address {
            IpAddr::V4(address) => address.is_link_local(),
            IpAddr::V6(address) => address.is_unicast_link_local(),
        }
    }
}

impl fmt::Display for ScopedIpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.scope {
            Some(scope) => write!(f, "{}%{scope}", self.address),
            None => write!(f, "{}", self.address),
        }
    }
}

impl FromStr for ScopedIpAddr {
    type Err = AddressParseError;

    /// Parses `a.b.c.d`, an IPv6 literal, or `ipv6%zone`.
    ///
    /// Strict: no surrounding whitespace, brackets, ports, or host names.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (address, scope) = match input.split_once('%') {
            Some((address, scope)) => (address, Some(scope)),
            None => (input, None),
        };
        let parsed: IpAddr = address
            .parse()
            .map_err(|_| AddressParseError::InvalidAddress {
                input: input.to_string(),
            })?;
        match (parsed, scope) {
            (address, None) => Ok(Self::new(address)),
            (IpAddr::V4(_), Some(_)) => Err(AddressParseError::ScopeOnIpv4 {
                input: input.to_string(),
            }),
            (IpAddr::V6(address), Some(scope)) => Self::with_scope(address, scope),
        }
    }
}

impl From<IpAddr> for ScopedIpAddr {
    fn from(address: IpAddr) -> Self {
        Self::new(address)
    }
}

impl Serialize for ScopedIpAddr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ScopedIpAddr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

fn is_valid_scope(scope: &str) -> bool {
    !scope.is_empty()
        && scope
            .chars()
            .all(|c| !c.is_whitespace() && !c.is_control() && c != '%' && c != '/')
}

/// The shared address space of RFC 6598, `100.64.0.0/10`, used by carrier-grade
/// NAT and by Tailscale for tailnet addresses.
pub fn cgnat_network() -> IpNet {
    IpNet::V4(Ipv4Net::new_assert(Ipv4Addr::new(100, 64, 0, 0), 10))
}

/// True when any address is inside [`cgnat_network`].
///
/// A CGNAT ISP address or another VPN using that range also returns `true`.
pub fn contains_cgnat_address<'a>(addresses: impl IntoIterator<Item = &'a ScopedIpAddr>) -> bool {
    let network = cgnat_network();
    addresses
        .into_iter()
        .any(|address| address.is_within(&network))
}

/// Every interface address, deduplicated by address and zone, in stable order.
///
/// IPv6 link-local addresses carry the zone that makes them routable: the
/// interface name on Unix (`fe80::1%en0`) and the numeric interface index on
/// Windows (`fe80::1%12`), the spelling each platform's own tools accept.
/// Loopback and link-local addresses are included; filtering is the caller's.
pub fn host_addresses(interfaces: &[NetworkInterface]) -> Vec<ScopedIpAddr> {
    let mut addresses = BTreeSet::new();
    for interface in interfaces {
        for cidr in &interface.ipv4_addresses {
            addresses.insert(ScopedIpAddr::new(IpAddr::V4(cidr.address)));
        }
        for cidr in &interface.ipv6_addresses {
            let address = if cidr.address.is_unicast_link_local() {
                ScopedIpAddr::with_scope(cidr.address, zone_id(interface))
                    .unwrap_or_else(|_| ScopedIpAddr::new(IpAddr::V6(cidr.address)))
            } else {
                ScopedIpAddr::new(IpAddr::V6(cidr.address))
            };
            addresses.insert(address);
        }
    }
    addresses.into_iter().collect()
}

#[cfg(windows)]
fn zone_id(interface: &NetworkInterface) -> String {
    interface
        .index
        .map(|index| index.to_string())
        .unwrap_or_else(|| interface.name.clone())
}

#[cfg(not(windows))]
fn zone_id(interface: &NetworkInterface) -> String {
    interface.name.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{Ipv4Cidr, Ipv6Cidr};

    fn interface(name: &str, index: u32, v4: &[&str], v6: &[&str]) -> NetworkInterface {
        let mut interface = NetworkInterface::new(name.to_string());
        interface.index = Some(index);
        interface.ipv4_addresses = v4
            .iter()
            .map(|a| Ipv4Cidr::new(a.parse().unwrap(), Some(24)))
            .collect();
        interface.ipv6_addresses = v6
            .iter()
            .map(|a| Ipv6Cidr::new(a.parse().unwrap(), Some(64)))
            .collect();
        interface
    }

    #[test]
    fn parses_and_displays_every_accepted_spelling() {
        for spelling in [
            "192.168.1.10",
            "2001:db8::1",
            "::1",
            "fe80::1%en0",
            "fe80::1%12",
            "::ffff:192.0.2.1",
        ] {
            let parsed: ScopedIpAddr = spelling.parse().unwrap();
            assert_eq!(parsed.to_string(), spelling);
        }
    }

    #[test]
    fn normalizes_ipv6_spelling_but_keeps_the_zone_verbatim() {
        let parsed: ScopedIpAddr = "FE80:0:0:0::0001%Ethernet_32769".parse().unwrap();
        assert_eq!(parsed.to_string(), "fe80::1%Ethernet_32769");
        assert_eq!(parsed.scope(), Some("Ethernet_32769"));
        assert_eq!(parsed.address(), "fe80::1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn rejects_malformed_addresses_without_resolving_names() {
        for input in [
            "",
            "localhost",
            "example.com",
            "192.168.1",
            "192.168.1.256",
            " 192.168.1.1",
            "192.168.1.1 ",
            "[::1]",
            "::1:80:",
            "10.0.0.0/8",
        ] {
            assert_eq!(
                input.parse::<ScopedIpAddr>(),
                Err(AddressParseError::InvalidAddress {
                    input: input.to_string()
                }),
                "{input:?}"
            );
        }
    }

    #[test]
    fn rejects_a_scope_on_ipv4_as_a_family_mismatch() {
        assert_eq!(
            "192.168.1.1%en0".parse::<ScopedIpAddr>(),
            Err(AddressParseError::ScopeOnIpv4 {
                input: "192.168.1.1%en0".to_string()
            })
        );
    }

    #[test]
    fn rejects_empty_or_malformed_zones() {
        for input in ["fe80::1%", "fe80::1%en 0", "fe80::1%en0%1", "fe80::1%en0/64"] {
            assert!(
                matches!(
                    input.parse::<ScopedIpAddr>(),
                    Err(AddressParseError::InvalidScope { .. })
                ),
                "{input:?}"
            );
        }
    }

    #[test]
    fn serde_round_trips_through_the_display_spelling() {
        let addresses: Vec<ScopedIpAddr> = vec![
            "10.0.0.1".parse().unwrap(),
            "fe80::1%en0".parse().unwrap(),
        ];
        let json = serde_json::to_string(&addresses).unwrap();
        assert_eq!(json, r#"["10.0.0.1","fe80::1%en0"]"#);
        let decoded: Vec<ScopedIpAddr> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, addresses);
        assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        assert!(serde_json::from_str::<ScopedIpAddr>(r#""10.0.0.1%en0""#).is_err());
    }

    #[test]
    fn network_membership_compares_bits_and_ignores_the_zone() {
        let link_local: ScopedIpAddr = "fe80::1%en0".parse().unwrap();
        assert!(link_local.is_within(&"fe80::/10".parse().unwrap()));
        assert!(!link_local.is_within(&"fe80::/128".parse().unwrap()));

        let private: ScopedIpAddr = "192.168.10.20".parse().unwrap();
        assert!(private.is_within(&"192.168.10.0/24".parse().unwrap()));
        assert!(!private.is_within(&"192.168.11.0/24".parse().unwrap()));
    }

    #[test]
    fn a_network_of_the_other_family_never_matches() {
        let v4: ScopedIpAddr = "0.0.0.1".parse().unwrap();
        let v6: ScopedIpAddr = "::1".parse().unwrap();
        assert!(!v4.is_within(&"::/0".parse().unwrap()));
        assert!(!v6.is_within(&"0.0.0.0/0".parse().unwrap()));
        let mapped: ScopedIpAddr = "::ffff:100.64.0.1".parse().unwrap();
        assert!(!mapped.is_within(&cgnat_network()));
    }

    #[test]
    fn loopback_and_link_local_classification_covers_both_families() {
        let cases = [
            ("127.0.0.1", true, false),
            ("127.255.0.9", true, false),
            ("::1", true, false),
            ("169.254.3.4", false, true),
            ("fe80::1%en0", false, true),
            ("febf::1", false, true),
            ("fec0::1", false, false),
            ("10.0.0.1", false, false),
        ];
        for (input, loopback, link_local) in cases {
            let address: ScopedIpAddr = input.parse().unwrap();
            assert_eq!(address.is_loopback(), loopback, "{input}");
            assert_eq!(address.is_link_local(), link_local, "{input}");
        }
    }

    #[test]
    fn cgnat_boundaries_are_exact() {
        let inside = ["100.64.0.0", "100.100.100.100", "100.127.255.255"];
        let outside = ["100.63.255.255", "100.128.0.0", "10.0.0.1", "fd7a:115c:a1e0::1"];
        for input in inside {
            assert!(
                contains_cgnat_address([&input.parse::<ScopedIpAddr>().unwrap()]),
                "{input}"
            );
        }
        for input in outside {
            assert!(
                !contains_cgnat_address([&input.parse::<ScopedIpAddr>().unwrap()]),
                "{input}"
            );
        }
    }

    #[test]
    fn tailnet_predicate_is_true_only_with_a_cgnat_interface_address() {
        let lan = interface("en0", 4, &["192.168.1.20"], &["fe80::1c"]);
        let tailscale = interface("utun4", 21, &["100.101.102.103"], &["fd7a:115c:a1e0::1"]);

        assert!(!contains_cgnat_address(&host_addresses(&[lan.clone()])));
        assert!(contains_cgnat_address(&host_addresses(&[lan, tailscale])));
        assert!(!contains_cgnat_address(&host_addresses(&[])));
    }

    #[test]
    fn host_addresses_deduplicate_and_order_stably() {
        let first = interface(
            "en1",
            5,
            &["192.168.1.20", "10.0.0.2"],
            &["2001:db8::2", "fe80::1"],
        );
        let second = interface("en0", 4, &["10.0.0.2"], &["fe80::1", "2001:db8::2"]);
        let loopback = interface("lo0", 1, &["127.0.0.1"], &["::1"]);

        let forward = host_addresses(&[first.clone(), second.clone(), loopback.clone()]);
        let reverse = host_addresses(&[loopback, second, first]);
        assert_eq!(forward, reverse);

        let rendered: Vec<String> = forward.iter().map(ToString::to_string).collect();
        #[cfg(not(windows))]
        let expected = [
            "10.0.0.2",
            "127.0.0.1",
            "192.168.1.20",
            "::1",
            "2001:db8::2",
            "fe80::1%en0",
            "fe80::1%en1",
        ];
        #[cfg(windows)]
        let expected = [
            "10.0.0.2",
            "127.0.0.1",
            "192.168.1.20",
            "::1",
            "2001:db8::2",
            "fe80::1%4",
            "fe80::1%5",
        ];
        assert_eq!(rendered, expected);
    }
}
