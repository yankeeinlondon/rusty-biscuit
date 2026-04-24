mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn choose_many_help_lists_options_and_validation_flags() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--options"))
        .stdout(predicate::str::contains("--options-from-file"))
        .stdout(predicate::str::contains("--options-from-dictionary"))
        .stdout(predicate::str::contains("--required"))
        .stdout(predicate::str::contains("--min-selections"))
        .stdout(predicate::str::contains("--max-selections"));
}

#[test]
fn choose_many_rejects_unknown_flag() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn choose_many_fails_when_no_option_source_provided() {
    cargo_bin_cmd!("question")
        .args(["choose-many"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("one of --options"));
}

#[test]
fn choose_many_reaches_event_loop_then_exits_with_error_when_stdin_is_not_a_tty() {
    cargo_bin_cmd!("question")
        .args([
            "choose-many",
            "--options",
            "A,B,C",
            "--initial",
            "A,C",
            "--output",
            "raw",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_accepts_output_flag_at_global_position() {
    cargo_bin_cmd!("question")
        .args(["--output", "json", "choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn choose_many_submits_raw_output_via_real_tty() {
    let output = common::run_question_in_pty(
        &[
            "--output",
            "raw",
            "choose-many",
            "--options",
            "A,B,C",
            "--initial",
            "A,C",
        ],
        r"\r",
        0,
    );
    let stdout = common::clean_terminal_text(&output.stdout);

    assert!(stdout.contains("A\r\nC\r\n"), "stdout was {stdout:?}");
    assert!(output.stderr.is_empty(), "stderr was {:?}", output.stderr);
}
