use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    Command::cargo_bin("sniff")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detect system"));
}

#[test]
fn test_version_flag() {
    Command::cargo_bin("sniff")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff"));
}

#[test]
fn test_json_output() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""));
}

#[test]
fn test_text_output() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--format", "text"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Hardware ==="));
}

#[test]
fn test_base_dir_flag() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--base", "."])
        .assert()
        .success();
}

#[test]
fn test_skip_hardware_flag() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--skip-hardware", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"\""));
}

#[test]
fn test_skip_network_flag() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--skip-network", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"interfaces\": []"));
}

#[test]
fn test_skip_filesystem_flag() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--skip-filesystem", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filesystem\": null"));
}

#[test]
fn test_invalid_format_fails() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--format", "invalid"])
        .assert()
        .failure();
}
