//! Analytical (OLAP) projection layer backed by DuckDB.
//!
//! The projection mirrors a subset of session-log entries into a
//! columnar table so the daemon can answer "give me everything from
//! session X" without rebuilding the full Loro state. The projection is
//! intentionally lossy and lagging — redb (see [`crate::storage`]) is
//! the authoritative source.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use duckdb::Connection;
use parking_lot::Mutex;
use rendezvous_core::ChunkId;

/// SQL applied at startup to ensure the projection table exists.
/// A UNIQUE constraint on `(chunk_id, sequence)` prevents duplicate
/// rows when the same chunk is projected more than once (e.g. repeated
/// incremental syncs). DuckDB `INSERT OR IGNORE` silently skips rows
/// that would violate the constraint.
const SCHEMA_SQL: &str = "
    CREATE TABLE IF NOT EXISTS session_entries (
        chunk_id TEXT,
        owner_node_id TEXT,
        session_id TEXT,
        chunk_index BIGINT,
        sequence BIGINT,
        created_at_unix_ms BIGINT,
        source TEXT,
        level TEXT,
        message TEXT,
        metadata_json TEXT,
        UNIQUE(chunk_id, sequence)
    );
";

/// One row in the analytical projection. Mirrors the fields exposed via
/// gRPC `ProjectionRow` but lives in the daemon-internal namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionRow {
    pub chunk: ChunkId,
    pub sequence: u64,
    pub created_at_unix_ms: i64,
    pub source: String,
    pub level: String,
    pub message: String,
    pub metadata_json: String,
}

/// Errors that the projection layer can surface.
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    /// The DuckDB database could not be opened.
    #[error("failed to open duckdb at {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: duckdb::Error,
    },

    /// A SQL operation failed.
    #[error("duckdb error: {0}")]
    Db(#[from] duckdb::Error),
}

/// Handle to the DuckDB-backed projection. Cloning is cheap (it clones
/// an `Arc<Mutex<Connection>>`).
#[derive(Clone)]
pub struct Projection {
    inner: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl std::fmt::Debug for Projection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Projection")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Projection {
    /// Open (or create) the DuckDB projection database.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectionError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|source| ProjectionError::Open {
                path: path.clone(),
                source: duckdb::Error::ToSqlConversionFailure(Box::new(source)),
            })?;
        }
        let conn = Connection::open(&path).map_err(|source| ProjectionError::Open {
            path: path.clone(),
            source,
        })?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
            path,
        })
    }

    /// Open an in-memory DuckDB database. Used by tests and when callers
    /// do not need durability for the projection (since it can always be
    /// rebuilt from redb).
    pub fn in_memory() -> Result<Self, ProjectionError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
            path: PathBuf::from(":memory:"),
        })
    }

    /// Filesystem path of the underlying DuckDB database, or the sentinel `":memory:"` when constructed via [`Self::in_memory`].
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Insert a batch of rows using `INSERT OR IGNORE` so that duplicate
    /// `(chunk_id, sequence)` pairs are silently skipped instead of
    /// producing duplicate rows. This makes the projection idempotent
    /// when the same chunk is submitted multiple times (e.g. repeated
    /// incremental syncs that re-project already-materialised entries).
    pub fn append_rows(&self, rows: &[ProjectionRow]) -> Result<(), ProjectionError> {
        if rows.is_empty() {
            return Ok(());
        }
        let conn = self.inner.lock();
        for row in rows {
            let chunk_index = i64::try_from(row.chunk.chunk_index).unwrap_or(i64::MAX);
            let sequence = i64::try_from(row.sequence).unwrap_or(i64::MAX);
            conn.execute(
                "INSERT OR IGNORE INTO session_entries \
                 (chunk_id, owner_node_id, session_id, chunk_index, sequence, \
                  created_at_unix_ms, source, level, message, metadata_json) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    row.chunk.as_path(),
                    row.chunk.owner_node_id.as_str(),
                    row.chunk.session_id.as_str(),
                    chunk_index,
                    sequence,
                    row.created_at_unix_ms,
                    row.source.as_str(),
                    row.level.as_str(),
                    row.message.as_str(),
                    row.metadata_json.as_str(),
                ],
            )?;
        }
        Ok(())
    }

    /// Read every row for `(owner_node_id, session_id)` in stable
    /// ascending order (`chunk_index`, then `sequence`).
    pub fn entries_for_session(
        &self,
        owner_node_id: &str,
        session_id: &str,
    ) -> Result<Vec<ProjectionRow>, ProjectionError> {
        let conn = self.inner.lock();
        let mut stmt = conn.prepare(
            "SELECT owner_node_id, session_id, chunk_index, sequence, \
             created_at_unix_ms, source, level, message, metadata_json \
             FROM session_entries \
             WHERE owner_node_id = ? AND session_id = ? \
             ORDER BY chunk_index ASC, sequence ASC",
        )?;
        let rows = stmt.query_map(duckdb::params![owner_node_id, session_id], |row| {
            let owner: String = row.get(0)?;
            let session: String = row.get(1)?;
            let chunk_index: i64 = row.get(2)?;
            let sequence: i64 = row.get(3)?;
            Ok(ProjectionRow {
                chunk: ChunkId::new(owner, session, chunk_index.max(0) as u64),
                sequence: sequence.max(0) as u64,
                created_at_unix_ms: row.get(4)?,
                source: row.get(5)?,
                level: row.get(6)?,
                message: row.get(7)?,
                metadata_json: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Total row count, used by tests to wait for batches to flush.
    pub fn row_count(&self) -> Result<u64, ProjectionError> {
        let conn = self.inner.lock();
        let value: i64 =
            conn.query_row("SELECT COUNT(*) FROM session_entries", [], |row| row.get(0))?;
        Ok(value.max(0) as u64)
    }

    /// Remove all rows from the projection table. Used during startup
    /// to rebuild the projection from the authoritative redb source of
    /// truth.
    pub fn truncate(&self) -> Result<(), ProjectionError> {
        let conn = self.inner.lock();
        conn.execute("DELETE FROM session_entries", [])?;
        Ok(())
    }

    /// Atomically replace the entire projection table with `rows`.
    ///
    /// The truncate and inserts run inside a single DuckDB transaction;
    /// if any insert fails, the previous contents remain intact.
    pub fn replace_all_rows(&self, rows: &[ProjectionRow]) -> Result<(), ProjectionError> {
        let conn = self.inner.lock();
        conn.execute_batch("BEGIN TRANSACTION;")?;
        if let Err(e) = conn.execute("DELETE FROM session_entries", []) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(e.into());
        }
        for row in rows {
            let chunk_index = i64::try_from(row.chunk.chunk_index).unwrap_or(i64::MAX);
            let sequence = i64::try_from(row.sequence).unwrap_or(i64::MAX);
            if let Err(e) = conn.execute(
                "INSERT OR IGNORE INTO session_entries \
                 (chunk_id, owner_node_id, session_id, chunk_index, sequence, \
                  created_at_unix_ms, source, level, message, metadata_json) \
                  VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    row.chunk.as_path(),
                    row.chunk.owner_node_id.as_str(),
                    row.chunk.session_id.as_str(),
                    chunk_index,
                    sequence,
                    row.created_at_unix_ms,
                    row.source.as_str(),
                    row.level.as_str(),
                    row.message.as_str(),
                    row.metadata_json.as_str(),
                ],
            ) {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(e.into());
            }
        }
        conn.execute_batch("COMMIT;")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row(idx: u64, seq: u64) -> ProjectionRow {
        ProjectionRow {
            chunk: ChunkId::new("node-a", "session-1", idx),
            sequence: seq,
            created_at_unix_ms: 1_000 + seq as i64,
            source: "test".into(),
            level: "info".into(),
            message: format!("msg-{seq}"),
            metadata_json: String::new(),
        }
    }

    #[test]
    fn append_and_query_in_memory() {
        let proj = Projection::in_memory().expect("open");
        let rows = vec![sample_row(0, 0), sample_row(0, 1), sample_row(1, 2)];
        proj.append_rows(&rows).expect("append");
        assert_eq!(proj.row_count().expect("count"), 3);

        let fetched = proj
            .entries_for_session("node-a", "session-1")
            .expect("query");
        assert_eq!(fetched.len(), 3);
        assert_eq!(fetched[0].sequence, 0);
        assert_eq!(fetched[2].chunk.chunk_index, 1);
    }

    #[test]
    fn append_empty_is_noop() {
        let proj = Projection::in_memory().expect("open");
        proj.append_rows(&[]).expect("append");
        assert_eq!(proj.row_count().expect("count"), 0);
    }

    #[test]
    fn truncate_removes_all_rows() {
        let proj = Projection::in_memory().expect("open");
        let rows = vec![sample_row(0, 0), sample_row(0, 1)];
        proj.append_rows(&rows).expect("append");
        assert_eq!(proj.row_count().expect("count"), 2);
        proj.truncate().expect("truncate");
        assert_eq!(proj.row_count().expect("count"), 0);
    }

    #[test]
    fn duplicate_rows_are_ignored() {
        let proj = Projection::in_memory().expect("open");
        let rows = vec![sample_row(0, 0), sample_row(0, 1)];
        proj.append_rows(&rows).expect("first append");
        assert_eq!(proj.row_count().expect("count"), 2);

        proj.append_rows(&rows).expect("duplicate append");
        assert_eq!(proj.row_count().expect("count"), 2);

        let new_row = sample_row(0, 2);
        proj.append_rows(&[new_row]).expect("new row");
        assert_eq!(proj.row_count().expect("count"), 3);

        let fetched = proj
            .entries_for_session("node-a", "session-1")
            .expect("query");
        assert_eq!(fetched.len(), 3);
        assert_eq!(fetched[0].sequence, 0);
        assert_eq!(fetched[1].sequence, 1);
        assert_eq!(fetched[2].sequence, 2);
    }
}
