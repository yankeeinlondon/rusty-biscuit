//! Task `when:` guards and caller values observed through `claudine
//! sequence`: what runs, what is published, and what a guard or a command
//! reads.
//!
//! Tasks are `side_effect:` appends to `trace.log` or `shell:` commands, so
//! nothing launches a provider and the files are the run's observable effect.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

struct Run {
    output: std::process::Output,
    trace: Vec<String>,
    stderr: String,
    fixture: CliProcessFixture,
}

/// Run `claudine sequence --goose --yolo seq.md <setters>` with `files`
/// written into the fixture.
fn run(files: &[(&str, &str)], setters: &[&str]) -> Run {
    let fixture = CliProcessFixture::named("task-guards");
    for (name, text) in files {
        fs::write(fixture.cwd().join(name), text).unwrap();
    }
    let output = fixture
        .command()
        // Not `--silent`: the skipped step's status line is a progress line.
        .args(["sequence", "--goose", "--yolo"])
        .arg(fixture.cwd().join("seq.md"))
        .args(setters)
        .output()
        .unwrap();
    let trace = fs::read_to_string(fixture.cwd().join("trace.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(!fixture.audio_spool().exists());
    Run { output, trace, stderr, fixture }
}

/// A false guard composes nothing, fires none of the task's events, publishes
/// no `seq.outputs` entry, and is not a failure; the step reports `skipped`.
#[test]
fn a_false_guard_runs_nothing_fires_no_event_and_publishes_nothing() {
    let seq = r#"---
sequence:
  - name: first
    side_effect: { append_line: [trace.log, "first"] }
  - name: guarded
    when: "false"
    start:
      - append_line: [trace.log, "guarded start"]
    finalize:
      - append_line: [trace.log, "guarded finalize"]
    sequence:
      - name: inner
        side_effect: { append_line: [trace.log, "inner"] }
  - name: after
    side_effect: { append_line: [trace.log, "outputs {{ length(seq.outputs) }}"] }
---
"#;
    let run = run(&[("seq.md", seq)], &[]);
    assert!(run.output.status.success(), "status {:?}; stderr:\n{}", run.output.status, run.stderr);
    assert_eq!(run.trace, ["first", "outputs 1"], "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("step 2/3 skipped"), "stderr:\n{}", run.stderr);
}

/// The guard is read at the task's turn on every pass of an enclosing loop.
#[test]
fn a_guard_is_reevaluated_on_every_outer_pass() {
    let seq = r#"---
loop:
  until: "loop.count >= 3"
  max: 3
sequence:
  - name: second-pass-only
    when: "loop.count == 2"
    side_effect: { append_line: [trace.log, "ran {{ loop.count }}"] }
  - name: every-pass
    side_effect: { append_line: [trace.log, "pass {{ loop.count }}"] }
---
"#;
    let run = run(&[("seq.md", seq)], &[]);
    assert!(run.output.status.success(), "status {:?}; stderr:\n{}", run.output.status, run.stderr);
    assert_eq!(run.trace, ["pass 1", "ran 2", "pass 2", "pass 3"], "stderr:\n{}", run.stderr);
}

/// A nested sequence's children are guarded at their own turn, with the
/// container's runtime writes visible.
#[test]
fn a_nested_child_guard_reads_earlier_runtime_writes() {
    let seq = r#"---
sequence:
  - name: nested
    sequence:
      - name: mark
        side_effect: { set: { ready: true } }
      - name: when-ready
        when: "ready == true"
        side_effect: { append_line: [trace.log, "ready ran"] }
      - name: when-not-ready
        when: "ready != true"
        side_effect: { append_line: [trace.log, "not-ready ran"] }
      - name: count
        side_effect: { append_line: [trace.log, "outputs {{ length(seq.outputs) }}"] }
---
"#;
    let run = run(&[("seq.md", seq)], &[]);
    assert!(run.output.status.success(), "status {:?}; stderr:\n{}", run.output.status, run.stderr);
    assert_eq!(run.trace, ["ready ran", "outputs 2"], "stderr:\n{}", run.stderr);
}

/// A guard that raises fails its task, located at the guard; it is never
/// read as false, and nothing after it runs. A root step's runtime location
/// reads `tasks[N]`, as every root step's run-time diagnostic does.
#[test]
fn a_guard_that_raises_fails_the_task_at_its_location() {
    let seq = r#"---
sequence:
  - name: first
    side_effect: { append_line: [trace.log, "first"] }
  - name: broken-guard
    when: "nope(1)"
    side_effect: { append_line: [trace.log, "guarded"] }
  - name: after
    side_effect: { append_line: [trace.log, "after"] }
---
"#;
    let run = run(&[("seq.md", seq)], &[]);
    assert!(!run.output.status.success(), "a raising guard fails the run; stderr:\n{}", run.stderr);
    assert_eq!(run.trace, ["first"], "stderr:\n{}", run.stderr);
    assert!(
        run.stderr.contains("task guard `tasks[1].when` could not be evaluated"),
        "stderr:\n{}",
        run.stderr
    );
}

/// Static preflight still judges a task whose guard will be false: the guard
/// cannot hide an invalid task, and nothing runs.
#[test]
fn a_false_guard_does_not_hide_an_invalid_task_from_preflight() {
    let seq = r#"---
sequence:
  - name: first
    side_effect: { append_line: [trace.log, "first"] }
  - name: never
    when: "false"
    prompt: "@missing.md"
---
"#;
    let run = run(&[("seq.md", seq)], &[]);
    assert!(!run.output.status.success(), "preflight must fail; stderr:\n{}", run.stderr);
    assert!(run.trace.is_empty(), "preflight runs nothing; stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("missing.md"), "stderr:\n{}", run.stderr);
}

/// A guard reads caller setters and runtime writes as plain names; a missing
/// name reads as `null`, which is falsy.
#[test]
fn a_guard_reads_caller_values_and_runtime_writes() {
    let seq = r#"---
sequence:
  - name: setter
    side_effect: { set: { flag: true } }
  - name: both
    when: "stage == 'all' && flag == true"
    side_effect: { append_line: [trace.log, "both"] }
  - name: absent
    when: "never_supplied"
    side_effect: { append_line: [trace.log, "absent"] }
---
"#;
    let run = run(&[("seq.md", seq)], &["stage=all"]);
    assert!(run.output.status.success(), "status {:?}; stderr:\n{}", run.output.status, run.stderr);
    assert_eq!(run.trace, ["both"], "stderr:\n{}", run.stderr);
}

const LATE_LEG: &str = r#"---
sequence:
  - name: fan
    group:
      max_parallel: 1
      finalize:
        - append_line: [trace.log, "late {{ group.legs.late.outcome }} [{{ group.legs.late.output }}]"]
      tasks:
        - name: writer
          side_effect: { append_line: [written.txt, "x"] }
        - name: setter
          side_effect: { set: { flag: true } }
        - name: late
          when: "GUARD"
          side_effect: { append_line: [trace.log, "late ran"] }
---
"#;

/// A leg admitted after its siblings finished, under `max_parallel: 1`, reads
/// the group's start snapshot: a sibling's runtime write is invisible to its
/// guard, while a file the sibling wrote is read live.
#[test]
fn a_late_admitted_leg_guard_reads_the_start_snapshot_and_live_files() {
    let seq = LATE_LEG.replace("GUARD", "flag != true && file_exists('written.txt')");
    let run = run(&[("seq.md", &seq)], &[]);
    assert!(run.output.status.success(), "status {:?}; stderr:\n{}", run.output.status, run.stderr);
    assert_eq!(run.trace.len(), 2, "stderr:\n{}", run.stderr);
    assert_eq!(run.trace[0], "late ran", "stderr:\n{}", run.stderr);
    assert!(run.trace[1].starts_with("late succeeded [") && run.trace[1].ends_with("trace.log]"), "{:?}", run.trace);
    assert!(run.fixture.cwd().join("written.txt").exists());
}

/// A leg whose guard is false records `skipped` with a `null` output, and the
/// group still succeeds.
#[test]
fn a_skipped_leg_records_skipped_with_a_null_output() {
    let seq = LATE_LEG.replace("GUARD", "flag == true");
    let run = run(&[("seq.md", &seq)], &[]);
    assert!(run.output.status.success(), "status {:?}; stderr:\n{}", run.output.status, run.stderr);
    assert_eq!(run.trace, ["late skipped []"], "stderr:\n{}", run.stderr);
}

const CALLER_VALUES: &str = r#"---
greeting: hello
sequence:
  - name: first
    shell: "echo first {{ agent }}"
  - name: second
    shell: "echo second {{ agent }} {{ greeting }}"
  - name: third
    side_effect: { append_line: [trace.log, "{{ seq.outputs[0] }}|{{ seq.outputs[1] }}|third {{ agent }} {{ greeting }}"] }
---
"#;

/// A caller setter is a plain name to a `shell:` task, in the first task
/// (approved with its resolved value) and in a later one (approved as a
/// template), and to authored step data beside the document's frontmatter.
// Asserts on what POSIX `sh` `echo` prints.
#[cfg(unix)]
#[test]
fn a_caller_setter_is_read_by_shell_tasks_and_step_data() {
    let run = run(&[("seq.md", CALLER_VALUES)], &["agent=bob"]);
    assert!(run.output.status.success(), "status {:?}; stderr:\n{}", run.output.status, run.stderr);
    let trace: Vec<String> = run.trace.iter().map(|line| line.replace(" |", "|")).collect();
    assert_eq!(trace, ["first bob|second bob hello|third bob hello"], "stderr:\n{}", run.stderr);
}

/// A shell task reading a caller value nobody supplied fails preflight, before
/// any task runs.
#[test]
fn a_missing_caller_value_fails_preflight() {
    let run = run(&[("seq.md", CALLER_VALUES)], &[]);
    assert!(!run.output.status.success(), "a missing value must fail; stderr:\n{}", run.stderr);
    assert!(run.trace.is_empty(), "stderr:\n{}", run.stderr);
    let flat: String = run.stderr.replace('┃', "").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("agent") && flat.contains("which is missing"), "stderr:\n{}", run.stderr);
}

/// A first task's command is approved as its resolved bytes. On a later outer
/// pass a loop action changes a value it reads, and the changed bytes are
/// refused before they run rather than replaced by the approved ones.
// Asserts on what POSIX `sh` `echo` appends.
#[cfg(unix)]
#[test]
fn a_first_task_whose_bytes_change_on_a_later_pass_is_refused() {
    let seq = r#"---
n: 1
loop:
  until: "n >= 3"
  action: "increment(n)"
sequence:
  - name: first
    shell: "echo first {{ n }} >> shell.log"
---
"#;
    let run = run(&[("seq.md", seq)], &[]);
    assert!(!run.output.status.success(), "the changed command must be refused; stderr:\n{}", run.stderr);
    let log = fs::read_to_string(run.fixture.cwd().join("shell.log")).unwrap();
    assert_eq!(log.lines().map(str::trim).collect::<Vec<_>>(), ["first 1"], "stderr:\n{}", run.stderr);
    let flat: String = run.stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("(changed: n)"), "stderr:\n{}", run.stderr);
}

/// A guard on a looping container decides whether the container runs at all
/// on each outer pass; when it runs, its own loop starts from a fresh count.
#[test]
fn a_guarded_nested_loop_runs_zero_times_or_from_a_fresh_count() {
    let seq = r#"---
loop:
  until: "loop.count >= 2"
  max: 2
sequence:
  - name: inner
    when: "loop.count == 2"
    loop:
      until: "loop.count >= 2"
      max: 3
    sequence:
      - name: work
        side_effect: { append_line: [trace.log, "inner {{ loop.count }}"] }
  - name: outer
    side_effect: { append_line: [trace.log, "outer {{ loop.count }}"] }
---
"#;
    let run = run(&[("seq.md", seq)], &[]);
    assert!(run.output.status.success(), "status {:?}; stderr:\n{}", run.output.status, run.stderr);
    assert_eq!(run.trace, ["outer 1", "inner 1", "inner 2", "outer 2"], "stderr:\n{}", run.stderr);
}
