//! Compile-time / public-API tests pinning the shared `SourceSpan`
//! vocabulary (acceptance C-7).
//!
//! `biscuit_file::SourceSpan` is the single byte-offset span type shared with
//! Darkmatter, which re-exports it at `darkmatter::SourceSpan` and
//! `darkmatter::markdown::span::SourceSpan`. The cross-crate half of this
//! contract lives in `darkmatter/lib/tests/span_compat.rs`; a cyclic
//! dev-dependency would be needed to assert it from here.

use std::ops::Range;

use biscuit_file::SourceSpan;

/// Compile-time proof that the crate-root export path resolves and that the
/// type is literally `Range<usize>` — any change to a distinct/newtype span
/// fails to compile here and in the Darkmatter mirror test.
fn assert_range_identity(span: SourceSpan) -> Range<usize> {
    span
}

#[test]
fn test_source_span_is_range_of_usize() {
    let span: SourceSpan = 3..9;
    let range: Range<usize> = assert_range_identity(span);
    assert_eq!(range, 3..9);
}

#[test]
fn test_source_span_construction_and_access() {
    // A Range<usize> constructed anywhere converts freely both ways.
    let span: SourceSpan = Range { start: 11, end: 24 };
    assert_eq!(span.start, 11);
    assert_eq!(span.end, 24);
    assert_eq!(span.len(), 13);
    assert!(!span.is_empty());
}

#[test]
fn test_source_span_slices_multibyte_source() {
    // Spans are byte offsets: they slice `str` directly when aligned.
    let source = "title: 日本語 daily";
    let span: SourceSpan = 7..16;
    assert_eq!(&source[span], "日本語");
}

#[test]
fn test_source_span_zero_length_and_bounds() {
    let span: SourceSpan = 0..0;
    assert!(span.is_empty());
    let source = "abc";
    assert_eq!(&source[span], "");
}

#[test]
fn test_public_diagnostic_types_reachable_from_crate_root() {
    // Phase 2 checkpoint: every new public type is reachable from
    // `biscuit_file::` directly.
    fn assert_paths()
    where
        biscuit_file::YamlDiagnostic: Sized,
        biscuit_file::YamlRepair: Sized,
        biscuit_file::YamlCertainty: Sized,
        biscuit_file::YamlDiagnosticCode: Sized,
        biscuit_file::YamlAnalysis: Sized,
        biscuit_file::YamlParseOutcome: Sized,
        biscuit_file::YamlParseFailure: Sized,
        biscuit_file::YamlLocation: Sized,
        biscuit_file::EditAudit: Sized,
        biscuit_file::EditRejection: Sized,
        biscuit_file::EditSetOutcome: Sized,
        biscuit_file::RejectedEdit: Sized,
    {
    }
    assert_paths();
    let _apply: fn(&str, &[biscuit_file::YamlRepair]) -> biscuit_file::EditSetOutcome =
        biscuit_file::apply_edit_set;
}

#[test]
fn test_crate_root_span_matches_yaml_diagnostic_field_type() {
    // The diagnostic structs must carry the shared span type, not a private
    // lookalike: a root-path span assigns into a repair field directly.
    let span: SourceSpan = 1..4;
    let repair = biscuit_file::YamlRepair {
        span,
        replacement: "x".to_string(),
        explanation: "y".to_string(),
    };
    let diagnostic = biscuit_file::YamlDiagnostic {
        code: biscuit_file::YamlDiagnosticCode::Parse,
        span: repair.span.clone(),
        classification: biscuit_file::YamlCertainty::Deterministic,
        message: "m".to_string(),
        repairs: vec![repair],
    };
    assert_eq!(diagnostic.span, 1..4);
}
