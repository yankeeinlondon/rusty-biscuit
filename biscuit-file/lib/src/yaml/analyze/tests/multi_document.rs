//! Tests for multiple-document detection (acceptance B-3). A stream with
//! two or more YAML documents is incompatible with single-document
//! analysis; the detector reports each extra document's opening span and
//! never selects, splits, or rewrites a document.

use super::super::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, analyze_yaml};

fn multi_document_diagnostics(source: &str) -> Vec<YamlDiagnostic> {
    analyze_yaml(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::MultiDocument)
        .cloned()
        .collect()
}

fn assert_untouched(source: &str) {
    let analysis = analyze_yaml(source);
    for diagnostic in analysis.diagnostics() {
        assert_ne!(
            diagnostic.classification,
            YamlCertainty::Deterministic,
            "multi-document findings are never auto-applied: {diagnostic:?}"
        );
        assert!(diagnostic.repairs.is_empty());
    }
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source, "source must stay byte-identical");
}

#[test]
fn test_two_documents() {
    let source = "---\nname: first\n---\nname: second\n";
    let diagnostics = multi_document_diagnostics(source);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    let diagnostic = &diagnostics[0];
    // The span covers the marker line opening the second document.
    assert_eq!(&source[diagnostic.span.clone()], "---");
    assert_eq!(diagnostic.span.start, source.find("---\nname: second").unwrap());
    assert!(diagnostic.message.contains("contains 2 YAML documents"));
    assert!(diagnostic.message.contains("starts document 2"));
    assert_eq!(
        diagnostic.classification,
        YamlCertainty::DeterministicFindNonDeterministicSolution
    );
    assert_untouched(source);
}

#[test]
fn test_three_documents_report_each_extra_document() {
    let source = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let diagnostics = multi_document_diagnostics(source);
    assert_eq!(diagnostics.len(), 2, "got {diagnostics:?}");
    assert!(diagnostics[0].message.contains("starts document 2"));
    assert!(diagnostics[1].message.contains("starts document 3"));
    assert!(diagnostics[0].message.contains("contains 3 YAML documents"));
    assert!(diagnostics[0].span.start < diagnostics[1].span.start);
    assert_untouched(source);
}

#[test]
fn test_implicit_first_document_plus_marker() {
    let source = "name: first\n---\nname: second\n";
    let diagnostics = multi_document_diagnostics(source);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "---");
    assert_untouched(source);
}

#[test]
fn test_single_document_with_start_marker_is_quiet() {
    let source = "---\nname: only\n";
    assert!(multi_document_diagnostics(source).is_empty());
}

#[test]
fn test_single_document_with_end_marker_is_quiet() {
    let source = "name: only\n...\n";
    assert!(multi_document_diagnostics(source).is_empty());
}

#[test]
fn test_single_document_with_both_markers_is_quiet() {
    let source = "---\nname: only\n...\n";
    assert!(multi_document_diagnostics(source).is_empty());
}

#[test]
fn test_content_after_end_marker_without_a_start_marker() {
    // After `...`, bare content is a new document fragment even without
    // `---`; the parser rejects it, and the detector explains why.
    let source = "name: first\n...\nname: second\n";
    let diagnostics = multi_document_diagnostics(source);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert!(diagnostics[0].span.start > source.find("...").unwrap());
    assert_untouched(source);
}

#[test]
fn test_trailing_comment_after_end_marker_is_quiet() {
    // Comments are not content: a comment after `...` does not open a
    // document.
    let source = "name: only\n...\n# trailer\n";
    assert!(multi_document_diagnostics(source).is_empty());
}

#[test]
fn test_empty_source_is_quiet() {
    assert!(multi_document_diagnostics("").is_empty());
}

#[test]
fn test_comment_only_source_is_quiet() {
    assert!(multi_document_diagnostics("# nothing here\n").is_empty());
}

#[test]
fn test_single_empty_document_is_quiet() {
    assert!(multi_document_diagnostics("---\n").is_empty());
    assert!(multi_document_diagnostics("---\n...\n").is_empty());
}

#[test]
fn test_indented_marker_inside_block_scalar_is_not_a_document() {
    // Document markers only exist at column zero; an indented `---` is
    // literal block-scalar content.
    let source = "script: |\n  ---\n  echo hi\n";
    assert!(multi_document_diagnostics(source).is_empty());
    assert!(analyze_yaml(source).is_parseable());
}

#[test]
fn test_zero_indent_marker_ends_a_block_scalar() {
    // A `---` at column zero closes the block scalar above and opens a new
    // document.
    let source = "script: |\n  echo hi\n---\nb: 1\n";
    let diagnostics = multi_document_diagnostics(source);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
}

#[test]
fn test_documents_with_comments_between_markers() {
    let source = "# leading\n---\na: 1\n# between\n---\nb: 2\n";
    let diagnostics = multi_document_diagnostics(source);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_untouched(source);
}

#[test]
fn test_deterministic_across_runs() {
    let source = "---\na: 1\n---\nb: 2\n";
    let first: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    let second: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    assert_eq!(first, second);
}
