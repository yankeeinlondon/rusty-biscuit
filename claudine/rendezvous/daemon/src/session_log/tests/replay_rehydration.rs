//! replay rehydration session-log tests.

use super::*;

#[test]
fn accepted_envelope_only_replay_recovers_missing_snapshot() {
    let tmp = TempDir::new().expect("tempdir");
    let storage_path = tmp.path().join("session.redb");
    let storage = Storage::open(&storage_path).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([44u8; 32]));

    let projection1 = Projection::in_memory().expect("projection1");
    let worker1 = spawn(
        projection1.clone(),
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );
    let manager1 = SessionLogManager::with_clock(
        storage.clone(),
        worker1.handle(),
        projection1,
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(1)),
    )
    .expect("manager1");

    let sender_identity = NodeIdentity::from_seed([55u8; 32]);
    let sender_hex = sender_identity.node_id();
    let sender_chunk = ChunkId::new(&sender_hex, "remote-session", 0);

    let mut sender_sealer = EnvelopeSealer::with_start(sender_identity, 0);
    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "from-envelope".into(),
        metadata: None,
    };
    let snapshot = make_remote_snapshot(&sender_hex, "remote-session", 0, &[entry]);
    let envelope = sender_sealer.seal(
        sender_chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot.clone(),
    );
    let msg_id_hex = {
        let mut out = String::with_capacity(envelope.message_id.len() * 2);
        for byte in &envelope.message_id {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
        out
    };
    let content_hash_hex = {
        let mut out = String::with_capacity(envelope.content_hash.len() * 2);
        for byte in &envelope.content_hash {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
        out
    };
    let signature_hex = {
        let mut out = String::with_capacity(envelope.signature.len() * 2);
        for byte in &envelope.signature {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
        out
    };

    manager1
        .apply_remote_update(&sender_chunk, &snapshot)
        .expect("apply");

    let accepted = AcceptedEnvelope {
        sender_hex: sender_hex.clone(),
        message_id_hex: msg_id_hex.clone(),
        document_id: sender_chunk.as_path(),
        payload_kind: PayloadKind::Snapshot.to_byte(),
        content_hash_hex,
        signature_hex,
        payload_bytes: snapshot.clone(),
        accepted_at_unix_ms: 5_001,
    };
    storage
        .save_accepted_envelope(&sender_hex, &msg_id_hex, &accepted)
        .expect("save accepted envelope");

    assert!(
        storage
            .load_snapshot(&sender_chunk)
            .expect("load")
            .is_some(),
        "snapshot must exist before removal",
    );
    storage
        .remove_snapshot(&sender_chunk)
        .expect("remove snapshot");
    assert!(
        storage
            .load_snapshot(&sender_chunk)
            .expect("load")
            .is_none(),
        "snapshot must be gone after removal",
    );

    worker1.shutdown();
    drop(manager1);

    let projection2 = Projection::in_memory().expect("projection2");
    let worker2 = spawn(
        projection2.clone(),
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );
    let manager2 = SessionLogManager::with_clock(
        storage.clone(),
        worker2.handle(),
        projection2.clone(),
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(10_000)),
    )
    .expect("manager2");

    let entries = manager2
        .list_chunk_entries(&sender_chunk)
        .expect("list after replay");
    assert_eq!(
        entries.len(),
        1,
        "replay should recover one entry; got {entries:?}"
    );
    assert_eq!(entries[0].message, "from-envelope");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if projection2.row_count().unwrap() >= 1 {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("timed out waiting for projection row from envelope replay");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let rows = projection2
        .entries_for_session(&sender_hex, "remote-session")
        .expect("query");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].message, "from-envelope");

    worker2.shutdown();
    drop(tmp);
}

#[test]
fn envelope_before_snapshot_crash_window_recovers_on_restart() {
    let tmp = TempDir::new().expect("tempdir");
    let storage_path = tmp.path().join("session.redb");
    let storage = Storage::open(&storage_path).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([88u8; 32]));

    let sender_identity = NodeIdentity::from_seed([99u8; 32]);
    let sender_hex = sender_identity.node_id();
    let sender_chunk = ChunkId::new(&sender_hex, "crash-session", 0);

    let mut sender_sealer = EnvelopeSealer::with_start(sender_identity, 0);
    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 7_000,
        source: "remote".into(),
        level: "info".into(),
        message: "crash-window".into(),
        metadata: None,
    };
    let snapshot = make_remote_snapshot(&sender_hex, "crash-session", 0, &[entry]);
    let envelope = sender_sealer.seal(
        sender_chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot.clone(),
    );
    let msg_id_hex = {
        let mut out = String::with_capacity(envelope.message_id.len() * 2);
        for byte in &envelope.message_id {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
        out
    };
    let content_hash_hex = {
        let mut out = String::with_capacity(envelope.content_hash.len() * 2);
        for byte in &envelope.content_hash {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
        out
    };
    let signature_hex = {
        let mut out = String::with_capacity(envelope.signature.len() * 2);
        for byte in &envelope.signature {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
        out
    };

    let accepted = AcceptedEnvelope {
        sender_hex: sender_hex.clone(),
        message_id_hex: msg_id_hex.clone(),
        document_id: sender_chunk.as_path(),
        payload_kind: PayloadKind::Snapshot.to_byte(),
        content_hash_hex,
        signature_hex,
        payload_bytes: snapshot.clone(),
        accepted_at_unix_ms: 7_001,
    };

    storage
        .save_accepted_envelope(&sender_hex, &msg_id_hex, &accepted)
        .expect("save envelope");

    assert!(
        storage
            .load_snapshot(&sender_chunk)
            .expect("load")
            .is_none(),
        "snapshot must not exist — we simulate the crash window where \
         the envelope was persisted but the snapshot write was lost",
    );

    let projection2 = Projection::in_memory().expect("projection2");
    let worker2 = spawn(
        projection2.clone(),
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );
    let manager2 = SessionLogManager::with_clock(
        storage.clone(),
        worker2.handle(),
        projection2.clone(),
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(20_000)),
    )
    .expect("manager2");

    let entries = manager2
        .list_chunk_entries(&sender_chunk)
        .expect("entries after replay");
    assert_eq!(
        entries.len(),
        1,
        "replay should recover one entry from the crash window; got {entries:?}",
    );
    assert_eq!(entries[0].message, "crash-window");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if projection2.row_count().unwrap() >= 1 {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("timed out waiting for projection row from crash-window replay");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let has_dup = storage
        .has_accepted_envelope(&sender_hex, &msg_id_hex)
        .expect("has");
    assert!(has_dup, "accepted envelope must be durable after restart");

    worker2.shutdown();
    drop(tmp);
}

#[test]
fn malformed_loro_payload_rejected_without_envelope_row() {
    let tmp = TempDir::new().expect("tempdir");
    let storage_path = tmp.path().join("session.redb");
    let storage = Storage::open(&storage_path).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([66u8; 32]));

    let projection = Projection::in_memory().expect("projection");
    let worker = spawn(
        projection.clone(),
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );
    let manager = SessionLogManager::with_clock(
        storage.clone(),
        worker.handle(),
        projection,
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(1)),
    )
    .expect("manager");

    let garbage_chunk = ChunkId::new("remote-node", "bad-session", 0);
    let garbage_payload = b"not valid loro data".to_vec();
    let result = manager.apply_remote_update(&garbage_chunk, &garbage_payload);
    assert!(result.is_err(), "malformed Loro payload must be rejected");

    assert_eq!(
        storage.accepted_envelope_count().expect("count"),
        0,
        "no accepted envelope should exist for rejected payload",
    );

    worker.shutdown();
    drop(manager);

    let projection2 = Projection::in_memory().expect("projection2");
    let worker2 = spawn(
        projection2.clone(),
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );
    let manager2 = SessionLogManager::with_clock(
        storage,
        worker2.handle(),
        projection2,
        ChunkConfig::default(),
        identity,
        Arc::new(FixedClock::new(100)),
    )
    .expect("restart must succeed when no malformed envelope was persisted");

    let entries = manager2.list_chunk_entries(&garbage_chunk).expect("list");
    assert!(entries.is_empty(), "no entries from rejected payload");

    worker2.shutdown();
    drop(tmp);
}

#[test]
fn malformed_loro_payload_in_accepted_envelope_skipped_on_restart() {
    let tmp = TempDir::new().expect("tempdir");
    let storage_path = tmp.path().join("session.redb");
    let storage = Storage::open(&storage_path).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([77u8; 32]));

    let sender_identity = NodeIdentity::from_seed([88u8; 32]);
    let sender_hex = sender_identity.node_id();
    let sender_chunk = ChunkId::new(&sender_hex, "poison-session", 0);

    let mut sender_sealer = EnvelopeSealer::with_start(sender_identity, 0);
    let garbage_payload = b"this is not a valid loro snapshot".to_vec();
    let envelope = sender_sealer.seal(
        sender_chunk.as_path(),
        PayloadKind::Snapshot,
        garbage_payload.clone(),
    );

    let accepted = AcceptedEnvelope {
        sender_hex: sender_hex.clone(),
        message_id_hex: envelope.message_id_hex(),
        document_id: sender_chunk.as_path(),
        payload_kind: PayloadKind::Snapshot.to_byte(),
        content_hash_hex: {
            let mut out = String::with_capacity(envelope.content_hash.len() * 2);
            for byte in &envelope.content_hash {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        },
        signature_hex: {
            let mut out = String::with_capacity(envelope.signature.len() * 2);
            for byte in &envelope.signature {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        },
        payload_bytes: garbage_payload,
        accepted_at_unix_ms: 1_000,
    };
    storage
        .save_accepted_envelope(&sender_hex, &envelope.message_id_hex(), &accepted)
        .expect("save");

    let projection = Projection::in_memory().expect("projection");
    let worker = spawn(
        projection,
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );
    let manager = SessionLogManager::with_clock(
        storage,
        worker.handle(),
        Projection::in_memory().expect("projection2"),
        ChunkConfig::default(),
        identity,
        Arc::new(FixedClock::new(1)),
    )
    .expect("restarting with a malformed accepted envelope must succeed (tolerant replay)");

    let entries = manager.list_chunk_entries(&sender_chunk).expect("list");
    assert!(
        entries.is_empty(),
        "malformed payload should be skipped; got {entries:?}"
    );

    worker.shutdown();
    drop(tmp);
}

fn build_accepted_envelope(
    sender_identity: &NodeIdentity,
    chunk: &ChunkId,
    payload: Vec<u8>,
) -> (AcceptedEnvelope, rendezvous_core::SignedEnvelope) {
    let mut sealer = EnvelopeSealer::new(sender_identity.clone());
    let signed = sealer.seal(chunk.as_path(), PayloadKind::Snapshot, payload.clone());
    let accepted = AcceptedEnvelope {
        sender_hex: signed.sender_node_id(),
        message_id_hex: signed.message_id_hex(),
        document_id: chunk.as_path(),
        payload_kind: PayloadKind::Snapshot.to_byte(),
        content_hash_hex: {
            let mut out = String::with_capacity(signed.content_hash.len() * 2);
            for byte in &signed.content_hash {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        },
        signature_hex: {
            let mut out = String::with_capacity(signed.signature.len() * 2);
            for byte in &signed.signature {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        },
        payload_bytes: payload,
        accepted_at_unix_ms: 5_001,
    };
    (accepted, signed)
}

fn start_fresh_manager(
    storage: Storage,
    identity: &Arc<NodeIdentity>,
    clock_start: i64,
) -> (SessionLogManager, BatcherWorker) {
    let projection = Projection::in_memory().expect("projection");
    let worker = spawn(
        projection.clone(),
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );
    let manager = SessionLogManager::with_clock(
        storage,
        worker.handle(),
        projection,
        ChunkConfig::default(),
        Arc::clone(identity),
        Arc::new(FixedClock::new(clock_start)),
    )
    .expect("manager");
    (manager, worker)
}

#[test]
fn replay_skips_accepted_envelope_with_tampered_signature() {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([55u8; 32]));

    let sender_identity = NodeIdentity::from_seed([66u8; 32]);
    let sender_hex = sender_identity.node_id();
    let chunk = ChunkId::new(&sender_hex, "replay-sig-session", 0);

    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "replay-sig".into(),
        metadata: None,
    };
    let payload = make_remote_snapshot(&sender_hex, "replay-sig-session", 0, &[entry]);

    let (mut accepted, _signed) =
        build_accepted_envelope(&sender_identity, &chunk, payload.clone());
    accepted.signature_hex = {
        let bytes: Vec<u8> = (0..64).map(|_| 0xFFu8).collect();
        let mut out = String::with_capacity(128);
        for byte in &bytes {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
        out
    };

    storage
        .save_accepted_envelope(&sender_hex, &accepted.message_id_hex, &accepted)
        .expect("save");

    let (manager, worker) = start_fresh_manager(storage, &identity, 10_000);

    let entries = manager.list_chunk_entries(&chunk).expect("list");
    assert!(
        entries.is_empty(),
        "tampered signature must be skipped during replay; got {entries:?}"
    );

    worker.shutdown();
}

#[test]
fn replay_skips_accepted_envelope_with_tampered_content_hash() {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([56u8; 32]));

    let sender_identity = NodeIdentity::from_seed([67u8; 32]);
    let sender_hex = sender_identity.node_id();
    let chunk = ChunkId::new(&sender_hex, "replay-hash-session", 0);

    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "replay-hash".into(),
        metadata: None,
    };
    let payload = make_remote_snapshot(&sender_hex, "replay-hash-session", 0, &[entry]);

    let (mut accepted, _signed) =
        build_accepted_envelope(&sender_identity, &chunk, payload.clone());
    accepted.content_hash_hex = "ff".repeat(32);

    storage
        .save_accepted_envelope(&sender_hex, &accepted.message_id_hex, &accepted)
        .expect("save");

    let (manager, worker) = start_fresh_manager(storage, &identity, 10_000);

    let entries = manager.list_chunk_entries(&chunk).expect("list");
    assert!(
        entries.is_empty(),
        "tampered content hash must be skipped during replay; got {entries:?}"
    );

    worker.shutdown();
}

#[test]
fn replay_skips_accepted_envelope_with_malformed_sender_hex() {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([57u8; 32]));

    let sender_identity = NodeIdentity::from_seed([68u8; 32]);
    let sender_hex = sender_identity.node_id();
    let chunk = ChunkId::new(&sender_hex, "replay-malformed-session", 0);

    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "replay-malformed".into(),
        metadata: None,
    };
    let payload = make_remote_snapshot(&sender_hex, "replay-malformed-session", 0, &[entry]);

    let (mut accepted, _signed) =
        build_accepted_envelope(&sender_identity, &chunk, payload.clone());
    accepted.sender_hex = "ZZZZ_not_valid_hex".to_string();

    storage
        .save_accepted_envelope(&accepted.sender_hex, &accepted.message_id_hex, &accepted)
        .expect("save");

    let (manager, worker) = start_fresh_manager(storage, &identity, 10_000);

    let entries = manager.list_chunk_entries(&chunk).expect("list");
    assert!(
        entries.is_empty(),
        "malformed sender hex must be skipped during replay; got {entries:?}"
    );

    worker.shutdown();
}

#[test]
fn replay_skips_accepted_envelope_with_wrong_payload_kind() {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([58u8; 32]));

    let sender_identity = NodeIdentity::from_seed([69u8; 32]);
    let sender_hex = sender_identity.node_id();
    let chunk = ChunkId::new(&sender_hex, "replay-kind-session", 0);

    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "replay-kind".into(),
        metadata: None,
    };
    let payload = make_remote_snapshot(&sender_hex, "replay-kind-session", 0, &[entry]);

    let (mut accepted, _signed) =
        build_accepted_envelope(&sender_identity, &chunk, payload.clone());
    accepted.payload_kind = PayloadKind::Delta.to_byte();

    storage
        .save_accepted_envelope(&sender_hex, &accepted.message_id_hex, &accepted)
        .expect("save");

    let (manager, worker) = start_fresh_manager(storage, &identity, 10_000);

    let entries = manager.list_chunk_entries(&chunk).expect("list");
    assert!(
        entries.is_empty(),
        "wrong payload kind must be skipped during replay; got {entries:?}"
    );

    worker.shutdown();
}
