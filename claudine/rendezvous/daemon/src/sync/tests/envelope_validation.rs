//! Signed-envelope contract and its error mapping: a delta is accepted
//! only when sender, document-id, payload-kind, signature, payload hash,
//! and message-id novelty all check out, and any failure must leave
//! neither an accepted-envelope row nor a snapshot behind.

use super::*;

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
