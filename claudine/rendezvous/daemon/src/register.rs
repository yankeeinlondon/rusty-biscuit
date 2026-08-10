//! Kind-2 state-register store.
//!
//! A register is a single-writer Loro document holding the *current
//! value* of something (host capabilities today; `repos/{node}` and
//! `sessions-active/{node}` later) as one flat map of fields. It is the
//! second document kind from the ratified data model
//! (`rendezvous/docs/crdt.md`) — session-log chunks are Kind-1 fact
//! logs owned by [`crate::session_log::SessionLogManager`]; this store
//! owns everything register-shaped.
//!
//! Semantics that differ from chunks:
//!
//! - **Write-on-change:** [`RegisterStore::upsert_local_fields`] writes
//!   only fields whose value actually changed; an identical detection
//!   pass touches nothing (keeping the document's history within its
//!   cadence budget).
//! - **Owner peer binding:** the document's Loro peer id is derived
//!   deterministically from the owner's node id, and every imported
//!   update is rejected unless *all* of its ops carry that peer id.
//!   This is the op-level half of single-writer enforcement (the sync
//!   layer already rejects writes to foreign namespaces).
//! - **Compaction:** registers are overwritten in place forever, so the
//!   store re-bases a register past the ratified thresholds (256 KiB /
//!   10k ops) via a shallow snapshot — the machinery validated by the
//!   register-compaction spike. Readers that fall behind a re-base
//!   recover through the sync layer's snapshot-replace path.

use std::collections::HashMap;
use std::sync::Arc;

use loro::{ExportMode, LoroDoc, LoroValue, VersionVector};
use parking_lot::Mutex;
use rendezvous_core::{DocumentId, NodeIdentity, PayloadKind};
use serde_json::Value as JsonValue;

use crate::session_log::ExportedUpdate;
use crate::storage::{Storage, StorageError};

/// Loro container holding a register's flat field map.
const FIELDS_CONTAINER: &str = "fields";

/// Re-base a register whose persisted snapshot exceeds this size.
const COMPACT_MAX_SNAPSHOT_BYTES: usize = 256 * 1024;

/// Re-base a register whose op count exceeds this.
const COMPACT_MAX_OPS: usize = 10_000;

/// Errors surfaced by the register store.
#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    /// The underlying redb storage layer failed.
    #[error(transparent)]
    Storage(#[from] StorageError),

    /// The Loro CRDT engine failed.
    #[error("loro error: {0}")]
    Loro(String),

    /// A local write targeted a register owned by another node.
    #[error("register {doc} is owned by {owner}, not this node")]
    NotOwner { doc: String, owner: String },

    /// A field value type registers cannot carry (arrays, objects,
    /// null). Register fields are flat scalars by design.
    #[error("register field {field} has unsupported value type")]
    UnsupportedValue { field: String },

    /// An imported update contained ops from a peer id other than the
    /// one derived from the document's owner.
    #[error(
        "register {doc} update carries ops from peer {found_peer} but only {expected_peer} (derived from owner) may write"
    )]
    ForeignWriter {
        doc: String,
        expected_peer: u64,
        found_peer: u64,
    },

    /// An imported update left ops pending (causal gap — the sender
    /// re-based past our version). Recovery is a snapshot-replace.
    #[error("register {doc} update left ops pending ({detail}); snapshot-replace required")]
    PendingOps { doc: String, detail: String },
}

impl From<loro::LoroError> for RegisterError {
    fn from(error: loro::LoroError) -> Self {
        Self::Loro(error.to_string())
    }
}

impl From<loro::LoroEncodeError> for RegisterError {
    fn from(error: loro::LoroEncodeError) -> Self {
        Self::Loro(error.to_string())
    }
}

/// Deterministically derive a register's Loro peer id from its owner's
/// node id (FNV-1a 64). Any stable map works — the value is a *binding*
/// (imported ops must all carry it), not a secret. Both the writing
/// daemon and every validating receiver compute it from the document
/// path alone.
#[must_use]
pub fn owner_peer_id(owner_node_id: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in owner_node_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Opaque staged result of importing a remote register update. Commit
/// with [`RegisterStore::commit_staged`] or drop to discard.
pub struct StagedRegister {
    doc: LoroDoc,
    replace: bool,
}

/// Store for all Kind-2 register documents on this daemon — the local
/// node's own registers (writable) plus synced replicas of every peer's
/// (read-only). Cloning is cheap.
#[derive(Clone)]
pub struct RegisterStore {
    storage: Storage,
    identity: Arc<NodeIdentity>,
    inner: Arc<Mutex<HashMap<String, LoroDoc>>>,
}

impl RegisterStore {
    /// Build the store, rehydrating every persisted register from redb.
    pub fn new(storage: Storage, identity: Arc<NodeIdentity>) -> Result<Self, RegisterError> {
        let mut docs = HashMap::new();
        storage.iter_registers(|path, snapshot| {
            match LoroDoc::from_snapshot(&snapshot) {
                Ok(doc) => {
                    docs.insert(path, doc);
                }
                Err(err) => {
                    tracing::warn!(
                        target: "rendezvous_daemon::register",
                        %path,
                        %err,
                        "skipping corrupt register snapshot during rehydrate; peer re-sync will restore it",
                    );
                }
            }
            Ok(())
        })?;
        Ok(Self {
            storage,
            identity,
            inner: Arc::new(Mutex::new(docs)),
        })
    }

    /// Document id of this host's own capability register.
    #[must_use]
    pub fn local_capability_id(&self) -> DocumentId {
        DocumentId::capability(self.identity.node_id())
    }

    /// Document id of this host's own checked-out-repos register.
    #[must_use]
    pub fn local_repos_id(&self) -> DocumentId {
        DocumentId::repos(self.identity.node_id())
    }

    /// Document id of this host's own active-sessions register.
    #[must_use]
    pub fn local_sessions_active_id(&self) -> DocumentId {
        DocumentId::sessions_active(self.identity.node_id())
    }

    /// Write `fields` into a locally-owned register, touching only the
    /// fields whose value differs from what the register already holds.
    /// Fields absent from `fields` are left alone — right for registers
    /// whose producers may legitimately omit fields (e.g. a failed
    /// detection must not erase a previously known capability).
    /// Returns `true` when anything was written (and persisted).
    pub fn upsert_local_fields(
        &self,
        doc_id: &DocumentId,
        fields: &serde_json::Map<String, JsonValue>,
    ) -> Result<bool, RegisterError> {
        self.write_local_fields(doc_id, fields, false)
    }

    /// Like [`Self::upsert_local_fields`] but `fields` is the complete
    /// current set: keys present in the register but absent from
    /// `fields` are deleted. Right for registers that mirror a
    /// disk/world state (e.g. `repos/{node}` — a deleted checkout must
    /// leave the register).
    pub fn replace_local_fields(
        &self,
        doc_id: &DocumentId,
        fields: &serde_json::Map<String, JsonValue>,
    ) -> Result<bool, RegisterError> {
        self.write_local_fields(doc_id, fields, true)
    }

    /// Delete `keys` from a locally-owned register; keys not present
    /// are ignored. Returns `true` when anything was removed. Built on
    /// the replace path so persistence and compaction behavior are
    /// identical to every other local write.
    pub fn remove_local_fields(
        &self,
        doc_id: &DocumentId,
        keys: &[&str],
    ) -> Result<bool, RegisterError> {
        let Some(JsonValue::Object(current)) = self.deep_value(doc_id)? else {
            return Ok(false);
        };
        if !keys.iter().any(|k| current.contains_key(*k)) {
            return Ok(false);
        }
        let remaining: serde_json::Map<String, JsonValue> = current
            .into_iter()
            .filter(|(k, _)| !keys.contains(&k.as_str()))
            .collect();
        self.replace_local_fields(doc_id, &remaining)
    }

    /// Atomically read-modify-write a locally-owned register under a
    /// single held lock. `mutate` receives the register's current flat
    /// field map (already inflated from Loro) and returns the COMPLETE
    /// desired field set plus an arbitrary result `T`. The store diffs
    /// `desired` against the live document and persists exactly as
    /// [`Self::replace_local_fields`] does (delete-missing, write-changed,
    /// then compaction), all while `inner` is held — so the read the
    /// closure saw and the write it produces are one indivisible
    /// operation. No concurrent transition can interleave between them.
    ///
    /// The `bool` in the result reports whether anything actually changed
    /// (drives the write-on-change budget).
    ///
    /// The closure runs under the store mutex: it MUST be pure/synchronous
    /// and MUST NOT call back into `RegisterStore` (parking_lot is not
    /// reentrant).
    pub fn mutate_local_fields<F, T>(
        &self,
        doc_id: &DocumentId,
        mutate: F,
    ) -> Result<(bool, T), RegisterError>
    where
        F: FnOnce(
            &serde_json::Map<String, JsonValue>,
        ) -> Result<(serde_json::Map<String, JsonValue>, T), RegisterError>,
    {
        let local = self.identity.node_id();
        if doc_id.owner_node_id() != local {
            return Err(RegisterError::NotOwner {
                doc: doc_id.as_path(),
                owner: doc_id.owner_node_id().to_string(),
            });
        }
        let key = doc_id.as_path();
        let peer = owner_peer_id(&local);

        let mut inner = self.inner.lock();
        // Read the current field map through the same inflate path
        // `deep_value` uses, but WITHOUT releasing the lock.
        let current = match inner.get(&key) {
            Some(doc) => {
                let value: LoroValue = doc.get_map(FIELDS_CONTAINER).get_deep_value();
                match serde_json::to_value(&value).unwrap_or(JsonValue::Null) {
                    JsonValue::Object(map) => map,
                    _ => serde_json::Map::new(),
                }
            }
            None => serde_json::Map::new(),
        };
        let (desired, result) = mutate(&current)?;
        let changed = self.write_locked(&mut inner, &key, peer, &desired, true)?;
        Ok((changed, result))
    }

    fn write_local_fields(
        &self,
        doc_id: &DocumentId,
        fields: &serde_json::Map<String, JsonValue>,
        delete_missing: bool,
    ) -> Result<bool, RegisterError> {
        let local = self.identity.node_id();
        if doc_id.owner_node_id() != local {
            return Err(RegisterError::NotOwner {
                doc: doc_id.as_path(),
                owner: doc_id.owner_node_id().to_string(),
            });
        }
        let key = doc_id.as_path();
        let peer = owner_peer_id(&local);

        let mut inner = self.inner.lock();
        self.write_locked(&mut inner, &key, peer, fields, delete_missing)
    }

    /// Diff `fields` against the register at `key` and persist the change,
    /// assuming the `inner` lock is already held by the caller. Shared by
    /// the public write wrappers and [`Self::mutate_local_fields`] so the
    /// persistence + compaction path is identical everywhere.
    fn write_locked(
        &self,
        inner: &mut HashMap<String, LoroDoc>,
        key: &str,
        peer: u64,
        fields: &serde_json::Map<String, JsonValue>,
        delete_missing: bool,
    ) -> Result<bool, RegisterError> {
        if !inner.contains_key(key) {
            let doc = LoroDoc::new();
            doc.set_peer_id(peer)?;
            inner.insert(key.to_string(), doc);
        }
        // Reference clone: shares the underlying doc with the stored
        // handle, so writes below land in the live register.
        let doc = inner.get(key).expect("just ensured").clone();

        let map = doc.get_map(FIELDS_CONTAINER);
        let mut changed = false;
        for (field, value) in fields {
            let Some(candidate) = json_to_loro(value) else {
                return Err(RegisterError::UnsupportedValue {
                    field: field.clone(),
                });
            };
            let current = map.get(field).and_then(|v| match v {
                loro::ValueOrContainer::Value(value) => Some(value),
                loro::ValueOrContainer::Container(_) => None,
            });
            if current.as_ref() != Some(&candidate) {
                map.insert(field, candidate)?;
                changed = true;
            }
        }
        if delete_missing {
            let stale: Vec<String> = match map.get_deep_value() {
                LoroValue::Map(current) => current
                    .keys()
                    .filter(|k| !fields.contains_key(k.as_str()))
                    .map(|k| k.to_string())
                    .collect(),
                _ => Vec::new(),
            };
            for field in stale {
                map.delete(&field)?;
                changed = true;
            }
        }
        if !changed {
            return Ok(false);
        }
        doc.commit();

        let snapshot = doc.export(ExportMode::Snapshot)?;
        if snapshot.len() > COMPACT_MAX_SNAPSHOT_BYTES || doc.len_ops() > COMPACT_MAX_OPS {
            let rebased = self.rebase(&doc, peer)?;
            let rebased_snapshot = rebased.export(ExportMode::Snapshot)?;
            self.storage.save_register(key, &rebased_snapshot)?;
            inner.insert(key.to_string(), rebased);
        } else {
            self.storage.save_register(key, &snapshot)?;
        }
        Ok(true)
    }

    /// Re-base a register past its compaction threshold: adopt a
    /// shallow snapshot at the current frontier as the new working doc.
    /// Peers behind the new root recover via the sync layer's
    /// snapshot-replace path.
    fn rebase(&self, doc: &LoroDoc, peer: u64) -> Result<LoroDoc, RegisterError> {
        let frontier = doc.oplog_frontiers();
        let shallow = doc.export(ExportMode::shallow_snapshot(&frontier))?;
        let rebased = LoroDoc::new();
        rebased.import(&shallow)?;
        rebased.set_peer_id(peer)?;
        tracing::info!(
            target: "rendezvous_daemon::register",
            ops = doc.len_ops(),
            "re-based register past compaction threshold",
        );
        Ok(rebased)
    }

    /// Every register document currently known (local + replicas).
    pub fn list_documents(&self) -> Result<Vec<DocumentId>, RegisterError> {
        let inner = self.inner.lock();
        let mut out: Vec<DocumentId> = inner
            .keys()
            .filter_map(|path| path.parse::<DocumentId>().ok())
            .collect();
        out.sort_by_key(DocumentId::as_path);
        Ok(out)
    }

    /// Encoded Loro version vector for a register, or `None` when we
    /// hold no copy.
    pub fn state_vector(&self, doc_id: &DocumentId) -> Result<Option<Vec<u8>>, RegisterError> {
        let inner = self.inner.lock();
        Ok(inner
            .get(&doc_id.as_path())
            .map(|doc| doc.oplog_vv().encode()))
    }

    /// Export what a peer at `remote_state_vector` is missing, with the
    /// same shallow-root gate as the session-log path: a peer whose
    /// version predates this register's shallow root gets a
    /// `SnapshotReplace` (deltas across a re-base import as
    /// permanently-pending — spike-verified).
    pub fn export_updates_since(
        &self,
        doc_id: &DocumentId,
        remote_state_vector: Option<&[u8]>,
    ) -> Result<Option<ExportedUpdate>, RegisterError> {
        let inner = self.inner.lock();
        let Some(doc) = inner.get(&doc_id.as_path()) else {
            return Ok(None);
        };
        match remote_state_vector {
            Some(raw) => {
                let vv = VersionVector::decode(raw)?;
                if vv == doc.oplog_vv() {
                    // Peer is current. Explicitly short-circuit: a
                    // shallow doc's zero-op updates export is a
                    // non-empty header blob, so byte-emptiness cannot
                    // signal "nothing to send".
                    return Ok(Some(ExportedUpdate {
                        kind: PayloadKind::Delta,
                        bytes: Vec::new(),
                    }));
                }
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
                Ok(Some(ExportedUpdate {
                    kind: PayloadKind::Delta,
                    bytes: doc.export(ExportMode::updates(&vv))?,
                }))
            }
            None => Ok(Some(ExportedUpdate {
                kind: PayloadKind::Snapshot,
                bytes: doc.export(ExportMode::Snapshot)?,
            })),
        }
    }

    /// Stage a remote register update without persisting. Validates the
    /// pending-ops guard and the owner peer binding; `SnapshotReplace`
    /// stages a fresh document (replica swap), other kinds stage a
    /// merge into a copy of the current replica.
    pub fn stage_remote(
        &self,
        doc_id: &DocumentId,
        bytes: &[u8],
        kind: PayloadKind,
    ) -> Result<StagedRegister, RegisterError> {
        let key = doc_id.as_path();
        let replace = kind == PayloadKind::SnapshotReplace;

        let base_snapshot = if replace {
            None
        } else {
            let inner = self.inner.lock();
            match inner.get(&key) {
                Some(current) => Some(current.export(ExportMode::Snapshot)?),
                None => None,
            }
        };
        let staged_doc = match base_snapshot {
            Some(bytes) => LoroDoc::from_snapshot(&bytes)?,
            None => LoroDoc::new(),
        };
        let status = staged_doc.import(bytes)?;
        if let Some(pending) = status.pending.as_ref() {
            return Err(RegisterError::PendingOps {
                doc: key,
                detail: format!("{pending:?}"),
            });
        }

        let expected_peer = owner_peer_id(doc_id.owner_node_id());
        for peer in staged_doc.oplog_vv().keys() {
            if *peer != expected_peer {
                return Err(RegisterError::ForeignWriter {
                    doc: key,
                    expected_peer,
                    found_peer: *peer,
                });
            }
        }

        Ok(StagedRegister {
            doc: staged_doc,
            replace,
        })
    }

    /// Persist and adopt a staged remote update. Returns `true` when
    /// the local replica advanced.
    pub fn commit_staged(
        &self,
        doc_id: &DocumentId,
        staged: StagedRegister,
    ) -> Result<bool, RegisterError> {
        let key = doc_id.as_path();
        let snapshot = staged.doc.export(ExportMode::Snapshot)?;
        self.storage.save_register(&key, &snapshot)?;

        let mut inner = self.inner.lock();
        let advanced = match inner.get(&key) {
            Some(existing) if !staged.replace => {
                // Merge path: replicas of the same single-writer doc
                // never conflict; import is the cheapest union.
                let before = existing.oplog_vv();
                existing.import(&snapshot)?;
                existing.oplog_vv() != before
            }
            Some(existing) => existing.oplog_vv() != staged.doc.oplog_vv(),
            None => true,
        };
        if staged.replace || !inner.contains_key(&key) {
            inner.insert(key, staged.doc);
        }
        Ok(advanced)
    }

    /// Current field values of a register as JSON, or `None` when we
    /// hold no copy. Serves the gRPC read surface.
    pub fn deep_value(&self, doc_id: &DocumentId) -> Result<Option<JsonValue>, RegisterError> {
        let inner = self.inner.lock();
        let Some(doc) = inner.get(&doc_id.as_path()) else {
            return Ok(None);
        };
        let value: LoroValue = doc.get_map(FIELDS_CONTAINER).get_deep_value();
        Ok(Some(
            serde_json::to_value(&value).unwrap_or(JsonValue::Null),
        ))
    }
}

/// Map a JSON scalar onto a Loro value. Arrays, objects, and null are
/// rejected — register fields are flat scalars by design (nested data
/// belongs in its own register or a fact log).
fn json_to_loro(value: &JsonValue) -> Option<LoroValue> {
    match value {
        JsonValue::Bool(b) => Some(LoroValue::from(*b)),
        JsonValue::Number(n) => n
            .as_i64()
            .map(LoroValue::from)
            .or_else(|| n.as_f64().map(LoroValue::from)),
        JsonValue::String(s) => Some(LoroValue::from(s.as_str())),
        JsonValue::Null | JsonValue::Array(_) | JsonValue::Object(_) => None,
    }
}

#[cfg(test)]
mod tests;
