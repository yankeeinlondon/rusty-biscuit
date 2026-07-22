//! The inverse of the snapshot boundary: a [`DiagnosticSnapshot`] rehydrated
//! into a live [`Diagnostic`].
//!
//! [`snapshot`](super::snapshot) exists because concrete Rust error values
//! cannot be cloned, stored, or sent. Some of them are captured *early* and
//! carried through a `Clone` record — `CompositionPrepContext` snapshots the
//! launch-detection failure at prep time so `--repo` can fail hard on it much
//! later. When that moment arrives the snapshot has to become an error again,
//! and the tempting shortcut is to lift `snapshot.message` into an `eyre!`
//! string. That is a *second* erasure: the code, category, disposition,
//! origin, detail, and cause the projection preserved are thrown away at the
//! one boundary that still had them.
//!
//! [`RestoredDiagnostic`] is what a boundary returns instead. Its facets come
//! from the snapshot, so the failure re-enters
//! [`select_effective_diagnostic`](super::select_effective_diagnostic) and
//! renders as a `StatusBlock` with the identity it was projected with.
//! Re-projecting one is a fixed point: `DiagnosticSnapshot::from_diagnostic`
//! over a `RestoredDiagnostic` yields the snapshot it was built from.
//!
//! ## Examples
//!
//! ```
//! use claudine::composition::CompositionError;
//! use claudine::diagnostics::{DiagnosticSnapshot, RestoredDiagnostic};
//!
//! let snapshot =
//!     DiagnosticSnapshot::from_diagnostic(&CompositionError::FileNotFound("a.md".into()));
//! let restored = RestoredDiagnostic::new(snapshot.clone());
//!
//! assert_eq!(DiagnosticSnapshot::from_diagnostic(&restored), snapshot);
//! ```

use std::error::Error as StdError;
use std::fmt;

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use serde_json::Value;

use super::{
    Category, CodeSpec, DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, Diagnostic, DiagnosticCause,
    DiagnosticSnapshot, Disposition, Origin, Severity, code_spec,
};

/// The code a snapshot degrades to when its own code is absent from this
/// build's catalog.
///
/// Facets are owned strings precisely so a newer producer's code survives an
/// older consumer, but [`Diagnostic::code`] returns `&'static str` and an
/// arbitrary owned string cannot become one. A snapshot restored *in-process*
/// always names a code this binary compiled, so the fallback is unreachable in
/// practice; it exists so restoration is total rather than fallible at a
/// boundary that has no better error to return. `detail` and `message` still
/// carry through untouched.
const UNKNOWN_CODE_FALLBACK: &str = "internal.bug";

/// A [`DiagnosticSnapshot`] presented as a live [`Diagnostic`] again.
///
/// Carries the snapshot verbatim; the facet accessors read the catalog row for
/// its `code` rather than re-parsing the owned facet strings, which is the
/// same derivation [`DiagnosticSnapshot::from_code`] uses.
#[derive(Debug)]
pub struct RestoredDiagnostic {
    snapshot: DiagnosticSnapshot,
    /// The snapshot's one-level cause, itself restored, so
    /// [`next_registered_cause`](super::next_registered_cause) can reach it and
    /// a re-projection carries the same `cause`.
    cause: Option<Box<RestoredDiagnostic>>,
    /// Prose the restoring boundary adds in front of the snapshot's message —
    /// *why this failure is fatal here*, which the snapshot cannot know.
    /// Rendering and `Display` show it; the facets are untouched by it.
    context: Option<String>,
    spec: &'static CodeSpec,
}

impl RestoredDiagnostic {
    /// Restore `snapshot`, and its one-level cause, to a live diagnostic.
    pub fn new(snapshot: DiagnosticSnapshot) -> Self {
        let spec = resolve_spec(&snapshot.code);
        let cause = snapshot
            .cause
            .as_ref()
            .map(|cause| Box::new(Self::new(snapshot_from_cause(cause))));
        Self {
            snapshot,
            cause,
            context: None,
            spec,
        }
    }

    /// Prefix the restored message with the boundary's own framing.
    ///
    /// Use for the context the snapshot genuinely does not hold — "`--repo`
    /// requires startup repo detection", not a restatement of what failed.
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// The snapshot this diagnostic was restored from.
    pub fn snapshot(&self) -> &DiagnosticSnapshot {
        &self.snapshot
    }
}

/// The catalog row for `code`, degrading to [`UNKNOWN_CODE_FALLBACK`].
fn resolve_spec(code: &str) -> &'static CodeSpec {
    code_spec(code).unwrap_or_else(|| {
        code_spec(UNKNOWN_CODE_FALLBACK).expect("the fallback code is in the locked catalog")
    })
}

/// Widen a one-level [`DiagnosticCause`] back into a causeless snapshot.
fn snapshot_from_cause(cause: &DiagnosticCause) -> DiagnosticSnapshot {
    DiagnosticSnapshot {
        schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: cause.category.clone(),
        code: cause.code.clone(),
        disposition: cause.disposition.clone(),
        origin: cause.origin.clone(),
        severity: cause.severity.clone(),
        detail: cause.detail.clone(),
        message: cause.message.clone(),
        cause: None,
    }
}

impl fmt::Display for RestoredDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(context) => write!(f, "{context}: {}", self.snapshot.message),
            None => f.write_str(&self.snapshot.message),
        }
    }
}

impl StdError for RestoredDiagnostic {
    /// Deliberately `None`: the restored cause reaches the selection walk
    /// through [`Diagnostic::diagnostic_source`] instead, so `color_eyre`'s
    /// cause-chain fallback does not print a second, near-identical line for a
    /// projection of the same failure.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

impl BlockError for RestoredDiagnostic {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("Error", self.spec.code))
            .body(self.to_string())
    }

    fn block_source(&self) -> Option<&(dyn BlockError + 'static)> {
        self.cause.as_deref().map(|cause| cause as &dyn BlockError)
    }
}

impl Diagnostic for RestoredDiagnostic {
    fn category(&self) -> Category {
        self.spec.category
    }

    fn code(&self) -> &'static str {
        self.spec.code
    }

    fn disposition(&self) -> Disposition {
        self.spec.disposition
    }

    fn origin(&self) -> Origin {
        self.spec.origin
    }

    fn detail(&self) -> Value {
        self.snapshot.detail.clone()
    }

    fn severity(&self) -> Severity {
        self.spec.severity()
    }

    fn diagnostic_source(&self) -> Option<&(dyn StdError + 'static)> {
        self.cause
            .as_deref()
            .map(|cause| cause as &(dyn StdError + 'static))
    }
}

#[cfg(test)]
mod tests;
