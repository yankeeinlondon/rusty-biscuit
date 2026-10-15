//! Container lifecycle regressions through the normal CLI and filesystem boundary.

use std::fs;
use crate::common::{CliProcessFixture, strip_ansi, write_executable};

fn run(document: &str, dry_run: bool) -> (String, String, i32) {
    let fixture = CliProcessFixture::named("sequence-containers");
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nprintf 'body-result\\n'\n");
    let path = fixture.cwd().join("sequence.md");
    fs::write(&path, document).unwrap();
    let mut command = fixture.command();
    command.args(["sequence", "--goose", "--yolo", "--silent"]);
    if dry_run { command.arg("--dry-run"); }
    let output = command.arg(&path).assert().get_output().clone();
    assert_eq!(fs::read_to_string(&path).unwrap(), document, "runtime values must not be written back");
    assert!(!fixture.audio_spool().exists());
    (strip_ansi(&String::from_utf8_lossy(&output.stdout)),
     strip_ansi(&String::from_utf8_lossy(&output.stderr)), output.status.code().unwrap_or(-1))
}

#[test]
fn root_events_wrap_all_children_once_and_expose_native_results() {
    let (stdout, stderr, code) = run(r#"---
initialize: [{ stdout: root-initialize }]
start: [{ stdout: root-start }]
success: [{ stdout: "{{ seq.outputs }}" }]
finalize: [{ stdout: root-finalize }]
sequence:
  - { name: first, shell: echo one }
  - name: nested
    sequence:
      - { name: second, shell: echo two }
      - { name: third, shell: echo three }
  - name: checks
    group:
      name: checks
      tasks:
        - { name: lint, shell: echo clean }
---
"#, false);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains(r#"["one",["two","three"],{"lint":"clean"}]"#), "{stdout}");
    assert_eq!(stdout.matches("root-initialize").count(), 1, "{stdout}");
    assert_eq!(stdout.matches("root-start").count(), 1, "{stdout}");
    assert_eq!(stdout.matches("root-finalize").count(), 1, "{stdout}");
}

#[test]
fn root_initialize_skip_precedes_invalid_static_preflight() {
    let (stdout, stderr, code) = run("---\ninitialize: [{ stdout: entered }, skip]\nfinalize: [{ stdout: forbidden }]\nsequence:\n  - name: missing\n    prompt: missing.md\n---\n", false);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout.trim(), "entered");
}

#[test]
fn preflight_failure_emits_blocked_then_finalize_without_start() {
    let (stdout, stderr, code) = run("---\ninitialize: [{ stdout: entered }]\nstart: [{ stdout: forbidden }]\nblocked: [{ stdout: blocked }]\nfinalize: [{ stdout: cleaned }]\nsequence:\n  - name: missing\n    prompt: missing.md\n---\n", false);
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), ["entered", "blocked", "cleaned"]);
}

#[test]
fn root_retry_retains_writes_clears_abandoned_outputs_and_finalizes_once() {
    let (stdout, stderr, code) = run(r#"---
ready: false
initialize: [{ stdout: initialize }]
start: [{ stdout: start }]
failure:
  - stderr: "{{ err.msg }}"
  - stdout: failure
  - retry:
      max_attempts: 1
      with: { ready: true }
success: [{ stdout: "{{ seq.outputs }}" }]
finalize: [{ stdout: finalize }]
sequence:
  - { name: first, shell: echo first }
  - name: gate
    sequence: []
    start:
      - when: "!ready"
        then: [{ error: not-ready }]
---
"#, false);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    assert_eq!(stdout.matches("initialize").count(), 1, "{stdout}");
    assert_eq!(stdout.matches("start").count(), 2, "{stdout}");
    assert_eq!(stdout.matches("finalize").count(), 1, "{stdout}");
    assert!(stdout.contains("failure\nstart"), "{stdout}");
    assert!(stdout.contains(r#"["first",[]]"#), "{stdout}");
}

#[test]
fn finalize_failure_changes_success_to_failure() {
    let (stdout, stderr, code) = run("---\nfinalize: [{ error: cleanup-failed }]\nsequence: [{ name: first, shell: echo first }]\n---\n", false);
    assert_eq!(code, 1, "{stderr}\n{stdout}");
}

#[test]
fn recursive_dry_run_fires_no_container_events() {
    let fixture = CliProcessFixture::named("sequence-container-dry-run");
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nprintf launched > provider-ran.txt\n");
    let source = fixture.cwd().join("source.md");
    let document = "---\ninitialize: [{ stdout: forbidden }]\nfinalize: [{ stdout: forbidden }]\nsequence:\n  - name: nested\n    sequence: [{ name: child, shell: 'echo child > shell-ran.txt' }]\n    start: [{ stdout: forbidden }]\n---\n";
    fs::write(&source, document).unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent", "--dry-run"])
        .arg(&source).assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("forbidden"), "{stdout}");
    assert_eq!(fs::read_to_string(fixture.cwd().join("shell-ran.txt")).unwrap().trim(), "child");
    assert!(!fixture.cwd().join("provider-ran.txt").exists());
    assert_eq!(fs::read_to_string(&source).unwrap(), document);
    assert!(!fixture.audio_spool().exists());
}


#[test]
fn empty_root_sequence_runs_one_lifecycle_with_empty_outputs() {
    let (stdout, stderr, code) = run("---\ninitialize: [{ stdout: entered }]\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nfinalize: [{ stdout: cleaned }]\nsequence: []\n---\n", false);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), ["entered", "[]", "cleaned"]);
}

#[test]
fn root_resume_reports_typed_no_session_error_and_cleans_up() {
    let (stdout, stderr, code) = run("---\nfailure: [{ resume: try-again }]\nfinalize: [{ stdout: cleaned }]\nsequence: [{ name: fail, shell: exit 9 }]\n---\n", false);
    assert_eq!(code, 1, "{stderr}\n{stdout}");
    assert!(stderr.contains("requires a live provider session"), "{stderr}");
    assert_eq!(stdout.lines().filter(|line| *line == "cleaned").count(), 1, "{stdout}");
}


#[test]
fn body_only_steps_run_root_success_once() {
    let (stdout, stderr, code) = run("---\nsuccess: [{ stdout: root-success }]\nfailure: [{ stderr: \"{{ err.msg }}\" }]\nfinalize: [{ stdout: root-cleanup }]\nsequence: [first, second]\n---\nBody for {{ seq.name }}.\n", false);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    assert_eq!(stdout.matches("root-success").count(), 1, "{stdout}");
    assert_eq!(stdout.matches("root-cleanup").count(), 1, "{stdout}");
    assert_eq!(stdout.matches("body-result").count(), 2, "{stdout}");
}

#[test]
fn blocked_cleanup_cannot_run_shell_commands() {
    let fixture = CliProcessFixture::named("sequence-container-blocked");
    let source = fixture.cwd().join("source.md");
    fs::write(&source, "---\nfinalize: [{ shell: 'echo escaped > escaped.txt' }]\nsequence: [{ name: missing, prompt: missing.md }]\n---\n").unwrap();
    fixture.command().args(["sequence", "--yolo"]).arg(&source).assert().code(1);
    assert!(!fixture.cwd().join("escaped.txt").exists());
}

#[test]
fn root_failure_proxy_runs_target_cleanup_without_source_cleanup() {
    let fixture = CliProcessFixture::named("sequence-container-proxy");
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nprintf 'target-result\\n'\n");
    let source = fixture.cwd().join("source.md");
    fs::write(&source, "---\nfailure: [{ proxy: target.md }]\nfinalize: [{ stdout: source-finalize }]\nsequence: [{ name: bad, shell: exit 9 }]\n---\n").unwrap();
    fs::write(fixture.cwd().join("target.md"), "---\nfinalize: [{ stdout: target-finalize }]\n---\nTarget body.\n").unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent"])
        .arg(&source).assert().success().get_output().clone();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert!(stdout.contains("target-result") && stdout.contains("target-finalize"), "{stdout}");
    assert!(!stdout.contains("source-finalize"), "{stdout}");
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn shipped_container_example_runs_through_the_normal_sequence_command() {
    let example = include_str!("../../../docs/examples/sequence-containers.md");
    let (stdout, stderr, code) = run(example, false);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    assert!(stdout.contains(r#"["outer",["inner"],{"lint":"clean"}]"#), "{stdout}");
}

#[test]
fn shipped_container_example_corpus_has_parseable_lifecycles() {
    let directory = biscuit_test_harness::manifest_dir!().join("../docs/examples");
    let mut files = vec![directory];
    let mut count = 0;
    while let Some(path) = files.pop() {
        if path.is_dir() {
            files.extend(fs::read_dir(&path).unwrap().map(|entry| entry.unwrap().path()));
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "md") { continue; }
        count += 1;
        let document = darkmatter::markdown::Markdown::try_from(path.as_path()).unwrap();
        let root = serde_json::Value::Object(document.frontmatter().as_map().iter()
            .map(|(key,value)| (key.clone(),value.clone())).collect());
        let mut pending = vec![&root];
        while let Some(value) = pending.pop() {
            match value {
                serde_json::Value::Object(map) => {
                    if map.contains_key("sequence") || map.contains_key("tasks") {
                        claudine::composition::parse_lifecycle_config(value, &path).unwrap();
                    }
                    pending.extend(map.values());
                }
                serde_json::Value::Array(values) => pending.extend(values.iter()),
                _ => {}
            }
        }
    }
    assert!(count > 0);
}

#[test]
fn invalid_root_retry_state_still_finalizes_once() {
    let (stdout, stderr, code) = run("---\nfailure: [{ retry: { max_attempts: 1, with: { seq: invalid } } }]\nfinalize: [{ stdout: cleaned }]\nsequence: [{ name: fail, shell: exit 9 }]\n---\n", false);
    assert_eq!(code, 1, "{stderr}\n{stdout}");
    assert_eq!(stdout.lines().filter(|line| *line == "cleaned").count(), 1, "{stdout}");
}
