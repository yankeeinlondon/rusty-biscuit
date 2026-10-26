//! Root-indentation repair: a parseable root mapping indented as a whole gets
//! one deterministic `yaml.root-indentation` diagnostic whose prefix removals
//! move its keys to column 0 without changing the parsed value.

use serde_yaml_ng::Value;

use super::super::{YamlCertainty, YamlDiagnosticCode, analyze_yaml};
use super::assert_untouched_bytes_preserved;

fn parse(source: &str) -> Value {
    serde_yaml_ng::from_str(source).expect("source parses")
}

fn root_indentation_count(source: &str) -> usize {
    analyze_yaml(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::RootIndentation)
        .count()
}

/// Analyzes `source`, asserts exactly one deterministic root-indentation
/// diagnostic, applies every eligible repair, and asserts the applied text
/// is audit-clean, byte-preserving outside its spans, and parse-equivalent.
fn repair(source: &str) -> String {
    let analysis = analyze_yaml(source);
    assert!(analysis.is_parseable(), "input must parse: {source:?}");
    let roots: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::RootIndentation)
        .collect();
    assert_eq!(roots.len(), 1, "one root-indentation diagnostic: {:?}", analysis.diagnostics());
    assert_eq!(roots[0].classification, YamlCertainty::Deterministic);
    for repair in &roots[0].repairs {
        assert!(repair.replacement.is_empty(), "prefix removals only: {repair:?}");
        assert!(
            source[repair.span.clone()].bytes().all(|byte| byte == b' '),
            "removes only spaces: {repair:?}"
        );
    }
    let outcome = analysis.apply();
    assert!(outcome.audit.rejected.is_empty(), "{:?}", outcome.audit.rejected);
    assert_untouched_bytes_preserved(source, &outcome);
    assert_eq!(parse(&outcome.source), parse(source), "parsed value preserved");
    assert_eq!(root_indentation_count(&outcome.source), 0, "second pass offers no dedent");
    outcome.source
}

#[test]
fn test_reproduction_prompt_block_moves_to_column_zero() {
    let source = "    prompt: |-\n        Your task is to ...\n";

    let repaired = repair(source);

    assert_eq!(repaired, "prompt: |-\n    Your task is to ...\n");
    assert_eq!(parse(&repaired)["prompt"], Value::from("Your task is to ..."));
}

#[test]
fn test_common_indent_is_removed_preserving_order_nesting_and_comments() {
    let source = "  # about\n  b: 1\n  a:\n    nested: two # trailing note\n    list:\n      - x\n  \"quoted key\": 3\n";

    let repaired = repair(source);

    assert_eq!(
        repaired,
        "# about\nb: 1\na:\n  nested: two # trailing note\n  list:\n    - x\n\"quoted key\": 3\n"
    );
    let keys: Vec<_> = parse(&repaired)
        .as_mapping()
        .expect("mapping")
        .keys()
        .cloned()
        .collect();
    assert_eq!(keys, vec![Value::from("b"), Value::from("a"), Value::from("quoted key")]);
}

#[test]
fn test_less_indented_comments_are_kept() {
    let source = "# top\n    a: 1\n  # between\n    b: 2\n";

    assert_eq!(repair(source), "# top\na: 1\n  # between\nb: 2\n");
}

#[test]
fn test_crlf_terminators_survive_prefix_edits_and_combine_with_line_endings() {
    let source = "    a: 1\r\n    b: |\r\n      text\r\n";

    let analysis = analyze_yaml(source);
    let root = analysis
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == YamlDiagnosticCode::RootIndentation)
        .expect("root-indentation diagnostic");
    let only_root = super::super::apply_edit_set(source, &root.repairs);
    assert_eq!(only_root.source, "a: 1\r\nb: |\r\n  text\r\n");

    let repaired = repair(source);
    assert_eq!(repaired, "a: 1\nb: |\n  text\n");
}

#[test]
fn test_combines_with_trailing_whitespace_and_final_newline_repairs() {
    let source = "    a: 1   \n    \n    b: two\n\n\n";

    let repaired = repair(source);

    assert_eq!(repaired, "a: 1\n\nb: two\n");
}

#[test]
fn test_missing_final_newline_combines_with_dedent() {
    assert_eq!(repair("    a: 1\n    b: 2"), "a: 1\nb: 2\n");
}

#[test]
fn test_blank_lines_outside_block_scalars_are_left_to_trailing_whitespace() {
    let source = "    a: 1\n        \n    b: 2\n";
    let analysis = analyze_yaml(source);
    let root = analysis
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == YamlDiagnosticCode::RootIndentation)
        .expect("root-indentation diagnostic");
    assert_eq!(root.repairs.len(), 2, "the blank line is not dedented: {root:?}");
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == YamlDiagnosticCode::TrailingWhitespace)
    );

    assert_eq!(repair(source), "a: 1\n\nb: 2\n");
}

#[test]
fn test_whitespace_only_scalar_lines_at_below_and_above_content_indent() {
    // Content indent is 8; blank scalar lines carry 8 (at), 2 (below), and
    // 10 (above) spaces. The 10-space line holds two spaces of content.
    let source = "    prompt: |-\n        one\n        \n        two\n  \n        three\n          \n        four\n    next: 1\n";

    let repaired = repair(source);

    assert_eq!(
        repaired,
        "prompt: |-\n    one\n    \n    two\n\n    three\n      \n    four\nnext: 1\n"
    );
    assert_eq!(
        parse(&repaired)["prompt"],
        Value::from("one\n\ntwo\n\nthree\n  \nfour")
    );
}

#[test]
fn test_literal_and_folded_scalars_keep_their_values() {
    let source = "    literal: |\n        line one\n          indented\n    folded: >\n        folded one\n        folded two\n\n        para\n    keep: |+\n        kept\n\n    tail: end\n";

    repair(source);
}

#[test]
fn test_explicit_indentation_indicator_keeps_value() {
    let source = "    text: |2\n          leading spaces kept\n        base\n    other: 1\n";

    let repaired = repair(source);

    assert_eq!(
        parse(&repaired)["text"],
        Value::from("    leading spaces kept\n  base\n")
    );
}

#[test]
fn test_scalar_lines_beginning_with_hash_are_content_not_comments() {
    let source = "    script: |\n        # not a comment\n        echo hi\n    after: 1\n";

    let repaired = repair(source);

    assert_eq!(
        parse(&repaired)["script"],
        Value::from("# not a comment\necho hi\n")
    );
}

#[test]
fn test_multiline_quoted_and_plain_scalars_keep_their_values() {
    let source = "    double: \"first\n      second\"\n    single: 'one\n      two'\n    plain: alpha\n      beta\n";

    repair(source);
}

#[test]
fn test_nested_multiline_flow_value_keeps_its_value() {
    let source = "    list: [\n        a,\n        b\n      ]\n    after: 1\n";

    repair(source);
}

#[test]
fn test_column_zero_mapping_gets_no_root_repair() {
    assert_eq!(root_indentation_count("a: 1\nb:\n  c: 2\n"), 0);
    assert_eq!(root_indentation_count("# note\na: 1\n"), 0);
}

#[test]
fn test_excluded_root_forms_get_no_root_repair() {
    for source in [
        "",
        "   \n",
        "    # only a comment\n",
        "    - a\n    - b\n",
        "    just a scalar\n",
        "    {a: 1, b: 2}\n",
        "    [a, b]\n",
        "    ? complex\n    : value\n",
        "    !!map\n    a: 1\n",
        "    &root a: 1\n    b: 2\n",
        "---\n    a: 1\n",
        "%YAML 1.2\n---\n    a: 1\n",
        "    a: 1\n...\n",
    ] {
        assert_eq!(root_indentation_count(source), 0, "no root repair for {source:?}");
    }
}

#[test]
fn test_invalid_documents_keep_their_outcomes_without_a_root_repair() {
    // Duplicate keys and trailing garbage fail the parse, so no S1 repair
    // can run; a second document is reported, never dedented.
    for (source, parseable) in [
        ("    a: 1\n    a: 2\n", false),
        ("    a: 1\n  ] garbage\n", false),
        ("    a: 1\n---\n    b: 2\n", false),
    ] {
        let analysis = analyze_yaml(source);
        assert_eq!(analysis.is_parseable(), parseable, "{source:?}");
        let codes: Vec<_> = analysis.diagnostics().iter().map(|diagnostic| diagnostic.code).collect();
        assert!(
            !codes.contains(&YamlDiagnosticCode::RootIndentation),
            "no root repair for {source:?}: {codes:?}"
        );
        assert!(!codes.is_empty(), "{source:?} is still reported");
    }
    let second = analyze_yaml("    a: 1\n---\n    b: 2\n");
    assert!(
        second
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == YamlDiagnosticCode::MultiDocument)
    );
}

#[test]
fn test_tab_indented_root_is_recovered_first_then_dedented_on_rescan() {
    let source = "\tprompt: |-\n\t\tbody\n\tnext: 1\n";

    let first = analyze_yaml(source);
    assert!(!first.is_parseable());
    assert_eq!(root_indentation_count(source), 0);
    let recovered = first.apply().source;
    assert_eq!(recovered, "  prompt: |-\n    body\n  next: 1\n");

    assert_eq!(repair(&recovered), "prompt: |-\n  body\nnext: 1\n");
}
