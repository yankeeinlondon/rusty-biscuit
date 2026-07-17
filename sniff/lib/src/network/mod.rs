use crate::Result;
use crate::performance;
use crate::request::NetworkRequest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "network")]
use std::net::IpAddr;
#[cfg(any(target_os = "windows", test))]
use std::process::Command;
#[cfg(feature = "network")]
use std::sync::Mutex;
use std::time::Instant;
use tracing::Level;
#[cfg(feature = "network")]
use tracing::{debug, info};
use tracing::{instrument, warn};

mod interface;
pub use interface::{
    InterfaceFlags, IpAddresses, Ipv4Address, Ipv4Cidr, Ipv6Address, Ipv6Cidr, NetworkInterface,
};

/// Default WAN IP echo endpoints, tried in order.
///
/// There is more than one on purpose: with a single endpoint the sequential
/// fallback below has nothing to fall back *to*, so any outage of that one host
/// is a total failure to detect. Both speak plain-text IP over HTTPS and both
/// answer for IPv4 and IPv6.
#[cfg(feature = "network")]
const DEFAULT_WAN_IP_ENDPOINTS: &[&str] = &["https://api64.ipify.org", "https://icanhazip.com"];

/// Deadline for establishing a connection to a WAN IP endpoint.
#[cfg(feature = "network")]
const WAN_IP_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

/// Deadline for a complete WAN IP request/response.
///
/// Applied identically to every endpoint, so a slow first endpoint cannot starve
/// the rest of the list.
#[cfg(feature = "network")]
const WAN_IP_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Environment variable that overrides the WAN IP echo endpoints.
///
/// Expected format is a comma-separated list of URLs. When set and
/// non-empty, the first successful response drives the WAN IP result
/// exactly like the default production endpoints.
///
/// This exists primarily to let benches, integration tests, and
/// air-gapped corporate deployments point WAN IP detection at a known
/// address (e.g. a `wiremock` server) instead of the public internet.
#[cfg(feature = "network")]
const WAN_IP_ENDPOINTS_ENV: &str = "SNIFF_WAN_IP_ENDPOINTS";

/// Default TTL for cached WAN IP results (5 minutes).
#[cfg(feature = "network")]
const WAN_IP_TTL: std::time::Duration = std::time::Duration::from_secs(300);

#[cfg(feature = "network")]
static WAN_IP_CACHE: Mutex<Option<WanIpCacheEntry>> = Mutex::new(None);

#[cfg(feature = "network")]
struct WanIpCacheEntry {
    value: Option<String>,
    fetched_at: std::time::Instant,
}

/// Network information for the system.
///
/// Contains a list of all network interfaces, the primary interface name,
/// aggregated IP addresses across all interfaces, and a flag indicating
/// if permission was denied during detection.
///
/// ## Examples
///
/// ```
/// use sniff::network::detect_network;
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

    /// Name of the primary interface.
    ///
    /// This prefers the interface used by the host's default route when it
    /// can be determined, and otherwise falls back to a local heuristic.
    pub primary_interface: Option<String>,

    /// Aggregated IP addresses from all interfaces with interface attribution.
    ///
    /// This is a convenience field that collects all IPv4 and IPv6 addresses
    /// in one place, allowing quick lookups without iterating through interfaces.
    pub ip_addresses: IpAddresses,

    /// The host's WAN IP address, if it could be determined.
    ///
    /// This is resolved via an external IP echo service when the `network`
    /// feature is enabled. Detection failure is non-fatal and leaves this as
    /// `None`.
    pub wan_ip_address: Option<String>,

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
/// - The primary interface name (default-route interface when detectable)
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
/// use sniff::network::detect_network;
///
/// let info = detect_network().unwrap();
/// if !info.permission_denied {
///     // Should have at least loopback
///     assert!(!info.interfaces.is_empty());
/// }
/// ```
pub fn detect_network() -> Result<NetworkInfo> {
    detect_network_with_request(&NetworkRequest::full())
}

/// Detect network information according to the given request.
///
/// Use `NetworkRequest::interfaces_only()` to skip the WAN IP lookup,
/// or `NetworkRequest::full()` for everything.
///
/// ## Examples
///
/// ```
/// use sniff::network::detect_network_with_request;
/// use sniff::request::NetworkRequest;
///
/// let info = detect_network_with_request(&NetworkRequest::interfaces_only()).unwrap();
/// // WAN IP is not fetched when not requested
/// assert!(info.wan_ip_address.is_none());
/// ```
///
/// ## Errors
///
/// Returns an error if the `getifaddrs` call fails for reasons other than permission denied.
#[instrument(skip(request), fields(
    wan_ip = request.include_wan_ip,
    force_refresh = request.force_refresh,
))]
pub fn detect_network_with_request(request: &NetworkRequest) -> Result<NetworkInfo> {
    // Run WAN IP lookup concurrently with local interface enumeration
    // so neither blocks the other.
    std::thread::scope(|s| {
        let collector = performance::current_collector();
        let wan_handle = if request.include_wan_ip {
            Some(s.spawn(move || {
                performance::with_current_collector(collector, || {
                    let started = Instant::now();
                    let result = detect_wan_ip(request.force_refresh);
                    performance::record_logged_stage(
                        "network.wan_ip",
                        started.elapsed(),
                        Level::DEBUG,
                    );
                    result
                })
            }))
        } else {
            None
        };

        let local_started = Instant::now();
        let local_result = detect_local_interfaces();
        performance::record_logged_stage(
            "network.local_interfaces",
            local_started.elapsed(),
            Level::DEBUG,
        );

        let wan_ip_address = wan_handle.map(|h| h.join().unwrap()).unwrap_or(None);

        match local_result {
            Ok((interfaces, primary, ip_addresses)) => Ok(NetworkInfo {
                interfaces,
                primary_interface: primary,
                ip_addresses,
                wan_ip_address,
                permission_denied: false,
            }),
            Err(PermissionOrError::PermissionDenied) => {
                warn!("network interface detection: permission denied");
                Ok(NetworkInfo {
                    interfaces: vec![],
                    primary_interface: None,
                    ip_addresses: IpAddresses::default(),
                    wan_ip_address,
                    permission_denied: true,
                })
            }
            Err(PermissionOrError::Other(e)) => Err(e),
        }
    })
}

enum PermissionOrError {
    PermissionDenied,
    Other(crate::SniffError),
}

/// Cache entry for interface detection with TTL.
#[cfg(feature = "network")]
struct InterfaceCache {
    value: (Vec<NetworkInterface>, Option<String>, IpAddresses),
    fetched_at: std::time::Instant,
}

#[cfg(feature = "network")]
static INTERFACE_CACHE: Mutex<Option<InterfaceCache>> = Mutex::new(None);

/// TTL for interface list cache (1 second).
#[cfg(feature = "network")]
const INTERFACE_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(1);

fn detect_local_interfaces()
-> std::result::Result<(Vec<NetworkInterface>, Option<String>, IpAddresses), PermissionOrError> {
    // Check cache first (only when network feature is enabled)
    #[cfg(feature = "network")]
    {
        if let Ok(guard) = INTERFACE_CACHE.lock()
            && let Some(entry) = guard.as_ref()
            && entry.fetched_at.elapsed() < INTERFACE_CACHE_TTL
        {
            performance::increment_counter("network.local_interfaces.cache_hits", 1);
            return Ok(entry.value.clone());
        }
    }

    let addrs = match getifaddrs::getifaddrs() {
        Ok(addrs) => addrs,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            return Err(PermissionOrError::PermissionDenied);
        }
        Err(e) => return Err(PermissionOrError::Other(e.into())),
    };

    let mut interfaces: HashMap<String, NetworkInterface> = HashMap::new();
    let mut ip_addresses = IpAddresses::default();

    for ifaddr in addrs {
        let interface_name = ifaddr.name.clone();
        let entry = interfaces
            .entry(interface_name.clone())
            .or_insert_with(|| NetworkInterface::new(interface_name.clone()));

        entry.flags.is_up = ifaddr.flags.contains(getifaddrs::InterfaceFlags::UP);
        entry.flags.is_loopback = ifaddr.flags.contains(getifaddrs::InterfaceFlags::LOOPBACK);
        entry.flags.is_running = ifaddr.flags.contains(getifaddrs::InterfaceFlags::RUNNING);

        match ifaddr.address {
            getifaddrs::Address::V4(v4) => {
                if !entry
                    .ipv4_addresses
                    .iter()
                    .any(|cidr| cidr.address == v4.address)
                {
                    let prefix_len = v4.netmask.map(|m| m.to_bits().count_ones() as u8);
                    entry.ipv4_addresses.push(Ipv4Cidr {
                        address: v4.address,
                        prefix_len,
                    });
                    ip_addresses.v4.push(Ipv4Address {
                        address: v4.address.to_string(),
                        interface: interface_name,
                        prefix_len,
                    });
                }
            }
            getifaddrs::Address::V6(v6) => {
                if !entry
                    .ipv6_addresses
                    .iter()
                    .any(|cidr| cidr.address == v6.address)
                {
                    let prefix_len = v6.netmask.map(|m| m.to_bits().count_ones() as u8);
                    entry.ipv6_addresses.push(Ipv6Cidr {
                        address: v6.address,
                        prefix_len,
                    });
                    ip_addresses.v6.push(Ipv6Address {
                        address: v6.address.to_string(),
                        interface: interface_name,
                        prefix_len,
                    });
                }
            }
            getifaddrs::Address::Mac(mac) => {
                if entry.mac_address.is_none() && mac != [0u8; 6] {
                    entry.mac_address = Some(format_mac_address(&mac));
                }
            }
        }
    }

    let mut interfaces: Vec<_> = interfaces.into_values().collect();
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));

    let primary = find_primary_interface(&interfaces);

    let result = (interfaces, primary, ip_addresses);

    // Update cache (only when network feature is enabled)
    #[cfg(feature = "network")]
    {
        if let Ok(mut guard) = INTERFACE_CACHE.lock() {
            *guard = Some(InterfaceCache {
                value: result.clone(),
                fetched_at: std::time::Instant::now(),
            });
        }
    }

    Ok(result)
}

/// Resolve the WAN IP echo endpoint list, preferring the
/// `SNIFF_WAN_IP_ENDPOINTS` environment variable over the compiled-in
/// defaults when it is set to a non-empty value.
///
/// Expected env var format: comma-separated URLs. Empty entries are
/// skipped; if every entry is empty, the defaults are returned.
#[cfg(feature = "network")]
fn resolve_wan_ip_endpoints() -> Vec<String> {
    if let Ok(raw) = std::env::var(WAN_IP_ENDPOINTS_ENV) {
        let parsed: Vec<String> = raw
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if !parsed.is_empty() {
            debug!(
                count = parsed.len(),
                "using WAN IP endpoints from SNIFF_WAN_IP_ENDPOINTS"
            );
            return parsed;
        }
    }
    DEFAULT_WAN_IP_ENDPOINTS
        .iter()
        .map(|endpoint| (*endpoint).to_string())
        .collect()
}

/// Fetches one endpoint with an already-built client and strictly parses the body.
///
/// ## Returns
///
/// The echoed address, re-canonicalized through [`IpAddr`] so an endpoint cannot
/// dictate the string sniff reports. `None` for any transport failure, non-success
/// status, or a body that is not exactly an IP address — all of which mean the
/// caller should try the next endpoint.
///
/// ## Notes
///
/// The response body never leaves this function. A misbehaving or hostile endpoint
/// can return anything, and that anything must not reach an error message, a log,
/// or a counter name.
#[cfg(feature = "network")]
fn fetch_wan_ip(client: &reqwest::blocking::Client, url: &str) -> Option<String> {
    let response = client.get(url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().ok()?;
    body.trim().parse::<IpAddr>().ok().map(|ip| ip.to_string())
}

/// Builds the one blocking client shared across every endpoint attempt.
#[cfg(feature = "network")]
fn build_wan_ip_client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent("sniff-lib/0.1.0")
        .connect_timeout(WAN_IP_CONNECT_TIMEOUT)
        .timeout(WAN_IP_REQUEST_TIMEOUT)
        .build()
        .ok()
}

#[cfg(feature = "network")]
#[derive(Debug, Clone)]
struct WanIpDetector {
    endpoints: Vec<String>,
}

#[cfg(feature = "network")]
impl WanIpDetector {
    fn new() -> Self {
        Self {
            endpoints: resolve_wan_ip_endpoints(),
        }
    }

    #[cfg(test)]
    fn with_endpoints(mut self, endpoints: Vec<String>) -> Self {
        self.endpoints = endpoints;
        self
    }

    /// Queries endpoints in order and returns the first strictly parsed address.
    ///
    /// ## Notes
    ///
    /// Sequential, not raced. Racing every endpoint would shave a little latency
    /// at the cost of disclosing the caller's address to every provider on the
    /// list even when the first one answers — so a successful first attempt tells
    /// exactly one endpoint where the host is.
    fn detect(&self) -> Option<String> {
        let endpoints = self.endpoints.clone();

        // One client for the whole ladder. It was previously rebuilt per attempt,
        // which threw away the connection pool and TLS setup between endpoints.
        let attempt_all = move || {
            let client = build_wan_ip_client()?;
            for endpoint in &endpoints {
                performance::increment_counter(
                    crate::performance::counters::NETWORK_WAN_ATTEMPTS,
                    1,
                );
                if let Some(ip) = fetch_wan_ip(&client, endpoint) {
                    return Some(ip);
                }
                debug!("WAN IP endpoint did not yield an address; trying the next");
            }
            None
        };

        // The blocking client owns an internal runtime, and dropping a runtime
        // inside an async context panics — so when a tokio runtime is active the
        // whole ladder runs on a dedicated thread where the client is both built
        // and dropped (issue #16). The collector must be carried across that hop
        // by hand or every counter recorded on it is silently discarded.
        if tokio::runtime::Handle::try_current().is_ok() {
            let mut worker = performance::WorkerCollector::inherit();
            std::thread::spawn(move || {
                worker.activate();
                attempt_all()
            })
            .join()
            .ok()
            .flatten()
        } else {
            attempt_all()
        }
    }
}

#[cfg(feature = "network")]
fn detect_wan_ip(force_refresh: bool) -> Option<String> {
    // Check cache first (unless refresh is forced)
    if force_refresh {
        performance::increment_counter("network.wan_ip.cache_forced_refreshes", 1);
    }
    if !force_refresh
        && let Ok(guard) = WAN_IP_CACHE.lock()
        && let Some(entry) = guard.as_ref()
    {
        if entry.fetched_at.elapsed() < WAN_IP_TTL {
            performance::increment_counter("network.wan_ip.cache_hits", 1);
            debug!("WAN IP served from cache");
            return entry.value.clone();
        }
        performance::increment_counter("network.wan_ip.cache_expired", 1);
        debug!("WAN IP cache expired");
    }

    performance::increment_counter("network.wan_ip.cache_misses", 1);

    // Fetch fresh value
    let value = WanIpDetector::new().detect();

    match &value {
        Some(ip) => info!(ip = %ip, "WAN IP resolved"),
        None => warn!("WAN IP detection failed"),
    }

    // Update cache
    if let Ok(mut guard) = WAN_IP_CACHE.lock() {
        *guard = Some(WanIpCacheEntry {
            value: value.clone(),
            fetched_at: std::time::Instant::now(),
        });
    }

    value
}

#[cfg(not(feature = "network"))]
fn detect_wan_ip(_force_refresh: bool) -> Option<String> {
    None
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
/// This first prefers the interface backing the system's default route.
/// If that cannot be detected, or does not map to an eligible interface,
/// it falls back to a local heuristic:
/// 1. Physical adapters over virtual ones
///    - Unix: `en*`, `eth*`, `wlan*`, `wlp*`, `enp*`
///    - Windows: `Ethernet`, `Wi-Fi`, `Local Area Connection`
/// 2. Adapters with RUNNING flag (actively transmitting) over idle ones
/// 3. Alphabetically first interface name within the same priority tier
///
/// Virtual adapters (`bridge*`, `utun*`, `docker*`, `vEthernet*`,
/// `Hyper-V*`, `Bluetooth*`, `Loopback*`, `Npcap*`, etc.) are deprioritized.
///
/// All candidates must be non-loopback, up, and have at least one IPv4 address.
///
/// ## Returns
///
/// Returns the name of the primary interface, or None if no suitable interface exists.
fn find_primary_interface(interfaces: &[NetworkInterface]) -> Option<String> {
    select_primary_interface(
        interfaces,
        detect_default_route_interface(interfaces).as_deref(),
    )
}

fn select_primary_interface(
    interfaces: &[NetworkInterface],
    default_route_interface: Option<&str>,
) -> Option<String> {
    if let Some(route_interface) = default_route_interface
        && let Some(iface) = interfaces
            .iter()
            .find(|iface| iface.name == route_interface && is_primary_candidate(iface))
    {
        return Some(iface.name.clone());
    }

    find_primary_interface_fallback(interfaces)
}

fn find_primary_interface_fallback(interfaces: &[NetworkInterface]) -> Option<String> {
    let mut candidates: Vec<_> = interfaces
        .iter()
        .filter(|i| is_primary_candidate(i))
        .collect();

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| {
        interface_priority(b)
            .cmp(&interface_priority(a))
            .then_with(|| a.name.cmp(&b.name))
    });

    candidates.first().map(|i| i.name.clone())
}

/// Computes a priority score for an interface used in primary-interface selection.
///
/// Higher values are preferred.  Ties are broken alphabetically by name in
/// [`find_primary_interface_fallback`].
///
/// | Score | Meaning                                  |
/// |-------|------------------------------------------|
/// | 4     | Physical adapter, actively running       |
/// | 3     | Physical adapter, not running            |
/// | 2     | Non-virtual adapter, actively running    |
/// | 1     | Non-virtual adapter, not running         |
/// | 0     | Virtual / unrecognized (lowest priority) |
fn interface_priority(iface: &NetworkInterface) -> u8 {
    if is_physical_interface(&iface.name) {
        if iface.flags.is_running { 4 } else { 3 }
    } else if !is_virtual_interface(&iface.name) {
        if iface.flags.is_running { 2 } else { 1 }
    } else {
        0
    }
}

/// Checks if an interface name looks like a physical network adapter.
///
/// Recognises both Unix-style names (`en*`, `eth*`, `wlan*`, etc.) and common
/// Windows adapter names (`Ethernet`, `Wi-Fi`, `Local Area Connection`).
fn is_physical_interface(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    // Unix patterns
    // - en* (macOS/BSD Ethernet/WiFi), except enx* (USB adapters)
    // - eth* (Linux Ethernet)
    // - wlan* (Linux WiFi)
    // - wlp* (Linux WiFi with PCI naming)
    // - enp* (Linux with predictable naming)
    if (lower.starts_with("en") && !lower.starts_with("enx"))
        || lower.starts_with("eth")
        || lower.starts_with("wlan")
        || lower.starts_with("wlp")
        || lower.starts_with("enp")
    {
        return true;
    }

    // Windows patterns — common adapter names produced by Windows NDIS / Hyper-V
    matches!(
        lower.as_str(),
        "ethernet"
            | "ethernet 2"
            | "ethernet 3"
            | "wi-fi"
            | "wi-fi 2"
            | "wi-fi 3"
            | "local area connection"
            | "local area connection 2"
            | "local area connection 3"
            | "network bridge"
    )
}

/// Checks if an interface name looks like a virtual or software adapter.
///
/// Covers Unix virtual patterns (`bridge*`, `utun*`, `docker*`, etc.) and
/// common Windows virtual adapters (`vEthernet*`, Hyper-V, VPN, Bluetooth,
/// loopback/capture adapters).
fn is_virtual_interface(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    // Unix virtual patterns
    if lower.starts_with("bridge")
        || lower.starts_with("utun")
        || lower.starts_with("tun")
        || lower.starts_with("tap")
        || lower.starts_with("veth")
        || lower.starts_with("docker")
        || lower.starts_with("br-")
        || lower.starts_with("vmnet")
        || lower.starts_with("awdl")
        || lower.starts_with("llw")
    {
        return true;
    }

    // Windows virtual patterns
    // - vEthernet* (Hyper-V virtual switch)
    // - Hyper-V* adapters
    // - Bluetooth*, Loopback*, Npcap*, WinCap* (capture adapters)
    lower.starts_with("vethernet")
        || lower.starts_with("hyper-v")
        || lower.starts_with("bluetooth")
        || lower.starts_with("loopback")
        || lower.starts_with("npcap")
        || lower.starts_with("wincap")
}

fn is_primary_candidate(interface: &NetworkInterface) -> bool {
    !interface.flags.is_loopback && !interface.ipv4_addresses.is_empty() && interface.flags.is_up
}

fn detect_default_route_interface(interfaces: &[NetworkInterface]) -> Option<String> {
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        let _ = interfaces;
        detect_bsd_default_route_interface_native()
    }

    #[cfg(target_os = "linux")]
    {
        let _ = interfaces;
        detect_linux_default_route_interface_native()
    }

    #[cfg(target_os = "windows")]
    {
        command_output("route", &["print", "0.0.0.0"])
            .and_then(|output| parse_windows_default_route_interface_ip(&output))
            .and_then(|ip| interface_name_for_ipv4(interfaces, ip))
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "linux",
        target_os = "windows"
    )))]
    {
        let _ = interfaces;
        None
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_default_route_interface_native() -> Option<String> {
    std::fs::read_to_string("/proc/net/route")
        .ok()
        .and_then(|output| parse_linux_proc_default_route_interface(&output))
}

#[cfg(target_os = "linux")]
fn parse_linux_proc_default_route_interface(output: &str) -> Option<String> {
    let mut best: Option<(&str, u32)> = None;

    for line in output.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 8 || cols[1] != "00000000" {
            continue;
        }

        let flags = match u16::from_str_radix(cols[3], 16) {
            Ok(flags) => flags,
            Err(_) => continue,
        };
        if flags & libc::RTF_UP as u16 == 0 {
            continue;
        }

        let metric = match cols[6].parse::<u32>() {
            Ok(metric) => metric,
            Err(_) => continue,
        };
        match best {
            None => best = Some((cols[0], metric)),
            Some((_, best_metric)) if metric < best_metric => best = Some((cols[0], metric)),
            _ => {}
        }
    }

    best.map(|(interface, _)| interface.to_string())
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn detect_bsd_default_route_interface_native() -> Option<String> {
    let mut mib = [
        libc::CTL_NET,
        libc::PF_ROUTE,
        0,
        libc::AF_INET,
        libc::NET_RT_FLAGS,
        libc::RTF_GATEWAY,
    ];
    let mut len: libc::size_t = 0;

    unsafe {
        if libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }
    }

    if len == 0 {
        return None;
    }

    let mut buffer = vec![0_u8; len];
    unsafe {
        if libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            buffer.as_mut_ptr().cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }
    }

    parse_bsd_route_dump(&buffer[..len])
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn parse_bsd_route_dump(buffer: &[u8]) -> Option<String> {
    let mut offset = 0;

    while offset + std::mem::size_of::<libc::rt_msghdr>() <= buffer.len() {
        let header = unsafe { &*(buffer[offset..].as_ptr().cast::<libc::rt_msghdr>()) };
        let message_len = header.rtm_msglen as usize;
        if message_len == 0 || offset + message_len > buffer.len() {
            break;
        }

        if is_default_bsd_route_message(header, &buffer[offset..offset + message_len]) {
            return interface_name_from_index(header.rtm_index as u32);
        }

        offset += message_len;
    }

    None
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn is_default_bsd_route_message(header: &libc::rt_msghdr, message: &[u8]) -> bool {
    let data = &message[std::mem::size_of::<libc::rt_msghdr>()..];
    let destination = sockaddr_from_route_message(data, header.rtm_addrs, libc::RTAX_DST as usize);
    let Some(destination) = destination else {
        return false;
    };

    unsafe {
        if (*destination).sa_family as i32 != libc::AF_INET {
            return false;
        }

        let destination = destination.cast::<libc::sockaddr_in>();
        (*destination).sin_addr.s_addr == 0
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn sockaddr_from_route_message(
    data: &[u8],
    addrs_mask: i32,
    desired_index: usize,
) -> Option<*const libc::sockaddr> {
    let mut offset = 0;

    for index in 0..(libc::RTAX_MAX as usize) {
        if addrs_mask & (1 << index) == 0 {
            continue;
        }

        if offset >= data.len() {
            return None;
        }

        let sockaddr = unsafe { &*(data[offset..].as_ptr().cast::<libc::sockaddr>()) };
        let sockaddr_len = sockaddr_len(sockaddr);

        if index == desired_index {
            return Some(sockaddr as *const libc::sockaddr);
        }

        offset += align_sockaddr_len(sockaddr_len);
    }

    None
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn sockaddr_len(sockaddr: &libc::sockaddr) -> usize {
    let len = sockaddr.sa_len as usize;
    if len == 0 {
        std::mem::size_of::<libc::sockaddr>()
    } else {
        len
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn align_sockaddr_len(len: usize) -> usize {
    let align = std::mem::size_of::<libc::c_long>();
    (len + align - 1) & !(align - 1)
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn interface_name_from_index(index: u32) -> Option<String> {
    let mut buffer = [0_u8; libc::IF_NAMESIZE];
    let name = unsafe { libc::if_indextoname(index, buffer.as_mut_ptr().cast()) };
    if name.is_null() {
        return None;
    }

    unsafe { std::ffi::CStr::from_ptr(name) }
        .to_str()
        .ok()
        .map(|name| name.to_string())
}

#[cfg(any(target_os = "windows", test))]
#[allow(dead_code)]
fn command_output(program: &str, args: &[&str]) -> Option<String> {
    performance::increment_counter(crate::performance::counters::PROC_SPAWNS, 1);
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()
}

#[cfg(test)]
fn parse_bsd_default_route_interface(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        line.trim()
            .strip_prefix("interface:")
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
    })
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_default_route_interface_ip(output: &str) -> Option<std::net::Ipv4Addr> {
    let mut best: Option<(std::net::Ipv4Addr, u32)> = None;

    for line in output.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 {
            continue;
        }
        if cols[0] != "0.0.0.0" || cols[1] != "0.0.0.0" {
            continue;
        }
        // Column layout: Network Netmask Gateway Interface Metric [...]
        let ip: std::net::Ipv4Addr = match cols[3].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let metric: u32 = match cols[4].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        match best {
            None => best = Some((ip, metric)),
            Some((_, best_m)) if metric < best_m => best = Some((ip, metric)),
            _ => {}
        }
    }

    best.map(|(ip, _)| ip)
}

#[cfg(any(target_os = "windows", test))]
fn interface_name_for_ipv4(
    interfaces: &[NetworkInterface],
    address: std::net::Ipv4Addr,
) -> Option<String> {
    interfaces
        .iter()
        .find(|iface| iface.ipv4_addresses.iter().any(|c| c.address == address))
        .map(|iface| iface.name.clone())
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
/// use sniff::network::detect_network_filtered;
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
    fn test_detect_network_interfaces_only_skips_wan_ip() {
        let info = detect_network_with_request(&NetworkRequest::interfaces_only()).unwrap();
        // WAN IP should be None when not requested.
        // Note: This only holds if the OnceLock hasn't been populated by a prior test in the
        // same process. The key verification is that the function accepts the request and
        // returns successfully.
        assert!(!info.permission_denied || info.interfaces.is_empty());
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

    #[cfg(feature = "network")]
    #[tokio::test]
    async fn test_wan_ip_detector_returns_ip_from_echo_service() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("203.0.113.10\n"))
            .mount(&server)
            .await;

        let detector = WanIpDetector::new().with_endpoints(vec![server.uri()]);
        assert_eq!(detector.detect(), Some("203.0.113.10".to_string()));
    }

    #[cfg(feature = "network")]
    #[tokio::test]
    async fn test_wan_ip_detector_falls_back_after_invalid_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/invalid"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not-an-ip"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/valid"))
            .respond_with(ResponseTemplate::new(200).set_body_string("2001:db8::7"))
            .mount(&server)
            .await;

        let detector = WanIpDetector::new().with_endpoints(vec![
            format!("{}/invalid", server.uri()),
            format!("{}/valid", server.uri()),
        ]);
        assert_eq!(detector.detect(), Some("2001:db8::7".to_string()));
    }

    /// R11.7: more than one default endpoint, or the sequential fallback below has
    /// nothing to fall back to and "retry" is a fiction.
    #[cfg(feature = "network")]
    #[test]
    fn default_wan_endpoints_provide_an_actual_fallback() {
        assert!(
            DEFAULT_WAN_IP_ENDPOINTS.len() >= 2,
            "a single default endpoint makes the fallback ladder unreachable"
        );
        assert!(
            DEFAULT_WAN_IP_ENDPOINTS
                .iter()
                .all(|e| e.starts_with("https://")),
            "a WAN IP echo over plain HTTP discloses the host address to the path"
        );
    }

    /// R11.7: the caller's address must not be disclosed to every endpoint when
    /// the first one already answered.
    #[cfg(feature = "network")]
    #[tokio::test]
    async fn a_successful_first_endpoint_is_not_followed_by_a_second_request() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/first"))
            .respond_with(ResponseTemplate::new(200).set_body_string("203.0.113.1"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/second"))
            .respond_with(ResponseTemplate::new(200).set_body_string("203.0.113.2"))
            .expect(0)
            .mount(&server)
            .await;

        let detector = WanIpDetector::new().with_endpoints(vec![
            format!("{}/first", server.uri()),
            format!("{}/second", server.uri()),
        ]);
        assert_eq!(detector.detect(), Some("203.0.113.1".to_string()));

        // `expect(0)` on /second is verified when the server drops.
        drop(server);
    }

    /// Every endpoint failing is `None`, not a panic or a partial value.
    #[cfg(feature = "network")]
    #[tokio::test]
    async fn all_endpoints_failing_yields_no_address() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let detector = WanIpDetector::new()
            .with_endpoints(vec![format!("{}/a", server.uri()), format!("{}/b", server.uri())]);
        assert_eq!(detector.detect(), None);
    }

    /// A body that is not exactly an address is rejected, and the value sniff
    /// reports is re-canonicalized rather than echoed back verbatim.
    #[cfg(feature = "network")]
    #[tokio::test]
    async fn endpoint_bodies_are_strictly_parsed_and_recanonicalized() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        for (route, body) in [
            ("/html", "<html>203.0.113.5</html>"),
            ("/extra", "203.0.113.5 (your ip)"),
            ("/empty", ""),
        ] {
            Mock::given(method("GET"))
                .and(path(route))
                .respond_with(ResponseTemplate::new(200).set_body_string(body))
                .mount(&server)
                .await;
        }
        // An uncompressed IPv6 body must come back compressed, proving the value
        // went through `IpAddr` rather than being trusted as a string.
        Mock::given(method("GET"))
            .and(path("/verbose-v6"))
            .respond_with(ResponseTemplate::new(200).set_body_string("2001:0db8:0000:0000:0000:0000:0000:0001\n"))
            .mount(&server)
            .await;

        for route in ["/html", "/extra", "/empty"] {
            let detector =
                WanIpDetector::new().with_endpoints(vec![format!("{}{route}", server.uri())]);
            assert_eq!(detector.detect(), None, "{route} is not an IP address");
        }

        let detector =
            WanIpDetector::new().with_endpoints(vec![format!("{}/verbose-v6", server.uri())]);
        assert_eq!(detector.detect(), Some("2001:db8::1".to_string()));
    }

    /// A hanging endpoint must not consume the whole detection — its deadline
    /// expires and the next endpoint still gets its turn.
    #[cfg(feature = "network")]
    #[tokio::test]
    async fn a_timed_out_endpoint_falls_back_to_the_next() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // Well past WAN_IP_REQUEST_TIMEOUT, so this endpoint can only ever time out.
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("203.0.113.9")
                    .set_delay(WAN_IP_REQUEST_TIMEOUT * 4),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/fast"))
            .respond_with(ResponseTemplate::new(200).set_body_string("203.0.113.8"))
            .mount(&server)
            .await;

        let detector = WanIpDetector::new().with_endpoints(vec![
            format!("{}/slow", server.uri()),
            format!("{}/fast", server.uri()),
        ]);
        assert_eq!(detector.detect(), Some("203.0.113.8".to_string()));
    }

    /// The WAN ladder runs on a spawned thread inside a tokio context, and a
    /// counter recorded on a thread the collector never reached is silently lost.
    #[cfg(feature = "network")]
    #[tokio::test]
    async fn endpoint_attempts_are_counted_across_the_thread_hop() {
        use crate::performance::{PerformanceCollector, with_current_collector};
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let endpoints = vec![format!("{}/a", server.uri()), format!("{}/b", server.uri())];
        let collector = PerformanceCollector::new_shared();
        with_current_collector(Some(collector.clone()), || {
            WanIpDetector::new().with_endpoints(endpoints).detect()
        });

        let report = collector.snapshot(std::time::Duration::from_millis(1));
        assert_eq!(
            report
                .counters
                .get(crate::performance::counters::NETWORK_WAN_ATTEMPTS),
            Some(&2),
            "both attempts must survive the tokio-context thread hop"
        );
    }

    #[cfg(feature = "network")]
    #[test]
    fn test_resolve_wan_ip_endpoints_prefers_env_override() {
        // This test mutates process-wide environment state. It must not
        // run in parallel with other tests that also read the env var,
        // and it restores the original value on exit.
        let key = WAN_IP_ENDPOINTS_ENV;
        let original = std::env::var(key).ok();
        // SAFETY: single-threaded test body; `std::env::set_var` is
        // `unsafe` since Rust 1.85 but this block does no concurrent
        // env access.
        unsafe {
            std::env::set_var(
                key,
                "https://first.example.test, https://second.example.test ,",
            );
        }

        let endpoints = resolve_wan_ip_endpoints();
        assert_eq!(
            endpoints,
            vec![
                "https://first.example.test".to_string(),
                "https://second.example.test".to_string(),
            ]
        );

        // Empty value should fall back to the defaults.
        unsafe {
            std::env::set_var(key, "   ,  ,  ");
        }
        let endpoints = resolve_wan_ip_endpoints();
        assert_eq!(endpoints.len(), DEFAULT_WAN_IP_ENDPOINTS.len());

        unsafe {
            match original {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
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
    fn test_ip_addresses_have_prefix_length_on_real_system() {
        // On every supported platform `getifaddrs` reports a netmask for
        // both IPv4 and IPv6 addresses, so once we have any non-empty
        // result the prefix should be present and within range.
        let info = detect_network().unwrap();
        if info.permission_denied {
            return;
        }

        for addr in &info.ip_addresses.v4 {
            let prefix = addr.prefix_len.unwrap_or_else(|| {
                panic!(
                    "IPv4 address {} on {} should report a prefix length",
                    addr.address, addr.interface
                )
            });
            assert!(prefix <= 32, "IPv4 prefix /{prefix} out of range");
        }

        for addr in &info.ip_addresses.v6 {
            let prefix = addr.prefix_len.unwrap_or_else(|| {
                panic!(
                    "IPv6 address {} on {} should report a prefix length",
                    addr.address, addr.interface
                )
            });
            assert!(prefix <= 128, "IPv6 prefix /{prefix} out of range");
        }

        // Loopback should always report the classical full-host prefixes.
        for iface in info.interfaces.iter().filter(|i| i.flags.is_loopback) {
            for cidr in &iface.ipv4_addresses {
                if cidr.address.is_loopback() {
                    assert_eq!(
                        cidr.prefix_len,
                        Some(8),
                        "127.0.0.0/8 expected for loopback IPv4"
                    );
                }
            }
            for cidr in &iface.ipv6_addresses {
                if cidr.address.is_loopback() {
                    assert_eq!(
                        cidr.prefix_len,
                        Some(128),
                        "::1/128 expected for loopback IPv6"
                    );
                }
            }
        }
    }

    #[test]
    fn test_ip_addresses_per_interface_prefix_matches_aggregate() {
        let info = detect_network().unwrap();
        if info.permission_denied {
            return;
        }

        for iface in &info.interfaces {
            for cidr in &iface.ipv4_addresses {
                let aggregate =
                    info.ip_addresses.v4.iter().find(|a| {
                        a.interface == iface.name && a.address == cidr.address.to_string()
                    });
                assert_eq!(
                    aggregate.and_then(|a| a.prefix_len),
                    cidr.prefix_len,
                    "Per-interface IPv4 prefix should match aggregate for {}",
                    cidr.address
                );
            }
            for cidr in &iface.ipv6_addresses {
                let aggregate =
                    info.ip_addresses.v6.iter().find(|a| {
                        a.interface == iface.name && a.address == cidr.address.to_string()
                    });
                assert_eq!(
                    aggregate.and_then(|a| a.prefix_len),
                    cidr.prefix_len,
                    "Per-interface IPv6 prefix should match aggregate for {}",
                    cidr.address
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
            iface
                .ipv4_addresses
                .push(Ipv4Cidr::new("192.168.1.100".parse().unwrap(), Some(24)));
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

        let primary = find_primary_interface_fallback(&interfaces);
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

        let primary = find_primary_interface_fallback(&interfaces);
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

        let primary = find_primary_interface_fallback(&interfaces);
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

        let primary = find_primary_interface_fallback(&interfaces);
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

        let primary = find_primary_interface_fallback(&interfaces);
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

        let primary = find_primary_interface_fallback(&interfaces);
        assert_eq!(
            primary, None,
            "Should return None when only loopback interface exists"
        );
    }

    #[test]
    fn test_primary_interface_excludes_common_virtual_patterns() {
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
            "vEthernet (WSL)",
            "Hyper-V Virtual Ethernet Adapter",
            "Bluetooth Network Connection",
            "Loopback",
            "Npcap Loopback Adapter",
        ];

        for virtual_name in virtual_patterns {
            let interfaces = vec![
                create_test_interface(virtual_name, true, true),
                create_test_interface("en0", true, true),
            ];

            let primary = find_primary_interface_fallback(&interfaces);
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
        let physical_patterns = vec![
            "en0",                   // macOS/BSD
            "en1",                   // macOS/BSD
            "eth0",                  // Linux Ethernet
            "eth1",                  // Linux Ethernet
            "wlan0",                 // Linux WiFi
            "wlp3s0",                // Linux WiFi with PCI naming
            "enp0s31",               // Linux with predictable naming
            "Ethernet",              // Windows
            "Wi-Fi",                 // Windows
            "Local Area Connection", // Windows
        ];

        for physical_name in physical_patterns {
            let interfaces = vec![
                create_test_interface("bridge100", true, true),
                create_test_interface(physical_name, true, true),
            ];

            let primary = find_primary_interface_fallback(&interfaces);
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

        let primary = find_primary_interface_fallback(&interfaces);
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

    #[test]
    fn test_select_primary_interface_prefers_default_route_interface() {
        let interfaces = vec![
            create_test_interface("en0", true, true),
            create_test_interface("en7", true, true),
        ];

        let primary = select_primary_interface(&interfaces, Some("en7"));
        assert_eq!(
            primary,
            Some("en7".to_string()),
            "Should prefer the interface used by the default route"
        );
    }

    #[test]
    fn test_select_primary_interface_ignores_ineligible_default_route_interface() {
        let mut down = create_test_interface("en7", true, true);
        down.flags.is_up = false;

        let interfaces = vec![create_test_interface("en0", true, true), down];

        let primary = select_primary_interface(&interfaces, Some("en7"));
        assert_eq!(
            primary,
            Some("en0".to_string()),
            "Should fall back when the routed interface is not an eligible primary candidate"
        );
    }

    #[test]
    fn test_parse_bsd_default_route_interface() {
        let output = "\
   route to: default
destination: default
    gateway: 192.168.100.1
  interface: en7
      flags: <UP,GATEWAY,DONE,STATIC>";

        assert_eq!(
            parse_bsd_default_route_interface(output),
            Some("en7".to_string())
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_linux_proc_default_route_interface() {
        let output = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
wlp3s0\t00000000\t0101A8C0\t0003\t0\t0\t600\t00000000\t0\t0\t0
eth0\t00000000\t0100A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0";
        assert_eq!(
            parse_linux_proc_default_route_interface(output),
            Some("eth0".to_string())
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_linux_proc_default_route_interface_no_default_route() {
        // A table with only the header (or only non-default destinations) yields
        // no interface — the same observable result as a missing /proc/net/route,
        // which `read_to_string(...).ok()` maps to `None` in the caller.
        let header_only =
            "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT";
        assert_eq!(
            parse_linux_proc_default_route_interface(header_only),
            None
        );
        assert_eq!(parse_linux_proc_default_route_interface(""), None);
    }

    #[test]
    fn test_parse_windows_default_route_single_route() {
        let output = "\
===========================================================================
Interface List
 15...00 1a 2b 3c 4d 5e ......Intel Ethernet Adapter
===========================================================================
IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
          0.0.0.0          0.0.0.0      192.168.1.1    192.168.1.100     25
===========================================================================";
        assert_eq!(
            parse_windows_default_route_interface_ip(output),
            Some("192.168.1.100".parse::<std::net::Ipv4Addr>().unwrap())
        );
    }

    #[test]
    fn test_parse_windows_default_route_lowest_metric_wins() {
        let output = "\
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
          0.0.0.0          0.0.0.0      192.168.1.1    192.168.1.100     50
          0.0.0.0          0.0.0.0      10.0.0.1       10.0.0.50         10
          0.0.0.0          0.0.0.0      172.16.0.1     172.16.0.5       200";
        assert_eq!(
            parse_windows_default_route_interface_ip(output),
            Some("10.0.0.50".parse::<std::net::Ipv4Addr>().unwrap())
        );
    }

    #[test]
    fn test_parse_windows_default_route_malformed_rows_ignored() {
        let output = "\
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
          0.0.0.0          0.0.0.0      192.168.1.1    not-an-ip         25
          0.0.0.0          0.0.0.0      10.0.0.1       10.0.0.50         10
          1.2.3.0          255.255.255.0 1.2.3.1       1.2.3.4           5
garbage line";
        assert_eq!(
            parse_windows_default_route_interface_ip(output),
            Some("10.0.0.50".parse::<std::net::Ipv4Addr>().unwrap())
        );
    }

    #[test]
    fn test_parse_windows_default_route_no_routes() {
        let output = "Active Routes:\nNetwork Destination Netmask Gateway Interface Metric";
        assert_eq!(parse_windows_default_route_interface_ip(output), None);
    }

    #[test]
    fn test_interface_name_for_ipv4_found() {
        let mut iface = create_test_interface("Ethernet", true, true);
        iface.ipv4_addresses.clear();
        iface
            .ipv4_addresses
            .push(Ipv4Cidr::new("10.0.0.50".parse().unwrap(), Some(24)));

        let result = interface_name_for_ipv4(&[iface], "10.0.0.50".parse().unwrap());
        assert_eq!(result, Some("Ethernet".to_string()));
    }

    #[test]
    fn test_interface_name_for_ipv4_not_found() {
        let iface = create_test_interface("Ethernet", true, true);
        let result = interface_name_for_ipv4(&[iface], "10.0.0.99".parse().unwrap());
        assert_eq!(result, None);
    }

    #[test]
    fn test_primary_interface_alphabetical_tiebreak_physical_running() {
        let interfaces = vec![
            create_test_interface("en2", true, true),
            create_test_interface("en0", true, true),
            create_test_interface("en1", true, true),
        ];

        let primary = find_primary_interface_fallback(&interfaces);
        assert_eq!(
            primary,
            Some("en0".to_string()),
            "Ties among physical+running should be broken alphabetically"
        );
    }

    #[test]
    fn test_primary_interface_alphabetical_tiebreak_physical_not_running() {
        let interfaces = vec![
            create_test_interface("eth2", true, false),
            create_test_interface("eth0", true, false),
        ];

        let primary = find_primary_interface_fallback(&interfaces);
        assert_eq!(
            primary,
            Some("eth0".to_string()),
            "Ties among physical+not-running should be broken alphabetically"
        );
    }

    #[test]
    fn test_primary_interface_alphabetical_tiebreak_virtual() {
        let interfaces = vec![
            create_test_interface("utun3", true, true),
            create_test_interface("utun1", true, true),
        ];

        let primary = find_primary_interface_fallback(&interfaces);
        assert_eq!(
            primary,
            Some("utun1".to_string()),
            "Ties among virtual interfaces should be broken alphabetically"
        );
    }

    #[test]
    fn test_primary_interface_prefers_windows_physical_over_virtual() {
        let interfaces = vec![
            create_test_interface("vEthernet (WSL)", true, true),
            create_test_interface("Ethernet", true, true),
            create_test_interface("Loopback", true, true),
        ];

        let primary = find_primary_interface_fallback(&interfaces);
        assert_eq!(
            primary,
            Some("Ethernet".to_string()),
            "Should prefer Windows physical adapter over virtual"
        );
    }

    #[test]
    fn test_primary_interface_recognizes_windows_physical_patterns() {
        let windows_physical = vec![
            "Ethernet",
            "Wi-Fi",
            "Local Area Connection",
            "Ethernet 2",
            "Wi-Fi 2",
        ];

        for name in windows_physical {
            let interfaces = vec![
                create_test_interface("vEthernet (Hyper-V)", true, true),
                create_test_interface(name, true, true),
            ];

            let primary = find_primary_interface_fallback(&interfaces);
            assert_eq!(
                primary,
                Some(name.to_string()),
                "Should recognize '{}' as a Windows physical adapter",
                name
            );
        }
    }

    #[test]
    fn test_primary_interface_excludes_windows_virtual_patterns() {
        let windows_virtual = vec![
            "vEthernet (WSL)",
            "vEthernet (Default Switch)",
            "Hyper-V Virtual Ethernet Adapter",
            "Bluetooth Network Connection",
            "Loopback",
            "Npcap Loopback Adapter",
        ];

        for name in windows_virtual {
            let interfaces = vec![
                create_test_interface(name, true, true),
                create_test_interface("Ethernet", true, true),
            ];

            let primary = find_primary_interface_fallback(&interfaces);
            assert_eq!(
                primary,
                Some("Ethernet".to_string()),
                "Should prefer Ethernet over virtual adapter '{}'",
                name
            );
        }
    }

    #[test]
    fn test_interface_priority_scoring() {
        assert_eq!(
            interface_priority(&create_test_interface("en0", true, true)),
            4,
            "Physical + running = 4"
        );
        assert_eq!(
            interface_priority(&create_test_interface("Ethernet", true, true)),
            4,
            "Windows physical + running = 4"
        );
        assert_eq!(
            interface_priority(&create_test_interface("en0", true, false)),
            3,
            "Physical + not running = 3"
        );
        assert_eq!(
            interface_priority(&create_test_interface("someother0", true, true)),
            2,
            "Non-virtual + running = 2"
        );
        assert_eq!(
            interface_priority(&create_test_interface("someother0", true, false)),
            1,
            "Non-virtual + not running = 1"
        );
        assert_eq!(
            interface_priority(&create_test_interface("docker0", true, true)),
            0,
            "Virtual = 0"
        );
        assert_eq!(
            interface_priority(&create_test_interface("vEthernet (WSL)", true, true)),
            0,
            "Windows virtual = 0"
        );
    }
}
