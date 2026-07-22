//! Table-driven tests for S1 parse-equivalent whitespace cleanup
//! (acceptance A-2): whitespace around flow delimiters and commas, mapping
//! colons, and sequence markers. Every accepted candidate must reparse to a
//! value exactly equal to the original's; `host:localhost` proves the gate
//! rejects shape-changing edits.

use super::super::{YamlDiagnosticCode, analyze_yaml};
use super::assert_untouched_bytes_preserved;

/// Asserts the repaired output and value equality between original and
/// patched parses.
fn assert_whitespace_cleanup(source: &str, expected_output: &str) {
    let analysis = analyze_yaml(source);
    assert!(analysis.is_parseable(), "input must parse: {source:?}");
    let outcome = analysis.apply();
    assert_eq!(outcome.source, expected_output, "for {source:?}");
    assert_untouched_bytes_preserved(source, &outcome);
    let original = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source).unwrap();
    let patched = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&outcome.source).unwrap();
    assert_eq!(original, patched, "value must not change for {source:?}");
}

fn whitespace_diagnostics(source: &str) -> usize {
    analyze_yaml(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::Whitespace)
        .count()
}

#[test]
fn test_flow_sequence_whitespace() {
    assert_whitespace_cleanup("key: [ 80,443 ]", "key: [80, 443]\n");
}

#[test]
fn test_spaces_before_mapping_colon() {
    assert_whitespace_cleanup("key :  value", "key: value\n");
}

#[test]
fn test_spaces_after_sequence_marker() {
    assert_whitespace_cleanup("-  item", "- item\n");
}

#[test]
fn test_flow_mapping_whitespace() {
    assert_whitespace_cleanup("{ a:  1 , b: 2 }\n", "{a: 1, b: 2}\n");
}

#[test]
fn test_json_style_quoted_key_colon() {
    assert_whitespace_cleanup("{\"a\":1}\n", "{\"a\": 1}\n");
}

#[test]
fn test_nested_flow_collections() {
    assert_whitespace_cleanup("[ [ 1 ], {a:  2} ]\n", "[[1], {a: 2}]\n");
}

#[test]
fn test_quoted_strings_in_flow_preserved() {
    assert_whitespace_cleanup("[\"a b\" , \"c\" ]\n", "[\"a b\", \"c\"]\n");
}

#[test]
fn test_url_in_flow_not_mistaken_for_mapping_colon() {
    assert_whitespace_cleanup("[http://x,  y]\n", "[http://x, y]\n");
}

#[test]
fn test_host_localhost_untouched() {
    // `host:localhost` is a plain scalar, not a mapping: inserting a space
    // would change the shape, so no whitespace candidate may be offered.
    let source = "host:localhost\n";
    assert_eq!(whitespace_diagnostics(source), 0);
    let outcome = analyze_yaml(source).apply();
    assert_eq!(outcome.source, source);
}

#[test]
fn test_missing_space_after_colon_in_block_untouched() {
    // Same class: `key:value` is one scalar; the colon is not a mapping
    // colon, so nothing is a candidate.
    let source = "key:value\n";
    assert_eq!(whitespace_diagnostics(source), 0);
    let outcome = analyze_yaml(source).apply();
    assert_eq!(outcome.source, source);
}

#[test]
fn test_negative_number_not_a_sequence_marker() {
    let source = "ports:\n  -80\n  -443\n";
    assert_eq!(whitespace_diagnostics(source), 0);
    let outcome = analyze_yaml(source).apply();
    assert_eq!(outcome.source, source);
}

#[test]
fn test_already_canonical_flow_is_clean() {
    let source = "key: [80, 443]\n";
    assert_eq!(whitespace_diagnostics(source), 0);
}

#[test]
fn test_comment_inside_flow_preserved() {
    // The whitespace after the comma is followed by a comment; only the
    // missing-final-newline diagnostic may fire.
    let source = "key: [1, # c\n 2]\n";
    assert_eq!(whitespace_diagnostics(source), 0);
    let outcome = analyze_yaml(source).apply();
    assert_eq!(outcome.source, source);
}

#[test]
fn test_multiline_flow_whitespace_runs_left_alone() {
    // Runs containing line breaks are outside the whitespace candidate
    // grammar; the line-ending and trailing-whitespace normalizers own those.
    let source = "key: [1,\n 2]\n";
    assert_eq!(whitespace_diagnostics(source), 0);
    let outcome = analyze_yaml(source).apply();
    assert_eq!(outcome.source, source);
}

#[test]
fn test_sequence_entry_with_inline_mapping() {
    assert_whitespace_cleanup("- key :  value\n", "- key: value\n");
}

#[test]
fn test_tab_after_mapping_colon() {
    assert_whitespace_cleanup("key:\tvalue\n", "key: value\n");
}

#[test]
fn test_multiple_mapping_colon_sites() {
    assert_whitespace_cleanup("a : 1\nb : 2\n", "a: 1\nb: 2\n");
}

#[test]
fn test_flow_mapping_colon_spacing() {
    assert_whitespace_cleanup("key: {a : 1}\n", "key: {a: 1}\n");
}

#[test]
fn test_whitespace_diagnostics_are_deterministic_and_sorted() {
    let source = "key :  [ 80,443 ]";
    let analysis = analyze_yaml(source);
    let spans: Vec<_> = analysis
        .diagnostics()
        .iter()
        .map(|diagnostic| (diagnostic.span.start, diagnostic.span.end))
        .collect();
    let mut sorted = spans.clone();
    sorted.sort_unstable();
    assert_eq!(spans, sorted, "diagnostics must be in stable source order");
}
