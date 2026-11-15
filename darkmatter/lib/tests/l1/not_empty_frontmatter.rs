//! A `not-empty` property through Darkmatter's frontmatter validation, the
//! in-process entry point `md schema validate` calls
//! (`DarkmatterSchemas::validate_summarized`): the key may be omitted, hold
//! `null` (unset), or hold content, but blank text is a located problem; the
//! removed `non-null` marker is a schema error rather than a silent no-op.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};
use tempfile::TempDir;

/// Validates a document declaring `title: string(not-empty)` and
/// `tags: string[](not-empty)`, with `values` as its authored frontmatter
/// lines.
fn validate(values: &str) -> ValidationReport {
    let dir = TempDir::new().expect("tempdir");
    let source: Markdown =
        format!("---\n$schema:\n  title: string(not-empty)\n  tags: string[](not-empty)\n{values}---\nbody\n").into();
    let (report, summary) = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .validate_summarized(&source)
        .expect("the schema prepares");
    assert!(summary.has_schema(), "the inline schema applies");
    report
}

#[test]
fn absent_null_and_content_are_valid() {
    for values in ["", "title: null\ntags: null\n", "title: Hello\ntags: [a]\n"] {
        let report = validate(values);
        assert!(report.valid, "{values:?}: {:?}", report.problems);
    }
}

#[test]
fn blank_text_is_a_problem_located_at_the_property() {
    let report = validate("tags: [a]\ntitle: '  '\n");
    assert!(!report.valid);
    let [problem] = report.problems.as_slice() else {
        panic!("one problem: {:?}", report.problems);
    };
    assert_eq!(problem.path, "/title", "{problem:?}");
    // Line 1 is the opening `---`; `title: '  '` is the sixth line.
    assert_eq!(problem.line, Some(6), "{problem:?}");
}

#[test]
fn an_empty_list_is_a_problem_located_at_the_property() {
    let report = validate("tags: []\n");
    assert!(!report.valid);
    assert!(report.problems.iter().any(|problem| problem.path == "/tags"), "{:?}", report.problems);
}

#[test]
fn the_removed_non_null_marker_fails_schema_preparation() {
    let dir = TempDir::new().expect("tempdir");
    let source: Markdown = "---\n$schema:\n  title: string(non-null)\n---\nbody\n".into();
    let result = DarkmatterSchemas::new(FileResolutionContext::new(dir.path())).validate_summarized(&source);
    let message = match result {
        Err(error) => error.to_string(),
        Ok((report, _)) => format!("{:?}", report.problems),
    };
    assert!(message.contains("`non-null` is no longer supported"), "{message}");
}
