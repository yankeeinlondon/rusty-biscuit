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
    // Skipped sections are omitted from JSON output entirely
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--skip-hardware", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\"").not());
}

#[test]
fn test_skip_network_flag() {
    // Skipped sections are omitted from JSON output entirely
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--skip-network", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"network\"").not());
}

#[test]
fn test_skip_filesystem_flag() {
    // Skipped sections are omitted from JSON output entirely
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--skip-filesystem", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filesystem\"").not());
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
    // Skipped sections are omitted entirely
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--hardware", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""))
        .stdout(predicate::str::contains("\"network\"").not()); // network skipped
}

// === Regression tests for JSON output filtering ===
// These tests ensure skipped sections are completely absent from JSON output,
// not just empty. Bug: --hardware flag was outputting empty network/filesystem data.

#[test]
fn test_hardware_only_json_excludes_all_other_sections() {
    // Regression test: --hardware should output ONLY hardware in JSON
    // Bug: Previously output `"network": { "interfaces": [], ... }` and `"filesystem": null`
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--hardware", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""))
        .stdout(predicate::str::contains("\"network\"").not())
        .stdout(predicate::str::contains("\"filesystem\"").not())
        .stdout(predicate::str::contains("\"interfaces\"").not())
        .stdout(predicate::str::contains("\"permission_denied\"").not());
}

#[test]
fn test_network_only_json_excludes_all_other_sections() {
    // Regression test: --network should output ONLY network in JSON
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--network", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"network\""))
        .stdout(predicate::str::contains("\"hardware\"").not())
        .stdout(predicate::str::contains("\"filesystem\"").not())
        .stdout(predicate::str::contains("\"os\"").not())
        .stdout(predicate::str::contains("\"cpu\"").not());
}

#[test]
fn test_filesystem_only_json_excludes_all_other_sections() {
    // Regression test: --filesystem should output ONLY filesystem in JSON
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--filesystem", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filesystem\""))
        .stdout(predicate::str::contains("\"hardware\"").not())
        .stdout(predicate::str::contains("\"network\"").not())
        .stdout(predicate::str::contains("\"interfaces\"").not())
        .stdout(predicate::str::contains("\"os\"").not());
}

#[test]
fn test_hardware_network_json_excludes_filesystem() {
    // Regression test: --hardware --network should exclude filesystem
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--hardware", "--network", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""))
        .stdout(predicate::str::contains("\"network\""))
        .stdout(predicate::str::contains("\"filesystem\"").not());
}

// === Deep flag tests ===
// Tests for the --deep flag which enables deep git inspection (remote operations)

#[test]
fn test_deep_flag_in_help() {
    // Verify --deep flag appears in help output with appropriate description
    Command::cargo_bin("sniff")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--deep"))
        .stdout(predicate::str::contains("remote").or(predicate::str::contains("git")));
}

#[test]
fn test_deep_flag_parses_correctly() {
    // Verify --deep flag is properly parsed and command succeeds
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--deep", "--filesystem"])
        .assert()
        .success();
}

#[test]
fn test_deep_flag_with_json_output() {
    // Verify --deep with JSON output produces valid JSON with git fields
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--deep", "--filesystem", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filesystem\""))
        .stdout(predicate::str::contains("\"recent\"")); // new commits array field
}

#[test]
fn test_filesystem_json_contains_new_git_fields() {
    // Verify JSON output contains the new git-related fields
    // These fields should exist even without --deep flag
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--filesystem", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"recent\"")) // commits array
        .stdout(predicate::str::contains("\"in_worktree\"")) // boolean flag
        .stdout(predicate::str::contains("\"worktrees\"")); // worktrees object
}

#[test]
fn test_deep_flag_does_not_affect_non_filesystem() {
    // --deep flag should not break other sections
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--deep", "--hardware", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""))
        .stdout(predicate::str::contains("\"os\""));
}

#[test]
fn test_verbose_with_deep_shows_git_info() {
    // Verbose output with --deep should show git repository section
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--deep", "--filesystem", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Git Repository:").or(predicate::str::contains("git")));
}

#[test]
fn test_deep_and_verbose_combined() {
    // Both --deep and -vv should work together without errors
    Command::cargo_bin("sniff")
        .unwrap()
        .args(["--deep", "--filesystem", "-vv"])
        .assert()
        .success();
}
