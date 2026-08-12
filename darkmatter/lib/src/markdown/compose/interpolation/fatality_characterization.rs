//! Characterization matrix for compose-time expression-failure fatality.
//!
//! The "real errors" effort
//! (`claudine/features/2026-06-28-real-errors/plan.md`) replaced the
//! string-prefix gate [`is_fatal_eval_error`] with a checked `match` over typed
//! causes ([`is_authoring_fatal`]). These tests pin the
//! disposition for every (failure kind × compose mode × interpolation surface)
//! cell so a later change that silently flips a cell fails loudly.
//!
//! ## The ratified policy this matrix locks
//!
//! - **Body interpolation** ([`interpolate_text`]): under `fail_fast = true`
//!   every failure is fatal. In lenient (`fail_fast = false`) mode, **three**
//!   classes are fatal — an **unknown function**, a **present file reference
//!   that fails to resolve** (missing file or malformed path), and a **focused
//!   provider failure** ([`ExpressionError::Provider`]). A present file
//!   reference that does not resolve is treated as a real authoring mistake
//!   rather than a tolerated absence (real-errors finding #1, ratified), and a
//!   provider failure is actionable state the spec forbids demoting to a
//!   warning. The provider class is exercised end-to-end in
//!   `compose/tests/provider_network.rs` (it needs a repository fixture and a
//!   mock transport, which this harness deliberately does not build); the six
//!   cells below pin the purely local failure kinds.
//!   The remaining failures (arity, arg-type, parse) are demoted to a
//!   `ComposeWarning` with the original `{{ … }}` left in place.
//! - **Frontmatter whole-value span** ([`interpolate_value`]): a value whose
//!   trimmed content is exactly one `{{ expr }}` is executable state, so *every*
//!   parse/eval failure is fatal regardless of `fail_fast`.
//!
//! The file-reference fatality promotion is the one **intentional** behavior
//! change in this effort; it is characterized here and tested separately from
//! the behavior-neutral typing refactor.
//!
//! [`is_fatal_eval_error`]: super::rewrite
//! [`is_authoring_fatal`]: crate::markdown::compose::expression::ExpressionError::is_authoring_fatal
//! [`interpolate_text`]: super::rewrite::interpolate_text
//! [`interpolate_value`]: super::rewrite::interpolate_value

use super::rewrite::{ScanMode, interpolate_text, interpolate_value};
use super::Evaluator;
use crate::markdown::compose::expression::ResolutionContext;
use crate::markdown::compose::{
    ComposeContext, EffectiveState, EffectiveStateBuilder, ResolvingLookup,
};
use std::collections::HashMap;

/// The two interpolation surfaces the matrix distinguishes.
///
/// They map to the two production entry points, not merely to "where the text
/// lives": body interpolation routes through [`interpolate_text`], while a
/// frontmatter value that is a single `{{ expr }}` span routes through the
/// strict whole-value path in [`interpolate_value`].
///
/// [`interpolate_text`]: super::rewrite::interpolate_text
/// [`interpolate_value`]: super::rewrite::interpolate_value
#[derive(Clone, Copy, Debug)]
enum Surface {
    Body,
    FrontmatterWholeValue,
}

/// The observed disposition of a single failing interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    /// Composition aborted: the surface returned `Err`.
    Fatal,
    /// Composition continued: `Ok`, with at least one `ComposeWarning` and the
    /// failing `{{ … }}` left unresolved.
    Warning,
    /// `Ok` with no warnings — the failure was silently swallowed. Never
    /// expected for any cell; modeled so a drift into silent success is caught.
    Clean,
}

/// One expression-failure fixture.
///
/// `input` is the same `{{ expr }}` text for both surfaces — the difference is
/// purely which entry point evaluates it. `fragment` is a stable substring of
/// the surfaced message proving *which* failure fired, so a drift that keeps the
/// fatal/warn verdict but changes the underlying cause is still caught.
struct Case {
    kind: &'static str,
    input: &'static str,
    fragment: &'static str,
}

/// The six failure kinds from the integrated-design §5 matrix.
///
/// `missing-file` and `malformed-path` share the `invalid file path` fragment
/// and the same (now fatal) lenient-body disposition, but are listed separately
/// because they map to distinct typed causes
/// (`FileRefFailure::NotFound` vs the malformed/`Malformed` case).
const CASES: &[Case] = &[
    Case {
        kind: "unknown-function",
        input: "{{ unknown_fn(\"x\") }}",
        fragment: "Unknown function",
    },
    Case {
        kind: "missing-file",
        input: "{{ frontmatter(\"does-not-exist.md\", \"x\") }}",
        fragment: "invalid file path",
    },
    Case {
        // An empty reference string is rejected by `FileReference::new` itself
        // (the reference is malformed, distinct from a well-formed-but-absent
        // file).
        kind: "malformed-path",
        input: "{{ absolute(\"\") }}",
        fragment: "invalid file path",
    },
    Case {
        kind: "arity",
        input: "{{ min(1) }}",
        fragment: "requires",
    },
    Case {
        kind: "arg-type",
        input: "{{ min(\"a\", \"b\") }}",
        fragment: "numeric",
    },
    Case {
        kind: "parse",
        input: "{{ > invalid }}",
        fragment: "parse",
    },
];

fn empty_state() -> EffectiveState {
    EffectiveStateBuilder::new()
        .with_frontmatter(HashMap::new())
        .with_context(ComposeContext::fixed_for_testing())
        .build()
        .unwrap()
}

/// Evaluates `input` on `surface` under `fail_fast`, returning the observed
/// outcome and the surfaced message (the `Err` text, or the joined warning
/// messages).
///
/// A tempdir base directory is supplied through [`ResolvingLookup`] so the
/// filesystem builtins (`frontmatter`, `absolute`) actually run rather than
/// short-circuiting to the "no resolution context" recoverable error. The
/// directory is intentionally empty: `does-not-exist.md` must miss.
fn classify(input: &str, surface: Surface, fail_fast: bool) -> (Outcome, String) {
    let dir = tempfile::TempDir::new().unwrap();
    let state = empty_state();
    let ctx = ResolutionContext::new(dir.path().to_path_buf());
    let lookup = ResolvingLookup::new(&state, ctx);
    let evaluator = Evaluator::new(&lookup);

    match surface {
        Surface::Body => {
            match interpolate_text(
                input,
                &evaluator,
                ScanMode::MarkdownAware,
                fail_fast,
                "characterization",
            ) {
                Ok(rewrite) => {
                    let msg = join_warnings(&rewrite.warnings);
                    let outcome = if rewrite.warnings.is_empty() {
                        Outcome::Clean
                    } else {
                        Outcome::Warning
                    };
                    (outcome, msg)
                }
                Err(e) => (Outcome::Fatal, e.to_string()),
            }
        }
        Surface::FrontmatterWholeValue => {
            match interpolate_value(input, &evaluator, fail_fast, "characterization") {
                Ok((_value, _replacements, warnings)) => {
                    let msg = join_warnings(&warnings);
                    let outcome = if warnings.is_empty() {
                        Outcome::Clean
                    } else {
                        Outcome::Warning
                    };
                    (outcome, msg)
                }
                Err(e) => (Outcome::Fatal, e.to_string()),
            }
        }
    }
}

fn join_warnings(warnings: &[crate::markdown::compose::ComposeWarning]) -> String {
    warnings
        .iter()
        .map(|w| w.message.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The ratified expected disposition for a cell.
///
/// This function *is* the locked matrix — keep it the single source of truth so
/// a refactor that changes a verdict has exactly one place to (deliberately)
/// update, with the test names below documenting the headline cases.
fn expected(kind: &str, surface: Surface, fail_fast: bool) -> Outcome {
    match surface {
        // A whole-value frontmatter span is executable state: every failure is
        // fatal, regardless of fail_fast.
        Surface::FrontmatterWholeValue => Outcome::Fatal,
        Surface::Body => {
            if fail_fast {
                Outcome::Fatal
            } else {
                // Ratified (real-errors finding #1): a present-but-unresolvable
                // file reference is now fatal in lenient body mode too, alongside
                // the always-fatal unknown function. Arity/arg-type/parse remain
                // warnings — they are not file references.
                match kind {
                    "unknown-function" | "missing-file" | "malformed-path" => Outcome::Fatal,
                    _ => Outcome::Warning,
                }
            }
        }
    }
}

#[test]
fn fatality_matrix_is_locked() {
    for case in CASES {
        for surface in [Surface::Body, Surface::FrontmatterWholeValue] {
            for fail_fast in [false, true] {
                let (outcome, message) = classify(case.input, surface, fail_fast);
                let want = expected(case.kind, surface, fail_fast);
                assert_eq!(
                    outcome, want,
                    "fatality drift: kind={} surface={:?} fail_fast={} \
                     expected {:?} but observed {:?}; message: {}",
                    case.kind, surface, fail_fast, want, outcome, message
                );
                // A fatal or warning outcome must surface the cause that fired,
                // so a verdict-preserving cause swap is still caught.
                if outcome != Outcome::Clean {
                    assert!(
                        message.contains(case.fragment),
                        "message-content drift: kind={} surface={:?} fail_fast={} \
                         expected fragment {:?} in message: {}",
                        case.kind,
                        surface,
                        fail_fast,
                        case.fragment,
                        message
                    );
                }
            }
        }
    }
}

// ── Headline cells, named so a drift names the semantic case ──────────────────
//
// These duplicate individual `fatality_matrix_is_locked` cells on purpose: the
// test name is the record of the expected outcome, so a failure points straight
// at the drifted contract rather than at a generic matrix loop.

#[test]
fn body_unknown_function_is_fatal_in_lenient_mode() {
    let (outcome, message) = classify("{{ unknown_fn(\"x\") }}", Surface::Body, false);
    assert_eq!(outcome, Outcome::Fatal);
    assert!(message.contains("Unknown function"), "message: {message}");
}

#[test]
fn body_missing_file_is_fatal_in_lenient_mode() {
    // Ratified (real-errors finding #1): a present file reference that misses is
    // fatal even in lenient body mode — it was a warning before this change.
    let (outcome, message) = classify(
        "{{ frontmatter(\"does-not-exist.md\", \"x\") }}",
        Surface::Body,
        false,
    );
    assert_eq!(outcome, Outcome::Fatal);
    assert!(message.contains("invalid file path"), "message: {message}");
}

#[test]
fn body_malformed_path_is_fatal_in_lenient_mode() {
    // Ratified (real-errors finding #1): a malformed (present) reference is fatal
    // even in lenient body mode — it was a warning before this change.
    let (outcome, message) = classify("{{ absolute(\"\") }}", Surface::Body, false);
    assert_eq!(outcome, Outcome::Fatal);
    assert!(message.contains("invalid file path"), "message: {message}");
}

#[test]
fn body_arity_is_warning_in_lenient_mode() {
    let (outcome, message) = classify("{{ min(1) }}", Surface::Body, false);
    assert_eq!(outcome, Outcome::Warning);
    assert!(message.contains("requires"), "message: {message}");
}

#[test]
fn body_arg_type_is_warning_in_lenient_mode() {
    let (outcome, message) = classify("{{ min(\"a\", \"b\") }}", Surface::Body, false);
    assert_eq!(outcome, Outcome::Warning);
    assert!(message.contains("numeric"), "message: {message}");
}

#[test]
fn body_parse_failure_is_warning_in_lenient_mode() {
    let (outcome, message) = classify("{{ > invalid }}", Surface::Body, false);
    assert_eq!(outcome, Outcome::Warning);
    assert!(message.contains("parse"), "message: {message}");
}

#[test]
fn body_every_failure_is_fatal_in_fail_fast_mode() {
    for case in CASES {
        let (outcome, _message) = classify(case.input, Surface::Body, true);
        assert_eq!(
            outcome,
            Outcome::Fatal,
            "kind {} should be fatal under fail_fast",
            case.kind
        );
    }
}

#[test]
fn frontmatter_whole_value_missing_file_is_fatal_in_lenient_mode() {
    // The most important contrast with body interpolation: the same missing-file
    // failure that merely warns in body text aborts in a whole-value frontmatter
    // span even when fail_fast is off.
    let (outcome, message) = classify(
        "{{ frontmatter(\"does-not-exist.md\", \"x\") }}",
        Surface::FrontmatterWholeValue,
        false,
    );
    assert_eq!(outcome, Outcome::Fatal);
    assert!(message.contains("invalid file path"), "message: {message}");
}

#[test]
fn frontmatter_whole_value_every_failure_is_fatal_in_lenient_mode() {
    for case in CASES {
        let (outcome, _message) =
            classify(case.input, Surface::FrontmatterWholeValue, false);
        assert_eq!(
            outcome,
            Outcome::Fatal,
            "kind {} should be fatal on a whole-value frontmatter span even in lenient mode",
            case.kind
        );
    }
}
