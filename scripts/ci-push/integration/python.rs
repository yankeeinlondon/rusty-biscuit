//! Interpreter selection against fixture interpreters: each candidate is
//! executed, and the first that proves Python 3.10 or newer wins.

use std::ffi::OsString;
use std::time::{Duration, Instant};

use repo_deps::ci_push::process::Cancellation;
use repo_deps::ci_push::PythonCommand;
use serde_json::json;

use crate::fixture::{pid_of, process_gone, wait_until, Fixture};

fn probed_tools(fixture: &Fixture) -> Vec<String> {
    fixture
        .invocations()
        .iter()
        .map(|invocation| invocation["tool"].as_str().unwrap().to_owned())
        .collect()
}

fn select(fixture: &Fixture, timeout: Duration) -> Result<PythonCommand, String> {
    PythonCommand::select_with(Some(&fixture.path()), timeout, &Cancellation::new())
        .map_err(|error| error.to_string())
}

#[test]
fn the_first_candidate_that_proves_itself_is_selected() {
    let fixture = Fixture::new(
        &["python3", "python"],
        json!({ "python3": { "python_version": "3.12" }, "python": { "python_version": "3.13" } }),
    );

    let selected = PythonCommand::select(Some(&fixture.path())).unwrap();

    assert_eq!(selected.executable, fixture.tool("python3"));
    assert!(selected.prefix.is_empty());
    let probe = fixture.only_invocation();
    assert_eq!(probe["tool"], json!("python3"));
    assert_eq!(probe["argv"][0], json!("-c"));
}

#[test]
fn a_broken_python3_falls_back_to_python() {
    let fixture = Fixture::new(
        &["python3", "python"],
        json!({
            "python3": { "exit": 2, "stderr": "Python was not found" },
            "python": { "python_version": "3.11" },
        }),
    );

    let selected = select(&fixture, Duration::from_secs(2)).unwrap();

    assert_eq!(selected.executable, fixture.tool("python"));
    assert_eq!(probed_tools(&fixture), ["python3", "python"]);
}

#[test]
fn an_old_python_is_rejected_and_named_in_the_one_error() {
    let fixture = Fixture::new(
        &["python3"],
        json!({ "python3": { "python_version": "3.9" } }),
    );

    let error = select(&fixture, Duration::from_secs(2)).unwrap_err();

    assert!(
        error.contains("is Python 3.9; 3.10 or newer is required"),
        "{error}"
    );
    assert!(
        error.contains(&fixture.tool("python3").display().to_string()),
        "{error}"
    );
    assert!(error.contains("python not found on PATH"), "{error}");
}

#[test]
fn a_failed_probe_reports_its_status_and_stderr() {
    let fixture = Fixture::new(
        &["python3", "python"],
        json!({
            "python3": { "exit": 1, "stderr": "Python was not found" },
            "python": { "python_version": "not a version" },
        }),
    );

    let error = select(&fixture, Duration::from_secs(2)).unwrap_err();

    assert!(
        error.contains("version probe exited with status 1: Python was not found"),
        "{error}"
    );
    assert!(
        error.contains(r#"answered the version probe with "not a version""#),
        "{error}"
    );
}

#[test]
fn a_probe_that_does_not_answer_is_killed_at_the_injected_timeout() {
    let fixture = Fixture::new(&["python3"], json!({ "python3": { "hang": true } }));
    let started = Instant::now();

    let error = select(&fixture, Duration::from_millis(300)).unwrap_err();

    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    assert!(
        error.contains("did not answer the version probe within 300 ms"),
        "{error}"
    );
    let probe = pid_of(&fixture.only_invocation());
    wait_until(
        Duration::from_secs(3),
        "the timed-out probe to be gone",
        || process_gone(probe),
    );
}

#[test]
fn the_selected_prefix_starts_every_python_invocation() {
    let fixture = Fixture::new(&["py"], json!({ "py": {} }));
    let python = PythonCommand {
        executable: fixture.tool("py"),
        prefix: vec![OsString::from("-3")],
    };

    let completed = python
        .command()
        .args(["planner.py", "--flag"])
        .cancellation(Cancellation::new())
        .deadline(Duration::from_secs(4))
        .run()
        .unwrap();

    assert!(completed.success(), "{completed:?}");
    assert_eq!(
        fixture.only_invocation()["argv"],
        json!(["-3", "planner.py", "--flag"])
    );
}

#[cfg(windows)]
#[test]
fn py_dash_3_is_tried_after_python3_and_before_python() {
    let fixture = Fixture::new(
        &["py", "python"],
        json!({ "py": { "python_version": "3.12" }, "python": { "python_version": "3.12" } }),
    );

    let selected = select(&fixture, Duration::from_secs(2)).unwrap();

    assert_eq!(selected.executable, fixture.tool("py"));
    assert_eq!(selected.prefix, [OsString::from("-3")]);
    let probe = fixture.only_invocation();
    assert_eq!(probe["argv"][0], json!("-3"));
    assert_eq!(probe["argv"][1], json!("-c"));
}
