use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn text_area_input_help_lists_width_and_scrollbar_flags() {
    cargo_bin_cmd!("question")
        .args(["text-area-input", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--width"))
        .stdout(predicate::str::contains("--scrollbar"));
}

#[test]
fn text_area_input_rejects_unknown_flag() {
    cargo_bin_cmd!("question")
        .args(["text-area-input", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}
