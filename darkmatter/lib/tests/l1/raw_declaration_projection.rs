//! An effective schema's merged JSON Schema is projected to its declared
//! layer only when an expression reads a declaration the document's own
//! state does not answer, so composing a document with no expressions (or
//! whose expressions read only what it sets) never pays for projecting the
//! baseline.
//!
//! Each case composes a real document in its own temporary directory
//! through the public compose API against the built-in baseline, and counts
//! `Work::RawDeclarationProjection`. The case that reads a declaration also
//! checks the result the projection decides (the call check's argument
//! type), so a projection skipped where it is needed fails a verdict, not
//! only a count.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport};
use darkmatter::work_counters::{Work, snapshot};
use tempfile::TempDir;

const UNDECLARED: &str = "dm.expression.undeclared_property";

/// Composes `content` as `doc.md` in `dir` against the built-in baseline and
/// returns the output, the report, and the projections it made.
fn compose_counting(dir: &Path, content: &str) -> (String, ComposeReport, u64) {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone())
        .with_darkmatter_baseline_schema();
    let request = crate::request_support::request_at(dir, options);
    let before = snapshot();
    let (composed, report) = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&request)
        .unwrap_or_else(|error| panic!("{content:?} failed: {error}"));
    let projections = snapshot().since(&before).get(Work::RawDeclarationProjection);
    (composed.content().to_string(), report, projections)
}

fn undeclared(report: &ComposeReport) -> usize {
    report.warnings.iter().filter(|warning| warning.code.as_deref() == Some(UNDECLARED)).count()
}

#[test]
fn a_document_without_expressions_never_projects_the_baseline() {
    let dir = TempDir::new().unwrap();
    let (output, report, projections) = compose_counting(dir.path(), "# Sample\n\nHello.\n");
    assert!(output.contains("Hello."), "{output}");
    assert_eq!(undeclared(&report), 0);
    assert_eq!(projections, 0);
}

/// `tags` is typed only by the baseline (`string[]`), and the document does
/// not set it, so the call check reads its type from the projection.
#[test]
fn a_call_on_a_baseline_property_reads_its_projected_type() {
    let dir = TempDir::new().unwrap();
    let (output, report, projections) = compose_counting(dir.path(), "---\ntitle: Greeting\n---\n# {{ title }}\n\n[{{ upper(tags) }}]\n");
    assert!(output.contains("# Greeting"), "{output}");
    let typed: Vec<Option<&str>> = report
        .warnings
        .iter()
        .filter(|warning| warning.code.as_deref() == Some("dm.expression.uncertain_call"))
        .map(|warning| warning.diagnostic.as_ref().and_then(|diagnostic| diagnostic.actual_type.as_deref()))
        .collect();
    assert_eq!(typed, [Some("string[] | null")], "the argument is typed by the baseline: {:?}", report.warnings);
    assert_eq!(projections, 1, "projected once, on the first declaration read");
}

/// A property no layer declares still raises the undeclared warning.
#[test]
fn a_misspelled_property_is_still_reported_undeclared() {
    let dir = TempDir::new().unwrap();
    let (output, report, _) = compose_counting(dir.path(), "---\ntitle: Greeting\n---\n[{{ descripton }}]\n");
    assert!(output.contains("[]"), "{output}");
    assert_eq!(undeclared(&report), 1, "{:?}", report.warnings);
}
