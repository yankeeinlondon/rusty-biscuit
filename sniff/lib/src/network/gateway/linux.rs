//! `/proc/net/route` and `/proc/net/ipv6_route` parsers.

use std::net::{Ipv4Addr, Ipv6Addr};

use super::{LowestMetric, scoped_ipv6_gateway};
use crate::network::ScopedIpAddr;

// `include/uapi/linux/route.h`; the IPv6 table reuses the same low bits.
const RTF_UP: u32 = 0x0001;
const RTF_GATEWAY: u32 = 0x0002;
const RTF_REJECT: u32 = 0x0200;

/// Primary IPv4 default gateway from `/proc/net/route`.
///
/// A default route has destination and mask `00000000`; a VPN's `0.0.0.0/1`
/// split route is not one.
pub(super) fn ipv4_default_gateway(table: &str) -> Option<Ipv4Addr> {
    let mut best = LowestMetric::new();
    for line in table.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 8 || cols[1] != "00000000" || cols[7] != "00000000" {
            continue;
        }
        let (Ok(flags), Ok(metric), Ok(gateway)) = (
            u32::from_str_radix(cols[3], 16),
            cols[6].parse::<u32>(),
            u32::from_str_radix(cols[2], 16),
        ) else {
            continue;
        };
        if flags & RTF_UP == 0 || flags & RTF_REJECT != 0 {
            continue;
        }
        // The kernel prints the network-order word as a host-order integer, so
        // the native byte order recovers the address bytes.
        let gateway = Ipv4Addr::from(gateway.to_ne_bytes());
        let gateway = (flags & RTF_GATEWAY != 0 && !gateway.is_unspecified()).then_some(gateway);
        best.offer(metric, gateway);
    }
    best.gateway()
}

/// Primary IPv6 default gateway from `/proc/net/ipv6_route`, zoned with the
/// route's device when link-local.
///
/// Source-specific default routes (non-zero source prefix) are not the host's
/// default and are skipped, as is the kernel's reject route on `lo`.
pub(super) fn ipv6_default_gateway(table: &str) -> Option<ScopedIpAddr> {
    const UNSPECIFIED: &str = "00000000000000000000000000000000";

    let mut best = LowestMetric::new();
    for line in table.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 10 || cols[0] != UNSPECIFIED || cols[1] != "00" || cols[3] != "00" {
            continue;
        }
        let (Ok(nexthop), Ok(metric), Ok(flags)) = (
            u128::from_str_radix(cols[4], 16),
            u32::from_str_radix(cols[5], 16),
            u32::from_str_radix(cols[8], 16),
        ) else {
            continue;
        };
        if flags & RTF_UP == 0 || flags & RTF_REJECT != 0 {
            continue;
        }
        let device = cols[9];
        let gateway = (flags & RTF_GATEWAY != 0)
            .then(|| scoped_ipv6_gateway(Ipv6Addr::from(nexthop), || device.to_string()))
            .flatten();
        best.offer(metric, gateway);
    }
    best.gateway()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTE_V4: &str = include_str!("fixtures/linux_route_v4.txt");
    const ROUTE_V4_ON_LINK: &str = include_str!("fixtures/linux_route_v4_on_link.txt");
    const ROUTE_V4_NO_DEFAULT: &str = include_str!("fixtures/linux_route_v4_no_default.txt");
    const ROUTE_V6: &str = include_str!("fixtures/linux_ipv6_route.txt");
    const ROUTE_V6_ON_LINK: &str = include_str!("fixtures/linux_ipv6_route_on_link.txt");

    #[test]
    fn ipv4_picks_the_lowest_metric_up_default_route() {
        // Also present: a lower-metric DOWN default, a metric-0 `0.0.0.0/1`
        // split route, and a worse `wlan0` default.
        assert_eq!(
            ipv4_default_gateway(ROUTE_V4),
            Some(Ipv4Addr::new(192, 168, 1, 1))
        );
    }

    #[test]
    fn ipv4_on_link_default_route_has_no_gateway() {
        // `ppp0` is the best default and has gateway `00000000`; the worse
        // `eth0` route with a gateway must not be chosen instead.
        assert_eq!(ipv4_default_gateway(ROUTE_V4_ON_LINK), None);
    }

    #[test]
    fn ipv4_without_a_default_route_has_no_gateway() {
        assert_eq!(ipv4_default_gateway(ROUTE_V4_NO_DEFAULT), None);
        assert_eq!(ipv4_default_gateway(""), None);
    }

    #[test]
    fn ipv6_link_local_gateway_keeps_the_device_scope() {
        // Also present: the kernel's `lo` reject default and a worse `wlan0`
        // default.
        assert_eq!(
            ipv6_default_gateway(ROUTE_V6).map(|g| g.to_string()),
            Some("fe80::1%eth0".to_string())
        );
    }

    #[test]
    fn ipv6_on_link_default_route_has_no_gateway() {
        assert_eq!(ipv6_default_gateway(ROUTE_V6_ON_LINK), None);
    }

    #[test]
    fn ipv6_global_gateway_is_unscoped() {
        let table = "00000000000000000000000000000000 00 00000000000000000000000000000000 00 20010db8000000000000000000000001 00000064 00000001 00000000 00000003     eth1\n";
        assert_eq!(
            ipv6_default_gateway(table).map(|g| g.to_string()),
            Some("2001:db8::1".to_string())
        );
    }

    #[test]
    fn ipv6_without_a_default_route_has_no_gateway() {
        let table = "fe800000000000000000000000000000 40 00000000000000000000000000000000 00 00000000000000000000000000000000 00000100 00000001 00000000 00000001     eth0\n";
        assert_eq!(ipv6_default_gateway(table), None);
        assert_eq!(ipv6_default_gateway(""), None);
    }

    #[test]
    fn malformed_rows_are_skipped() {
        let v4 = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\n\
                  eth0\t00000000\tZZZZZZZZ\t0003\t0\t0\t1\t00000000\n\
                  eth1\t00000000\t0101A8C0\t0003\t0\t0\n\
                  eth2\t00000000\t0201A8C0\t0003\t0\t0\t9\t00000000\n";
        assert_eq!(ipv4_default_gateway(v4), Some(Ipv4Addr::new(192, 168, 1, 2)));
    }
}
