//! Integration tests for the `terminal` CLI binary.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_no_args_shows_basic_info() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Terminal:"));
}

#[test]
fn test_meta_flag_shows_metadata() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.arg("--meta")
        .assert()
        .success()
        .stdout(predicate::str::contains("Terminal Metadata"));
}

#[test]
fn test_meta_json_flag_outputs_json() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args(["--meta", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"app\""))
        .stdout(predicate::str::contains("\"is_tty\""));
}

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Display terminal metadata"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("bt"));
}

#[test]
fn test_respects_no_color() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.env("NO_COLOR", "1")
        .arg("--meta")
        .assert()
        .success()
        // Should NOT contain escape codes when NO_COLOR is set
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn test_meta_shows_underline_support() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.arg("--meta")
        .assert()
        .success()
        .stdout(predicate::str::contains("Underline Support"))
        .stdout(predicate::str::contains("Straight:"))
        .stdout(predicate::str::contains("Curly:"));
}

#[test]
fn test_json_output_is_valid_json() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    let output = cmd
        .args(["--meta", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Verify it parses as valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON: {}", stdout);

    // Verify expected fields exist
    let json = parsed.unwrap();
    assert!(json.get("app").is_some(), "Should have 'app' field");
    assert!(json.get("os").is_some(), "Should have 'os' field");
    assert!(json.get("width").is_some(), "Should have 'width' field");
    assert!(json.get("height").is_some(), "Should have 'height' field");
    assert!(json.get("is_tty").is_some(), "Should have 'is_tty' field");
    assert!(json.get("is_ci").is_some(), "Should have 'is_ci' field");
    assert!(
        json.get("color_depth").is_some(),
        "Should have 'color_depth' field"
    );
    assert!(
        json.get("supports_italic").is_some(),
        "Should have 'supports_italic' field"
    );
    assert!(
        json.get("multiplex").is_some(),
        "Should have 'multiplex' field"
    );
}

#[test]
fn test_default_output_shows_size() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Size:"));
}

#[test]
fn test_default_output_shows_tty_status() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Is TTY:"));
}
