//! Cross-platform line-ending tests (acceptance E-2, E-3): span accuracy
//! with CRLF, lone-CR normalization, CRLF inside block scalars and flow
//! collections, and CRLF byte preservation on the parse-recovery path.

use super::super::{YamlDiagnosticCode, analyze_yaml};
use super::assert_untouched_bytes_preserved;

#[test]
fn test_crlf_normalized() {
    let analysis = analyze_yaml("a: 1\r\nb: 2\r\n");
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "a: 1\nb: 2\n");
    assert_untouched_bytes_preserved("a: 1\r\nb: 2\r\n", &outcome);
}

#[test]
fn test_lone_cr_normalized() {
    let analysis = analyze_yaml("a: 1\rb: 2\r");
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "a: 1\nb: 2\n");
    assert_untouched_bytes_preserved("a: 1\rb: 2\r", &outcome);
}

#[test]
fn test_crlf_spans_account_for_carriage_return() {
    // Spans are byte offsets: the second line starts one byte later per CRLF.
    let analysis = analyze_yaml("ab: 1\r\ncd: 2\r\n");
    let spans: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::LineEnding)
        .map(|diagnostic| diagnostic.span.clone())
        .collect();
    assert_eq!(spans, vec![5..7, 12..14]);
}

#[test]
fn test_crlf_inside_block_scalar_normalized_value_equal() {
    let source = "script: |\r\n  line1\r\n  line2\r\n";
    let analysis = analyze_yaml(source);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "script: |\n  line1\n  line2\n");
    assert_untouched_bytes_preserved(source, &outcome);
    let original = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source).unwrap();
    let patched = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&outcome.source).unwrap();
    assert_eq!(original, patched);
}

#[test]
fn test_crlf_inside_flow_collection() {
    let source = "key: [1,\r\n 2]";
    let analysis = analyze_yaml(source);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "key: [1,\n 2]\n");
    assert_untouched_bytes_preserved(source, &outcome);
}

#[test]
fn test_crlf_preserved_on_parse_recovery_path() {
    // Unparseable input takes the S3 path: line-ending normalization is not
    // applicable there, so the CRLF outside the repaired lexeme is preserved
    // byte-for-byte.
    let source = "title: @x\r\n";
    let analysis = analyze_yaml(source);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "title: \"@x\"\r\n");
    assert_untouched_bytes_preserved(source, &outcome);
}

#[test]
fn test_crlf_multibyte_offsets() {
    let source = "key: 日本語\r\nother: 1\r\n";
    let analysis = analyze_yaml(source);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "key: 日本語\nother: 1\n");
    assert_untouched_bytes_preserved(source, &outcome);
}
