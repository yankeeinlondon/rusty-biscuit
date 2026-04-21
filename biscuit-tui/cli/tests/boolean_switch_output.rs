use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn boolean_switch_help_lists_labels_flag() {
    cargo_bin_cmd!("question")
        .args(["boolean-switch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--labels"))
        .stdout(predicate::str::contains("--initial"));
}

#[test]
fn boolean_switch_rejects_unknown_flag() {
    cargo_bin_cmd!("question")
        .args(["boolean-switch", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}
