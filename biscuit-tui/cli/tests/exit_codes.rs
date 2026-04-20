use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn choose_one_exits_nonzero_when_no_option_source() {
    cargo_bin_cmd!("question")
        .args(["choose-one"])
        .assert()
        .failure()
        .code(predicate::ne(0))
        .stderr(predicate::str::contains("one of --options"));
}

#[test]
fn choose_many_exits_nonzero_when_no_option_source() {
    cargo_bin_cmd!("question")
        .args(["choose-many"])
        .assert()
        .failure()
        .code(predicate::ne(0))
        .stderr(predicate::str::contains("one of --options"));
}

#[test]
fn input_table_exits_nonzero_with_malformed_columns() {
    cargo_bin_cmd!("question")
        .args(["input-table", "--columns", "not-json"])
        .assert()
        .failure()
        .code(predicate::ne(0))
        .stderr(predicate::str::contains("--columns").or(predicate::str::contains("JSON")));
}
