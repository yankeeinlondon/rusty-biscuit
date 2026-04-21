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
