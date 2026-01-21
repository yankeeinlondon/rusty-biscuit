//! Error types for the queue library.

use thiserror::Error;

/// Errors that can occur when working with history storage.
#[derive(Debug, Error)]
pub enum HistoryError {
    /// Failed to read from the history file.
    #[error("failed to read history: {0}")]
    Read(#[from] std::io::Error),

    /// Failed to parse a history entry.
    #[error("failed to parse history: {0}")]
    Parse(#[from] serde_json::Error),

    /// Failed to acquire a file lock.
    #[error("failed to acquire lock")]
    Lock,
}
