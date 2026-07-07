//! Thread-safe snapshot handoff between the worker pool and the protocol loop.
//!
//! The invalidation engine builds immutable [`WorkspaceGraph`] snapshots
//! (AD-3); [`SharedSnapshot`] is the single cell the worker pool *stores* into
//! and the protocol thread *loads* from, so a request handler always reads a
//! complete, self-consistent snapshot — never a half-mutated graph. Readers
//! hold the whole `Arc`, so a swap mid-request cannot pull data out from under
//! them.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::graph::WorkspaceGraph;

/// An atomically swappable handle to the current graph snapshot.
#[derive(Debug, Clone)]
pub struct SharedSnapshot {
    inner: Arc<RwLock<Arc<WorkspaceGraph>>>,
}

impl SharedSnapshot {
    /// Wraps an initial snapshot.
    pub fn new(graph: Arc<WorkspaceGraph>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(graph)),
        }
    }

    /// Loads the current snapshot (cheap `Arc` clone).
    pub fn load(&self) -> Arc<WorkspaceGraph> {
        Arc::clone(&self.inner.read().expect("snapshot lock poisoned"))
    }

    /// Publishes a new snapshot, replacing the current one.
    pub fn store(&self, graph: Arc<WorkspaceGraph>) {
        *self.inner.write().expect("snapshot lock poisoned") = graph;
    }

    /// The generation of the current snapshot.
    pub fn generation(&self) -> u64 {
        self.load().generation()
    }
}

/// A monotonic generation counter for superseding async work.
///
/// Each indexing/diagnostics job stamps itself with [`Generation::next`]; the
/// protocol loop discards a job whose stamp is older than the latest via
/// [`Generation::is_current`]. This is the cancellation primitive AD-3 chose
/// over per-job cancellation tokens.
#[derive(Debug, Default)]
pub struct Generation {
    current: AtomicU64,
}

impl Generation {
    /// Reserves and returns the next generation stamp.
    pub fn next(&self) -> u64 {
        self.current.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// The latest reserved stamp.
    pub fn current(&self) -> u64 {
        self.current.load(Ordering::SeqCst)
    }

    /// Whether `stamp` is still the latest (no newer job superseded it).
    pub fn is_current(&self, stamp: u64) -> bool {
        self.current() == stamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::WorkspaceIndex;

    #[test]
    fn test_shared_snapshot_store_and_load() {
        let index = WorkspaceIndex::new();
        let shared = SharedSnapshot::new(index.snapshot());
        assert_eq!(shared.generation(), 0);

        let mut index = index;
        index.set_document(std::path::Path::new("/w/a.md"), "# A\n");
        shared.store(index.snapshot());
        assert_eq!(shared.generation(), index.generation());
        assert_eq!(shared.load().document_count(), 1);
    }

    #[test]
    fn test_generation_supersession() {
        let generation = Generation::default();
        let first = generation.next();
        assert!(generation.is_current(first));
        let second = generation.next();
        assert!(!generation.is_current(first));
        assert!(generation.is_current(second));
        assert!(second > first);
    }
}
