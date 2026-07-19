//! Tests for duplicate mapping key detection (acceptance B-1).
//!
//! `serde_yaml_ng` rejects documents containing duplicate keys, so the
//! parser alone can never report both conflicting entries; the lexical
//! detector reports every occurrence with its own span, at every block
//! nesting level and inside flow mappings. The finding is certain, but no
//! repair is ever selected automatically: keeping the first value, the
//! last, renaming, merging, or deleting are all intent decisions.

use super::super::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, analyze_yaml};

/// All duplicate-key diagnostics for `source`.
fn duplicate_key_diagnostics(source: &str) -> Vec<YamlDiagnostic> {
    analyze_yaml(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::DuplicateKey)
        .cloned()
        .collect()
}

/// Asserts every duplicate-key diagnostic is report-only with no repair,
/// and that applying the analysis leaves the source byte-identical.
fn assert_report_only_and_untouched(source: &str) {
    let analysis = analyze_yaml(source);
    for diagnostic in analysis.diagnostics() {
        assert_ne!(
            diagnostic.classification,
            YamlCertainty::Deterministic,
            "duplicate keys are never auto-applied: {diagnostic:?}"
        );
        assert!(
            diagnostic.repairs.is_empty(),
            "report-only findings carry no repairs: {diagnostic:?}"
        );
    }
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source, "source must stay byte-identical");
    assert!(!outcome.changed());
}

#[test]
fn test_root_level_duplicate_reports_both_spans() {
    let source = "environment: production\nreplicas: 3\nenvironment: staging\n";
    let diagnostics = duplicate_key_diagnostics(source);
    assert_eq!(diagnostics.len(), 2, "got {diagnostics:?}");
    // Both conflicting entries carry their own span.
    assert_eq!(&source[diagnostics[0].span.clone()], "environment");
    assert_eq!(&source[diagnostics[1].span.clone()], "environment");
    assert_ne!(diagnostics[0].span, diagnostics[1].span);
    for diagnostic in &diagnostics {
        assert_eq!(
            diagnostic.classification,
            YamlCertainty::DeterministicFindNonDeterministicSolution
        );
    }
    // The messages cross-reference the conflicting lines.
    assert!(diagnostics[0].message.contains("redefined at line 3"));
    assert!(diagnostics[1].message.contains("first defined at line 1"));
    assert_report_only_and_untouched(source);
}

#[test]
fn test_nested_duplicate() {
    let source = "outer:\n  key: one\n  key: two\n";
    let diagnostics = duplicate_key_diagnostics(source);
    assert_eq!(diagnostics.len(), 2, "got {diagnostics:?}");
    assert!(diagnostics.iter().all(|d| &source[d.span.clone()] == "key"));
    assert_report_only_and_untouched(source);
}

#[test]
fn test_deeply_nested_duplicate() {
    let source = "a:\n  b:\n    c:\n      x: 1\n      x: 2\n";
    let diagnostics = duplicate_key_diagnostics(source);
    assert_eq!(diagnostics.len(), 2, "got {diagnostics:?}");
    assert!(diagnostics.iter().all(|d| &source[d.span.clone()] == "x"));
    assert_report_only_and_untouched(source);
}

#[test]
fn test_same_key_in_sibling_scopes_is_not_a_duplicate() {
    // Distinct mapping scopes may reuse key names freely.
    let source = "development:\n  timeout: 10\nproduction:\n  timeout: 30\n";
    assert!(duplicate_key_diagnostics(source).is_empty());
}

#[test]
fn test_same_key_in_sequence_items_is_not_a_duplicate() {
    // Each sequence item is its own mapping scope.
    let source = "- name: one\n- name: two\n";
    assert!(duplicate_key_diagnostics(source).is_empty());
}

#[test]
fn test_quoted_and_plain_spellings_of_one_key_conflict() {
    // `"a"` and `a` are the same key; the parser rejects both spellings.
    let source = "a: 1\n\"a\": 2\n";
    let diagnostics = duplicate_key_diagnostics(source);
    assert_eq!(diagnostics.len(), 2, "got {diagnostics:?}");
    assert_report_only_and_untouched(source);
}

#[test]
fn test_three_occurrences_report_three_spans() {
    let source = "a: 1\na: 2\na: 3\n";
    let diagnostics = duplicate_key_diagnostics(source);
    assert_eq!(diagnostics.len(), 3, "got {diagnostics:?}");
    assert!(diagnostics[0].message.contains("redefined at line 2"));
    assert!(diagnostics[1].message.contains("first defined at line 1"));
    assert!(diagnostics[2].message.contains("first defined at line 1"));
    assert_report_only_and_untouched(source);
}

#[test]
fn test_duplicate_inside_flow_mapping() {
    let source = "{a: 1, b: 2, a: 3}\n";
    let diagnostics = duplicate_key_diagnostics(source);
    assert_eq!(diagnostics.len(), 2, "got {diagnostics:?}");
    assert!(diagnostics.iter().all(|d| &source[d.span.clone()] == "a"));
    assert_report_only_and_untouched(source);
}

#[test]
fn test_duplicate_inside_nested_flow_mapping() {
    // The inner flow mapping has the conflict; the outer `a` keys live in
    // different scopes.
    let source = "{a: 1, b: {a: 2, a: 3}}\n";
    let diagnostics = duplicate_key_diagnostics(source);
    assert_eq!(diagnostics.len(), 2, "got {diagnostics:?}");
    let first = &diagnostics[0].span;
    let second = &diagnostics[1].span;
    assert_eq!(&source[first.clone()], "a");
    assert_eq!(&source[second.clone()], "a");
    // Both spans are inside the inner mapping.
    assert!(first.start > source.find("{a: 2").unwrap() - 1);
    assert_report_only_and_untouched(source);
}

#[test]
fn test_no_duplicate_is_quiet() {
    let source = "a: 1\nb: 2\nc:\n  d: 3\n";
    assert!(duplicate_key_diagnostics(source).is_empty());
}

#[test]
fn test_comments_around_duplicates_are_preserved() {
    let source = "a: 1 # first definition\na: 2 # second definition\n";
    let diagnostics = duplicate_key_diagnostics(source);
    assert_eq!(diagnostics.len(), 2);
    let outcome = analyze_yaml(source).apply();
    assert_eq!(outcome.source, source);
    assert!(outcome.source.contains("# first definition"));
    assert!(outcome.source.contains("# second definition"));
}

#[test]
fn test_duplicate_keys_across_documents_are_not_conflicts() {
    // Anchor scopes and key scopes never cross a document boundary.
    let source = "---\na: 1\n---\na: 2\n";
    assert!(duplicate_key_diagnostics(source).is_empty());
}

#[test]
fn test_diagnostics_sorted_in_source_order() {
    let source = "b: 1\nb: 2\na:\n  c: 3\n  c: 4\n";
    let analysis = analyze_yaml(source);
    let spans: Vec<_> = analysis
        .diagnostics()
        .iter()
        .map(|diagnostic| (diagnostic.span.start, diagnostic.span.end))
        .collect();
    let mut sorted = spans.clone();
    sorted.sort();
    assert_eq!(spans, sorted, "diagnostics must be in stable source order");
}

#[test]
fn test_deterministic_across_runs() {
    let source = "a: 1\nb: 2\na: 3\n";
    let first: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    let second: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    assert_eq!(first, second);
}
