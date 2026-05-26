//! Phase-6 end-to-end integration tests.
//!
//! These tests exercise the entire stack as a black box: spawn two (or
//! more) daemons, pair them, drive appends and sync through the gRPC
//! surface, and assert that the system converges, rotates chunks
//! deterministically, survives a restart, and refuses to leak data to
//! unpaired peers.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use remote_signal_client::connect_uds;
use remote_signal_core::{
    AppendEntryRequest, ApprovePeerRequest, ChunkConfig, ConnectToPeerRequest,
    CreateInvitationRequest, ListChunkEntriesRequest, ListSessionChunksRequest, PeerConnectionState,
    RemoteSignalClient, SyncWithPeerRequest,
};
use remote_signal_daemon::server::{DaemonConfig, NetworkConfig, ServerHandle, spawn_uds_server};
use tempfile::TempDir;
use tokio::time::sleep;
use tonic::transport::Channel;

fn networking_config() -> NetworkConfig {
    NetworkConfig {
        quic_bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        mdns_enabled: false,
    }
}

fn base_config(data_dir: PathBuf) -> DaemonConfig {
    DaemonConfig::with_data_dir(data_dir)
        .with_in_memory_projection()
        .with_networking(networking_config())
}

async fn boot_daemon(
    tmp: &TempDir,
    name: &str,
) -> (ServerHandle, PathBuf) {
    let data_dir = tmp.path().join(name);
    boot_daemon_with(tmp, name, base_config(data_dir)).await
}

async fn boot_daemon_with(
    tmp: &TempDir,
    name: &str,
    config: DaemonConfig,
) -> (ServerHandle, PathBuf) {
    let socket = tmp.path().join(format!("{name}.sock"));
    let handle = spawn_uds_server(socket.clone(), config).expect("spawn daemon");
    wait_until_bound(&socket).await;
    (handle, socket)
}

async fn wait_until_bound(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        if Instant::now() >= deadline {
            panic!("socket {} never appeared", path.display());
        }
        sleep(Duration::from_millis(10)).await;
    }
}

async fn pair_and_connect(
    alice: &ServerHandle,
    alice_client: &mut RemoteSignalClient<Channel>,
    bob: &ServerHandle,
    bob_client: &mut RemoteSignalClient<Channel>,
) {
    alice_client
        .approve_peer(ApprovePeerRequest {
            node_id: bob.node_id(),
            note: "bob".into(),
        })
        .await
        .expect("alice approves bob");
    bob_client
        .approve_peer(ApprovePeerRequest {
            node_id: alice.node_id(),
            note: "alice".into(),
        })
        .await
        .expect("bob approves alice");

    let invitation = bob_client
        .create_invitation(CreateInvitationRequest {
            advertise_addr: String::new(),
        })
        .await
        .expect("invitation")
        .into_inner();
    let connect = alice_client
        .connect_to_peer(ConnectToPeerRequest {
            invitation: invitation.invitation,
        })
        .await
        .expect("connect_to_peer")
        .into_inner();
    let peer = connect.peer.expect("peer info");
    assert_eq!(peer.node_id, bob.node_id());
    assert_eq!(peer.state, PeerConnectionState::Connected as i32);
}

async fn append(
    client: &mut RemoteSignalClient<Channel>,
    owner: &str,
    session: &str,
    source: &str,
    message: &str,
) {
    client
        .append_entry(AppendEntryRequest {
            owner_node_id: owner.into(),
            session_id: session.into(),
            source: source.into(),
            level: "info".into(),
            message: message.into(),
            metadata_json: String::new(),
        })
        .await
        .expect("append");
}

async fn collect_messages(
    client: &mut RemoteSignalClient<Channel>,
    owner: &str,
    session: &str,
) -> Vec<String> {
    let chunks = client
        .list_session_chunks(ListSessionChunksRequest {
            owner_node_id: owner.into(),
            session_id: session.into(),
        })
        .await
        .expect("list chunks")
        .into_inner();
    let mut messages = Vec::new();
    for chunk in chunks.chunk_ids {
        let entries = client
            .list_chunk_entries(ListChunkEntriesRequest {
                chunk_id: chunk.clone(),
            })
            .await
            .expect("list entries")
            .into_inner();
        messages.extend(entries.entries.into_iter().map(|e| e.message));
    }
    messages
}

async fn collect_chunk_ids(
    client: &mut RemoteSignalClient<Channel>,
    owner: &str,
    session: &str,
) -> Vec<String> {
    client
        .list_session_chunks(ListSessionChunksRequest {
            owner_node_id: owner.into(),
            session_id: session.into(),
        })
        .await
        .expect("list chunks")
        .into_inner()
        .chunk_ids
}

async fn wait_until<F, Fut>(deadline: Duration, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let end = Instant::now() + deadline;
    loop {
        if check().await {
            return;
        }
        if Instant::now() >= end {
            panic!("condition never satisfied within {deadline:?}");
        }
        sleep(Duration::from_millis(100)).await;
    }
}

/// **Convergence test** — Two paired daemons append concurrently to the
/// same session-log chunk and converge to the identical set of entries
/// after a direct-sync round.
#[tokio::test]
async fn two_nodes_converge_on_shared_chunk() {
    let tmp = TempDir::new().expect("tempdir");
    let (alice, alice_sock) = boot_daemon(&tmp, "alice").await;
    let (bob, bob_sock) = boot_daemon(&tmp, "bob").await;

    let mut alice_client = connect_uds(alice_sock).await.expect("alice client");
    let mut bob_client = connect_uds(bob_sock).await.expect("bob client");

    pair_and_connect(&alice, &mut alice_client, &bob, &mut bob_client).await;

    // Each side writes a few entries before any sync happens.
    for i in 0..3 {
        append(&mut alice_client, "shared", "s1", "alice", &format!("a-{i}")).await;
        append(&mut bob_client, "shared", "s1", "bob", &format!("b-{i}")).await;
    }

    // Drive a sync from Alice and wait for both sides to reflect the
    // union of entries.
    alice_client
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob.node_id(),
        })
        .await
        .expect("alice sync");

    let expected: Vec<String> = (0..3)
        .flat_map(|i| [format!("a-{i}"), format!("b-{i}")])
        .collect();
    wait_for_messages(&mut alice_client, "shared", "s1", &expected).await;
    wait_for_messages(&mut bob_client, "shared", "s1", &expected).await;

    // Both sides must agree on every chunk-id at the path level.
    let mut alice_chunks = collect_chunk_ids(&mut alice_client, "shared", "s1").await;
    let mut bob_chunks = collect_chunk_ids(&mut bob_client, "shared", "s1").await;
    alice_chunks.sort();
    bob_chunks.sort();
    assert_eq!(alice_chunks, bob_chunks, "chunk catalogs must converge");

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

/// **Chunking test** — When the configured chunk threshold is exceeded
/// the daemon rotates to a new deterministic chunk id, and after sync
/// both peers materialise every chunk.
#[tokio::test]
async fn chunk_rotation_propagates_through_sync() {
    let tmp = TempDir::new().expect("tempdir");
    // Force rotation after every 2 entries so the test triggers
    // multiple chunks without ballooning runtime.
    let chunk_cfg = ChunkConfig::new(2, 64 * 1024);
    let (alice, alice_sock) = boot_daemon_with(
        &tmp,
        "alice",
        base_config(tmp.path().join("alice")).with_chunk_config(chunk_cfg),
    )
    .await;
    let (bob, bob_sock) = boot_daemon_with(
        &tmp,
        "bob",
        base_config(tmp.path().join("bob")).with_chunk_config(chunk_cfg),
    )
    .await;

    let mut alice_client = connect_uds(alice_sock).await.expect("alice client");
    let mut bob_client = connect_uds(bob_sock).await.expect("bob client");

    pair_and_connect(&alice, &mut alice_client, &bob, &mut bob_client).await;

    // Alice writes 5 entries — should produce chunks 0, 1, 2 (2 + 2 + 1).
    for i in 0..5 {
        append(&mut alice_client, "shared", "rot", "alice", &format!("m-{i}")).await;
    }

    let alice_chunks = collect_chunk_ids(&mut alice_client, "shared", "rot").await;
    assert!(
        alice_chunks.len() >= 3,
        "expected at least three chunks on Alice, got {alice_chunks:?}",
    );
    assert_eq!(
        alice_chunks[0], "session/shared/rot/part/0",
        "chunk ids must be deterministic",
    );
    assert_eq!(
        alice_chunks[1], "session/shared/rot/part/1",
        "chunk ids must be deterministic",
    );

    alice_client
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob.node_id(),
        })
        .await
        .expect("alice sync");

    let expected: Vec<String> = (0..5).map(|i| format!("m-{i}")).collect();
    wait_for_messages(&mut bob_client, "shared", "rot", &expected).await;

    let bob_chunks = collect_chunk_ids(&mut bob_client, "shared", "rot").await;
    assert_eq!(
        alice_chunks, bob_chunks,
        "Bob must mirror Alice's chunk catalog after sync",
    );

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

/// **Restart/Replay test** — A daemon that has appended entries can be
/// stopped and re-launched on the same data directory; the rehydrated
/// daemon must replay redb state and resume sync with a paired peer.
#[tokio::test]
async fn restart_replays_state_and_resumes_sync() {
    let tmp = TempDir::new().expect("tempdir");
    let alice_dir = tmp.path().join("alice");
    let bob_dir = tmp.path().join("bob");

    // Phase 1: boot both daemons, pair, write on Alice's side only.
    let (alice, alice_sock) =
        boot_daemon_with(&tmp, "alice", base_config(alice_dir.clone())).await;
    let (bob, bob_sock) = boot_daemon_with(&tmp, "bob", base_config(bob_dir.clone())).await;
    let alice_node = alice.node_id();
    let bob_node = bob.node_id();

    let mut alice_client = connect_uds(alice_sock.clone()).await.expect("alice client");
    let mut bob_client = connect_uds(bob_sock.clone()).await.expect("bob client");

    pair_and_connect(&alice, &mut alice_client, &bob, &mut bob_client).await;
    for i in 0..4 {
        append(&mut alice_client, "shared", "rs", "alice", &format!("pre-{i}")).await;
    }

    // Tear Alice down — Bob keeps running.
    drop(alice_client);
    alice.shutdown().await.expect("alice shutdown");

    // Phase 2: re-spawn Alice on the same data directory.
    let (alice2, alice_sock2) =
        boot_daemon_with(&tmp, "alice", base_config(alice_dir.clone())).await;
    // Identity is persisted, so the node id must be stable.
    assert_eq!(alice2.node_id(), alice_node, "identity must persist across restart");

    let mut alice_client2 = connect_uds(alice_sock2).await.expect("alice2 client");

    // The pre-restart entries must be visible from the rehydrated state.
    let recovered = collect_messages(&mut alice_client2, "shared", "rs").await;
    let expected_pre: Vec<String> = (0..4).map(|i| format!("pre-{i}")).collect();
    for want in &expected_pre {
        assert!(
            recovered.contains(want),
            "rehydrated Alice missing {want:?}, saw {recovered:?}",
        );
    }

    // Re-pair on Alice's side: the pairings table is persisted in redb
    // too, so the existing pairing should already be intact — but
    // upserting is idempotent and a useful sanity check.
    alice_client2
        .approve_peer(ApprovePeerRequest {
            node_id: bob_node.clone(),
            note: "bob".into(),
        })
        .await
        .expect("re-approve bob");

    // Re-connect to Bob via a fresh invitation (QUIC certs are per-run).
    let invitation = bob_client
        .create_invitation(CreateInvitationRequest {
            advertise_addr: String::new(),
        })
        .await
        .expect("invitation")
        .into_inner();
    alice_client2
        .connect_to_peer(ConnectToPeerRequest {
            invitation: invitation.invitation,
        })
        .await
        .expect("alice2 connect");

    // Append one more entry on Alice and one on Bob, then sync — both
    // sides must converge on the union of pre- and post-restart writes.
    append(&mut alice_client2, "shared", "rs", "alice", "post-alice").await;
    append(&mut bob_client, "shared", "rs", "bob", "post-bob").await;

    alice_client2
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob_node.clone(),
        })
        .await
        .expect("alice2 sync");

    let mut expected = expected_pre.clone();
    expected.push("post-alice".into());
    expected.push("post-bob".into());
    wait_for_messages(&mut alice_client2, "shared", "rs", &expected).await;
    wait_for_messages(&mut bob_client, "shared", "rs", &expected).await;

    alice2.shutdown().await.expect("alice2 shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

/// **Security test** — An unpaired peer cannot pull session-log data
/// even when it has a healthy QUIC connection. The server returns
/// `FailedPrecondition` and the caller's session-log catalog remains
/// untouched.
#[tokio::test]
async fn unpaired_peer_cannot_pull_data_over_quic() {
    let tmp = TempDir::new().expect("tempdir");
    let (alice, alice_sock) = boot_daemon(&tmp, "alice").await;
    let (bob, bob_sock) = boot_daemon(&tmp, "bob").await;

    let mut alice_client = connect_uds(alice_sock).await.expect("alice client");
    let mut bob_client = connect_uds(bob_sock).await.expect("bob client");

    // Bob writes a "secret" entry that must NOT leak to Alice.
    append(&mut bob_client, "bob", "secret", "bob", "do-not-leak").await;

    // QUIC connect without approving the pairing on either side.
    let invitation = bob_client
        .create_invitation(CreateInvitationRequest {
            advertise_addr: String::new(),
        })
        .await
        .expect("invitation")
        .into_inner();
    alice_client
        .connect_to_peer(ConnectToPeerRequest {
            invitation: invitation.invitation,
        })
        .await
        .expect("connect");

    // Sync MUST fail with FailedPrecondition.
    let err = alice_client
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob.node_id(),
        })
        .await
        .expect_err("sync without pairing must fail");
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);

    // Alice's view of the session must still be empty.
    let leaked = collect_messages(&mut alice_client, "bob", "secret").await;
    assert!(
        leaked.is_empty(),
        "unpaired peer must not see remote data; got {leaked:?}",
    );

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

/// **Final POC demo** — Walk the end-to-end multi-node flow that the
/// `scripts/poc-demo.sh` script exercises against a running daemon:
///
/// 1. Bootstrap, 2. pair, 3. write on both sides, 4. converge,
/// 5. rotate, 6. converge again. This mirrors the script so a CI run
/// can validate every step the demo claims to perform without needing
/// the binaries on PATH.
#[tokio::test]
async fn poc_demo_end_to_end_flow() {
    let tmp = TempDir::new().expect("tempdir");
    let chunk_cfg = ChunkConfig::new(3, 64 * 1024);
    let (alice, alice_sock) = boot_daemon_with(
        &tmp,
        "alice",
        base_config(tmp.path().join("alice")).with_chunk_config(chunk_cfg),
    )
    .await;
    let (bob, bob_sock) = boot_daemon_with(
        &tmp,
        "bob",
        base_config(tmp.path().join("bob")).with_chunk_config(chunk_cfg),
    )
    .await;

    let mut alice_client = connect_uds(alice_sock).await.expect("alice client");
    let mut bob_client = connect_uds(bob_sock).await.expect("bob client");

    // 1-2. Pair + connect.
    pair_and_connect(&alice, &mut alice_client, &bob, &mut bob_client).await;

    // 3. Each side writes — enough on Alice to rotate.
    for i in 0..4 {
        append(&mut alice_client, "demo", "main", "alice", &format!("alice-{i}")).await;
    }
    append(&mut bob_client, "demo", "main", "bob", "bob-0").await;

    // 4. Sync and converge.
    alice_client
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob.node_id(),
        })
        .await
        .expect("sync");

    let expected: Vec<String> = (0..4)
        .map(|i| format!("alice-{i}"))
        .chain(std::iter::once("bob-0".into()))
        .collect();
    wait_for_messages(&mut alice_client, "demo", "main", &expected).await;
    wait_for_messages(&mut bob_client, "demo", "main", &expected).await;

    // 5. Confirm chunk rotation happened (4 + 1 > 3 per chunk).
    let alice_chunks = collect_chunk_ids(&mut alice_client, "demo", "main").await;
    assert!(
        alice_chunks.len() >= 2,
        "demo flow must trigger at least one chunk rotation, got {alice_chunks:?}",
    );

    // 6. Final convergence sanity — Bob holds the same chunk catalog.
    let target_len = alice_chunks.len();
    wait_until(Duration::from_secs(5), || {
        let mut bob_client = bob_client.clone();
        async move {
            let chunks = collect_chunk_ids(&mut bob_client, "demo", "main").await;
            chunks.len() == target_len
        }
    })
    .await;

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

async fn wait_for_messages(
    client: &mut RemoteSignalClient<Channel>,
    owner: &str,
    session: &str,
    expected: &[String],
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let messages = collect_messages(client, owner, session).await;
        if expected.iter().all(|want| messages.contains(want)) {
            return;
        }
        if Instant::now() >= deadline {
            panic!(
                "timed out waiting for messages {expected:?}; saw {messages:?}",
            );
        }
        sleep(Duration::from_millis(100)).await;
    }
}
