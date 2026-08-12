//! Remote-document invariants: metadata identity, entry schema and
//! sequence monotonicity, and the append-only prefix guard. Every
//! function here is read-only over a staged [`LoroDoc`] — nothing
//! persists — so the staging path can reject a hostile or corrupt peer
//! payload before it touches durable state.

use super::*;

pub(super) fn read_map_string(doc: &LoroDoc, key: &str) -> Result<String, SessionLogError> {
    use loro::ValueOrContainer;
    let map = doc.get_map(METADATA_CONTAINER);
    let Some(voc) = map.get(key) else {
        return Err(SessionLogError::SchemaValidation {
            reason: format!("metadata missing required key: {key}"),
        });
    };
    match voc {
        ValueOrContainer::Value(loro::LoroValue::String(s)) => Ok((*s).to_string()),
        _ => Err(SessionLogError::SchemaValidation {
            reason: format!("metadata key {key} is not a string"),
        }),
    }
}

pub(super) fn read_map_i64(doc: &LoroDoc, key: &str) -> Result<i64, SessionLogError> {
    use loro::ValueOrContainer;
    let map = doc.get_map(METADATA_CONTAINER);
    let Some(voc) = map.get(key) else {
        return Err(SessionLogError::SchemaValidation {
            reason: format!("metadata missing required key: {key}"),
        });
    };
    match voc {
        ValueOrContainer::Value(loro::LoroValue::I64(n)) => Ok(n),
        ValueOrContainer::Value(loro::LoroValue::Double(n)) if n.fract() == 0.0 => Ok(n as i64),
        _ => Err(SessionLogError::SchemaValidation {
            reason: format!("metadata key {key} is not an integer"),
        }),
    }
}

pub(super) fn validate_and_extract_metadata(
    doc: &LoroDoc,
    expected_owner: &str,
    expected_session_id: &str,
    expected_chunk_index: u64,
) -> Result<ChunkMetadata, SessionLogError> {
    let owner = read_map_string(doc, "owner_node_id")?;
    if owner != expected_owner {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata owner_node_id {owner:?} does not match expected {expected_owner:?}"
            ),
        });
    }

    let session_id = read_map_string(doc, "session_id")?;
    if session_id != expected_session_id {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata session_id {session_id:?} does not match expected {expected_session_id:?}"
            ),
        });
    }

    let chunk_index = read_map_i64(doc, "chunk_index")?;
    if chunk_index < 0 || chunk_index as u64 != expected_chunk_index {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata chunk_index {chunk_index} does not match expected {expected_chunk_index}"
            ),
        });
    }

    let created_at = read_map_i64(doc, "created_at_unix_ms")?;
    if created_at <= 0 {
        return Err(SessionLogError::SchemaValidation {
            reason: format!("metadata created_at_unix_ms is invalid: {created_at}"),
        });
    }

    let previous_chunk_id = if expected_chunk_index == 0 {
        let map = doc.get_map(METADATA_CONTAINER);
        if map.get("previous_chunk_id").is_some() {
            return Err(SessionLogError::SchemaValidation {
                reason: "metadata previous_chunk_id must be absent for chunk 0".into(),
            });
        }
        None
    } else {
        let prev = read_map_string(doc, "previous_chunk_id")?;
        let expected_prev = ChunkId::new(
            expected_owner.to_string(),
            expected_session_id.to_string(),
            expected_chunk_index - 1,
        )
        .as_path();
        if prev != expected_prev {
            return Err(SessionLogError::SchemaValidation {
                reason: format!(
                    "metadata previous_chunk_id {prev:?} does not match expected {expected_prev:?}"
                ),
            });
        }
        Some(prev)
    };

    Ok(ChunkMetadata {
        owner_node_id: owner,
        session_id,
        chunk_index: chunk_index as u64,
        created_at_unix_ms: created_at,
        previous_chunk_id,
    })
}

pub(super) fn validate_metadata_unchanged(
    doc: &LoroDoc,
    expected: &ChunkMetadata,
) -> Result<(), SessionLogError> {
    let owner = read_map_string(doc, "owner_node_id")?;
    if owner != expected.owner_node_id {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata owner_node_id {owner:?} mutated from {:?}",
                expected.owner_node_id
            ),
        });
    }

    let session_id = read_map_string(doc, "session_id")?;
    if session_id != expected.session_id {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata session_id {session_id:?} mutated from {:?}",
                expected.session_id
            ),
        });
    }

    let chunk_index = read_map_i64(doc, "chunk_index")?;
    if chunk_index < 0 || chunk_index as u64 != expected.chunk_index {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata chunk_index {chunk_index} mutated from {}",
                expected.chunk_index
            ),
        });
    }

    let created_at = read_map_i64(doc, "created_at_unix_ms")?;
    if created_at != expected.created_at_unix_ms {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata created_at_unix_ms {created_at} mutated from {}",
                expected.created_at_unix_ms
            ),
        });
    }

    match &expected.previous_chunk_id {
        Some(expected_prev) => {
            let prev = read_map_string(doc, "previous_chunk_id")?;
            if prev != *expected_prev {
                return Err(SessionLogError::SchemaValidation {
                    reason: format!(
                        "metadata previous_chunk_id {prev:?} mutated from {expected_prev:?}"
                    ),
                });
            }
        }
        None => {
            let map = doc.get_map(METADATA_CONTAINER);
            if map.get("previous_chunk_id").is_some() {
                return Err(SessionLogError::SchemaValidation {
                    reason: "metadata previous_chunk_id mutated from None".into(),
                });
            }
        }
    }

    Ok(())
}

pub(super) fn validate_remote_entries(
    doc: &LoroDoc,
    existing_entry_count: u64,
) -> Result<(), SessionLogError> {
    let list = doc.get_list(ENTRIES_CONTAINER);
    let len = list.len() as u64;

    if len < existing_entry_count {
        return Err(SessionLogError::SchemaValidation {
            reason: format!("entry count decreased from {existing_entry_count} to {len}"),
        });
    }

    let mut validation_error: Option<SessionLogError> = None;
    let mut last_seq: Option<u64> = None;

    list.for_each(|value| {
        if validation_error.is_some() {
            return;
        }
        match value_to_string(&value) {
            Some(json) => match serde_json::from_str::<Entry>(&json) {
                Ok(entry) => {
                    if let Some(last) = last_seq
                        && entry.sequence <= last
                    {
                        validation_error = Some(SessionLogError::SchemaValidation {
                            reason: format!(
                                "non-monotonic sequence: {} follows {}",
                                entry.sequence, last
                            ),
                        });
                    }
                    last_seq = Some(entry.sequence);
                }
                Err(err) => {
                    validation_error = Some(SessionLogError::SchemaValidation {
                        reason: format!("entry JSON decode failed: {err}"),
                    });
                }
            },
            None => {
                validation_error = Some(SessionLogError::SchemaValidation {
                    reason: "entries list contained a non-string value".into(),
                });
            }
        }
    });

    if let Some(err) = validation_error {
        Err(err)
    } else {
        Ok(())
    }
}

pub(super) fn collect_entry_json_strings(doc: &LoroDoc) -> Result<Vec<String>, SessionLogError> {
    let list = doc.get_list(ENTRIES_CONTAINER);
    let mut out = Vec::with_capacity(list.len());
    let mut err: Option<SessionLogError> = None;
    list.for_each(|value| {
        if err.is_some() {
            return;
        }
        match value_to_string(&value) {
            Some(json) => out.push(json),
            None => {
                err = Some(SessionLogError::DecodeEntry {
                    reason: "entries list contained a non-string value".into(),
                });
            }
        }
    });
    if let Some(e) = err { Err(e) } else { Ok(out) }
}

pub(super) fn validate_append_only_prefix(
    doc: &LoroDoc,
    original_json: &[String],
) -> Result<(), SessionLogError> {
    let staged_json = collect_entry_json_strings(doc)?;
    if staged_json.len() < original_json.len() {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "entry count decreased from {} to {}",
                original_json.len(),
                staged_json.len()
            ),
        });
    }
    for (i, original) in original_json.iter().enumerate() {
        if staged_json[i] != *original {
            return Err(SessionLogError::SchemaValidation {
                reason: format!("entry at index {i} mutated in append-only prefix"),
            });
        }
    }
    Ok(())
}

/// Fail an import whose [`loro::ImportStatus`] parked operations as
/// pending. Loro reports this as a *successful* import, so without the
/// guard a replica silently stops converging (the ops' causal
/// dependencies are missing locally and can never arrive if the sender
/// re-based past us).
pub(super) fn reject_pending_ops(
    chunk: &ChunkId,
    status: &loro::ImportStatus,
) -> Result<(), SessionLogError> {
    match status.pending.as_ref() {
        Some(pending) => Err(SessionLogError::PendingRemoteOps {
            chunk_id: chunk.as_path(),
            detail: format!("{pending:?}"),
        }),
        None => Ok(()),
    }
}

/// Advance the per-session cursor (active chunk index + next sequence)
/// after `chunk`'s in-memory state changed via a remote update or
/// replace. Expects the chunk to be present in `inner.chunks`.
pub(super) fn refresh_session_cursor(inner: &mut ManagerInner, chunk: &ChunkId) {
    let key = chunk.as_path();
    let session_key = chunk.session_key();
    let state = inner.chunks.get(&key).expect("just inserted or merged");
    let mut highest_seq = None;
    state.doc.get_list(ENTRIES_CONTAINER).for_each(|value| {
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
}
