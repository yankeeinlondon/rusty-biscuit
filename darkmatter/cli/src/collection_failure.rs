//! The terminal and `--output report-json` report of a failed `::map` /
//! `::filter` collection.
//!
//! The library's own block names the code and the iteration path but not the
//! authored location, and its diagnostic record names the location but not
//! the path. [`CollectionFailure`] carries both to both sinks from the one
//! typed [`CollectionError`], so the terminal and the report agree on code,
//! file, line, column, and path.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::compose::collection_blocks::CollectionError;
use darkmatter::markdown::compose::expression::Diagnostic;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::{MarkdownError, SourceRef};

/// A collection failure with everything its sinks print.
#[derive(Debug)]
pub struct CollectionFailure {
    error: CollectionError,
    diagnostics: Vec<Diagnostic>,
}

impl CollectionFailure {
    /// Wraps `error`, deriving its diagnostic record.
    pub fn new(error: CollectionError) -> Self {
        let path = error.iteration_path();
        let mut diagnostics = error_diagnostics(&MarkdownError::Collection(Box::new(error.clone())));
        if !path.is_empty() {
            let path = path.to_string();
            for diagnostic in &mut diagnostics {
                // The record's message may already name the path.
                if !diagnostic.message.contains(&path) {
                    diagnostic.message = format!("{} at {path}", diagnostic.message);
                }
            }
        }
        Self { error, diagnostics }
    }

    /// The `--output report-json` records.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl std::fmt::Display for CollectionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Collection error: {}", self.error)
    }
}

impl std::error::Error for CollectionFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl BlockError for CollectionFailure {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        let mut body = vec![Prose::new(Prose::escape_text_outside_code_spans(&self.error.to_string()))];
        if let Some(site) = self.error.site() {
            match site.source.as_ref() {
                SourceRef::OnDiskSpan { context, span } => {
                    body.push(Prose::new("Defined in:"));
                    body.push(context.linked_path_prose());
                    body.push(Prose::new(format!("Collection at line: {}, column: {}", span.line(), span.column())));
                    body.push(context.excerpt_prose(span.line(), 2, "markdown"));
                }
                SourceRef::OnDisk(context) if context.display != std::path::Path::new("unknown") => {
                    body.push(Prose::new("Defined in:"));
                    body.push(context.linked_path_prose());
                }
                SourceRef::OnDisk(_) | SourceRef::Supplied { .. } | SourceRef::Effective { .. } => {}
            }
        }
        let path = self.error.iteration_path();
        if !path.is_empty() {
            body.push(Prose::new(format!(
                "Iteration path: {}",
                Prose::escape_text_outside_code_spans(&path.to_string())
            )));
        }
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("CollectionError", self.error.code()))
            .body(body)
    }
}
