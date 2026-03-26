//! Error types for the reference analysis subsystem.

use thiserror::Error;

/// Errors produced by reference analysis.
#[derive(Debug, Error)]
pub enum ReferenceError {
    /// Failed to parse a directive.
    #[error("Failed to parse directive at line {line}: {message}")]
    ParseDirective { line: usize, message: String },

    /// Reference requires source context that is not available.
    #[error("Missing source context for reference '{reference}' at line {line}")]
    MissingSourceContext { reference: String, line: usize },

    /// A validation rule was violated.
    #[error("Validation error: {0}")]
    Validation(String),

    /// An error propagated from the compose pipeline.
    #[error("{0}")]
    Compose(Box<crate::markdown::MarkdownError>),

    /// An error from file reference resolution.
    #[error(transparent)]
    FileReference(#[from] biscuit_file::FileReferenceError),

    /// An I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A URL parsing error.
    #[error(transparent)]
    Url(#[from] url::ParseError),
}

impl From<crate::markdown::MarkdownError> for ReferenceError {
    fn from(err: crate::markdown::MarkdownError) -> Self {
        Self::Compose(Box::new(err))
    }
}
