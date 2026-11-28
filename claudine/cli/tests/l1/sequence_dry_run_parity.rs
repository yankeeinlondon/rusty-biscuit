//! A dry run reports the refusal the real run would stop at, with the same
//! code, and invents none: each case runs the same file once for real
//! (`--yolo`, stdin closed, no provider on `PATH`) and once as
//! `--dry-run --json`.
//!
//! The real run's `err.code` is read from its `finalize` record. Shell tasks
//! only `echo` a marker, which the real run's stdout must not show: a refused
//! sequence runs no task.

use std::fs;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

const FINALIZE: &str = "finalize:\n  - append_line: [\"events.log\", \"{{ err.code }}\"]\n";

struct Real {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    /// `err.code` as the root's `finalize` read it, when it ran.
    err_code: Option<String>,
}

struct Pair {
    real: Real,
    dry_code: Option<i32>,
    /// `(code, severity, file name, line)` of every diagnostic.
    dry: Vec<(String, String, Option<String>, Option<u64>)>,
}

fn pair(root: &str, files: &[(&str, &str)]) -> Pair {
    let fixture = CliProcessFixture::named("sequence-dry-run-parity");
    for (name, text) in files {
        fs::write(fixture.cwd().join(name), text).unwrap();
    }
    let output = fixture.command().args(["sequence", root, "--yolo"]).output().unwrap();
    let err_code = fs::read_to_string(fixture.cwd().join("events.log"))
        .ok()
        .and_then(|text| text.lines().next().map(str::to_string));
    let real = Real {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        err_code,
    };
    let output = fixture.command().args(["sequence", root, "--dry-run", "--json"]).output().unwrap();
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!("dry run JSON:\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
    });
    let dry = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| {
            let file = diagnostic["source"]["file"].as_str().map(|file| {
                std::path::Path::new(file).file_name().unwrap().to_string_lossy().into_owned()
            });
            (
                diagnostic["code"].as_str().unwrap().to_string(),
                diagnostic["severity"].as_str().unwrap().to_string(),
                file,
                diagnostic["source"]["line"].as_u64(),
            )
        })
        .collect();
    assert!(!fixture.audio_spool().exists());
    Pair { real, dry_code: output.status.code(), dry }
}

fn errors(pair: &Pair) -> Vec<(String, Option<String>, Option<u64>)> {
    pair.dry.iter()
        .filter(|(_, severity, _, _)| severity == "error")
        .map(|(code, _, file, line)| (code.clone(), file.clone(), *line))
        .collect()
}

fn not_checked(pair: &Pair) -> usize {
    pair.dry.iter().filter(|(code, ..)| code == "dm.check.not_checked").count()
}

/// The real run failed before any task with `code` reaching `finalize`.
fn assert_refused(pair: &Pair, code: &str) {
    let real = &pair.real;
    assert_eq!(real.code, Some(1), "stderr:\n{}", real.stderr);
    assert_eq!(real.err_code.as_deref(), Some(code), "stderr:\n{}", real.stderr);
    assert!(!real.stdout.contains("task-ran"), "a task ran:\n{}", real.stdout);
    assert_eq!(pair.dry_code, Some(2), "{:?}", pair.dry);
}

/// A Markdown prompt with no `sequence` is refused, not run as a one-task
/// body sequence; the dry run checks no task and asks for no provider.
#[test]
fn a_prompt_without_sequence_is_the_real_runs_refusal() {
    let pair = pair("prompt.md", &[("prompt.md", &format!("---\n{FINALIZE}---\nSummarize the change.\n"))]);
    assert_refused(&pair, "composition.sequence_undefined");
    assert_eq!(errors(&pair), [("composition.sequence_undefined".to_string(), Some("prompt.md".to_string()), None)]);
    assert_eq!(not_checked(&pair), 1, "{:?}", pair.dry);
}

/// A YAML file with no `sequence` is the same refusal.
#[test]
fn a_yaml_file_without_sequence_is_the_real_runs_refusal() {
    let pair = pair("data.yaml", &[("data.yaml", &format!("title: notes\n{FINALIZE}"))]);
    assert_refused(&pair, "composition.sequence_undefined");
    assert_eq!(errors(&pair), [("composition.sequence_undefined".to_string(), Some("data.yaml".to_string()), None)]);
}

/// An empty list is a sequence of no steps: both runs succeed.
#[test]
fn an_empty_sequence_is_clean_in_both_runs() {
    let pair = pair("seq.md", &[("seq.md", "---\nsequence: []\n---\n")]);
    assert_eq!(pair.real.code, Some(0), "stderr:\n{}", pair.real.stderr);
    assert_eq!(pair.dry_code, Some(0), "{:?}", pair.dry);
    assert!(pair.dry.is_empty(), "{:?}", pair.dry);
}

/// The root's own `$schema` refusing an authored value stops the real run;
/// the dry run reports it at the value and leaves the shell task unchecked.
#[test]
fn a_roots_own_schema_failure_is_the_real_runs_refusal() {
    let root = format!(
        "---\n$schema: {{ count: 'number(required)' }}\ncount: abc\n{FINALIZE}sequence:\n  - name: hello\n    shell: echo task-ran\n---\n"
    );
    let pair = pair("seq.md", &[("seq.md", &root)]);
    assert_refused(&pair, "composition.schema_validation");
    assert_eq!(errors(&pair), [("composition.schema_validation".to_string(), Some("seq.md".to_string()), Some(3))]);
}

/// A referenced sequence document whose own values fail its `$schema`
/// stops the real run; the dry run reports it under the task that
/// references it, at the value in the referenced file.
#[test]
fn a_nested_owners_schema_failure_is_the_real_runs_refusal() {
    let child = "---\n$schema: { count: 'number(required)' }\ncount: abc\nsequence:\n  - name: inner\n    shell: echo task-ran\n---\n";
    let root = format!("---\n{FINALIZE}sequence:\n  - name: nested\n    sequence: ./child.md\n---\n");
    let pair = pair("seq.md", &[("seq.md", &root), ("child.md", child)]);
    assert_refused(&pair, "composition.schema_validation");
    assert_eq!(errors(&pair), [("composition.schema_validation".to_string(), Some("child.md".to_string()), Some(3))]);
}

/// A prompt task under a failed owner still has its provider checked (the
/// real run selects providers first) but not its inputs or preview, which
/// read the refused values.
#[test]
fn a_failed_owners_prompt_task_is_not_previewed() {
    let root = format!(
        "---\n$schema: {{ count: 'number(required)' }}\ncount: abc\n{FINALIZE}sequence:\n  - name: ask\n    prompt: ./ask.md\n    with: {{ count: \"{{{{ count }}}}\" }}\n---\n"
    );
    let ask = "---\nagent: goose\n$schema: { count: 'number(required)' }\n---\nCount {{ count }}\n";
    let pair = pair("seq.md", &[("seq.md", &root), ("ask.md", ask)]);
    assert_eq!(pair.real.code, Some(1), "stderr:\n{}", pair.real.stderr);
    assert!(!pair.real.stdout.contains("Count"), "{}", pair.real.stdout);
    let codes: Vec<String> = errors(&pair).into_iter().map(|(code, ..)| code).collect();
    assert_eq!(codes, ["composition.schema_validation", "composition.provider_not_installed"], "{:?}", pair.dry);
    assert!(
        pair.dry.iter().all(|(code, ..)| code != "composition.missing_properties"),
        "inputs are not judged: {:?}", pair.dry
    );
}
