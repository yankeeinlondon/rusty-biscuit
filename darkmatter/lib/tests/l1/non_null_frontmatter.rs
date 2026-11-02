//! A `non-null` property through Darkmatter's frontmatter validation, the
//! in-process entry point `md schema validate` calls
//! (`DarkmatterSchemas::validate_summarized`): the key may be omitted or hold
//! a value of its type, but an explicit `null` is a located problem, while an
//! unmarked optional property still accepts `null`.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};
use tempfile::TempDir;

/// Validates a document declaring `title: string(non-null)` and the unmarked
/// control `note: string`, with `values` as its authored frontmatter lines.
fn validate(values: &str) -> ValidationReport {
    let dir = TempDir::new().expect("tempdir");
    let source: Markdown =
        format!("---\n$schema:\n  title: string(non-null)\n  note: string\n{values}---\nbody\n").into();
    let (report, summary) = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .validate_summarized(&source)
        .expect("the schema prepares");
    assert!(summary.has_schema, "the inline schema applies");
    report
}

#[test]
fn an_absent_non_null_property_is_valid() {
    let report = validate("");
    assert!(report.valid, "{:?}", report.problems);
}

#[test]
fn a_string_non_null_property_is_valid() {
    let report = validate("title: Hello\n");
    assert!(report.valid, "{:?}", report.problems);
}

#[test]
fn an_explicit_null_is_a_problem_located_at_the_property() {
    let report = validate("note: ok\ntitle: null\n");
    assert!(!report.valid);
    let [problem] = report.problems.as_slice() else {
        panic!("one problem: {:?}", report.problems);
    };
    assert_eq!(problem.path, "/title", "{problem:?}");
    // Line 1 is the opening `---`; `title: null` is the sixth line.
    assert_eq!(problem.line, Some(6), "{problem:?}");
}

#[test]
fn the_unmarked_control_accepts_null() {
    let report = validate("note: null\n");
    assert!(report.valid, "{:?}", report.problems);
}
