//! In-memory session-log manager.
//!
//! Phase 2 holds one [`LoroDoc`] per active chunk, keyed by the
//! deterministic chunk ID. Every append:
//!
//! 1. Resolves (or rotates to) the active chunk for the session,
//! 2. Inserts the entry as a JSON-encoded string in the Loro list,
//! 3. Exports a fresh snapshot,
//! 4. Persists the snapshot to redb (the source of truth),
//! 5. Queues the entry for the DuckDB projection batcher.
//!
//! On startup, the manager rehydrates the in-memory state from redb so
//! the daemon can resume sequence counters and active chunk pointers
//! after a crash.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use loro::{ExportMode, LoroDoc, VersionVector};
use parking_lot::Mutex;
use remote_signal_core::{
    ChunkConfig, ChunkId, ChunkMetadata, Entry, NodeIdentity, SignedEnvelope,
};
use serde_json::Value as JsonValue;

use crate::batcher::{BatcherError, BatcherHandle};
use crate::projection::ProjectionRow;
use crate::storage::{Storage, StorageError};

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

    /// A persisted entry could not be decoded back into the strongly
    /// typed [`Entry`] schema.
    #[error("decoded entry has unexpected shape: {reason}")]
    DecodeEntry { reason: String },
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
    /// `true` when the bytes carry a full snapshot (the peer had no
    /// prior state), `false` for an incremental update bundle.
    pub is_snapshot: bool,
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
        let list = doc.get_list(ENTRIES_CONTAINER);
        let entry_count = list.len() as u64;
        let mut byte_estimate = 0u64;
        list.for_each(|value| {
            if let Some(json) = value_to_string(&value)
                && let Ok(entry) = serde_json::from_str::<Entry>(&json)
            {
                byte_estimate += entry.size_estimate();
            }
        });
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
}

/// Owner of the session-log in-memory state plus the redb storage handle
/// and the batcher submission point.
#[derive(Clone)]
pub struct SessionLogManager {
    inner: Arc<Mutex<ManagerInner>>,
    storage: Storage,
    batcher: BatcherHandle,
    config: ChunkConfig,
    clock: Arc<dyn Clock + Send + Sync>,
    identity: Arc<NodeIdentity>,
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
        config: ChunkConfig,
        identity: Arc<NodeIdentity>,
    ) -> Result<Self, SessionLogError> {
        Self::with_clock(storage, batcher, config, identity, Arc::new(SystemClock))
    }

    /// Build a manager with an injected clock. Tests use this to feed
    /// deterministic `created_at_unix_ms` values into entries.
    pub fn with_clock(
        storage: Storage,
        batcher: BatcherHandle,
        config: ChunkConfig,
        identity: Arc<NodeIdentity>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, SessionLogError> {
        let manager = Self {
            inner: Arc::new(Mutex::new(ManagerInner {
                chunks: HashMap::new(),
                sessions: HashMap::new(),
            })),
            storage,
            batcher,
            config,
            clock,
            identity,
        };
        manager.rehydrate_from_storage()?;
        Ok(manager)
    }

    /// Identity used to sign outgoing Loro deltas.
    #[must_use]
    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    /// Stable hex-encoded node identifier of this manager's identity.
    #[must_use]
    pub fn node_id(&self) -> String {
        self.identity.node_id()
    }

    /// Export the current Loro snapshot for `chunk` and seal it in a
    /// [`SignedEnvelope`] authenticated by this manager's identity.
    ///
    /// Used by the (forthcoming) network layer to push deltas to
    /// paired peers. The returned envelope can be transmitted as-is;
    /// receivers verify it with [`SignedEnvelope::verify`] or feed it
    /// through an [`remote_signal_core::EnvelopeInbox`] for replay
    /// protection.
    pub fn sign_chunk_snapshot(
        &self,
        chunk: &ChunkId,
    ) -> Result<Option<SignedEnvelope>, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();
        let snapshot_bytes = if let Some(state) = inner.chunks.get(&key) {
            state.snapshot_bytes()?
        } else {
            drop(inner);
            let Some(bytes) = self.storage.load_snapshot(chunk)? else {
                return Ok(None);
            };
            bytes
        };
        Ok(Some(SignedEnvelope::seal(&self.identity, snapshot_bytes)))
    }

    /// Active chunk-rotation configuration.
    #[must_use]
    pub fn config(&self) -> ChunkConfig {
        self.config
    }

    /// Append a new entry to the session, rotating chunks if required.
    pub fn append_entry(
        &self,
        owner_node_id: &str,
        session_id: &str,
        source: impl Into<String>,
        level: impl Into<String>,
        message: impl Into<String>,
        metadata: Option<JsonValue>,
    ) -> Result<AppendOutcome, SessionLogError> {
        let session_key = format!("session/{owner_node_id}/{session_id}");
        let now = self.clock.now_unix_ms();

        let mut inner = self.inner.lock();

        let cursor = inner
            .sessions
            .entry(session_key.clone())
            .or_insert(SessionCursor {
                active_chunk_index: 0,
                next_sequence: 0,
            })
            .clone();

        let active_id = ChunkId::new(owner_node_id, session_id, cursor.active_chunk_index);
        let active_path = active_id.as_path();

        // Make sure the active chunk exists in memory.
        if !inner.chunks.contains_key(&active_path) {
            let metadata = ChunkMetadata::initial(owner_node_id, session_id, now);
            inner
                .chunks
                .insert(active_path.clone(), ChunkState::new(metadata)?);
        }

        let needs_rotation = inner
            .chunks
            .get(&active_path)
            .map(|state| {
                self.config
                    .should_rotate(state.entry_count, state.byte_estimate)
            })
            .unwrap_or(false);

        let (chunk_path, rotated) = if needs_rotation {
            let previous_metadata = inner
                .chunks
                .get(&active_path)
                .expect("active chunk exists")
                .metadata
                .clone();
            let next_metadata = ChunkMetadata::rotated_from(&previous_metadata, now);
            let next_id = next_metadata.chunk_id();
            let next_path = next_id.as_path();
            inner
                .chunks
                .insert(next_path.clone(), ChunkState::new(next_metadata)?);
            let session_cursor = inner
                .sessions
                .get_mut(&session_key)
                .expect("session cursor exists");
            session_cursor.active_chunk_index = next_id.chunk_index;
            (next_path, true)
        } else {
            (active_path, false)
        };

        let sequence = inner
            .sessions
            .get(&session_key)
            .expect("session cursor exists")
            .next_sequence;

        let entry = Entry {
            sequence,
            created_at_unix_ms: now,
            source: source.into(),
            level: level.into(),
            message: message.into(),
            metadata,
        };

        let chunk_state = inner
            .chunks
            .get_mut(&chunk_path)
            .expect("chunk state exists after rotation handling");
        chunk_state.append(&entry)?;
        let chunk_id = chunk_state.metadata.chunk_id();
        let snapshot_bytes = chunk_state.snapshot_bytes()?;

        // Persist to redb before acknowledging the write so we honour the
        // "writes are durable in redb before ack" acceptance criterion.
        self.storage.save_snapshot(&chunk_id, &snapshot_bytes)?;

        let session_cursor = inner
            .sessions
            .get_mut(&session_key)
            .expect("session cursor exists");
        session_cursor.next_sequence = sequence + 1;

        drop(inner);

        // Fan out to the projection batcher. A closed batcher is a
        // non-fatal condition for the in-memory model and we
        // intentionally surface it as an error to the caller so tests
        // can observe shutdown ordering issues.
        let metadata_json = entry
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_default();
        self.batcher.submit(ProjectionRow {
            chunk: chunk_id.clone(),
            sequence,
            created_at_unix_ms: entry.created_at_unix_ms,
            source: entry.source.clone(),
            level: entry.level.clone(),
            message: entry.message.clone(),
            metadata_json,
        })?;

        Ok(AppendOutcome {
            chunk: chunk_id,
            sequence,
            created_at_unix_ms: entry.created_at_unix_ms,
            rotated,
        })
    }

    /// List the in-memory entries for a chunk, in insertion order.
    pub fn list_chunk_entries(&self, chunk: &ChunkId) -> Result<Vec<Entry>, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();
        if let Some(state) = inner.chunks.get(&key) {
            return state.collect_entries();
        }
        drop(inner);
        // Fallback: maybe the chunk lives only on disk (e.g. caller is
        // asking about a rotated chunk we haven't hydrated). Load it
        // lazily, but do not cache the result — that path is only used
        // when consumers want a one-shot read.
        let Some(snapshot) = self.storage.load_snapshot(chunk)? else {
            return Ok(Vec::new());
        };
        let metadata =
            ChunkMetadata::initial(chunk.owner_node_id.clone(), chunk.session_id.clone(), 0);
        let state = ChunkState::from_snapshot(&snapshot, ChunkMetadata {
            chunk_index: chunk.chunk_index,
            ..metadata
        })?;
        state.collect_entries()
    }

    /// List all chunk IDs known for the session, in ascending order.
    pub fn list_session_chunks(
        &self,
        owner_node_id: &str,
        session_id: &str,
    ) -> Result<Vec<ChunkId>, SessionLogError> {
        Ok(self.storage.list_chunks(owner_node_id, session_id)?)
    }

    /// List every chunk that this manager knows about (in-memory plus
    /// any chunk persisted to redb), used as the advertise set during
    /// sync. Returns paths in stable lexicographic order.
    pub fn list_all_chunks(&self) -> Result<Vec<ChunkId>, SessionLogError> {
        let mut seen: HashMap<String, ChunkId> = HashMap::new();
        {
            let inner = self.inner.lock();
            for (path, state) in &inner.chunks {
                seen.insert(path.clone(), state.metadata.chunk_id());
            }
        }
        self.storage.iter_snapshots(|chunk, _bytes| {
            seen.entry(chunk.as_path()).or_insert(chunk);
            Ok(())
        })?;
        let mut out: Vec<(String, ChunkId)> = seen.into_iter().collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out.into_iter().map(|(_, c)| c).collect())
    }

    /// Encode the Loro version vector for `chunk`, or `None` if the
    /// daemon has no copy of that chunk locally.
    pub fn chunk_state_vector(&self, chunk: &ChunkId) -> Result<Option<Vec<u8>>, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();
        if let Some(state) = inner.chunks.get(&key) {
            return Ok(Some(state.doc.oplog_vv().encode()));
        }
        drop(inner);
        let Some(snapshot) = self.storage.load_snapshot(chunk)? else {
            return Ok(None);
        };
        let doc = LoroDoc::from_snapshot(&snapshot)?;
        Ok(Some(doc.oplog_vv().encode()))
    }

    /// Export the Loro updates the local chunk has that are missing
    /// from `remote_state_vector`. Returns `None` if the chunk does not
    /// exist locally. When `remote_state_vector` is `None`, exports the
    /// full snapshot (used when the peer has no copy of the chunk).
    pub fn export_updates_since(
        &self,
        chunk: &ChunkId,
        remote_state_vector: Option<&[u8]>,
    ) -> Result<Option<ExportedUpdate>, SessionLogError> {
        let key = chunk.as_path();
        let snapshot_bytes;
        let doc = {
            let inner = self.inner.lock();
            if let Some(state) = inner.chunks.get(&key) {
                let bytes = state.snapshot_bytes()?;
                drop(inner);
                snapshot_bytes = bytes;
                LoroDoc::from_snapshot(&snapshot_bytes)?
            } else {
                drop(inner);
                let Some(bytes) = self.storage.load_snapshot(chunk)? else {
                    return Ok(None);
                };
                snapshot_bytes = bytes;
                LoroDoc::from_snapshot(&snapshot_bytes)?
            }
        };
        match remote_state_vector {
            Some(raw) => {
                let vv = VersionVector::decode(raw)?;
                let updates = doc.export(ExportMode::updates(&vv))?;
                if updates.is_empty() {
                    Ok(Some(ExportedUpdate {
                        is_snapshot: false,
                        bytes: Vec::new(),
                    }))
                } else {
                    Ok(Some(ExportedUpdate {
                        is_snapshot: false,
                        bytes: updates,
                    }))
                }
            }
            None => Ok(Some(ExportedUpdate {
                is_snapshot: true,
                bytes: doc.export(ExportMode::Snapshot)?,
            })),
        }
    }

    /// Apply a delta or snapshot received from a peer. The bytes are
    /// imported into the in-memory chunk state (creating it if
    /// necessary) and the resulting snapshot is persisted to redb so
    /// the on-disk view stays authoritative.
    ///
    /// Returns `true` when the local chunk advanced as a result of the
    /// import (the version vector grew or a new chunk was created).
    pub fn apply_remote_update(
        &self,
        chunk: &ChunkId,
        update_bytes: &[u8],
    ) -> Result<bool, SessionLogError> {
        if update_bytes.is_empty() {
            return Ok(false);
        }
        let key = chunk.as_path();
        let mut inner = self.inner.lock();
        let now = self.clock.now_unix_ms();

        let advanced;
        if let Some(state) = inner.chunks.get_mut(&key) {
            let before = state.doc.oplog_vv();
            state.doc.import(update_bytes)?;
            let after = state.doc.oplog_vv();
            advanced = before != after;
            // Recompute counts from the doc's authoritative state.
            let list = state.doc.get_list(ENTRIES_CONTAINER);
            let mut bytes = 0u64;
            let mut count = 0u64;
            list.for_each(|value| {
                if let Some(json) = value_to_string(&value)
                    && let Ok(entry) = serde_json::from_str::<Entry>(&json)
                {
                    bytes += entry.size_estimate();
                    count += 1;
                }
            });
            state.entry_count = count;
            state.byte_estimate = bytes;
        } else {
            // No local copy yet — create one from the remote snapshot
            // bytes. We treat the bytes as a snapshot regardless of
            // their wire flag because Loro's importer accepts both
            // shapes (snapshots overwrite, updates merge into an empty
            // doc).
            let metadata = ChunkMetadata {
                owner_node_id: chunk.owner_node_id.clone(),
                session_id: chunk.session_id.clone(),
                chunk_index: chunk.chunk_index,
                created_at_unix_ms: now,
                previous_chunk_id: if chunk.chunk_index == 0 {
                    None
                } else {
                    Some(
                        ChunkId::new(
                            chunk.owner_node_id.clone(),
                            chunk.session_id.clone(),
                            chunk.chunk_index - 1,
                        )
                        .as_path(),
                    )
                },
            };
            let doc = LoroDoc::new();
            doc.import(update_bytes)?;
            let list = doc.get_list(ENTRIES_CONTAINER);
            let mut bytes = 0u64;
            let mut count = 0u64;
            list.for_each(|value| {
                if let Some(json) = value_to_string(&value)
                    && let Ok(entry) = serde_json::from_str::<Entry>(&json)
                {
                    bytes += entry.size_estimate();
                    count += 1;
                }
            });
            inner.chunks.insert(
                key.clone(),
                ChunkState {
                    doc,
                    metadata,
                    entry_count: count,
                    byte_estimate: bytes,
                },
            );
            advanced = true;
        }

        // Persist the latest snapshot to redb so restart/replay sees
        // the merged state.
        let state = inner.chunks.get(&key).expect("chunk exists after import");
        let snapshot_bytes = state.snapshot_bytes()?;
        let session_key = chunk.session_key();
        drop(inner);
        self.storage.save_snapshot(chunk, &snapshot_bytes)?;

        // Bump the session cursor so subsequent local appends use the
        // next sequence number after whatever we just merged in.
        let mut inner = self.inner.lock();
        let state = inner.chunks.get(&key).expect("chunk still exists");
        let mut highest_seq = None;
        state
            .doc
            .get_list(ENTRIES_CONTAINER)
            .for_each(|value| {
                if let Some(json) = value_to_string(&value)
                    && let Ok(entry) = serde_json::from_str::<Entry>(&json)
                {
                    highest_seq = Some(highest_seq.map_or(entry.sequence, |h: u64| h.max(entry.sequence)));
                }
            });
        let cursor = inner.sessions.entry(session_key).or_insert(SessionCursor {
            active_chunk_index: chunk.chunk_index,
            next_sequence: 0,
        });
        if chunk.chunk_index > cursor.active_chunk_index {
            cursor.active_chunk_index = chunk.chunk_index;
        }
        if let Some(seq) = highest_seq
            && seq + 1 > cursor.next_sequence
        {
            cursor.next_sequence = seq + 1;
        }
        Ok(advanced)
    }

    /// Rehydrate the in-memory state from every snapshot currently
    /// persisted in redb. Subsequent appends pick up where the previous
    /// process left off.
    fn rehydrate_from_storage(&self) -> Result<(), SessionLogError> {
        let mut to_load: Vec<(ChunkId, Vec<u8>)> = Vec::new();
        self.storage.iter_snapshots(|chunk, bytes| {
            to_load.push((chunk, bytes));
            Ok(())
        })?;

        let mut inner = self.inner.lock();
        for (chunk_id, snapshot) in &to_load {
            let metadata = ChunkMetadata {
                owner_node_id: chunk_id.owner_node_id.clone(),
                session_id: chunk_id.session_id.clone(),
                chunk_index: chunk_id.chunk_index,
                created_at_unix_ms: 0,
                previous_chunk_id: if chunk_id.chunk_index == 0 {
                    None
                } else {
                    Some(
                        ChunkId::new(
                            chunk_id.owner_node_id.clone(),
                            chunk_id.session_id.clone(),
                            chunk_id.chunk_index - 1,
                        )
                        .as_path(),
                    )
                },
            };
            let state = ChunkState::from_snapshot(snapshot, metadata)?;
            let session_key = chunk_id.session_key();
            let cursor = inner
                .sessions
                .entry(session_key)
                .or_insert(SessionCursor {
                    active_chunk_index: chunk_id.chunk_index,
                    next_sequence: 0,
                });
            if chunk_id.chunk_index >= cursor.active_chunk_index {
                cursor.active_chunk_index = chunk_id.chunk_index;
            }
            // The next sequence number is one greater than the highest
            // sequence persisted across all chunks of this session.
            let max_seq = state
                .collect_entries()?
                .into_iter()
                .map(|e| e.sequence)
                .max()
                .map(|m| m + 1)
                .unwrap_or(0);
            if max_seq > cursor.next_sequence {
                cursor.next_sequence = max_seq;
            }
            inner.chunks.insert(chunk_id.as_path(), state);
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batcher::{BatcherConfig, BatcherWorker, spawn};
    use crate::projection::Projection;
    use std::sync::atomic::{AtomicI64, Ordering};
    use tempfile::TempDir;

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
        let worker = spawn(projection, BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager = SessionLogManager::with_clock(
            storage.clone(),
            worker.handle(),
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
        let worker2 = spawn(projection2, BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager2 = SessionLogManager::with_clock(
            storage,
            worker2.handle(),
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
        let worker_a = spawn(projection_a, BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager_a = SessionLogManager::with_clock(
            storage.clone(),
            worker_a.handle(),
            ChunkConfig::default(),
            Arc::new(NodeIdentity::from_seed([11u8; 32])),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager_a");
        let outcome = manager_a
            .append_entry("node-a", "session-1", "src", "info", "msg", None)
            .expect("append");

        let projection_b = Projection::in_memory().expect("projection_b");
        let worker_b = spawn(projection_b, BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager_b = SessionLogManager::with_clock(
            storage,
            worker_b.handle(),
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
}
