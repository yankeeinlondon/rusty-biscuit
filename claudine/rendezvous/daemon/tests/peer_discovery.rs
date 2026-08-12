//! Phase-4 end-to-end validation.
//!
//! These tests exercise the networking stack from the outside: two
//! daemons are spun up on private, per-test local endpoints and either (a) handed a
//! manual invitation to dial each other or (b) left to discover each
//! other over mDNS multicast. The validation requires the QUIC
//! handshake to complete and the peer registries to converge to a
//! `CONNECTED` view of the remote node.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use rendezvous_client::connect;
use rendezvous_core::local_endpoint::{LocalEndpoint, test_support::private_endpoint};
use rendezvous_core::{
    AppendEntryRequest, ConnectToPeerRequest, CreateInvitationRequest, ListChunkEntriesRequest,
    ListPeersRequest, ListSessionChunksRequest, PeerConnectionState, PeerSource,
    SyncWithPeerRequest,
};
use rendezvous_daemon::local_transport::spawn_local_server;
use rendezvous_daemon::server::{DaemonConfig, NetworkConfig, ServerHandle};
use tempfile::TempDir;
use tokio::time::sleep;

fn networking_config(mdns_enabled: bool) -> NetworkConfig {
    NetworkConfig {
        quic_bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        mdns_enabled,
    }
}

fn daemon_config(tmp: &TempDir, name: &str, mdns_enabled: bool) -> DaemonConfig {
    DaemonConfig::with_data_dir(tmp.path().join(name))
        .with_in_memory_projection()
        .with_networking(networking_config(mdns_enabled))
}

async fn boot_daemon(
    tmp: &TempDir,
    name: &str,
    mdns_enabled: bool,
) -> (ServerHandle, LocalEndpoint) {
    let socket = private_endpoint(tmp.path(), name);
    let handle = spawn_local_server(socket.clone(), daemon_config(tmp, name, mdns_enabled))
        .expect("spawn daemon");
    (handle, socket)
}

#[tokio::test]
async fn two_daemons_connect_via_manual_invitation() {
    let tmp = TempDir::new().expect("tempdir");
    let (alice, alice_sock) = boot_daemon(&tmp, "alice", false).await;
    let (bob, bob_sock) = boot_daemon(&tmp, "bob", false).await;

    // Bob creates an invitation; Alice consumes it.
    let mut bob_client = connect(&bob_sock).await.expect("bob connect");
    let invitation = bob_client
        .create_invitation(CreateInvitationRequest {
            advertise_addr: String::new(),
        })
        .await
        .expect("create invitation")
        .into_inner();

    assert!(!invitation.invitation.is_empty(), "invitation must be set");
    assert_eq!(invitation.node_id, bob.node_id());

    let mut alice_client = connect(&alice_sock).await.expect("alice connect");
    let response = alice_client
        .connect_to_peer(ConnectToPeerRequest {
            invitation: invitation.invitation,
        })
        .await
        .expect("alice connect_to_peer")
        .into_inner();
    let peer = response.peer.expect("peer info populated");

    assert_eq!(peer.node_id, bob.node_id(), "node id matches Bob");
    assert_eq!(
        peer.state,
        PeerConnectionState::Connected as i32,
        "Alice should see Bob as connected, last_error={}",
        peer.last_error,
    );
    assert_eq!(peer.source, PeerSource::Manual as i32);

    // Bob should now see an inbound peer entry.
    wait_for_inbound(&mut bob_client).await;

    // Verify Alice's ListPeers includes Bob.
    let alice_peers = alice_client
        .list_peers(ListPeersRequest {})
        .await
        .expect("alice list_peers")
        .into_inner();
    assert!(
        alice_peers.peers.iter().any(|p| p.node_id == bob.node_id()),
        "Alice's peer list should include Bob (got {:?})",
        alice_peers.peers,
    );

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

#[tokio::test]
async fn real_two_daemons_discover_each_other_via_mdns() {
    if std::env::var("RENDEZVOUS_REAL_MDNS").as_deref() != Ok("1") {
        eprintln!("skipping real_ mDNS discovery test (set RENDEZVOUS_REAL_MDNS=1 to run)");
        return;
    }
    let tmp = TempDir::new().expect("tempdir");
    let (alice, alice_sock) = boot_daemon(&tmp, "alice", true).await;
    let (bob, _bob_sock) = boot_daemon(&tmp, "bob", true).await;

    let mut alice_client = connect(&alice_sock).await.expect("alice connect");

    let deadline = Instant::now() + Duration::from_secs(15);
    let target_node = bob.node_id();
    loop {
        let peers = alice_client
            .list_peers(ListPeersRequest {})
            .await
            .expect("alice list_peers")
            .into_inner();
        if peers.peers.iter().any(|p| p.node_id == target_node) {
            break;
        }
        if Instant::now() >= deadline {
            panic!(
                "Alice never discovered Bob ({target_node}) via mDNS; peers={:?}",
                peers.peers,
            );
        }
        sleep(Duration::from_millis(200)).await;
    }

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}

async fn wait_for_inbound(
    bob_client: &mut rendezvous_core::RendezvousClient<tonic::transport::Channel>,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let peers = bob_client
            .list_peers(ListPeersRequest {})
            .await
            .expect("bob list_peers")
            .into_inner();
        if peers
            .peers
            .iter()
            .any(|p| p.source == PeerSource::Inbound as i32)
        {
            break;
        }
        if Instant::now() >= deadline {
            panic!(
                "Bob never observed an inbound connection; peers={:?}",
                peers.peers,
            );
        }
        sleep(Duration::from_millis(50)).await;
    }
}

/// **mDNS unpaired data-exchange boundary** — Two daemons discover each
/// other over real mDNS. Before the operator approves the pairing, any
/// attempt to sync session-log deltas MUST fail and the receiver's redb
/// snapshot must remain unchanged. After approval, sync must succeed.
#[tokio::test]
async fn real_mdns_discovered_peer_cannot_sync_before_approval() {
    if std::env::var("RENDEZVOUS_REAL_MDNS").as_deref() != Ok("1") {
        eprintln!(
            "skipping real_ mDNS data-exchange boundary test (set RENDEZVOUS_REAL_MDNS=1 to run)"
        );
        return;
    }
    let tmp = TempDir::new().expect("tempdir");
    let (alice, alice_sock) = boot_daemon(&tmp, "alice", true).await;
    let (bob, bob_sock) = boot_daemon(&tmp, "bob", true).await;
    let alice_node = alice.node_id();
    let bob_node = bob.node_id();

    let mut alice_client = connect(&alice_sock).await.expect("alice connect");
    let mut bob_client = connect(&bob_sock).await.expect("bob connect");

    // Bob writes a "secret" entry in his own namespace.
    bob_client
        .append_entry(AppendEntryRequest {
            owner_node_id: String::new(),
            session_id: "secret".into(),
            source: "bob".into(),
            level: "info".into(),
            message: "do-not-leak".into(),
            metadata_json: String::new(),
        })
        .await
        .expect("bob append");

    // Wait for mDNS discovery to populate Alice's peer list.
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let peers = alice_client
            .list_peers(ListPeersRequest {})
            .await
            .expect("alice list_peers")
            .into_inner();
        if peers.peers.iter().any(|p| p.node_id == bob_node) {
            break;
        }
        if Instant::now() >= deadline {
            panic!(
                "Alice never discovered Bob ({bob_node}) via mDNS; peers={:?}",
                peers.peers,
            );
        }
        sleep(Duration::from_millis(200)).await;
    }

    // Attempt sync BEFORE pairing — must fail.
    let err = alice_client
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob_node.clone(),
        })
        .await
        .expect_err("sync before pairing must fail");
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);

    // Alice's view of Bob's namespace must be empty.
    let leaked_chunks = alice_client
        .list_session_chunks(ListSessionChunksRequest {
            owner_node_id: bob_node.clone(),
            session_id: "secret".into(),
        })
        .await
        .expect("list chunks")
        .into_inner()
        .chunk_ids;
    assert!(
        leaked_chunks.is_empty(),
        "unpaired peer must not see remote data; got chunks {leaked_chunks:?}",
    );

    // Now approve the pairing on both sides and verify sync works.
    alice_client
        .approve_peer(rendezvous_core::ApprovePeerRequest {
            node_id: bob_node.clone(),
            note: "bob".into(),
        })
        .await
        .expect("alice approves bob");
    bob_client
        .approve_peer(rendezvous_core::ApprovePeerRequest {
            node_id: alice_node.clone(),
            note: "alice".into(),
        })
        .await
        .expect("bob approves alice");

    // Connect via invitation (mDNS-discovered peers need explicit QUIC
    // connection for sync).
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
        .expect("alice connect");

    // Sync now succeeds.
    alice_client
        .sync_with_peer(SyncWithPeerRequest {
            node_id: bob_node.clone(),
        })
        .await
        .expect("sync after pairing must succeed");

    // Alice now sees Bob's data in Bob's namespace.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let chunks = alice_client
            .list_session_chunks(ListSessionChunksRequest {
                owner_node_id: bob_node.clone(),
                session_id: "secret".into(),
            })
            .await
            .expect("list chunks")
            .into_inner()
            .chunk_ids;
        if !chunks.is_empty() {
            let mut messages = Vec::new();
            for chunk in &chunks {
                let entries = alice_client
                    .list_chunk_entries(ListChunkEntriesRequest {
                        chunk_id: chunk.clone(),
                    })
                    .await
                    .expect("list entries")
                    .into_inner();
                messages.extend(entries.entries.into_iter().map(|e| e.message));
            }
            assert!(
                messages.contains(&"do-not-leak".to_string()),
                "after sync, alice should have bob's data; got {messages:?}",
            );
            break;
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for Bob's data to appear on Alice");
        }
        sleep(Duration::from_millis(100)).await;
    }

    alice.shutdown().await.expect("alice shutdown");
    bob.shutdown().await.expect("bob shutdown");
}
