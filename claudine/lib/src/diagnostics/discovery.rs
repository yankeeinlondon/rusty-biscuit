//! Discovery and selection of the **effective diagnostic** in a cause chain.
//!
//! Two seams live here:
//!
//! - [`as_diagnostic`] — Claudine's single downcast allowlist, the counterpart
//!   to Darkmatter's [`as_block_error`]. Stable Rust cannot upcast an arbitrary
//!   `&dyn Error` to `&dyn Diagnostic`, so the set of concrete types is
//!   hand-authored. That list is only exhaustive because a source-parity test
//!   compares it against every production `impl Diagnostic for …`.
//! - [`select_effective_diagnostic`] — the one walk that decides *which* error
//!   in a chain speaks for the failure. Rendering, `err.*`, and machine output
//!   all resolve through it, so a route cannot classify one cause while
//!   rendering another.
//!
//! Selection walks outer-to-inner. The first [`Semantic`] diagnostic wins and
//! stops; when the chain holds only [`Transparent`] diagnostics and lower-layer
//! blocks, the deepest candidate wins.
//!
//! See `claudine/features/2026-07-13-error-propogation/spec.md` §D2 and §D4.
//!
//! [`Semantic`]: DiagnosticRole::Semantic
//! [`Transparent`]: DiagnosticRole::Transparent

use std::error::Error as StdError;

use biscuit_terminal::errors::BlockError;
use darkmatter::markdown::errors::as_block_error;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::Diagnostic;
use crate::composition::CompositionError;
use crate::error::ClaudineError;
use crate::harness::error::HarnessError;

/// Maximum number of causes the selection walk visits.
///
/// A malformed third-party `source()` implementation must not hang error
/// reporting. The cap is deliberately far above any real Claudine chain (the
/// deepest today is ~6), so hitting it means the chain is pathological.
pub const MAX_SELECTION_DEPTH: usize = 64;

/// Whether a diagnostic speaks for the failure or defers to its cause.
///
/// The role is data, never inferred from an enum name or `Display` text: a
/// wrapper that looks generic may deliberately reclassify, and one that looks
/// specific may be a pass-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticRole {
    /// The boundary deliberately classifies the operation and owns both its
    /// facets and its rendering. Selection stops here.
    ///
    /// Owning the rendering does not mean rendering *locally*: a semantic
    /// wrapper may build its [`BlockError::status_block`] from its cause's
    /// block (as `CompositionError::ShellExpansionFailed` does) while still
    /// owning the code. What makes it semantic is that the facets are its own.
    Semantic,
    /// The wrapper delegates **both** facets and rendering to its cause, so it
    /// has no identity of its own. Selection continues through it.
    Transparent,
}

/// The error selected to speak for a failure.
///
/// A Claudine diagnostic carries facets *and* rendering. A lower-layer cause
/// (Darkmatter) is renderable only — it is selected when no Claudine
/// diagnostic in the chain owns the failure, which under Option A means a
/// wrapper is missing rather than that the facets are unavailable.
#[derive(Debug, Clone, Copy)]
pub enum EffectiveDiagnostic<'a> {
    /// A Claudine diagnostic: renderable and classifiable.
    Claudine(&'a (dyn Diagnostic + 'static)),
    /// A lower-layer `BlockError` with no Claudine facets.
    LowerLayer(&'a (dyn BlockError + 'static)),
}

impl<'a> EffectiveDiagnostic<'a> {
    /// The value that renders the human-facing report.
    pub fn block_error(self) -> &'a (dyn BlockError + 'static) {
        match self {
            EffectiveDiagnostic::Claudine(diagnostic) => diagnostic,
            EffectiveDiagnostic::LowerLayer(block) => block,
        }
    }

    /// The value that supplies `err.*` facets, when the selection has any.
    pub fn diagnostic(self) -> Option<&'a (dyn Diagnostic + 'static)> {
        match self {
            EffectiveDiagnostic::Claudine(diagnostic) => Some(diagnostic),
            EffectiveDiagnostic::LowerLayer(_) => None,
        }
    }
}

/// Try to view `error` as one of Claudine's [`Diagnostic`] implementations.
///
/// The complete Claudine allowlist. `claudine-cli` renders through this rather
/// than keeping its own partial downcast list.
///
/// ## Examples
///
/// ```
/// use std::error::Error;
/// use claudine::diagnostics::as_diagnostic;
/// use claudine::composition::CompositionError;
///
/// let err = CompositionError::FileNotFound("missing.md".to_string());
/// let erased: &(dyn Error + 'static) = &err;
/// assert_eq!(
///     as_diagnostic(erased).map(|d| d.code()),
///     Some("composition.invalid_file_reference"),
/// );
/// ```
pub fn as_diagnostic<'a>(
    error: &'a (dyn StdError + 'static),
) -> Option<&'a (dyn Diagnostic + 'static)> {
    if let Some(v) = error.downcast_ref::<CompositionError>() {
        return Some(v);
    }
    if let Some(v) = error.downcast_ref::<ClaudineError>() {
        return Some(v);
    }
    if let Some(v) = error.downcast_ref::<HarnessError>() {
        return Some(v);
    }
    None
}

/// Select the diagnostic that speaks for `error`'s failure.
///
/// Walks outer-to-inner: the first [`DiagnosticRole::Semantic`] Claudine
/// diagnostic wins immediately; otherwise the deepest candidate — a
/// [`DiagnosticRole::Transparent`] Claudine diagnostic or a Darkmatter
/// [`BlockError`] — wins, so a generic wrapper cannot hide a richer cause.
///
/// Returns `None` when nothing in the chain is renderable, which is the
/// caller's cue to fall back to unstructured output.
///
/// ## Examples
///
/// ```
/// use std::error::Error;
/// use claudine::diagnostics::select_effective_diagnostic;
/// use claudine::composition::CompositionError;
///
/// // An already-emitted wrapper is transparent: the inner error is selected,
/// // not the wrapper.
/// let inner = CompositionError::FileNotFound("missing.md".to_string());
/// let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
///     inner: Box::new(inner),
/// };
/// let erased: &(dyn Error + 'static) = &wrapped;
/// let selected = select_effective_diagnostic(erased).unwrap();
/// assert_eq!(
///     selected.diagnostic().map(|d| d.code()),
///     Some("composition.invalid_file_reference"),
/// );
/// ```
pub fn select_effective_diagnostic<'a>(
    error: &'a (dyn StdError + 'static),
) -> Option<EffectiveDiagnostic<'a>> {
    select_with(error, as_diagnostic)
}

/// The selection walk, parameterized by its Claudine discovery function.
///
/// `discover` is [`as_diagnostic`] in production. The seam exists because the
/// registry is a closed downcast list over concrete types: every Claudine
/// diagnostic that terminates a chain today is `Semantic`, so the
/// deepest-transparent and guard-with-candidate rules are unreachable from the
/// public entry point until a transparent leaf type ships. Tests supply a probe
/// registry to exercise them.
fn select_with<'a>(
    error: &'a (dyn StdError + 'static),
    discover: fn(&'a (dyn StdError + 'static)) -> Option<&'a (dyn Diagnostic + 'static)>,
) -> Option<EffectiveDiagnostic<'a>> {
    let mut best: Option<EffectiveDiagnostic<'a>> = None;
    let mut visited: Vec<*const (dyn StdError + 'static)> = Vec::new();
    let mut current = Some(error);

    while let Some(err) = current {
        // Identity is the whole wide pointer (address *and* vtable): a
        // `#[from]` payload can share its parent's address, and treating that
        // as a revisit would truncate a healthy chain. Comparing vtables can
        // miss a genuine revisit across codegen units, which only defers the
        // stop to the depth guard.
        let identity: *const (dyn StdError + 'static) = err;
        if visited.contains(&identity) {
            // Both guards keep `best`: a malformed chain still reports the best
            // cause seen before the chain went wrong.
            debug!("diagnostic selection stopped: repeated error identity in source chain");
            break;
        }
        if visited.len() >= MAX_SELECTION_DEPTH {
            debug!(
                depth = MAX_SELECTION_DEPTH,
                "diagnostic selection stopped: source chain exceeded maximum depth"
            );
            break;
        }
        visited.push(identity);

        current = match discover(err) {
            Some(diagnostic) => {
                if matches!(diagnostic.role(), DiagnosticRole::Semantic) {
                    return Some(EffectiveDiagnostic::Claudine(diagnostic));
                }
                best = Some(EffectiveDiagnostic::Claudine(diagnostic));
                diagnostic.diagnostic_source()
            }
            None => {
                if let Some(block) = as_block_error(err) {
                    best = Some(EffectiveDiagnostic::LowerLayer(block));
                }
                err.source()
            }
        };
    }

    best
}

#[cfg(test)]
mod tests;
