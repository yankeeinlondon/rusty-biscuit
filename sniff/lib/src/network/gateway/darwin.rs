//! macOS `PF_ROUTE` `NET_RT_DUMP` parser.
//!
//! The dump is read by byte offset rather than through `libc::rt_msghdr`, so
//! fixtures parse on every host. `tests::layout_matches_libc` pins the offsets
//! against the real structure on macOS.

use std::net::{Ipv4Addr, Ipv6Addr};

use super::scoped_ipv6_gateway;
use crate::network::ScopedIpAddr;

/// `sizeof(struct rt_msghdr)`: fixed fields plus the 56-byte `rt_metrics`.
const HEADER_LEN: usize = 92;
const OFFSET_VERSION: usize = 2;
const OFFSET_INDEX: usize = 4;
const OFFSET_FLAGS: usize = 8;
const OFFSET_ADDRS: usize = 12;

const RTM_VERSION: u8 = 5;
const RTF_UP: i32 = 0x1;
const RTF_GATEWAY: i32 = 0x2;
const RTF_REJECT: i32 = 0x8;
const RTF_BLACKHOLE: i32 = 0x1000;
const RTF_IFSCOPE: i32 = 0x100_0000;

const RTAX_DST: usize = 0;
const RTAX_GATEWAY: usize = 1;
const RTAX_NETMASK: usize = 2;
const RTAX_MAX: usize = 8;

const AF_INET: u8 = 2;
const AF_INET6: u8 = 30;

/// `sockaddr_in.sin_addr` and `sockaddr_in6.sin6_addr` offsets.
const IPV4_ADDRESS_OFFSET: usize = 4;
const IPV6_ADDRESS_OFFSET: usize = 8;
const IPV6_SCOPE_OFFSET: usize = 24;

/// Gateway of the first unscoped UP IPv4 default route.
pub(super) fn ipv4_default_gateway(dump: &[u8]) -> Option<Ipv4Addr> {
    let route = first_default_route(dump, AF_INET)?;
    let gateway = route.gateway_sockaddr(AF_INET)?;
    let octets: [u8; 4] = gateway
        .get(IPV4_ADDRESS_OFFSET..IPV4_ADDRESS_OFFSET + 4)?
        .try_into()
        .ok()?;
    Some(Ipv4Addr::from(octets)).filter(|gateway| !gateway.is_unspecified())
}

/// Gateway of the first unscoped UP IPv6 default route.
///
/// A link-local gateway is zoned with the interface name `name_for_index`
/// returns, or the numeric index when the name is unknown.
pub(super) fn ipv6_default_gateway(
    dump: &[u8],
    name_for_index: &dyn Fn(u16) -> Option<String>,
) -> Option<ScopedIpAddr> {
    let route = first_default_route(dump, AF_INET6)?;
    let gateway = route.gateway_sockaddr(AF_INET6)?;
    let mut octets: [u8; 16] = gateway
        .get(IPV6_ADDRESS_OFFSET..IPV6_ADDRESS_OFFSET + 16)?
        .try_into()
        .ok()?;
    let scope_id = gateway
        .get(IPV6_SCOPE_OFFSET..IPV6_SCOPE_OFFSET + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_ne_bytes)
        .unwrap_or(0);

    // The kernel embeds a link-local address's zone in bytes 2..4 (the KAME
    // convention) and may leave `sin6_scope_id` zero.
    let mut index = 0_u32;
    let address = Ipv6Addr::from(octets);
    if address.is_unicast_link_local() {
        let embedded = u16::from_be_bytes([octets[2], octets[3]]);
        octets[2] = 0;
        octets[3] = 0;
        index = [u32::from(embedded), scope_id, u32::from(route.index)]
            .into_iter()
            .find(|candidate| *candidate != 0)
            .unwrap_or(0);
    }
    scoped_ipv6_gateway(Ipv6Addr::from(octets), || {
        u16::try_from(index)
            .ok()
            .and_then(name_for_index)
            .unwrap_or_else(|| index.to_string())
    })
}

struct RouteMessage<'a> {
    flags: i32,
    index: u16,
    sockaddrs: [Option<&'a [u8]>; RTAX_MAX],
}

impl<'a> RouteMessage<'a> {
    fn gateway_sockaddr(&self, family: u8) -> Option<&'a [u8]> {
        if self.flags & RTF_GATEWAY == 0 {
            return None;
        }
        self.sockaddrs[RTAX_GATEWAY].filter(|sockaddr| sockaddr.get(1) == Some(&family))
    }
}

fn first_default_route(dump: &[u8], family: u8) -> Option<RouteMessage<'_>> {
    let address_offset = match family {
        AF_INET => IPV4_ADDRESS_OFFSET,
        _ => IPV6_ADDRESS_OFFSET,
    };
    let address_len = match family {
        AF_INET => 4,
        _ => 16,
    };
    let is_zero_from = |sockaddr: &[u8], start: usize, end: usize| {
        sockaddr
            .iter()
            .take(end)
            .skip(start)
            .all(|byte| *byte == 0)
    };

    messages(dump).find(|route| {
        if route.flags & RTF_UP == 0
            || route.flags & (RTF_REJECT | RTF_BLACKHOLE | RTF_IFSCOPE) != 0
        {
            return false;
        }
        let Some(destination) = route.sockaddrs[RTAX_DST] else {
            return false;
        };
        if destination.get(1) != Some(&family)
            || !is_zero_from(destination, address_offset, address_offset + address_len)
        {
            return false;
        }
        route.sockaddrs[RTAX_NETMASK]
            .is_none_or(|netmask| is_zero_from(netmask, address_offset, netmask.len()))
    })
}

fn messages(dump: &[u8]) -> impl Iterator<Item = RouteMessage<'_>> {
    let mut offset = 0;
    std::iter::from_fn(move || {
        loop {
            let header = dump.get(offset..offset + HEADER_LEN)?;
            let message_len = usize::from(u16::from_ne_bytes([header[0], header[1]]));
            if message_len < HEADER_LEN {
                return None;
            }
            let message = dump.get(offset..offset + message_len)?;
            offset += message_len;
            if message[OFFSET_VERSION] != RTM_VERSION {
                continue;
            }

            let read_i32 = |at: usize| {
                i32::from_ne_bytes([message[at], message[at + 1], message[at + 2], message[at + 3]])
            };
            let addrs = read_i32(OFFSET_ADDRS);
            let mut sockaddrs = [None; RTAX_MAX];
            let mut cursor = HEADER_LEN;
            for (slot, sockaddr) in sockaddrs.iter_mut().enumerate() {
                if addrs & (1 << slot) == 0 {
                    continue;
                }
                let Some(&len) = message.get(cursor) else {
                    break;
                };
                let len = usize::from(len);
                *sockaddr = Some(&message[cursor..(cursor + len).min(message.len())]);
                // Darwin rounds each sockaddr up to 4 bytes; a zero length
                // occupies 4.
                cursor += if len == 0 { 4 } else { (len + 3) & !3 };
            }

            return Some(RouteMessage {
                flags: read_i32(OFFSET_FLAGS),
                index: u16::from_ne_bytes([message[OFFSET_INDEX], message[OFFSET_INDEX + 1]]),
                sockaddrs,
            });
        }
    })
}

/// Reads the kernel routing table for one address family.
#[cfg(target_os = "macos")]
pub(super) fn route_dump(family: libc::c_int) -> std::io::Result<Vec<u8>> {
    let mut mib = [
        libc::CTL_NET,
        libc::PF_ROUTE,
        0,
        family,
        libc::NET_RT_DUMP,
        0,
    ];
    // The table can grow between the size query and the read.
    for _ in 0..4 {
        let mut len: libc::size_t = 0;
        // SAFETY: a null buffer asks the kernel only for the required length.
        let status = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as libc::c_uint,
                std::ptr::null_mut(),
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if status != 0 {
            return Err(std::io::Error::last_os_error());
        }
        if len == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = vec![0_u8; len + len / 4];
        let mut filled = buffer.len();
        // SAFETY: `buffer` is writable for `filled` bytes and outlives the call.
        let status = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as libc::c_uint,
                buffer.as_mut_ptr().cast(),
                &mut filled,
                std::ptr::null_mut(),
                0,
            )
        };
        if status == 0 {
            buffer.truncate(filled);
            return Ok(buffer);
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ENOMEM) {
            return Err(error);
        }
    }
    Err(std::io::Error::from_raw_os_error(libc::ENOMEM))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AF_LINK: u8 = 18;
    const RTF_STATIC: i32 = 0x800;

    fn pad(mut sockaddr: Vec<u8>) -> Vec<u8> {
        let padded = if sockaddr.is_empty() {
            4
        } else {
            (sockaddr.len() + 3) & !3
        };
        sockaddr.resize(padded, 0);
        sockaddr
    }

    fn inet(octets: [u8; 4]) -> Vec<u8> {
        let mut sockaddr = vec![0_u8; 16];
        sockaddr[0] = 16;
        sockaddr[1] = AF_INET;
        sockaddr[4..8].copy_from_slice(&octets);
        sockaddr
    }

    fn inet6(address: &str, scope_id: u32) -> Vec<u8> {
        let mut sockaddr = vec![0_u8; 28];
        sockaddr[0] = 28;
        sockaddr[1] = AF_INET6;
        sockaddr[8..24].copy_from_slice(&address.parse::<Ipv6Addr>().unwrap().octets());
        sockaddr[24..28].copy_from_slice(&scope_id.to_ne_bytes());
        sockaddr
    }

    /// `link#N`: an on-link gateway.
    fn link(index: u16) -> Vec<u8> {
        let mut sockaddr = vec![0_u8; 20];
        sockaddr[0] = 20;
        sockaddr[1] = AF_LINK;
        sockaddr[2..4].copy_from_slice(&index.to_ne_bytes());
        sockaddr
    }

    /// A netmask of `prefix` bits: Darwin trims trailing zero bytes, so
    /// `/0` has length zero.
    fn netmask(family: u8, prefix: u8) -> Vec<u8> {
        let offset = if family == AF_INET { 4 } else { 8 };
        let mut bytes = vec![0_u8; offset];
        bytes[1] = family;
        let mut remaining = prefix;
        while remaining > 0 {
            let take = remaining.min(8);
            bytes.push(0xff_u8 << (8 - take));
            remaining -= take;
        }
        if prefix == 0 {
            return Vec::new();
        }
        bytes[0] = bytes.len() as u8;
        bytes
    }

    fn message(flags: i32, index: u16, sockaddrs: &[(usize, Vec<u8>)]) -> Vec<u8> {
        let mut body = Vec::new();
        let mut addrs = 0_i32;
        for (slot, sockaddr) in sockaddrs {
            addrs |= 1 << slot;
            body.extend(pad(sockaddr.clone()));
        }
        let mut header = vec![0_u8; HEADER_LEN];
        let total = u16::try_from(HEADER_LEN + body.len()).unwrap();
        header[0..2].copy_from_slice(&total.to_ne_bytes());
        header[OFFSET_VERSION] = RTM_VERSION;
        header[3] = 4; // RTM_GET
        header[OFFSET_INDEX..OFFSET_INDEX + 2].copy_from_slice(&index.to_ne_bytes());
        header[OFFSET_FLAGS..OFFSET_FLAGS + 4].copy_from_slice(&flags.to_ne_bytes());
        header[OFFSET_ADDRS..OFFSET_ADDRS + 4].copy_from_slice(&addrs.to_ne_bytes());
        header.extend(body);
        header
    }

    fn route(flags: i32, index: u16, dst: Vec<u8>, gateway: Vec<u8>, mask: Vec<u8>) -> Vec<u8> {
        message(
            flags,
            index,
            &[(RTAX_DST, dst), (RTAX_GATEWAY, gateway), (RTAX_NETMASK, mask)],
        )
    }

    fn dump(messages: &[Vec<u8>]) -> Vec<u8> {
        messages.concat()
    }

    const UG: i32 = RTF_UP | RTF_GATEWAY | RTF_STATIC;

    fn names(index: u16) -> Option<String> {
        match index {
            4 => Some("en0".to_string()),
            20 => Some("utun3".to_string()),
            _ => None,
        }
    }

    #[test]
    fn ipv4_picks_the_first_unscoped_up_default_route() {
        let table = dump(&[
            // default link#34 UCSIg bridge100 (interface-scoped, on-link)
            route(RTF_UP | RTF_IFSCOPE, 34, inet([0; 4]), link(34), netmask(AF_INET, 0)),
            // 192.168.10/24 link#4
            route(RTF_UP, 4, inet([192, 168, 10, 0]), link(4), netmask(AF_INET, 24)),
            // 0/1 via a VPN: a split route, not a default
            route(UG, 20, inet([0; 4]), inet([10, 8, 0, 1]), netmask(AF_INET, 1)),
            // a DOWN default and a reject default
            route(RTF_GATEWAY, 4, inet([0; 4]), inet([10, 9, 9, 9]), netmask(AF_INET, 0)),
            route(UG | RTF_REJECT, 4, inet([0; 4]), inet([10, 9, 9, 8]), netmask(AF_INET, 0)),
            // default 192.168.10.1 UGScg en0
            route(UG, 4, inet([0; 4]), inet([192, 168, 10, 1]), netmask(AF_INET, 0)),
            route(UG, 5, inet([0; 4]), inet([10, 0, 0, 1]), netmask(AF_INET, 0)),
        ]);
        assert_eq!(
            ipv4_default_gateway(&table),
            Some(Ipv4Addr::new(192, 168, 10, 1))
        );
    }

    #[test]
    fn ipv4_on_link_default_route_has_no_gateway() {
        let table = dump(&[
            // default link#20 UCS utun3: a point-to-point VPN default
            route(RTF_UP | RTF_STATIC, 20, inet([0; 4]), link(20), netmask(AF_INET, 0)),
            route(UG, 4, inet([0; 4]), inet([192, 168, 10, 1]), netmask(AF_INET, 0)),
        ]);
        assert_eq!(ipv4_default_gateway(&table), None);
    }

    #[test]
    fn ipv4_without_a_default_route_has_no_gateway() {
        let table = dump(&[
            route(RTF_UP, 4, inet([192, 168, 10, 0]), link(4), netmask(AF_INET, 24)),
            route(RTF_UP, 1, inet([127, 0, 0, 0]), inet([127, 0, 0, 1]), netmask(AF_INET, 8)),
        ]);
        assert_eq!(ipv4_default_gateway(&table), None);
        assert_eq!(ipv4_default_gateway(&[]), None);
    }

    #[test]
    fn a_default_route_without_a_netmask_sockaddr_is_still_default() {
        let table = message(
            UG,
            4,
            &[(RTAX_DST, inet([0; 4])), (RTAX_GATEWAY, inet([172, 16, 0, 1]))],
        );
        assert_eq!(
            ipv4_default_gateway(&table),
            Some(Ipv4Addr::new(172, 16, 0, 1))
        );
    }

    #[test]
    fn ipv6_link_local_gateway_keeps_its_embedded_scope() {
        // KAME: zone 4 embedded as fe80:0004::1, sin6_scope_id left zero.
        let table = dump(&[
            // default fe80::%utun0 UGcIg utun0 (interface-scoped)
            route(UG | RTF_IFSCOPE, 20, inet6("::", 0), inet6("fe80:14::1", 0), netmask(AF_INET6, 0)),
            route(UG, 9, inet6("::", 0), inet6("fe80:4::1", 0), netmask(AF_INET6, 0)),
        ]);
        assert_eq!(
            ipv6_default_gateway(&table, &names).map(|g| g.to_string()),
            Some("fe80::1%en0".to_string())
        );
    }

    #[test]
    fn ipv6_scope_falls_back_to_scope_id_then_route_index_then_number() {
        let scope_id = dump(&[route(UG, 9, inet6("::", 0), inet6("fe80::1", 20), netmask(AF_INET6, 0))]);
        assert_eq!(
            ipv6_default_gateway(&scope_id, &names).map(|g| g.to_string()),
            Some("fe80::1%utun3".to_string())
        );

        let route_index = dump(&[route(UG, 4, inet6("::", 0), inet6("fe80::1", 0), netmask(AF_INET6, 0))]);
        assert_eq!(
            ipv6_default_gateway(&route_index, &names).map(|g| g.to_string()),
            Some("fe80::1%en0".to_string())
        );

        let unnamed = dump(&[route(UG, 77, inet6("::", 0), inet6("fe80::1", 0), netmask(AF_INET6, 0))]);
        assert_eq!(
            ipv6_default_gateway(&unnamed, &names).map(|g| g.to_string()),
            Some("fe80::1%77".to_string())
        );
    }

    #[test]
    fn ipv6_global_gateway_is_unscoped() {
        let table = dump(&[route(UG, 4, inet6("::", 0), inet6("2001:db8::1", 0), netmask(AF_INET6, 0))]);
        assert_eq!(
            ipv6_default_gateway(&table, &names).map(|g| g.to_string()),
            Some("2001:db8::1".to_string())
        );
    }

    #[test]
    fn ipv6_on_link_or_missing_default_route_has_no_gateway() {
        let on_link = dump(&[
            route(RTF_UP | RTF_STATIC, 20, inet6("::", 0), link(20), netmask(AF_INET6, 0)),
            route(UG, 4, inet6("::", 0), inet6("fe80:4::1", 0), netmask(AF_INET6, 0)),
        ]);
        assert_eq!(ipv6_default_gateway(&on_link, &names), None);

        let no_default = dump(&[
            route(RTF_UP, 1, inet6("::1", 0), link(1), netmask(AF_INET6, 128)),
            route(UG, 4, inet6("2000::", 0), inet6("fe80:4::1", 0), netmask(AF_INET6, 3)),
        ]);
        assert_eq!(ipv6_default_gateway(&no_default, &names), None);
    }

    #[test]
    fn a_family_mismatch_is_not_a_default_route() {
        let v4_table = dump(&[route(UG, 4, inet([0; 4]), inet([192, 168, 10, 1]), netmask(AF_INET, 0))]);
        assert_eq!(ipv6_default_gateway(&v4_table, &names), None);
        let v6_table = dump(&[route(UG, 4, inet6("::", 0), inet6("2001:db8::1", 0), netmask(AF_INET6, 0))]);
        assert_eq!(ipv4_default_gateway(&v6_table), None);
    }

    #[test]
    fn truncated_or_foreign_messages_never_panic_or_match() {
        let good = route(UG, 4, inet([0; 4]), inet([192, 168, 10, 1]), netmask(AF_INET, 0));

        for cut in 0..good.len() {
            assert_eq!(ipv4_default_gateway(&good[..cut]), None, "cut at {cut}");
        }

        let mut wrong_version = good.clone();
        wrong_version[OFFSET_VERSION] = 4;
        assert_eq!(ipv4_default_gateway(&wrong_version), None);
        // A skipped message does not stop the scan.
        assert_eq!(
            ipv4_default_gateway(&dump(&[wrong_version, good.clone()])),
            Some(Ipv4Addr::new(192, 168, 10, 1))
        );

        let mut zero_length = good.clone();
        zero_length[0..2].copy_from_slice(&0_u16.to_ne_bytes());
        assert_eq!(ipv4_default_gateway(&zero_length), None);

        // The header claims every sockaddr, but the body is empty.
        let mut empty_body = good[..HEADER_LEN].to_vec();
        empty_body[0..2].copy_from_slice(&(HEADER_LEN as u16).to_ne_bytes());
        empty_body[OFFSET_ADDRS..OFFSET_ADDRS + 4].copy_from_slice(&0xff_i32.to_ne_bytes());
        assert_eq!(ipv4_default_gateway(&empty_body), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn layout_matches_libc() {
        use std::mem::{offset_of, size_of};
        assert_eq!(size_of::<libc::rt_msghdr>(), HEADER_LEN);
        assert_eq!(offset_of!(libc::rt_msghdr, rtm_version), OFFSET_VERSION);
        assert_eq!(offset_of!(libc::rt_msghdr, rtm_index), OFFSET_INDEX);
        assert_eq!(offset_of!(libc::rt_msghdr, rtm_flags), OFFSET_FLAGS);
        assert_eq!(offset_of!(libc::rt_msghdr, rtm_addrs), OFFSET_ADDRS);
        assert_eq!(libc::RTM_VERSION, i32::from(RTM_VERSION));
        assert_eq!(libc::RTF_UP, RTF_UP);
        assert_eq!(libc::RTF_GATEWAY, RTF_GATEWAY);
        assert_eq!(libc::RTF_REJECT, RTF_REJECT);
        assert_eq!(libc::RTF_BLACKHOLE, RTF_BLACKHOLE);
        assert_eq!(libc::RTF_IFSCOPE, RTF_IFSCOPE);
        assert_eq!(libc::RTAX_DST as usize, RTAX_DST);
        assert_eq!(libc::RTAX_GATEWAY as usize, RTAX_GATEWAY);
        assert_eq!(libc::RTAX_NETMASK as usize, RTAX_NETMASK);
        assert_eq!(libc::RTAX_MAX as usize, RTAX_MAX);
        assert_eq!(libc::AF_INET, i32::from(AF_INET));
        assert_eq!(libc::AF_INET6, i32::from(AF_INET6));
        assert_eq!(offset_of!(libc::sockaddr_in, sin_addr), IPV4_ADDRESS_OFFSET);
        assert_eq!(offset_of!(libc::sockaddr_in6, sin6_addr), IPV6_ADDRESS_OFFSET);
        assert_eq!(offset_of!(libc::sockaddr_in6, sin6_scope_id), IPV6_SCOPE_OFFSET);
    }

    /// Cross-checks the parser against the system's own `route get`.
    #[cfg(target_os = "macos")]
    #[test]
    fn real_native_gateways_match_route_get() {
        fn route_get(family_flag: &str) -> Option<String> {
            let output = std::process::Command::new("/sbin/route")
                .args(["-n", "get", family_flag, "default"])
                .stdin(std::process::Stdio::null())
                .output()
                .ok()?;
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .find_map(|line| line.trim().strip_prefix("gateway:").map(|g| g.trim().to_string()))
        }

        let gateways = super::super::detect_default_gateways().unwrap();
        let expected_v4 = route_get("-inet").and_then(|g| g.parse::<Ipv4Addr>().ok());
        assert_eq!(gateways.v4, expected_v4);
        let expected_v6 = route_get("-inet6").and_then(|g| g.parse::<ScopedIpAddr>().ok());
        assert_eq!(gateways.v6, expected_v6);
    }
}
