//! Tab-indentation recovery: a document that parses only once its leading
//! tabs become spaces gets one deterministic `yaml.tab-indentation` repair per
//! tab-indented line, matching Darkmatter's frontmatter tab normalization.

use serde_yaml_ng::Value;

use super::super::{YamlCertainty, YamlDiagnosticCode, analyze_yaml};
use super::assert_untouched_bytes_preserved;

/// Analyzes `source`, asserts every diagnostic is a deterministic tab repair,
/// and returns the repaired source.
fn repair(source: &str, expected_lines: usize) -> String {
    let analysis = analyze_yaml(source);
    assert!(!analysis.is_parseable(), "input must not parse: {source:?}");
    let tabs: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::TabIndentation)
        .collect();
    assert_eq!(
        tabs.len(),
        expected_lines,
        "one repair per tab-indented line: {:?}",
        analysis.diagnostics()
    );
    for diagnostic in &tabs {
        assert_eq!(diagnostic.classification, YamlCertainty::Deterministic);
        assert_eq!(diagnostic.repairs.len(), 1);
    }
    let outcome = analysis.apply();
    assert!(outcome.audit.rejected.is_empty());
    assert_untouched_bytes_preserved(source, &outcome);
    outcome.source
}

fn parse(source: &str) -> Value {
    serde_yaml_ng::from_str(source).expect("repaired source parses")
}

#[test]
fn test_tab_indented_block_scalar_reads_inner_tabs_as_two_spaces() {
    let source = "prompt: |-\n\tLine one\n\t\tLine two\nlast_updated: 2026-02-27\n";

    let repaired = repair(source, 2);

    assert_eq!(
        repaired,
        "prompt: |-\n  Line one\n    Line two\nlast_updated: 2026-02-27\n"
    );
    let value = parse(&repaired);
    assert_eq!(value["prompt"], Value::from("Line one\n  Line two"));
    assert_eq!(value["last_updated"], Value::from("2026-02-27"));
}

#[test]
fn test_tab_indented_list_item_is_repaired() {
    let source = "last_updated: 2026-02-27\nupdate_policy:\n\t- Duration(6mo)\n";

    let repaired = repair(source, 1);

    assert_eq!(
        repaired,
        "last_updated: 2026-02-27\nupdate_policy:\n  - Duration(6mo)\n"
    );
    assert_eq!(
        parse(&repaired)["update_policy"],
        Value::Sequence(vec![Value::from("Duration(6mo)")])
    );
}

#[test]
fn test_tab_indented_nested_mappings_are_repaired() {
    let source = "outer:\n\tinner:\n\t\tleaf: 1\n  \tspaced: 2\n\tsibling: true\n";

    let repaired = repair(source, 4);

    assert_eq!(
        repaired,
        "outer:\n  inner:\n    leaf: 1\n    spaced: 2\n  sibling: true\n"
    );
    let value = parse(&repaired);
    assert_eq!(value["outer"]["inner"]["leaf"], Value::from(1));
    assert_eq!(value["outer"]["inner"]["spaced"], Value::from(2));
    assert_eq!(value["outer"]["sibling"], Value::Bool(true));
}

#[test]
fn test_tab_repair_leaves_tabs_after_indentation_alone() {
    let source = "outer:\n\tkey: \"a\tb\"\n";

    let analysis = analyze_yaml(source);
    let repair = &analysis.diagnostics()[0].repairs[0];

    assert_eq!(repair.span, 7..8);
    assert_eq!(repair.replacement, "  ");
    assert_eq!(analysis.apply().source, "outer:\n  key: \"a\tb\"\n");
}

#[test]
fn test_document_without_tabs_gets_no_tab_repair() {
    let source = "update_policy:\n  - Duration(6mo)\n";

    let analysis = analyze_yaml(source);

    assert!(analysis.is_parseable());
    assert!(analysis.is_clean(), "{:?}", analysis.diagnostics());
}

#[test]
fn test_parseable_document_with_tabs_gets_no_tab_repair() {
    // Darkmatter normalizes only when the direct parse fails; so does the
    // analyzer, so tabs that YAML accepts are never rewritten.
    let source = "key: |\n  first\n  \tindented content\n";

    let analysis = analyze_yaml(source);

    assert!(analysis.is_parseable());
    assert!(
        analysis
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code != YamlDiagnosticCode::TabIndentation)
    );
}

#[test]
fn test_tab_repair_is_withheld_when_normalized_text_still_fails() {
    let source = "outer:\n\tkey: [unclosed\n";

    let analysis = analyze_yaml(source);

    assert!(
        analysis
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code != YamlDiagnosticCode::TabIndentation)
    );
    assert_eq!(analysis.apply().source, source);
}
