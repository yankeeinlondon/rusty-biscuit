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
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""));
}

#[test]
fn test_text_output() {
    // Default output is text
    Command::cargo_bin("sniff")
        .unwrap()
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
        .args(["--skip-hardware", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"\""));
}

#[test]
fn test_skip_network_flag() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--skip-network", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"interfaces\": []"));
}

#[test]
fn test_skip_filesystem_flag() {
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--skip-filesystem", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filesystem\": null"));
}

#[test]
fn test_verbose_flag() {
    // -v should show more details
    Command::cargo_bin("sniff")
        .unwrap()
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("Total:"));
}

#[test]
fn test_double_verbose_flag() {
    // -vv should work (higher verbosity)
    Command::cargo_bin("sniff")
        .unwrap()
        .arg("-vv")
        .assert()
        .success();
}

// === Include-only mode tests ===

#[test]
fn test_hardware_include_only_flag() {
    // --hardware should output only hardware section
    Command::cargo_bin("sniff")
        .unwrap()
        .arg("--hardware")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Hardware ==="))
        .stdout(predicate::str::contains("=== Network ===").not())
        .stdout(predicate::str::contains("=== Filesystem ===").not());
}

#[test]
fn test_network_include_only_flag() {
    // --network should output only network section
    Command::cargo_bin("sniff")
        .unwrap()
        .arg("--network")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Network ==="))
        .stdout(predicate::str::contains("=== Hardware ===").not())
        .stdout(predicate::str::contains("=== Filesystem ===").not());
}

#[test]
fn test_filesystem_include_only_flag() {
    // --filesystem should output only filesystem section
    Command::cargo_bin("sniff")
        .unwrap()
        .arg("--filesystem")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Filesystem ==="))
        .stdout(predicate::str::contains("=== Hardware ===").not())
        .stdout(predicate::str::contains("=== Network ===").not());
}

#[test]
fn test_combined_include_flags() {
    // --hardware --network should output both sections, skip filesystem
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--hardware", "--network"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Hardware ==="))
        .stdout(predicate::str::contains("=== Network ==="))
        .stdout(predicate::str::contains("=== Filesystem ===").not());
}

#[test]
fn test_include_mode_ignores_skip_flags() {
    // In include-only mode, skip flags should be ignored
    // --hardware --skip-network should still output only hardware (skip ignored)
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--hardware", "--skip-network"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Hardware ==="))
        .stdout(predicate::str::contains("=== Network ===").not());
}

#[test]
fn test_include_mode_json_output() {
    // Include-only mode should work with JSON output
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--hardware", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""))
        .stdout(predicate::str::contains("\"interfaces\": []")); // network skipped
}
