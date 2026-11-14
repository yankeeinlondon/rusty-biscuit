//! The `edit` action's input matrix judged twice: by schema validation
//! against Darkmatter's shipped baseline (the verdict `md schema validate`
//! and DMLS report) and by md's lifecycle parser (the verdict `md compose`
//! acts on). Every cell must get the same verdict from both, and a schema
//! refusal must sit at the action that holds the bad operand.
//!
//! Each cell is placed in `initialize` and in the `else` branch of a
//! `composed` conditional, so neither the shell-free initialize profile nor
//! a branch the run would not take changes the verdict.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};

use crate::request_support::context_at;

/// Actions both checks accept.
const ACCEPTED: &[&str] = &[
    "{ edit: notes.md }",
    "{ edit: [notes.md] }",
    "{ edit: ' padded name.md ' }",
    "{ edit: '{{ fqn }}' }",
    "{ edit: 'notes/{{ fqn }}.md' }",
    "{ edit: { file: notes.md } }",
    "{ edit: { file: '{{ fqn }}', create_new: true, gui: false, no_error: true } }",
    "{ edit: { file: notes.md, create_new: false, gui: true } }",
    "{ edit: notes.md, info: hi, no_error: true }",
    "{ edit: notes.md, when: 'true' }",
];

/// Actions both checks refuse, each with the shape it exercises.
const REFUSED: &[(&str, &str)] = &[
    ("short: null", "{ edit: null }"),
    ("short: empty list", "{ edit: [] }"),
    ("short: two items", "{ edit: [a.md, b.md] }"),
    ("short: one wrong element", "{ edit: [a.md, 1] }"),
    ("short: every element wrong", "{ edit: [1] }"),
    ("short: number", "{ edit: 5 }"),
    ("short: boolean", "{ edit: true }"),
    ("short: empty string", "{ edit: '' }"),
    ("short: blank string", "{ edit: '   ' }"),
    ("short: blank list element", "{ edit: ['  '] }"),
    ("long: file absent", "{ edit: { create_new: true } }"),
    ("long: file null", "{ edit: { file: null } }"),
    ("long: file number", "{ edit: { file: 5 } }"),
    ("long: file boolean", "{ edit: { file: true } }"),
    ("long: file empty", "{ edit: { file: '' } }"),
    ("long: file blank", "{ edit: { file: '  ' } }"),
    ("long: file object", "{ edit: { file: { a: 1 } } }"),
    ("long: gui text", "{ edit: { file: notes.md, gui: 'yes' } }"),
    ("long: gui quoted boolean", "{ edit: { file: notes.md, gui: 'true' } }"),
    ("long: gui number", "{ edit: { file: notes.md, gui: 1 } }"),
    ("long: gui template", "{ edit: { file: notes.md, gui: '{{ true }}' } }"),
    ("long: gui null", "{ edit: { file: notes.md, gui: null } }"),
    ("long: gui object", "{ edit: { file: notes.md, gui: { a: 1 } } }"),
    ("long: create_new text", "{ edit: { file: notes.md, create_new: 'yes' } }"),
    ("long: create_new null", "{ edit: { file: notes.md, create_new: null } }"),
    ("long: unknown key", "{ edit: { file: notes.md, bogus: 1 } }"),
    ("long: with", "{ edit: { file: notes.md, with: { a: 1 } } }"),
    ("long form in a bundle", "{ edit: { file: notes.md }, info: hi }"),
];

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    baseline: DarkmatterSchemas,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let baseline = DarkmatterSchemas::new(context_at(&root))
            .with_darkmatter_baseline_json_schema()
            .expect("the shipped baseline attaches");
        Self { _dir: dir, path: root.join("doc.md"), baseline }
    }

    fn validate(&self, frontmatter: &str) -> ValidationReport {
        std::fs::write(&self.path, format!("---\n{frontmatter}\n---\nBody\n")).expect("write document");
        let markdown = Markdown::try_from(self.path.as_path())
            .expect("the document loads")
            .with_source(ComposeSource::File(self.path.clone()));
        self.baseline.validate(&markdown).expect("the schema prepares")
    }
}

/// md's lifecycle parser's verdict on `frontmatter`.
fn md_parses(frontmatter: &str) -> Result<(), String> {
    let value: serde_json::Value = serde_yaml_ng::from_str(frontmatter).expect("YAML frontmatter");
    darkmatter::lifecycle::md_host::parse_lifecycle(&value, Path::new("doc.md"), None)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Each placement of `action`: its frontmatter and the JSON pointer of the
/// action in it.
fn placements(action: &str) -> [(String, &'static str); 2] {
    [
        (format!("initialize: [{action}, skip]"), "/initialize/0"),
        (format!("composed: [{{ when: 'false', then: [stop], else: [{action}] }}]"), "/composed/0/else/0"),
    ]
}

fn problems(report: &ValidationReport) -> Vec<(String, String)> {
    report.problems.iter().map(|problem| (problem.path.clone(), problem.message.clone())).collect()
}

#[test]
fn every_accepted_cell_passes_schema_validation_and_the_parser() {
    let fixture = Fixture::new();
    for action in ACCEPTED {
        for (frontmatter, _) in placements(action) {
            let frontmatter = format!("fqn: spec.md\n{frontmatter}");
            let report = fixture.validate(&frontmatter);
            assert!(report.valid, "the baseline accepts `{frontmatter}`: {:?}", problems(&report));
            assert_eq!(md_parses(&frontmatter), Ok(()), "md parses `{frontmatter}`");
        }
    }
}

/// The spec's motivating workflow, unedited, is the control row.
#[test]
fn the_motivating_workflow_passes_schema_validation_and_the_parser() {
    let frontmatter = "fqn: \"{{ ctx.cwd }}/specs/2026-11-14-example/spec.md\"\n\
                       initialize:\n  - when: file_exists(fqn)\n    then:\n      - warn: \"The spec already exists: {{ fqn }}\"\n      - skip\n    \
                       else:\n      - edit:\n          file: \"{{ fqn }}\"\n          create_new: true\n      - info: \"Ready: {{ fqn }}\"\n      - skip\n";
    let report = Fixture::new().validate(frontmatter);
    assert!(report.valid, "{:?}", problems(&report));
    assert_eq!(md_parses(frontmatter), Ok(()));
}

#[test]
fn every_refused_cell_fails_schema_validation_at_the_action_and_the_parser() {
    let fixture = Fixture::new();
    for (shape, action) in REFUSED {
        for (frontmatter, pointer) in placements(action) {
            let report = fixture.validate(&frontmatter);
            assert!(!report.valid, "{shape}: the baseline rejects `{frontmatter}`");
            assert!(
                report.problems.iter().all(|problem| problem.path.starts_with(pointer)),
                "{shape}: every problem sits at `{pointer}`: {:?}",
                problems(&report)
            );
            let error = md_parses(&frontmatter).expect_err(&format!("{shape}: md refuses `{frontmatter}`"));
            assert!(error.contains("edit"), "{shape}: the parser names `edit`: {error}");
        }
    }
}
