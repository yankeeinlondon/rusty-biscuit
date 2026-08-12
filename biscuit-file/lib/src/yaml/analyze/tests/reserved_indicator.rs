//! Tests for S3 reserved-indicator parse-recovery quoting (acceptance A-3,
//! decision D3): the flagship no-schema case plus every refusal boundary.
//! Auto-apply requires the dedicated parse-recovery proof — candidate parses,
//! lexeme byte-equality at the lexically scanned context path — and anything
//! ambiguous stays report-only with the source byte-identical.

use super::super::{YamlCertainty, YamlDiagnosticCode, analyze_yaml};
use super::assert_untouched_bytes_preserved;

/// Asserts the analysis produces exactly one deterministic reserved-indicator
/// diagnostic whose repair quotes `lexeme`, and that applying it yields
/// `expected_output` with the parsed node byte-equal to the original lexeme.
fn assert_quoting(source: &str, lexeme: &str, expected_output: &str) {
    let analysis = analyze_yaml(source);
    assert!(!analysis.is_parseable(), "input must fail to parse: {source:?}");
    let deterministic: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.classification == YamlCertainty::Deterministic)
        .collect();
    assert_eq!(
        deterministic.len(),
        1,
        "expected exactly one deterministic diagnostic for {source:?}, got {:?}",
        analysis.diagnostics()
    );
    let diagnostic = deterministic[0];
    assert_eq!(diagnostic.code, YamlDiagnosticCode::ReservedIndicator);
    assert_eq!(diagnostic.repairs.len(), 1);
    assert_eq!(
        diagnostic.repairs[0].replacement,
        format!("\"{lexeme}\""),
        "repair must quote the exact authored lexeme"
    );
    assert_eq!(
        &source[diagnostic.span.clone()],
        lexeme,
        "diagnostic span must cover the authored lexeme"
    );

    let outcome = analysis.apply();
    assert_eq!(outcome.source, expected_output);
    assert_untouched_bytes_preserved(source, &outcome);

    // Downstream state: the repaired document parses and holds the lexeme as
    // a string at a matching position.
    let repaired = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&outcome.source)
        .expect("repaired document must parse");
    let found = contains_string(&repaired, lexeme);
    assert!(found, "repaired value must contain {lexeme:?} as a string: {repaired:?}");
}

fn contains_string(value: &serde_yaml_ng::Value, expected: &str) -> bool {
    use serde_yaml_ng::Value;
    match value {
        Value::String(text) => text == expected,
        Value::Sequence(sequence) => sequence.iter().any(|item| contains_string(item, expected)),
        Value::Mapping(mapping) => mapping.values().any(|item| contains_string(item, expected)),
        _ => false,
    }
}

/// Asserts the analysis never offers an automatic repair: no deterministic
/// diagnostics, no repairs anywhere, and byte-identical output.
fn assert_report_only(source: &str) {
    let analysis = analyze_yaml(source);
    for diagnostic in analysis.diagnostics() {
        assert_ne!(
            diagnostic.classification,
            YamlCertainty::Deterministic,
            "no deterministic diagnostic expected for {source:?}: {diagnostic:?}"
        );
        assert!(
            diagnostic.repairs.is_empty(),
            "report-only findings carry no repairs for {source:?}: {diagnostic:?}"
        );
    }
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source, "source must stay byte-identical");
    assert!(!outcome.changed());
}

#[test]
fn test_flagship_reserved_indicator() {
    // The original failing input from the feature report.
    assert_quoting(
        "title: @daily-report",
        "@daily-report",
        "title: \"@daily-report\"",
    );
}

#[test]
fn test_flagship_with_final_newline() {
    assert_quoting(
        "title: @daily-report\n",
        "@daily-report",
        "title: \"@daily-report\"\n",
    );
}

#[test]
fn test_backtick_indicator() {
    assert_quoting("title: `daily-report`", "`daily-report`", "title: \"`daily-report`\"");
}

#[test]
fn test_percent_indicator() {
    assert_quoting("title: %directive", "%directive", "title: \"%directive\"");
}

#[test]
fn test_ampersand_indicator() {
    assert_quoting("title: &&x", "&&x", "title: \"&&x\"");
}

#[test]
fn test_alias_indicator() {
    assert_quoting("title: *undefined", "*undefined", "title: \"*undefined\"");
}

#[test]
fn test_block_scalar_indicator() {
    assert_quoting("title: |foo", "|foo", "title: \"|foo\"");
    assert_quoting("title: >foo", ">foo", "title: \">foo\"");
}

#[test]
fn test_flow_indicator_scalars() {
    assert_quoting("title: ,foo", ",foo", "title: \",foo\"");
    assert_quoting("title: ]foo", "]foo", "title: \"]foo\"");
    assert_quoting("title: }foo", "}foo", "title: \"}foo\"");
}

#[test]
fn test_sequence_context() {
    assert_quoting("items:\n  - @foo\n", "@foo", "items:\n  - \"@foo\"\n");
}

#[test]
fn test_sequence_context_second_entry() {
    assert_quoting(
        "items:\n  - one\n  - @two\n",
        "@two",
        "items:\n  - one\n  - \"@two\"\n",
    );
}

#[test]
fn test_nested_mapping_context() {
    assert_quoting(
        "a:\n  b:\n    title: @x\n",
        "@x",
        "a:\n  b:\n    title: \"@x\"\n",
    );
}

#[test]
fn test_sequence_entry_inline_mapping_context() {
    assert_quoting(
        "outer:\n  items:\n    - name: a\n      title: @x\n",
        "@x",
        "outer:\n  items:\n    - name: a\n      title: \"@x\"\n",
    );
}

#[test]
fn test_document_marker_context() {
    assert_quoting("---\nkey: @x\n", "@x", "---\nkey: \"@x\"\n");
}

#[test]
fn test_multibyte_lexeme() {
    assert_quoting("title: @日本語", "@日本語", "title: \"@日本語\"");
}

#[test]
fn test_lexeme_byte_equality_proof() {
    // The parse-recovery proof: after repair, the node at the lexically
    // scanned context path is a string byte-equal to the original lexeme.
    let source = "title: @daily-report";
    let outcome = analyze_yaml(source).apply();
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&outcome.source).unwrap();
    assert_eq!(
        value.get("title").and_then(|node| node.as_str()),
        Some("@daily-report"),
        "the parsed value at the context path must byte-equal the original lexeme"
    );
}

#[test]
fn test_two_error_chain_repairs_both() {
    // Iteration: one repair per round at a strictly advancing error offset.
    let source = "title: @a\ndate: `b\n";
    let analysis = analyze_yaml(source);
    let deterministic: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.classification == YamlCertainty::Deterministic)
        .collect();
    assert_eq!(deterministic.len(), 2);
    assert_eq!(deterministic[0].span, 7..9);
    assert_eq!(deterministic[1].span, 16..18);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "title: \"@a\"\ndate: \"`b\"\n");
    assert_untouched_bytes_preserved(source, &outcome);
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&outcome.source).unwrap();
    assert_eq!(value.get("title").and_then(|n| n.as_str()), Some("@a"));
    assert_eq!(value.get("date").and_then(|n| n.as_str()), Some("`b"));
}

#[test]
fn test_round_cap_leaves_original_bytes() {
    // Nine broken scalars exceed the 8-round bound: the chain is abandoned,
    // every finding is report-only, and the source stays byte-identical.
    let source = "k1: @x1\nk2: @x2\nk3: @x3\nk4: @x4\nk5: @x5\nk6: @x6\nk7: @x7\nk8: @x8\nk9: @x9\n";
    let analysis = analyze_yaml(source);
    assert!(!analysis.diagnostics().is_empty());
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source, "a partial chain is never emitted");
    assert!(!outcome.changed());
}

#[test]
fn test_comment_after_scalar_is_report_only() {
    // ` #` inside the remainder is comment-versus-content ambiguity.
    assert_report_only("title: @foo # comment");
    assert_report_only("title: @x # trailing\n");
}

#[test]
fn test_multiline_scalar_is_report_only() {
    assert_report_only("title: @daily\n  report\n");
}

#[test]
fn test_flow_context_is_report_only() {
    assert_report_only("data: {@x: 1}");
    assert_report_only("title: [foo");
    assert_report_only("title: {foo");
}

#[test]
fn test_unterminated_quote_is_report_only() {
    assert_report_only("title: 'unterminated");
    assert_report_only("title: \"unterminated");
}

#[test]
fn test_backslash_in_lexeme_is_report_only() {
    // Double-quoting could not reproduce the lexeme byte-for-byte.
    assert_report_only("title: @C:\\Users");
}

#[test]
fn test_quote_inside_lexeme_is_report_only() {
    assert_report_only("title: @say \"hi\"");
}

#[test]
fn test_parseable_inputs_never_trigger_quoting() {
    // No parse error means no S3 proof: these stay exactly as authored.
    for source in [
        "host:localhost\n",
        "ports:\n  -80\n  -443\n",
        "token: abc #123\n",
        "url: http://example.com\n",
        "path: C:\\Users\\Ken\n",
        "title: :foo\n",
        "title: ?foo\n",
        "title: -foo\n",
        "title: #foo\n",
    ] {
        let analysis = analyze_yaml(source);
        assert!(
            analysis
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.code != YamlDiagnosticCode::ReservedIndicator),
            "no reserved-indicator diagnostic for {source:?}"
        );
        let outcome = analysis.apply();
        assert_eq!(outcome.source, source, "untouched for {source:?}");
    }
}

#[test]
fn test_unrepairable_yaml_reports_parse_diagnostic() {
    let analysis = analyze_yaml("title: 'unterminated");
    let parse = analysis
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == YamlDiagnosticCode::Parse)
        .expect("parse diagnostic");
    assert_eq!(
        parse.classification,
        YamlCertainty::DeterministicFindNonDeterministicSolution
    );
    assert!(parse.repairs.is_empty());
}

#[test]
fn test_failure_location_is_structured() {
    let analysis = analyze_yaml("title: @daily-report");
    let failure = analysis.parse_failure().expect("parse failure");
    let location = failure.location.expect("structured location");
    assert_eq!(location.byte, 7);
    assert_eq!(location.line, 1);
}
