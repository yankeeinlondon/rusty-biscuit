//! Error types for the reference analysis subsystem.

use std::path::PathBuf;

use biscuit_terminal::errors::SourceContext;
use thiserror::Error;

/// Errors produced by reference analysis.
#[derive(Debug, Error)]
pub enum ReferenceError {
    /// Failed to parse a directive.
    #[error("Failed to parse directive in {} at line {line}: {message}", .ctx.display.display())]
    ParseDirective {
        ctx: SourceContext,
        line: usize,
        message: String,
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
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            ReferenceError::ParseDirective {
                ctx,
                line,
                message,
                directive_text,
                caret_col,
            } => {
                let mut body = vec![Prose::new(format!(
                    "Directive parsing failed in {}:",
                    ctx.linked_path_prose().content()
                ))];
                if let Some(fm) = ctx.frontmatter_prose() {
                    body.push(Prose::new("The Frontmatter of this document was:"));
                    body.push(fm);
                }
                body.push(ctx.excerpt_prose(*line, 1, "md"));

                if let Some(col) = caret_col {
                    body.push(Prose::new(format!(
                        "<dim>Gutter:</dim> Column {col} is near the error in: <dim>{directive_text}</dim>"
                    )));
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ReferenceError", "directive parse failed"))
                    .body(body)
                    .hint(format!(
                        "Error: {message}\nCheck syntax: <cyan>::file path=\"...\"</cyan>"
                    ))
            }

            ReferenceError::MissingSourceContext { reference, line } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ReferenceError", "missing source context"))
                    .body(vec![
                        Prose::new(format!(
                            "Could not resolve <cyan>{reference}</cyan> at line {line}."
                        )),
                        Prose::new(
                            "<dim>Note:</dim> Relative references require a file-backed source.",
                        ),
                    ])
                    .hint("Try using an absolute path or `@/` repo-root reference.")
            }

            ReferenceError::Validation(message) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "validation failed"))
                .body(message.clone())
                .hint("Review the reference graph and fix any reported cycles or dangling edges."),

            ReferenceError::Compose(inner) => inner.status_block(term),

            ReferenceError::FileReference(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "file reference failure"))
                .body(source.to_string())
                .hint("Check repository-root (`@/`) or package (`!`) prefix usage."),

            ReferenceError::Io(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "I/O failure"))
                .body(source.to_string())
                .hint("Check file existence and permissions."),

            ReferenceError::Url(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "URL parse failure"))
                .body(source.to_string())
                .hint("Verify the URL scheme and format (e.g., https://example.com)."),
        }
    }
}
