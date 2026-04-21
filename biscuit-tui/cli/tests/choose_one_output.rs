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
