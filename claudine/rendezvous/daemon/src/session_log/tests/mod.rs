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
        map.insert("previous_chunk_id", prev.as_path().as_str())
            .unwrap();
    }
    let list = doc.get_list(ENTRIES_CONTAINER);
    for entry in entries {
        list.push(serde_json::to_string(entry).unwrap().as_str())
            .unwrap();
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

mod append_rotation;
mod durability;
mod remote_validation;
mod replace_update;
mod replay_rehydration;
