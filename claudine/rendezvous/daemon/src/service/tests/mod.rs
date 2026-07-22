//! `RendezvousService` tests, split by the surface each suite exercises:
//!
//! - [`rpc`] — thin request/response RPC behavior (`Ping`, `Status`,
//!   `AppendEntry` → `ListChunkEntries`).
//! - [`session_register`] — the sessions-active register projection:
//!   descriptive merge, daemon-owned clocks, the per-producer status
//!   reducer, and the atomic transition barrier that prevents
//!   resurrection/lost-update races.
//! - [`validation`] — RPC input error mapping.

use super::*;
use crate::batcher::{BatcherConfig, BatcherWorker, spawn};
use crate::projection::Projection;
use crate::session_log::SessionLogManager;
use crate::storage::Storage;
use rendezvous_core::{ChunkConfig, NodeIdentity};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

mod rpc;
mod session_register;
mod validation;

struct Harness {
    service: RendezvousService,
    _worker: BatcherWorker,
    _tmp: TempDir,
}

fn harness() -> Harness {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
    let projection = Projection::in_memory().expect("projection");
    let worker = spawn(projection.clone(), BatcherConfig {
        flush_interval: Duration::from_millis(20),
        flush_size: 16,
    });
    let identity = Arc::new(NodeIdentity::from_seed([5u8; 32]));
    let session_log = SessionLogManager::new(
        storage.clone(),
        worker.handle(),
        projection.clone(),
        ChunkConfig::default(),
        Arc::clone(&identity),
    )
    .expect("mgr");
    let registers = crate::register::RegisterStore::new(
        storage.clone(),
        Arc::clone(&identity),
    )
    .expect("registers");
    let sync_service = SyncService::new(
        session_log.clone(),
        registers.clone(),
        storage.clone(),
        Arc::clone(&identity),
    );
    let service = RendezvousService::new(
        session_log,
        projection,
        identity,
        storage,
        sync_service,
        registers,
    );
    Harness {
        service,
        _worker: worker,
        _tmp: tmp,
    }
}
