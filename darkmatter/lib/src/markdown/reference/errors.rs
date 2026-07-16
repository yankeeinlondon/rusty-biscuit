//! Error types for the reference analysis subsystem.

use biscuit_terminal::errors::SourceContext;
use thiserror::Error;

use super::provenance::{DependencyMismatchKind, ReferenceGraphMismatch};
use crate::markdown::compose::ComposeSource;

/// A prebuilt [`ReferenceGraph`] was incompatible with a validation request.
///
/// Wraps the crate-private mismatch reason so the differing dimension (root
/// document, source, mode, options, or a visited descendant) can be reported
/// without exposing the private provenance surface or any content fingerprint.
/// Constructed only inside the crate by prebuilt-graph validation.
///
/// [`ReferenceGraph`]: super::types::ReferenceGraph
#[derive(Debug, Clone)]
pub struct ReferenceGraphMismatchError {
    reason: ReferenceGraphMismatch,
}

impl ReferenceGraphMismatchError {
    /// Wraps a mismatch reason produced by prebuilt-graph verification.
    pub(crate) fn new(reason: ReferenceGraphMismatch) -> Self {
        Self { reason }
    }

    /// One-line description of which dimension differs.
    fn headline(&self) -> String {
        match &self.reason {
            ReferenceGraphMismatch::Document => {
                "the root document's represented state changed since the graph was built".to_string()
            }
            ReferenceGraphMismatch::Source => {
                "the graph was built from a different document source".to_string()
            }
            ReferenceGraphMismatch::Mode { .. } => {
                "the graph was not built for full reference validation".to_string()
            }
            ReferenceGraphMismatch::Options => {
                "the graph was built with different reference-graph options".to_string()
            }
            ReferenceGraphMismatch::Dependency { source, kind } => {
                let name = source_display(source);
                match kind {
                    DependencyMismatchKind::Changed => {
                        format!("a transcluded child changed on disk: {name}")
                    }
                    DependencyMismatchKind::Missing => {
                        format!("a transcluded child is now missing: {name}")
                    }
                    DependencyMismatchKind::Unreadable => {
                        format!("a transcluded child is no longer readable: {name}")
                    }
                }
            }
        }
    }
}

impl std::fmt::Display for ReferenceGraphMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Prebuilt reference graph does not match this validation request: {}. \
             Rebuild the graph from the same document and options before validating.",
            self.headline()
        )
    }
}

/// Renders a compose source for a diagnostic, never exposing a content hash.
fn source_display(source: &ComposeSource) -> String {
    match source {
        ComposeSource::File(p) => p.display().to_string(),
        ComposeSource::Url(u) => u.to_string(),
        ComposeSource::Unknown => "unknown".to_string(),
    }
}

/// Errors produced by reference analysis.
#[derive(Debug, Error)]
pub enum ReferenceError {
    /// Failed to parse a directive.
    #[error("Failed to parse directive in {} at line {line}: {message}", .ctx.display.display())]
    ParseDirective {
        ctx: Box<SourceContext>,
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

    /// A prebuilt reference graph did not match the validation request.
    ///
    /// Raised by [`Markdown::validate_references_with_graph`] before the graph
    /// is flattened when the graph's build provenance (document, source, mode,
    /// options) or a visited descendant no longer matches the request.
    ///
    /// [`Markdown::validate_references_with_graph`]: crate::markdown::Markdown::validate_references_with_graph
    #[error("{0}")]
    ReferenceGraphMismatch(ReferenceGraphMismatchError),

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
                        "<dim>Gutter:</dim> Column {col} is near the error in: <dim>{}</dim>",
                        Prose::escape_text(directive_text)
                    )));
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ReferenceError", "directive parse failed"))
                    .body(body)
                    .hint(format!(
                        "Error: {}\nCheck syntax: <cyan>::file path=\"...\"</cyan>",
                        Prose::escape_text(message)
                    ))
            }

            ReferenceError::MissingSourceContext { reference, line } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ReferenceError", "missing source context"))
                    .body(vec![
                        Prose::new(format!(
                            "Could not resolve <cyan>{}</cyan> at line {line}.",
                            Prose::escape_text(reference)
                        )),
                        Prose::new(
                            "<dim>Note:</dim> Relative references require a file-backed source.",
                        ),
                    ])
                    .hint("Try using an absolute path or `@/` repo-root reference.")
            }

            ReferenceError::Validation(message) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "validation failed"))
                .body(Prose::escape_text(message))
                .hint("Review the reference graph and fix any reported cycles or dangling edges."),

            ReferenceError::ReferenceGraphMismatch(err) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ReferenceError", "graph out of date"))
                .body(vec![
                    Prose::new(Prose::escape_text(&err.headline())),
                    Prose::new(
                        "<dim>Note:</dim> the prebuilt graph no longer matches this document, its \
                         source, mode, options, or a transcluded child.",
                    ),
                ])
                .hint(
                    "Rebuild the graph from the same document and options, or call \
                     `validate_references` to build and validate in one step.",
                ),

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
