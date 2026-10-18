//! `break` observed through the CLI: which loop it ends, what still runs and
//! is published, how failures and Ctrl-C outrank it, and when its `code`
//! becomes the process exit status.
//!
//! Lifecycle events and tasks append to `trace.log`; prompt documents launch
//! a recording stub provider. Concurrent cases order their legs with marker
//! files, never fixed sleeps.

use std::fs;
use std::time::Duration;

use crate::common::signal::SignalledRun;
use crate::common::{CliProcessFixture, launch_recorder, strip_ansi};

struct Run {
    status: std::process::ExitStatus,
    trace: Vec<String>,
    prompts: Vec<String>,
    stderr: String,
}

impl Run {
    fn code(&self) -> Option<i32> {
        self.status.code()
    }

    fn broken_summaries(&self) -> Vec<&str> {
        self.stderr.lines().map(str::trim).filter(|line| line.contains("loop broken")).collect()
    }
}

/// Run `claudine <command> --claude --yolo <entry>` with `files` written
/// into the fixture.
fn run(command: &str, files: &[(&str, &str)], entry: &str) -> Run {
    let fixture = CliProcessFixture::named("loop-break");
    for (name, text) in files {
        fs::write(fixture.cwd().join(name), text).unwrap();
    }
    let recordings = launch_recorder::install(&fixture, "claude", "printf 'ok\\n'");
    let output = fixture
        .command()
        .args([command, "--claude", "--yolo"])
        .arg(fixture.cwd().join(entry))
        .output()
        .unwrap();
    let trace = fs::read_to_string(fixture.cwd().join("trace.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let prompts = launch_recorder::prompts(&recordings).iter().map(|prompt| prompt.trim().to_string()).collect();
    assert!(!fixture.audio_spool().exists());
    Run { status: output.status, trace, prompts, stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)) }
}

fn assert_trace(run: &Run, expected: &[&str]) {
    assert_eq!(run.trace, expected, "status {:?}; stderr:\n{}", run.status, run.stderr);
}

/// A leg document's `break` ends its group's loop after the group's
/// `finalize`, and the sequence continues with its next step.
#[test]
fn a_leg_document_breaks_its_looping_group_and_the_parent_continues() {
    let leg = r#"---
success:
  - when: "loop.count == 2"
    then:
      - break: "leg done"
finalize:
  - append_line: [trace.log, "leg finalize {{ loop.count }}"]
---
Leg pass {{ loop.count }}
"#;
    let seq = r#"---
sequence:
  - name: fan
    group:
      loop:
        until: "false"
        max: 5
      finalize:
        - append_line: [trace.log, "group finalize {{ loop.count }}"]
      tasks:
        - name: leg
          prompt: "@leg.md"
  - name: after
    side_effect: { append_line: [trace.log, "after"] }
---
"#;
    let run = run("sequence", &[("leg.md", leg), ("seq.md", seq)], "seq.md");
    assert_eq!(run.code(), Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(run.prompts, ["Leg pass 1", "Leg pass 2"]);
    assert_trace(&run, &["leg finalize 1", "group finalize 1", "leg finalize 2", "group finalize 2", "after"]);
    assert_eq!(run.broken_summaries(), ["ℹ loop broken: leg done (code 0)"]);
}

/// A group's `break` ends the looping sequence around it: the rest of that
/// pass does not start, every container between them runs its `finalize`
/// once, and the root loop's `code` is the process exit status.
#[test]
fn a_group_breaks_a_looping_sequence_and_its_code_is_the_exit_status() {
    let seq = r#"---
loop:
  until: "false"
  max: 5
finalize:
  - append_line: [trace.log, "root finalize {{ loop.count }}"]
sequence:
  - name: a
    side_effect: { append_line: [trace.log, "a {{ loop.count }}"] }
  - name: fan
    group:
      tasks:
        - name: x
          side_effect: { append_line: [trace.log, "x {{ loop.count }}"] }
      finalize:
        - append_line: [trace.log, "group finalize {{ loop.count }}"]
        - when: "loop.count == 2"
          then:
            - break: { reason: "done at two", code: 3 }
  - name: c
    side_effect: { append_line: [trace.log, "c {{ loop.count }}"] }
---
"#;
    let run = run("sequence", &[("seq.md", seq)], "seq.md");
    assert_eq!(run.code(), Some(3), "stderr:\n{}", run.stderr);
    assert_trace(
        &run,
        &["a 1", "x 1", "group finalize 1", "c 1", "root finalize 1", "a 2", "x 2", "group finalize 2", "root finalize 2"],
    );
    assert_eq!(run.broken_summaries(), ["ℹ loop broken: done at two (code 3)"]);
}

/// With nested loops only the innermost ends; the outer loop reads its own
/// condition, and the inner `code` stays in the summary.
#[test]
fn nested_loops_end_only_the_innermost_and_its_code_stays_in_the_summary() {
    let seq = r#"---
loop:
  until: "loop.count >= 2"
  max: 3
sequence:
  - name: inner
    loop:
      until: "false"
      max: 5
    finalize:
      - when: "loop.count == 2"
        then:
          - break: { code: 7 }
    sequence:
      - name: work
        side_effect: { append_line: [trace.log, "inner {{ loop.count }}"] }
  - name: outer
    side_effect: { append_line: [trace.log, "outer {{ loop.count }}"] }
---
"#;
    let run = run("sequence", &[("seq.md", seq)], "seq.md");
    assert_eq!(run.code(), Some(0), "an inner break's code is not the exit status; stderr:\n{}", run.stderr);
    assert_trace(&run, &["inner 1", "inner 2", "outer 1", "inner 1", "inner 2", "outer 2"]);
    assert_eq!(run.broken_summaries(), ["ℹ loop broken (code 7)", "ℹ loop broken (code 7)"]);
}

/// A `break` from a document's `initialize` composes and launches nothing,
/// appends nothing, still runs the document's `finalize`, and is not a
/// failure. The looping container publishes its pass's results so far.
#[test]
fn a_break_from_initialize_launches_nothing_appends_nothing_and_finalizes() {
    let doc = "---\ninitialize:\n  - break\nfinalize:\n  - append_line: [trace.log, \"doc finalize\"]\n---\nNever sent\n";
    let seq = r#"---
sequence:
  - name: nested
    loop:
      until: "false"
      max: 3
    sequence:
      - name: doc
        prompt: "@doc.md"
      - name: never
        side_effect: { append_line: [trace.log, "never"] }
  - name: after
    side_effect: { append_line: [trace.log, "outputs {{ length(seq.outputs) }} {{ length(last(seq.outputs)) }}"] }
---
"#;
    let run = run("sequence", &[("doc.md", doc), ("seq.md", seq)], "seq.md");
    assert_eq!(run.code(), Some(0), "stderr:\n{}", run.stderr);
    assert!(run.prompts.is_empty(), "nothing launches: {:?}", run.prompts);
    assert_trace(&run, &["doc finalize", "outputs 1 0"]);
}

/// The same `break` with no loop around it is a typed run-time error.
#[test]
fn a_break_with_no_enclosing_loop_is_a_typed_runtime_error() {
    let doc = "---\nstart:\n  - break\n---\nNever sent\n";
    let run = run("compose", &[("doc.md", doc)], "doc.md");
    assert_eq!(run.code(), Some(1), "stderr:\n{}", run.stderr);
    assert!(run.prompts.is_empty());
    let prose: String = run.stderr.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(prose.contains("lifecycle `break` at `start[0].break` fired outside any loop"), "stderr:\n{}", run.stderr);
}

/// A document's own loop is the nearest one: a `break` in a pass ends it, and
/// under `compose` (the root invocation) its code is the exit status.
#[test]
fn a_looping_document_break_ends_its_loop_with_the_root_exit_code() {
    let doc = r#"---
n: 1
loop:
  until: "false"
  max: 5
  action: "increment(n)"
success:
  - when: "n == 2"
    then:
      - break: { reason: "second", code: 9 }
finalize:
  - append_line: [trace.log, "finalize {{ n }}"]
---
Iteration {{ n }}
"#;
    let run = run("compose", &[("doc.md", doc)], "doc.md");
    assert_eq!(run.code(), Some(9), "stderr:\n{}", run.stderr);
    assert_eq!(run.prompts, ["Iteration 1", "Iteration 2"]);
    assert_trace(&run, &["finalize 1", "finalize 2"]);
    assert_eq!(run.broken_summaries(), ["ℹ loop broken: second (code 9)"]);
}

/// A break from `finalize`, or from the loop gate, never runs cleanup again
/// and ends the loop before its condition.
#[test]
fn breaks_from_finalize_and_the_gate_clean_up_exactly_once() {
    let from_finalize = r#"---
loop:
  until: "false"
  max: 3
finalize:
  - append_line: [trace.log, "finalize {{ loop.count }}"]
  - when: "loop.count == 1"
    then: [break]
sequence:
  - name: a
    side_effect: { append_line: [trace.log, "a {{ loop.count }}"] }
---
"#;
    let run_finalize = run("sequence", &[("seq.md", from_finalize)], "seq.md");
    assert_eq!(run_finalize.code(), Some(0), "stderr:\n{}", run_finalize.stderr);
    assert_trace(&run_finalize, &["a 1", "finalize 1"]);

    let from_gate = r#"---
loop:
  until: "false"
  max: 3
  gate:
    - append_line: [trace.log, "gate {{ loop.count }}"]
    - when: "loop.count == 2"
      then: [break]
finalize:
  - append_line: [trace.log, "finalize {{ loop.count }}"]
sequence:
  - name: a
    side_effect: { append_line: [trace.log, "a {{ loop.count }}"] }
---
"#;
    let run_gate = run("sequence", &[("seq.md", from_gate)], "seq.md");
    assert_eq!(run_gate.code(), Some(0), "stderr:\n{}", run_gate.stderr);
    assert_trace(&run_gate, &["a 1", "finalize 1", "gate 1", "a 2", "finalize 2", "gate 2"]);
}

/// A broken pass publishes the work it completed: each container between the
/// break and its loop publishes its partial result once, with no entry for a
/// child that never ran and no synthetic break result.
#[test]
fn a_broken_pass_publishes_its_completed_partial_results() {
    let seq = r#"---
sequence:
  - name: nested
    loop:
      until: "false"
      max: 3
    sequence:
      - name: a
        side_effect: { append_line: [trace.log, "a"] }
      - name: inner
        finalize:
          - break
        sequence:
          - name: b
            side_effect: { append_line: [trace.log, "b"] }
      - name: c
        side_effect: { append_line: [trace.log, "c"] }
  - name: after
    side_effect: { append_line: [trace.log, "nested {{ length(last(seq.outputs)) }} inner {{ length(last(last(seq.outputs))) }}"] }
---
"#;
    let run = run("sequence", &[("seq.md", seq)], "seq.md");
    assert_eq!(run.code(), Some(0), "stderr:\n{}", run.stderr);
    assert_trace(&run, &["a", "b", "nested 2 inner 1"]);
}

/// The concurrent-group cases: a root loop around a group whose legs break.
fn group_seq(group_head: &str, legs: &str) -> String {
    format!(
        "---\nloop:\n  until: \"false\"\n  max: 3\nfinalize:\n  - append_line: [trace.log, \"root finalize {{{{ loop.count }}}}\"]\nsequence:\n  - name: fan\n    group:\n{group_head}      tasks:\n{legs}---\n"
    )
}

/// An inline sequence leg whose `finalize` records `name` and breaks with
/// `code`, after running `body` (YAML list items at leg-child depth).
fn breaking_leg(name: &str, code: u8, body: &str) -> String {
    format!(
        "        - name: {name}\n          finalize:\n            - append_line: [trace.log, \"{name} finalize\"]\n            - break: {{ reason: {name}, code: {code} }}\n          sequence:\n{body}"
    )
}

/// A pending break waits for every leg, and the group ends its loop with the
/// first breaking leg in declaration order, whatever order the legs finish
/// in: `first` waits until `second` has finished.
#[test]
fn a_pending_break_waits_for_every_leg_and_takes_the_first_in_declaration_order() {
    let legs = [
        breaking_leg("first", 4, "            - name: wait\n              shell: \"until [ -f second.done ]; do sleep 0.02; done\"\n"),
        breaking_leg("second", 5, "            - name: mark\n              side_effect: { append_line: [second.done, x] }\n"),
        "        - name: third\n          side_effect: { append_line: [trace.log, third] }\n".to_string(),
    ]
    .concat();
    let run = run("sequence", &[("seq.md", &group_seq("", &legs))], "seq.md");
    assert_eq!(run.code(), Some(4), "stderr:\n{}", run.stderr);
    let mut trace = run.trace.clone();
    trace.sort();
    assert_eq!(trace, ["first finalize", "root finalize 1", "second finalize", "third"], "stderr:\n{}", run.stderr);
    assert_eq!(run.trace.last().map(String::as_str), Some("root finalize 1"));
    assert_eq!(run.broken_summaries(), ["ℹ loop broken: first (code 4)"]);
}

/// A pending break stops no admission: legs still queued behind
/// `max_parallel: 1` run after the breaking leg finished.
#[test]
fn queued_legs_still_run_after_a_pending_break() {
    let legs = [
        breaking_leg("first", 6, "            - name: work\n              side_effect: { append_line: [trace.log, first] }\n"),
        "        - name: second\n          side_effect: { append_line: [trace.log, second] }\n".to_string(),
        "        - name: third\n          side_effect: { append_line: [trace.log, third] }\n".to_string(),
    ]
    .concat();
    let run = run("sequence", &[("seq.md", &group_seq("      max_parallel: 1\n", &legs))], "seq.md");
    assert_eq!(run.code(), Some(6), "stderr:\n{}", run.stderr);
    assert_trace(&run, &["first", "first finalize", "second", "third", "root finalize 1"]);
}

/// An unhandled sibling failure beats a pending break: the failure is the
/// outcome, and no loop reports broken.
#[test]
fn an_unhandled_sibling_failure_beats_a_pending_break() {
    let legs = [
        breaking_leg("first", 4, "            - name: work\n              side_effect: { append_line: [trace.log, first] }\n"),
        "        - name: bad\n          shell: \"exit 3\"\n".to_string(),
    ]
    .concat();
    let run = run("sequence", &[("seq.md", &group_seq("", &legs))], "seq.md");
    assert_eq!(run.code(), Some(1), "stderr:\n{}", run.stderr);
    assert!(run.broken_summaries().is_empty(), "stderr:\n{}", run.stderr);
    assert!(run.trace.contains(&"root finalize 1".to_string()), "{:?}", run.trace);
}

/// A leg failure the group handles with `fail_fast: false` is not an
/// unhandled failure, so the pending break still ends the loop.
#[test]
fn a_handled_leg_failure_keeps_the_pending_break() {
    let legs = [
        breaking_leg("first", 4, "            - name: work\n              side_effect: { append_line: [trace.log, first] }\n"),
        "        - name: bad\n          shell: \"exit 3\"\n".to_string(),
    ]
    .concat();
    let run = run("sequence", &[("seq.md", &group_seq("      fail_fast: false\n", &legs))], "seq.md");
    assert_eq!(run.code(), Some(4), "stderr:\n{}", run.stderr);
    assert_eq!(run.broken_summaries(), ["ℹ loop broken: first (code 4)"]);
}

/// A failure in the cleanup a `break` runs beats the break.
#[test]
fn a_failed_finalize_during_a_break_beats_it() {
    let seq = r#"---
loop:
  until: "false"
  max: 3
finalize:
  - append_line: [trace.log, "root finalize"]
sequence:
  - name: inner
    start: [break]
    finalize:
      - error: "cleanup refused"
    sequence:
      - name: never
        side_effect: { append_line: [trace.log, "never"] }
---
"#;
    let run = run("sequence", &[("seq.md", seq)], "seq.md");
    assert_eq!(run.code(), Some(1), "stderr:\n{}", run.stderr);
    assert!(run.broken_summaries().is_empty(), "stderr:\n{}", run.stderr);
    assert_trace(&run, &["root finalize"]);
}

/// Ctrl-C overrides a pending break: cancellation cleanup runs and the run
/// exits 130 rather than ending the loop successfully.
#[test]
fn ctrl_c_overrides_a_pending_break() {
    let fixture = CliProcessFixture::named("loop-break-interrupt");
    let legs = [
        breaking_leg("first", 4, "            - name: mark\n              side_effect: { append_line: [first.done, x] }\n"),
        "        - name: waiter\n          shell: \"until [ -f never.done ]; do sleep 0.05; done\"\n".to_string(),
    ]
    .concat();
    fs::write(fixture.cwd().join("seq.md"), group_seq("", &legs)).unwrap();
    let mut command = fixture.command_std();
    command.args(["sequence", "--claude", "--yolo"]).arg(fixture.cwd().join("seq.md"));
    let mut run = SignalledRun::spawn(command, fixture.workspace_path());
    // The breaking leg has settled, so its break is pending while the
    // waiter still runs.
    run.wait_for_path(&fixture.cwd().join("trace.log"), Duration::from_secs(20));
    run.interrupt();
    let exit = run.wait_for_exit(Duration::from_secs(15));
    assert_eq!(exit.status.code(), Some(130), "stderr:\n{}", exit.stderr);
    assert!(!exit.stderr.contains("loop broken"), "stderr:\n{}", exit.stderr);
    assert!(!fixture.audio_spool().exists());
}
