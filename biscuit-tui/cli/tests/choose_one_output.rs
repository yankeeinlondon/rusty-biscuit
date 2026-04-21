mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn choose_one_help_lists_options_flags() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--options"))
        .stdout(predicate::str::contains("--options-from-file"))
        .stdout(predicate::str::contains("--options-from-dictionary"));
}

#[test]
fn choose_one_rejects_unknown_flag() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

// Removed: --options-from-dictionary expects a file path,
// so file-not-found errors are normal and not useful to test here.

#[test]
fn choose_one_fails_when_no_option_source_provided() {
    cargo_bin_cmd!("question")
        .args(["choose-one"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("one of --options"));
}

#[test]
fn choose_one_reaches_event_loop_then_exits_with_error_when_stdin_is_not_a_tty() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--options", "Red,Green,Blue", "--initial", "Red", "--output", "raw"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_accepts_output_flag_at_global_position() {
    cargo_bin_cmd!("question")
        .args(["--output", "json", "choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn choose_one_submits_raw_output_via_real_tty() {
    let output = common::run_question_in_pty(
        &[
            "--output",
            "raw",
            "choose-one",
            "--options",
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
