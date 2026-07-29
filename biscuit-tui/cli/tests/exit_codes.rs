mod common;

use predicates::prelude::*;

#[test]
fn choose_one_exits_nonzero_when_no_option_source() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["choose-one"])
        .assert()
        .failure()
        .code(predicate::ne(0))
        .stderr(predicate::str::contains("no options provided"));
}

#[test]
fn choose_many_exits_nonzero_when_no_option_source() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["choose-many"])
        .assert()
        .failure()
        .code(predicate::ne(0))
        .stderr(predicate::str::contains("no options provided"));
}

#[test]
fn input_table_exits_nonzero_with_malformed_columns() {
    assert_cmd::Command::cargo_bin("question").unwrap()
        .args(["input-table", "--columns", "not-json"])
        .assert()
        .failure()
        .code(predicate::ne(0))
        .stderr(predicate::str::contains("--columns").or(predicate::str::contains("JSON")));
}

#[test]
fn every_subcommand_exits_with_code_1_on_non_tty_event_loop_read() {
    let test_cases = [
        (
            "text-input",
            vec!["text-input", "--initial", "Ada", "--max-length", "10"],
        ),
        (
            "text-area-input",
            vec![
                "text-area-input",
                "--initial",
                "hello\nworld",
                "--width",
                "20",
            ],
        ),
        (
            "boolean-switch",
            vec!["boolean-switch", "--initial", "true"],
        ),
        (
            "choose-one",
            vec!["choose-one", "--csv", "Red,Green,Blue", "--initial", "Red"],
        ),
        (
            "choose-many",
            vec!["choose-many", "--csv", "A,B,C", "--initial", "A,C"],
        ),
        (
            "input-table",
            vec![
                "input-table",
                "--columns",
                r#"[{"type":"text-input","id":"name"}]"#,
            ],
        ),
    ];

    for (_name, args) in test_cases {
        assert_cmd::Command::cargo_bin("question").unwrap()
            .args(&args)
            .assert()
            .failure()
            .code(1)
            .stderr(predicate::str::contains("question:"));
    }
}

#[test]
fn text_input_exits_with_code_1_on_escape_in_real_tty() {
    // Phase 12 split: Esc now maps to exit code 1 (ABORTED), distinct
    // from Ctrl+C which continues to map to 130 (CANCELLED).
    let output = common::run_question_in_pty(&["text-input", "--initial", "Ada"], r"\033", 1);
    let stdout = common::clean_terminal_text(&output.stdout);

    assert!(
        !stdout.contains("Ada\r\n"),
        "cancel should not emit submitted output: {stdout:?}"
    );
    assert!(output.stderr.is_empty(), "stderr was {:?}", output.stderr);
}
