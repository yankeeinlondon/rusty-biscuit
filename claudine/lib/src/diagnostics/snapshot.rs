//! The serialization boundary for a selected [`Diagnostic`].
//!
//! Concrete Rust error values never cross a process, wire, or persistence
//! boundary. At the last in-process boundary they project **once** into a
//! [`DiagnosticSnapshot`], and every downstream consumer reads that shared
//! shape rather than rebuilding facets from the error itself: the lifecycle
//! `err.*` projection embeds one (`LifecycleErrorInfo`), and the MCP import
//! report serializes one per failed config (`mcp::import::ImportError`).
//!
//! Two rules make the snapshot survive version skew in both directions
//! (spec §D9):
//!
//! - **Facets are owned strings, not the closed in-process enums.** A newer
//!   producer may know a code an older consumer's enum cannot name; the older
//!   consumer must still read, store, and forward it. Re-narrowing a string to
//!   its enum is the *consumer's* choice, made where an unknown value can be
//!   handled — never a deserialization failure.
//! - **`detail` is an opaque [`Value`](serde_json::Value).** The catalog
//!   evolves additively, so a field added after a consumer shipped round-trips
//!   through it untouched.
//!
//! The snapshot types are Darkmatter's lifecycle envelope
//! ([`darkmatter::lifecycle::envelope`]), re-exported here. What reads
//! Claudine's catalog stays here: the [`DiagnosticSnapshotExt`] and
//! [`DiagnosticCauseExt`] constructors, and [`ClaudineDiagnostics`], the
//! catalog the envelope's own constructors are given.
//!
//! The snapshot is lossless for the *public diagnostic projection*, not for
//! arbitrary private source types: a cause with no Claudine facets is not
//! representable, and a deeper unregistered prose chain is not retained.
//!
//! ## Examples
//!
//! ```
//! use claudine::composition::CompositionError;
//! use claudine::diagnostics::{DiagnosticSnapshot, DiagnosticSnapshotExt};
//!
//! let err = CompositionError::FileNotFound("missing.md".to_string());
//! let snapshot = DiagnosticSnapshot::from_diagnostic(&err);
//!
//! assert_eq!(snapshot.code, "composition.invalid_file_reference");
//! assert_eq!(snapshot.origin, "author");
//! assert_eq!(snapshot.detail["reference"], "missing.md");
//! ```

use std::error::Error as StdError;

pub use darkmatter::lifecycle::envelope::{
    DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, DiagnosticCatalog, DiagnosticCause, DiagnosticSnapshot,
};

use super::{Diagnostic, code_spec, next_registered_cause, null_detail_for};
use crate::harness::runtime::concise_message;

/// Claudine's diagnostic catalog as the lifecycle envelope's
/// [`DiagnosticCatalog`] service.
///
/// Selection is [`select_effective_diagnostic`](super::select_effective_diagnostic),
/// codes and facets come from the locked catalog, and a failure label maps
/// through [`code_for_error_kind`](super::code_for_error_kind).
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudineDiagnostics;

impl DiagnosticCatalog for ClaudineDiagnostics {
    fn select(&self, error: &(dyn StdError + 'static)) -> Option<DiagnosticSnapshot> {
        DiagnosticSnapshot::select(error)
    }

    fn snapshot_for_code(&self, code: &str, message: String) -> Option<DiagnosticSnapshot> {
        <DiagnosticSnapshot as DiagnosticSnapshotExt>::from_code(code, message)
    }

    fn snapshot_for_error_kind(&self, label: &str, message: String) -> Option<DiagnosticSnapshot> {
        super::code_for_error_kind(label)
            .and_then(|code| <DiagnosticSnapshot as DiagnosticSnapshotExt>::from_code(code, message))
    }
}

/// The constructors of [`DiagnosticSnapshot`] that read Claudine's diagnostic
/// catalog. The snapshot itself is Darkmatter's envelope data.
pub trait DiagnosticSnapshotExt: Sized {
    /// Project `diagnostic` and its one-level registered cause.
    ///
    /// The caller is expected to pass the value
    /// [`select_effective_diagnostic`](super::select_effective_diagnostic)
    /// chose, so the snapshot, the rendered report, and `err.*` cannot disagree
    /// about which error spoke for the failure.
    fn from_diagnostic(diagnostic: &(dyn Diagnostic + 'static)) -> Self;

    /// Project the diagnostic [`select_effective_diagnostic`] chooses for
    /// `error`'s chain.
    ///
    /// The seam for a boundary that holds an *erased* error — a
    /// `color_eyre::eyre::Report` (via `report.as_ref()`) or a `Box<dyn Error>`
    /// — rather than a concrete type. A `Report` boxes its source rather than
    /// discarding it, so the typed value is still reachable by downcast; this
    /// runs the same selection walk the CLI renders through, so a record that
    /// stores the result cannot disagree with what the failure printed.
    ///
    /// ## Returns
    ///
    /// `None` when nothing in the chain is a registered Claudine diagnostic —
    /// the caller's cue that only prose was ever available.
    ///
    /// [`select_effective_diagnostic`]: super::select_effective_diagnostic
    fn select(error: &(dyn StdError + 'static)) -> Option<Self>;

    /// Project a snapshot from a locked catalog `code` and a `message`.
    ///
    /// The seam for a failure that reaches a boundary as a synthesized
    /// `error_kind` label rather than a typed [`Diagnostic`] value (the
    /// provider/cap/timeout/runaway path through
    /// [`code_for_error_kind`](super::code_for_error_kind)). The
    /// `category`/`disposition`/`origin`/`severity` derive from the code's
    /// [`CodeSpec`](super::CodeSpec); the per-instance `detail` values are not
    /// reconstructable from a label, so every declared key is seeded `null` via
    /// [`null_detail_for`] — the object stays catalog-shaped rather than a
    /// top-level `null` (spec §D7). There is no typed cause to project.
    ///
    /// ## Returns
    ///
    /// `None` for a code absent from the catalog.
    fn from_code(code: &str, message: String) -> Option<Self>;
}

impl DiagnosticSnapshotExt for DiagnosticSnapshot {
    fn from_diagnostic(diagnostic: &(dyn Diagnostic + 'static)) -> Self {
        Self {
            schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
            category: diagnostic.category().as_str().to_string(),
            code: diagnostic.code().to_string(),
            disposition: diagnostic.disposition().as_str().to_string(),
            origin: diagnostic.origin().as_str().to_string(),
            // Disambiguate from the `BlockError::severity` supertrait method.
            severity: Diagnostic::severity(diagnostic).as_str().to_string(),
            detail: diagnostic.detail(),
            message: concise_message(&diagnostic.to_string()),
            cause: next_registered_cause(diagnostic).map(DiagnosticCause::from_diagnostic),
            frontmatter_excerpt: None,
        }
    }

    fn select(error: &(dyn StdError + 'static)) -> Option<Self> {
        let selected = super::select_effective_diagnostic(error)?.diagnostic()?;
        Some(Self::from_diagnostic(selected))
    }

    fn from_code(code: &str, message: String) -> Option<Self> {
        let spec = code_spec(code)?;
        Some(Self {
            schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
            category: spec.category.as_str().to_string(),
            code: spec.code.to_string(),
            disposition: spec.disposition.as_str().to_string(),
            origin: spec.origin.as_str().to_string(),
            severity: spec.severity().as_str().to_string(),
            detail: null_detail_for(spec.code),
            message,
            cause: None,
            frontmatter_excerpt: None,
        })
    }
}

/// The constructor of [`DiagnosticCause`] that reads Claudine's diagnostic
/// facets.
pub trait DiagnosticCauseExt {
    /// Project `diagnostic` as a cause, dropping any cause of its own.
    fn from_diagnostic(diagnostic: &(dyn Diagnostic + 'static)) -> Self;
}

impl DiagnosticCauseExt for DiagnosticCause {
    fn from_diagnostic(diagnostic: &(dyn Diagnostic + 'static)) -> Self {
        Self {
            category: diagnostic.category().as_str().to_string(),
            code: diagnostic.code().to_string(),
            disposition: diagnostic.disposition().as_str().to_string(),
            origin: diagnostic.origin().as_str().to_string(),
            severity: Diagnostic::severity(diagnostic).as_str().to_string(),
            detail: diagnostic.detail(),
            message: concise_message(&diagnostic.to_string()),
        }
    }
}

#[cfg(test)]
mod tests;
