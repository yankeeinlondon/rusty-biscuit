# Network primitives

Focused APIs under `sniff::network` for scoped addresses, default gateways, and
ICMP reachability. They are separate from `detect_network_with_request` on
purpose: that function is widely consumed (GitNexus HIGH), and the Windows
gateway read costs a `route print` subprocess that ordinary interface detection
should not pay.

## Addresses — `ScopedIpAddr`

- One spelling for interface addresses, gateways, and ICMP targets: an IPv4
  literal, an IPv6 literal, or `ipv6%zone`. `FromStr` is strict (no DNS, no
  brackets, no whitespace, no zone on IPv4); `Display` and serde round-trip the
  normalized address with the zone kept verbatim.
- `is_within(&IpNet)` compares address bits only. The zone is ignored, and a
  network of the other family never matches (`::ffff:100.64.0.1` is not CGNAT).
- `host_addresses(&interfaces)` deduplicates by address and zone and sorts
  (IPv4 before IPv6, numeric, then zone). Only IPv6 link-local addresses get a
  zone: the interface **name** on Unix (`fe80::1%en0`), the numeric interface
  **index** on Windows (`fe80::1%12`) — each platform's native spelling. Loopback
  and link-local are included; filtering belongs to the caller.
- `NetworkInterface.index` carries the OS interface index for that purpose.
- `contains_cgnat_address` / `cgnat_network()` implement `ctx.tailnet`
  (`100.64.0.0/10`; CGNAT ISP and other VPN false positives are accepted).

## Gateways — `detect_default_gateways() -> Result<DefaultGateways>`

- Primary = lowest-metric UP default route, first wins a tie (Linux, Windows);
  first **unscoped** UP default in the `NET_RT_DUMP` sysctl (macOS). macOS lists
  per-interface `RTF_IFSCOPE` defaults (VPN `utun*`, `bridge*`) that are never
  the system default, and `route get default` agrees with skipping them.
- The best route decides. If it is on-link (Linux gateway `0`, Windows `On-link`,
  macOS `link#N`), the gateway is `None` even when a worse route has one.
- A VPN's `0.0.0.0/1` split route is not a default (mask must be zero).
- Windows `route print` is parsed structurally so localized headings work:
  `=` separator blocks, route-list blocks start with a `…:` heading and
  alternate active/persistent. Persistent routes are never in effect.
- The macOS parser reads by byte offset, not `libc::rt_msghdr`, so fixtures run
  everywhere; `darwin::tests::layout_matches_libc` pins the offsets on macOS.
  Sockaddrs round to 4 bytes on Darwin (not `sizeof(long)`), and link-local
  gateways embed their zone in address bytes 2..4 (KAME).
- Errors only when the IPv4 table is unreadable; a missing IPv6 table is `None`.

## ICMP — `network::icmp`

- `ping(target, ProbeBudget)` → `PingReport { outcomes }` with `verdict()`
  `AllReplied | NoneReplied | Unstable`. No reply (including unreachable
  targets: `EHOSTUNREACH`/`ENETUNREACH`/`EHOSTDOWN`, Windows `IP_DEST_*`) is an
  outcome; `IcmpError` means the host could not send and aborts the series even
  after earlier replies.
- `ProbeBudget::from_millis(f64, f64)` range-checks before converting (no
  truncation): timeout in `(0, 60000]` ms, attempts a whole number `1..=100`.
  `deadline_bound()` = total + 500 ms setup allowance.
- `ping_with(&mut dyn EchoProbe, …)` is the deterministic seam; it counts a
  reply only if both the reported round trip and measured elapsed time are
  below the per-attempt timeout (a reply exactly at the threshold is late).
- Transports: Unix `socket2` `SOCK_DGRAM` ICMP/ICMPv6 (Linux requires the
  process group inside `net.ipv4.ping_group_range`, else `NotPermitted` with
  that hint; macOS delivers the IPv4 header, Linux does not, and Linux rewrites
  the echo identifier — replies match on sequence + random payload token).
  Windows `IcmpSendEcho2`/`Icmp6SendEcho2` (no privilege, no subprocess).
- A scoped target's zone resolves numerically or via `if_nametoindex`; an
  unknown name fails with `UnknownScope` before any packet is sent.
- Real round trips live in `lib/tests/l1/network_primitives.rs` as `real_` tests
  (`just test-real`); they fail with the missing capability instead of skipping.
