mod common;
use test_toolkit::{Level, require_level};

use predicates::prelude::*;

#[test]
fn text_area_input_help_lists_width_and_scrollbar_flags() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["text-area-input", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--width"))
        .stdout(predicate::str::contains("--scrollbar"));
}

#[test]
fn text_area_input_rejects_unknown_flag() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["text-area-input", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn text_area_input_reaches_event_loop_then_exits_with_error_when_stdin_is_not_a_tty() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args([
            "text-area-input",
            "--initial",
            "hello\nworld",
            "--width",
            "20",
            "--output",
            "raw",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn text_area_input_accepts_output_flag_at_global_position() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["--output", "json", "text-area-input", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn level2_text_area_input_submits_json_output_via_pty() {
    require_level!(
        Level::L2,
        common::expect_driver_available(),
        "expect (PTY script driver)"
    );
    let output = common::run_question_in_pty(
        &[
            "--output",
            "json",
            "text-area-input",
            "--initial",
            "hello\nworld",
            "--width",
            "20",
        ],
        r"\023",
        0,
    );
    let stdout = common::clean_terminal_text(&output.stdout);

    assert!(
        stdout.contains("\"hello\\nworld\"\r\n"),
        "stdout was {stdout:?}"
    );
    assert!(output.stderr.is_empty(), "stderr was {:?}", output.stderr);
}
