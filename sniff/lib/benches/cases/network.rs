//! Network-detection Criterion benches.
//!
//! Covers the three network paths the design calls out as variance
//! sources:
//!
//! - local interface enumeration only (no HTTP)
//! - full path (WAN IP lookup) against a wiremock-backed endpoint, with
//!   `force_refresh` so the TTL cache is bypassed every iteration
//! - full path with a warm cache so cache-hit cost is isolated from
//!   the actual HTTP round-trip
//!
//! When the `network` feature is disabled the WAN IP calls degrade to
//! a no-op inside `sniff::network::detect_wan_ip`, so these benches
//! still run but only measure orchestration cost. CI enables
//! `--features network` so the full wiremock path is exercised end to
//! end.

use criterion::{Criterion, black_box};
use sniff::network::detect_network_with_request;
use sniff::request::NetworkRequest;

use crate::support::{network_fixture, util};

pub fn register(c: &mut Criterion) {
    network_fixture::ensure_ready();

    let mut group = util::configure_group(c, "network");

    group.bench_function("network_interfaces_enumeration_only", |b| {
        let req = NetworkRequest::interfaces_only();
        b.iter(|| {
            let info = detect_network_with_request(black_box(&req)).expect("interfaces_only");
            black_box(info);
        });
    });

    // `force_refresh` bypasses the global TTL cache, so this group
    // measures the real HTTP round-trip against the wiremock fixture.
    group.bench_function("wan_ip_http_roundtrip_no_cache", |b| {
        let req = NetworkRequest::full().force_refresh(true);
        b.iter(|| {
            let info = detect_network_with_request(black_box(&req)).expect("wan_ip_forced_refresh");
            black_box(info);
        });
    });

    // Warm the cache once before measurement so `wan_ip_cached` only
    // measures the interfaces walk plus cache-hit bookkeeping.
    let warm = NetworkRequest::full();
    let _ = detect_network_with_request(&warm).expect("prime WAN IP cache");
    group.bench_function("wan_ip_cache_hit_plus_interfaces", |b| {
        let req = NetworkRequest::full();
        b.iter(|| {
            let info = detect_network_with_request(black_box(&req)).expect("wan_ip_cached");
            black_box(info);
        });
    });

    group.finish();
}
