//! Level-1 process coverage for an `error` that converts a terminal event to
//! failure: the owning command's exit status and rendered diagnostic, not only
//! the lifecycle events that fired.
//!
//! Every row asserts the exit status **and** how many times the error text is
//! rendered. The authored `warn` lines never interpolate `err.msg`, so any
//! occurrence of [`REASON`] in the output is Claudine's own rendering.

use crate::common;

use common::CliProcessFixture;
use common::write;
#[cfg(unix)]
use common::write_executable;
use std::fs;
use std::path::{Path, PathBuf};

const REASON: &str = "artifact never written";

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

impl Run {
    fn reason_count(&self) -> usize {
        self.output.matches(REASON).count()
    }

    fn error_line_count(&self) -> usize {
        self.output
            .lines()
            .filter(|line| line.trim_start().starts_with("Error:"))
            .count()
    }
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

fn compose(fixture: &CliProcessFixture, doc: &Path) -> Run {
    run(
        fixture,
        &["compose", "--claude", doc.to_str().expect("UTF-8 path")],
    )
}

fn lines_of(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// The event trail every row records, so a row proves the lifecycle it claims
/// ran before asserting what the command reported.
const TRAIL: &str = r#"failure:
    stack:
        - action:
              - append_line: ["events.log", "failure"]
finalize:
    stack:
        - action:
              - append_line: ["events.log", "finalize"]
"#;

#[test]
fn start_error_fails_with_one_diagnostic() {
    let fixture = fixture("downgrade-start");
    let doc = write_doc(
        &fixture,
        "start.md",
        &format!(
            "---\nstart:\n    stack:\n        - action:\n              - error: \"{REASON}\"\n---\nbody\n"
        ),
    );

    let run = compose(&fixture, &doc);

    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.error_line_count(), 1, "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}

#[test]
fn success_error_on_first_attempt_fails_with_one_diagnostic() {
    let fixture = fixture("downgrade-success");
    let doc = write_doc(
        &fixture,
        "success.md",
        &format!(
            "---\nsuccess:\n    info: \"success communication\"\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"success\"]\n              - error: \"{REASON}\"\n{TRAIL}---\nbody\n"
        ),
    );

    let run = compose(&fixture, &doc);

    assert_eq!(
        lines_of(&fixture.cwd().join("events.log")),
        ["success", "failure", "finalize"],
        "{}",
        run.output
    );
    assert!(
        run.output.contains("success communication"),
        "the already-emitted success communication is kept: {}",
        run.output
    );
    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.error_line_count(), 1, "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}

#[test]
fn success_error_without_reason_renders_default_message() {
    let fixture = fixture("downgrade-success-no-reason");
    let doc = write_doc(
        &fixture,
        "success.md",
        "---\nsuccess:\n    stack:\n        - action:\n              - error: null\n---\nbody\n",
    );

    let run = compose(&fixture, &doc);

    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.error_line_count(), 1, "{}", run.output);
    assert!(
        run.output.contains("Error: lifecycle success error"),
        "{}",
        run.output
    );
}

#[test]
fn success_error_after_exhausted_retry_fails_with_one_diagnostic() {
    let fixture = fixture("downgrade-retry-exhausted");
    let doc = write_doc(
        &fixture,
        "retry.md",
        &format!(
            "---\nsuccess:\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"success\"]\n              - error: \"{REASON}\"\nfinalize:\n    stack:\n        - when: \"err\"\n          action: {{ retry: 1 }}\n---\nbody\n"
        ),
    );

    let run = compose(&fixture, &doc);

    assert_eq!(
        lines_of(&fixture.cwd().join("events.log")),
        ["success", "success"],
        "the retry budget allows exactly one more attempt: {}",
        run.output
    );
    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.error_line_count(), 1, "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}

#[test]
fn success_error_after_exhausted_resume_fails_with_one_diagnostic() {
    let fixture = fixture("downgrade-resume-exhausted");
    let doc = write_doc(
        &fixture,
        "resume.md",
        &format!(
            "---\nsuccess:\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"success\"]\n              - error: \"{REASON}\"\nfailure:\n    stack:\n        - action:\n              - action: resume\n                message: \"try again\"\n                max_attempts: 1\n---\nbody\n"
        ),
    );

    let run = compose(&fixture, &doc);

    assert_eq!(
        lines_of(&fixture.cwd().join("events.log")),
        ["success", "success"],
        "{}",
        run.output
    );
    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.error_line_count(), 1, "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}

/// `success` raises only while `marker.txt` is absent, and creates it as it
/// does, so the first attempt downgrades and the recovered attempt succeeds.
fn recovering_success(recovery: &str) -> String {
    format!(
        "---\nsuccess:\n    stack:\n        - when: \"!file_exists('marker.txt')\"\n          action:\n              - append_line: [\"marker.txt\", \"first\"]\n              - error: \"{REASON}\"\n{recovery}---\nbody\n"
    )
}

#[test]
fn success_error_recovered_by_retry_succeeds_without_stale_error() {
    let fixture = fixture("downgrade-retry-recovers");
    let doc = write_doc(
        &fixture,
        "retry.md",
        &recovering_success(
            "finalize:\n    stack:\n        - when: \"err\"\n          action: { retry: 1 }\n",
        ),
    );

    let run = compose(&fixture, &doc);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(run.error_line_count(), 0, "{}", run.output);
    assert_eq!(run.reason_count(), 0, "{}", run.output);
}

#[test]
fn success_error_recovered_by_resume_succeeds_without_stale_error() {
    let fixture = fixture("downgrade-resume-recovers");
    let doc = write_doc(
        &fixture,
        "resume.md",
        &recovering_success(
            "failure:\n    stack:\n        - action:\n              - action: resume\n                message: \"try again\"\n                max_attempts: 1\n",
        ),
    );

    let run = compose(&fixture, &doc);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(run.error_line_count(), 0, "{}", run.output);
    assert_eq!(run.reason_count(), 0, "{}", run.output);
}

#[test]
fn success_error_recovered_by_proxy_reports_target_outcome() {
    let fixture = fixture("downgrade-proxy-recovers");
    write_doc(
        &fixture,
        "target.md",
        "---\nsuccess:\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"target success\"]\n---\ntarget body\n",
    );
    let doc = write_doc(
        &fixture,
        "source.md",
        &format!(
            "---\nsuccess:\n    stack:\n        - action:\n              - error: \"{REASON}\"\nfailure:\n    stack:\n        - action:\n              - proxy: \"./target.md\"\n---\nbody\n"
        ),
    );

    let run = compose(&fixture, &doc);

    assert_eq!(
        lines_of(&fixture.cwd().join("events.log")),
        ["target success"],
        "{}",
        run.output
    );
    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(run.error_line_count(), 0, "{}", run.output);
    assert_eq!(run.reason_count(), 0, "{}", run.output);
}

#[test]
fn finalize_error_fails_with_one_diagnostic() {
    let fixture = fixture("downgrade-finalize");
    let doc = write_doc(
        &fixture,
        "finalize.md",
        &format!(
            "---\nfinalize:\n    stack:\n        - action:\n              - error: \"{REASON}\"\n---\nbody\n"
        ),
    );

    let run = compose(&fixture, &doc);

    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.error_line_count(), 1, "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}

fn downgrading_step(fixture: &CliProcessFixture) {
    write_doc(
        fixture,
        "step.md",
        &format!(
            "---\nsuccess:\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"step success\"]\n              - error: \"{REASON}\"\n---\nstep body\n"
        ),
    );
}

fn sequence_doc(fail_fast: bool) -> String {
    format!(
        "---\nfail_fast: {fail_fast}\nsequence:\n    - name: first\n      prompt: \"./step.md\"\n    - name: second\n      side_effect: {{ append_line: [\"events.log\", \"second step\"] }}\n---\n"
    )
}

#[test]
fn sequence_step_downgrade_halts_under_fail_fast_true() {
    let fixture = fixture("downgrade-sequence-fail-fast");
    downgrading_step(&fixture);
    let doc = write_doc(&fixture, "sequence.md", &sequence_doc(true));

    let run = run(
        &fixture,
        &["sequence", "--claude", doc.to_str().expect("UTF-8 path")],
    );

    assert_eq!(
        lines_of(&fixture.cwd().join("events.log")),
        ["step success"],
        "the second step must not run: {}",
        run.output
    );
    assert!(run.output.contains("step 1/2 failed"), "{}", run.output);
    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}

#[test]
fn sequence_step_downgrade_continues_and_fails_under_fail_fast_false() {
    let fixture = fixture("downgrade-sequence-continue");
    downgrading_step(&fixture);
    let doc = write_doc(&fixture, "sequence.md", &sequence_doc(false));

    let run = run(
        &fixture,
        &["sequence", "--claude", doc.to_str().expect("UTF-8 path")],
    );

    assert_eq!(
        lines_of(&fixture.cwd().join("events.log")),
        ["step success", "second step"],
        "the second step still runs: {}",
        run.output
    );
    assert!(run.output.contains("step 1/2 failed"), "{}", run.output);
    assert!(
        run.output.contains("1 succeeded, 1 failed"),
        "{}",
        run.output
    );
    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}

fn looping_doc(fail_fast: bool) -> String {
    format!(
        "---\ncounter: 0\nloop:\n    while: \"counter < 2\"\n    action: \"increment(counter)\"\n    fail_fast: {fail_fast}\nsuccess:\n    stack:\n        - action:\n              - append_line: [\"events.log\", \"iteration {{{{ _loop_count }}}}\"]\n              - error: \"{REASON}\"\n---\nbody\n"
    )
}

#[test]
fn loop_iteration_downgrade_halts_under_fail_fast_true() {
    let fixture = fixture("downgrade-loop-fail-fast");
    let doc = write_doc(&fixture, "loop.md", &looping_doc(true));

    let run = compose(&fixture, &doc);

    assert_eq!(
        lines_of(&fixture.cwd().join("events.log")),
        ["iteration 1"],
        "the failed iteration halts the loop: {}",
        run.output
    );
    assert!(run.output.contains("iteration failed"), "{}", run.output);
    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}

#[test]
fn loop_iteration_downgrade_reaches_gate_under_fail_fast_false() {
    let fixture = fixture("downgrade-loop-continue");
    let doc = write_doc(&fixture, "loop.md", &looping_doc(false));

    let run = compose(&fixture, &doc);

    // Each failed iteration still reaches the gate, whose action advances the
    // counter, so the condition ends the loop after the counter-2 iteration.
    assert_eq!(
        lines_of(&fixture.cwd().join("events.log")),
        ["iteration 1", "iteration 2", "iteration 3"],
        "{}",
        run.output
    );
    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(
        run.reason_count(),
        3,
        "each failed iteration reports its error once: {}",
        run.output
    );
    assert_eq!(run.error_line_count(), 0, "{}", run.output);
}

#[cfg(unix)]
#[test]
fn inline_success_error_fails_with_one_diagnostic() {
    let fixture = CliProcessFixture::named("downgrade-inline");
    fixture.seed_user_config();
    let doc = fixture.cwd().join("inline.md");
    common::InlineAgentStub::new(&doc).install(fixture.bin_dir(), "claude");
    write(
        &doc,
        &format!(
            "---\nprompt: \"Write the body.\"\nsuccess:\n    stack:\n        - action:\n              - error: \"{REASON}\"\n---\nOriginal body\n"
        ),
    );

    let run = run(
        &fixture,
        &[
            "inline-compose",
            "--claude",
            doc.to_str().expect("UTF-8 path"),
        ],
    );

    assert_eq!(run.code, Some(1), "{}", run.output);
    assert_eq!(run.error_line_count(), 1, "{}", run.output);
    assert_eq!(run.reason_count(), 1, "{}", run.output);
}
