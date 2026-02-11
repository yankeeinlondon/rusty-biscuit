use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("unchained").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "AI pipeline tools and agent status monitoring",
        ))
        .stdout(predicate::str::contains("limits"));
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("unchained").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_limits_help() {
    let mut cmd = Command::cargo_bin("unchained").unwrap();
    cmd.arg("limits")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Show usage limits and cap status"))
        .stdout(predicate::str::contains("--platform"));
}

#[test]
fn test_limits_invalid_platform() {
    let mut cmd = Command::cargo_bin("unchained").unwrap();
    cmd.arg("limits")
        .arg("--platform")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown platform"));
}

#[test]
fn test_no_subcommand() {
    let mut cmd = Command::cargo_bin("unchained").unwrap();
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Usage: unchained"));
}

#[test]
fn test_completions_bash() {
    let mut cmd = Command::cargo_bin("unchained").unwrap();
    cmd.arg("--completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_unchained()"));
}

#[test]
fn test_completions_zsh() {
    let mut cmd = Command::cargo_bin("unchained").unwrap();
    cmd.arg("--completions")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef unchained"));
}
