//! A `$schema` reference is read as the path it spells: `{{ … }}` slots are
//! resolved inside a schema's constraints, never in the reference that names
//! the schema file. `$schema: "./{{ kind }}.yaml"` therefore names a file
//! called `{{ kind }}.yaml`, which a real run fails to load
//! (`composition.schema_load`, exit 1, nothing launched) whether `kind` is a
//! literal or absent, for `compose` and a sequence `prompt:` task alike, and
//! the dry run reports that same refusal (exit 2). The controls spell the
//! file literally: a value the schema admits passes both runs, a value it
//! refuses is the schema finding both report.
//!
//! The real runs use the compiled `claudine-fake-goose`, which appends one
//! line per launch to `PROVIDER_MARKER`.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

const SCHEMA: &str = "kind: schema\n$schema:\n  topic: enum(a; required)\n";

fn staged(name: &str, files: &[(&str, String)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    fs::write(fixture.cwd().join("note.yaml"), SCHEMA).unwrap();
    for (path, text) in files {
        fs::write(fixture.cwd().join(path), text).unwrap();
    }
    fixture
}

fn marker(fixture: &CliProcessFixture) -> PathBuf {
    fixture.cwd().join("provider-launched.txt")
}

fn launches(fixture: &CliProcessFixture) -> usize {
    fs::read_to_string(marker(fixture)).map_or(0, |log| log.lines().count())
}

/// A prompt whose frontmatter is `front`, and which reads `topic`.
fn prompt(front: &str) -> String {
    format!("---\n{front}---\nAbout {{{{ topic }}}}.\n")
}

/// The files and command of `route` running a prompt whose frontmatter is
/// `front`.
fn route(route: &str, front: &str) -> (Vec<(&'static str, String)>, &'static [&'static str]) {
    match route {
        "compose" => (vec![("doc.md", prompt(&format!("agent: goose\n{front}")))], &["compose", "doc.md"]),
        _ => (
            vec![
                ("seq.md", "---\nagent: goose\nsequence:\n  - name: task\n    prompt: doc.md\n---\n".to_string()),
                ("doc.md", prompt(front)),
            ],
            &["sequence", "seq.md"],
        ),
    }
}

/// Runs `front` on `route` dry, then for real, and checks both: `refusal`
/// is the one error code both runs meet (`None`: both pass, the real run
/// launching once).
fn assert_paired(name: &str, which: &str, front: &str, refusal: Option<&str>) {
    let (files, command) = route(which, front);
    let fixture = staged(name, &files);
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker(&fixture))
        .args(command)
        .args(["--dry-run", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    assert_eq!(launches(&fixture), 0, "a dry run launched a provider");
    let rows = envelope["diagnostics"].as_array().unwrap();
    let errors: Vec<&str> =
        rows.iter().filter(|row| row["severity"] == "error").map(|row| row["code"].as_str().unwrap()).collect();
    let unchecked = rows.iter().filter(|row| row["code"] == "dm.check.not_checked").count();
    match refusal {
        Some(code) => {
            assert_eq!(output.status.code(), Some(2), "{envelope:#}");
            assert_eq!(errors, [code], "{envelope:#}");
        }
        None => {
            assert_eq!(output.status.code(), Some(0), "{envelope:#}");
            assert!(errors.is_empty() && unchecked == 0, "{envelope:#}");
        }
    }

    let output = fixture.command().env("PROVIDER_MARKER", marker(&fixture)).args(command).output().unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    match refusal {
        Some(code) => {
            assert_eq!(output.status.code(), Some(1), "{stderr}");
            assert_eq!(launches(&fixture), 0, "{stderr}");
            if code == "composition.schema_load" {
                assert!(stderr.contains("could not resolve $schema reference `./{{ kind }}.yaml`"), "{stderr}");
            }
        }
        None => {
            assert_eq!(output.status.code(), Some(0), "{stderr}");
            assert_eq!(launches(&fixture), 1, "{stderr}");
        }
    }
}

const TEMPLATED_KNOWN: &str = "kind: note\ntopic: a\n$schema: \"./{{ kind }}.yaml\"\n";
const TEMPLATED_ABSENT: &str = "topic: a\n$schema: \"./{{ kind }}.yaml\"\n";
const LITERAL_VALID: &str = "topic: a\n$schema: ./note.yaml\n";
const LITERAL_VIOLATING: &str = "topic: b\n$schema: ./note.yaml\n";

#[test]
fn a_compose_schema_reference_reading_a_known_value_names_no_schema_in_either_run() {
    assert_paired("schema-ref-compose-known", "compose", TEMPLATED_KNOWN, Some("composition.schema_load"));
}

#[test]
fn a_compose_schema_reference_reading_an_absent_value_names_no_schema_in_either_run() {
    assert_paired("schema-ref-compose-absent", "compose", TEMPLATED_ABSENT, Some("composition.schema_load"));
}

#[test]
fn a_compose_literal_schema_reference_admits_a_valid_value() {
    assert_paired("schema-ref-compose-valid", "compose", LITERAL_VALID, None);
}

#[test]
fn a_compose_literal_schema_reference_refuses_a_violating_value() {
    assert_paired("schema-ref-compose-violating", "compose", LITERAL_VIOLATING, Some("dm.schema.constraint"));
}

#[test]
fn a_prompt_task_schema_reference_reading_a_known_value_names_no_schema_in_either_run() {
    assert_paired("schema-ref-task-known", "sequence", TEMPLATED_KNOWN, Some("composition.schema_load"));
}

#[test]
fn a_prompt_task_literal_schema_reference_admits_a_valid_value() {
    assert_paired("schema-ref-task-valid", "sequence", LITERAL_VALID, None);
}

#[test]
fn a_prompt_task_literal_schema_reference_refuses_a_violating_value() {
    assert_paired("schema-ref-task-violating", "sequence", LITERAL_VIOLATING, Some("dm.schema.constraint"));
}
