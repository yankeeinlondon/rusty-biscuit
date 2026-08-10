//! replace update session-log tests.

use super::*;

fn remote_entry(sequence: u64, message: &str) -> Entry {
    Entry {
        sequence,
        created_at_unix_ms: 5_000 + sequence as i64,
        source: "remote".into(),
        level: "info".into(),
        message: message.into(),
        metadata: None,
    }
}

/// Owner-side document builder for re-base scenarios: metadata for
/// chunk 0 plus one initial entry, returned live so tests can extend
/// the same lineage and export shallow snapshots from it.
fn owner_doc_with_first_entry(owner: &str, session: &str) -> loro::LoroDoc {
    let doc = loro::LoroDoc::new();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", owner).unwrap();
    map.insert("session_id", session).unwrap();
    map.insert("chunk_index", 0i64).unwrap();
    map.insert("created_at_unix_ms", 5_000_i64).unwrap();
    let list = doc.get_list(ENTRIES_CONTAINER);
    list.push(
        serde_json::to_string(&remote_entry(0, "first"))
            .unwrap()
            .as_str(),
    )
    .unwrap();
    doc.commit();
    doc
}

fn push_entry(doc: &loro::LoroDoc, sequence: u64, message: &str) {
    let list = doc.get_list(ENTRIES_CONTAINER);
    list.push(
        serde_json::to_string(&remote_entry(sequence, message))
            .unwrap()
            .as_str(),
    )
    .unwrap();
    doc.commit();
}

#[test]
fn export_updates_since_sends_replace_when_peer_is_behind_shallow_root() {
    let harness = build_harness(ChunkConfig::default());
    let chunk = ChunkId::new("shallow-owner", "shallow-session", 0);

    let doc = owner_doc_with_first_entry("shallow-owner", "shallow-session");
    let stale_vv = doc.oplog_vv();
    // Two commits past the stale capture: the shallow root lands at the
    // final frontier, so the history connecting the stale peer's version
    // to the root is genuinely gone (a single commit would leave the
    // root exactly at the stale version, which deltas can still serve).
    push_entry(&doc, 1, "second");
    push_entry(&doc, 2, "third");
    let head_vv = doc.oplog_vv();
    let frontier = doc.oplog_frontiers();
    let shallow = doc.export(ExportMode::shallow_snapshot(&frontier)).unwrap();
    harness
        .storage
        .save_snapshot(&chunk, &shallow)
        .expect("persist shallow chunk");

    // Peer behind the shallow root: delta history is gone, so the only
    // safe answer is a snapshot the peer adopts wholesale.
    let exported = harness
        .manager
        .export_updates_since(&chunk, Some(&stale_vv.encode()))
        .expect("export")
        .expect("chunk exists");
    assert_eq!(exported.kind, PayloadKind::SnapshotReplace);
    assert!(!exported.bytes.is_empty());

    // Peer at the head: normal delta path (the bytes may be a small
    // non-empty header blob — Loro's updates export from a shallow doc
    // is not zero-length even with no new ops — but the kind is what
    // decides replace-vs-merge semantics on the wire).
    let exported = harness
        .manager
        .export_updates_since(&chunk, Some(&head_vv.encode()))
        .expect("export")
        .expect("chunk exists");
    assert_eq!(exported.kind, PayloadKind::Delta);

    // Peer with no copy at all: plain full snapshot, not a replace.
    let exported = harness
        .manager
        .export_updates_since(&chunk, None)
        .expect("export")
        .expect("chunk exists");
    assert_eq!(exported.kind, PayloadKind::Snapshot);

    harness.worker.shutdown();
}

#[test]
fn apply_remote_replace_adopts_rebased_snapshot() {
    let harness = build_harness(ChunkConfig::default());
    let chunk = ChunkId::new("rebase-owner", "rebase-session", 0);

    let doc = owner_doc_with_first_entry("rebase-owner", "rebase-session");
    let initial = doc.export(ExportMode::Snapshot).unwrap();
    assert!(
        harness
            .manager
            .apply_remote_update(&chunk, &initial)
            .expect("initial")
    );

    push_entry(&doc, 1, "second");
    let frontier = doc.oplog_frontiers();
    let shallow = doc.export(ExportMode::shallow_snapshot(&frontier)).unwrap();

    let advanced = harness
        .manager
        .apply_remote_replace(&chunk, &shallow)
        .expect("replace");
    assert!(advanced);
    let messages: Vec<String> = harness
        .manager
        .list_chunk_entries(&chunk)
        .expect("entries")
        .into_iter()
        .map(|e| e.message)
        .collect();
    assert_eq!(messages, vec!["first".to_string(), "second".to_string()]);

    // Re-delivering the same replace is a no-op, not a regression.
    let advanced = harness
        .manager
        .apply_remote_replace(&chunk, &shallow)
        .expect("idempotent");
    assert!(!advanced);

    harness.worker.shutdown();
}

#[test]
fn apply_remote_replace_rejects_rewritten_prefix() {
    let harness = build_harness(ChunkConfig::default());
    let chunk = ChunkId::new("forge-owner", "forge-session", 0);

    let doc = owner_doc_with_first_entry("forge-owner", "forge-session");
    let initial = doc.export(ExportMode::Snapshot).unwrap();
    assert!(
        harness
            .manager
            .apply_remote_update(&chunk, &initial)
            .expect("initial")
    );

    // Forged replacement with identical metadata but a rewritten
    // history: replace may extend the entry list, never mutate it.
    let forged = loro::LoroDoc::new();
    let map = forged.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", "forge-owner").unwrap();
    map.insert("session_id", "forge-session").unwrap();
    map.insert("chunk_index", 0i64).unwrap();
    map.insert("created_at_unix_ms", 5_000_i64).unwrap();
    let list = forged.get_list(ENTRIES_CONTAINER);
    list.push(
        serde_json::to_string(&remote_entry(0, "REWRITTEN"))
            .unwrap()
            .as_str(),
    )
    .unwrap();
    forged.commit();
    let forged_snapshot = forged.export(ExportMode::Snapshot).unwrap();

    let result = harness
        .manager
        .apply_remote_replace(&chunk, &forged_snapshot);
    assert!(
        matches!(result, Err(SessionLogError::SchemaValidation { .. })),
        "rewritten prefix must be rejected; got {result:?}"
    );
    let messages: Vec<String> = harness
        .manager
        .list_chunk_entries(&chunk)
        .expect("entries")
        .into_iter()
        .map(|e| e.message)
        .collect();
    assert_eq!(messages, vec!["first".to_string()]);

    harness.worker.shutdown();
}

#[test]
fn apply_remote_update_rejects_disconnected_history() {
    let harness = build_harness(ChunkConfig::default());
    let chunk = ChunkId::new("gap-owner", "gap-session", 0);

    let doc = owner_doc_with_first_entry("gap-owner", "gap-session");
    let initial = doc.export(ExportMode::Snapshot).unwrap();
    assert!(
        harness
            .manager
            .apply_remote_update(&chunk, &initial)
            .expect("initial")
    );

    // The owner advances twice, but the delta is exported only from the
    // intermediate version — its causal dependencies never reach us.
    // Loro reports this import as Ok-with-pending; the guard must
    // surface it as a failed sync instead of silently not converging.
    push_entry(&doc, 1, "second");
    let mid_vv = doc.oplog_vv();
    push_entry(&doc, 2, "third");
    let gap_delta = doc.export(ExportMode::updates(&mid_vv)).unwrap();

    let result = harness.manager.apply_remote_update(&chunk, &gap_delta);
    assert!(
        matches!(result, Err(SessionLogError::PendingRemoteOps { .. })),
        "disconnected delta must trip the pending guard; got {result:?}"
    );
    let messages: Vec<String> = harness
        .manager
        .list_chunk_entries(&chunk)
        .expect("entries")
        .into_iter()
        .map(|e| e.message)
        .collect();
    assert_eq!(messages, vec!["first".to_string()]);

    harness.worker.shutdown();
}
