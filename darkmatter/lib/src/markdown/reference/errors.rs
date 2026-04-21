//! Error types for the reference analysis subsystem.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced by reference analysis.
#[derive(Debug, Error)]
pub enum ReferenceError {
    /// Failed to parse a directive.
    #[error("Failed to parse directive in {} at line {line}: {message}", .source_file.display())]
    ParseDirective {
        line: usize,
        message: String,
        source_file: PathBuf,
        directive_text: String,
        caret_col: Option<usize>,
    },

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

impl biscuit_terminal::errors::BlockError for ReferenceError {
    fn status_block(
        &self,
        term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            ReferenceError::ParseDirective {
                line,
                message,
                source_file,
                directive_text,
                caret_col,
            } => {
                let caret_line = caret_col.map(|col| {
                    format!(
                        "\n  {}^",
                        " ".repeat(col.saturating_sub(1))
                    )
                });

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "ReferenceError",
                        "directive parse failed",
                    ))
                    .body(format!(
                        "<dim>Source:</dim> <cyan>{}</cyan>\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}\n<dim>Directive:</dim>\n  {directive_text}{}",
                        source_file.display(),
                        caret_line.unwrap_or_default()
                    ))
                    .hint("Expected: <cyan>::file ./doc.md</cyan>, <cyan>::code ./file.rs</cyan>, or <cyan>::url https://…</cyan>.")
            }

            ReferenceError::MissingSourceContext { reference, line } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "ReferenceError",
                    "missing source context",
                ))
                .body(format!(
                    "<dim>Reference:</dim> <cyan>{reference}</cyan>\n<dim>Line:</dim> {line}"
                ))
                .hint("Load the document from a path or URL so relative references can resolve."),

            ReferenceError::Validation(message) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "validation failed"))
                .body(message.clone())
                .hint("Review the reference graph and fix any reported cycles or dangling edges."),

            ReferenceError::Compose(inner) => {
                biscuit_terminal::errors::BlockError::status_block(inner.as_ref(), term)
            }

            ReferenceError::FileReference(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "ReferenceError",
                    "file reference error",
                ))
                .body(format!("{source}"))
                .hint("Verify the reference string and surrounding compose source."),

            ReferenceError::Io(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "I/O error"))
                .body(format!(
                    "<dim>Kind:</dim> {:?}\n{source}",
                    source.kind()
                ))
                .hint("Confirm the referenced file exists and is readable."),

            ReferenceError::Url(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "URL parse error"))
                .body(format!("{source}"))
                .hint("Use a fully qualified URL including the <cyan>https://</cyan> scheme."),
        }
    }
}
