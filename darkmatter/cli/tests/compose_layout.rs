mod common;

use common::md_cmd;
use predicates::prelude::*;

#[test]
fn test_compose_compact() {
    let input = "---\n---\n\n- item 1\n\n- item 2\n\n- item 3";
    md_cmd()
        .args(["compose", "--compact", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("- item 1\n- item 2\n- item 3"));
}

#[test]
fn test_compose_loose() {
    let input = "---\n---\n\n- item 1\n- item 2\n- item 3";
    md_cmd()
        .args(["compose", "--loose", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("- item 1\n\n- item 2\n\n- item 3"));
}
