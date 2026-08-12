//! Thin request/response RPCs: echoing a ping nonce with the daemon
//! version, reporting the local node id, and the append→list round trip.

use super::*;

#[tokio::test]
async fn ping_echoes_nonce_and_reports_version() {
    let h = harness();
    let response = h
        .service
        .ping(Request::new(PingRequest {
            nonce: "abc-123".into(),
        }))
        .await
        .expect("ping ok");
    let body = response.into_inner();
    assert_eq!(body.nonce, "abc-123");
    assert_eq!(body.daemon_version, DAEMON_VERSION);
    assert!(body.timestamp_unix_ms > 0);
}

#[tokio::test]
async fn status_reports_the_local_node_id() {
    let h = harness();
    let body = h
        .service
        .status(Request::new(StatusRequest {}))
        .await
        .expect("status ok")
        .into_inner();
    // The dashboard uses this to tell its own registers (always
    // fresh) apart from synced peer replicas.
    assert_eq!(body.node_id, h.service.identity.node_id());
    assert!(!body.node_id.is_empty());
}

#[tokio::test]
async fn append_then_list_chunk_entries() {
    let h = harness();
    let node_id = h.service.identity.node_id();
    h.service
        .append_entry(Request::new(AppendEntryRequest {
            owner_node_id: String::new(),
            session_id: "s-1".into(),
            source: "test".into(),
            level: "info".into(),
            message: "hello".into(),
            metadata_json: String::new(),
        }))
        .await
        .expect("append");
    let chunk_id = format!("session/{node_id}/s-1/part/0");
    let listed = h
        .service
        .list_chunk_entries(Request::new(ListChunkEntriesRequest {
            chunk_id: chunk_id.clone(),
        }))
        .await
        .expect("list")
        .into_inner();
    assert_eq!(listed.entries.len(), 1);
    assert_eq!(listed.entries[0].message, "hello");
}
