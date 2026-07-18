//! Compile-time / public-API tests pinning Darkmatter's `SourceSpan` import
//! paths to the shared `biscuit-file` span vocabulary (acceptance C-7).
//!
//! `darkmatter::SourceSpan` and `darkmatter::markdown::span::SourceSpan` must
//! keep resolving for existing consumers, and both must be literally the same
//! type as `biscuit_file::SourceSpan` so the two crates can never drift
//! apart. Any divergence fails to compile this file.

use std::ops::Range;

use biscuit_file::SourceSpan as BiscuitSpan;
use darkmatter::SourceSpan as RootSpan;
use darkmatter::markdown::span::SourceSpan as ModuleSpan;

/// Identity function requiring the crate-root and module paths to name the
/// biscuit-file type.
fn assert_shared_type(span: BiscuitSpan) -> RootSpan {
    let module: ModuleSpan = span;
    module
}

#[test]
fn test_root_and_module_paths_are_the_biscuit_file_type() {
    let span: BiscuitSpan = 4..17;
    let root: RootSpan = assert_shared_type(span);
    assert_eq!(root, 4..17);
}

#[test]
fn test_spans_are_plain_ranges_in_both_crates() {
    // A Range<usize> built here is assignable through both crates' paths —
    // the vocabulary is a transparent alias, not a lookalike newtype.
    let range: Range<usize> = Range { start: 2, end: 8 };
    let biscuit: BiscuitSpan = range.clone();
    let darkmatter: RootSpan = range;
    assert_eq!(biscuit, darkmatter);
}

#[test]
fn test_spanned_struct_uses_the_shared_span() {
    // `Spanned<T>` fields accept a biscuit-file span without conversion.
    let span: BiscuitSpan = 1..6;
    let spanned = darkmatter::Spanned::new("value", span);
    let biscuit_span: BiscuitSpan = spanned.span;
    assert_eq!(biscuit_span, 1..6);
}

#[test]
fn test_span_helpers_accept_shared_spans() {
    // Darkmatter's line/column helpers operate on offsets drawn from the
    // shared span type with no impedance mismatch.
    let source = "first\nsecond\n";
    let span: BiscuitSpan = 6..12;
    let (line, column) = darkmatter::line_col_of_offset(source, span.start);
    assert_eq!((line, column), (2, 1));
    assert_eq!(darkmatter::line_of_offset(source, span.end), 2);
    assert_eq!(&source[span], "second");
}
