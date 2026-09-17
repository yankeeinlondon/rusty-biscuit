//! Primary default-gateway discovery for IPv4 and IPv6.
//!
//! "Primary" is the gateway of the lowest-metric UP default route on Linux and
//! Windows (first wins a tie), and of the first unscoped UP default route in the
//! routing-socket dump on macOS. A default route that has no gateway address
//! (on-link or point-to-point) yields no gateway, exactly like no default route;
//! selection never falls through to a worse route that happens to have one.
//!
//! Each platform's parser is a pure function over the captured table, so every
//! parser is exercised by fixtures on every host.

use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};

use super::ScopedIpAddr;

#[cfg(any(target_os = "macos", test))]
mod darwin;
#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(any(target_os = "windows", test))]
mod windows;

/// The host's primary default gateway for each address family.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefaultGateways {
    /// Gateway of the primary IPv4 default route.
    pub v4: Option<Ipv4Addr>,
    /// Gateway of the primary IPv6 default route. A link-local gateway keeps
    /// its zone (`fe80::1%en0`, or `fe80::1%12` on Windows).
    pub v6: Option<ScopedIpAddr>,
}

/// Reads the routing table and returns the primary default gateway per family.
///
/// Performs no DNS lookup and sends no packet. On Windows this runs one
/// `route print` under the bounded subprocess boundary.
///
/// ## Errors
///
/// Returns an error when the IPv4 routing table cannot be read at all (the
/// Linux `/proc` file, the macOS routing sysctl, or the Windows command). A
/// missing IPv6 table — IPv6 disabled — is not an error and yields no IPv6
/// gateway. Platforms outside macOS, Linux, and Windows return no gateways.
pub fn detect_default_gateways() -> crate::Result<DefaultGateways> {
    detect_native()
}

#[cfg(target_os = "linux")]
fn detect_native() -> crate::Result<DefaultGateways> {
    let v4 = std::fs::read_to_string("/proc/net/route")?;
    let v6 = match std::fs::read_to_string("/proc/net/ipv6_route") {
        Ok(table) => linux::ipv6_default_gateway(&table),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    Ok(DefaultGateways {
        v4: linux::ipv4_default_gateway(&v4),
        v6,
    })
}

#[cfg(target_os = "macos")]
fn detect_native() -> crate::Result<DefaultGateways> {
    let resolve = |index: u16| getifaddrs::if_indextoname(u32::from(index)).ok();
    let v4 = darwin::route_dump(libc::AF_INET)?;
    let v6 = match darwin::route_dump(libc::AF_INET6) {
        Ok(dump) => darwin::ipv6_default_gateway(&dump, &resolve),
        Err(error) if error.raw_os_error() == Some(libc::EAFNOSUPPORT) => None,
        Err(error) => return Err(error.into()),
    };
    Ok(DefaultGateways {
        v4: darwin::ipv4_default_gateway(&v4),
        v6,
    })
}

#[cfg(target_os = "windows")]
fn detect_native() -> crate::Result<DefaultGateways> {
    use crate::process::{self, timeouts};

    let captured = process::run_with_timeout("route", &["print"], timeouts::WINDOWS_DEFAULT_ROUTE)
        .map_err(|error| std::io::Error::other(format!("`route print` failed: {error}")))?;
    if !captured.status.success() {
        return Err(std::io::Error::other(format!(
            "`route print` exited with {}",
            captured.status
        ))
        .into());
    }
    Ok(windows::default_gateways(&String::from_utf8_lossy(
        &captured.stdout,
    )))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn detect_native() -> crate::Result<DefaultGateways> {
    Ok(DefaultGateways::default())
}

/// The best route seen so far: lowest metric, first one wins a tie.
#[cfg(any(target_os = "linux", target_os = "windows", test))]
struct LowestMetric<G> {
    best: Option<(u32, Option<G>)>,
}

#[cfg(any(target_os = "linux", target_os = "windows", test))]
impl<G> LowestMetric<G> {
    fn new() -> Self {
        Self { best: None }
    }

    fn offer(&mut self, metric: u32, gateway: Option<G>) {
        if self.best.as_ref().is_none_or(|(best, _)| metric < *best) {
            self.best = Some((metric, gateway));
        }
    }

    fn gateway(self) -> Option<G> {
        self.best.and_then(|(_, gateway)| gateway)
    }
}

/// An IPv6 gateway, zoned with `scope` when it is link-local.
fn scoped_ipv6_gateway(address: Ipv6Addr, scope: impl FnOnce() -> String) -> Option<ScopedIpAddr> {
    if address.is_unspecified() {
        return None;
    }
    if address.is_unicast_link_local() {
        ScopedIpAddr::with_scope(address, scope()).ok()
    } else {
        Some(ScopedIpAddr::new(address.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_both_families_with_the_zone_and_round_trips() {
        let gateways = DefaultGateways {
            v4: Some(Ipv4Addr::new(192, 168, 1, 1)),
            v6: Some("fe80::1%en0".parse().unwrap()),
        };
        let json = serde_json::to_string(&gateways).unwrap();
        assert_eq!(json, r#"{"v4":"192.168.1.1","v6":"fe80::1%en0"}"#);
        let decoded: DefaultGateways = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, gateways);
        assert_eq!(serde_json::to_string(&decoded).unwrap(), json);

        let none = serde_json::to_string(&DefaultGateways::default()).unwrap();
        assert_eq!(none, r#"{"v4":null,"v6":null}"#);
    }

    #[test]
    fn a_later_route_with_an_equal_metric_does_not_replace_the_first() {
        let mut best = LowestMetric::new();
        best.offer(10, Some(1));
        best.offer(10, Some(2));
        best.offer(11, Some(3));
        assert_eq!(best.gateway(), Some(1));
    }

    #[test]
    fn a_better_on_link_route_hides_a_worse_gateway_route() {
        let mut best = LowestMetric::new();
        best.offer(100, Some(1));
        best.offer(5, None);
        assert_eq!(best.gateway(), None::<i32>);
    }

    #[test]
    fn real_detect_default_gateways_reads_the_host_table() {
        // Host-dependent values; the contract here is only that the native
        // reader succeeds and yields spellings the parsers accept.
        let gateways = detect_default_gateways().expect("routing table is readable");
        if let Some(v6) = &gateways.v6 {
            assert_eq!(v6.to_string().parse::<ScopedIpAddr>().unwrap(), *v6);
        }
    }
}
