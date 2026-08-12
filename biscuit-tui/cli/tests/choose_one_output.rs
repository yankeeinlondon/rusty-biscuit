mod common;
use test_toolkit::{Level, require_level};

use predicates::prelude::*;

#[test]
fn choose_one_help_lists_options_flags() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--csv"))
        .stdout(predicate::str::contains("--list"))
        .stdout(predicate::str::contains("--rows"))
        .stdout(predicate::str::contains("--file"))
        .stdout(predicate::str::contains("--md"))
        .stdout(predicate::str::contains("--numeric-hot-keys"))
        .stdout(predicate::str::contains("--label-convention"))
        .stdout(predicate::str::contains("--value-convention"));
}

#[test]
fn choose_one_rejects_unknown_flag() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["choose-one", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

// Removed: --options-from-dictionary expects a file path,
// so file-not-found errors are normal and not useful to test here.

#[test]
fn choose_one_fails_when_no_option_source_provided() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["choose-one"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no options provided"));
}

#[test]
fn choose_one_reaches_event_loop_then_exits_with_error_when_stdin_is_not_a_tty() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args([
            "choose-one",
            "--csv",
            "Red,Green,Blue",
            "--initial",
            "Red",
            "--output",
            "raw",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_accepts_output_flag_at_global_position() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["--output", "json", "choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn level2_choose_one_submits_raw_output_via_pty() {
    require_level!(
        Level::L2,
        common::expect_driver_available(),
        "expect (PTY script driver)"
    );
    let output = common::run_question_in_pty(
        &[
            "--output",
            "raw",
            "choose-one",
            "--csv",
            "Red,Green,Blue",
            "--initial",
            "Red",
        ],
        r"\r",
        0,
    );
    let stdout = common::clean_terminal_text(&output.stdout);

    assert!(stdout.contains("Red\r\n"), "stdout was {stdout:?}");
    assert!(output.stderr.is_empty(), "stderr was {:?}", output.stderr);
}
