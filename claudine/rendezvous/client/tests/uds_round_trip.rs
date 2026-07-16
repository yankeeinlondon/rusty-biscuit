//! End-to-end validation: spawn the daemon's gRPC server on an
//! ephemeral Unix Domain Socket and exercise the live Phase-1 RPCs over
//! the same connection path that the binary uses.

use std::time::Duration;

use rendezvous_client::connect_uds;
use rendezvous_core::{DAEMON_VERSION, PingRequest, StatusRequest};
use rendezvous_daemon::server::{DaemonConfig, spawn_uds_server};
use tempfile::TempDir;
use tokio::time::sleep;

/// A private directory to hold the endpoint.
///
/// `tempfile` creates 0755 directories, which the daemon refuses: the endpoint
/// directory is the Unix security boundary. The real default resolves into a
/// private directory, so a fixture has to build one too rather than dropping
/// the socket straight into the temp root.
fn runtime_socket(tmp: &TempDir, name: &str) -> std::path::PathBuf {
    use std::os::unix::fs::DirBuilderExt;

    let dir = tmp.path().join("runtime");
    if !dir.exists() {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .expect("create runtime dir");
    }
    dir.join(format!("{name}.sock"))
}


fn ephemeral_config(tmp: &TempDir) -> DaemonConfig {
    DaemonConfig::with_data_dir(tmp.path().join("data"))
        .with_in_memory_projection()
        .without_networking()
}

#[tokio::test]
async fn ping_round_trips_over_uds() {
    let tmp = TempDir::new().expect("tempdir");
    let socket_path = runtime_socket(&tmp, "daemon");
    let config = ephemeral_config(&tmp);

    let handle = spawn_uds_server(socket_path.clone(), config).expect("spawn daemon");
    wait_until_bound(&socket_path).await;

    let mut client = connect_uds(socket_path.clone()).await.expect("connect");
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
async fn status_round_trips_over_uds() {
    let tmp = TempDir::new().expect("tempdir");
    let socket_path = runtime_socket(&tmp, "daemon");
    let config = ephemeral_config(&tmp);

    let handle = spawn_uds_server(socket_path.clone(), config).expect("spawn daemon");
    wait_until_bound(&socket_path).await;

    let mut client = connect_uds(socket_path.clone()).await.expect("connect");
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

#[tokio::test]
async fn dropping_handle_removes_socket_file() {
    let tmp = TempDir::new().expect("tempdir");
    let socket_path = runtime_socket(&tmp, "daemon");
    let config = ephemeral_config(&tmp);

    {
        let handle = spawn_uds_server(socket_path.clone(), config).expect("spawn daemon");
        wait_until_bound(&socket_path).await;
        assert!(socket_path.exists(), "socket should exist while server runs");
        drop(handle);
    }

    assert!(!socket_path.exists(), "socket should be cleaned up on drop");
}

/// Poll until the socket file exists or a generous timeout elapses. The
/// `UnixListener::bind` call inside the spawned task is synchronous but
/// runs on a Tokio worker thread, so the file may not be visible the
/// instant `spawn_uds_server` returns.
async fn wait_until_bound(path: &std::path::Path) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        if std::time::Instant::now() >= deadline {
            panic!("socket {} never appeared", path.display());
        }
        sleep(Duration::from_millis(10)).await;
    }
}
