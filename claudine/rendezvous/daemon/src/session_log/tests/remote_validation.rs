//! remote validation session-log tests.

use super::*;

#[test]
fn existing_entry_message_mutation_rejected_as_append_only_violation() {
    let harness = build_harness(ChunkConfig::default());
    let remote_node = "remote-node";
    let chunk = ChunkId::new(remote_node, "append-only-mutate", 0);

    let entry1 = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "original".into(),
        metadata: None,
    };
    let snapshot1 = make_remote_snapshot(remote_node, "append-only-mutate", 0, &[entry1]);
    harness
        .manager
        .apply_remote_update(&chunk, &snapshot1)
        .expect("first apply");

    let entries_before = harness.manager.list_chunk_entries(&chunk).expect("list");
    assert_eq!(entries_before.len(), 1);
    assert_eq!(entries_before[0].message, "original");

    let doc = LoroDoc::from_snapshot(&snapshot1).unwrap();
    let list = doc.get_list(ENTRIES_CONTAINER);
    list.delete(0, 1).unwrap();
    let mutated_entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "mutated".into(),
        metadata: None,
    };
    list.insert(
        0,
        serde_json::to_string(&mutated_entry).unwrap().as_str(),
    )
    .unwrap();
    doc.commit();
    let snapshot2 = doc.export(ExportMode::Snapshot).unwrap();

    let result = harness.manager.apply_remote_update(&chunk, &snapshot2);
    assert!(
        result.is_err(),
        "mutation of existing entry must be rejected"
    );

    let entries_after = harness.manager.list_chunk_entries(&chunk).expect("list");
    assert_eq!(entries_after.len(), 1);
    assert_eq!(entries_after[0].message, "original");

    harness.worker.shutdown();
}

#[test]
fn existing_entry_deletion_with_replacement_rejected_as_append_only_violation() {
    let harness = build_harness(ChunkConfig::default());
    let remote_node = "remote-node";
    let chunk = ChunkId::new(remote_node, "append-only-replace", 0);

    let entry_a = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "alpha".into(),
        metadata: None,
    };
    let entry_b = Entry {
        sequence: 1,
        created_at_unix_ms: 6_000,
        source: "remote".into(),
        level: "info".into(),
        message: "beta".into(),
        metadata: None,
    };
    let snapshot1 =
        make_remote_snapshot(remote_node, "append-only-replace", 0, &[entry_a, entry_b]);
    harness
        .manager
        .apply_remote_update(&chunk, &snapshot1)
        .expect("first apply");

    let snapshots_before = harness.storage.snapshot_count().expect("count");

    let doc = LoroDoc::from_snapshot(&snapshot1).unwrap();
    let list = doc.get_list(ENTRIES_CONTAINER);
    list.delete(0, 1).unwrap();
    let replacement = Entry {
        sequence: 0,
        created_at_unix_ms: 9_000,
        source: "attacker".into(),
        level: "warn".into(),
        message: "replaced".into(),
        metadata: None,
    };
    list.insert(
        0,
        serde_json::to_string(&replacement).unwrap().as_str(),
    )
    .unwrap();
    doc.commit();
    let snapshot2 = doc.export(ExportMode::Snapshot).unwrap();

    let result = harness.manager.apply_remote_update(&chunk, &snapshot2);
    assert!(
        result.is_err(),
        "deletion+replacement of existing entry must be rejected"
    );

    let entries_after = harness.manager.list_chunk_entries(&chunk).expect("list");
    assert_eq!(entries_after.len(), 2);
    assert_eq!(entries_after[0].message, "alpha");
    assert_eq!(entries_after[1].message, "beta");
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_before,
    );

    harness.worker.shutdown();
}

#[test]
fn existing_entry_reordering_rejected_as_append_only_violation() {
    let harness = build_harness(ChunkConfig::default());
    let remote_node = "remote-node";
    let chunk = ChunkId::new(remote_node, "append-only-reorder", 0);

    let entry_a = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "alpha".into(),
        metadata: None,
    };
    let entry_b = Entry {
        sequence: 1,
        created_at_unix_ms: 6_000,
        source: "remote".into(),
        level: "info".into(),
        message: "beta".into(),
        metadata: None,
    };
    let snapshot1 =
        make_remote_snapshot(remote_node, "append-only-reorder", 0, &[entry_a, entry_b]);
    harness
        .manager
        .apply_remote_update(&chunk, &snapshot1)
        .expect("first apply");

    let doc = LoroDoc::from_snapshot(&snapshot1).unwrap();
    let list = doc.get_list(ENTRIES_CONTAINER);
    list.delete(0, 2).unwrap();
    let swapped_b = Entry {
        sequence: 0,
        created_at_unix_ms: 6_000,
        source: "remote".into(),
        level: "info".into(),
        message: "beta".into(),
        metadata: None,
    };
    let swapped_a = Entry {
        sequence: 1,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "alpha".into(),
        metadata: None,
    };
    list.insert(
        0,
        serde_json::to_string(&swapped_b).unwrap().as_str(),
    )
    .unwrap();
    list.insert(
        1,
        serde_json::to_string(&swapped_a).unwrap().as_str(),
    )
    .unwrap();
    doc.commit();
    let snapshot2 = doc.export(ExportMode::Snapshot).unwrap();

    let result = harness.manager.apply_remote_update(&chunk, &snapshot2);
    assert!(
        result.is_err(),
        "reordering of existing entries must be rejected"
    );

    let entries_after = harness.manager.list_chunk_entries(&chunk).expect("list");
    assert_eq!(entries_after[0].message, "alpha");
    assert_eq!(entries_after[1].message, "beta");

    harness.worker.shutdown();
}

#[test]
fn first_snapshot_with_previous_chunk_id_on_chunk_0_rejected() {
    let harness = build_harness(ChunkConfig::default());
    let remote_node = "remote-node";
    let chunk = ChunkId::new(remote_node, "prev-id-session", 0);

    let doc = loro::LoroDoc::new();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", remote_node).unwrap();
    map.insert("session_id", "prev-id-session").unwrap();
    map.insert("chunk_index", 0i64).unwrap();
    map.insert("created_at_unix_ms", 5_000i64).unwrap();
    map.insert("previous_chunk_id", "bogus-prev-chunk").unwrap();
    let list = doc.get_list(ENTRIES_CONTAINER);
    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "bad-prev".into(),
        metadata: None,
    };
    list.push(serde_json::to_string(&entry).unwrap().as_str())
        .unwrap();
    doc.commit();
    let snapshot = doc.export(ExportMode::Snapshot).unwrap();

    let result = harness.manager.apply_remote_update(&chunk, &snapshot);
    assert!(
        result.is_err(),
        "first snapshot with previous_chunk_id for chunk 0 must be rejected"
    );

    let entries = harness.manager.list_chunk_entries(&chunk).expect("list");
    assert!(
        entries.is_empty(),
        "no entries should be visible; got {entries:?}"
    );

    harness.worker.shutdown();
}

#[test]
fn first_snapshot_with_integer_previous_chunk_id_on_chunk_0_rejected() {
    let harness = build_harness(ChunkConfig::default());
    let remote_node = "remote-node";
    let chunk = ChunkId::new(remote_node, "prev-int-session", 0);

    let doc = loro::LoroDoc::new();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", remote_node).unwrap();
    map.insert("session_id", "prev-int-session").unwrap();
    map.insert("chunk_index", 0i64).unwrap();
    map.insert("created_at_unix_ms", 5_000i64).unwrap();
    map.insert("previous_chunk_id", 999i64).unwrap();
    let list = doc.get_list(ENTRIES_CONTAINER);
    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "int-prev".into(),
        metadata: None,
    };
    list.push(serde_json::to_string(&entry).unwrap().as_str())
        .unwrap();
    doc.commit();
    let snapshot = doc.export(ExportMode::Snapshot).unwrap();

    let result = harness.manager.apply_remote_update(&chunk, &snapshot);
    assert!(
        result.is_err(),
        "first snapshot with integer previous_chunk_id for chunk 0 must be rejected"
    );

    harness.worker.shutdown();
}

#[test]
fn existing_chunk_0_with_integer_previous_chunk_id_rejected() {
    let harness = build_harness(ChunkConfig::default());
    let remote_node = "remote-node";
    let chunk = ChunkId::new(remote_node, "exist-prev-int", 0);

    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "initial".into(),
        metadata: None,
    };
    let snapshot1 = make_remote_snapshot(remote_node, "exist-prev-int", 0, &[entry]);
    harness
        .manager
        .apply_remote_update(&chunk, &snapshot1)
        .expect("first apply");

    let doc = LoroDoc::from_snapshot(&snapshot1).unwrap();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert("previous_chunk_id", 42i64).unwrap();
    doc.commit();
    let snapshot2 = doc.export(ExportMode::Snapshot).unwrap();

    let result = harness.manager.apply_remote_update(&chunk, &snapshot2);
    assert!(
        result.is_err(),
        "existing chunk 0 with injected integer previous_chunk_id must be rejected"
    );

    let entries = harness.manager.list_chunk_entries(&chunk).expect("list");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message, "initial");

    harness.worker.shutdown();
}

