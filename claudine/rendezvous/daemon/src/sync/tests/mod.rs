//! Sync-engine reception tests, split by the invariant each suite
//! guards. All exercise [`SyncService::receive_delta`], the single
//! inbound-delta code path, and assert the stage→persist→commit ordering
//! leaves no partial durable state when a peer payload is rejected.
//!
//! - [`envelope_validation`] — signed-envelope contract and error
//!   mapping (signature, sender, hash, document-id, payload-kind,
//!   duplicate, foreign namespace, persistence failure).
//! - [`schema_validation`] — CRDT document schema and metadata
//!   invariants (entry shape/monotonicity, metadata identity, and
//!   append-only mutation rejection).
//! - [`snapshot_replace`] — the snapshot-replace path for a peer whose
//!   document was re-based past our version.

use super::*;
use crate::batcher::{BatcherConfig, BatcherWorker, spawn};
use crate::projection::Projection;
use crate::session_log::Clock;
use loro::{ExportMode, LoroDoc};
use rendezvous_core::{ChunkConfig, ChunkId, Entry, EnvelopeSealer, NodeIdentity};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tempfile::TempDir;

const ENTRIES_CONTAINER: &str = "entries";
const METADATA_CONTAINER: &str = "metadata";

mod envelope_validation;
mod schema_validation;
mod snapshot_replace;

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
    let worker = spawn(
        projection.clone(),
        BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        },
    );

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

    let registers = RegisterStore::new(storage.clone(), Arc::clone(&identity)).expect("registers");
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
        map.insert("previous_chunk_id", prev_chunk.as_path().as_str())
            .unwrap();
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

fn baseline_snapshot_count(harness: &Harness) -> u64 {
    harness.storage.snapshot_count().expect("snapshot count")
}

fn baseline_envelope_count(harness: &Harness) -> u64 {
    harness
        .storage
        .accepted_envelope_count()
        .expect("envelope count")
}
