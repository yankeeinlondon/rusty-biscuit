use super::*;
use crate::batcher::{BatcherConfig, BatcherWorker, spawn};
use crate::projection::Projection;
use crate::session_log::Clock;
use loro::{ExportMode, LoroDoc};
use rendezvous_core::{
    ChunkConfig, ChunkId, EnvelopeSealer, Entry, NodeIdentity, PayloadKind,
};

const ENTRIES_CONTAINER: &str = "entries";
const METADATA_CONTAINER: &str = "metadata";
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

struct FixedClock {
    next: AtomicI64,
}

impl FixedClock {
    fn new(start: i64) -> Self {
        Self {
            next: AtomicI64::new(start),
        }
    }
}

impl Clock for FixedClock {
    fn now_unix_ms(&self) -> i64 {
        self.next.fetch_add(1, Ordering::SeqCst)
    }
}

struct Harness {
    service: SyncService,
    manager: crate::session_log::SessionLogManager,
    storage: Storage,
    peer_identity: NodeIdentity,
    peer_sealer: EnvelopeSealer,
    worker: BatcherWorker,
    _tmp: TempDir,
}

fn build_harness() -> Harness {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("open storage");
    let projection = Projection::in_memory().expect("projection");
    let worker = spawn(projection.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });

    let identity = Arc::new(NodeIdentity::from_seed([10u8; 32]));

    let manager = crate::session_log::SessionLogManager::with_clock(
        storage.clone(),
        worker.handle(),
        projection,
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(1_000)),
    )
    .expect("manager");

    let peer_identity = NodeIdentity::from_seed([20u8; 32]);
    let peer_hex = peer_identity.node_id();
    storage
        .upsert_pairing(&peer_hex, 1_000, "test peer")
        .expect("pair");

    let registers = RegisterStore::new(storage.clone(), Arc::clone(&identity))
        .expect("registers");
    let service = SyncService::new(manager.clone(), registers, storage.clone(), identity);
    let peer_sealer = EnvelopeSealer::new(peer_identity.clone());

    Harness {
        service,
        manager,
        storage,
        peer_identity,
        peer_sealer,
        worker,
        _tmp: tmp,
    }
}

fn make_valid_loro_snapshot(
    message: &str,
    owner_node_id: &str,
    session_id: &str,
    chunk_index: u64,
) -> Vec<u8> {
    let doc = LoroDoc::new();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", owner_node_id).unwrap();
    map.insert("session_id", session_id).unwrap();
    map.insert("chunk_index", chunk_index as i64).unwrap();
    map.insert("created_at_unix_ms", 5_000_i64).unwrap();
    if chunk_index > 0 {
        let prev_chunk = ChunkId::new(
            owner_node_id.to_string(),
            session_id.to_string(),
            chunk_index - 1,
        );
        map.insert("previous_chunk_id", prev_chunk.as_path().as_str()).unwrap();
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
    list.push(serde_json::to_string(&entry).unwrap().as_str()).unwrap();
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

fn baseline_snapshot_count(harness: &Harness) -> u64 {
    harness.storage.snapshot_count().expect("snapshot count")
}

fn baseline_envelope_count(harness: &Harness) -> u64 {
    harness.storage.accepted_envelope_count().expect("envelope count")
}

#[test]
fn valid_delta_persists_accepted_envelope_and_snapshot() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "test-session", 0);
    let snapshot = make_valid_loro_snapshot("hello", &peer_hex, "test-session", 0);
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot.clone(),
    );
    let mut inbox = EnvelopeInbox::new();

    let advanced = harness
        .service
        .receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            PayloadKind::Snapshot,
            &mut inbox,
        )
        .expect("receive_delta");

    assert!(advanced);
    assert!(
        harness.storage.snapshot_count().expect("count") >= 1,
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        1,
    );
    harness.worker.shutdown();
}

#[test]
fn malformed_crdt_payload_leaves_no_accepted_envelope_or_snapshot() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "malformed-session", 0);
    let garbage = b"not valid loro data at all".to_vec();
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        garbage,
    );
    let mut inbox = EnvelopeInbox::new();

    let snapshots_before = baseline_snapshot_count(&harness);
    let envelopes_before = baseline_envelope_count(&harness);

    let result = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &envelope,
        PayloadKind::Snapshot,
        &mut inbox,
    );

    assert!(result.is_err(), "malformed payload must be rejected");
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_before,
        "no snapshot should be persisted for malformed payload",
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_before,
        "no accepted envelope should be persisted for malformed payload",
    );
    harness.worker.shutdown();
}

#[test]
fn invalid_signature_rejected_without_storage_mutation() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "sig-session", 0);
    let snapshot = make_valid_loro_snapshot("sig-test", &peer_hex, "sig-session", 0);
    let mut envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );
    envelope.signature[0] ^= 0xFF;

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
fn payload_hash_mismatch_rejected_without_storage_mutation() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "hash-session", 0);
    let snapshot = make_valid_loro_snapshot("hash-test", &peer_hex, "hash-session", 0);
    let mut envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );
    envelope.payload = b"tampered payload".to_vec();

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
fn mismatched_sender_rejected_without_storage_mutation() {
    let harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "sender-session", 0);
    let snapshot = make_valid_loro_snapshot("sender-test", &peer_hex, "sender-session", 0);

    let impostor = NodeIdentity::from_seed([99u8; 32]);
    let mut impostor_sealer = EnvelopeSealer::new(impostor);
    let envelope = impostor_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
fn foreign_namespace_rejected_without_storage_mutation() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let local_hex = harness.service.node_id();
    let foreign_chunk = ChunkId::new(&local_hex, "stolen-session", 0);
    let snapshot = make_valid_loro_snapshot("namespace-test", &peer_hex, "stolen-session", 0);
    let envelope = harness.peer_sealer.seal(
        foreign_chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

    let snapshots_before = baseline_snapshot_count(&harness);
    let envelopes_before = baseline_envelope_count(&harness);
    let mut inbox = EnvelopeInbox::new();

    let result = harness.service.receive_delta(
        &peer_hex,
        &foreign_chunk,
        &foreign_chunk.as_path(),
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
fn duplicate_message_id_rejected_without_storage_mutation() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "dup-session", 0);
    let snapshot = make_valid_loro_snapshot("dup-test", &peer_hex, "dup-session", 0);
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot.clone(),
    );

    let mut inbox = EnvelopeInbox::new();

    let first = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &envelope,
        PayloadKind::Snapshot,
        &mut inbox,
    );
    assert!(first.is_ok());

    let snapshots_after_first = harness.storage.snapshot_count().expect("count");
    let envelopes_after_first = harness.storage.accepted_envelope_count().expect("count");

    let second = harness.service.receive_delta(
        &peer_hex,
        &chunk,
        &chunk.as_path(),
        &envelope,
        PayloadKind::Snapshot,
        &mut inbox,
    );
    assert!(second.is_err(), "duplicate must be rejected");

    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_after_first,
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_after_first,
    );
    harness.worker.shutdown();
}

#[test]
fn payload_kind_mismatch_rejected_without_storage_mutation() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "kind-session", 0);
    let snapshot = make_valid_loro_snapshot("kind-test", &peer_hex, "kind-session", 0);
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Delta,
        snapshot,
    );

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
fn document_id_mismatch_rejected_without_storage_mutation() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "docid-session", 0);
    let other_chunk = ChunkId::new(&peer_hex, "other-session", 0);
    let snapshot = make_valid_loro_snapshot("docid-test", &peer_hex, "docid-session", 0);
    let envelope = harness.peer_sealer.seal(
        other_chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
fn envelope_persistence_failure_leaves_no_snapshot_nor_envelope() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "envelope-fail-session", 0);
    let snapshot = make_valid_loro_snapshot("envelope-fail-entry", &peer_hex, "envelope-fail-session", 0);
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

    let snapshots_before = baseline_snapshot_count(&harness);
    let envelopes_before = baseline_envelope_count(&harness);

    harness.storage.inject_accepted_envelope_failure();
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
        "receive_delta must fail when envelope persistence fails",
    );
    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        snapshots_before,
        "no snapshot should be persisted when envelope save fails",
    );
    assert_eq!(
        harness.storage.accepted_envelope_count().expect("count"),
        envelopes_before,
        "no accepted envelope should exist when envelope save fails",
    );
    harness.worker.shutdown();
}

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
    list.push(serde_json::to_string(&entry1).unwrap().as_str()).unwrap();
    list.push(serde_json::to_string(&entry2).unwrap().as_str()).unwrap();
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

#[test]
fn schema_invalid_non_string_entry_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "schema-non-string", 0);
    let snapshot = make_schema_invalid_snapshot_non_string_entry();
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
    list.push(serde_json::to_string(&entry).unwrap().as_str()).unwrap();
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
    list.push(serde_json::to_string(&entry).unwrap().as_str()).unwrap();
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

#[test]
fn missing_metadata_rejected_without_persistence() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "no-meta-session", 0);
    let snapshot = make_snapshot_missing_metadata("no-meta");
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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

    assert!(result.is_err(), "snapshot without metadata must be rejected");
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
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
    let snapshot = make_snapshot_wrong_metadata(
        "wrong-idx",
        &peer_hex,
        "wrong-idx-session",
        99,
        5_000,
        None,
    );
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
    let snapshot = make_snapshot_wrong_metadata(
        "bad-ts",
        &peer_hex,
        "bad-ts-session",
        0,
        -1,
        None,
    );
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
    harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &first_envelope, PayloadKind::Snapshot, &mut inbox,
    ).expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated = make_mutated_metadata_snapshot(&valid_snapshot, "owner_node_id", "impostor-node");
    let second_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        mutated,
    );
    let result = harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &second_envelope, PayloadKind::Snapshot, &mut inbox,
    );
    assert!(result.is_err(), "mutated owner_node_id must be rejected on existing chunk");
    assert_eq!(harness.storage.snapshot_count().expect("count"), snapshots_after);
    assert_eq!(harness.storage.accepted_envelope_count().expect("count"), envelopes_after);
    harness.worker.shutdown();
}

#[test]
fn metadata_session_mutation_on_existing_chunk_rejected() {
    let mut harness = build_harness();
    let peer_hex = harness.peer_identity.node_id();
    let chunk = ChunkId::new(&peer_hex, "mutate-session-session", 0);
    let valid_snapshot = make_valid_loro_snapshot("initial", &peer_hex, "mutate-session-session", 0);

    let first_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        valid_snapshot.clone(),
    );
    let mut inbox = EnvelopeInbox::new();
    harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &first_envelope, PayloadKind::Snapshot, &mut inbox,
    ).expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated = make_mutated_metadata_snapshot(&valid_snapshot, "session_id", "different-session");
    let second_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        mutated,
    );
    let result = harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &second_envelope, PayloadKind::Snapshot, &mut inbox,
    );
    assert!(result.is_err(), "mutated session_id must be rejected on existing chunk");
    assert_eq!(harness.storage.snapshot_count().expect("count"), snapshots_after);
    assert_eq!(harness.storage.accepted_envelope_count().expect("count"), envelopes_after);
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
    harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &first_envelope, PayloadKind::Snapshot, &mut inbox,
    ).expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated = make_mutated_metadata_i64_snapshot(&valid_snapshot, "chunk_index", 99);
    let second_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        mutated,
    );
    let result = harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &second_envelope, PayloadKind::Snapshot, &mut inbox,
    );
    assert!(result.is_err(), "mutated chunk_index must be rejected on existing chunk");
    assert_eq!(harness.storage.snapshot_count().expect("count"), snapshots_after);
    assert_eq!(harness.storage.accepted_envelope_count().expect("count"), envelopes_after);
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
    harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &first_envelope, PayloadKind::Snapshot, &mut inbox,
    ).expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated = make_mutated_metadata_i64_snapshot(&valid_snapshot, "created_at_unix_ms", 99_999);
    let second_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        mutated,
    );
    let result = harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &second_envelope, PayloadKind::Snapshot, &mut inbox,
    );
    assert!(result.is_err(), "mutated created_at_unix_ms must be rejected on existing chunk");
    assert_eq!(harness.storage.snapshot_count().expect("count"), snapshots_after);
    assert_eq!(harness.storage.accepted_envelope_count().expect("count"), envelopes_after);
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
    harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &first_envelope, PayloadKind::Snapshot, &mut inbox,
    ).expect("first accept");

    let snapshots_after = baseline_snapshot_count(&harness);
    let envelopes_after = baseline_envelope_count(&harness);

    let mutated = make_mutated_metadata_snapshot(&valid_snapshot, "previous_chunk_id", "bogus-chunk");
    let second_envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        mutated,
    );
    let result = harness.service.receive_delta(
        &peer_hex, &chunk, &chunk.as_path(), &second_envelope, PayloadKind::Snapshot, &mut inbox,
    );
    assert!(result.is_err(), "injected previous_chunk_id must be rejected on existing chunk");
    assert_eq!(harness.storage.snapshot_count().expect("count"), snapshots_after);
    assert_eq!(harness.storage.accepted_envelope_count().expect("count"), envelopes_after);
    harness.worker.shutdown();
}

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
    list.push(serde_json::to_string(&entry).unwrap().as_str()).unwrap();
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
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        initial,
    );
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
    list.push(serde_json::to_string(&entry).unwrap().as_str()).unwrap();
    doc.commit();
    let frontier = doc.oplog_frontiers();
    let shallow = doc.export(ExportMode::shallow_snapshot(&frontier)).unwrap();
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::SnapshotReplace,
        shallow,
    );

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
    let envelope = harness.peer_sealer.seal(
        chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot,
    );

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
