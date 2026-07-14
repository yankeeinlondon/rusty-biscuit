//! Transactional (OLTP) persistence layer backed by redb.
//!
//! Phase 2 stores one Loro snapshot per session-log chunk plus a small
//! catalog mapping each `(owner_node_id, session_id)` pair to the
//! ordered list of chunk indices that exist on disk. The redb file is
//! the source of truth for restart/replay — DuckDB is only a derived
//! projection.
//!
//! The tables intentionally use string keys so the redb dump is
//! human-readable and so the schema can be extended in later phases
//! (signed envelopes, deltas, per-peer state) without breaking
//! backward compatibility.

use std::path::{Path, PathBuf};

use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use rendezvous_core::ChunkId;

/// Snapshot table: chunk-id path → Loro snapshot bytes.
const SNAPSHOTS: TableDefinition<'_, &str, &[u8]> = TableDefinition::new("snapshots");

/// Catalog table: session key (`session/{node}/{session}`) → comma-separated
/// ascending chunk-index list. The format is intentionally trivial to
/// parse; later phases may switch to a structured value type.
const SESSION_CHUNKS: TableDefinition<'_, &str, &str> = TableDefinition::new("session_chunks");

/// Pairings table: hex-encoded `node_id` → JSON-encoded `PairingValue`
/// (paired-at timestamp and an optional note). Stored as JSON so the
/// schema can grow without breaking older daemons.
const PAIRINGS: TableDefinition<'_, &str, &str> = TableDefinition::new("pairings");

/// Accepted-envelopes table: composite key `sender_hex:message_id_hex`
/// → serialised [`AcceptedEnvelope`]. Used for durable replay
/// protection across daemon restarts, scoped per sender so that
/// different senders can independently reuse the same numeric message
/// ID without falsely rejecting each other.
const ACCEPTED_ENVELOPES: TableDefinition<'_, &str, &[u8]> =
    TableDefinition::new("accepted_envelopes");

/// Outbound counter table: hex-encoded `node_id` → last-issued
/// monotonic message-ID counter (`u64`). Allows the daemon to resume
/// the outbound counter after restart instead of resetting to zero.
const OUTBOUND_COUNTER: TableDefinition<'_, &str, u64> =
    TableDefinition::new("outbound_counter");

/// Register table: document path (`{domain}/{owner_node_id}`) → Loro
/// snapshot bytes for Kind-2 state registers. Kept separate from
/// `snapshots` so that table's chunk-only iteration and count semantics
/// stay undisturbed.
const REGISTERS: TableDefinition<'_, &str, &[u8]> = TableDefinition::new("registers");

/// Errors that the storage layer can return.
///
/// The redb error variants carry sizable structured data, so they are
/// boxed to keep the `Err` size of the storage API small (clippy's
/// `result_large_err` lint flags the alternative).
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The redb file could not be opened or created.
    #[error("failed to open redb at {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: Box<redb::DatabaseError>,
    },

    /// A transaction could not be started, committed, or aborted.
    #[error("redb transaction error: {0}")]
    Transaction(Box<redb::TransactionError>),

    /// A table could not be opened inside a transaction.
    #[error("redb table error: {0}")]
    Table(Box<redb::TableError>),

    /// A storage operation (insert, get, etc.) failed.
    #[error("redb storage error: {0}")]
    Storage(Box<redb::StorageError>),

    /// A commit failed after writes were staged.
    #[error("redb commit error: {0}")]
    Commit(Box<redb::CommitError>),

    /// The session-chunks catalog held a value that was not a sequence
    /// of comma-separated unsigned integers.
    #[error("malformed session_chunks value for `{session_key}`: `{raw}`")]
    MalformedCatalog { session_key: String, raw: String },

    /// A pairings row held a JSON value that could not be parsed.
    #[error("malformed pairing record for `{node_id}`: {source}")]
    MalformedPairing {
        node_id: String,
        #[source]
        source: serde_json::Error,
    },

    /// An accepted-envelope row held a JSON value that could not be
    /// parsed.
    #[error("malformed accepted envelope for `{message_id}`: {source}")]
    MalformedAcceptedEnvelope {
        message_id: String,
        #[source]
        source: serde_json::Error,
    },
}

impl From<redb::TransactionError> for StorageError {
    fn from(error: redb::TransactionError) -> Self {
        Self::Transaction(Box::new(error))
    }
}

impl From<redb::TableError> for StorageError {
    fn from(error: redb::TableError) -> Self {
        Self::Table(Box::new(error))
    }
}

impl From<redb::StorageError> for StorageError {
    fn from(error: redb::StorageError) -> Self {
        Self::Storage(Box::new(error))
    }
}

impl From<redb::CommitError> for StorageError {
    fn from(error: redb::CommitError) -> Self {
        Self::Commit(Box::new(error))
    }
}

/// Thin façade over a redb [`Database`] dedicated to session-log
/// persistence. Cloning the handle is cheap (it clones an `Arc`).
#[derive(Clone, Debug)]
pub struct Storage {
    db: std::sync::Arc<Database>,
    path: PathBuf,
    #[cfg(test)]
    fail_next_save: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[cfg(test)]
    fail_next_accepted_envelope: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Storage {
    /// Open (or create) the redb database backing the session log.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|source| StorageError::Open {
                path: path.clone(),
                source: Box::new(redb::DatabaseError::Storage(redb::StorageError::Io(source))),
            })?;
        }
        let db = Database::create(&path).map_err(|source| StorageError::Open {
            path: path.clone(),
            source: Box::new(source),
        })?;

        let storage = Self {
            db: std::sync::Arc::new(db),
            path,
            #[cfg(test)]
            fail_next_save: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            #[cfg(test)]
            fail_next_accepted_envelope: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        storage.bootstrap_tables()?;
        Ok(storage)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Inject a failure into the next `save_snapshot` call. The flag
    /// auto-resets after firing so that subsequent writes succeed.
    #[cfg(test)]
    pub fn inject_save_failure(&self) {
        self.fail_next_save.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Inject a failure into the next `save_accepted_envelope` call. The
    /// flag auto-resets after firing.
    #[cfg(test)]
    pub fn inject_accepted_envelope_failure(&self) {
        self.fail_next_accepted_envelope.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Open the tables once with an empty write transaction so subsequent
    /// reads do not surface "table does not exist" errors before the
    /// first write.
    fn bootstrap_tables(&self) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        {
            let _ = txn.open_table(SNAPSHOTS)?;
            let _ = txn.open_table(SESSION_CHUNKS)?;
            let _ = txn.open_table(PAIRINGS)?;
            let _ = txn.open_table(ACCEPTED_ENVELOPES)?;
            let _ = txn.open_table(OUTBOUND_COUNTER)?;
            let _ = txn.open_table(REGISTERS)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Persist a Loro snapshot for the given chunk and ensure the
    /// session-chunks catalog includes its index. The catalog update
    /// and snapshot write happen inside a single redb transaction so the
    /// on-disk view never disagrees with the in-memory state.
    pub fn save_snapshot(&self, chunk: &ChunkId, snapshot: &[u8]) -> Result<(), StorageError> {
        #[cfg(test)]
        if self.fail_next_save.swap(false, std::sync::atomic::Ordering::SeqCst) {
            return Err(StorageError::Storage(Box::new(redb::StorageError::Io(
                std::io::Error::other("injected save failure"),
            ))));
        }
        let session_key = chunk.session_key();
        let path_key = chunk.as_path();

        let txn = self.db.begin_write()?;
        {
            let mut snaps = txn.open_table(SNAPSHOTS)?;
            snaps.insert(path_key.as_str(), snapshot)?;
            drop(snaps);

            let mut catalog = txn.open_table(SESSION_CHUNKS)?;
            let mut indices = match catalog.get(session_key.as_str())? {
                Some(value) => parse_indices(&session_key, value.value())?,
                None => Vec::new(),
            };
            if !indices.contains(&chunk.chunk_index) {
                indices.push(chunk.chunk_index);
                indices.sort_unstable();
            }
            let encoded = encode_indices(&indices);
            catalog.insert(session_key.as_str(), encoded.as_str())?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Load the latest persisted snapshot for the given chunk, if any.
    pub fn load_snapshot(&self, chunk: &ChunkId) -> Result<Option<Vec<u8>>, StorageError> {
        let path_key = chunk.as_path();
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SNAPSHOTS)?;
        let value = table.get(path_key.as_str())?;
        Ok(value.map(|v| v.value().to_vec()))
    }

    /// Return all chunk IDs persisted for a given session, in ascending
    /// chunk-index order. Returns an empty vector when the session is
    /// unknown.
    pub fn list_chunks(
        &self,
        owner_node_id: &str,
        session_id: &str,
    ) -> Result<Vec<ChunkId>, StorageError> {
        let session_key = format!("session/{owner_node_id}/{session_id}");
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SESSION_CHUNKS)?;
        let Some(value) = table.get(session_key.as_str())? else {
            return Ok(Vec::new());
        };
        let indices = parse_indices(&session_key, value.value())?;
        Ok(indices
            .into_iter()
            .map(|idx| ChunkId::new(owner_node_id, session_id, idx))
            .collect())
    }

    /// Iterate over every persisted `(ChunkId, snapshot)` pair in the
    /// database. Useful for restart/replay.
    pub fn iter_snapshots<F>(&self, mut visit: F) -> Result<(), StorageError>
    where
        F: FnMut(ChunkId, Vec<u8>) -> Result<(), StorageError>,
    {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SNAPSHOTS)?;
        for entry in table.iter()? {
            let (key, value) = entry?;
            let key_str = key.value();
            let Ok(chunk_id) = key_str.parse::<ChunkId>() else {
                tracing::warn!(target: "rendezvous_daemon::storage", key = %key_str, "skipping snapshot with malformed key");
                continue;
            };
            visit(chunk_id, value.value().to_vec())?;
        }
        Ok(())
    }

    /// Number of snapshots currently stored. Convenient for tests.
    pub fn snapshot_count(&self) -> Result<u64, StorageError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SNAPSHOTS)?;
        Ok(table.len()?)
    }

    /// Remove a single snapshot from the snapshots table. Returns `true`
    /// when a row was actually deleted. Used in tests to simulate a crash
    /// that persists an accepted envelope but loses the corresponding
    /// snapshot.
    pub fn remove_snapshot(&self, chunk: &ChunkId) -> Result<bool, StorageError> {
        let path_key = chunk.as_path();
        let txn = self.db.begin_write()?;
        let removed;
        {
            let mut table = txn.open_table(SNAPSHOTS)?;
            removed = table.remove(path_key.as_str())?.is_some();
        }
        txn.commit()?;
        Ok(removed)
    }

    /// Persist a Kind-2 register snapshot under its document path.
    pub fn save_register(&self, doc_path: &str, snapshot: &[u8]) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(REGISTERS)?;
            table.insert(doc_path, snapshot)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Load the latest persisted register snapshot, if any.
    pub fn load_register(&self, doc_path: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(REGISTERS)?;
        let value = table.get(doc_path)?;
        Ok(value.map(|v| v.value().to_vec()))
    }

    /// Iterate every persisted `(document path, snapshot)` register pair.
    pub fn iter_registers<F>(&self, mut visit: F) -> Result<(), StorageError>
    where
        F: FnMut(String, Vec<u8>) -> Result<(), StorageError>,
    {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(REGISTERS)?;
        for entry in table.iter()? {
            let (key, value) = entry?;
            visit(key.value().to_string(), value.value().to_vec())?;
        }
        Ok(())
    }

    /// Number of registers currently stored. Convenient for tests.
    pub fn register_count(&self) -> Result<u64, StorageError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(REGISTERS)?;
        Ok(table.len()?)
    }

    /// Upsert a pairing entry. The stored value is a JSON blob with the
    /// `paired_at_unix_ms` and `note` fields so the schema can grow
    /// without breaking the table layout.
    pub fn upsert_pairing(
        &self,
        node_id: &str,
        paired_at_unix_ms: i64,
        note: &str,
    ) -> Result<(), StorageError> {
        let value = PairingValue {
            paired_at_unix_ms,
            note: note.to_string(),
        };
        let encoded = serde_json::to_string(&value).map_err(|source| StorageError::MalformedPairing {
            node_id: node_id.to_string(),
            source,
        })?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(PAIRINGS)?;
            table.insert(node_id, encoded.as_str())?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Delete a pairing entry. Returns `true` if a row was removed.
    pub fn remove_pairing(&self, node_id: &str) -> Result<bool, StorageError> {
        let txn = self.db.begin_write()?;
        let removed;
        {
            let mut table = txn.open_table(PAIRINGS)?;
            removed = table.remove(node_id)?.is_some();
        }
        txn.commit()?;
        Ok(removed)
    }

    /// Fetch a single pairing entry, if present.
    pub fn get_pairing(&self, node_id: &str) -> Result<Option<PairingValue>, StorageError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(PAIRINGS)?;
        let Some(value) = table.get(node_id)? else {
            return Ok(None);
        };
        let parsed: PairingValue =
            serde_json::from_str(value.value()).map_err(|source| StorageError::MalformedPairing {
                node_id: node_id.to_string(),
                source,
            })?;
        Ok(Some(parsed))
    }

    /// List all pairings in stable lexicographic order.
    pub fn list_pairings(&self) -> Result<Vec<(String, PairingValue)>, StorageError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(PAIRINGS)?;
        let mut out = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            let node_id = key.value().to_string();
            let parsed: PairingValue = serde_json::from_str(value.value()).map_err(|source| {
                StorageError::MalformedPairing {
                    node_id: node_id.clone(),
                    source,
                }
            })?;
            out.push((node_id, parsed));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// Persist an accepted envelope. The composite key
    /// `sender_hex:message_id_hex` ensures duplicate detection is
    /// scoped per sender so different senders can independently reuse
    /// the same numeric message ID. Returns `true` when the envelope
    /// was newly inserted, `false` when it was already present (i.e.
    /// a duplicate from a previous session).
    pub fn save_accepted_envelope(
        &self,
        sender_hex: &str,
        message_id_hex: &str,
        envelope: &AcceptedEnvelope,
    ) -> Result<bool, StorageError> {
        #[cfg(test)]
        if self.fail_next_accepted_envelope.swap(false, std::sync::atomic::Ordering::SeqCst) {
            return Err(StorageError::Storage(Box::new(redb::StorageError::Io(
                std::io::Error::other("injected accepted-envelope failure"),
            ))));
        }
        let composite_key = format!("{sender_hex}:{message_id_hex}");
        let encoded = serde_json::to_vec(envelope).map_err(|source| {
            StorageError::MalformedAcceptedEnvelope {
                message_id: composite_key.clone(),
                source,
            }
        })?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(ACCEPTED_ENVELOPES)?;
            let exists = table.get(composite_key.as_str())?.is_some();
            if exists {
                drop(table);
                txn.commit()?;
                return Ok(false);
            }
            table.insert(composite_key.as_str(), encoded.as_slice())?;
        }
        txn.commit()?;
        Ok(true)
    }

    /// Check whether an envelope with the given
    /// `(sender_hex, message_id_hex)` pair was already accepted in a
    /// previous session.
    pub fn has_accepted_envelope(
        &self,
        sender_hex: &str,
        message_id_hex: &str,
    ) -> Result<bool, StorageError> {
        let composite_key = format!("{sender_hex}:{message_id_hex}");
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ACCEPTED_ENVELOPES)?;
        Ok(table.get(composite_key.as_str())?.is_some())
    }

    /// Iterate all accepted envelopes in no particular order. Used to
    /// rebuild the DuckDB projection on startup.
    pub fn iter_accepted_envelopes<F>(&self, mut visit: F) -> Result<(), StorageError>
    where
        F: FnMut(AcceptedEnvelope) -> Result<(), StorageError>,
    {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ACCEPTED_ENVELOPES)?;
        for entry in table.iter()? {
            let (_key, value) = entry?;
            let parsed: AcceptedEnvelope =
                serde_json::from_slice(value.value()).map_err(|source| {
                    StorageError::MalformedAcceptedEnvelope {
                        message_id: "(unknown)".to_string(),
                        source,
                    }
                })?;
            visit(parsed)?;
        }
        Ok(())
    }

    /// Number of accepted envelopes currently stored. Convenient for
    /// tests.
    pub fn accepted_envelope_count(&self) -> Result<u64, StorageError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ACCEPTED_ENVELOPES)?;
        Ok(table.len()?)
    }

    /// Persist the outbound message-ID counter for the given node.
    pub fn save_outbound_counter(
        &self,
        node_id_hex: &str,
        counter: u64,
    ) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(OUTBOUND_COUNTER)?;
            table.insert(node_id_hex, counter)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Load the persisted outbound message-ID counter for the given
    /// node. Returns `0` when no counter has been persisted yet.
    pub fn load_outbound_counter(&self, node_id_hex: &str) -> Result<u64, StorageError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(OUTBOUND_COUNTER)?;
        Ok(table.get(node_id_hex)?.map_or(0, |v| v.value()))
    }
}

/// Persisted shape of a pairing entry. JSON-encoded so the schema can
/// gain fields without breaking the on-disk format.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PairingValue {
    /// Wall-clock at which the pairing was recorded, in milliseconds
    /// since the Unix epoch.
    pub paired_at_unix_ms: i64,
    /// Free-form operator note attached to the pairing.
    #[serde(default)]
    pub note: String,
}

/// Persisted audit record for an accepted network envelope. Stored as
/// JSON inside redb so the schema can grow without breaking the on-disk
/// format. Includes the full signed envelope data (signature and
/// payload bytes) so redb can replay or audit the exact network payload
/// that was verified.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AcceptedEnvelope {
    pub sender_hex: String,
    pub message_id_hex: String,
    pub document_id: String,
    pub payload_kind: u8,
    pub content_hash_hex: String,
    /// Hex-encoded Ed25519 signature (128 hex chars).
    #[serde(default)]
    pub signature_hex: String,
    /// Raw payload bytes (Loro delta or snapshot).
    #[serde(with = "serde_bytes", default)]
    pub payload_bytes: Vec<u8>,
    pub accepted_at_unix_ms: i64,
}

fn parse_indices(session_key: &str, raw: &str) -> Result<Vec<u64>, StorageError> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    raw.split(',')
        .map(|piece| {
            piece
                .parse::<u64>()
                .map_err(|_| StorageError::MalformedCatalog {
                    session_key: session_key.to_string(),
                    raw: raw.to_string(),
                })
        })
        .collect()
}

fn encode_indices(indices: &[u64]) -> String {
    indices
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_storage() -> (Storage, TempDir) {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("session.redb");
        let storage = Storage::open(&path).expect("open");
        (storage, tmp)
    }

    #[test]
    fn save_and_load_snapshot_round_trips() {
        let (storage, _tmp) = fresh_storage();
        let chunk = ChunkId::new("node-a", "session-1", 0);
        storage.save_snapshot(&chunk, b"hello").expect("save");
        let loaded = storage.load_snapshot(&chunk).expect("load");
        assert_eq!(loaded.as_deref(), Some(&b"hello"[..]));
    }

    #[test]
    fn list_chunks_returns_ascending_indices() {
        let (storage, _tmp) = fresh_storage();
        for idx in [2u64, 0, 1] {
            let chunk = ChunkId::new("node-a", "session-1", idx);
            storage.save_snapshot(&chunk, b"x").expect("save");
        }
        let chunks = storage.list_chunks("node-a", "session-1").expect("list");
        assert_eq!(
            chunks
                .iter()
                .map(|c| c.chunk_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2],
        );
    }

    #[test]
    fn list_chunks_for_unknown_session_is_empty() {
        let (storage, _tmp) = fresh_storage();
        let chunks = storage.list_chunks("node-a", "missing").expect("list");
        assert!(chunks.is_empty());
    }

    #[test]
    fn pairings_upsert_get_remove_round_trips() {
        let (storage, _tmp) = fresh_storage();
        storage
            .upsert_pairing("aabb", 1_234, "Bob's laptop")
            .expect("upsert");
        let fetched = storage.get_pairing("aabb").expect("get").expect("present");
        assert_eq!(fetched.paired_at_unix_ms, 1_234);
        assert_eq!(fetched.note, "Bob's laptop");

        // Upsert overwrites in place.
        storage
            .upsert_pairing("aabb", 5_000, "Bob's desktop")
            .expect("upsert again");
        let fetched = storage.get_pairing("aabb").expect("get").expect("present");
        assert_eq!(fetched.paired_at_unix_ms, 5_000);
        assert_eq!(fetched.note, "Bob's desktop");

        let removed = storage.remove_pairing("aabb").expect("remove");
        assert!(removed);
        assert!(storage.get_pairing("aabb").expect("get").is_none());
        let removed_again = storage.remove_pairing("aabb").expect("remove twice");
        assert!(!removed_again);
    }

    #[test]
    fn list_pairings_is_sorted() {
        let (storage, _tmp) = fresh_storage();
        for node in ["cc", "aa", "bb"] {
            storage.upsert_pairing(node, 1, "").expect("upsert");
        }
        let listed: Vec<String> = storage
            .list_pairings()
            .expect("list")
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(listed, vec!["aa", "bb", "cc"]);
    }

    #[test]
    fn iter_snapshots_visits_all_entries() {
        let (storage, _tmp) = fresh_storage();
        for idx in 0..3u64 {
            let chunk = ChunkId::new("n", "s", idx);
            storage
                .save_snapshot(&chunk, format!("snap-{idx}").as_bytes())
                .expect("save");
        }
        let mut seen = Vec::new();
        storage
            .iter_snapshots(|chunk, bytes| {
                seen.push((chunk.chunk_index, bytes));
                Ok(())
            })
            .expect("iter");
        seen.sort_by_key(|(idx, _)| *idx);
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[1].1, b"snap-1");
    }

    #[test]
    fn save_and_check_accepted_envelope_round_trips() {
        let (storage, _tmp) = fresh_storage();
        assert!(!storage.has_accepted_envelope("sender-hex", "ab01").expect("has"));

        let envelope = AcceptedEnvelope {
            sender_hex: "sender-hex".into(),
            message_id_hex: "ab01".into(),
            document_id: "session/n/s/0".into(),
            payload_kind: 0,
            content_hash_hex: "deadbeef".into(),
            signature_hex: "ff".repeat(64),
            payload_bytes: b"hello".to_vec(),
            accepted_at_unix_ms: 1_234,
        };
        let inserted = storage
            .save_accepted_envelope("sender-hex", "ab01", &envelope)
            .expect("save");
        assert!(inserted);

        assert!(storage.has_accepted_envelope("sender-hex", "ab01").expect("has"));
        assert_eq!(storage.accepted_envelope_count().expect("count"), 1);

        let mut visited = Vec::new();
        storage
            .iter_accepted_envelopes(|e| {
                visited.push(e);
                Ok(())
            })
            .expect("iter");
        assert_eq!(visited.len(), 1);
        assert_eq!(visited[0].message_id_hex, "ab01");
        assert_eq!(visited[0].sender_hex, "sender-hex");
        assert_eq!(visited[0].document_id, "session/n/s/0");
        assert_eq!(visited[0].payload_bytes, b"hello");
        assert_eq!(visited[0].signature_hex, "ff".repeat(64));
    }

    #[test]
    fn save_accepted_envelope_returns_false_for_duplicate() {
        let (storage, _tmp) = fresh_storage();
        let envelope = AcceptedEnvelope {
            sender_hex: "s".into(),
            message_id_hex: "cd02".into(),
            document_id: "doc".into(),
            payload_kind: 1,
            content_hash_hex: "hash".into(),
            signature_hex: String::new(),
            payload_bytes: Vec::new(),
            accepted_at_unix_ms: 999,
        };
        let first = storage
            .save_accepted_envelope("s", "cd02", &envelope)
            .expect("first");
        assert!(first);
        let second = storage
            .save_accepted_envelope("s", "cd02", &envelope)
            .expect("second");
        assert!(!second);
        assert_eq!(storage.accepted_envelope_count().expect("count"), 1);
    }

    #[test]
    fn different_senders_with_same_message_id_are_both_accepted() {
        let (storage, _tmp) = fresh_storage();
        let envelope_a = AcceptedEnvelope {
            sender_hex: "alice".into(),
            message_id_hex: "0000".into(),
            document_id: "doc".into(),
            payload_kind: 0,
            content_hash_hex: "hash-a".into(),
            signature_hex: String::new(),
            payload_bytes: Vec::new(),
            accepted_at_unix_ms: 100,
        };
        let envelope_b = AcceptedEnvelope {
            sender_hex: "bob".into(),
            message_id_hex: "0000".into(),
            document_id: "doc".into(),
            payload_kind: 0,
            content_hash_hex: "hash-b".into(),
            signature_hex: String::new(),
            payload_bytes: Vec::new(),
            accepted_at_unix_ms: 200,
        };
        assert!(storage.save_accepted_envelope("alice", "0000", &envelope_a).expect("a"));
        assert!(storage.save_accepted_envelope("bob", "0000", &envelope_b).expect("b"));
        assert_eq!(storage.accepted_envelope_count().expect("count"), 2);
    }

    #[test]
    fn outbound_counter_round_trips() {
        let (storage, _tmp) = fresh_storage();
        assert_eq!(storage.load_outbound_counter("node-x").expect("load"), 0);
        storage.save_outbound_counter("node-x", 42).expect("save");
        assert_eq!(storage.load_outbound_counter("node-x").expect("load"), 42);
        storage.save_outbound_counter("node-x", 100).expect("update");
        assert_eq!(storage.load_outbound_counter("node-x").expect("load"), 100);
    }

    #[test]
    fn accepted_envelope_persists_signature_and_payload_bytes() {
        let (storage, _tmp) = fresh_storage();
        let signature = [0xABu8; 64];
        let signature_hex: String = signature.iter().map(|b| format!("{b:02x}")).collect();
        let payload = b"crdt-delta-bytes".to_vec();

        let envelope = AcceptedEnvelope {
            sender_hex: "sender".into(),
            message_id_hex: "id1".into(),
            document_id: "session/n/s/0".into(),
            payload_kind: 1,
            content_hash_hex: "hash".into(),
            signature_hex: signature_hex.clone(),
            payload_bytes: payload.clone(),
            accepted_at_unix_ms: 999,
        };
        storage
            .save_accepted_envelope("sender", "id1", &envelope)
            .expect("save");

        let mut visited = Vec::new();
        storage
            .iter_accepted_envelopes(|e| {
                visited.push(e);
                Ok(())
            })
            .expect("iter");
        assert_eq!(visited.len(), 1);
        assert_eq!(visited[0].signature_hex, signature_hex);
        assert_eq!(visited[0].payload_bytes, payload);
    }

    #[test]
    fn per_sender_duplicate_rejection_is_scoped() {
        let (storage, _tmp) = fresh_storage();
        let env_a = AcceptedEnvelope {
            sender_hex: "alice".into(),
            message_id_hex: "0000".into(),
            document_id: "doc".into(),
            payload_kind: 0,
            content_hash_hex: "h1".into(),
            signature_hex: String::new(),
            payload_bytes: Vec::new(),
            accepted_at_unix_ms: 100,
        };
        assert!(storage.save_accepted_envelope("alice", "0000", &env_a).expect("save a"));
        assert!(storage.has_accepted_envelope("alice", "0000").expect("has a"));
        assert!(!storage.has_accepted_envelope("bob", "0000").expect("no bob"));
        let env_b = AcceptedEnvelope {
            sender_hex: "bob".into(),
            message_id_hex: "0000".into(),
            document_id: "doc".into(),
            payload_kind: 0,
            content_hash_hex: "h2".into(),
            signature_hex: String::new(),
            payload_bytes: Vec::new(),
            accepted_at_unix_ms: 200,
        };
        assert!(storage.save_accepted_envelope("bob", "0000", &env_b).expect("save b"));
        assert!(storage.has_accepted_envelope("bob", "0000").expect("has b"));
        assert!(!storage.save_accepted_envelope("alice", "0000", &env_a).expect("dup a"));
        assert!(!storage.save_accepted_envelope("bob", "0000", &env_b).expect("dup b"));
    }
}
