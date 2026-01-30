//! Database module for SQLite-backed research inventory.
//!
//! This module provides connection pooling, migration management, error types,
//! and row adapter types for the SQLite database that stores research topics.
//!
//! ## Architecture
//!
//! The database layer uses a row adapter pattern to handle the mismatch between
//! Rust's rich enum types and SQLite's flat table structure:
//!
//! - **Row types** (`TopicRow`, `DocumentRow`, etc.) - Direct database representation
//! - **Domain types** (`Topic`, `Document`, etc.) - Application-level types
//! - **Converters** - Transform between row and domain types
//!
//! ## Examples
//!
//! ```no_run
//! use research_lib::metadata::db::{init_pool, run_migrations};
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = init_pool(Path::new("research.db")).await?;
//! run_migrations(&pool).await?;
//! # Ok(())
//! # }
//! ```

mod inventory;
mod queries;
mod rows;

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use thiserror::Error;

pub use inventory::ResearchInventoryDb;
pub use queries::*;
pub use rows::*;

/// Type alias for the SQLite connection pool.
pub type DbPool = SqlitePool;

/// Errors that can occur during database operations.
#[derive(Debug, Error)]
pub enum DbError {
    /// Failed to establish a database connection.
    #[error("Failed to connect to database: {0}")]
    ConnectionFailed(#[source] sqlx::Error),

    /// Failed to run database migrations.
    #[error("Migration failed: {0}")]
    MigrationFailed(#[source] sqlx::migrate::MigrateError),

    /// A database query failed.
    #[error("Query failed: {0}")]
    QueryFailed(#[source] sqlx::Error),

    /// Database is busy (SQLITE_BUSY).
    #[error("Database is busy, please retry")]
    BusyTimeout,

    /// Topic not found in database.
    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    /// Invalid enum value in database.
    #[error("Invalid enum value '{value}' for type {type_name}")]
    InvalidEnumValue { type_name: String, value: String },

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for database operations.
pub type DbResult<T> = std::result::Result<T, DbError>;

/// Initialize a SQLite connection pool with optimal settings.
///
/// Configures the pool with:
/// - `busy_timeout=5000ms` - Wait up to 5 seconds for locked database
/// - `foreign_keys=ON` - Enforce foreign key constraints
/// - `journal_mode=WAL` - Write-ahead logging for better concurrency
///
/// ## Arguments
///
/// * `path` - Path to the SQLite database file. Use `:memory:` for in-memory database.
///
/// ## Errors
///
/// Returns `DbError::ConnectionFailed` if the connection cannot be established.
pub async fn init_pool(path: &Path) -> DbResult<DbPool> {
    let path_str = path.to_string_lossy();

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .busy_timeout(std::time::Duration::from_millis(5000))
        .pragma("foreign_keys", "ON")
        .pragma("journal_mode", "WAL");

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| {
            if is_busy_error(&e) {
                DbError::BusyTimeout
            } else {
                DbError::ConnectionFailed(e)
            }
        })
        .inspect(|_| {
            tracing::debug!(db.path = %path_str, "Database pool initialized");
        })
}

/// Initialize an in-memory SQLite database pool for testing.
///
/// Uses `sqlite::memory:` with a shared cache so multiple connections
/// see the same database.
///
/// ## Errors
///
/// Returns `DbError::ConnectionFailed` if the connection cannot be established.
pub async fn init_memory_pool() -> DbResult<DbPool> {
    // Use shared cache mode for in-memory DB so migrations persist across connections
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .shared_cache(true)
        .busy_timeout(std::time::Duration::from_millis(5000))
        .pragma("foreign_keys", "ON");

    SqlitePoolOptions::new()
        .max_connections(1) // Single connection for in-memory to avoid issues
        .connect_with(options)
        .await
        .map_err(|e| {
            if is_busy_error(&e) {
                DbError::BusyTimeout
            } else {
                DbError::ConnectionFailed(e)
            }
        })
}

/// Run embedded database migrations.
///
/// Applies all pending migrations from the `migrations/` directory.
/// Migrations are embedded into the binary at compile time.
///
/// ## Errors
///
/// Returns `DbError::MigrationFailed` if any migration fails to apply.
pub async fn run_migrations(pool: &DbPool) -> DbResult<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(DbError::MigrationFailed)?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

/// Check if a sqlx error is a SQLITE_BUSY error.
fn is_busy_error(e: &sqlx::Error) -> bool {
    match e {
        sqlx::Error::Database(db_err) => {
            // SQLite BUSY error code is 5
            db_err.code().is_some_and(|code| code == "5")
        }
        _ => false,
    }
}

/// Convert a u64 hash to i64 for SQLite storage.
///
/// SQLite INTEGER type is signed 64-bit. We use bit reinterpretation
/// to store u64 values without loss of precision.
#[inline]
pub fn u64_to_i64(hash: u64) -> i64 {
    hash as i64
}

/// Convert an i64 from SQLite back to u64.
///
/// Reverses the bit reinterpretation used for storage.
#[inline]
pub fn i64_to_u64(val: i64) -> u64 {
    val as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_memory_pool() {
        let pool = init_memory_pool().await;
        assert!(pool.is_ok(), "Failed to create in-memory pool: {:?}", pool.err());
    }

    #[tokio::test]
    async fn test_run_migrations_on_memory_db() {
        let pool = init_memory_pool().await.expect("Failed to create pool");
        let result = run_migrations(&pool).await;
        assert!(result.is_ok(), "Migrations failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_migrations_create_tables() {
        let pool = init_memory_pool().await.expect("Failed to create pool");
        run_migrations(&pool).await.expect("Migrations failed");

        // Verify tables exist
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY name"
        )
        .fetch_all(&pool)
        .await
        .expect("Failed to query tables");

        let table_names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();
        assert!(table_names.contains(&"topics"), "topics table not found");
        assert!(table_names.contains(&"documents"), "documents table not found");
        assert!(table_names.contains(&"library_details"), "library_details table not found");
        assert!(table_names.contains(&"software_details"), "software_details table not found");
        assert!(table_names.contains(&"schema_meta"), "schema_meta table not found");
    }

    #[tokio::test]
    async fn test_schema_version() {
        let pool = init_memory_pool().await.expect("Failed to create pool");
        run_migrations(&pool).await.expect("Migrations failed");

        let (version,): (String,) = sqlx::query_as(
            "SELECT value FROM schema_meta WHERE key = 'schema_version'"
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to query schema version");

        assert_eq!(version, "2");
    }

    #[test]
    fn test_u64_i64_roundtrip() {
        // Test normal values
        assert_eq!(i64_to_u64(u64_to_i64(0)), 0);
        assert_eq!(i64_to_u64(u64_to_i64(12345)), 12345);

        // Test max i64 value
        let max_i64_as_u64 = i64::MAX as u64;
        assert_eq!(i64_to_u64(u64_to_i64(max_i64_as_u64)), max_i64_as_u64);

        // Test values greater than i64::MAX (critical for hash values)
        let large_u64 = u64::MAX;
        assert_eq!(i64_to_u64(u64_to_i64(large_u64)), large_u64);

        let mid_large = (i64::MAX as u64) + 1000;
        assert_eq!(i64_to_u64(u64_to_i64(mid_large)), mid_large);
    }

    #[test]
    fn test_hash_overflow_values() {
        // These are actual xxHash values that might exceed i64::MAX
        let test_hashes: Vec<u64> = vec![
            0,
            1,
            u64::MAX,
            u64::MAX - 1,
            0x8000_0000_0000_0000, // i64::MIN as u64
            0x7FFF_FFFF_FFFF_FFFF, // i64::MAX as u64
            0xFFFF_FFFF_FFFF_FFFF, // u64::MAX
            0xDEAD_BEEF_CAFE_BABE, // Random large value
        ];

        for hash in test_hashes {
            let stored = u64_to_i64(hash);
            let recovered = i64_to_u64(stored);
            assert_eq!(
                recovered, hash,
                "Roundtrip failed for hash {:#018x}: stored as {}, recovered as {:#018x}",
                hash, stored, recovered
            );
        }
    }
}
