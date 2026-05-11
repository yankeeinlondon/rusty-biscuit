use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("clipboard backend error: {0}")]
    Backend(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("entry not found: {0}")]
    NotFound(EntryId),

    #[error("format not available: {0:?}")]
    FormatNotAvailable(ContentType),
}

use crate::content::ContentType;
use crate::entry::EntryId;
