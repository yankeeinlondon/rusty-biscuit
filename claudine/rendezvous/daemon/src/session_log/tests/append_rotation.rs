//! append rotation session-log tests.

use super::*;

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
    let mut in_memory_messages: Vec<String> = in_memory.iter().map(|e| e.message.clone()).collect();
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
    let worker2 = spawn(
        projection2.clone(),
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );
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
    let mut durable_messages: Vec<String> = durable.iter().map(|e| e.message.clone()).collect();
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
fn append_succeeds_after_batcher_shutdown_without_retry_ambiguity() {
    let harness = build_harness(ChunkConfig::default());

    let first = harness
        .manager
        .append_entry(
            "node-a",
            "session-1",
            "test",
            "info",
            "before-shutdown",
            None,
        )
        .expect("first append");
    assert_eq!(first.sequence, 0);

    harness.worker.shutdown();

    let second = harness
        .manager
        .append_entry(
            "node-a",
            "session-1",
            "test",
            "info",
            "after-shutdown",
            None,
        )
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
