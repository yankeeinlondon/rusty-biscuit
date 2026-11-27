//! A file the real command would ask the caller to choose is a pending
//! choice under `--dry-run`, never a miss and never a chooser.
//!
//! Two real routes recover a reference that names no file by asking: the
//! root argument's picker (a bare name such as `spec.md`, found in the
//! user's prompts folder) and the supplied eager-file chooser
//! (`spec=spec` against `file(required; eager; match(deep/*.md))`). The dry
//! run runs each route's read-only search and reports
//! `composition.file_choice_pending` (a warning) where the route would ask
//! and found files, and the route's own refusal (an error) where it would
//! not ask: redirected streams, `--silent` for the input chooser, an
//! anchored reference, no candidate, and every sequence input, which a
//! sequence never offers a chooser for.
//!
//! The terminal is injected through the `test-fixtures` seam
//! `CLAUDINE_TEST_DRY_RUN_TERMINAL`: no terminal is opened, nothing gains
//! focus, and stdin is closed, so a chooser that opened would fail the run
//! instead of waiting.

use std::fs;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

/// Both streams graded as terminals.
const TERMINAL: &str = "stdin,stderr";
/// Neither stream graded as a terminal: a redirected run.
const REDIRECTED: &str = "";

const EAGER: &str = "---\nagent: claude\n$schema:\n  spec: file(required;eager;match(deep/*.md))\n---\nReview {{ spec }}.\n";
const EAGER_INLINE: &str =
    "---\nagent: claude\n$schema:\n  spec: file(required;eager;match(deep/*.md))\nprompt: Review {{ spec }}.\n---\n";

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Run {
    /// The `--json` report's findings as `(code, severity)`.
    fn rows(&self) -> Vec<(String, String)> {
        let report: Value =
            serde_json::from_str(&self.stdout).unwrap_or_else(|_| panic!("not JSON:\n{}\n{}", self.stdout, self.stderr));
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| (row["code"].as_str().unwrap().to_string(), row["severity"].as_str().unwrap().to_string()))
            .collect()
    }

    fn row(&self, code: &str) -> Value {
        let report: Value = serde_json::from_str(&self.stdout).unwrap();
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["code"] == code)
            .cloned()
            .unwrap_or_else(|| panic!("no `{code}` row:\n{}", self.stdout))
    }
}

fn run(fixture: &CliProcessFixture, terminal: &str, args: &[&str]) -> Run {
    let output = fixture
        .command()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_TEST_DRY_RUN_TERMINAL", terminal)
        .args(args)
        .output()
        .unwrap();
    assert!(!fixture.audio_spool().exists());
    Run {
        code: output.status.code(),
        stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    }
}

/// A fixture with a `claude` the check only looks up, a document for each
/// mode in the user's prompts folder, and `deep/spec.md` beside the launch
/// directory's eager documents.
fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    let path = fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX));
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let prompts = fixture.home().join(".claudine/prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(prompts.join("spec.md"), "---\nagent: claude\n---\nHello\n").unwrap();
    fs::write(prompts.join("brief.md"), "---\nagent: claude\nprompt: Hello\n---\n").unwrap();
    fs::write(prompts.join("flow.md"), "---\nagent: claude\nsequence:\n  - name: a\n    prompt: ./spec.md\n---\n")
        .unwrap();
    fs::create_dir_all(fixture.cwd().join("deep")).unwrap();
    fs::write(fixture.cwd().join("deep/spec.md"), "The spec.\n").unwrap();
    fs::write(fixture.cwd().join("eager.md"), EAGER).unwrap();
    fs::write(fixture.cwd().join("inline.md"), EAGER_INLINE).unwrap();
    fixture
}

fn pending() -> (String, String) {
    ("composition.file_choice_pending".into(), "warning".into())
}

/// R1-F13.1: a bare name each mode's picker would offer a file for is a
/// pending choice naming the candidate, in JSON and plain alike; nothing is
/// asked.
#[test]
fn a_bare_compose_root_with_a_candidate_is_a_pending_choice() {
    let fixture = fixture("dry-run-choice-compose");
    let json = run(&fixture, TERMINAL, &["compose", "spec.md", "--dry-run", "--json"]);
    assert_eq!(json.code, Some(2), "{}", json.stderr);
    assert_eq!(json.rows(), [pending()]);
    assert_eq!(json.row("composition.file_choice_pending")["notes"], serde_json::json!(["~/.claudine/prompts/spec.md"]));
    let human = run(&fixture, TERMINAL, &["compose", "spec.md", "--dry-run"]);
    assert!(human.stderr.contains("⚠ `spec.md` names no file, so an interactive run asks which file to use"), "{}", human.stderr);
    assert!(human.stdout.is_empty(), "nothing is previewed or asked:\n{}", human.stdout);
}

#[test]
fn a_bare_inline_compose_root_with_a_candidate_is_a_pending_choice() {
    let fixture = fixture("dry-run-choice-inline");
    let json = run(&fixture, TERMINAL, &["inline-compose", "brief.md", "--dry-run", "--json"]);
    assert_eq!(json.code, Some(2), "{}", json.stderr);
    assert_eq!(json.rows(), [pending()]);
    assert_eq!(json.row("composition.file_choice_pending")["notes"], serde_json::json!(["~/.claudine/prompts/brief.md"]));
}

#[test]
fn a_bare_sequence_root_with_a_candidate_is_a_pending_choice() {
    let fixture = fixture("dry-run-choice-sequence");
    let json = run(&fixture, TERMINAL, &["sequence", "flow.md", "--dry-run", "--json"]);
    assert_eq!(json.code, Some(2), "{}", json.stderr);
    assert_eq!(json.rows(), [pending()]);
    assert_eq!(json.row("composition.file_choice_pending")["notes"], serde_json::json!(["~/.claudine/prompts/flow.md"]));
}

/// A redirected run cannot ask, so its picker refuses: the miss is an
/// error carrying that refusal. `--silent` does not stop the picker, so on a
/// terminal it stays a pending choice.
#[test]
fn a_bare_root_on_a_redirected_run_is_the_pickers_refusal() {
    let fixture = fixture("dry-run-choice-redirected");
    let redirected = run(&fixture, REDIRECTED, &["compose", "spec.md", "--dry-run", "--json"]);
    assert_eq!(redirected.rows(), [("dm.reference.missing_document".into(), "error".into())]);
    let notes = redirected.row("dm.reference.missing_document")["notes"].to_string();
    assert!(notes.contains("autocomplete requires an interactive terminal"), "{notes}");
    let silent = run(&fixture, TERMINAL, &["compose", "spec.md", "--silent", "--dry-run", "--json"]);
    assert_eq!(silent.rows(), [pending()]);
}

/// R1-F13.4: the exact paths resolve without asking and check clean.
#[test]
fn exact_root_and_input_references_check_clean() {
    let fixture = fixture("dry-run-choice-exact");
    let root = fixture.home().join(".claudine/prompts/spec.md");
    let exact_root = run(&fixture, TERMINAL, &["compose", root.to_str().unwrap(), "--dry-run", "--json"]);
    assert_eq!(exact_root.code, Some(0), "{}", exact_root.stdout);
    assert_eq!(exact_root.rows(), Vec::<(String, String)>::new());
    let exact_input = run(&fixture, TERMINAL, &["compose", "eager.md", "spec=deep/spec.md", "--dry-run", "--json"]);
    assert_eq!(exact_input.code, Some(0), "{}", exact_input.stdout);
    assert_eq!(exact_input.rows(), Vec::<(String, String)>::new());
}

/// R1-F13.5: an anchored reference never reaches the picker, and a bare
/// name with no candidate is the picker's no-match: both stay errors.
#[test]
fn anchored_misses_and_bare_names_without_a_candidate_stay_errors() {
    let fixture = fixture("dry-run-choice-misses");
    let anchored = run(&fixture, TERMINAL, &["compose", "./spec.md", "--dry-run", "--json"]);
    assert_eq!(anchored.rows(), [("dm.reference.missing_document".into(), "error".into())]);
    assert!(anchored.row("dm.reference.missing_document")["notes"].as_array().is_none_or(Vec::is_empty));
    let unmatched = run(&fixture, TERMINAL, &["compose", "nomatch.md", "--dry-run", "--json"]);
    assert_eq!(unmatched.rows(), [("dm.reference.missing_document".into(), "error".into())]);
    let notes = unmatched.row("dm.reference.missing_document")["notes"].to_string();
    assert!(notes.contains("no files matched autocomplete query `nomatch.md`"), "{notes}");
}

/// R1-F13.2: a supplied eager partial the chooser would offer a file for is
/// a pending choice on `compose`; what reads it stays as written.
#[test]
fn a_supplied_eager_partial_is_a_pending_choice_on_compose() {
    let fixture = fixture("dry-run-choice-eager-compose");
    let json = run(&fixture, TERMINAL, &["compose", "eager.md", "spec=spec", "--dry-run", "--json"]);
    assert_eq!(json.code, Some(2), "{}", json.stderr);
    assert_eq!(json.rows(), [pending()]);
    assert_eq!(json.row("composition.file_choice_pending")["notes"], serde_json::json!(["deep/spec.md"]));
    let human = run(&fixture, TERMINAL, &["compose", "eager.md", "spec=spec", "--dry-run"]);
    assert_eq!(human.stdout.trim(), "Review {{ spec }} [missing input].", "{}", human.stderr);
}

#[test]
fn a_supplied_eager_partial_is_a_pending_choice_on_inline_compose() {
    let fixture = fixture("dry-run-choice-eager-inline");
    let json = run(&fixture, TERMINAL, &["inline-compose", "inline.md", "spec=spec", "--dry-run", "--json"]);
    assert_eq!(json.code, Some(2), "{}", json.stderr);
    assert_eq!(json.rows(), [pending()]);
}

/// Where the chooser would not open (redirected, `--silent`) the partial is
/// the real run's schema refusal.
#[test]
fn a_supplied_eager_partial_the_run_cannot_ask_about_is_its_refusal() {
    let fixture = fixture("dry-run-choice-eager-refused");
    let refusal = [("dm.schema.invalid_file_reference".to_string(), "error".to_string())];
    let redirected = run(&fixture, REDIRECTED, &["compose", "eager.md", "spec=spec", "--dry-run", "--json"]);
    assert_eq!(redirected.rows(), refusal);
    let silent = run(&fixture, TERMINAL, &["compose", "eager.md", "spec=spec", "--silent", "--dry-run", "--json"]);
    assert_eq!(silent.rows(), refusal);
}

/// A sequence offers no chooser for its root's supplied file: the partial
/// is the run's refusal even on a terminal, where it was not reported at
/// all before.
#[test]
fn a_sequence_roots_supplied_partial_is_its_refusal_on_a_terminal() {
    let fixture = fixture("dry-run-choice-eager-sequence");
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nagent: claude\n$schema:\n  spec: file(required;eager;match(deep/*.md))\nsequence:\n  - name: a\n    prompt: ./plain.md\n---\n",
    )
    .unwrap();
    fs::write(fixture.cwd().join("plain.md"), "---\nagent: claude\n---\nplain\n").unwrap();
    let partial = run(&fixture, TERMINAL, &["sequence", "seq.md", "spec=spec", "--dry-run", "--json"]);
    assert_eq!(partial.rows(), [("composition.schema_validation".into(), "error".into())]);
    let exact = run(&fixture, TERMINAL, &["sequence", "seq.md", "spec=deep/spec.md", "--dry-run", "--json"]);
    assert_eq!(exact.code, Some(0), "{}", exact.stdout);
}

/// R1-F13.3: a sequence task's input is prepared without a chooser, so its
/// partial is the run's refusal on a terminal; the exact path checks clean.
#[test]
fn a_sequence_task_input_partial_is_its_refusal_on_a_terminal() {
    let fixture = fixture("dry-run-choice-task");
    let task = |value: &str| {
        format!("---\nagent: claude\nsequence:\n  - name: one\n    prompt: ./eager.md\n    with:\n      spec: {value}\n---\n")
    };
    fs::write(fixture.cwd().join("partial.md"), task("spec")).unwrap();
    fs::write(fixture.cwd().join("exact.md"), task("deep/spec.md")).unwrap();
    let partial = run(&fixture, TERMINAL, &["sequence", "partial.md", "--dry-run", "--json"]);
    assert_eq!(partial.rows(), [("dm.schema.invalid_file_reference".into(), "error".into())]);
    let exact = run(&fixture, TERMINAL, &["sequence", "exact.md", "--dry-run", "--json"]);
    assert_eq!(exact.code, Some(0), "{}", exact.stdout);
}
