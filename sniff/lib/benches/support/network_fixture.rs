//! Wiremock-backed WAN IP fixture for bench groups that exercise the
//! `detect_network_with_request(NetworkRequest::full())` path.
//!
//! The real `api64.ipify.org` endpoint is unsuitable for benches: its
//! latency is dominated by public internet RTT and can vary by orders
//! of magnitude across hosts. Instead we spin up a local `wiremock`
//! server once for the whole bench process, point
//! `SNIFF_WAN_IP_ENDPOINTS` at it, and let every downstream benchmark
//! hit that local address.
//!
//! The fixture is idempotent — [`ensure_ready`] can be called from
//! multiple bench groups without re-provisioning the mock.

#![allow(dead_code)]

use std::sync::OnceLock;

use tokio::runtime::Runtime;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Environment variable observed by `sniff::network` to override the
/// WAN IP echo endpoints. Kept as a literal here so the bench module
/// compiles regardless of whether the `network` feature is enabled.
/// Must stay in sync with `WAN_IP_ENDPOINTS_ENV` in
/// `sniff/lib/src/network/mod.rs`.
const WAN_IP_ENDPOINTS_ENV: &str = "SNIFF_WAN_IP_ENDPOINTS";

/// Constant WAN IP string served by the bench mock.
pub const BENCH_WAN_IP: &str = "203.0.113.77";

/// Guard that owns the tokio runtime and wiremock server backing the
/// fixture for the lifetime of the bench process.
struct NetworkFixture {
    // The runtime must outlive the server, which owns its own internal
    // tasks. We never read either field — they exist solely to pin the
    // resources in memory until process exit.
    _runtime: Runtime,
    _server: MockServer,
}

static FIXTURE: OnceLock<NetworkFixture> = OnceLock::new();

/// Idempotently ensure the wiremock-backed WAN IP fixture is running
/// and that `SNIFF_WAN_IP_ENDPOINTS` points at it.
pub fn ensure_ready() {
    FIXTURE.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime for wiremock fixture");

        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_string(BENCH_WAN_IP))
                .mount(&server)
                .await;
            server
        });

        // SAFETY: benches run single-threaded through Criterion's
        // registration loop before worker threads spawn, so setting an
        // env var here has no concurrent-mutator risk. This is the
        // standard pattern for seeding process-wide configuration
        // before the code under bench observes it.
        unsafe {
            std::env::set_var(WAN_IP_ENDPOINTS_ENV, server.uri());
        }

        NetworkFixture {
            _runtime: runtime,
            _server: server,
        }
    });
}

/// Return the configured WAN IP endpoint base URL.
///
/// Primarily exposed so the network bench case can assert the fixture
/// is wired up before running latency-sensitive measurements.
pub fn endpoint_uri() -> String {
    std::env::var(WAN_IP_ENDPOINTS_ENV).unwrap_or_default()
}
