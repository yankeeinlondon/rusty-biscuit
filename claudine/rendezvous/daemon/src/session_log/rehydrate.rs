//! Startup recovery: rehydrating the in-memory chunk map from redb,
//! replaying accepted envelopes whose snapshots a crash may have lost,
//! and rebuilding the DuckDB projection from durable state. Runs once
//! during [`SessionLogManager::with_clock`], before any live traffic.

use super::validate::validate_and_extract_metadata;
use super::*;
use crate::projection::ProjectionRow;
use rendezvous_core::{
    SignedEnvelope, ENVELOPE_HASH_LENGTH, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH,
};

impl SessionLogManager {
    /// Rebuild the DuckDB projection from every snapshot persisted in
    /// redb. Called once during startup so a restarted daemon with an
    /// empty DuckDB file can serve correct analytical queries.
    ///
    /// The truncate+repopulate is performed in a single DuckDB
    /// transaction so a failure mid-rebuild cannot leave the projection
    /// silently empty.
    pub(super) fn rebuild_projection_from_storage(&self) -> Result<(), SessionLogError> {
        let mut snapshots: Vec<(ChunkId, Vec<u8>)> = Vec::new();
        self.storage.iter_snapshots(|chunk, bytes| {
            snapshots.push((chunk, bytes));
            Ok(())
        })?;
        let mut rows: Vec<ProjectionRow> = Vec::new();
        for (chunk, bytes) in snapshots {
            let doc = LoroDoc::from_snapshot(&bytes)?;
            let list = doc.get_list(ENTRIES_CONTAINER);
            list.for_each(|value| {
                if let Some(json) = value_to_string(&value)
                    && let Ok(entry) = serde_json::from_str::<Entry>(&json)
                {
                    let metadata_json = entry
                        .metadata
                        .as_ref()
                        .map(|v| serde_json::to_string(v).unwrap_or_default())
                        .unwrap_or_default();
                    rows.push(ProjectionRow {
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
        }
        self.projection.replace_all_rows(&rows)?;
        Ok(())
    }

    /// Rehydrate the in-memory state from every snapshot currently
    /// persisted in redb, then replay any accepted envelopes whose
    /// payloads may not yet be reflected in the snapshot (crash
    /// recovery). Subsequent appends pick up where the previous process
    /// left off.
    pub(super) fn rehydrate_from_storage(&self) -> Result<(), SessionLogError> {
        let mut to_load: Vec<(ChunkId, Vec<u8>)> = Vec::new();
        self.storage.iter_snapshots(|chunk, bytes| {
            to_load.push((chunk, bytes));
            Ok(())
        })?;

        let mut inner = self.inner.lock();
        for (chunk_id, snapshot) in &to_load {
            let doc = LoroDoc::from_snapshot(snapshot)?;
            let metadata = validate_and_extract_metadata(
                &doc,
                &chunk_id.owner_node_id,
                &chunk_id.session_id,
                chunk_id.chunk_index,
            )?;
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
        drop(inner);

        self.replay_accepted_envelopes_on_startup()?;

        Ok(())
    }

    /// Re-apply all persisted accepted-envelope payloads into the
    /// in-memory chunks. Loro imports are idempotent (a delta already
    /// present in the version vector is silently ignored), so this is
    /// safe for envelopes that were already applied. Envelopes whose
    /// `save_snapshot` was lost due to a crash will be recovered here.
    fn replay_accepted_envelopes_on_startup(&self) -> Result<(), SessionLogError> {
        let mut to_replay: Vec<crate::storage::AcceptedEnvelope> = Vec::new();
        self.storage.iter_accepted_envelopes(|envelope| {
            to_replay.push(envelope);
            Ok(())
        })?;

        for accepted in &to_replay {
            if accepted.payload_bytes.is_empty() {
                continue;
            }
            let chunk_id = match accepted.document_id.parse::<rendezvous_core::DocumentId>() {
                Ok(rendezvous_core::DocumentId::SessionChunk(chunk)) => chunk,
                // Register envelopes are persisted for replay
                // protection only — registers rehydrate from their own
                // redb table, and a crash-lost register commit heals on
                // the next sync (the peer re-sends against our
                // advertised version). Nothing to replay here.
                Ok(_) => continue,
                Err(err) => {
                    tracing::warn!(
                        target: "rendezvous_daemon::session_log",
                        document_id = %accepted.document_id,
                        %err,
                        "skipping accepted envelope with unrecognized document_id during replay",
                    );
                    continue;
                }
            };

            let Some(signed) = reconstruct_signed_envelope(accepted) else {
                tracing::warn!(
                    target: "rendezvous_daemon::session_log",
                    chunk_id = %chunk_id.as_path(),
                    "skipping accepted envelope with corrupt hex fields during replay",
                );
                continue;
            };

            if let Err(err) = signed.verify() {
                tracing::warn!(
                    target: "rendezvous_daemon::session_log",
                    chunk_id = %chunk_id.as_path(),
                    %err,
                    "skipping accepted envelope that failed signature verification during replay",
                );
                continue;
            }

            let apply_result = match PayloadKind::from_byte(accepted.payload_kind as i32) {
                Some(PayloadKind::SnapshotReplace) => {
                    self.apply_remote_replace(&chunk_id, &accepted.payload_bytes)
                }
                _ => self.apply_remote_update(&chunk_id, &accepted.payload_bytes),
            };
            match apply_result {
                Ok(_) => {}
                Err(SessionLogError::Loro(reason)) => {
                    tracing::warn!(
                        target: "rendezvous_daemon::session_log",
                        chunk_id = %chunk_id.as_path(),
                        %reason,
                        "skipping malformed accepted envelope during replay",
                    );
                }
                Err(SessionLogError::PendingRemoteOps { detail, .. }) => {
                    // Replay iterates envelopes in storage order, which
                    // can deliver an old delta after a replace already
                    // moved the doc's history root. The snapshot in redb
                    // is authoritative; the stale envelope is skipped.
                    tracing::warn!(
                        target: "rendezvous_daemon::session_log",
                        chunk_id = %chunk_id.as_path(),
                        %detail,
                        "skipping accepted envelope with disconnected history during replay",
                    );
                }
                Err(other) => return Err(other),
            }
        }

        Ok(())
    }
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hex[i..i + 2], 16).ok()?);
    }
    Some(bytes)
}

fn reconstruct_signed_envelope(accepted: &crate::storage::AcceptedEnvelope) -> Option<SignedEnvelope> {
    let sender_bytes = hex_decode(&accepted.sender_hex)?;
    let sender: [u8; PUBLIC_KEY_LENGTH] = sender_bytes.try_into().ok()?;
    let message_id_bytes = hex_decode(&accepted.message_id_hex)?;
    let message_id: [u8; ENVELOPE_HASH_LENGTH] = message_id_bytes.try_into().ok()?;
    let content_hash_bytes = hex_decode(&accepted.content_hash_hex)?;
    let content_hash: [u8; ENVELOPE_HASH_LENGTH] = content_hash_bytes.try_into().ok()?;
    let signature_bytes = hex_decode(&accepted.signature_hex)?;
    let signature: [u8; SIGNATURE_LENGTH] = signature_bytes.try_into().ok()?;
    let payload_kind = PayloadKind::from_byte(accepted.payload_kind as i32)?;
    Some(SignedEnvelope {
        sender,
        message_id,
        document_id: accepted.document_id.clone(),
        payload_kind,
        content_hash,
        signature,
        payload: accepted.payload_bytes.clone(),
    })
}
