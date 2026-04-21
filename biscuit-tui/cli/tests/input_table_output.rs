mod common;

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

#[test]
fn input_table_reaches_event_loop_then_exits_with_error_when_stdin_is_not_a_tty() {
    cargo_bin_cmd!("question")
        .args(["input-table", "--columns", r#"[{"type":"text-input","id":"name"}]"#, "--output", "raw"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn input_table_accepts_output_flag_at_global_position() {
    cargo_bin_cmd!("question")
        .args(["--output", "json", "input-table", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn input_table_submits_json_output_via_real_tty() {
    let output = common::run_question_in_pty(
        &[
            "--output",
            "json",
            "input-table",
            "--columns",
            r#"[{"type":"text-input","id":"name"},{"type":"boolean-switch","id":"active"}]"#,
            "--rows",
            r#"[["alice",true]]"#,
        ],
        r"\023",
        0,
    );
    let stdout = common::clean_terminal_text(&output.stdout);

    assert!(
        stdout.contains(r#"[{"active":true,"name":"alice"}]"#)
            || stdout.contains(r#"[{"name":"alice","active":true}]"#),
        "stdout was {stdout:?}"
    );
    assert!(output.stderr.is_empty(), "stderr was {:?}", output.stderr);
}
