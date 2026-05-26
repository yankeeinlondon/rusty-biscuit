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
use remote_signal_core::ChunkId;

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
        };
        storage.bootstrap_tables()?;
        Ok(storage)
    }

    /// Filesystem path of the underlying redb file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
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
        }
        txn.commit()?;
        Ok(())
    }

    /// Persist a Loro snapshot for the given chunk and ensure the
    /// session-chunks catalog includes its index. The catalog update
    /// and snapshot write happen inside a single redb transaction so the
    /// on-disk view never disagrees with the in-memory state.
    pub fn save_snapshot(&self, chunk: &ChunkId, snapshot: &[u8]) -> Result<(), StorageError> {
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
                tracing::warn!(target: "remote_signal_daemon::storage", key = %key_str, "skipping snapshot with malformed key");
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
}
