//! Level-1 process coverage for a lifecycle `proxy` reaching the coordinator
//! that owns the document: the `--loop` engine and each `sequence` step.
//!
//! Every row changes only the event that raises the `proxy` or the owner that
//! runs the source, and reads the event trail both documents append to
//! `events.log`. A handoff that was dropped shows up as a further source event
//! or a missing target event; a request the ledger recorded without adopting
//! shows up as a false cycle on the next request.

use crate::common;

use common::CliProcessFixture;
use common::write;
#[cfg(unix)]
use common::write_executable;
use std::fs;
use std::path::{Path, PathBuf};

/// A Claude stub that reports one successful result and exits 0.
fn write_succeeding_claude(bin_dir: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"session-1","model":"claude-test"}'
printf '%s\n' '{"type":"result","subtype":"success","result":"done","session_id":"session-1","is_error":false}'
exit 0
"#,
    );

    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        "@echo off\r\n\
echo {\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"session-1\",\"model\":\"claude-test\"}\r\n\
echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
exit /b 0\r\n",
    );
}

fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    write_succeeding_claude(fixture.bin_dir());
    fixture
}

fn write_doc(fixture: &CliProcessFixture, name: &str, content: &str) -> PathBuf {
    let path = fixture.cwd().join(name);
    write(&path, content);
    path
}

struct Run {
    code: Option<i32>,
    output: String,
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    let output = fixture
        .command()
        .args(args)
        .output()
        .expect("claudine runs");
    Run {
        code: output.status.code(),
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}

fn events(fixture: &CliProcessFixture) -> Vec<String> {
    fs::read_to_string(fixture.cwd().join("events.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// One stack item appending `line` to the shared trail.
fn trail(event: &str, line: &str) -> String {
    format!(
        "{event}:\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"{line}\"]\n"
    )
}

/// A stack item appending `line` and then proxying to `./target.md`.
fn trail_then_proxy(event: &str, line: &str) -> String {
    trail_then_proxy_to(event, line, "./target.md")
}

/// A stack item appending `line` and then proxying to `target`.
fn trail_then_proxy_to(event: &str, line: &str, target: &str) -> String {
    format!(
        "{event}:\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"{line}\"]\n              - proxy: {target}\n"
    )
}

/// The adopted target: it records its own `initialize`, `success`, and
/// `finalize`, so a row can prove it entered at `initialize` and ran once.
fn write_target(fixture: &CliProcessFixture) {
    write_doc(
        fixture,
        "target.md",
        &format!(
            "---\n{}{}{}---\ntarget body\n",
            trail("initialize", "target initialize"),
            trail("success", "target success"),
            trail("finalize", "target finalize"),
        ),
    );
}

const TARGET_TRAIL: [&str; 3] = ["target initialize", "target success", "target finalize"];

fn assert_no_false_cycle(run: &Run) {
    assert!(
        !run.output.contains("forms a cycle"),
        "an adopted handoff leaves no request behind to collide with: {}",
        run.output
    );
}

// ---------------------------------------------------------------------------
// The loop owner
// ---------------------------------------------------------------------------

/// A looping source. `stacks` supplies its lifecycle; the loop would run two
/// iterations if nothing handed it off.
fn looping_source(stacks: &str) -> String {
    format!(
        "---\nn: 0\nloop:\n    while: \"n < 2\"\n    action: \"increment(n)\"\n{stacks}---\nloop body {{{{ _loop_count }}}}\n"
    )
}

fn compose(fixture: &CliProcessFixture, doc: &Path) -> Run {
    run(fixture, &["compose", "--claude", doc.to_str().expect("UTF-8 path")])
}

#[test]
fn loop_success_proxy_hands_off_after_one_iteration() {
    let fixture = fixture("handoff-loop-success");
    write_target(&fixture);
    let source = write_doc(
        &fixture,
        "loop.md",
        &looping_source(&format!(
            "{}{}",
            trail_then_proxy("success", "source success {{ _loop_count }}"),
            trail("finalize", "source finalize"),
        )),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["source success 1"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_no_false_cycle(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

#[test]
fn loop_failure_proxy_hands_off_after_one_iteration() {
    let fixture = fixture("handoff-loop-failure");
    write_target(&fixture);
    // `error` in `success` turns the attempt into a failure, so `failure` is the
    // event that raises the proxy.
    let source = write_doc(
        &fixture,
        "loop.md",
        &looping_source(&format!(
            "success:\n    stack:\n        - action:\n              - error: \"not good enough\"\n{}{}",
            trail_then_proxy("failure", "source failure {{ _loop_count }}"),
            trail("finalize", "source finalize"),
        )),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["source failure 1"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_no_false_cycle(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

#[test]
fn loop_finalize_proxy_hands_off_without_rerunning_finalize() {
    let fixture = fixture("handoff-loop-finalize");
    write_target(&fixture);
    let source = write_doc(
        &fixture,
        "loop.md",
        &looping_source(&format!(
            "{}{}",
            trail("success", "source success {{ _loop_count }}"),
            trail_then_proxy("finalize", "source finalize {{ _loop_count }}"),
        )),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["source success 1", "source finalize 1"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_no_false_cycle(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// The loop gate belongs to the iteration the proxy abandoned, so it must not
/// fire; a gate concern that appends to the trail makes that visible.
#[test]
fn loop_terminal_proxy_does_not_fire_the_abandoned_iterations_gate() {
    let fixture = fixture("handoff-loop-gate");
    write_target(&fixture);
    let source = write_doc(
        &fixture,
        "loop.md",
        &format!(
            "---\nn: 0\nloop:\n    while: \"n < 2\"\n    action: \"increment(n)\"\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"gate\"]\n{}---\nloop body\n",
            trail_then_proxy("success", "source success"),
        ),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["source success"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

#[test]
fn loop_start_proxy_hands_off_before_the_iteration_runs() {
    let fixture = fixture("handoff-loop-start");
    write_target(&fixture);
    let source = write_doc(
        &fixture,
        "loop.md",
        &looping_source(&format!(
            "{}{}{}",
            trail_then_proxy("start", "source start {{ _loop_count }}"),
            trail("success", "source success"),
            trail("finalize", "source finalize"),
        )),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["source start 1"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_no_false_cycle(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

#[test]
fn loop_proxy_after_a_retry_hands_off_after_one_iteration() {
    let fixture = fixture("handoff-loop-retry");
    write_target(&fixture);
    let source = write_doc(
        &fixture,
        "loop.md",
        &looping_source(&retry_then_proxy("source success after retry {{ _loop_count }}")),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["source success after retry 1"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_no_false_cycle(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// The baseline every other owner is measured against: a document with no loop
/// and no sequence, whose `success` proxies.
#[test]
fn compose_success_proxy_adopts_the_target() {
    let fixture = fixture("handoff-compose-success");
    write_target(&fixture);
    let source = write_doc(
        &fixture,
        "source.md",
        &format!(
            "---\n{}{}---\nsource body\n",
            trail_then_proxy("success", "source success"),
            trail("finalize", "source finalize"),
        ),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["source success"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

// ---------------------------------------------------------------------------
// The sequence owner
// ---------------------------------------------------------------------------

/// A two-step sequence: a `prompt:` task on `./step.md`, then a side effect that
/// records how many `outputs` entries the first step published.
fn sequence_doc(fail_fast: bool, task_extras: &str) -> String {
    format!(
        "---\nfail_fast: {fail_fast}\nsequence:\n    - name: first\n      prompt: \"./step.md\"\n{task_extras}    - name: second\n      side_effect: {{ append_line: [\"events.log\", \"second step outputs={{{{ length(outputs) }}}}\"] }}\n---\n"
    )
}

fn run_sequence(fixture: &CliProcessFixture, doc: &Path) -> Run {
    run(fixture, &["sequence", "--claude", doc.to_str().expect("UTF-8 path")])
}

fn assert_step_one_succeeded_once(run: &Run) {
    assert_eq!(
        run.output.matches("step 1/2 succeeded").count(),
        1,
        "the first step completes exactly once: {}",
        run.output
    );
}

fn task_proxy_row(name: &str, event: &str, source_line: &str) {
    task_stacks_row(name, &trail_then_proxy(event, source_line), &[source_line]);
}

/// Run `step.md` with `stacks` as the first step's `prompt:` task and assert the
/// target ran inside that step, after the source's own `source_trail`.
fn task_stacks_row(name: &str, stacks: &str, source_trail: &[&str]) {
    let fixture = fixture(name);
    write_target(&fixture);
    write_doc(&fixture, "step.md", &format!("---\n{stacks}---\nstep body\n"));
    let doc = write_doc(&fixture, "sequence.md", &sequence_doc(true, ""));

    let run = run_sequence(&fixture, &doc);

    let mut expected = source_trail.to_vec();
    expected.extend(TARGET_TRAIL);
    expected.push("second step outputs=1");
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_step_one_succeeded_once(&run);
    assert_no_false_cycle(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

#[test]
fn sequence_task_initialize_proxy_runs_target_within_the_step() {
    task_proxy_row("handoff-task-initialize", "initialize", "step initialize");
}

#[test]
fn sequence_task_start_proxy_runs_target_within_the_step() {
    task_proxy_row("handoff-task-start", "start", "step start");
}

#[test]
fn sequence_task_success_proxy_runs_target_within_the_step() {
    task_proxy_row("handoff-task-success", "success", "step success");
}

#[test]
fn sequence_task_finalize_proxy_runs_target_within_the_step() {
    task_proxy_row("handoff-task-finalize", "finalize", "step finalize");
}

#[test]
fn sequence_task_failure_proxy_runs_target_within_the_step() {
    // `error` in `success` turns the attempt into a failure, so `failure` is the
    // event that raises the proxy.
    task_stacks_row(
        "handoff-task-failure",
        &format!(
            "success:\n    stack:\n        - action:\n              - error: \"not good enough\"\n{}",
            trail_then_proxy("failure", "step failure")
        ),
        &["step failure"],
    );
}

/// `success` retries the first attempt and proxies from the second, so the
/// handoff comes out of the retry/resume attempt loop rather than a first
/// attempt; the owner adopts it the same way.
fn retry_then_proxy(line: &str) -> String {
    format!(
        "success:\n    stack:\n        - when: \"!file_exists('retried.txt')\"\n          action:\n              - append_line: [\"retried.txt\", \"once\"]\n              - retry: 1\n        - action:\n              - append_line: [\"events.log\", \"{line}\"]\n              - proxy: ./target.md\n"
    )
}

#[test]
fn sequence_task_proxy_after_a_retry_runs_target_within_the_step() {
    task_stacks_row(
        "handoff-task-retry",
        &retry_then_proxy("step success after retry"),
        &["step success after retry"],
    );
}

/// Setup and teardown belong to the task, not to a document, so a handoff inside
/// the task runs each once, and the task publishes one `outputs` entry: the
/// final target's.
#[test]
fn sequence_task_setup_teardown_and_output_run_once_across_a_handoff() {
    let fixture = fixture("handoff-task-setup-teardown");
    write_target(&fixture);
    write_doc(
        &fixture,
        "step.md",
        &format!("---\n{}---\nstep body\n", trail_then_proxy("start", "step start")),
    );
    let doc = write_doc(
        &fixture,
        "sequence.md",
        &sequence_doc(
            true,
            "      setup:\n          - action:\n                - append_line: [\"events.log\", \"setup\"]\n      teardown:\n          - action:\n                - append_line: [\"events.log\", \"teardown\"]\n",
        ),
    );

    let run = run_sequence(&fixture, &doc);

    let mut expected = vec!["setup", "step start"];
    expected.extend(TARGET_TRAIL);
    expected.extend(["teardown", "second step outputs=1"]);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_step_one_succeeded_once(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// A target that fails is the step's failure: teardown still runs once, no
/// output is published, and `fail_fast` decides whether step two runs.
fn failing_target_row(name: &str, fail_fast: bool) {
    let fixture = fixture(name);
    write_doc(
        &fixture,
        "target.md",
        &format!(
            "---\n{}success:\n    stack:\n        - action:\n              - error: \"target rejected\"\n---\ntarget body\n",
            trail("initialize", "target initialize"),
        ),
    );
    write_doc(
        &fixture,
        "step.md",
        &format!("---\n{}---\nstep body\n", trail_then_proxy("start", "step start")),
    );
    let doc = write_doc(
        &fixture,
        "sequence.md",
        &sequence_doc(
            fail_fast,
            "      teardown:\n          - action:\n                - append_line: [\"events.log\", \"teardown\"]\n",
        ),
    );

    let run = run_sequence(&fixture, &doc);

    let mut expected = vec!["step start", "target initialize", "teardown"];
    if !fail_fast {
        expected.push("second step outputs=0");
    }
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_eq!(
        run.output.matches("step 1/2 failed").count(),
        1,
        "{}",
        run.output
    );
    assert!(run.output.contains("target rejected"), "{}", run.output);
    assert_eq!(run.code, Some(1), "{}", run.output);
}

#[test]
fn sequence_task_failing_target_halts_under_fail_fast_true() {
    failing_target_row("handoff-task-fail-fast", true);
}

#[test]
fn sequence_task_failing_target_continues_under_fail_fast_false() {
    failing_target_row("handoff-task-continue", false);
}

/// Parallel siblings own independent chains, so both may adopt the same target
/// without either seeing the other's hop as a cycle.
#[test]
fn parallel_siblings_adopt_the_same_target_independently() {
    let fixture = fixture("handoff-task-parallel");
    write_doc(
        &fixture,
        "target.md",
        &format!("---\n{}---\ntarget body\n", trail("success", "target success")),
    );
    write_doc(
        &fixture,
        "left.md",
        &format!("---\n{}---\nleft body\n", trail_then_proxy("start", "left start")),
    );
    write_doc(
        &fixture,
        "right.md",
        &format!("---\n{}---\nright body\n", trail_then_proxy("start", "right start")),
    );
    let doc = write_doc(
        &fixture,
        "sequence.md",
        "---\nsequence:\n    - name: both\n      group:\n          name: pair\n          execution: parallel\n          tasks:\n              - name: left\n                prompt: \"./left.md\"\n              - name: right\n                prompt: \"./right.md\"\n    - name: after\n      side_effect: { append_line: [\"events.log\", \"after outputs={{ length(outputs) }} slots={{ length(last(outputs)) }}\"] }\n---\n",
    );

    let run = run_sequence(&fixture, &doc);

    let mut lines = events(&fixture);
    let last = lines.pop();
    lines.sort();
    assert_eq!(
        lines,
        ["left start", "right start", "target success", "target success"],
        "{}",
        run.output
    );
    // A parallel group publishes one entry with a slot per member.
    assert_eq!(last.as_deref(), Some("after outputs=1 slots=2"), "{}", run.output);
    assert_no_false_cycle(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// The task's inputs outlive the handoff: the target sees the task's
/// `operation` and the step's environment exactly as the source did.
#[test]
fn sequence_task_target_keeps_the_tasks_operation_and_environment() {
    let fixture = fixture("handoff-task-inputs");
    let probe = "op={{ env.OPERATION }} fail_fast={{ env.CLAUDINE_FAIL_FAST }}";
    write_doc(
        &fixture,
        "target.md",
        &format!("---\n{}---\ntarget body\n", trail("start", &format!("target {probe}"))),
    );
    write_doc(
        &fixture,
        "step.md",
        &format!(
            "---\n{}---\nstep body\n",
            trail_then_proxy("start", &format!("source {probe}"))
        ),
    );
    let doc = write_doc(
        &fixture,
        "sequence.md",
        &sequence_doc(false, "      operation: review\n"),
    );

    let run = run_sequence(&fixture, &doc);

    assert_eq!(
        events(&fixture),
        [
            "source op=review fail_fast=false",
            "target op=review fail_fast=false",
            "second step outputs=1"
        ],
        "{}",
        run.output
    );
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// Serial group members are separate tasks too, so each owns its chain and the
/// second may adopt the target the first already adopted.
#[test]
fn serial_group_members_adopt_the_same_target_independently() {
    let fixture = fixture("handoff-task-serial");
    write_doc(
        &fixture,
        "target.md",
        &format!("---\n{}---\ntarget body\n", trail("success", "target success")),
    );
    for name in ["left", "right"] {
        write_doc(
            &fixture,
            &format!("{name}.md"),
            &format!(
                "---\n{}---\n{name} body\n",
                trail_then_proxy("start", &format!("{name} start"))
            ),
        );
    }
    let doc = write_doc(
        &fixture,
        "sequence.md",
        "---\nsequence:\n    - name: both\n      group:\n          name: pair\n          execution: serial\n          tasks:\n              - name: left\n                prompt: \"./left.md\"\n              - name: right\n                prompt: \"./right.md\"\n    - name: after\n      side_effect: { append_line: [\"events.log\", \"after outputs={{ length(outputs) }}\"] }\n---\n",
    );

    let run = run_sequence(&fixture, &doc);

    // A serial group commits one entry per member.
    assert_eq!(
        events(&fixture),
        [
            "left start",
            "target success",
            "right start",
            "target success",
            "after outputs=2"
        ],
        "{}",
        run.output
    );
    assert_no_false_cycle(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// A cycle is still a cycle inside one task's chain.
#[test]
fn a_cycle_within_one_tasks_chain_is_refused() {
    let fixture = fixture("handoff-task-cycle");
    write_doc(
        &fixture,
        "target.md",
        "---\nstart:\n    stack:\n        - action:\n              - proxy: ./step.md\n---\ntarget body\n",
    );
    write_doc(
        &fixture,
        "step.md",
        &format!("---\n{}---\nstep body\n", trail_then_proxy("start", "step start")),
    );
    let doc = write_doc(&fixture, "sequence.md", &sequence_doc(true, ""));

    let run = run_sequence(&fixture, &doc);

    assert!(run.output.contains("forms a cycle"), "{}", run.output);
    assert_eq!(events(&fixture), ["step start"], "{}", run.output);
    assert_eq!(run.code, Some(1), "{}", run.output);
}

/// A step that runs the sequence document's own body keeps its scope across a
/// handoff too: the target's output is the one entry the step publishes.
#[test]
fn sequence_body_step_proxy_publishes_the_targets_output_once() {
    let fixture = fixture("handoff-body-step");
    write_target(&fixture);
    let doc = write_doc(
        &fixture,
        "sequence.md",
        &format!(
            "---\nsequence:\n    - name: first\n    - name: second\n      side_effect: {{ append_line: [\"events.log\", \"second step outputs={{{{ length(outputs) }}}}\"] }}\n{}---\nBody {{{{ state }}}}.\n",
            "start:\n    stack:\n        - when: \"state.name == 'first'\"\n          action:\n              - append_line: [\"events.log\", \"body start\"]\n              - proxy: ./target.md\n",
        ),
    );

    let run = run_sequence(&fixture, &doc);

    let mut expected = vec!["body start"];
    expected.extend(TARGET_TRAIL);
    expected.push("second step outputs=1");
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_step_one_succeeded_once(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// The ledger records committed handoffs, never requests. Iteration 1's proxy
/// is refused as a cycle and fails that iteration; with `fail_fast: false` the
/// loop continues, and iteration 2's legitimate proxy is adopted rather than
/// refused against an entry the refusal left behind.
#[test]
fn a_refused_request_leaves_no_entry_for_the_next_one_to_collide_with() {
    let fixture = fixture("handoff-refused-then-legitimate");
    write_target(&fixture);
    let source = write_doc(
        &fixture,
        "loop.md",
        concat!(
            "---\nn: 0\nloop:\n    while: \"n < 3\"\n    action: \"increment(n)\"\n    fail_fast: false\n",
            "success:\n    stack:\n",
            "        - when: \"_loop_count == 1\"\n          action:\n              - append_line: [\"events.log\", \"refused request\"]\n              - proxy: ./loop.md\n",
            "        - when: \"_loop_count == 2\"\n          action:\n              - append_line: [\"events.log\", \"legitimate request\"]\n              - proxy: ./target.md\n",
            "---\nloop body\n",
        ),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["refused request", "legitimate request"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    // One "refused request" line: an adopted self-proxy would have restarted
    // the loop at iteration 1 and appended it again.
    // Reported once: the engine drops a non-halting iteration's error, so the
    // loop executor is the only place that can show it.
    assert_eq!(
        run.output.matches("forms a cycle").count(),
        1,
        "iteration 1's refused self-proxy is reported exactly once: {}",
        run.output
    );
    assert_no_false_cycle_on_target(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// A refused overlay evaluation and a refused resolution are refusals too:
/// neither leaves an entry, so the next iteration's legitimate proxy is adopted.
fn refused_then_legitimate_row(name: &str, refused_action: &str, refusal: &str) {
    let fixture = fixture(name);
    write_target(&fixture);
    let source = write_doc(
        &fixture,
        "loop.md",
        &format!(
            concat!(
                "---\nn: 0\nloop:\n    while: \"n < 3\"\n    action: \"increment(n)\"\n    fail_fast: false\n",
                "success:\n    stack:\n",
                "        - when: \"_loop_count == 1\"\n          action:\n              - append_line: [\"events.log\", \"refused request\"]\n              - {}\n",
                "        - when: \"_loop_count == 2\"\n          action:\n              - append_line: [\"events.log\", \"legitimate request\"]\n              - proxy: ./target.md\n",
                "---\nloop body\n",
            ),
            refused_action
        ),
    );

    let run = compose(&fixture, &source);

    let mut expected = vec!["refused request", "legitimate request"];
    expected.extend(TARGET_TRAIL);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert!(
        run.output.contains(refusal),
        "fixture check: iteration 1's request must really be refused: {}",
        run.output
    );
    assert_no_false_cycle_on_target(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

#[test]
fn a_refused_resolution_leaves_no_entry_for_the_next_request() {
    refused_then_legitimate_row(
        "handoff-refused-resolution",
        "proxy: ./missing.md",
        "missing.md",
    );
}

#[test]
fn a_refused_overlay_evaluation_leaves_no_entry_for_the_next_request() {
    refused_then_legitimate_row(
        "handoff-refused-overlay",
        "{ action: proxy, target: ./target.md, with: { topic: \"{{ no_such_root }}\" } }",
        "no_such_root",
    );
}

fn assert_no_false_cycle_on_target(run: &Run) {
    assert!(
        !run.output.contains("hand-off to `./target.md` forms a cycle"),
        "{}",
        run.output
    );
}

/// Output with hard wraps and the error box's border glyphs removed, so a
/// wrapped diagnostic reads as one line.
fn collapsed(run: &Run) -> String {
    run.output.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A target that fails after adoption: `success` raises, and `failure` hands
/// off to `./other.md`, whose `start` hands straight back.
fn write_failing_target_that_is_proxied_back_to(fixture: &CliProcessFixture) {
    write_doc(
        fixture,
        "target.md",
        &format!(
            "---\n{}success:\n    stack:\n        - action:\n              - error: \"target rejected\"\n{}---\ntarget body\n",
            trail("initialize", "target initialize"),
            trail_then_proxy_to("failure", "target failure", "./other.md"),
        ),
    );
    write_doc(
        fixture,
        "other.md",
        &format!(
            "---\n{}---\nother body\n",
            trail_then_proxy_to("start", "other start", "./target.md")
        ),
    );
}

/// Committing is adopting: a target that fails after adoption stays on the
/// chain, so the hop back to it is a cycle rather than a second entry at its
/// `initialize`.
fn assert_adopted_target_stays_recorded(run: &Run, events: Vec<String>, source_line: &str) {
    assert_eq!(
        events,
        [source_line, "target initialize", "target failure", "other start"],
        "{}",
        run.output
    );
    // `compose` renders the refusal as a box and `sequence` as a one-line
    // summary; both may break inside a word, so match with spaces removed.
    let compact = collapsed(run).replace(' ', "");
    assert!(compact.contains("formsacycle"), "{}", run.output);
    assert!(
        compact.contains("to`./target.md`"),
        "the refused hop is the one back to the adopted target: {}",
        run.output
    );
    assert_eq!(run.code, Some(1), "{}", run.output);
}

#[test]
fn compose_an_adopted_target_that_fails_remains_recorded() {
    let fixture = fixture("handoff-adopted-fails-compose");
    write_failing_target_that_is_proxied_back_to(&fixture);
    let source = write_doc(
        &fixture,
        "source.md",
        &format!("---\n{}---\nsource body\n", trail_then_proxy("success", "source success")),
    );

    let run = compose(&fixture, &source);

    assert_adopted_target_stays_recorded(&run, events(&fixture), "source success");
}

#[test]
fn sequence_task_an_adopted_target_that_fails_remains_recorded() {
    let fixture = fixture("handoff-adopted-fails-task");
    write_failing_target_that_is_proxied_back_to(&fixture);
    write_doc(
        &fixture,
        "step.md",
        &format!("---\n{}---\nstep body\n", trail_then_proxy("start", "step start")),
    );
    let doc = write_doc(&fixture, "sequence.md", &sequence_doc(true, ""));

    let run = run_sequence(&fixture, &doc);

    assert_adopted_target_stays_recorded(&run, events(&fixture), "step start");
    assert_eq!(run.output.matches("step 1/2 failed").count(), 1, "{}", run.output);
}

/// The terminal-event counterpart of the `start` row above: a handoff from the
/// task document's `success` still runs setup and teardown once and publishes
/// one `outputs` entry, the final target's.
#[test]
fn sequence_task_setup_teardown_and_output_run_once_across_a_terminal_handoff() {
    let fixture = fixture("handoff-task-setup-teardown-terminal");
    write_target(&fixture);
    write_doc(
        &fixture,
        "step.md",
        &format!("---\n{}---\nstep body\n", trail_then_proxy("success", "step success")),
    );
    let doc = write_doc(
        &fixture,
        "sequence.md",
        &sequence_doc(
            true,
            "      setup:\n          - action:\n                - append_line: [\"events.log\", \"setup\"]\n      teardown:\n          - action:\n                - append_line: [\"events.log\", \"teardown\"]\n",
        ),
    );

    let run = run_sequence(&fixture, &doc);

    let mut expected = vec!["setup", "step success"];
    expected.extend(TARGET_TRAIL);
    expected.extend(["teardown", "second step outputs=1"]);
    assert_eq!(events(&fixture), expected, "{}", run.output);
    assert_step_one_succeeded_once(&run);
    assert_eq!(run.code, Some(0), "{}", run.output);
}

/// Must match `claudine::composition::MAX_PROXY_HOPS`.
const MAX_PROXY_HOPS: usize = 16;

/// A chain of distinct documents longer than the hop limit is refused at the
/// hop that would overflow it. The refusal is reported once, the refused
/// target never starts, and the active chain the refusal reports ends at the
/// last adopted document: the refused request left no entry behind.
#[test]
fn a_chain_past_the_hop_limit_is_refused_without_recording_the_refused_target() {
    let fixture = fixture("handoff-hop-limit");
    for i in 0..=MAX_PROXY_HOPS {
        write_doc(
            &fixture,
            &format!("hop{i}.md"),
            &format!(
                "---\n{}---\nhop body\n",
                trail_then_proxy_to("start", &format!("hop{i} start"), &format!("./hop{}.md", i + 1)),
            ),
        );
    }

    let run = compose(&fixture, &fixture.cwd().join("hop0.md"));

    let adopted: Vec<String> = (0..MAX_PROXY_HOPS).map(|i| format!("hop{i} start")).collect();
    assert_eq!(events(&fixture), adopted, "{}", run.output);
    let compact = collapsed(&run).replace(' ', "");
    assert_eq!(compact.matches("formsacycle").count(), 1, "{}", run.output);
    assert!(compact.contains(&format!("hoplimitof{MAX_PROXY_HOPS}")), "{}", run.output);
    let last_adopted = format!("hop{}.md`", MAX_PROXY_HOPS - 1);
    assert!(compact.contains(&last_adopted), "the chain ends at {last_adopted}: {}", run.output);
    assert_eq!(
        compact.matches(&format!("hop{MAX_PROXY_HOPS}.md")).count(),
        1,
        "the refused target is named once, as the target, and is not on the chain: {}",
        run.output
    );
    assert_eq!(run.code, Some(1), "{}", run.output);
}
