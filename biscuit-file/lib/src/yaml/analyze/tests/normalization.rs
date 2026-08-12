//! Table-driven tests for S1 source normalization (acceptance A-1, E-4,
//! E-6): BOM, CRLF/CR → LF, trailing whitespace outside scalar content, and
//! final-newline handling. Every accepted candidate must be parse-equivalent
//! (the patched source reparses to the original's `serde_yaml_ng::Value`).

use super::super::{YamlDiagnosticCode, analyze_yaml};
use super::assert_untouched_bytes_preserved;

/// Asserts the single diagnostic of `code` with `span`, repaired output, and
/// value equality between original and patched parses.
fn assert_normalization(
    source: &str,
    code: YamlDiagnosticCode,
    expected_diags: usize,
    expected_output: &str,
) {
    let analysis = analyze_yaml(source);
    assert!(analysis.is_parseable(), "input must parse: {source:?}");
    let matching: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .collect();
    assert_eq!(
        matching.len(),
        expected_diags,
        "expected {expected_diags} {code} diagnostics for {source:?}, got {:?}",
        analysis.diagnostics()
    );
    let outcome = analysis.apply();
    assert_eq!(outcome.source, expected_output, "for {source:?}");
    assert_untouched_bytes_preserved(source, &outcome);
    // Parse-equivalence: original and patched values must be identical.
    let original = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source).unwrap();
    let patched = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&outcome.source).unwrap();
    assert_eq!(original, patched, "value must not change for {source:?}");
}

#[test]
fn test_bom_at_stream_start_removed() {
    let source = "\u{FEFF}key: value\n";
    let analysis = analyze_yaml(source);
    let bom = analysis
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == YamlDiagnosticCode::Bom)
        .expect("BOM diagnostic");
    assert_eq!(bom.span, 0..3);
    assert_eq!(bom.repairs[0].span, 0..3);
    assert_eq!(bom.repairs[0].replacement, "");
    assert_normalization(source, YamlDiagnosticCode::Bom, 1, "key: value\n");
}

#[test]
fn test_crlf_normalized_to_lf() {
    assert_normalization(
        "a: 1\r\nb: 2\r\n",
        YamlDiagnosticCode::LineEnding,
        2,
        "a: 1\nb: 2\n",
    );
}

#[test]
fn test_lone_cr_normalized_to_lf() {
    assert_normalization(
        "a: 1\rb: 2\r",
        YamlDiagnosticCode::LineEnding,
        2,
        "a: 1\nb: 2\n",
    );
}

#[test]
fn test_mixed_line_endings_normalized() {
    assert_normalization(
        "a: 1\r\nb: 2\rc: 3\n",
        YamlDiagnosticCode::LineEnding,
        2,
        "a: 1\nb: 2\nc: 3\n",
    );
}

#[test]
fn test_crlf_spans_are_byte_accurate() {
    let analysis = analyze_yaml("a: 1\r\nb: 2\r\n");
    let spans: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::LineEnding)
        .map(|diagnostic| diagnostic.span.clone())
        .collect();
    assert_eq!(spans, vec![4..6, 10..12]);
}

#[test]
fn test_trailing_spaces_removed() {
    assert_normalization(
        "key: value  \n",
        YamlDiagnosticCode::TrailingWhitespace,
        1,
        "key: value\n",
    );
}

#[test]
fn test_trailing_tab_removed() {
    assert_normalization(
        "key: value\t\n",
        YamlDiagnosticCode::TrailingWhitespace,
        1,
        "key: value\n",
    );
}

#[test]
fn test_blank_line_whitespace_removed() {
    assert_normalization(
        "a: 1\n  \nb: 2\n",
        YamlDiagnosticCode::TrailingWhitespace,
        1,
        "a: 1\n\nb: 2\n",
    );
}

#[test]
fn test_trailing_whitespace_after_quoted_scalar_removed_but_content_kept() {
    // The two spaces inside the quotes are scalar content; only the trailing
    // pair outside the quotes is a candidate.
    assert_normalization(
        "key: \"va  lue\"  \n",
        YamlDiagnosticCode::TrailingWhitespace,
        1,
        "key: \"va  lue\"\n",
    );
}

#[test]
fn test_trailing_whitespace_inside_block_scalar_untouched() {
    // Trailing spaces inside literal block content are significant.
    let source = "script: |\n  echo hi  \n  echo bye\n";
    let analysis = analyze_yaml(source);
    assert!(
        analysis
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code != YamlDiagnosticCode::TrailingWhitespace),
        "no trailing-whitespace candidate inside block content: {:?}",
        analysis.diagnostics()
    );
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source);
}

#[test]
fn test_trailing_whitespace_inside_multiline_quoted_scalar_untouched() {
    let source = "multi: \"line1  \n  line2\"\n";
    let analysis = analyze_yaml(source);
    assert!(
        analysis
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code != YamlDiagnosticCode::TrailingWhitespace),
        "no trailing-whitespace candidate inside quoted scalar: {:?}",
        analysis.diagnostics()
    );
}

#[test]
fn test_missing_final_newline_added() {
    let source = "key: value";
    let analysis = analyze_yaml(source);
    let newline = analysis
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == YamlDiagnosticCode::FinalNewline)
        .expect("final-newline diagnostic");
    assert_eq!(newline.span, 10..10);
    assert_eq!(newline.repairs[0].replacement, "\n");
    assert_normalization(source, YamlDiagnosticCode::FinalNewline, 1, "key: value\n");
}

#[test]
fn test_present_final_newline_is_clean() {
    let analysis = analyze_yaml("key: value\n");
    assert!(analysis.is_clean(), "{:?}", analysis.diagnostics());
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "key: value\n");
    assert!(!outcome.changed());
}

#[test]
fn test_superfluous_trailing_blank_lines_collapsed() {
    assert_normalization(
        "key: value\n\n\n",
        YamlDiagnosticCode::FinalNewline,
        1,
        "key: value\n",
    );
}

#[test]
fn test_trailing_whitespace_and_missing_final_newline_combine() {
    let source = "key: value  ";
    let analysis = analyze_yaml(source);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "key: value\n");
    assert_untouched_bytes_preserved(source, &outcome);
    let original = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source).unwrap();
    let patched = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&outcome.source).unwrap();
    assert_eq!(original, patched);
}

#[test]
fn test_bom_crlf_and_trailing_whitespace_combine() {
    let source = "\u{FEFF}key: value  \r\n";
    let analysis = analyze_yaml(source);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "key: value\n");
    assert_untouched_bytes_preserved(source, &outcome);
    let original = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source).unwrap();
    let patched = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&outcome.source).unwrap();
    assert_eq!(original, patched);
}

#[test]
fn test_empty_source_is_clean() {
    let analysis = analyze_yaml("");
    assert!(analysis.is_clean());
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "");
}

#[test]
fn test_already_clean_document_has_no_candidates() {
    let analysis = analyze_yaml("a: 1\nb: [1, 2]\n");
    assert!(analysis.is_clean(), "{:?}", analysis.diagnostics());
}
