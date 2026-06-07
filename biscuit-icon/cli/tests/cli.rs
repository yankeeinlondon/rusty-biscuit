use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_subcommands() {
    Command::cargo_bin("icon")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("icons"))
        .stdout(predicate::str::contains("sets"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn cache_clear_succeeds_with_isolated_home() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache cleared"));
}

#[test]
fn completions_bash_emits_script() {
    Command::cargo_bin("icon")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("icon"));
}
