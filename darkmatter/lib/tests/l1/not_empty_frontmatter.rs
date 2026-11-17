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

/// Composes `content` as `doc.md` in a fresh directory anchored there.
fn compose(content: &str) -> Result<darkmatter::markdown::Markdown, darkmatter::markdown::MarkdownError> {
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("doc.md");
    std::fs::write(&path, content).expect("write");
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .with_source_file(path.clone());
    Markdown::try_from(path.as_path())
        .expect("loads")
        .compose_with(&crate::request_support::request_at(dir.path(), options))
        .map(|(composed, _)| composed)
}

/// A `yaml(not-empty)` value an interpolation produced as nothing is refused
/// at the property, while the same interpolation producing a definition is
/// accepted and an unwritten value is still "not set".
#[test]
fn an_interpolation_that_produced_no_yaml_is_refused() {
    let schema = "$schema:\n  source: string\n  config: yaml(not-empty)\n";

    let error = compose(&format!("---\n{schema}source: ''\nconfig: \"{{{{ source }}}}\"\n---\nbody\n"))
        .expect_err("an empty interpolation result is not YAML content");
    let darkmatter::markdown::MarkdownError::SchemaValidationFailed { problems, .. } = &error else {
        panic!("expected a schema failure, got {error:?}");
    };
    assert!(problems.iter().any(|problem| problem.path == "/config"), "{problems:?}");

    let composed = compose(&format!("---\n{schema}source: 'a: 1'\nconfig: \"{{{{ source }}}}\"\n---\nbody\n"))
        .expect("a produced definition is content");
    assert_eq!(composed.frontmatter().as_map().get("config"), Some(&serde_json::json!("a: 1")));

    compose(&format!("---\n{schema}---\nbody\n")).expect("an unwritten value is not set");
}
