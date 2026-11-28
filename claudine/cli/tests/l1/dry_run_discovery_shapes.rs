//! A real run's approval discovery refuses a directive whose target reads a
//! value that discovery does not evaluate the way the composition does: a
//! document's own frontmatter `topic: "{{ has_alias('zz') }}"` (a shell
//! probe discovery answers `false`) read by `::file ./{{ topic }}.md`. The
//! refusal depends on the document's shape, never on the value, so the dry
//! run reports it as the run's error (`dm.shell.failure`, "dynamic command
//! shape") on every route: `compose`, `inline-compose`, a sequence's root
//! body, and a `prompt:` task. A literal `topic` is the static control both
//! runs pass. A value the caller supplies (a task's `with:`) is data for
//! discovery, so the shape is known and only the value waits: both runs
//! accept the shape, and the dry run leaves the inclusion `not checked`.
//!
//! The real runs use the compiled `claudine-fake-goose`, which appends one
//! line per launch to `PROVIDER_MARKER`.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

const SHAPE: &str = "dm.shell.failure";
const NOT_CHECKED: &str = "dm.check.not_checked";
const PROBE: &str = "{{ has_alias('zz') }}";

fn staged(name: &str, files: &[(&str, String)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    for (path, text) in files {
        fs::write(fixture.cwd().join(path), text).unwrap();
    }
    // Whatever the probe answers, the file it names exists.
    for name in ["zz.md", "true.md", "false.md"] {
        fs::write(fixture.cwd().join(name), "Included.\n").unwrap();
    }
    fixture
}

fn marker(fixture: &CliProcessFixture) -> PathBuf {
    fixture.cwd().join("provider-launched.txt")
}

fn launches(fixture: &CliProcessFixture) -> usize {
    fs::read_to_string(marker(fixture)).map_or(0, |log| log.lines().count())
}

/// The exit code and `(code, severity, message)` rows of a `--json` dry run.
fn dry_run(fixture: &CliProcessFixture, command: &[&str]) -> (Option<i32>, Vec<(String, String, String)>) {
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker(fixture))
        .args(command)
        .args(["--dry-run", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    assert_eq!(launches(fixture), 0, "a dry run launched a provider");
    let rows = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let text = |key: &str| row[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("severity"), text("message"))
        })
        .collect();
    (output.status.code(), rows)
}

fn real_run(fixture: &CliProcessFixture, command: &[&str]) -> (Option<i32>, String) {
    let output = fixture.command().env("PROVIDER_MARKER", marker(fixture)).args(command).output().unwrap();
    (output.status.code(), strip_ansi(&String::from_utf8_lossy(&output.stderr)))
}

/// What the dry run concludes.
enum Dry {
    /// Exit 2 with the one error row the discovery refusal is.
    Refused,
    /// Exit 0 with no error, and the inclusion `not checked`.
    Unchecked,
    /// Exit 0 with no row at all.
    Proven,
}

fn assert_paired(name: &str, files: &[(&str, String)], command: &[&str], dry: Dry, real: i32, launched: usize) {
    let fixture = staged(name, files);
    let (code, rows) = dry_run(&fixture, command);
    let errors: Vec<&(String, String, String)> = rows.iter().filter(|(_, severity, _)| severity == "error").collect();
    match dry {
        Dry::Refused => {
            assert_eq!(code, Some(2), "{rows:#?}");
            assert_eq!(errors.len(), 1, "{rows:#?}");
            assert_eq!(errors[0].0, SHAPE, "{rows:#?}");
            assert!(errors[0].2.starts_with("dynamic command shape"), "{rows:#?}");
            assert!(errors[0].2.contains("frontmatter.topic"), "{rows:#?}");
        }
        Dry::Unchecked => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty(), "{rows:#?}");
            assert!(
                rows.iter().any(|(code, _, message)| code == NOT_CHECKED && message.starts_with("not checked: transclusion")),
                "{rows:#?}"
            );
        }
        Dry::Proven => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(rows.iter().all(|(code, _, _)| code != NOT_CHECKED) && errors.is_empty(), "{rows:#?}");
        }
    }
    let (code, stderr) = real_run(&fixture, command);
    assert_eq!(code, Some(real), "{stderr}");
    assert_eq!(launches(&fixture), launched, "{stderr}");
    if matches!(dry, Dry::Refused) {
        assert!(stderr.contains("dynamic command shape"), "{stderr}");
    }
}

fn compose_doc(topic: &str) -> Vec<(&'static str, String)> {
    vec![("doc.md", format!("---\nagent: goose\ntopic: \"{topic}\"\n---\nHi\n\n::file ./{{{{ topic }}}}.md\n"))]
}

fn inline_doc(topic: &str) -> Vec<(&'static str, String)> {
    vec![("doc.md", format!("---\nagent: goose\ntopic: \"{topic}\"\nprompt: |\n  Hi\n\n  ::file ./{{{{ topic }}}}.md\n---\n"))]
}

fn root_body(topic: &str) -> Vec<(&'static str, String)> {
    vec![(
        "seq.md",
        format!("---\nagent: goose\ntopic: \"{topic}\"\nsequence:\n  - name: body\n---\nHi\n\n::file ./{{{{ topic }}}}.md\n"),
    )]
}

fn prompt_task(topic: &str) -> Vec<(&'static str, String)> {
    vec![
        ("seq.md", "---\nagent: goose\nsequence:\n  - name: task\n    prompt: task.md\n---\n".to_string()),
        ("task.md", format!("---\ntopic: \"{topic}\"\n---\nHi\n\n::file ./{{{{ topic }}}}.md\n")),
    ]
}

#[test]
fn a_compose_target_reading_its_own_shell_probe_value_is_the_discovery_refusal() {
    assert_paired("shape-compose", &compose_doc(PROBE), &["compose", "doc.md"], Dry::Refused, 1, 0);
}

#[test]
fn a_compose_target_reading_a_literal_composes() {
    assert_paired("shape-compose-literal", &compose_doc("zz"), &["compose", "doc.md"], Dry::Proven, 0, 1);
}

/// The inline run delivers its `prompt:` as the body it walks; the stub
/// writes no artifact, so the real run fails after the launch, at its
/// artifact check, which the dry run does not judge.
#[test]
fn an_inline_target_reading_its_own_shell_probe_value_is_the_discovery_refusal() {
    assert_paired("shape-inline", &inline_doc(PROBE), &["inline-compose", "doc.md"], Dry::Refused, 1, 0);
}

#[test]
fn an_inline_target_reading_a_literal_launches() {
    assert_paired("shape-inline-literal", &inline_doc("zz"), &["inline-compose", "doc.md"], Dry::Proven, 1, 1);
}

#[test]
fn a_root_body_target_reading_the_roots_shell_probe_value_is_the_discovery_refusal() {
    assert_paired("shape-root-body", &root_body(PROBE), &["sequence", "seq.md"], Dry::Refused, 1, 0);
}

#[test]
fn a_root_body_target_reading_a_literal_composes() {
    assert_paired("shape-root-body-literal", &root_body("zz"), &["sequence", "seq.md"], Dry::Proven, 0, 1);
}

#[test]
fn a_prompt_task_target_reading_its_own_shell_probe_value_is_the_discovery_refusal() {
    assert_paired("shape-prompt-task", &prompt_task(PROBE), &["sequence", "seq.md"], Dry::Refused, 1, 0);
}

#[test]
fn a_prompt_task_target_reading_a_literal_composes() {
    assert_paired("shape-prompt-task-literal", &prompt_task("zz"), &["sequence", "seq.md"], Dry::Proven, 0, 1);
}

/// The same probe in the task's `with:` reaches the prompt as data, which
/// discovery observes: the shape is known, the value is not until the task
/// runs. The real run evaluates the probe and includes the file it names.
#[test]
fn a_prompt_task_target_reading_a_with_probe_value_is_not_checked() {
    let files = vec![
        (
            "seq.md",
            format!("---\nagent: goose\nsequence:\n  - name: task\n    prompt: task.md\n    with:\n      topic: \"{PROBE}\"\n---\n"),
        ),
        ("task.md", "---\n$schema:\n  topic: string(required)\n---\nHi\n\n::file ./{{ topic }}.md\n".to_string()),
    ];
    assert_paired("shape-prompt-task-with", &files, &["sequence", "seq.md"], Dry::Unchecked, 0, 1);
}
