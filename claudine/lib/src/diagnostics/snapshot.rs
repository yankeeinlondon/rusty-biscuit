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
//! - **`detail` is an opaque [`Value`].** The catalog evolves additively, so a
//!   field added after a consumer shipped round-trips through it untouched.
//!
//! The snapshot is lossless for the *public diagnostic projection*, not for
//! arbitrary private source types: a cause with no Claudine facets is not
//! representable, and a deeper unregistered prose chain is not retained.
//!
//! ## Examples
//!
//! ```
//! use claudine::composition::CompositionError;
//! use claudine::diagnostics::DiagnosticSnapshot;
//!
//! let err = CompositionError::FileNotFound("missing.md".to_string());
//! let snapshot = DiagnosticSnapshot::from_diagnostic(&err);
//!
//! assert_eq!(snapshot.code, "composition.invalid_file_reference");
//! assert_eq!(snapshot.origin, "author");
//! assert_eq!(snapshot.detail["reference"], "missing.md");
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Diagnostic, code_spec, next_registered_cause, null_detail_for};
use crate::harness::runtime::concise_message;

/// Schema version stamped into every [`DiagnosticSnapshot`].
///
/// Bumped only for a **non-additive** change — a removed or re-typed field.
/// Adding a facet, a detail key, or a new code is additive by construction and
/// does not move this number, because both sides already tolerate values they
/// do not know.
pub const DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// The public projection of the diagnostic selected to speak for a failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    /// [`DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION`] at the time of projection.
    pub schema_version: u32,

    /// Coarse domain slug — always the dotted prefix of `code`.
    pub category: String,
    /// Stable dotted code (`composition.invalid_file_reference`).
    pub code: String,
    /// Generic-strategy slug (`correctable`).
    pub disposition: String,
    /// Who must remediate (`author`).
    pub origin: String,
    /// Operator-facing severity slug (`info`/`warning`/`error`).
    pub severity: String,

    /// The catalog-shaped instance payload: every field `code` declares is a
    /// present key, with an unavailable optional carried as `null` rather than
    /// omitted.
    pub detail: Value,

    /// Concise, notification-safe message — single-line, escape-free, and
    /// length-clamped, so a TTS route, a webhook, or a desktop notification can
    /// use it verbatim. It is *not* the multi-line rendered block.
    pub message: String,

    /// The next registered diagnostic below this one, when the chain has one.
    ///
    /// One level only: [`DiagnosticCause`] carries no cause of its own, so
    /// `cause.cause` is unrepresentable in v1 rather than merely undocumented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<DiagnosticCause>,
}

/// The one-level cause projection of a [`DiagnosticSnapshot`].
///
/// Structurally identical to a snapshot minus the schema version (the parent
/// stamps it once) and minus a nested cause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticCause {
    /// Coarse domain slug.
    pub category: String,
    /// Stable dotted code.
    pub code: String,
    /// Generic-strategy slug.
    pub disposition: String,
    /// Who must remediate.
    pub origin: String,
    /// Operator-facing severity slug.
    pub severity: String,
    /// The catalog-shaped instance payload.
    pub detail: Value,
    /// Concise, notification-safe message.
    pub message: String,
}

impl DiagnosticSnapshot {
    /// Project `diagnostic` and its one-level registered cause.
    ///
    /// The caller is expected to pass the value
    /// [`select_effective_diagnostic`](super::select_effective_diagnostic)
    /// chose, so the snapshot, the rendered report, and `err.*` cannot disagree
    /// about which error spoke for the failure.
    pub fn from_diagnostic(diagnostic: &(dyn Diagnostic + 'static)) -> Self {
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
        }
    }

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
    pub fn from_code(code: &str, message: String) -> Option<Self> {
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
        })
    }
}

impl DiagnosticCause {
    /// Project `diagnostic` as a cause, dropping any cause of its own.
    pub fn from_diagnostic(diagnostic: &(dyn Diagnostic + 'static)) -> Self {
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
