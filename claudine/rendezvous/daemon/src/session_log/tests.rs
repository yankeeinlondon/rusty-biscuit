//! Tests for rendezvous session logging.

use super::*;
use crate::batcher::{BatcherConfig, BatcherWorker, spawn};
use crate::projection::Projection;
use crate::storage::AcceptedEnvelope;
use std::sync::atomic::{AtomicI64, Ordering};
use tempfile::TempDir;

fn make_remote_snapshot(
    owner_node_id: &str,
    session_id: &str,
    chunk_index: u64,
    entries: &[Entry],
) -> Vec<u8> {
    let doc = loro::LoroDoc::new();
    let map = doc.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", owner_node_id).unwrap();
    map.insert("session_id", session_id).unwrap();
    map.insert("chunk_index", chunk_index as i64).unwrap();
    map.insert("created_at_unix_ms", 5_000_i64).unwrap();
    if chunk_index > 0 {
        let prev = ChunkId::new(
            owner_node_id.to_string(),
            session_id.to_string(),
            chunk_index - 1,
        );
        map.insert("previous_chunk_id", prev.as_path().as_str()).unwrap();
    }
    let list = doc.get_list(ENTRIES_CONTAINER);
    for entry in entries {
        list.push(serde_json::to_string(entry).unwrap().as_str()).unwrap();
    }
    doc.commit();
    doc.export(ExportMode::Snapshot).unwrap()
}

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
    manager: SessionLogManager,
    storage: Storage,
    worker: BatcherWorker,
    _tmp: TempDir,
}

fn build_harness(config: ChunkConfig) -> Harness {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("open storage");
    let projection = Projection::in_memory().expect("projection");
    let worker = spawn(projection.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager = SessionLogManager::with_clock(
        storage.clone(),
        worker.handle(),
        projection,
        config,
        Arc::new(NodeIdentity::from_seed([7u8; 32])),
        Arc::new(FixedClock::new(1_000)),
    )
    .expect("manager");
    Harness {
        manager,
        storage,
        worker,
        _tmp: tmp,
    }
}

#[test]
fn append_persists_and_increments_sequence() {
    let harness = build_harness(ChunkConfig::default());
    let first = harness
        .manager
        .append_entry("node-a", "session-1", "test", "info", "hello", None)
        .expect("append");
    assert_eq!(first.sequence, 0);
    assert_eq!(first.chunk.chunk_index, 0);
    assert!(!first.rotated);

    let second = harness
        .manager
        .append_entry("node-a", "session-1", "test", "info", "world", None)
        .expect("append");
    assert_eq!(second.sequence, 1);
    assert_eq!(second.chunk.chunk_index, 0);

    let entries = harness
        .manager
        .list_chunk_entries(&first.chunk)
        .expect("list");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].message, "hello");
    assert_eq!(entries[1].message, "world");

    assert_eq!(harness.storage.snapshot_count().unwrap(), 1);
    harness.worker.shutdown();
}

#[test]
fn concurrent_appends_to_one_session_keep_unique_durable_sequences() {
    use std::sync::Barrier;

    let harness = build_harness(ChunkConfig::default());

    // Force genuine contention on the stage→persist→merge window: a
    // barrier releases both appends at once so, without per-session
    // serialization, they would clone the same cursor, reserve the
    // same sequence, and race their whole-snapshot redb writes.
    let append_count = 8usize;
    let barrier = Arc::new(Barrier::new(append_count));
    let handles: Vec<_> = (0..append_count)
        .map(|i| {
            let manager = harness.manager.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                manager
                    .append_entry(
                        "node-a",
                        "session-1",
                        "test",
                        "info",
                        format!("message-{i}"),
                        None,
                    )
                    .expect("concurrent append")
            })
        })
        .collect();

    let mut sequences: Vec<u64> = handles
        .into_iter()
        .map(|h| h.join().expect("append thread").sequence)
        .collect();
    sequences.sort_unstable();
    let expected: Vec<u64> = (0..append_count as u64).collect();
    assert_eq!(
        sequences, expected,
        "appends must hand out unique, gap-free sequences 0..N"
    );

    let chunk = ChunkId::new("node-a", "session-1", 0);

    let in_memory = harness
        .manager
        .list_chunk_entries(&chunk)
        .expect("list in-memory");
    let mut in_memory_messages: Vec<String> =
        in_memory.iter().map(|e| e.message.clone()).collect();
    in_memory_messages.sort();
    let expected_messages: Vec<String> =
        (0..append_count).map(|i| format!("message-{i}")).collect();
    assert_eq!(
        in_memory_messages, expected_messages,
        "every concurrent append must be present in memory"
    );

    harness.worker.shutdown();

    // Reload a fresh manager from the same redb storage: the durable
    // snapshot must already contain every entry. This is the part the
    // sequential tests miss — a lost redb write would only surface
    // after reload.
    let projection2 = Projection::in_memory().expect("projection2");
    let worker2 = spawn(projection2.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager2 = SessionLogManager::with_clock(
        harness.storage.clone(),
        worker2.handle(),
        projection2,
        ChunkConfig::default(),
        Arc::new(NodeIdentity::from_seed([7u8; 32])),
        Arc::new(FixedClock::new(10_000)),
    )
    .expect("reloaded manager");

    let durable = manager2
        .list_chunk_entries(&chunk)
        .expect("list after reload");
    let mut durable_messages: Vec<String> =
        durable.iter().map(|e| e.message.clone()).collect();
    durable_messages.sort();
    assert_eq!(
        durable_messages, expected_messages,
        "every concurrent append must survive in durable redb storage"
    );

    let mut durable_sequences: Vec<u64> = durable.iter().map(|e| e.sequence).collect();
    durable_sequences.sort_unstable();
    assert_eq!(
        durable_sequences, expected,
        "durable entries must keep unique, gap-free sequences after reload"
    );

    worker2.shutdown();
}

#[test]
fn rotation_happens_at_configured_entry_cap() {
    let harness = build_harness(ChunkConfig::new(2, 1024));
    let chunk_zero = harness
        .manager
        .append_entry("node-a", "session-1", "t", "info", "a", None)
        .expect("a")
        .chunk;
    let chunk_zero_again = harness
        .manager
        .append_entry("node-a", "session-1", "t", "info", "b", None)
        .expect("b")
        .chunk;
    assert_eq!(chunk_zero, chunk_zero_again);

    let rotated = harness
        .manager
        .append_entry("node-a", "session-1", "t", "info", "c", None)
        .expect("c");
    assert!(rotated.rotated);
    assert_eq!(rotated.chunk.chunk_index, 1);
    assert_eq!(rotated.sequence, 2);

    let chunks = harness
        .manager
        .list_session_chunks("node-a", "session-1")
        .expect("chunks");
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chunk_index, 0);
    assert_eq!(chunks[1].chunk_index, 1);
    harness.worker.shutdown();
}

#[test]
fn rehydrate_picks_up_existing_snapshots() {
    let tmp = TempDir::new().expect("tempdir");
    let storage_path = tmp.path().join("session.redb");
    let storage = Storage::open(&storage_path).expect("storage");
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
        Arc::new(NodeIdentity::from_seed([3u8; 32])),
        Arc::new(FixedClock::new(1)),
    )
    .expect("manager");
    for _ in 0..3 {
        manager
            .append_entry("node-a", "session-1", "t", "info", "x", None)
            .expect("append");
    }
    worker.shutdown();
    drop(manager);

    let projection2 = Projection::in_memory().expect("projection");
    let worker2 = spawn(projection2.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
    let manager2 = SessionLogManager::with_clock(
        storage,
        worker2.handle(),
        projection2,
        ChunkConfig::default(),
        Arc::new(NodeIdentity::from_seed([3u8; 32])),
        Arc::new(FixedClock::new(100)),
    )
    .expect("manager2");
    let resumed = manager2
        .append_entry("node-a", "session-1", "t", "info", "fourth", None)
        .expect("append");
    assert_eq!(resumed.sequence, 3);
    worker2.shutdown();
    drop(tmp);
}

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
fn accepted_envelope_only_replay_recovers_missing_snapshot() {
    let tmp = TempDir::new().expect("tempdir");
    let storage_path = tmp.path().join("session.redb");
    let storage = Storage::open(&storage_path).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([44u8; 32]));

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

    let sender_identity = NodeIdentity::from_seed([55u8; 32]);
    let sender_hex = sender_identity.node_id();
    let sender_chunk = ChunkId::new(&sender_hex, "remote-session", 0);

    let mut sender_sealer =
        EnvelopeSealer::with_start(sender_identity, 0);
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
        storage.load_snapshot(&sender_chunk).expect("load").is_some(),
        "snapshot must exist before removal",
    );
    storage.remove_snapshot(&sender_chunk).expect("remove snapshot");
    assert!(
        storage.load_snapshot(&sender_chunk).expect("load").is_none(),
        "snapshot must be gone after removal",
    );

    worker1.shutdown();
    drop(manager1);

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
        Arc::new(FixedClock::new(10_000)),
    )
    .expect("manager2");

    let entries = manager2
        .list_chunk_entries(&sender_chunk)
        .expect("list after replay");
    assert_eq!(entries.len(), 1, "replay should recover one entry; got {entries:?}");
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
        storage.load_snapshot(&sender_chunk).expect("load").is_none(),
        "snapshot must not exist — we simulate the crash window where \
         the envelope was persisted but the snapshot write was lost",
    );

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

    let has_dup = storage.has_accepted_envelope(&sender_hex, &msg_id_hex).expect("has");
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
    let worker2 = spawn(projection2.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
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
    let worker = spawn(projection, BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
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

#[test]
fn append_succeeds_after_batcher_shutdown_without_retry_ambiguity() {
    let harness = build_harness(ChunkConfig::default());

    let first = harness
        .manager
        .append_entry("node-a", "session-1", "test", "info", "before-shutdown", None)
        .expect("first append");
    assert_eq!(first.sequence, 0);

    harness.worker.shutdown();

    let second = harness
        .manager
        .append_entry("node-a", "session-1", "test", "info", "after-shutdown", None)
        .expect("append must succeed even with closed batcher");
    assert_eq!(second.sequence, 1);

    assert_eq!(
        harness.storage.snapshot_count().expect("count"),
        1,
        "snapshot must be durable in redb",
    );

    let entries = harness
        .manager
        .list_chunk_entries(&first.chunk)
        .expect("list");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].message, "before-shutdown");
    assert_eq!(entries[1].message, "after-shutdown");
}

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
    let worker = spawn(projection.clone(), BatcherConfig {
        flush_interval: std::time::Duration::from_millis(20),
        flush_size: 16,
    });
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

// --- shallow-root gate + snapshot-replace (register-compaction spike) ---

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
    list.push(serde_json::to_string(&remote_entry(0, "first")).unwrap().as_str())
        .unwrap();
    doc.commit();
    doc
}

fn push_entry(doc: &loro::LoroDoc, sequence: u64, message: &str) {
    let list = doc.get_list(ENTRIES_CONTAINER);
    list.push(serde_json::to_string(&remote_entry(sequence, message)).unwrap().as_str())
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
    harness.storage.save_snapshot(&chunk, &shallow).expect("persist shallow chunk");

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
    assert!(harness.manager.apply_remote_update(&chunk, &initial).expect("initial"));

    push_entry(&doc, 1, "second");
    let frontier = doc.oplog_frontiers();
    let shallow = doc.export(ExportMode::shallow_snapshot(&frontier)).unwrap();

    let advanced = harness.manager.apply_remote_replace(&chunk, &shallow).expect("replace");
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
    let advanced = harness.manager.apply_remote_replace(&chunk, &shallow).expect("idempotent");
    assert!(!advanced);

    harness.worker.shutdown();
}

#[test]
fn apply_remote_replace_rejects_rewritten_prefix() {
    let harness = build_harness(ChunkConfig::default());
    let chunk = ChunkId::new("forge-owner", "forge-session", 0);

    let doc = owner_doc_with_first_entry("forge-owner", "forge-session");
    let initial = doc.export(ExportMode::Snapshot).unwrap();
    assert!(harness.manager.apply_remote_update(&chunk, &initial).expect("initial"));

    // Forged replacement with identical metadata but a rewritten
    // history: replace may extend the entry list, never mutate it.
    let forged = loro::LoroDoc::new();
    let map = forged.get_map(METADATA_CONTAINER);
    map.insert("owner_node_id", "forge-owner").unwrap();
    map.insert("session_id", "forge-session").unwrap();
    map.insert("chunk_index", 0i64).unwrap();
    map.insert("created_at_unix_ms", 5_000_i64).unwrap();
    let list = forged.get_list(ENTRIES_CONTAINER);
    list.push(serde_json::to_string(&remote_entry(0, "REWRITTEN")).unwrap().as_str())
        .unwrap();
    forged.commit();
    let forged_snapshot = forged.export(ExportMode::Snapshot).unwrap();

    let result = harness.manager.apply_remote_replace(&chunk, &forged_snapshot);
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
    assert!(harness.manager.apply_remote_update(&chunk, &initial).expect("initial"));

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
