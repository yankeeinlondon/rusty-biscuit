//! In-memory session-log manager.
//!
//! Phase 2 holds one [`LoroDoc`] per active chunk, keyed by the
//! deterministic chunk ID. Every append:
//!
//! 1. Resolves (or rotates to) the active chunk for the session,
//! 2. Stages the entry in a cloned Loro doc,
//! 3. Exports a fresh snapshot from the staged doc,
//! 4. Persists the snapshot to redb (the source of truth),
//! 5. Swaps the staged doc into the live in-memory state,
//! 6. Queues the entry for the DuckDB projection batcher.
//!
//! On startup, the manager rehydrates the in-memory state from redb so
//! the daemon can resume sequence counters and active chunk pointers
//! after a crash.
//!
//! [`SessionLogManager`] is the single public facade. Its behavior is
//! split across sibling modules that each own one responsibility, while
//! the shared session state ([`ChunkState`], [`ManagerInner`],
//! [`SessionCursor`]) and its Loro/redb-facing invariants stay private to
//! this module tree:
//!
//! - [`append`] — local append/rotation and the read surfaces.
//! - [`staging`] — export plus remote-update/replace import staging.
//! - [`rehydrate`] — startup replay/rehydration and projection rebuild.
//! - [`validate`] — remote metadata/schema/append-only validation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use loro::{ExportMode, LoroDoc};
use parking_lot::Mutex;
use rendezvous_core::{
    ChunkConfig, ChunkId, ChunkMetadata, EnvelopeSealer, Entry, NodeIdentity, PayloadKind,
};

use crate::batcher::{BatcherError, BatcherHandle};
use crate::projection::{Projection, ProjectionError};
use crate::storage::{Storage, StorageError};

mod append;
mod rehydrate;
mod staging;
mod validate;

/// Loro container name used to hold the per-chunk metadata map.
const METADATA_CONTAINER: &str = "metadata";

/// Loro container name used to hold the per-chunk append-only entries
/// list. Each list element is a JSON-encoded [`Entry`].
const ENTRIES_CONTAINER: &str = "entries";

/// Errors that the session-log manager can surface.
#[derive(Debug, thiserror::Error)]
pub enum SessionLogError {
    /// The underlying redb storage layer failed.
    #[error(transparent)]
    Storage(#[from] StorageError),

    /// The Loro CRDT engine failed.
    #[error("loro error: {0}")]
    Loro(String),

    /// JSON serialization of an entry or metadata blob failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Submitting a row to the projection batcher failed.
    #[error(transparent)]
    Batcher(#[from] BatcherError),

    /// The DuckDB projection layer failed.
    #[error(transparent)]
    Projection(#[from] ProjectionError),

    /// A persisted entry could not be decoded back into the strongly
    /// typed [`Entry`] schema.
    #[error("decoded entry has unexpected shape: {reason}")]
    DecodeEntry { reason: String },

    /// A remote CRDT payload failed session-log schema validation.
    #[error("remote document schema validation failed: {reason}")]
    SchemaValidation { reason: String },

    /// A remote update imported without error but left operations
    /// pending (their causal dependencies are missing locally). Every
    /// delta in this protocol is exported against our advertised
    /// version, so pending ops mean the sender's history no longer
    /// reaches back to our version — typically because it re-based
    /// (compacted) its document past us. The update must not be
    /// treated as applied; recovery is a snapshot-replace.
    #[error("remote update for {chunk_id} left ops pending ({detail}); snapshot-replace required")]
    PendingRemoteOps { chunk_id: String, detail: String },
}

impl From<loro::LoroError> for SessionLogError {
    fn from(error: loro::LoroError) -> Self {
        Self::Loro(error.to_string())
    }
}

impl From<loro::LoroEncodeError> for SessionLogError {
    fn from(error: loro::LoroEncodeError) -> Self {
        Self::Loro(error.to_string())
    }
}

/// Bytes returned from [`SessionLogManager::export_updates_since`].
#[derive(Clone, Debug)]
pub struct ExportedUpdate {
    /// How the receiver must apply `bytes`: an incremental update
    /// bundle (`Delta`), a full snapshot for a peer with no prior
    /// state (`Snapshot`), or a full snapshot the peer must adopt
    /// wholesale (`SnapshotReplace` — sent when the peer's version
    /// predates this document's shallow root, so the connecting delta
    /// history no longer exists).
    pub kind: PayloadKind,
    /// Encoded Loro payload. Empty when there are no updates beyond
    /// the supplied state vector.
    pub bytes: Vec<u8>,
}

/// Outcome of an [`SessionLogManager::append_entry`] call.
#[derive(Clone, Debug)]
pub struct AppendOutcome {
    /// Chunk the entry was appended to (post-rotation).
    pub chunk: ChunkId,
    /// Sequence number assigned to the entry.
    pub sequence: u64,
    /// Wall-clock at the moment the entry was appended.
    pub created_at_unix_ms: i64,
    /// `true` if appending caused the active chunk to rotate to a new
    /// one.
    pub rotated: bool,
}

/// Opaque staged result of importing a remote update without persisting
/// the resulting snapshot. The sync path uses this to persist the
/// accepted envelope before committing the snapshot. Drop without
/// calling [`SessionLogManager::commit_staged_update`] to discard.
pub(crate) struct StagedRemoteUpdate {
    state: ChunkState,
}

/// In-memory state for one chunk.
struct ChunkState {
    doc: LoroDoc,
    metadata: ChunkMetadata,
    entry_count: u64,
    byte_estimate: u64,
}

impl ChunkState {
    fn new(metadata: ChunkMetadata) -> Result<Self, SessionLogError> {
        let doc = LoroDoc::new();
        // Initialise the metadata map so freshly created chunks are
        // immediately introspectable.
        let map = doc.get_map(METADATA_CONTAINER);
        map.insert("owner_node_id", metadata.owner_node_id.as_str())?;
        map.insert("session_id", metadata.session_id.as_str())?;
        map.insert("chunk_index", metadata.chunk_index as i64)?;
        map.insert("created_at_unix_ms", metadata.created_at_unix_ms)?;
        if let Some(prev) = metadata.previous_chunk_id.as_deref() {
            map.insert("previous_chunk_id", prev)?;
        }
        let _list = doc.get_list(ENTRIES_CONTAINER);
        doc.commit();
        Ok(Self {
            doc,
            metadata,
            entry_count: 0,
            byte_estimate: 0,
        })
    }

    fn from_snapshot(snapshot: &[u8], metadata: ChunkMetadata) -> Result<Self, SessionLogError> {
        let doc = LoroDoc::from_snapshot(snapshot)?;
        let (entry_count, byte_estimate) = doc_entry_stats(&doc);
        Ok(Self {
            doc,
            metadata,
            entry_count,
            byte_estimate,
        })
    }

    fn append(&mut self, entry: &Entry) -> Result<(), SessionLogError> {
        let serialised = serde_json::to_string(entry)?;
        let list = self.doc.get_list(ENTRIES_CONTAINER);
        list.push(serialised.as_str())?;
        self.doc.commit();
        self.entry_count += 1;
        self.byte_estimate += entry.size_estimate();
        Ok(())
    }

    fn snapshot_bytes(&self) -> Result<Vec<u8>, SessionLogError> {
        Ok(self.doc.export(ExportMode::Snapshot)?)
    }

    fn recompute_stats(&mut self) {
        let (count, bytes) = doc_entry_stats(&self.doc);
        self.entry_count = count;
        self.byte_estimate = bytes;
    }

    fn collect_entries(&self) -> Result<Vec<Entry>, SessionLogError> {
        let list = self.doc.get_list(ENTRIES_CONTAINER);
        let mut decode_error: Option<SessionLogError> = None;
        let mut out = Vec::with_capacity(list.len());
        list.for_each(|value| {
            if decode_error.is_some() {
                return;
            }
            match value_to_string(&value) {
                Some(json) => match serde_json::from_str::<Entry>(&json) {
                    Ok(entry) => out.push(entry),
                    Err(err) => decode_error = Some(SessionLogError::Json(err)),
                },
                None => {
                    decode_error = Some(SessionLogError::DecodeEntry {
                        reason: "entries list contained a non-string value".into(),
                    });
                }
            }
        });
        if let Some(err) = decode_error {
            Err(err)
        } else {
            Ok(out)
        }
    }
}

/// Per-session state tracked by the manager: which chunk is currently
/// being appended to, and the next sequence number to hand out.
#[derive(Clone, Debug)]
struct SessionCursor {
    active_chunk_index: u64,
    next_sequence: u64,
}

struct ManagerInner {
    /// Chunk path → in-memory state.
    chunks: HashMap<String, ChunkState>,
    /// Session key (`session/{node}/{session}`) → cursor.
    sessions: HashMap<String, SessionCursor>,
    /// Session key → per-session append serialization lock. Held for the
    /// whole stage→persist→merge window of an append so two appends to
    /// the same session cannot reserve the same sequence or stage from
    /// the same base chunk. Different sessions get different locks, so
    /// they still run concurrently across the redb fsync.
    session_locks: HashMap<String, Arc<Mutex<()>>>,
}

/// Owner of the session-log in-memory state plus the redb storage handle
/// and the batcher submission point.
#[derive(Clone)]
pub struct SessionLogManager {
    inner: Arc<Mutex<ManagerInner>>,
    storage: Storage,
    batcher: BatcherHandle,
    projection: Projection,
    config: ChunkConfig,
    clock: Arc<dyn Clock + Send + Sync>,
    identity: Arc<NodeIdentity>,
    sealer: Arc<Mutex<EnvelopeSealer>>,
}

impl std::fmt::Debug for SessionLogManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionLogManager")
            .field("config", &self.config)
            .field("storage_path", &self.storage.path())
            .finish_non_exhaustive()
    }
}

/// Trait used so tests can pin the wall-clock to deterministic values.
pub trait Clock: Send + Sync {
    fn now_unix_ms(&self) -> i64;
}

/// Default clock that reads `SystemTime::now()`.
#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_ms(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        i64::try_from(now).unwrap_or(i64::MAX)
    }
}

impl SessionLogManager {
    /// Build a new manager wired to the given storage, batcher,
    /// chunk-rotation configuration, and signing identity.
    pub fn new(
        storage: Storage,
        batcher: BatcherHandle,
        projection: Projection,
        config: ChunkConfig,
        identity: Arc<NodeIdentity>,
    ) -> Result<Self, SessionLogError> {
        Self::with_clock(storage, batcher, projection, config, identity, Arc::new(SystemClock))
    }

    /// Build a manager with an injected clock. Tests use this to feed
    /// deterministic `created_at_unix_ms` values into entries.
    pub fn with_clock(
        storage: Storage,
        batcher: BatcherHandle,
        projection: Projection,
        config: ChunkConfig,
        identity: Arc<NodeIdentity>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, SessionLogError> {
        let node_id = identity.node_id();
        let start_counter = storage.load_outbound_counter(&node_id)?;
        let sealer = Arc::new(Mutex::new(EnvelopeSealer::with_start((*identity).clone(), start_counter)));
        let manager = Self {
            inner: Arc::new(Mutex::new(ManagerInner {
                chunks: HashMap::new(),
                sessions: HashMap::new(),
                session_locks: HashMap::new(),
            })),
            storage,
            batcher,
            projection,
            config,
            clock,
            identity,
            sealer,
        };
        manager.rehydrate_from_storage()?;
        manager.rebuild_projection_from_storage()?;
        Ok(manager)
    }

    #[must_use]
    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    /// Stable hex-encoded node identifier of this manager's identity.
    #[must_use]
    pub fn node_id(&self) -> String {
        self.identity.node_id()
    }

    #[must_use]
    pub fn config(&self) -> ChunkConfig {
        self.config
    }

    /// Shared handle on the outbound envelope sealer. The sync service
    /// uses this instead of creating its own so both code paths issue
    /// monotonically increasing message IDs from the same counter.
    #[must_use]
    pub fn sealer(&self) -> Arc<Mutex<EnvelopeSealer>> {
        Arc::clone(&self.sealer)
    }
}

fn value_to_string(value: &loro::ValueOrContainer) -> Option<String> {
    use loro::LoroValue;
    use loro::ValueOrContainer;
    match value {
        ValueOrContainer::Value(LoroValue::String(s)) => Some((**s).to_string()),
        _ => None,
    }
}

fn doc_entry_stats(doc: &loro::LoroDoc) -> (u64, u64) {
    let list = doc.get_list(ENTRIES_CONTAINER);
    let mut count = 0u64;
    let mut bytes = 0u64;
    list.for_each(|value| {
        if let Some(json) = value_to_string(&value)
            && let Ok(entry) = serde_json::from_str::<Entry>(&json)
        {
            count += 1;
            bytes += entry.size_estimate();
        }
    });
    (count, bytes)
}

#[cfg(test)]
mod tests;
