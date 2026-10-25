//! Container lifecycle regressions through the normal CLI and filesystem boundary.

use std::fs;
use crate::common::{CliProcessFixture, strip_ansi, write_executable};

#[cfg(windows)]
fn install_windows_goose(fixture: &CliProcessFixture, output: &str, action: &str) {
    // Batch stubs cannot receive Goose's multiline prompt argument.
    fs::copy(biscuit_test_harness::bin_exe!("claudine-fake-relay"), fixture.bin_dir().join("goose.exe")).unwrap();
    fs::write(fixture.bin_dir().join("goose.relay.json"), serde_json::json!({
        "program":"cmd", "args":["/D", "/C", action],
        "stderr":fixture.cwd().join("relay-stderr.log"), "stdout":[output],
    }).to_string()).unwrap();
}

fn run(document: &str, dry_run: bool) -> (String, String, i32) {
    let fixture = CliProcessFixture::named("sequence-containers");
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nprintf 'body-result\\n'\n");
    #[cfg(windows)]
    install_windows_goose(&fixture, "body-result", "exit 0");
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
fn nested_sequence_retry_overlay_is_runtime_only_and_clears_abandoned_results() {
    let (stdout, stderr, code) = run(r#"---
ready: false
attempt_note: original
success: [{ stdout: "root={{ seq.outputs }}" }]
sequence:
  - name: nested
    initialize: [{ stdout: nested-initialize }]
    start: [{ stdout: "nested-start={{ ready }}:{{ attempt_note }}" }]
    failure:
      - stdout: nested-failure
      - retry: { max_attempts: 1, with: { ready: true, attempt_note: second } }
    finalize: [{ stdout: "nested-finalize={{ attempt_note }}" }]
    sequence:
      - { name: first, sequence: [] }
      - name: gate
        sequence: []
        start:
          - when: "!ready"
            then: [{ error: not-ready }]
---
"#, false);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), [
        "nested-initialize", "nested-start=false:original", "nested-failure",
        "nested-start=true:second", "nested-finalize=second", "root=[[[],[]]]",
    ]);
}

#[test]
fn nested_group_retry_overlay_reaches_legs_without_persisting_or_duplicate_results() {
    let (stdout, stderr, code) = run(r#"---
ready: false
attempt_note: original
success: [{ stdout: "root={{ seq.outputs }}" }]
sequence:
  - name: nested
    group:
      name: nested
      initialize: [{ stdout: group-initialize }]
      start: [{ stdout: "group-start={{ ready }}:{{ attempt_note }}" }]
      failure:
        - stdout: group-failure
        - retry: { max_attempts: 1, with: { ready: true, attempt_note: second } }
      finalize: [{ stdout: "group-finalize={{ attempt_note }}" }]
      tasks:
        - name: gate
          sequence: []
          start:
            - when: "!ready"
              then: [{ error: not-ready }]
          success: [{ stdout: "leg={{ attempt_note }}" }]
---
"#, false);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), [
        "group-initialize", "group-start=false:original", "group-failure",
        "group-start=true:second", "leg=second", "group-finalize=second", "root=[{\"gate\":[]}]",
    ]);
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
    #[cfg(windows)]
    install_windows_goose(&fixture, "", "echo launched > provider-ran.txt");
    let source = fixture.cwd().join("source.md");
    let document = "---\ninitialize: [{ stdout: forbidden }]\nfinalize: [{ stdout: forbidden }]\nsequence:\n  - name: nested\n    sequence: [{ name: child, shell: 'echo child > shell-ran.txt' }]\n    start: [{ stdout: forbidden }]\n---\n";
    fs::write(&source, document).unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent", "--dry-run"])
        .arg(&source).assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("forbidden"), "{stdout}");
    // A dry run renders `shell:` tasks instead of running them.
    assert!(!fixture.cwd().join("shell-ran.txt").exists());
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
    #[cfg(windows)]
    install_windows_goose(&fixture, "target-result", "exit 0");
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
        if document.frontmatter().as_map().contains_key("sequence") {
            let source = claudine::composition::ResolvedCompositionSource {
                original_ref: path.display().to_string(), resolved_path: path.clone(),
                original_text: document.as_string(), markdown: document.clone(),
            };
            let resolution = biscuit_file::FileResolutionContext::new(path.parent().unwrap());
            let plan = claudine::composition::resolve_sequence_plan(&source, &resolution).unwrap().unwrap();
            let graph = claudine::composition::build_preflight_graph(&plan, &source, &resolution).unwrap();
            assert!(graph.root_lifecycle.is_some(), "{}", path.display());
            assert!(!graph.shell_commands.is_empty(), "shipped executable examples must contribute commands");
        }
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

#[cfg(unix)]
#[test]
fn sibling_failure_terminates_an_opted_prompt_and_refuses_its_finalize_retry() {
    use std::time::Duration;
    let fixture = CliProcessFixture::named("sibling-provider-cancellation");
    fs::write(fixture.cwd().join("prompt.md"), r#"---
start: [{ append_line: [events.log, prompt-start] }]
failure: [{ append_line: [events.log, forbidden-failure] }]
cancelled: [{ append_line: [events.log, "cancelled={{ err.detail.group }}/{{ err.detail.leg }}"] }]
finalize:
  - append_line: [events.log, prompt-finalize]
  - retry: { max_attempts: 1 }
---
Work.
"#).unwrap();
    write_executable(&fixture.bin_dir().join("goose"), r#"#!/bin/sh
cat >/dev/null
printf '%s\n' "$$" > provider.ready
count=0
while [ ! -e provider.release ]; do
  count=$((count + 1))
  [ "$count" -lt 400 ] || exit 90
  sleep 0.01
done
printf 'forbidden completion\n' > provider.completed
"#);
    write_executable(&fixture.cwd().join("fail.sh"), r#"#!/bin/sh
count=0
while [ ! -s provider.ready ]; do
  count=$((count + 1))
  [ "$count" -lt 400 ] || exit 91
  sleep 0.01
done
exit 1
"#);
    let source = fixture.cwd().join("sequence.md");
    fs::write(&source, r#"---
sequence:
  - name: checks
    group:
      name: checks
      tasks:
        - { name: opted, prompt: prompt.md }
        - { name: bad, shell: ./fail.sh }
      failure: [{ append_line: [events.log, "failed={{ as_csv(group.failed) }};cancelled={{ as_csv(group.cancelled) }}"] }]
  - { name: later, shell: echo forbidden }
---
"#).unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent"])
        .arg(&source).timeout(Duration::from_secs(5)).assert().failure().get_output().clone();
    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stderr));
    let events = fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert_eq!(events.lines().collect::<Vec<_>>(), ["prompt-start", "cancelled=checks/bad",
        "prompt-finalize", "failed=bad;cancelled=opted"]);
    let pid: i32 = fs::read_to_string(fixture.cwd().join("provider.ready")).unwrap().trim().parse().unwrap();
    assert_ne!(unsafe { libc::kill(pid, 0) }, 0, "cancelled provider is still alive");
    assert!(!fixture.cwd().join("provider.completed").exists());
    assert!(!fixture.audio_spool().exists());
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("cannot recover a cancelled"), "{stderr}");
    assert!(!stderr.contains("User interrupted"), "{stderr}");
}

#[test]
fn container_shells_use_event_time_values_and_lexical_group_outcomes() {
    use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};
    let fixture = CliProcessFixture::named("container-event-arguments");
    fs::copy(biscuit_test_harness::bin_exe!("claudine-argv-recorder"),
        fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX))).unwrap();
    let source = fixture.cwd().join("source.md");
    let records = fixture.cwd().join("argv.bin");
    fs::write(&source, r#"---
value: before
finalize: [{ shell: 'argv-recorder root-{{ value }}' }]
sequence:
  - name: change
    side_effect: { set: { value: after } }
  - name: nested
    sequence:
      - { name: leaf, shell: echo child }
    success:
      - shell: 'argv-recorder nested-{{ value }}'
      - set: { captured: '$(argv-recorder captured-{{ value }})' }
    finalize: [{ stdout: '{{ captured }}' }]
  - name: checks
    group:
      name: checks
      variables: { label: check }
      tasks:
        - { name: a, shell: 'argv-recorder {{ group.label }}' }
      finalize: [{ shell: 'argv-recorder passed-{{ first(group.succeeded) }}' }]
success: [{ stdout: '{{ seq.outputs }}' }]
---
"#).unwrap();
    let output = fixture.command().env(ARGV_RECORDER_FILE, &records).env("ARGV_RECORDER_STDOUT", "captured\n")
        .args(["sequence", "--goose", "--yolo", "--silent"]).arg(&source).assert().success().get_output().clone();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert!(stdout.contains(r#"{"a":"captured"}"#), "{stdout}");
    assert!(stdout.contains("captured"), "{stdout}");
    assert_eq!(read_argv_records(&records).unwrap(),
        ["nested-after", "captured-after", "check", "passed-a", "root-after"].map(|argument| vec![argument.to_string()]));
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn nested_event_command_is_rejected_before_any_child_effect() {
    let fixture = CliProcessFixture::named("container-event-policy");
    let source = fixture.cwd().join("source.md");
    fs::write(&source, "---\nsequence:\n  - name: nested\n    sequence: [{ name: child, shell: 'echo unsafe > child-ran' }]\n    finalize: [{ shell: 'echo {{ current.model }}' }]\n---\n").unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent"])
        .arg(&source).assert().failure().get_output().clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("current"), "{stderr}");
    assert!(!fixture.cwd().join("child-ran").exists());
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn root_proxy_refuses_a_new_shell_after_approval_closes_even_with_yolo() {
    let fixture = CliProcessFixture::named("container-proxy-approval");
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nprintf should-not-launch > provider-ran\n");
    #[cfg(windows)]
    install_windows_goose(&fixture, "", "echo should-not-launch > provider-ran");
    let source = fixture.cwd().join("source.md");
    fs::write(&source, "---\nfailure: [{ proxy: target.md }]\nsequence: [{ name: fail, shell: exit 9 }]\n---\n").unwrap();
    fs::write(fixture.cwd().join("target.md"), "---\nstart: [{ shell: 'echo forbidden > shell-ran' }]\n---\nTarget.\n").unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent"])
        .arg(&source).assert().failure().get_output().clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("approval closed"), "{stderr}");
    assert!(!fixture.cwd().join("shell-ran").exists());
    assert!(!fixture.cwd().join("provider-ran").exists());
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn container_outcome_collection_cannot_be_used_as_one_shell_argument() {
    let (stdout, stderr, code) = run("---\nsequence:\n  - name: checks\n    group:\n      tasks: [{ name: a, shell: echo ok }]\n      finalize: [{ shell: 'echo passed-{{ group.succeeded }}' }]\nsuccess: [{ stdout: forbidden-publication }]\n---\n", false);
    assert_eq!(code, 1, "{stderr}\n{stdout}");
    assert!(!stdout.contains("passed-") && !stdout.contains("forbidden-publication"), "{stdout}");
}

#[test]
fn root_retry_keeps_approval_closed_when_a_referenced_task_changes() {
    let fixture = CliProcessFixture::named("container-retry-approval");
    let source = fixture.cwd().join("source.md");
    let task = fixture.cwd().join("task.md");
    fs::write(&task, "---\nkind: task\nshell: echo approved\n---\n").unwrap();
    fs::write(&source, r#"---
failure: [{ retry: { max_attempts: 1 } }]
sequence:
  - name: rewrite
    side_effect: { set_frontmatter: [task.md, shell, 'echo forbidden > shell-ran'] }
  - { name: task, task: task.md }
  - { name: fail, shell: exit 9 }
---
"#).unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent"])
        .arg(&source).assert().failure().get_output().clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("approval closed"), "{stderr}");
    assert!(!fixture.cwd().join("shell-ran").exists());
    let changed = fs::read_to_string(&task).unwrap();
    assert!(changed.contains("echo forbidden > shell-ran"), "the source mutation must have occurred: {changed}");
    assert_eq!(fs::read_to_string(&task).unwrap(), changed);
    assert!(!fixture.audio_spool().exists());
}
