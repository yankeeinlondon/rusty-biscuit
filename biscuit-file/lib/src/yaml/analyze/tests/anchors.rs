//! Tests for anchor/alias condition detection (acceptance B-2): undeclared,
//! forward, misspelled, duplicate, and unused anchors and aliases. Every
//! result is report-only and preserves graph-sensitive source: moving an
//! anchor, changing an alias, or expanding a value can each alter graph
//! semantics, so no repair is ever attached.

use super::super::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, analyze_yaml};

fn anchor_diagnostics(source: &str, code: YamlDiagnosticCode) -> Vec<YamlDiagnostic> {
    analyze_yaml(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .cloned()
        .collect()
}

/// Asserts no automatic repair exists anywhere and the source is
/// byte-identical after application.
fn assert_untouched(source: &str) {
    let analysis = analyze_yaml(source);
    for diagnostic in analysis.diagnostics() {
        assert_ne!(
            diagnostic.classification,
            YamlCertainty::Deterministic,
            "anchor/alias findings are never auto-applied: {diagnostic:?}"
        );
        assert!(diagnostic.repairs.is_empty());
    }
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source, "source must stay byte-identical");
}

/// Asserts the anchor/alias diagnostics (and only those) are report-only.
/// Block-position aliases also match the S3 reserved-indicator repair from
/// Phase 3 (quoting `*name` is the ratified parse-shape repair); that
/// deterministic coexistence is orthogonal to these findings.
fn assert_anchor_findings_report_only(source: &str) {
    let analysis = analyze_yaml(source);
    for diagnostic in analysis.diagnostics() {
        if diagnostic.code == YamlDiagnosticCode::ReservedIndicator {
            continue;
        }
        assert_ne!(
            diagnostic.classification,
            YamlCertainty::Deterministic,
            "anchor/alias findings are never auto-applied: {diagnostic:?}"
        );
        assert!(diagnostic.repairs.is_empty());
    }
}

#[test]
fn test_undeclared_alias() {
    let source = "service: *defaults\n";
    let diagnostics = anchor_diagnostics(source, YamlDiagnosticCode::AnchorUndeclared);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "defaults");
    assert_eq!(
        diagnostics[0].classification,
        YamlCertainty::DeterministicFindNonDeterministicSolution
    );
    assert_anchor_findings_report_only(source);
}

#[test]
fn test_undeclared_alias_in_flow_is_untouched() {
    // Flow contexts are outside the S3 grammar, so nothing auto-applies.
    let source = "data: {service: *defaults}\n";
    let diagnostics = anchor_diagnostics(source, YamlDiagnosticCode::AnchorUndeclared);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_untouched(source);
}

#[test]
fn test_forward_alias() {
    let source = "production: *defaults\ndefaults: &defaults\n  retries: 3\n";
    let diagnostics = anchor_diagnostics(source, YamlDiagnosticCode::AnchorForward);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "defaults");
    assert!(diagnostics[0].message.contains("declared later at line 2"));
    // The anchor is referenced (albeit early), so it is not also "unused".
    assert!(anchor_diagnostics(source, YamlDiagnosticCode::AnchorUnused).is_empty());
    assert_anchor_findings_report_only(source);
}

#[test]
fn test_forward_alias_in_flow_is_untouched() {
    let source = "data: {production: *defaults}\ndefaults: &defaults\n  retries: 3\n";
    assert_eq!(
        anchor_diagnostics(source, YamlDiagnosticCode::AnchorForward).len(),
        1
    );
    assert_untouched(source);
}

#[test]
fn test_misspelled_alias_suggests_nearby_anchor() {
    // The research example: `*defualts` is a transposition of `&defaults`.
    let source = "service: *defualts\ndefaults: &defaults\n  retries: 3\n";
    let diagnostics = anchor_diagnostics(source, YamlDiagnosticCode::AnchorMisspelled);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "defualts");
    assert!(diagnostics[0].message.contains("did you mean `&defaults`"));
    // The suggested anchor is presumed referenced; it is not also unused.
    assert!(anchor_diagnostics(source, YamlDiagnosticCode::AnchorUnused).is_empty());
    assert_anchor_findings_report_only(source);
}

#[test]
fn test_far_misspelling_stays_undeclared() {
    // `*prod` is not close to `&development` (edit distance above the
    // threshold), so no misspelling is claimed.
    let source = "service: *prod\ndefaults: &development\n  retries: 3\n";
    assert!(anchor_diagnostics(source, YamlDiagnosticCode::AnchorMisspelled).is_empty());
    assert_eq!(
        anchor_diagnostics(source, YamlDiagnosticCode::AnchorUndeclared).len(),
        1
    );
}

#[test]
fn test_duplicate_anchor() {
    // Duplicate anchors parse (the later declaration shadows), so nothing
    // else intervenes.
    let source = "a: &x 1\nb: &x 2\nc: *x\n";
    let diagnostics = anchor_diagnostics(source, YamlDiagnosticCode::AnchorDuplicate);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    // The redeclaration is flagged, not the original definition.
    assert_eq!(&source[diagnostics[0].span.clone()], "x");
    assert_eq!(diagnostics[0].span.start, source.find("&x 2").unwrap() + 1);
    assert!(diagnostics[0].message.contains("shadows"));
    assert_eq!(
        diagnostics[0].classification,
        YamlCertainty::DeterministicFindNonDeterministicSolution
    );
    assert_untouched(source);
}

#[test]
fn test_unused_anchor() {
    let source = "a: &x 1\nb: 2\n";
    let diagnostics = anchor_diagnostics(source, YamlDiagnosticCode::AnchorUnused);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "x");
    // Unused is a smell, not a certain problem: an anchor may exist for an
    // external consumer.
    assert_eq!(diagnostics[0].classification, YamlCertainty::NonDeterministicFind);
    assert_untouched(source);
}

#[test]
fn test_declared_and_used_anchor_is_quiet() {
    let source = "a: &x 1\nb: *x\n";
    for code in [
        YamlDiagnosticCode::AnchorUndeclared,
        YamlDiagnosticCode::AnchorForward,
        YamlDiagnosticCode::AnchorMisspelled,
        YamlDiagnosticCode::AnchorDuplicate,
        YamlDiagnosticCode::AnchorUnused,
    ] {
        assert!(
            anchor_diagnostics(source, code).is_empty(),
            "no {code} expected for {source:?}"
        );
    }
}

#[test]
fn test_merge_key_alias_counts_as_a_use() {
    let source = "base: &base\n  retries: 3\nservice:\n  <<: *base\n";
    assert!(anchor_diagnostics(source, YamlDiagnosticCode::AnchorUnused).is_empty());
    assert!(anchor_diagnostics(source, YamlDiagnosticCode::AnchorUndeclared).is_empty());
}

#[test]
fn test_alias_in_sequence_entry() {
    let source = "a: &x 1\nitems:\n  - *x\n";
    assert!(anchor_diagnostics(source, YamlDiagnosticCode::AnchorUndeclared).is_empty());
    assert!(anchor_diagnostics(source, YamlDiagnosticCode::AnchorUnused).is_empty());
}

#[test]
fn test_anchors_do_not_cross_document_boundaries() {
    // The second document's `*x` cannot see the first document's `&x`.
    let source = "---\na: &x 1\nb: *x\n---\nc: *x\n";
    let undeclared = anchor_diagnostics(source, YamlDiagnosticCode::AnchorUndeclared);
    assert_eq!(undeclared.len(), 1, "got {undeclared:?}");
    assert_eq!(&source[undeclared[0].span.clone()], "x");
    assert!(undeclared[0].span.start > source.find("---\nc:").unwrap());
    // The first document's anchor is used by its own alias.
    assert!(anchor_diagnostics(source, YamlDiagnosticCode::AnchorUnused).is_empty());
    assert_untouched(source);
}

#[test]
fn test_comments_around_anchors_are_preserved() {
    let source = "a: &x 1 # shared\nb: 2 # not referenced\n";
    let outcome = analyze_yaml(source).apply();
    assert_eq!(outcome.source, source);
    assert!(outcome.source.contains("# shared"));
}

#[test]
fn test_anchor_findings_sorted_in_source_order() {
    // The flow alias keeps S3 out, so every finding here is report-only.
    let source = "b: &two 2\na: {x: *nope}\nc: &two 3\n";
    let analysis = analyze_yaml(source);
    let spans: Vec<_> = analysis
        .diagnostics()
        .iter()
        .map(|diagnostic| (diagnostic.span.start, diagnostic.span.end))
        .collect();
    let mut sorted = spans.clone();
    sorted.sort();
    assert_eq!(spans, sorted, "diagnostics must be in stable source order");
    // The undeclared alias, the duplicate anchor, and both unused anchor
    // declarations were all found.
    assert_eq!(
        anchor_diagnostics(source, YamlDiagnosticCode::AnchorUndeclared).len(),
        1
    );
    assert_eq!(
        anchor_diagnostics(source, YamlDiagnosticCode::AnchorDuplicate).len(),
        1
    );
    assert_eq!(
        anchor_diagnostics(source, YamlDiagnosticCode::AnchorUnused).len(),
        2
    );
    assert_untouched(source);
}

#[test]
fn test_deterministic_across_runs() {
    let source = "a: &x 1\nb: *y\nc: &x 2\n";
    let first: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    let second: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    assert_eq!(first, second);
}
