mod common;

use common::md_cmd;
use predicates::prelude::*;

#[test]
fn test_delta_subcommand_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified"));
}

#[test]
fn test_delta_subcommand_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"classification\""));
}

