use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn input_table_help_lists_columns_and_rows_flags() {
    cargo_bin_cmd!("question")
        .args(["input-table", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--columns"))
        .stdout(predicate::str::contains("--rows"));
}

#[test]
fn input_table_rejects_unknown_flag() {
    cargo_bin_cmd!("question")
        .args(["input-table", "--nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn input_table_fails_with_invalid_json_columns() {
    cargo_bin_cmd!("question")
        .args(["input-table", "--columns", "[{"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--columns").or(predicate::str::contains("JSON")));
}

#[test]
fn input_table_fails_with_unknown_column_type() {
    cargo_bin_cmd!("question")
        .args(["input-table", "--columns", r#"[{"type":"frobnicate"}]"#])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--columns").or(predicate::str::contains("type")));
}
