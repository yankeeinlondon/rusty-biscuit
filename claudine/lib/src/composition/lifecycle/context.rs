//! Lifecycle execution context: the values of the lifecycle globals `err` and
//! `timing`.
//!
//! The values are Darkmatter's lifecycle envelope
//! ([`darkmatter::lifecycle::context`]), re-exported here. The constructors
//! that read Claudine's error types or its diagnostic catalog are
//! [`LifecycleErrorInfoExt`].
//!
//! These globals supplement the document state at event time. Which scopes
//! declare them, and as what, is the binding catalog's policy
//! ([`super::bindings`]); this module owns their values. Both are eager: they
//! are already captured by the time the event fires. The late-binding *facts* —
//! `current.<key>` — are not globals at all. They are a Darkmatter reserved
//! root served by the request's `CurrentAuthority`, which the executor installs
//! on the event's effective state (spec R30–R33).
//!
//! ## `err`
//!
//! Carries the active failure (if any) into `blocked`, `failure`, and the
//! optional-error `finalize` events. Exposed through the [`Diagnostic`](crate::diagnostics::Diagnostic) facets
//! `err.code` / `err.category` / `err.disposition` / `err.origin` /
//! `err.severity` / `err.detail.*` (when the source error is classifiable),
//! the one-level `err.cause.*` projection of the next registered diagnostic
//! below it, plus the deprecated aliases `err.kind` / `err.variant` /
//! `err.msg` — `err.kind` / `err.variant` are deprecated spellings of
//! `err.category` / `err.code`.
//!
//! Which error in a chain answers all of this is decided by
//! [`select_effective_diagnostic`](crate::diagnostics::select_effective_diagnostic) —
//! the same walk the CLI renders through and
//! the snapshot serializes through, so a route cannot classify one cause while
//! rendering another. The binding catalog declares `err` unavailable in
//! `initialize`, `start`, `success`, and `loop`, so a read there fails
//! preparation; in `finalize` and task teardown with no failure it is `null`.
//!
//! ## `timing`
//!
//! Carries observed durations. All fields are optional so the public shape
//! never commits to a value the runtime may not have captured.
//!
use std::error::Error as StdError;

pub use darkmatter::lifecycle::context::{
    LifecycleCause, LifecycleErrorInfo, LifecycleEvaluationReason, LifecycleTiming,
};

use super::executor::LifecycleExprError;

use super::super::error::CompositionError;
use crate::diagnostics::ClaudineDiagnostics;
use crate::error::ClaudineError;
use crate::harness::error::HarnessError;

/// The [`LifecycleErrorInfo`] constructors that read Claudine's error types and
/// its diagnostic catalog ([`ClaudineDiagnostics`]).
///
/// Each delegates the projection to Darkmatter's catalog-taking constructor,
/// so `err.*`, terminal rendering, and machine output keep reading one value.
pub trait LifecycleErrorInfoExt: Sized {
    /// Build the snapshot from a [`ClaudineError`], capturing its
    /// [`Diagnostic`](crate::diagnostics::Diagnostic) facets so `err.code` /
    /// `err.detail.*` project alongside the legacy aliases. Covers the
    /// top-level Claudine and provider error surface (`provider.unavailable`,
    /// `io.*`, `config.*`, `usage.*`, …).
    fn from_claudine_error(err: &ClaudineError) -> Self;

    /// Build the snapshot from a [`HarnessError`], capturing its
    /// [`Diagnostic`](crate::diagnostics::Diagnostic) facets so `err.code` /
    /// `err.detail.*` project alongside the legacy aliases.
    fn from_harness_error(err: &HarnessError) -> Self;

    /// Build the snapshot from a [`CompositionError`], capturing its
    /// [`Diagnostic`](crate::diagnostics::Diagnostic) facets so `err.code` /
    /// `err.detail.*` project alongside the legacy aliases. The error's
    /// lifecycle cause and frontmatter excerpt are kept.
    fn from_composition_error(err: &CompositionError) -> Self;

    /// Build the snapshot from a [`ProxyCommitError`], keeping whichever
    /// concrete error the failed hand-off carried.
    ///
    /// A refused hand-off is routed through the source's `failure`/`finalize`
    /// like any other setup failure, so its `err.*` facets must be as typed as
    /// a resolution or cycle failure reached any other way — a proxy cycle
    /// projects `err.code` off its [`CompositionError`], not off a rendered
    /// string. Resolution failures carry the authoring-context
    /// [`CompositionError::InvalidFileReference`] wrapper; its typed resolver
    /// remains the source.
    ///
    /// [`ProxyCommitError`]: crate::composition::ProxyCommitError
    fn from_proxy_commit_error(err: &crate::composition::ProxyCommitError) -> Self;

    /// Build the snapshot from an arbitrary error, preferring the effective
    /// diagnostic and falling back to a facet-less action failure labeled
    /// `verb` when nothing in the chain is registered; see
    /// [`LifecycleErrorInfo::from_error_or_action_with`].
    ///
    /// The seam for a boundary that holds a typed error behind an opaque
    /// `color_eyre` `Report` or a `Box<dyn Error>`.
    fn from_error_or_action(verb: impl Into<String>, error: &(dyn StdError + 'static)) -> Self;

    /// [`Self::from_error_or_action`] for an event-time evaluation failure,
    /// keeping the typed error as [`LifecycleErrorInfo::cause`].
    fn from_expr_error(verb: impl Into<String>, error: LifecycleExprError) -> Self;

    /// Build a snapshot for a failed lifecycle stack action; see
    /// [`LifecycleErrorInfo::from_action_failure_with`].
    ///
    /// When `verb` is a recognized provider/cap/timeout/runaway `error_kind`
    /// label (the wrapper attributes a guard trip or provider failure honestly
    /// via [`crate::diagnostics::code_for_error_kind`]), the matching catalog
    /// facets are attached. `msg` passes through
    /// [`concise_message`](crate::harness::concise_message), the same hygiene
    /// [`failure_message`](crate::harness::failure_message) applies on the
    /// provider-attempt path; the pass is idempotent, which is what lets the
    /// provider cascade keep its budget-reserved `(attempt N)` suffix.
    fn from_action_failure(verb: impl Into<String>, msg: impl Into<String>) -> Self;
}

impl LifecycleErrorInfoExt for LifecycleErrorInfo {
    fn from_claudine_error(err: &ClaudineError) -> Self {
        Self::from_selection_with(&ClaudineDiagnostics, "ClaudineError", variant_name_from_debug(err), err)
    }

    fn from_harness_error(err: &HarnessError) -> Self {
        Self::from_selection_with(&ClaudineDiagnostics, "HarnessError", variant_name_from_debug(err), err)
    }

    fn from_composition_error(err: &CompositionError) -> Self {
        let mut info = Self::from_selection_with(
            &ClaudineDiagnostics,
            "CompositionError",
            variant_name_from_debug(err),
            err,
        );
        info.cause = err.lifecycle_cause().cloned();
        if let (Some(snapshot), Some(excerpt)) =
            (info.snapshot.as_mut(), err.frontmatter_excerpt())
        {
            snapshot.frontmatter_excerpt =
                Some(darkmatter::lifecycle::SourceExcerpt::new(excerpt.clone()));
        }
        info
    }

    fn from_proxy_commit_error(err: &crate::composition::ProxyCommitError) -> Self {
        match err {
            crate::composition::ProxyCommitError::Resolution { error, .. }
            | crate::composition::ProxyCommitError::Rejected(error) => {
                Self::from_composition_error(error)
            }
        }
    }

    fn from_error_or_action(verb: impl Into<String>, error: &(dyn StdError + 'static)) -> Self {
        Self::from_error_or_action_with(&ClaudineDiagnostics, verb, error)
    }

    fn from_expr_error(verb: impl Into<String>, error: LifecycleExprError) -> Self {
        Self::from_expr_error_with(&ClaudineDiagnostics, verb, error)
    }

    fn from_action_failure(verb: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::from_action_failure_with(&ClaudineDiagnostics, verb, msg)
    }
}

/// Extract the variant name from a `Debug` rendering of an error.
///
/// `thiserror::Error` derive produces a `Debug` impl whose leading token
/// is the variant name (e.g. `Io(...)`, `ShellCommandDenied { ... }`,
/// `LifecycleInvalid { .. }`). We slice up to the first `(`, `{`, or
/// whitespace to recover that name.
fn variant_name_from_debug<T: std::fmt::Debug>(err: &T) -> String {
    let rendered = format!("{err:?}");
    let end = rendered
        .find(['(', ' ', '{'])
        .unwrap_or(rendered.len());
    rendered[..end].to_string()
}

#[cfg(test)]
mod tests;
