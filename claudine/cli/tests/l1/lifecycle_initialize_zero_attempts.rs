//! `retry` and `resume` with `max_attempts: 0` in a document's `initialize`
//! are refused like any other `initialize` retry or resume: the run reports
//! the typed refusal and ends with no provider launch, no catch event, and no
//! `finalize`. Both once panicked in the document pipeline's `initialize`
//! routing.

use crate::common::{CliProcessFixture, launch_recorder, strip_ansi, write};

/// Run a document whose `initialize` ends in `directive`; return the exit
/// code, stderr, and the events its stacks recorded.
fn run_initialize(name: &str, directive: &str) -> (Option<i32>, String, String) {
    let fixture = CliProcessFixture::named(name);
    // `append_line` resolves against the repository root.
    fixture.initialize_repository();
    let records = launch_recorder::install(&fixture, "claude", "exit 0");
    write(
        &fixture.cwd().join("doc.md"),
        &format!(
            "---\n\
             initialize:\n  - append_line: [\"trace.log\", \"event initialize\"]\n  - {directive}\n\
             start:\n  - append_line: [\"trace.log\", \"event start\"]\n\
             blocked:\n  - append_line: [\"trace.log\", \"event blocked\"]\n\
             failure:\n  - append_line: [\"trace.log\", \"event failure\"]\n\
             finalize:\n  - append_line: [\"trace.log\", \"event finalize\"]\n\
             ---\nHello\n"
        ),
    );

    let output = fixture.command().args(["compose", "doc.md", "--claude"]).output().unwrap();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(launch_recorder::launches(&records).is_empty(), "a provider launched:\n{stderr}");
    let trace = std::fs::read_to_string(fixture.cwd().join("trace.log")).unwrap_or_default();
    (output.status.code(), stderr, trace)
}

#[test]
fn a_zero_attempt_retry_in_initialize_reports_the_setup_refusal_without_finalize() {
    let (code, stderr, trace) = run_initialize("initialize-retry-zero", "retry: {max_attempts: 0}");

    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("lifecycle `retry` at `initialize` is accepted"), "{stderr}");
    assert_eq!(trace, "event initialize\n", "{stderr}");
}

#[test]
fn a_zero_attempt_resume_in_initialize_reports_no_session_without_finalize() {
    let (code, stderr, trace) =
        run_initialize("initialize-resume-zero", "resume: {message: \"continue\", max_attempts: 0}");

    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("lifecycle `resume` requires a live provider session"), "{stderr}");
    assert_eq!(trace, "event initialize\n", "{stderr}");
}
