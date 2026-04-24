//! Integration tests for the `choose-one` and `choose-many`
//! subcommands that exercise Phase 3 CLI-level behaviours: STDIN
//! sourcing, positional arguments, and the `--delimiter` split.
//!
//! Tests here verify that the CLI's parsing and source-resolution
//! layer reaches the event loop without tripping an arg-validation
//! error. The event loop itself fails with exit code 1 because the
//! assert_cmd harness does not attach a real TTY — we lean on that
//! "reached the event loop" signal as a proxy for "parsing worked".
//!
//! Phase 12 will replace the event-loop-level assertions with proper
//! autosubmit-backed flows once the `QUESTION_TEST_AUTOSUBMIT` hook
//! is wired up.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn choose_one_reads_from_stdin() {
    cargo_bin_cmd!("question")
        .args(["choose-one"])
        .write_stdin("alpha\nbeta\ngamma\n")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_reads_from_stdin() {
    cargo_bin_cmd!("question")
        .args(["choose-many"])
        .write_stdin("alpha\nbeta\ngamma\n")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_accepts_positional_args() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "alpha", "beta", "gamma"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_accepts_positional_args() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "alpha", "beta", "gamma"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn delimiter_separates_label_and_value() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--delimiter", ":", "Apple:1", "Berry:2"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_empty_stdin_errors_with_no_options_message() {
    cargo_bin_cmd!("question")
        .args(["choose-one"])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no options provided"));
}

#[test]
fn choose_one_selected_and_initial_conflict() {
    cargo_bin_cmd!("question")
        .args([
            "choose-one",
            "--options",
            "a,b",
            "--selected",
            "a",
            "--initial",
            "b",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn choose_one_initial_flag_is_hidden_from_help() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--selected"))
        .stdout(predicate::str::contains("--initial").not());
}

#[test]
fn choose_many_initial_flag_is_hidden_from_help() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--selected"))
        .stdout(predicate::str::contains("--initial").not());
}
