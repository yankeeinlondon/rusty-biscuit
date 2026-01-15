use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    cargo_bin_cmd!("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detect system"));
}

#[test]
fn test_version_flag() {
    cargo_bin_cmd!("sniff")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff"));
}

#[test]
fn test_json_output() {
    cargo_bin_cmd!("sniff")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""));
}

#[test]
fn test_text_output() {
    // Default output is text
    cargo_bin_cmd!("sniff")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Hardware ==="));
}

#[test]
fn test_base_dir_flag() {
    cargo_bin_cmd!("sniff")
        .args(["--base", "."])
        .assert()
        .success();
}

#[test]
fn test_skip_hardware_flag() {
    // Skipped sections are omitted from JSON output entirely
    cargo_bin_cmd!("sniff")
        .args(["--skip-hardware", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\"").not());
}

#[test]
fn test_skip_network_flag() {
    // Skipped sections are omitted from JSON output entirely
    cargo_bin_cmd!("sniff")
        .args(["--skip-network", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"network\"").not());
}

#[test]
fn test_skip_filesystem_flag() {
    // Skipped sections are omitted from JSON output entirely
    cargo_bin_cmd!("sniff")
        .args(["--skip-filesystem", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filesystem\"").not());
}

#[test]
fn test_verbose_flag() {
    // -v should show more details
    cargo_bin_cmd!("sniff")
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("Total:"));
}

#[test]
fn test_double_verbose_flag() {
    // -vv should work (higher verbosity)
    cargo_bin_cmd!("sniff")
        .arg("-vv")
        .assert()
        .success();
}

// === Include-only mode tests ===

#[test]
fn test_hardware_include_only_flag() {
    // --hardware should output only hardware section
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--deep"))
        .stdout(predicate::str::contains("remote").or(predicate::str::contains("git")));
}

#[test]
fn test_deep_flag_parses_correctly() {
    // Verify --deep flag is properly parsed and command succeeds
    cargo_bin_cmd!("sniff")
        .args(["--deep", "--filesystem"])
        .assert()
        .success();
}

#[test]
fn test_deep_flag_with_json_output() {
    // Verify --deep with JSON output produces valid JSON with git fields
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
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
    cargo_bin_cmd!("sniff")
        .args(["--deep", "--hardware", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""))
        .stdout(predicate::str::contains("\"os\""));
}

#[test]
fn test_verbose_with_deep_shows_git_info() {
    // Verbose output with --deep should show git repository section
    cargo_bin_cmd!("sniff")
        .args(["--deep", "--filesystem", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Git Repository:").or(predicate::str::contains("git")));
}

#[test]
fn test_deep_and_verbose_combined() {
    // Both --deep and -vv should work together without errors
    cargo_bin_cmd!("sniff")
        .args(["--deep", "--filesystem", "-vv"])
        .assert()
        .success();
}

// ============================================================================
// Regression tests for filter flags with --json mode
// Bug: Filter flags like --cpu, --gpu were ignored when combined with --json
// ============================================================================

#[test]
fn test_cpu_filter_with_json() {
    // Regression test: --cpu --json should return flattened CPU data at top level
    let output = cargo_bin_cmd!("sniff")
        .args(["--cpu", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have CPU fields at top level (flattened)
    assert!(json.get("brand").is_some(), "brand field should exist at top level");
    assert!(json.get("logical_cores").is_some(), "logical_cores should exist at top level");
    assert!(json.get("simd").is_some(), "simd should exist at top level");

    // Should NOT have hardware/os/network/filesystem containers
    assert!(json.get("hardware").is_none(), "hardware container should not exist");
    assert!(json.get("os").is_none(), "os section should not exist");
    assert!(json.get("network").is_none(), "network section should not exist");
    assert!(json.get("filesystem").is_none(), "filesystem section should not exist");
}

#[test]
fn test_memory_filter_with_json() {
    // Regression test: --memory --json should return flattened memory data at top level
    let output = cargo_bin_cmd!("sniff")
        .args(["--memory", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have memory fields at top level (flattened)
    assert!(json.get("total_bytes").unwrap().as_u64().unwrap() > 0, "memory should have non-zero total");
    assert!(json.get("available_bytes").is_some(), "available_bytes should exist");
    assert!(json.get("used_bytes").is_some(), "used_bytes should exist");

    // Should NOT have hardware/os/network/filesystem containers
    assert!(json.get("hardware").is_none(), "hardware container should not exist");
    assert!(json.get("os").is_none());
    assert!(json.get("network").is_none());
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_gpu_filter_with_json() {
    // Regression test: --gpu --json should return flattened GPU array at top level
    let output = cargo_bin_cmd!("sniff")
        .args(["--gpu", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Top level should be an array (GPU list)
    assert!(json.is_array(), "GPU output should be an array at top level");

    // The array might be empty on systems without GPU, but structure should be correct
    let gpu_array = json.as_array().unwrap();
    if !gpu_array.is_empty() {
        // If GPU exists, check it has expected fields
        assert!(gpu_array[0].get("name").is_some() || gpu_array[0].get("backend").is_some());
    }
}

#[test]
fn test_storage_filter_with_json() {
    // Regression test: --storage --json should return flattened storage array at top level
    let output = cargo_bin_cmd!("sniff")
        .args(["--storage", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Top level should be an array (storage/disk list)
    assert!(json.is_array(), "Storage output should be an array at top level");

    // Should have at least one disk
    let storage = json.as_array().unwrap();
    assert!(storage.len() > 0, "storage should have at least one disk");

    // Check first disk has expected fields
    assert!(storage[0].get("mount_point").is_some() || storage[0].get("file_system").is_some());
}

#[test]
fn test_git_filter_with_json() {
    // Regression test: --git --json should return flattened git data at top level
    let output = cargo_bin_cmd!("sniff")
        .args(["--git", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have git fields at top level (flattened)
    assert!(json.get("repo_root").is_some() || json.get("current_branch").is_some(), "git data should exist at top level");

    // Should NOT have filesystem/hardware/os/network containers
    assert!(json.get("filesystem").is_none(), "filesystem container should not exist");
    assert!(json.get("hardware").is_none());
    assert!(json.get("os").is_none());
    assert!(json.get("network").is_none());
}

#[test]
fn test_repo_filter_with_json() {
    // Regression test: --repo --json should return flattened repo data at top level
    let output = cargo_bin_cmd!("sniff")
        .args(["--repo", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Top level should have repo fields (might be null if no repo structure detected)
    // Check for typical repo fields like "root", "is_monorepo", "packages"
    assert!(json.is_object() || json.is_null(), "repo output should be object or null at top level");

    // Should NOT have filesystem/hardware/os/network containers
    assert!(json.get("filesystem").is_none(), "filesystem container should not exist");
    assert!(json.get("hardware").is_none());
    assert!(json.get("os").is_none());
    assert!(json.get("network").is_none());
}

#[test]
fn test_language_filter_with_json() {
    // Regression test: --language --json should return flattened language data at top level
    let output = cargo_bin_cmd!("sniff")
        .args(["--language", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Top level should have language fields (might be null if no languages detected)
    // Check for typical language breakdown fields like "total_files", "languages", "primary"
    assert!(json.is_object() || json.is_null(), "language output should be object or null at top level");

    // Should NOT have filesystem/hardware/os/network containers
    assert!(json.get("filesystem").is_none(), "filesystem container should not exist");
    assert!(json.get("hardware").is_none());
    assert!(json.get("os").is_none());
    assert!(json.get("network").is_none());
}

#[test]
fn test_os_filter_with_json() {
    // Regression test: --os --json should filter to only OS data
    let output = cargo_bin_cmd!("sniff")
        .args(["--os", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have only os section
    assert!(json.get("os").is_some());
    assert!(json.get("hardware").is_none());
    assert!(json.get("network").is_none());
    assert!(json.get("filesystem").is_none());

    // OS should have data
    let os = json.get("os").unwrap();
    assert!(os.get("name").is_some());
    assert!(os.get("kernel").is_some());
}
