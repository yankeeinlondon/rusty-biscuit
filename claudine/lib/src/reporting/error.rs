//! The typed ingest failure that reaches the sync report boundary.
//!
//! `ingest::sync_file` fails against four unrelated lower layers — the
//! filesystem, the fingerprint read, `serde_json`, and SQLite — and its caller
//! persists each outcome as a [`SyncFailure`] record. Spec §D9 forbids a
//! concrete Rust error at that persistence boundary, which is why the record
//! carries a [`DiagnosticSnapshot`]; [`IngestError`] is what keeps the failure
//! *typed* on the in-process side of it, so the projection happens **once**, at
//! the report, rather than at each innermost `?`.
//!
//! The classification this layer owns is "the reporting index could not read
//! this log file". It is therefore [`Semantic`](crate::diagnostics::DiagnosticRole::Semantic)
//! (the default role) rather than a pass-through: the precise lower identity is
//! not discarded, it survives as the snapshot's one-level `cause`.

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use serde_json::{Value, json};

use crate::diagnostics::{
    Category, Diagnostic, DiagnosticSnapshot, Disposition, Origin, code_spec, null_detail_for,
};
use crate::error::ClaudineError;

use super::types::SyncFailure;

/// One JSONL ingest failure, carried typed to the report boundary.
///
/// `line_number` is `0` for a failure that belongs to the file rather than to
/// one of its lines (metadata, fingerprint, transaction, state save).
#[derive(Debug, thiserror::Error)]
#[error("failed to ingest {source_file}: {source}")]
pub struct IngestError {
    /// JSONL file being ingested.
    pub source_file: String,
    /// One-based line that failed, or `0` for a whole-file failure.
    pub line_number: usize,
    /// The typed lower failure, still a Rust value.
    ///
    /// Deliberately **not** boxed. `Box<ClaudineError>` would satisfy
    /// `clippy::result_large_err` at `sync_file`, but `#[source]` on a box
    /// publishes the *box* as the erased cause, and `as_diagnostic`'s
    /// `downcast_ref::<ClaudineError>` misses it — the record would lose the
    /// one-level cause this whole type exists to preserve. `sync_file` allows
    /// the lint instead, which is how the rest of the area handles the same
    /// pressure.
    #[source]
    pub source: ClaudineError,
}

impl BlockError for IngestError {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("IngestError", self.code()))
            .body(self.to_string())
    }
}

impl Diagnostic for IngestError {
    fn code(&self) -> &'static str {
        // One code for every arm: what this boundary knows is that a log file
        // could not be indexed. Narrowing per lower layer here would duplicate
        // the cause's own classification, which the snapshot already carries.
        "io.read_failed"
    }

    fn category(&self) -> Category {
        code_spec(self.code())
            .map(|spec| spec.category)
            .unwrap_or(Category::Io)
    }

    fn disposition(&self) -> Disposition {
        code_spec(self.code())
            .map(|spec| spec.disposition)
            .unwrap_or(Disposition::Correctable)
    }

    fn origin(&self) -> Origin {
        code_spec(self.code())
            .map(|spec| spec.origin)
            .unwrap_or(Origin::Environment)
    }

    fn detail(&self) -> Value {
        let mut base = null_detail_for(self.code());
        base["path"] = json!(self.source_file);
        base
    }
}

impl From<IngestError> for SyncFailure {
    /// The single projection point (spec §D9).
    ///
    /// [`IngestError`] is `Semantic` and outermost, so it is already the value
    /// [`select_effective_diagnostic`](crate::diagnostics::select_effective_diagnostic)
    /// would choose for itself; the typed cause it wraps reaches the record as
    /// the snapshot's `cause` instead of being flattened a second time.
    fn from(error: IngestError) -> Self {
        let diagnostic = DiagnosticSnapshot::from_diagnostic(&error);
        SyncFailure {
            source_file: error.source_file,
            line_number: error.line_number,
            message: error.source.to_string(),
            diagnostic: Some(diagnostic),
        }
    }
}

#[cfg(test)]
mod tests;
