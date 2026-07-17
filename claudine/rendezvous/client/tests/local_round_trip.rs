//! End-to-end validation: spawn the daemon's gRPC server on a private local
//! endpoint and exercise the live Phase-1 RPCs over the same connection path
//! that the binary uses.
//!
//! This is the local control plane's cross-platform gate. Every test here runs
//! unchanged on macOS, Linux, and Windows — a Unix-domain socket on the first
//! two, a named pipe on the third — because that is the contract: a caller
//! resolves a `LocalEndpoint` and calls `connect`, and never learns which
//! transport carried the bytes. A test needing a `cfg` branch to say what it
//! means is a test that has found a leak in that contract, so the one Unix-only
//! case below is Unix-only for a stated reason rather than by default.

use rendezvous_client::connect;
use rendezvous_core::local_endpoint::test_support::{endpoint_env_value, private_endpoint};
use rendezvous_core::local_endpoint::{ENDPOINT_ENV_VAR, default_local_endpoint};
use rendezvous_core::{DAEMON_VERSION, PingRequest, StatusRequest};
use rendezvous_daemon::local_transport::spawn_local_server;
use rendezvous_daemon::server::DaemonConfig;
use tempfile::TempDir;

fn ephemeral_config(tmp: &TempDir) -> DaemonConfig {
    DaemonConfig::with_data_dir(tmp.path().join("data"))
        .with_in_memory_projection()
        .without_networking()
}

#[tokio::test]
async fn ping_round_trips_over_the_local_endpoint() {
    let tmp = TempDir::new().expect("tempdir");
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let config = ephemeral_config(&tmp);

    let handle = spawn_local_server(endpoint.clone(), config).expect("spawn daemon");
    assert_eq!(handle.local_endpoint(), &endpoint);

    let mut client = connect(&endpoint).await.expect("connect");
    let response = client
        .ping(PingRequest {
            nonce: "round-trip".into(),
        })
        .await
        .expect("ping rpc")
        .into_inner();

    assert_eq!(response.nonce, "round-trip");
    assert_eq!(response.daemon_version, DAEMON_VERSION);
    assert!(response.timestamp_unix_ms > 0);

    handle.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn status_round_trips_over_the_local_endpoint() {
    let tmp = TempDir::new().expect("tempdir");
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let config = ephemeral_config(&tmp);

    let handle = spawn_local_server(endpoint.clone(), config).expect("spawn daemon");

    let mut client = connect(&endpoint).await.expect("connect");
    let response = client
        .status(StatusRequest {})
        .await
        .expect("status rpc")
        .into_inner();

    assert_eq!(response.daemon_version, DAEMON_VERSION);
    assert!(response.uptime_seconds >= 0);
    assert!(response.started_at_unix_ms > 0);

    handle.shutdown().await.expect("shutdown");
}

/// Two clients at once, which is the ordinary case: the dashboard polls while a
/// wrapped session reports its status.
///
/// This is where the transports differ most and must not: a Unix listener
/// accepts independently of what it already handed out, but a named pipe serves
/// one client per *instance*, so an acceptor that failed to create its successor
/// before yielding the connected one would deadlock the second client here while
/// passing every single-client test above.
#[tokio::test]
async fn two_clients_are_served_concurrently() {
    let tmp = TempDir::new().expect("tempdir");
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let handle = spawn_local_server(endpoint.clone(), ephemeral_config(&tmp)).expect("spawn daemon");

    let mut first = connect(&endpoint).await.expect("first client");
    // The second connects while the first is still open and unfinished, so the
    // daemon has to be serving both at once rather than in sequence.
    let mut second = connect(&endpoint).await.expect("second client");

    let (a, b) = tokio::join!(
        first.ping(PingRequest {
            nonce: "client-a".into(),
        }),
        second.ping(PingRequest {
            nonce: "client-b".into(),
        }),
    );

    assert_eq!(a.expect("client a ping").into_inner().nonce, "client-a");
    assert_eq!(b.expect("client b ping").into_inner().nonce, "client-b");

    // The acceptor must have survived both: a third client proves the endpoint
    // is still served rather than consumed.
    let mut third = connect(&endpoint).await.expect("third client");
    assert_eq!(
        third
            .ping(PingRequest {
                nonce: "client-c".into(),
            })
            .await
            .expect("client c ping")
            .into_inner()
            .nonce,
        "client-c"
    );

    handle.shutdown().await.expect("shutdown");
}

/// The override path end to end, on the normal invocation route every binary
/// takes: `RENDEZVOUS_ENDPOINT` -> `default_local_endpoint` -> `connect` -> a
/// live daemon. This is the write/read round trip that `--endpoint` and the
/// env var both depend on, so a resolution that silently reinterpreted the
/// value would surface here rather than as a connect failure in the field.
#[tokio::test]
async fn the_endpoint_env_var_resolves_back_to_the_daemon_that_bound_it() {
    let tmp = TempDir::new().expect("tempdir");
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let handle =
        spawn_local_server(endpoint.clone(), ephemeral_config(&tmp)).expect("spawn daemon");

    // SAFETY: nextest gives each test its own process, so nothing else here
    // reads or writes this variable concurrently.
    unsafe { std::env::set_var(ENDPOINT_ENV_VAR, endpoint_env_value(&endpoint)) };

    let resolved = default_local_endpoint().expect("the override must resolve");
    assert_eq!(
        resolved, endpoint,
        "the override must round-trip to the endpoint it names"
    );

    let mut client = connect(&resolved).await.expect("connect via the override");
    let response = client
        .ping(PingRequest {
            nonce: "via-env".into(),
        })
        .await
        .expect("ping rpc")
        .into_inner();
    assert_eq!(response.nonce, "via-env");

    handle.shutdown().await.expect("shutdown");
}

/// Unix-only: the socket file is filesystem state the daemon must reclaim.
/// A named pipe has no filesystem entry to leave behind, so there is nothing
/// for this assertion to say on Windows.
#[cfg(unix)]
#[tokio::test]
async fn dropping_handle_removes_socket_file() {
    let tmp = TempDir::new().expect("tempdir");
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let path = endpoint
        .as_unix_path()
        .expect("private_endpoint yields a Unix socket on Unix")
        .to_path_buf();
    let config = ephemeral_config(&tmp);

    {
        let handle = spawn_local_server(endpoint.clone(), config).expect("spawn daemon");
        assert!(path.exists(), "socket should exist while server runs");
        drop(handle);
    }

    assert!(!path.exists(), "socket should be cleaned up on drop");
}
