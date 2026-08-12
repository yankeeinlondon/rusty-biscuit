//! CRDT document-schema and metadata invariants: entries must be
//! well-formed JSON with monotonic sequences, metadata must identify the
//! expected chunk, and a metadata field may never be mutated on an
//! already-accepted chunk (append-only). Each rejection must leave
//! storage untouched.

use super::*;

fn make_schema_invalid_snapshot_non_string_entry() -> Vec<u8> {
    let doc = LoroDoc::new();
    let list = doc.get_list(ENTRIES_CONTAINER);
    list.insert(0, 42).unwrap();
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

fn make_schema_invalid_snapshot_bad_json() -> Vec<u8> {
    let doc = LoroDoc::new();
    let list = doc.get_list(ENTRIES_CONTAINER);
    list.push("not valid entry json").unwrap();
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

fn make_schema_invalid_snapshot_non_monotonic() -> Vec<u8> {
    let doc = LoroDoc::new();
    let list = doc.get_list(ENTRIES_CONTAINER);
    let entry1 = Entry {
        sequence: 5,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "first".into(),
        metadata: None,
    };
    let entry2 = Entry {
        sequence: 3,
        created_at_unix_ms: 6_000,
        source: "remote".into(),
        level: "info".into(),
        message: "second".into(),
        metadata: None,
    };
    list.push(serde_json::to_string(&entry1).unwrap().as_str())
        .unwrap();
    list.push(serde_json::to_string(&entry2).unwrap().as_str())
        .unwrap();
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

#[test]
fn schema_invalid_non_string_entry_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "schema-non-string", 0);
    let snapshot = make_schema_invalid_snapshot_non_string_entry();
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
        PayloadKind::Snapshot,
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

#[test]
fn schema_invalid_bad_entry_json_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "schema-bad-json", 0);
    let snapshot = make_schema_invalid_snapshot_bad_json();
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
        PayloadKind::Snapshot,
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

#[test]
fn schema_invalid_non_monotonic_sequence_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "schema-non-monotonic", 0);
    let snapshot = make_schema_invalid_snapshot_non_monotonic();
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
        PayloadKind::Snapshot,
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

fn make_snapshot_missing_metadata(message: &str) -> Vec<u8> {
    let doc = LoroDoc::new();
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
    doc.export(ExportMode::Snapshot).unwrap()
}

fn make_snapshot_wrong_metadata(
    message: &str,
    owner: &str,
    session: &str,
    chunk_index: i64,
    created_at: i64,
    prev_chunk: Option<&str>,
) -> Vec<u8> {
    let doc = LoroDoc::new();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", owner).unwrap();
    map.insert("session_id", session).unwrap();
    map.insert("chunk_index", chunk_index).unwrap();
    map.insert("created_at_unix_ms", created_at).unwrap();
    if let Some(prev) = prev_chunk {
        map.insert("previous_chunk_id", prev).unwrap();
    }
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
    doc.export(ExportMode::Snapshot).unwrap()
}

#[test]
fn missing_metadata_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "no-meta-session", 0);
    let snapshot = make_snapshot_missing_metadata("no-meta");
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
        PayloadKind::Snapshot,
        &mut inbox,
    );

    assert!(
        result.is_err(),
        "snapshot without metadata must be rejected"
    );
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

#[test]
fn wrong_owner_in_metadata_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "wrong-owner-session", 0);
    let snapshot = make_snapshot_wrong_metadata(
        "wrong-owner",
        "wrong-owner-id",
        "wrong-owner-session",
        0,
        5_000,
        None,
    );
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
        PayloadKind::Snapshot,
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

#[test]
fn wrong_session_in_metadata_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "wrong-session-meta", 0);
    let snapshot = make_snapshot_wrong_metadata(
        "wrong-session",
        &peer_hex,
        "different-session",
        0,
        5_000,
        None,
    );
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
        PayloadKind::Snapshot,
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

#[test]
fn wrong_chunk_index_in_metadata_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "wrong-idx-session", 0);
    let snapshot =
        make_snapshot_wrong_metadata("wrong-idx", &peer_hex, "wrong-idx-session", 99, 5_000, None);
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
        PayloadKind::Snapshot,
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

#[test]
fn invalid_created_at_in_metadata_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "bad-ts-session", 0);
    let snapshot = make_snapshot_wrong_metadata("bad-ts", &peer_hex, "bad-ts-session", 0, -1, None);
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
        PayloadKind::Snapshot,
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

#[test]
fn wrong_previous_chunk_id_in_metadata_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "wrong-prev-session", 1);
    let snapshot = make_snapshot_wrong_metadata(
        "wrong-prev",
        &peer_hex,
        "wrong-prev-session",
        1,
        5_000,
        Some("wrong-previous-chunk-id"),
    );
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
        PayloadKind::Snapshot,
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

fn make_mutated_metadata_snapshot(
    original_snapshot: &[u8],
    field_to_mutate: &str,
    new_value: &str,
) -> Vec<u8> {
    let doc = LoroDoc::from_snapshot(original_snapshot).unwrap();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert(field_to_mutate, new_value).unwrap();
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

fn make_mutated_metadata_i64_snapshot(
    original_snapshot: &[u8],
    field_to_mutate: &str,
    new_value: i64,
) -> Vec<u8> {
    let doc = LoroDoc::from_snapshot(original_snapshot).unwrap();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert(field_to_mutate, new_value).unwrap();
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

#[test]
fn metadata_owner_mutation_on_existing_chunk_rejected() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "mutate-owner-session", 0);
    let valid_snapshot = make_valid_loro_snapshot("initial", &peer_hex, "mutate-owner-session", 0);

    let first_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        valid_snapshot.clone(),
    );
    let mut inbox = EnvelopeInbox::new();
    harness
        .service
        .receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &first_envelope,
            PayloadKind::Snapshot,
            &mut inbox,
        )
        .expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated = make_mutated_metadata_snapshot(&valid_snapshot, "owner_node_id", "impostor-node");
    let second_envelope = harness
        .peer_sealer
        .seal(chunk.as_path(), PayloadKind::Snapshot, mutated);
    let result = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &second_envelope,
        PayloadKind::Snapshot,
        &mut inbox,
    );
    assert!(
        result.is_err(),
        "mutated owner_node_id must be rejected on existing chunk"
    );
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_after
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_after
    );
    harness.worker.shutdown();
}

#[test]
fn metadata_session_mutation_on_existing_chunk_rejected() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "mutate-session-session", 0);
    let valid_snapshot =
        make_valid_loro_snapshot("initial", &peer_hex, "mutate-session-session", 0);

    let first_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        valid_snapshot.clone(),
    );
    let mut inbox = EnvelopeInbox::new();
    harness
        .service
        .receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &first_envelope,
            PayloadKind::Snapshot,
            &mut inbox,
        )
        .expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated =
        make_mutated_metadata_snapshot(&valid_snapshot, "session_id", "different-session");
    let second_envelope = harness
        .peer_sealer
        .seal(chunk.as_path(), PayloadKind::Snapshot, mutated);
    let result = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &second_envelope,
        PayloadKind::Snapshot,
        &mut inbox,
    );
    assert!(
        result.is_err(),
        "mutated session_id must be rejected on existing chunk"
    );
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_after
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_after
    );
    harness.worker.shutdown();
}

#[test]
fn metadata_chunk_index_mutation_on_existing_chunk_rejected() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "mutate-idx-session", 0);
    let valid_snapshot = make_valid_loro_snapshot("initial", &peer_hex, "mutate-idx-session", 0);

    let first_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        valid_snapshot.clone(),
    );
    let mut inbox = EnvelopeInbox::new();
    harness
        .service
        .receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &first_envelope,
            PayloadKind::Snapshot,
            &mut inbox,
        )
        .expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated = make_mutated_metadata_i64_snapshot(&valid_snapshot, "chunk_index", 99);
    let second_envelope = harness
        .peer_sealer
        .seal(chunk.as_path(), PayloadKind::Snapshot, mutated);
    let result = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &second_envelope,
        PayloadKind::Snapshot,
        &mut inbox,
    );
    assert!(
        result.is_err(),
        "mutated chunk_index must be rejected on existing chunk"
    );
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_after
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_after
    );
    harness.worker.shutdown();
}

#[test]
fn metadata_created_at_mutation_on_existing_chunk_rejected() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "mutate-ts-session", 0);
    let valid_snapshot = make_valid_loro_snapshot("initial", &peer_hex, "mutate-ts-session", 0);

    let first_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        valid_snapshot.clone(),
    );
    let mut inbox = EnvelopeInbox::new();
    harness
        .service
        .receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &first_envelope,
            PayloadKind::Snapshot,
            &mut inbox,
        )
        .expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated = make_mutated_metadata_i64_snapshot(&valid_snapshot, "created_at_unix_ms", 99_999);
    let second_envelope = harness
        .peer_sealer
        .seal(chunk.as_path(), PayloadKind::Snapshot, mutated);
    let result = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &second_envelope,
        PayloadKind::Snapshot,
        &mut inbox,
    );
    assert!(
        result.is_err(),
        "mutated created_at_unix_ms must be rejected on existing chunk"
    );
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_after
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_after
    );
    harness.worker.shutdown();
}

#[test]
fn metadata_previous_chunk_mutation_on_existing_chunk_rejected() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "mutate-prev-session", 0);
    let valid_snapshot = make_valid_loro_snapshot("initial", &peer_hex, "mutate-prev-session", 0);

    let first_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        valid_snapshot.clone(),
    );
    let mut inbox = EnvelopeInbox::new();
    harness
        .service
        .receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &first_envelope,
            PayloadKind::Snapshot,
            &mut inbox,
        )
        .expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated =
        make_mutated_metadata_snapshot(&valid_snapshot, "previous_chunk_id", "bogus-chunk");
    let second_envelope = harness
        .peer_sealer
        .seal(chunk.as_path(), PayloadKind::Snapshot, mutated);
    let result = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &second_envelope,
        PayloadKind::Snapshot,
        &mut inbox,
    );
    assert!(
        result.is_err(),
        "injected previous_chunk_id must be rejected on existing chunk"
    );
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_after
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_after
    );
    harness.worker.shutdown();
}
