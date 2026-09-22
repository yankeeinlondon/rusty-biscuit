//! Cross-crate reachability of Claudine's diagnostic discovery seam.
//!
//! `claudine-cli` is a separate crate, so the seam the error walker renders
//! through cannot be `pub(crate)` in the library. Compiling this file *is* the
//! assertion: `as_diagnostic` and `select_effective_diagnostic` must be callable
//! from outside `claudine` for the walker's D2 rewrite to be possible at all.
//!
//! See `claudine/features/2026-07-13-error-propogation/spec.md` §D2.

use std::error::Error as StdError;

use claudine::composition::CompositionError;
use claudine::diagnostics::{DiagnosticRole, as_diagnostic, select_effective_diagnostic};

#[test]
fn as_diagnostic_is_callable_from_the_cli_crate() {
    let error = CompositionError::FileNotFound("missing.md".to_string());
    let erased: &(dyn StdError + 'static) = &error;

    let diagnostic = as_diagnostic(erased).expect("CompositionError is a registered diagnostic");

    assert_eq!(diagnostic.code(), "composition.invalid_file_reference");
    assert_eq!(diagnostic.role(), DiagnosticRole::Semantic);
}

#[test]
fn selection_is_callable_from_the_cli_crate_and_renders() {
    // The shape the walker will use in Phase 5: erase, select, render — with no
    // concrete-type list on the CLI side.
    let error = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(CompositionError::FileNotFound("missing.md".to_string())),
    };
    let erased: &(dyn StdError + 'static) = &error;

    let selected = select_effective_diagnostic(erased).expect("a diagnostic is selected");
    let rendered = selected
        .block_error()
        .report_block_error_optimistic(Some(120));

    assert!(rendered.contains("missing.md"), "rendered: {rendered}");
    assert_eq!(
        selected.diagnostic().map(|d| d.code()),
        Some("composition.invalid_file_reference")
    );
}
