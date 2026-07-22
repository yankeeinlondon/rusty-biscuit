//! Local append, chunk rotation, and the in-memory/on-disk read
//! surfaces. This is the only path that mints new entries for
//! locally-owned sessions; remote reception lives in [`super::staging`].

use super::validate::validate_and_extract_metadata;
use super::*;
use crate::projection::ProjectionRow;
use serde_json::Value as JsonValue;

impl SessionLogManager {
    /// Per-session append serialization lock, created on first use.
    fn session_lock(&self, session_key: &str) -> Arc<Mutex<()>> {
        let mut inner = self.inner.lock();
        Arc::clone(
            inner
                .session_locks
                .entry(session_key.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
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

        // Serialize same-session appends across the entire
        // stage→persist→merge window. The global `inner` lock alone is
        // not enough: it is deliberately dropped across the redb fsync so
        // unrelated sessions stay concurrent, which leaves a window where
        // two appends to THIS session could clone the same cursor, reserve
        // the same sequence, and stage from the same base chunk — then
        // race their independent whole-snapshot redb writes and durably
        // drop one entry. Holding the per-session lock for the whole call
        // closes that window while preserving cross-session concurrency.
        let append_guard = self.session_lock(&session_key);
        let _append_guard = append_guard.lock();

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

        let needs_rotation = inner
            .chunks
            .get(&active_path)
            .map(|state| {
                self.config
                    .should_rotate(state.entry_count, state.byte_estimate)
            })
            .unwrap_or(false);

        let sequence = cursor.next_sequence;

        let entry = Entry {
            sequence,
            created_at_unix_ms: now,
            source: source.into(),
            level: level.into(),
            message: message.into(),
            metadata,
        };

        // Stage mutation in a separate doc so a persistence failure does
        // not leave unacknowledged data in the live in-memory state.
        let (chunk_id, rotated, staged_state) = if needs_rotation {
            let previous_metadata = inner
                .chunks
                .get(&active_path)
                .expect("active chunk exists")
                .metadata
                .clone();
            let next_metadata = ChunkMetadata::rotated_from(&previous_metadata, now);
            let next_id = next_metadata.chunk_id();
            let mut staged = ChunkState::new(next_metadata)?;
            staged.append(&entry)?;
            (next_id, true, staged)
        } else if let Some(existing) = inner.chunks.get(&active_path) {
            let pre_snapshot = existing.snapshot_bytes()?;
            let mut staged = ChunkState::from_snapshot(&pre_snapshot, existing.metadata.clone())?;
            staged.append(&entry)?;
            (active_id, false, staged)
        } else {
            let metadata = ChunkMetadata::initial(owner_node_id, session_id, now);
            let mut staged = ChunkState::new(metadata)?;
            staged.append(&entry)?;
            (active_id, false, staged)
        };

        let chunk_path = chunk_id.as_path();
        let snapshot_bytes = staged_state.snapshot_bytes()?;

        // Drop the lock before the synchronous redb fsync so unrelated
        // sessions can keep making progress.
        drop(inner);

        // Persist to redb BEFORE mutating the live in-memory state so a
        // storage failure cannot leave unacknowledged entries resident.
        self.storage.save_snapshot(&chunk_id, &snapshot_bytes)?;

        // Re-acquire briefly to merge the staged snapshot into the live
        // state. Importing is idempotent, so concurrent appends that
        // raced ahead are preserved rather than overwritten.
        let mut inner = self.inner.lock();
        match inner.chunks.get_mut(&chunk_path) {
            Some(existing) => {
                existing.doc.import(&snapshot_bytes)?;
                existing.recompute_stats();
            }
            None => {
                inner.chunks.insert(chunk_path.clone(), staged_state);
            }
        }

        if rotated {
            let session_cursor = inner
                .sessions
                .get_mut(&session_key)
                .expect("session cursor exists");
            if session_cursor.active_chunk_index < chunk_id.chunk_index {
                session_cursor.active_chunk_index = chunk_id.chunk_index;
            }
        }

        let session_cursor = inner
            .sessions
            .get_mut(&session_key)
            .expect("session cursor exists");
        if sequence + 1 > session_cursor.next_sequence {
            session_cursor.next_sequence = sequence + 1;
        }

        drop(inner);

        // Fan out to the projection batcher. redb is the source of truth
        // so a closed/failed batcher must not cause the caller to retry
        // and create a duplicate entry.
        let metadata_json = entry
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_default();
        if let Err(err) = self.batcher.submit(ProjectionRow {
            chunk: chunk_id.clone(),
            sequence,
            created_at_unix_ms: entry.created_at_unix_ms,
            source: entry.source.clone(),
            level: entry.level.clone(),
            message: entry.message.clone(),
            metadata_json,
        }) {
            tracing::warn!(
                target: "rendezvous_daemon::session_log",
                %err,
                "projection enqueue failed after redb durability; entry is still authoritative in redb",
            );
        }

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
        // Read the real metadata from the snapshot instead of fabricating
        // a created_at value that would fail the crate's own validator.
        let doc = LoroDoc::from_snapshot(&snapshot)?;
        let metadata = validate_and_extract_metadata(
            &doc,
            &chunk.owner_node_id,
            &chunk.session_id,
            chunk.chunk_index,
        )?;
        let state = ChunkState::from_snapshot(&snapshot, metadata)?;
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
}
