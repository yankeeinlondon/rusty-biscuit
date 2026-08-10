//! Phase-5 end-to-end validation: pairing + direct-sync convergence.
//!
//! Two daemons exchange invitations, approve each other, append an
//! entry on each side, then run the direct-sync protocol over the
//! QUIC connection. After sync, both daemons must report the same set
//! of entries for the shared session.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use rendezvous_client::connect;
use rendezvous_core::local_endpoint::{LocalEndpoint, test_support::private_endpoint};
use rendezvous_core::{
    AppendEntryRequest, ApprovePeerRequest, ConnectToPeerRequest, CreateInvitationRequest,
    ListChunkEntriesRequest, ListPairingsRequest, ListPeersRequest, ListSessionChunksRequest,
    PeerConnectionState, RendezvousClient, RevokePeerRequest, SyncWithPeerRequest,
};
use rendezvous_daemon::local_transport::spawn_local_server;
use rendezvous_daemon::server::{DaemonConfig, NetworkConfig, ServerHandle};
use tempfile::TempDir;
use tokio::time::sleep;
use tonic::transport::Channel;

fn networking_config() -> NetworkConfig {
    NetworkConfig {
        quic_bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        mdns_enabled: false,
    }
}

fn daemon_config(tmp: &TempDir, name: &str) -> DaemonConfig {
    DaemonConfig::with_data_dir(tmp.path().join(name))
        .with_in_memory_projection()
        .with_networking(networking_config())
}

async fn boot_daemon(tmp: &TempDir, name: &str) -> (ServerHandle, LocalEndpoint) {
    let socket = private_endpoint(tmp.path(), name);
    let handle =
        spawn_local_server(socket.clone(), daemon_config(tmp, name)).expect("spawn daemon");
    (handle, socket)
}

#[tokio::test]
async fn paired_daemons_converge_after_direct_sync() {
    let tmp = TempDir::new().expect("tempdir");
    let (alice, alice_sock) = boot_daemon(&tmp, "alice").await;
    let (bob, bob_sock) = boot_daemon(&tmp, "bob").await;

    let alice_node = alice.node_id();
    let bob_node = bob.node_id();

    let mut alice_client = connect(&alice_sock).await.expect("alice connect");
    let mut bob_client = connect(&bob_sock).await.expect("bob connect");

    // Pre-approve each other so the initial sync attempt that fires
    // automatically when QUIC connects can succeed.
    alice_client
        .approve_peer(ApprovePeerRequest {
            node_id: bob_node.clone(),
            note: "bob".into(),
        })
        .await
        .expect("alice approves bob");
    bob_client
        .approve_peer(ApprovePeerRequest {
            node_id: alice_node.clone(),
            note: "alice".into(),
        })
        .await
        .expect("bob approves alice");

    // Each side appends one entry into their own namespace before any
    // connection is established. After sync both sides must observe
    // both entries in their respective owner namespaces.
    alice_client
        .append_entry(AppendEntryRequest {
            owner_node_id: String::new(),
            session_id: "s1".into(),
            source: "alice".into(),
            level: "info".into(),
            message: "from-alice".into(),
            metadata_json: String::new(),
        })
        .await
        .expect("alice append");
    bob_client
        .append_entry(AppendEntryRequest {
            owner_node_id: String::new(),
            session_id: "s1".into(),
            source: "bob".into(),
            level: "info".into(),
            message: "from-bob".into(),
            metadata_json: String::new(),
        })
        .await
        .expect("bob append");

    // Bob creates an invitation and Alice dials it. The handshake
    // triggers a background sync that should converge both sides.
    let invitation = bob_client
        .create_invitation(CreateInvitationRequest {
            advertise_addr: String::new(),
        })
        .await
        .expect("invitation")
        .into_inner();
    let connect_response = alice_client
        .connect_to_peer(ConnectToPeerRequest {
            invitation: invitation.invitation,
        })
        .await
        .expect("alice connect_to_peer")
        .into_inner();
    let peer = connect_response.peer.expect("peer info");
    assert_eq!(peer.node_id, bob_node);
    assert_eq!(peer.state, PeerConnectionState::Connected as i32);

    // Explicitly drive a sync as well — the auto-sync fires
    // asynchronously and may race with the assertions below.
    let sync = alice_client
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob_node.clone(),
        })
        .await
        .expect("alice sync_with_peer")
        .into_inner();
    assert_eq!(sync.node_id, bob_node);

    // A successful sync stamps the freshness clock the dashboard reads
    // — distinct from mDNS-driven `last_seen`. Bob's peer row on Alice
    // must now carry a positive `last_synced_unix_ms`.
    let bob_peer = alice_client
        .list_peers(ListPeersRequest {})
        .await
        .expect("alice list_peers")
        .into_inner()
        .peers
        .into_iter()
        .find(|p| p.node_id == bob_node)
        .expect("bob present in alice's peer registry");
    assert!(
        bob_peer.last_synced_unix_ms > 0,
        "successful sync must stamp last_synced_unix_ms: {bob_peer:?}",
    );

    // After sync, each side should see their own and the other's data.
    let alice_node = alice_node.clone();
    let bob_node = bob_node.clone();
    wait_for_messages(&mut alice_client, &alice_node, "s1", &["from-alice"]).await;
    wait_for_replica(&mut alice_client, &bob_node, "s1", &["from-bob"]).await;
    wait_for_messages(&mut bob_client, &bob_node, "s1", &["from-bob"]).await;
    wait_for_replica(&mut bob_client, &alice_node, "s1", &["from-alice"]).await;

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

#[tokio::test]
async fn sync_is_rejected_when_pairing_is_missing() {
    let tmp = TempDir::new().expect("tempdir");
    let (alice, alice_sock) = boot_daemon(&tmp, "alice").await;
    let (bob, bob_sock) = boot_daemon(&tmp, "bob").await;

    let mut alice_client = connect(&alice_sock).await.expect("alice connect");
    let mut bob_client = connect(&bob_sock).await.expect("bob connect");

    // Connect QUIC via manual invitation. This auto-pairs Alice with
    // Bob (initiator side), but Bob has not approved Alice.
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

    // Sync fails because the responder (Bob) has not paired Alice.
    let err = alice_client
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob.node_id(),
        })
        .await
        .expect_err("sync without responder pairing must fail");
    assert!(
        err.code() == tonic::Code::Internal || err.code() == tonic::Code::FailedPrecondition,
        "expected Internal or FailedPrecondition, got {:?}: {}",
        err.code(),
        err.message(),
    );

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

#[tokio::test]
async fn pairings_can_be_listed_and_revoked() {
    let tmp = TempDir::new().expect("tempdir");
    let (alice, alice_sock) = boot_daemon(&tmp, "alice").await;
    let mut client = connect(&alice_sock).await.expect("connect");

    let node_id = "1111111111111111111111111111111111111111111111111111111111111111".to_string();
    client
        .approve_peer(ApprovePeerRequest {
            node_id: node_id.clone(),
            note: "test".into(),
        })
        .await
        .expect("approve");
    let listed = client
        .list_pairings(ListPairingsRequest {})
        .await
        .expect("list")
        .into_inner();
    assert!(
        listed.pairings.iter().any(|p| p.node_id == node_id),
        "expected pairing in {listed:?}",
    );

    let response = client
        .revoke_peer(RevokePeerRequest {
            node_id: node_id.clone(),
        })
        .await
        .expect("revoke")
        .into_inner();
    assert!(response.removed);

    // Listing again should not include the revoked pairing.
    let listed = client
        .list_pairings(ListPairingsRequest {})
        .await
        .expect("list")
        .into_inner();
    assert!(listed.pairings.iter().all(|p| p.node_id != node_id));

    alice.shutdown().await.expect("shutdown");
}

async fn wait_for_messages(
    client: &mut RendezvousClient<Channel>,
    owner: &str,
    session: &str,
    expected: &[&str],
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let messages = collect_messages_async(client, owner, session).await;
        if expected
            .iter()
            .all(|want| messages.iter().any(|m| m == *want))
        {
            return;
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for messages {expected:?}; saw {messages:?}",);
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_for_replica(
    client: &mut RendezvousClient<Channel>,
    owner: &str,
    session: &str,
    expected: &[&str],
) {
    wait_for_messages(client, owner, session, expected).await
}

async fn collect_messages_async(
    client: &mut RendezvousClient<Channel>,
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
    for chunk in &chunks.chunk_ids {
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
