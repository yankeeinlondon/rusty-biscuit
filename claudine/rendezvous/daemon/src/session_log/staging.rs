//! Export and remote-import staging. Owns the sync-facing surface:
//! signing snapshots, advertising version vectors, exporting the deltas
//! a peer is missing, and the two-phase stage→commit that validates a
//! remote update or snapshot-replace before it touches durable state.

use super::validate::{
    collect_entry_json_strings, refresh_session_cursor, reject_pending_ops,
    validate_and_extract_metadata, validate_append_only_prefix, validate_metadata_unchanged,
    validate_remote_entries,
};
use super::*;
use crate::projection::ProjectionRow;
use loro::VersionVector;
use rendezvous_core::SignedEnvelope;

impl SessionLogManager {
    /// Export the current Loro snapshot for `chunk` and seal it in a
    /// [`SignedEnvelope`] authenticated by this manager's identity.
    ///
    /// Used by the (forthcoming) network layer to push deltas to
    /// paired peers. The returned envelope can be transmitted as-is;
    /// receivers verify it with [`SignedEnvelope::verify`] or feed it
    /// through an [`rendezvous_core::EnvelopeInbox`] for replay
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
        // Capture the counter that follows the sealed envelope inside the
        // same lock scope so a concurrent seal cannot advance the counter
        // past the value we persist.
        let (envelope, counter_to_persist) = {
            let mut sealer = self.sealer.lock();
            let envelope = sealer.seal(chunk.as_path(), PayloadKind::Snapshot, snapshot_bytes);
            let counter = sealer.next_counter();
            (envelope, counter)
        };
        self.storage
            .save_outbound_counter(&self.node_id(), counter_to_persist)?;
        Ok(Some(envelope))
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
                // Shallow-root gate: a re-based (history-compacted) doc
                // no longer holds ops before its shallow root. Exporting
                // updates for an older peer version *succeeds*, but the
                // ops import as permanently-pending on the receiver
                // (verified by the register-compaction spike), so the
                // only safe answer for such a peer is a snapshot it
                // adopts wholesale.
                let shallow_since = doc.shallow_since_vv();
                let peer_covers_shallow_root = shallow_since
                    .iter()
                    .all(|(peer, counter)| vv.get(peer).is_some_and(|c| *c >= *counter));
                if !peer_covers_shallow_root {
                    return Ok(Some(ExportedUpdate {
                        kind: PayloadKind::SnapshotReplace,
                        bytes: doc.export(ExportMode::Snapshot)?,
                    }));
                }
                let updates = doc.export(ExportMode::updates(&vv))?;
                if updates.is_empty() {
                    Ok(Some(ExportedUpdate {
                        kind: PayloadKind::Delta,
                        bytes: Vec::new(),
                    }))
                } else {
                    Ok(Some(ExportedUpdate {
                        kind: PayloadKind::Delta,
                        bytes: updates,
                    }))
                }
            }
            None => Ok(Some(ExportedUpdate {
                kind: PayloadKind::Snapshot,
                bytes: doc.export(ExportMode::Snapshot)?,
            })),
        }
    }

    /// Stage a remote update by importing the payload into a temporary
    /// Loro doc, validating the document schema, but WITHOUT persisting
    /// the resulting snapshot. The caller must follow this with
    /// [`Self::commit_staged_update`] to persist and apply, or drop the
    /// result to discard.
    pub(crate) fn stage_remote_update(
        &self,
        chunk: &ChunkId,
        update_bytes: &[u8],
    ) -> Result<StagedRemoteUpdate, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();

        let staged_state = if let Some(state) = inner.chunks.get(&key) {
            let pre_snapshot = state.snapshot_bytes()?;
            let staged_doc = LoroDoc::from_snapshot(&pre_snapshot)?;
            let original_entry_json = collect_entry_json_strings(&staged_doc)?;
            let status = staged_doc.import(update_bytes)?;
            reject_pending_ops(chunk, &status)?;

            validate_metadata_unchanged(&staged_doc, &state.metadata)?;
            validate_remote_entries(&staged_doc, state.entry_count)?;
            validate_append_only_prefix(&staged_doc, &original_entry_json)?;

            let (count, bytes) = doc_entry_stats(&staged_doc);
            ChunkState {
                doc: staged_doc,
                metadata: state.metadata.clone(),
                entry_count: count,
                byte_estimate: bytes,
            }
        } else {
            let staged_doc = LoroDoc::new();
            let status = staged_doc.import(update_bytes)?;
            reject_pending_ops(chunk, &status)?;

            validate_remote_entries(&staged_doc, 0)?;
            let metadata = validate_and_extract_metadata(
                &staged_doc,
                &chunk.owner_node_id,
                &chunk.session_id,
                chunk.chunk_index,
            )?;

            let (count, bytes) = doc_entry_stats(&staged_doc);
            ChunkState {
                doc: staged_doc,
                metadata,
                entry_count: count,
                byte_estimate: bytes,
            }
        };

        drop(inner);
        Ok(StagedRemoteUpdate {
            state: staged_state,
        })
    }

    /// Commit a previously staged remote update by persisting the
    /// snapshot to redb and merging the staged state into the live
    /// in-memory map. Returns whether the local chunk advanced.
    pub(crate) fn commit_staged_update(
        &self,
        chunk: &ChunkId,
        staged: StagedRemoteUpdate,
    ) -> Result<bool, SessionLogError> {
        let key = chunk.as_path();
        let snapshot_bytes = staged.state.snapshot_bytes()?;

        self.storage.save_snapshot(chunk, &snapshot_bytes)?;

        // Merge rather than wholesale replace so concurrent inbound
        // sync sessions on the same chunk cannot drop each other's
        // entries.
        let mut inner = self.inner.lock();
        let advanced = match inner.chunks.get_mut(&key) {
            Some(existing) => {
                let before_vv = existing.doc.oplog_vv();
                existing.doc.import(&snapshot_bytes)?;
                existing.recompute_stats();
                existing.doc.oplog_vv() != before_vv
            }
            None => {
                inner.chunks.insert(key.clone(), staged.state);
                true
            }
        };

        refresh_session_cursor(&mut inner, chunk);

        Ok(advanced)
    }

    /// Stage a remote snapshot-replace: the payload is a self-contained
    /// snapshot (typically shallow, i.e. history-compacted) that will
    /// REPLACE the local replica wholesale instead of merging into it.
    /// Sent by an owner whose document was re-based past our version.
    /// Validation still enforces chunk identity, entry schema, and the
    /// append-only entry prefix against current local state — a replace
    /// may extend the entry list but never rewrite or shrink it.
    pub(crate) fn stage_remote_replace(
        &self,
        chunk: &ChunkId,
        snapshot_bytes: &[u8],
    ) -> Result<StagedRemoteUpdate, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();

        let staged_doc = LoroDoc::new();
        let status = staged_doc.import(snapshot_bytes)?;
        reject_pending_ops(chunk, &status)?;

        let metadata = if let Some(state) = inner.chunks.get(&key) {
            validate_metadata_unchanged(&staged_doc, &state.metadata)?;
            validate_remote_entries(&staged_doc, state.entry_count)?;
            let original_entry_json = collect_entry_json_strings(&state.doc)?;
            validate_append_only_prefix(&staged_doc, &original_entry_json)?;
            state.metadata.clone()
        } else {
            validate_remote_entries(&staged_doc, 0)?;
            validate_and_extract_metadata(
                &staged_doc,
                &chunk.owner_node_id,
                &chunk.session_id,
                chunk.chunk_index,
            )?
        };

        let (count, bytes) = doc_entry_stats(&staged_doc);
        drop(inner);
        Ok(StagedRemoteUpdate {
            state: ChunkState {
                doc: staged_doc,
                metadata,
                entry_count: count,
                byte_estimate: bytes,
            },
        })
    }

    /// Commit a staged snapshot-replace: persist the snapshot and swap
    /// the in-memory state wholesale. Unlike
    /// [`Self::commit_staged_update`] this deliberately does NOT merge —
    /// the incoming document's history root is newer than the local
    /// replica's, and importing across that gap does not converge
    /// (verified by the register-compaction spike). Nothing local can be
    /// lost: replicas of a foreign-owned chunk are read-only copies of
    /// the owner's document.
    pub(crate) fn commit_staged_replace(
        &self,
        chunk: &ChunkId,
        staged: StagedRemoteUpdate,
    ) -> Result<bool, SessionLogError> {
        let key = chunk.as_path();
        let snapshot_bytes = staged.state.snapshot_bytes()?;

        self.storage.save_snapshot(chunk, &snapshot_bytes)?;

        let mut inner = self.inner.lock();
        let advanced = match inner.chunks.get(&key) {
            Some(existing) => existing.doc.oplog_vv() != staged.state.doc.oplog_vv(),
            None => true,
        };
        inner.chunks.insert(key, staged.state);

        refresh_session_cursor(&mut inner, chunk);

        Ok(advanced)
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
        let staged = self.stage_remote_update(chunk, update_bytes)?;
        self.commit_staged_update(chunk, staged)
    }

    /// Apply a snapshot-replace received from a peer: stage, validate,
    /// and swap the local replica wholesale. See
    /// [`Self::stage_remote_replace`] / [`Self::commit_staged_replace`].
    pub fn apply_remote_replace(
        &self,
        chunk: &ChunkId,
        snapshot_bytes: &[u8],
    ) -> Result<bool, SessionLogError> {
        if snapshot_bytes.is_empty() {
            return Ok(false);
        }
        let staged = self.stage_remote_replace(chunk, snapshot_bytes)?;
        self.commit_staged_replace(chunk, staged)
    }

    /// After a remote update has been applied to a chunk, submit every
    /// entry in that chunk to the projection batcher. Callers should
    /// invoke this only from the live sync path — startup replay relies
    /// on [`Self::rebuild_projection_from_storage`] instead.
    pub fn submit_chunk_to_projection(&self, chunk: &ChunkId) -> Result<(), SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();
        let Some(state) = inner.chunks.get(&key) else {
            return Ok(());
        };
        let list = state.doc.get_list(ENTRIES_CONTAINER);
        list.for_each(|value| {
            if let Some(json) = value_to_string(&value)
                && let Ok(entry) = serde_json::from_str::<Entry>(&json)
            {
                let metadata_json = entry
                    .metadata
                    .as_ref()
                    .map(|v| serde_json::to_string(v).unwrap_or_default())
                    .unwrap_or_default();
                let _ = self.batcher.submit(ProjectionRow {
                    chunk: chunk.clone(),
                    sequence: entry.sequence,
                    created_at_unix_ms: entry.created_at_unix_ms,
                    source: entry.source.clone(),
                    level: entry.level.clone(),
                    message: entry.message.clone(),
                    metadata_json,
                });
            }
        });
        Ok(())
    }
}
