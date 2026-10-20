//! One loop on every primitive, observed through the CLI: the prompts a
//! recording provider receives and the exit status.
//!
//! The counting document repeats wherever it runs: directly, as a sequence
//! step, and as a group leg. Containers repeat by their own `loop:`.

use std::fs;

use crate::common::{CliProcessFixture, launch_recorder, strip_ansi};

/// The counting document: three passes, ended by its own condition.
const COUNT: &str = "---\nn: 1\nloop:\n  until: \"n >= 3\"\n  action: \"increment(n)\"\n---\nIteration {{ n }}\n";

/// Run `claudine <command> --claude --yolo --silent <file>` with `files`
/// written into the fixture, returning the trimmed prompts in launch order
/// and the process output.
fn run(command: &str, files: &[(&str, &str)], entry: &str) -> (Vec<String>, std::process::Output) {
    let fixture = CliProcessFixture::named("loop-primitives");
    for (name, text) in files {
        fs::write(fixture.cwd().join(name), text).unwrap();
    }
    let recordings = launch_recorder::install(&fixture, "claude", "printf 'ok %s\\n' \"$n\"");
    let output = fixture
        .command()
        .args([command, "--claude", "--yolo", "--silent"])
        .arg(fixture.cwd().join(entry))
        .output()
        .unwrap();
    let prompts = launch_recorder::prompts(&recordings).iter().map(|prompt| prompt.trim().to_string()).collect();
    (prompts, output)
}

fn assert_counts_to_three(prompts: &[String], output: &std::process::Output) {
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "status {:?}; stderr:\n{stderr}", output.status);
    assert_eq!(prompts, ["Iteration 1", "Iteration 2", "Iteration 3"], "stderr:\n{stderr}");
}

#[test]
fn a_looping_document_runs_three_passes_under_compose() {
    let (prompts, output) = run("compose", &[("count.md", COUNT)], "count.md");
    assert_counts_to_three(&prompts, &output);
}

#[test]
fn a_looping_document_runs_three_passes_as_the_only_sequence_step() {
    let seq = "---\nsequence:\n  - name: counting\n    prompt: \"@count.md\"\n---\n";
    let (prompts, output) = run("sequence", &[("count.md", COUNT), ("seq.md", seq)], "seq.md");
    assert_counts_to_three(&prompts, &output);
}

#[test]
fn a_looping_document_runs_three_passes_as_the_only_group_leg() {
    let seq = "---\nsequence:\n  - name: batch\n    group:\n      tasks:\n        - name: counting\n          prompt: \"@count.md\"\n---\n";
    let (prompts, output) = run("sequence", &[("count.md", COUNT), ("seq.md", seq)], "seq.md");
    assert_counts_to_three(&prompts, &output);
}

/// Run `claudine sequence` on `seq.md`, returning the recorded event lines
/// and the process output. Shell tasks only: nothing is launched.
fn run_sequence_events(seq: &str, extra: &[&str]) -> (Vec<String>, std::process::Output) {
    let fixture = CliProcessFixture::named("loop-primitives");
    fs::write(fixture.cwd().join("seq.md"), seq).unwrap();
    let recordings = launch_recorder::install(&fixture, "claude", "exit 0");
    let output = fixture
        .command()
        .args(["sequence", "--claude", "--yolo", "--silent"])
        .args(extra)
        .arg(fixture.cwd().join("seq.md"))
        .output()
        .unwrap();
    assert!(launch_recorder::launches(&recordings).is_empty(), "shell-only sequences launch no provider");
    let events = fs::read_to_string(fixture.cwd().join("events.log")).unwrap_or_default();
    (events.lines().map(str::to_string).collect(), output)
}

fn stderr_of(output: &std::process::Output) -> String {
    strip_ansi(&String::from_utf8_lossy(&output.stderr))
}

/// A looping three-step sequence: one `initialize`, then `start`, `success`,
/// `finalize`, and the gate per pass, with `seq.outputs` growing flat across
/// passes while `loop.last_output` holds only the pass's own array.
#[test]
fn a_looping_sequence_runs_three_passes_with_per_pass_events_and_flat_outputs() {
    let seq = r#"---
loop:
  until: "loop.count >= 3"
  max: 3
  gate:
    - append_line: ["events.log", "gate {{ loop.count }} {{ length(loop.last_output) }} {{ length(seq.outputs) }} {{ last(seq.outputs) }}"]
initialize:
  - append_line: ["events.log", "initialize {{ loop.count }} {{ loop.is_first }} {{ loop.is_last }}"]
start:
  - append_line: ["events.log", "start {{ loop.count }} {{ loop.is_first }} {{ loop.is_last }} {{ length(loop.last_output) }}"]
success:
  - append_line: ["events.log", "success {{ loop.count }} {{ length(loop.last_output) }}"]
finalize:
  - append_line: ["events.log", "finalize {{ loop.count }}"]
sequence:
  - name: a
    shell: "echo a{{ loop.count }}"
  - name: b
    shell: "echo b{{ loop.count }}"
  - name: c
    shell: "echo c{{ loop.count }}"
---
"#;
    let (events, output) = run_sequence_events(seq, &[]);
    assert!(output.status.success(), "status {:?}; stderr:\n{}", output.status, stderr_of(&output));
    assert_eq!(
        events,
        [
            "initialize 1 true false",
            "start 1 true false 0",
            "success 1 3",
            "finalize 1",
            "gate 1 3 3 c1",
            "start 2 false false 3",
            "success 2 3",
            "finalize 2",
            "gate 2 3 6 c2",
            "start 3 false true 3",
            "success 3 3",
            "finalize 3",
            "gate 3 3 9 c3",
        ],
        "stderr:\n{}",
        stderr_of(&output)
    );
}

/// A looping two-leg group: its dictionary is `loop.last_output`, rebuilt
/// each pass, and the parent reads only the final pass's dictionary.
#[test]
fn a_looping_group_runs_three_passes_and_publishes_its_final_dictionary() {
    let seq = r#"---
sequence:
  - name: batch
    group:
      loop:
        until: "loop.count >= 3"
        gate:
          - append_line: ["events.log", "gate {{ loop.count }} {{ loop.last_output.left }} {{ loop.last_output.right }}"]
      initialize:
        - append_line: ["events.log", "initialize {{ loop.count }}"]
      start:
        - append_line: ["events.log", "start {{ loop.count }} {{ length(loop.last_output) }}"]
      finalize:
        - append_line: ["events.log", "finalize {{ loop.count }} {{ length(group.succeeded) }}"]
      tasks:
        - name: left
          shell: "echo L{{ loop.count }}"
        - name: right
          shell: "echo R{{ loop.count }}"
finalize:
  - append_line: ["events.log", "root {{ length(seq.outputs) }} {{ seq.outputs[0].left }} {{ seq.outputs[0].right }}"]
---
"#;
    let (events, output) = run_sequence_events(seq, &[]);
    assert!(output.status.success(), "status {:?}; stderr:\n{}", output.status, stderr_of(&output));
    assert_eq!(
        events,
        [
            "initialize 1",
            "start 1 0",
            "finalize 1 2",
            "gate 1 L1 R1",
            "start 2 2",
            "finalize 2 2",
            "gate 2 L2 R2",
            "start 3 2",
            "finalize 3 2",
            "gate 3 L3 R3",
            "root 1 L3 R3",
        ],
        "stderr:\n{}",
        stderr_of(&output)
    );
}

/// A document pass's body and `start` read the previous pass's result; its
/// terminal event, `finalize`, and the gate read its own.
#[test]
fn a_document_pass_reads_its_own_result_from_its_terminal_events() {
    let count = "---\nn: 1\nloop:\n  until: \"n >= 2\"\n  action: \"increment(n)\"\n  gate:\n    - append_line: [\"events.log\", \"gate {{ loop.count }} [{{ loop.last_output }}]\"]\nstart:\n  - append_line: [\"events.log\", \"start {{ loop.count }} [{{ loop.last_output }}]\"]\nsuccess:\n  - append_line: [\"events.log\", \"success {{ loop.count }} [{{ loop.last_output }}]\"]\nfinalize:\n  - append_line: [\"events.log\", \"finalize {{ loop.count }} [{{ loop.last_output }}]\"]\n---\nIteration {{ n }} after [{{ loop.last_output }}]\n";
    let fixture = CliProcessFixture::named("loop-primitives");
    fs::write(fixture.cwd().join("count.md"), count).unwrap();
    let recordings = launch_recorder::install(&fixture, "goose", "printf 'out%s\\n' \"$n\"");
    let output = fixture
        .command()
        .args(["compose", "--goose", "--yolo", "--silent"])
        .arg(fixture.cwd().join("count.md"))
        .output()
        .unwrap();
    assert!(output.status.success(), "stderr:\n{}", stderr_of(&output));
    // `goose` takes its prompt as an argument and prints it as its answer.
    let launches = launch_recorder::launches(&recordings);
    assert_eq!(launches.len(), 2);
    assert!(launches[0].iter().any(|arg| arg.trim() == "Iteration 1 after []"), "{launches:?}");
    assert!(launches[1].iter().any(|arg| arg.trim() == "Iteration 2 after [out0]"), "{launches:?}");
    let events = fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert_eq!(
        events.lines().collect::<Vec<_>>(),
        [
            "start 1 []",
            "success 1 [out0]",
            "finalize 1 [out0]",
            "gate 1 [out0]",
            "start 2 [out0]",
            "success 2 [out1]",
            "finalize 2 [out1]",
            "gate 2 [out1]",
        ]
    );
}

/// Nested loops: an inner looping sequence inside a looping root reads its
/// own `loop.*`, restarts its count on each outer pass, and the root's events
/// read the root's values.
#[test]
fn nested_loops_shadow_the_outer_loop_and_restart_on_each_outer_pass() {
    let seq = r#"---
loop:
  until: "loop.count >= 2"
  gate:
    - append_line: ["events.log", "outer gate {{ loop.count }} {{ length(loop.last_output) }}"]
initialize:
  - append_line: ["events.log", "outer initialize {{ loop.count }}"]
sequence:
  - name: inner
    loop:
      until: "loop.count >= 2"
      gate:
        - append_line: ["events.log", "inner gate {{ loop.count }} {{ loop.last_output[0] }}"]
    initialize:
      - append_line: ["events.log", "inner initialize {{ loop.count }}"]
    sequence:
      - name: work
        shell: "echo w{{ loop.count }}"
---
"#;
    let (events, output) = run_sequence_events(seq, &[]);
    assert!(output.status.success(), "status {:?}; stderr:\n{}", output.status, stderr_of(&output));
    assert_eq!(
        events,
        [
            "outer initialize 1",
            "inner initialize 1",
            "inner gate 1 w1",
            "inner gate 2 w2",
            "outer gate 1 1",
            "inner initialize 1",
            "inner gate 1 w1",
            "inner gate 2 w2",
            "outer gate 2 1",
        ],
        "stderr:\n{}",
        stderr_of(&output)
    );
}

/// Run one loop-outcome cell: a looping `primitive` (`sequence` or `group`)
/// whose action increments `n` from 1, ended by `until` under `max`.
/// Asserts the exit status, one gate per pass, and that the action ran only
/// before another pass.
fn check_outcome(primitive: &str, until: &str, max: usize, succeeds: bool, passes: usize) {
    let gate = r#"["events.log", "pass {{ loop.count }} n={{ n }}"]"#;
    let doc = match primitive {
        "sequence" => format!(
            "---\nn: 1\nloop:\n  until: \"{until}\"\n  max: {max}\n  action: \"increment(n)\"\n  gate:\n    - append_line: {gate}\nsequence:\n  - name: s\n    shell: \"echo s\"\n---\n"
        ),
        _ => format!(
            "---\nn: 1\nsequence:\n  - name: g\n    group:\n      loop:\n        until: \"{until}\"\n        max: {max}\n        action: \"increment(n)\"\n        gate:\n          - append_line: {gate}\n      tasks:\n        - name: leg\n          shell: \"echo leg\"\n---\n"
        ),
    };
    let fixture = CliProcessFixture::named("loop-primitives");
    fs::write(fixture.cwd().join("seq.md"), doc).unwrap();
    let output = fixture
        .command()
        .args(["sequence", "--claude", "--yolo"])
        .arg(fixture.cwd().join("seq.md"))
        .output()
        .unwrap();
    let stderr = stderr_of(&output);
    let events = fs::read_to_string(fixture.cwd().join("events.log")).unwrap_or_default();
    let expected: Vec<String> = (1..=passes).map(|pass| format!("pass {pass} n={pass}")).collect();
    assert_eq!(events.lines().collect::<Vec<_>>(), expected, "stderr:\n{stderr}");
    if succeeds {
        assert!(output.status.success(), "stderr:\n{stderr}");
    } else {
        assert_eq!(output.status.code(), Some(1), "exhaustion exits 1; stderr:\n{stderr}");
        assert!(stderr.contains("loop limit exceeded"), "typed exhaustion:\n{stderr}");
    }
}

/// Each primitive's loop outcomes: the condition met (exit 0), met on the
/// final permitted pass (exit 0), and the cap reached (typed exhaustion,
/// exit 1, no action after the last pass).
macro_rules! outcome_cells {
    ($($name:ident: $primitive:literal, $until:literal, $max:literal => $succeeds:literal, $passes:literal;)*) => {
        $(
            #[test]
            fn $name() {
                check_outcome($primitive, $until, $max, $succeeds, $passes);
            }
        )*
    };
}

outcome_cells! {
    sequence_condition_met: "sequence", "n >= 2", 3 => true, 2;
    sequence_condition_met_on_the_final_pass: "sequence", "n >= 3", 3 => true, 3;
    sequence_cap_reached: "sequence", "n >= 9", 3 => false, 3;
    group_condition_met: "group", "n >= 2", 3 => true, 2;
    group_condition_met_on_the_final_pass: "group", "n >= 3", 3 => true, 3;
    group_cap_reached: "group", "n >= 9", 3 => false, 3;
}

/// A loop that fails publishes no result to its parent, even after passes
/// that completed. Its runtime writes follow its primitive: a sequence writes
/// to the invocation, an atomic group withholds its legs' writes.
fn check_failed_loop_publishes_nothing(step: &str, touched: &str) {
    let seq = format!(
        "---\ntouched: none\nsequence:\n{step}failure:\n  - append_line: [\"events.log\", \"root outputs={{{{ seq.outputs }}}} touched={{{{ touched }}}}\"]\n---\n"
    );
    let (events, output) = run_sequence_events(&seq, &[]);
    assert_eq!(output.status.code(), Some(1), "stderr:\n{}", stderr_of(&output));
    assert_eq!(events, [format!("root outputs=[] touched={touched}")], "stderr:\n{}", stderr_of(&output));
}

#[test]
fn a_failed_sequence_loop_publishes_nothing_to_its_parent() {
    check_failed_loop_publishes_nothing(
        "  - name: inner\n    loop:\n      until: \"false\"\n      max: 2\n    sequence:\n      - name: work\n        side_effect: { set: { touched: \"seq{{ loop.count }}\" } }\n",
        "seq2",
    );
}

#[test]
fn a_failed_group_loop_publishes_nothing_to_its_parent() {
    check_failed_loop_publishes_nothing(
        "  - name: batch\n    group:\n      loop:\n        until: \"false\"\n        max: 2\n      tasks:\n        - name: leg\n          side_effect: { set: { touched: \"leg{{ loop.count }}\" } }\n",
        "none",
    );
}

/// A container's staged action that fails commits none of its mutations and
/// fails the loop, located at the block and the pass.
#[test]
fn a_failed_container_action_commits_nothing_and_names_the_pass() {
    let seq = r#"---
n: 1
label: text
loop:
  until: "false"
  max: 3
  actions:
    - "increment(n)"
    - "increment(label)"
failure:
  - append_line: ["events.log", "failure n={{ n }}"]
sequence:
  - name: s
    shell: "echo s"
---
"#;
    let fixture = CliProcessFixture::named("loop-primitives");
    fs::write(fixture.cwd().join("seq.md"), seq).unwrap();
    let output = fixture.command().args(["sequence", "--claude", "--yolo"]).arg(fixture.cwd().join("seq.md")).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "stderr:\n{stderr}");
    assert!(stderr.contains("`loop.action` failed after pass 1"), "located at the block and pass:\n{stderr}");
    // The pass already settled, so no terminal event runs again for it.
    assert!(!fixture.cwd().join("events.log").exists(), "failure does not fire after the pass settled");
}

/// The condition reads the file system fresh after every pass.
#[test]
fn a_container_condition_reads_files_written_by_the_pass() {
    let seq = r#"---
loop:
  until: "file_exists('done.txt')"
  max: 5
  gate:
    - append_line: ["events.log", "pass {{ loop.count }}"]
sequence:
  - name: maybe-finish
    shell: "finish-at-two {{ loop.count }}"
---
"#;
    let fixture = CliProcessFixture::named("loop-primitives");
    fs::write(fixture.cwd().join("seq.md"), seq).unwrap();
    crate::common::write_executable(
        &fixture.bin_dir().join("finish-at-two"),
        "#!/bin/sh\nif [ \"$1\" -ge 2 ]; then echo done > done.txt; fi\necho ok\n",
    );
    let output = fixture.command().args(["sequence", "--claude", "--yolo", "--silent"]).arg(fixture.cwd().join("seq.md")).output().unwrap();
    assert!(output.status.success(), "stderr:\n{}", stderr_of(&output));
    let events = fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert_eq!(events.lines().collect::<Vec<_>>(), ["pass 1", "pass 2"]);
}
