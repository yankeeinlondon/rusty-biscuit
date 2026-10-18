//! Why a store could not be used.

use std::io;

/// A store failure. Every one ends in memory facts (R12) except
/// [`StoreFailure::Corrupt`] and [`StoreFailure::RootMismatch`] at open,
/// which first delete and recreate the store once.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreFailure {
    /// Not a database, a malformed file, a missing table, a format stamp
    /// that does not match the file name, or a row that does not decode.
    #[error("corrupt store: {0}")]
    Corrupt(String),
    /// The store was created for another root (a key collision).
    #[error("store belongs to another root: {stored}")]
    RootMismatch { stored: String },
    /// Locked beyond the busy timeout, or the WAL switch stayed busy.
    #[error("store busy: {0}")]
    Busy(String),
    /// Repair needed the exclusive lock, but another process holds the store.
    #[error("store in use by another process")]
    InUse,
    /// Read-only, full, cannot be opened, or another I/O failure.
    #[error("store unavailable: {0}")]
    Unavailable(String),
}

impl StoreFailure {
    /// A short, stable label for the `reason` field of the fallback log line.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Corrupt(_) => "corrupt",
            Self::RootMismatch { .. } => "root-mismatch",
            Self::Busy(_) => "busy",
            Self::InUse => "in-use",
            Self::Unavailable(_) => "unavailable",
        }
    }
}

impl From<rusqlite::Error> for StoreFailure {
    fn from(error: rusqlite::Error) -> Self {
        use rusqlite::{Error, ErrorCode};
        match &error {
            Error::SqliteFailure(failure, _) => match failure.code {
                ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase => Self::Corrupt(error.to_string()),
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => Self::Busy(error.to_string()),
                _ => Self::Unavailable(error.to_string()),
            },
            // A stored value that does not decode into its fact.
            Error::FromSqlConversionFailure(..)
            | Error::IntegralValueOutOfRange(..)
            | Error::InvalidColumnType(..)
            | Error::Utf8Error(..)
            | Error::InvalidColumnIndex(..)
            | Error::InvalidColumnName(..) => Self::Corrupt(error.to_string()),
            _ => Self::Unavailable(error.to_string()),
        }
    }
}

impl From<io::Error> for StoreFailure {
    fn from(error: io::Error) -> Self {
        Self::Unavailable(error.to_string())
    }
}
