//! Snapshot-replace path: when an owner re-bases (history-compacts) its
//! document past our version it sends a self-contained snapshot the
//! replica must adopt wholesale. The signature-covered payload kind is
//! authoritative, so a frame that claims replace over a plain-snapshot
//! envelope is rejected.

use super::*;

/// Owner-side live document for re-base scenarios (same shape as
/// [`make_valid_loro_snapshot`] but returned live so the test can
/// extend the lineage and export shallow snapshots from it).
fn make_owner_doc(owner_node_id: &str, session_id: &str, message: &str) -> LoroDoc {
    let doc = LoroDoc::new();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", owner_node_id).unwrap();
    map.insert("session_id", session_id).unwrap();
    map.insert("chunk_index", 0i64).unwrap();
    map.insert("created_at_unix_ms", 5_000_i64).unwrap();
    let list = doc.get_list(ENTRIES_CONTAINER);
    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: message.into(),
        metadata: None,
    };
    list.push(serde_json::to_string(&entry).unwrap().as_str())
        .unwrap();
    doc.commit();
    doc
}

#[test]
fn snapshot_replace_swaps_replica_after_owner_rebase() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "rebase-session", 0);
    let mut inbox = EnvelopeInbox::new();

    let doc = make_owner_doc(&peer_hex, "rebase-session", "first");
    let initial = doc.export(ExportMode::Snapshot).unwrap();
    let envelope = harness
        .peer_sealer
        .seal(chunk.as_path(), PayloadKind::Snapshot, initial);
    harness
        .service
        .receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            PayloadKind::Snapshot,
            &mut inbox,
        )
        .expect("initial snapshot");

    // The owner appends and re-bases: the shallow export drops all
    // history before the current frontier.
    let list = doc.get_list(ENTRIES_CONTAINER);
    let entry = Entry {
        sequence: 1,
        created_at_unix_ms: 5_001,
        source: "remote".into(),
        level: "info".into(),
        message: "second".into(),
        metadata: None,
    };
    list.push(serde_json::to_string(&entry).unwrap().as_str())
        .unwrap();
    doc.commit();
    let frontier = doc.oplog_frontiers();
    let shallow = doc.export(ExportMode::shallow_snapshot(&frontier)).unwrap();
    let envelope = harness
        .peer_sealer
        .seal(chunk.as_path(), PayloadKind::SnapshotReplace, shallow);

    let advanced = harness
        .service
        .receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            PayloadKind::SnapshotReplace,
            &mut inbox,
        )
        .expect("snapshot-replace");
    assert!(advanced);

    let messages: Vec<String> = harness
        .manager
        .list_chunk_entries(&chunk)
        .expect("entries")
        .into_iter()
        .map(|e| e.message)
        .collect();
    assert_eq!(messages, vec!["first".to_string(), "second".to_string()]);
    harness.worker.shutdown();
}

#[test]
fn replace_frame_with_mismatched_envelope_kind_rejected() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "replace-kind-session", 0);
    let snapshot = make_valid_loro_snapshot("kind", &peer_hex, "replace-kind-session", 0);
    // Envelope signed as a plain Snapshot, but the frame claims
    // replace semantics: the signature-covered kind wins.
    let envelope = harness
        .peer_sealer
        .seal(chunk.as_path(), PayloadKind::Snapshot, snapshot);

    let snapshots_before = baseline_snapshot_count(&harness);
    let envelopes_before = baseline_envelope_count(&harness);
    let mut inbox = EnvelopeInbox::new();

    let result = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &envelope,
        PayloadKind::SnapshotReplace,
        &mut inbox,
    );

    assert!(result.is_err());
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_before,
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_before,
    );
    harness.worker.shutdown();
}
