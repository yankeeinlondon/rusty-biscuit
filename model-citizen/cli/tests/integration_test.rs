use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use predicates::prelude::*;

#[test]
fn test_help() {
    let mut cmd = Command::new(cargo_bin("model"));
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CLI for managing local LLM models",
        ));
}

#[test]
fn test_list_unknown_runner() {
    let mut cmd = Command::new(cargo_bin("model"));
    cmd.arg("list")
        .arg("--runner")
        .arg("unknown_runner")
        .assert()
        .success()
        .stdout(predicate::str::contains("No models found"));
}

#[test]
fn test_list_json() {
    let mut cmd = Command::new(cargo_bin("model"));
    cmd.arg("--json")
        .arg("list")
        .assert()
        .success()
        // we can't be sure models exist, but we know it outputs a JSON array
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn test_completions() {
    let mut cmd = Command::new(cargo_bin("model"));
    cmd.arg("completions")
        .assert()
        .success()
        .stdout(predicate::str::contains("SHELL COMPLETIONS"));
}

// Example of insta usage, though testing the actual full table output
// would require a mocked backend because system models differ.
#[test]
fn test_search_no_results() {
    let mut cmd = Command::new(cargo_bin("model"));
    let assert = cmd
        .arg("search")
        .arg("nonexistent_model_1234567890_impossible")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&assert.get_output().stdout);
    // Since this is dynamic, we just check the string here instead of snapshot.
    // To do true snapshot testing, we'd want to mock the HuggingFace API.
    assert!(output.contains("No models found matching"));
}
