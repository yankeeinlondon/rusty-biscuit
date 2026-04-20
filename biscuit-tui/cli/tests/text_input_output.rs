use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn text_input_help_lists_max_length_flag() {
    cargo_bin_cmd!("question")
        .args(["text-input", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--max-length"));
}

#[test]
fn text_input_rejects_unknown_flag() {
    cargo_bin_cmd!("question")
        .args(["text-input", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}
