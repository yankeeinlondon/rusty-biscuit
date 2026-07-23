//! durability session-log tests.

use super::*;

#[test]
fn sign_chunk_snapshot_round_trips_through_verify() {
    let harness = build_harness(ChunkConfig::default());
    let outcome = harness
        .manager
        .append_entry("node-a", "session-1", "src", "info", "hello", None)
        .expect("append");
    let envelope = harness
        .manager
        .sign_chunk_snapshot(&outcome.chunk)
        .expect("sign")
        .expect("snapshot exists");
    assert_eq!(envelope.sender, harness.manager.identity().public_key_bytes());
    let payload = envelope.verify().expect("verify");
    assert!(!payload.is_empty());
    harness.worker.shutdown();
}

#[test]
fn sign_chunk_snapshot_for_unknown_chunk_returns_none() {
    let harness = build_harness(ChunkConfig::default());
    let missing = ChunkId::new("node-z", "session-missing", 0);
    let result = harness
        .manager
        .sign_chunk_snapshot(&missing)
        .expect("sign call");
    assert!(result.is_none());
    harness.worker.shutdown();
}

#[test]
fn signed_envelopes_from_different_identities_have_different_senders() {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
    let projection_a = Projection::in_memory().expect("projection_a");
    let worker_a = spawn(projection_a.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager_a = SessionLogManager::with_clock(
        storage.clone(),
        worker_a.handle(),
        projection_a,
        ChunkConfig::default(),
        Arc::new(NodeIdentity::from_seed([11u8; 32])),
        Arc::new(FixedClock::new(1)),
    )
    .expect("manager_a");
    let outcome = manager_a
        .append_entry("node-a", "session-1", "src", "info", "msg", None)
        .expect("append");

    let projection_b = Projection::in_memory().expect("projection_b");
    let worker_b = spawn(projection_b.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager_b = SessionLogManager::with_clock(
        storage,
        worker_b.handle(),
        projection_b,
        ChunkConfig::default(),
        Arc::new(NodeIdentity::from_seed([22u8; 32])),
        Arc::new(FixedClock::new(1)),
    )
    .expect("manager_b");

    let envelope_a = manager_a
        .sign_chunk_snapshot(&outcome.chunk)
        .expect("sign a")
        .expect("snapshot a");
    let envelope_b = manager_b
        .sign_chunk_snapshot(&outcome.chunk)
        .expect("sign b")
        .expect("snapshot b");
    assert_ne!(envelope_a.sender, envelope_b.sender);
    envelope_a.verify().expect("verify a");
    envelope_b.verify().expect("verify b");

    worker_a.shutdown();
    worker_b.shutdown();
    drop(tmp);
}

#[test]
fn rebuild_projection_from_storage_populates_duckdb() {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
    let projection1 = Projection::in_memory().expect("projection1");
    let worker1 = spawn(projection1.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager1 = SessionLogManager::with_clock(
        storage.clone(),
        worker1.handle(),
        projection1,
        ChunkConfig::default(),
        Arc::new(NodeIdentity::from_seed([5u8; 32])),
        Arc::new(FixedClock::new(500)),
    )
    .expect("manager1");
    manager1
        .append_entry("node-a", "session-1", "src", "info", "alpha", None)
        .expect("append 1");
    manager1
        .append_entry("node-a", "session-1", "src", "info", "beta", None)
        .expect("append 2");
    worker1.shutdown();
    drop(manager1);

    let projection2 = Projection::in_memory().expect("projection2");
    let worker2 = spawn(projection2.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let _manager2 = SessionLogManager::with_clock(
        storage,
        worker2.handle(),
        projection2.clone(),
        ChunkConfig::default(),
        Arc::new(NodeIdentity::from_seed([5u8; 32])),
        Arc::new(FixedClock::new(900)),
    )
    .expect("manager2");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if projection2.row_count().unwrap() >= 2 {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("timed out waiting for projection rebuild");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let rows = projection2
        .entries_for_session("node-a", "session-1")
        .expect("query");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].message, "alpha");
    assert_eq!(rows[1].message, "beta");
    worker2.shutdown();
}

#[test]
fn sealer_counter_persists_across_restart() {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([77u8; 32]));
    let node_id = identity.node_id();

    let projection1 = Projection::in_memory().expect("projection1");
    let worker1 = spawn(projection1.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager1 = SessionLogManager::with_clock(
        storage.clone(),
        worker1.handle(),
        projection1,
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(1)),
    )
    .expect("manager1");

    let outcome = manager1
        .append_entry("n", "s", "t", "info", "m", None)
        .expect("append");
    let env1 = manager1
        .sign_chunk_snapshot(&outcome.chunk)
        .expect("sign")
        .expect("envelope");
    let counter1 = u64::from_be_bytes(env1.message_id[24..].try_into().unwrap());
    worker1.shutdown();
    drop(manager1);

    assert_eq!(storage.load_outbound_counter(&node_id).expect("load"), counter1 + 1);

    let projection2 = Projection::in_memory().expect("projection2");
    let worker2 = spawn(projection2.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager2 = SessionLogManager::with_clock(
        storage.clone(),
        worker2.handle(),
        projection2,
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(100)),
    )
    .expect("manager2");

    let env2 = manager2
        .sign_chunk_snapshot(&outcome.chunk)
        .expect("sign")
        .expect("envelope");
    let counter2 = u64::from_be_bytes(env2.message_id[24..].try_into().unwrap());
    assert!(counter2 > counter1, "counter after restart ({counter2}) must exceed pre-restart ({counter1})");
    worker2.shutdown();
}

#[test]
fn failed_persist_does_not_leave_entry_in_memory() {
    let harness = build_harness(ChunkConfig::default());

    let first = harness
        .manager
        .append_entry("node-a", "session-1", "test", "info", "hello", None)
        .expect("first append");
    assert_eq!(first.sequence, 0);

    harness.storage.inject_save_failure();

    let result = harness
        .manager
        .append_entry("node-a", "session-1", "test", "info", "should-fail", None);
    assert!(result.is_err(), "append must fail when persistence fails");

    let entries = harness
        .manager
        .list_chunk_entries(&first.chunk)
        .expect("list");
    assert_eq!(entries.len(), 1, "only the first entry should be visible");
    assert_eq!(entries[0].message, "hello");

    let chunks = harness
        .manager
        .list_session_chunks("node-a", "session-1")
        .expect("chunks");
    assert_eq!(chunks.len(), 1);

    let exported = harness
        .manager
        .export_updates_since(&first.chunk, None)
        .expect("export")
        .expect("snapshot exists");
    let doc = LoroDoc::from_snapshot(&exported.bytes).expect("parse exported snapshot");
    let list = doc.get_list(ENTRIES_CONTAINER);
    assert_eq!(list.len(), 1, "exported snapshot should have one entry");

    let third = harness
        .manager
        .append_entry("node-a", "session-1", "test", "info", "after-failure", None)
        .expect("append after failure");
    assert_eq!(third.sequence, 1, "sequence should resume from 1");

    let entries = harness
        .manager
        .list_chunk_entries(&first.chunk)
        .expect("list after recovery");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].message, "hello");
    assert_eq!(entries[1].message, "after-failure");

    harness.worker.shutdown();
}

#[test]
fn failed_persist_on_remote_update_does_not_leave_data_in_memory() {
    let harness = build_harness(ChunkConfig::default());

    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 5_000,
        source: "remote".into(),
        level: "info".into(),
        message: "from-remote".into(),
        metadata: None,
    };
    let remote_snapshot = make_remote_snapshot("remote-node", "remote-session", 0, &[entry]);

    let remote_chunk = ChunkId::new("remote-node", "remote-session", 0);

    let advanced = harness
        .manager
        .apply_remote_update(&remote_chunk, &remote_snapshot)
        .expect("first remote update");
    assert!(advanced);

    let entries = harness
        .manager
        .list_chunk_entries(&remote_chunk)
        .expect("list after first update");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message, "from-remote");

    let remote_doc2 = LoroDoc::from_snapshot(&remote_snapshot).unwrap();
    let remote_list2 = remote_doc2.get_list(ENTRIES_CONTAINER);
    let entry2 = Entry {
        sequence: 1,
        created_at_unix_ms: 6_000,
        source: "remote".into(),
        level: "info".into(),
        message: "should-not-persist".into(),
        metadata: None,
    };
    remote_list2
        .push(serde_json::to_string(&entry2).unwrap().as_str())
        .unwrap();
    remote_doc2.commit();
    let remote_update = remote_doc2.export(ExportMode::Snapshot).unwrap();

    harness.storage.inject_save_failure();
    let result = harness
        .manager
        .apply_remote_update(&remote_chunk, &remote_update);
    assert!(
        result.is_err(),
        "remote update must fail when persistence fails"
    );

    let entries = harness
        .manager
        .list_chunk_entries(&remote_chunk)
        .expect("list after failed update");
    assert_eq!(entries.len(), 1, "only the first entry should be visible");
    assert_eq!(entries[0].message, "from-remote");

    let exported = harness
        .manager
        .export_updates_since(&remote_chunk, None)
        .expect("export")
        .expect("snapshot exists");
    let doc = LoroDoc::from_snapshot(&exported.bytes).expect("parse");
    let list = doc.get_list(ENTRIES_CONTAINER);
    assert_eq!(list.len(), 1);

    let advanced = harness
        .manager
        .apply_remote_update(&remote_chunk, &remote_update)
        .expect("retry");
    assert!(advanced);

    let entries = harness
        .manager
        .list_chunk_entries(&remote_chunk)
        .expect("list after retry");
    assert_eq!(entries.len(), 2);

    harness.worker.shutdown();
}

#[test]
fn accepted_envelope_failure_prevents_snapshot_persistence() {
    let tmp = TempDir::new().expect("tempdir");
    let storage_path = tmp.path().join("session.redb");
    let storage = Storage::open(&storage_path).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([11u8; 32]));

    let sender_identity = NodeIdentity::from_seed([22u8; 32]);
    let sender_hex = sender_identity.node_id();
    let sender_chunk = ChunkId::new(&sender_hex, "fail-envelope-session", 0);

    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 3_000,
        source: "remote".into(),
        level: "info".into(),
        message: "envelope-fail-entry".into(),
        metadata: None,
    };
    let payload = make_remote_snapshot(&sender_hex, "fail-envelope-session", 0, &[entry]);

    storage.inject_accepted_envelope_failure();
    let accepted = AcceptedEnvelope {
        sender_hex: sender_hex.clone(),
        message_id_hex: "deadbeef".into(),
        document_id: sender_chunk.as_path(),
        payload_kind: PayloadKind::Snapshot.to_byte(),
        content_hash_hex: "cafe".into(),
        signature_hex: String::new(),
        payload_bytes: payload.clone(),
        accepted_at_unix_ms: 3_001,
    };
    let result = storage.save_accepted_envelope(&sender_hex, "deadbeef", &accepted);
    assert!(result.is_err(), "accepted-envelope save must fail when injected");

    assert!(
        storage.load_snapshot(&sender_chunk).expect("load").is_none(),
        "no snapshot should exist when the envelope write was rejected",
    );

    assert_eq!(
        storage.accepted_envelope_count().expect("count"),
        0,
        "no accepted envelope should exist",
    );

    let projection = Projection::in_memory().expect("projection");
    let worker = spawn(projection.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager = SessionLogManager::with_clock(
        storage.clone(),
        worker.handle(),
        projection,
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(1)),
    )
    .expect("manager");

    let entries = manager.list_chunk_entries(&sender_chunk).expect("list");
    assert!(
        entries.is_empty(),
        "no entries should be visible without the accepted envelope; got {entries:?}",
    );

    worker.shutdown();
    drop(tmp);
}

#[test]
fn envelope_before_snapshot_ordering_prevents_duplicate_on_restart() {
    let tmp = TempDir::new().expect("tempdir");
    let storage_path = tmp.path().join("session.redb");
    let storage = Storage::open(&storage_path).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([33u8; 32]));

    let sender_identity = NodeIdentity::from_seed([44u8; 32]);
    let sender_hex = sender_identity.node_id();
    let sender_chunk = ChunkId::new(&sender_hex, "ordering-session", 0);

    let mut sender_sealer = EnvelopeSealer::with_start(sender_identity, 0);
    let entry = Entry {
        sequence: 0,
        created_at_unix_ms: 9_000,
        source: "remote".into(),
        level: "info".into(),
        message: "ordering-entry".into(),
        metadata: None,
    };
    let snapshot = make_remote_snapshot(&sender_hex, "ordering-session", 0, &[entry]);
    let envelope = sender_sealer.seal(
        sender_chunk.as_path(),
        PayloadKind::Snapshot,
        snapshot.clone(),
    );
    let msg_id_hex = envelope.message_id_hex();
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
        accepted_at_unix_ms: 9_001,
    };

    storage
        .save_accepted_envelope(&sender_hex, &msg_id_hex, &accepted)
        .expect("save accepted envelope first");

    let projection = Projection::in_memory().expect("projection");
    let worker = spawn(projection.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager = SessionLogManager::with_clock(
        storage.clone(),
        worker.handle(),
        projection,
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(1)),
    )
    .expect("manager");

    manager
        .apply_remote_update(&sender_chunk, &snapshot)
        .expect("apply after envelope is persisted");

    let entries = manager.list_chunk_entries(&sender_chunk).expect("list");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message, "ordering-entry");

    worker.shutdown();
    drop(manager);

    let projection2 = Projection::in_memory().expect("projection2");
    let worker2 = spawn(projection2.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager2 = SessionLogManager::with_clock(
        storage.clone(),
        worker2.handle(),
        projection2.clone(),
        ChunkConfig::default(),
        Arc::clone(&identity),
        Arc::new(FixedClock::new(50_000)),
    )
    .expect("manager2");

    let entries2 = manager2.list_chunk_entries(&sender_chunk).expect("list after restart");
    assert_eq!(entries2.len(), 1, "entry should survive restart");
    assert_eq!(entries2[0].message, "ordering-entry");

    assert!(
        storage.has_accepted_envelope(&sender_hex, &msg_id_hex).expect("has"),
        "accepted envelope must still be present after restart",
    );

    let projection3 = Projection::in_memory().expect("projection3");
    let worker3 = spawn(projection3.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let mut inbox = rendezvous_core::EnvelopeInbox::new();
    let verified = inbox.accept(&envelope).expect("verify envelope again").to_vec();
    let accepted2 = AcceptedEnvelope {
        sender_hex: sender_hex.clone(),
        message_id_hex: msg_id_hex.clone(),
        document_id: sender_chunk.as_path(),
        payload_kind: PayloadKind::Snapshot.to_byte(),
        content_hash_hex: accepted.content_hash_hex.clone(),
        signature_hex: accepted.signature_hex.clone(),
        payload_bytes: verified,
        accepted_at_unix_ms: 99_000,
    };
    let dup_result = storage.save_accepted_envelope(&sender_hex, &msg_id_hex, &accepted2);
    assert!(
        dup_result.is_ok(),
        "save_accepted_envelope must succeed (idempotent)",
    );
    assert_eq!(
        storage.accepted_envelope_count().expect("count"),
        1,
        "duplicate accepted envelope must not create a second row",
    );

    worker2.shutdown();
    worker3.shutdown();
    drop(tmp);
}


