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
    // A number or boolean target names a file by its text.
    "{ edit: 5 }",
    "{ edit: true }",
    "{ edit: [1] }",
    "{ edit: { file: 5 } }",
    "{ edit: { file: true } }",
    "{ edit: 2.5 }",
    // Boolean-like flag text converts; `null` is no value.
    "{ edit: { file: notes.md, gui: 'yes' } }",
    "{ edit: { file: notes.md, gui: 'true' } }",
    "{ edit: { file: notes.md, gui: null } }",
    "{ edit: { file: notes.md, create_new: 'yes' } }",
    "{ edit: { file: notes.md, create_new: null } }",
    "{ edit: { file: notes.md, no_error: 'yes' } }",
    "{ edit: { file: notes.md, no_error: null } }",
    "{ edit: notes.md, no_error: null }",
    // One placement of the flag, beside or inside the long form.
    "{ edit: { file: notes.md }, no_error: true }",
    "{ edit: { file: notes.md, no_error: true } }",
];

/// Actions both checks refuse, each with the shape it exercises.
const REFUSED: &[(&str, &str)] = &[
    ("short: null", "{ edit: null }"),
    ("short: empty list", "{ edit: [] }"),
    ("short: two items", "{ edit: [a.md, b.md] }"),
    ("short: one wrong element", "{ edit: [a.md, 1] }"),
    ("short: every element wrong", "{ edit: [null] }"),
    ("short: empty string", "{ edit: '' }"),
    ("short: blank string", "{ edit: '   ' }"),
    ("short: blank list element", "{ edit: ['  '] }"),
    ("short: object element", "{ edit: [{ file: a.md }] }"),
    ("long: file absent", "{ edit: { create_new: true } }"),
    ("long: file null", "{ edit: { file: null } }"),
    ("long: file empty", "{ edit: { file: '' } }"),
    ("long: file blank", "{ edit: { file: '  ' } }"),
    ("long: file object", "{ edit: { file: { a: 1 } } }"),
    ("long: gui text", "{ edit: { file: notes.md, gui: maybe } }"),
    ("long: gui number", "{ edit: { file: notes.md, gui: 1 } }"),
    ("long: gui template", "{ edit: { file: notes.md, gui: '{{ true }}' } }"),
    ("long: gui object", "{ edit: { file: notes.md, gui: { a: 1 } } }"),
    ("long: gui list", "{ edit: { file: notes.md, gui: [true] } }"),
    ("long: create_new number", "{ edit: { file: notes.md, create_new: 0 } }"),
    ("long: no_error number", "{ edit: { file: notes.md, no_error: 1 } }"),
    ("long: unknown key", "{ edit: { file: notes.md, bogus: 1 } }"),
    ("long: with", "{ edit: { file: notes.md, with: { a: 1 } } }"),
    ("long form in a bundle", "{ edit: { file: notes.md }, info: hi }"),
    ("null in a bundle", "{ edit: null, info: hi }"),
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

/// Placements of the flag the parser refuses although the schema, which
/// judges each placement alone, admits them: two authored `no_error`s on one
/// action, even equal ones.
const DUPLICATE_FLAGS: &[&str] = &[
    "{ edit: { file: notes.md, no_error: true }, no_error: true }",
    "{ edit: { file: notes.md, no_error: false }, no_error: false }",
    "{ edit: { file: notes.md, no_error: 'yes' }, no_error: false }",
];

/// `frontmatter` with the defaults the baseline's completion reports
/// written in: the value a host parses after schema completion.
fn completed(report: &ValidationReport, frontmatter: &serde_json::Value) -> serde_json::Value {
    let mut map: indexmap::IndexMap<String, serde_json::Value> =
        frontmatter.as_object().expect("a mapping").clone().into_iter().collect();
    for applied in &report.defaults {
        simplified_schema::set_at(&mut map, &applied.path, applied.value.clone()).expect("the default's path exists");
    }
    serde_json::Value::Object(map.into_iter().collect())
}

/// md's parser over `parsed`, told which values were authored.
fn md_parses_completed(
    parsed: &serde_json::Value,
    authored: Option<&serde_json::Value>,
) -> Result<darkmatter::lifecycle::LifecycleConfig, String> {
    let options = darkmatter::lifecycle::LifecycleParseOptions {
        authored_values: authored,
        ..darkmatter::lifecycle::md_host::parse_options(None)
    };
    darkmatter::lifecycle::parse_lifecycle_config_with(parsed, Path::new("doc.md"), &options)
        .map_err(|error| error.to_string())
}

/// Parsing the schema-completed value with its authored source gives the
/// authored parse, cell for cell: a filled `no_error: false` (beside the
/// action and inside its long form, which both schemas fill) never counts as
/// a placement, never meets `then`, and never changes a verdict.
#[test]
fn a_completed_cell_parses_with_its_authored_source_as_the_authored_cell_does() {
    let fixture = Fixture::new();
    let cells = ACCEPTED
        .iter()
        .copied()
        .chain(REFUSED.iter().map(|(_, action)| *action))
        .chain(DUPLICATE_FLAGS.iter().copied());
    let mut filled_inner_and_sibling = false;
    for action in cells {
        for (frontmatter, pointer) in placements(action) {
            let frontmatter = format!("fqn: spec.md\n{frontmatter}");
            let report = fixture.validate(&frontmatter);
            let authored: serde_json::Value = serde_yaml_ng::from_str(&frontmatter).expect("YAML frontmatter");
            let completed = completed(&report, &authored);
            let filled: Vec<String> = report.defaults.iter().map(|applied| applied.path.segments().join("/")).collect();
            if action == "{ edit: { file: notes.md } }"
                && filled.iter().any(|path| path.ends_with("/edit/no_error"))
                && filled.iter().any(|path| path.ends_with("/0/no_error"))
            {
                filled_inner_and_sibling = true;
                // Read as authored, the two filled flags would be two placements.
                let unaware = md_parses_completed(&completed, None).expect_err("two placements without the source");
                assert!(unaware.contains("set both beside `edit` and inside it"), "{unaware}");
            }
            assert_eq!(
                md_parses_completed(&completed, Some(&authored)),
                md_parses_completed(&authored, None),
                "`{frontmatter}` at {pointer}, completed as {completed}"
            );
        }
    }
    assert!(
        filled_inner_and_sibling,
        "the baseline fills `no_error` both beside and inside `{{ edit: {{ file: notes.md }} }}`"
    );
}

#[test]
fn two_authored_flag_placements_pass_the_schema_and_fail_the_parser() {
    let fixture = Fixture::new();
    for action in DUPLICATE_FLAGS {
        for (frontmatter, _) in placements(action) {
            let report = fixture.validate(&frontmatter);
            assert!(report.valid, "each placement alone is a valid flag: {:?}", problems(&report));
            let error = md_parses(&frontmatter).expect_err(&format!("md refuses `{frontmatter}`"));
            assert!(error.contains("set both beside `edit` and inside it"), "{error}");
        }
    }
}
