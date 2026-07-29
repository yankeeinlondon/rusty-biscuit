mod common;
use test_toolkit::{Level, require_level};

use predicates::prelude::*;

#[test]
fn boolean_switch_help_lists_labels_flag() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["boolean-switch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--labels"))
        .stdout(predicate::str::contains("--initial"));
}

#[test]
fn boolean_switch_rejects_unknown_flag() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["boolean-switch", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn boolean_switch_reaches_event_loop_then_exits_with_error_when_stdin_is_not_a_tty() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["boolean-switch", "--initial", "true", "--output", "raw"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn boolean_switch_accepts_output_flag_at_global_position() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["--output", "json", "boolean-switch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn level2_boolean_switch_submits_json_output_via_pty() {
    require_level!(
        Level::L2,
        common::expect_driver_available(),
        "expect (PTY script driver)"
    );
    let output = common::run_question_in_pty(
        &["--output", "json", "boolean-switch", "--initial", "true"],
        r"\r",
        0,
    );
    let stdout = common::clean_terminal_text(&output.stdout);

    assert!(stdout.contains("true\r\n"), "stdout was {stdout:?}");
    assert!(output.stderr.is_empty(), "stderr was {:?}", output.stderr);
}
