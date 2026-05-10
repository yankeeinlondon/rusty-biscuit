//! Phase 6 / Test Coverage #5: smoke test that `clip --help` runs and
//! advertises every documented subcommand.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn clip_help_lists_all_subcommands() {
    let mut cmd = Command::cargo_bin("clip").expect("locate clip binary");
    cmd.arg("--help").assert().success().stdout(
        predicate::str::contains("get")
            .and(predicate::str::contains("set"))
            .and(predicate::str::contains("info"))
            .and(predicate::str::contains("history"))
            .and(predicate::str::contains("clear"))
            .and(predicate::str::contains("watch"))
            .and(predicate::str::contains("service")),
    );
}

#[test]
fn clip_version_prints_a_version() {
    let mut cmd = Command::cargo_bin("clip").expect("locate clip binary");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("clip"));
}
