use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;

// ============================================================================
// Help and Version Tests
// ============================================================================

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
fn test_help_mentions_subcommands() {
    cargo_bin_cmd!("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("sniff os"))
        .stdout(predicate::str::contains("sniff cpu"))
        .stdout(predicate::str::contains("sniff hardware"))
        .stdout(predicate::str::contains("sniff agents"));
}

// ============================================================================
// Shell Completions Tests
// ============================================================================

#[test]
fn test_completions_bash_shows_setup() {
    cargo_bin_cmd!("sniff")
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source <(COMPLETE=bash sniff)"))
        .stdout(predicate::str::contains("~/.bashrc"));
}

#[test]
fn test_completions_zsh_shows_setup() {
    cargo_bin_cmd!("sniff")
        .args(["--completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source <(COMPLETE=zsh sniff)"))
        .stdout(predicate::str::contains("~/.zshrc"));
}

#[test]
fn test_completions_fish_shows_setup() {
    cargo_bin_cmd!("sniff")
        .args(["--completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("COMPLETE=fish sniff | source"))
        .stdout(predicate::str::contains("config.fish"));
}

#[test]
fn test_completions_powershell_shows_setup() {
    cargo_bin_cmd!("sniff")
        .args(["--completions", "powershell"])
        .assert()
        .success()
        .stdout(predicate::str::contains("$env:COMPLETE"))
        .stdout(predicate::str::contains("$PROFILE"));
}

#[test]
fn test_dynamic_completions_bash() {
    cargo_bin_cmd!("sniff")
        .env("COMPLETE", "bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_clap_complete_sniff"))
        .stdout(predicate::str::contains("COMPREPLY"));
}

#[test]
fn test_dynamic_completions_zsh() {
    cargo_bin_cmd!("sniff")
        .env("COMPLETE", "zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef sniff"))
        .stdout(predicate::str::contains("_clap_dynamic_completer_sniff"));
}

#[test]
fn test_dynamic_completions_fish() {
    cargo_bin_cmd!("sniff")
        .env("COMPLETE", "fish")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "complete --keep-order --exclusive --command sniff",
        ));
}

#[test]
fn test_help_mentions_completions() {
    cargo_bin_cmd!("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--completions").not())
        .stdout(predicate::str::contains("Shell completions").not());
}

#[test]
fn test_completions_help_flag_shows_setup() {
    cargo_bin_cmd!("sniff")
        .args(["--completions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Shell completions"))
        .stdout(predicate::str::contains("sniff --completions"))
        .stdout(predicate::str::contains("COMPLETE=bash sniff"));
}

// ============================================================================
// Output Mode Tests
// No subcommand = show help
// No subcommand + --json = JSON output (all data)
// With subcommand = text output by default, --json for JSON
// ============================================================================

#[test]
fn test_no_subcommand_shows_help() {
    // Without a subcommand, the output should be the help text
    cargo_bin_cmd!("sniff")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("sniff os"));
}

#[test]
fn test_no_subcommand_with_json_outputs_json() {
    // Without a subcommand but with --json, the output should be JSON
    cargo_bin_cmd!("sniff")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""))
        .stdout(predicate::str::contains("\"os\""));
}

#[test]
fn test_subcommand_outputs_text_by_default() {
    // With a subcommand (os), the output should be text by default
    cargo_bin_cmd!("sniff")
        .arg("os")
        .assert()
        .success()
        .stdout(predicate::str::contains("Operating System"));
}

#[test]
fn test_subcommand_with_json_flag_outputs_json() {
    // With a subcommand and --json, output should be JSON
    cargo_bin_cmd!("sniff")
        .args(["os", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"kernel\""));
}

// ============================================================================
// Global Flag Position Tests
// Global flags should work before or after subcommand
// ============================================================================

#[test]
fn test_json_flag_before_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["--json", "cpu"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"brand\""))
        .stdout(predicate::str::contains("\"logical_cores\""));
}

#[test]
fn test_json_flag_after_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["cpu", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"brand\""))
        .stdout(predicate::str::contains("\"logical_cores\""));
}

#[test]
fn test_verbose_flag_before_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["-v", "cpu"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== CPU ==="));
}

#[test]
fn test_verbose_flag_after_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["cpu", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== CPU ==="));
}

#[test]
fn test_double_verbose_flag() {
    cargo_bin_cmd!("sniff")
        .args(["cpu", "-vv"])
        .assert()
        .success();
}

#[test]
fn with_network_flag_parses() {
    // The flag should be accepted globally; pair with a fast subcommand
    // so the test doesn't pay full-detection cost.
    cargo_bin_cmd!("sniff")
        .args(["--with-network", "repo", "name"])
        .assert()
        .success();
}

#[test]
fn with_network_flag_parses_before_json() {
    cargo_bin_cmd!("sniff")
        .args(["--with-network", "repo", "name", "--json"])
        .assert()
        .success();
}

#[test]
fn repo_name_json_is_leaf_only() {
    let output = cargo_bin_cmd!("sniff")
        .args(["repo", "name", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");

    let obj = json.as_object().expect("json should be an object");

    // The only key allowed is `name`. No version, language, is_monorepo,
    // or package_count may appear at the leaf level.
    assert_eq!(
        obj.len(),
        1,
        "repo name --json must contain exactly one key; got: {json}"
    );
    assert!(
        obj.contains_key("name"),
        "repo name --json must contain `name`: {json}"
    );
    assert!(
        obj.get("name").and_then(|v| v.as_str()).is_some(),
        "`name` must be a string: {json}"
    );

    for forbidden in ["version", "language", "is_monorepo", "package_count"] {
        assert!(
            !obj.contains_key(forbidden),
            "repo name --json must NOT contain `{forbidden}`: {json}"
        );
    }
}

#[test]
fn test_base_flag_before_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["-b", ".", "filesystem"])
        .assert()
        .success();
}

#[test]
fn test_base_flag_after_subcommand_is_accepted() {
    cargo_bin_cmd!("sniff")
        .args(["filesystem", "-b", "."])
        .assert()
        .success();
}

#[test]
fn test_filesystem_scoped_flags_parse_in_help() {
    cargo_bin_cmd!("sniff")
        .args(["filesystem", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--refresh-remotes"))
        .stdout(predicate::str::contains("--latest-versions"));
}

#[test]
fn test_repo_scoped_flags_parse_in_help() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest-versions"))
        .stdout(predicate::str::contains("deps"))
        .stdout(predicate::str::contains("packages"))
        .stdout(predicate::str::contains("package-area"))
        .stdout(predicate::str::contains("dirty-packages"))
        .stdout(predicate::str::contains("dirty-package-areas"))
        .stdout(predicate::str::contains("--refresh-remotes").not());
}

#[test]
fn test_topics_subcommand_output() {
    cargo_bin_cmd!("sniff")
        .arg("topics")
        .assert()
        .success()
        .stdout(predicate::str::contains("hardware"))
        .stdout(predicate::str::contains("filesystem"))
        .stdout(predicate::str::contains("programs"));
}

// ============================================================================
// Top-Level Section Subcommand Tests
// os, hardware, network, filesystem
// ============================================================================

#[test]
fn test_os_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("os")
        .assert()
        .success()
        .stdout(predicate::str::contains("Operating System"))
        .stdout(predicate::str::contains("Name:"))
        .stdout(predicate::str::contains("Kernel:"));
}

#[test]
fn test_os_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["os", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have OS fields at top level (flattened)
    assert!(json.get("name").is_some(), "name should be at top level");
    assert!(
        json.get("kernel").is_some(),
        "kernel should be at top level"
    );
    assert!(
        json.get("hostname").is_some(),
        "hostname should be at top level"
    );

    // Should NOT have wrapper or other sections
    assert!(json.get("os").is_none(), "os wrapper should not exist");
    assert!(json.get("hardware").is_none());
    assert!(json.get("network").is_none());
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_hardware_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("hardware")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Hardware ==="))
        .stdout(predicate::str::contains("CPU:"))
        .stdout(predicate::str::contains("Memory:"));
}

#[test]
fn test_hardware_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["hardware", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have hardware fields at top level (flattened)
    assert!(json.get("cpu").is_some(), "cpu should be at top level");
    assert!(json.get("gpu").is_some(), "gpu should be at top level");
    assert!(
        json.get("memory").is_some(),
        "memory should be at top level"
    );
    assert!(
        json.get("storage").is_some(),
        "storage should be at top level"
    );

    // Should NOT have wrapper or other sections
    assert!(
        json.get("hardware").is_none(),
        "hardware wrapper should not exist"
    );
    assert!(json.get("os").is_none());
    assert!(json.get("network").is_none());
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_network_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("network")
        .assert()
        .success()
        .stdout(predicate::str::contains("Network"))
        .stdout(predicate::str::contains("Primary interface:"))
        .stdout(predicate::str::contains("##").not());
}

#[test]
fn test_network_subcommand_verbose_text_output() {
    cargo_bin_cmd!("sniff")
        .args(["network", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Interfaces"))
        .stdout(predicate::str::contains("##").not());
}

#[test]
fn test_network_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["network", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have network fields at top level (flattened)
    assert!(
        json.get("interfaces").is_some(),
        "interfaces should be at top level"
    );
    assert!(
        json.get("permission_denied").is_some(),
        "permission_denied should be at top level"
    );
    assert!(
        json.get("wan_ip_address").is_some(),
        "wan_ip_address should be at top level"
    );

    // Should NOT have wrapper or other sections
    assert!(
        json.get("network").is_none(),
        "network wrapper should not exist"
    );
    assert!(json.get("os").is_none());
    assert!(json.get("hardware").is_none());
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_filesystem_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("filesystem")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Filesystem ==="));
}

#[test]
fn test_filesystem_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["filesystem", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have filesystem fields at top level (flattened)
    assert!(json.get("git").is_some(), "git should be at top level");

    // Should NOT have wrapper or other sections
    assert!(
        json.get("filesystem").is_none(),
        "filesystem wrapper should not exist"
    );
    assert!(json.get("os").is_none());
    assert!(json.get("hardware").is_none());
    assert!(json.get("network").is_none());
}

// ============================================================================
// Hardware Detail Subcommand Tests
// cpu, gpu, memory, storage
// ============================================================================

#[test]
fn test_cpu_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("cpu")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== CPU ==="))
        .stdout(predicate::str::contains("Brand:"))
        .stdout(predicate::str::contains("Logical cores:"));
}

#[test]
fn test_cpu_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["cpu", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have CPU fields at top level (flattened)
    assert!(json.get("brand").is_some(), "brand should be at top level");
    assert!(
        json.get("logical_cores").is_some(),
        "logical_cores should be at top level"
    );
    assert!(json.get("simd").is_some(), "simd should be at top level");

    // Should NOT have wrappers
    assert!(json.get("cpu").is_none(), "cpu wrapper should not exist");
    assert!(json.get("hardware").is_none());
}

#[test]
fn test_gpu_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("gpu")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== GPU ==="));
}

#[test]
fn test_gpu_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["gpu", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Top level should be an array (GPU list)
    assert!(
        json.is_array(),
        "GPU output should be an array at top level"
    );
}

#[test]
fn test_memory_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("memory")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Memory ==="))
        .stdout(predicate::str::contains("Total:"))
        .stdout(predicate::str::contains("Available:"));
}

#[test]
fn test_memory_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["memory", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have memory fields at top level (flattened)
    assert!(
        json.get("total_bytes").is_some(),
        "total_bytes should be at top level"
    );
    assert!(
        json.get("available_bytes").is_some(),
        "available_bytes should be at top level"
    );
    assert!(
        json.get("used_bytes").is_some(),
        "used_bytes should be at top level"
    );

    // Should NOT have wrappers
    assert!(
        json.get("memory").is_none(),
        "memory wrapper should not exist"
    );
    assert!(json.get("hardware").is_none());
}

#[test]
fn test_storage_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("storage")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Storage ==="));
}

#[test]
fn test_storage_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["storage", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Top level should be an array (storage/disk list)
    assert!(
        json.is_array(),
        "Storage output should be an array at top level"
    );

    // Should have at least one disk
    let storage = json.as_array().unwrap();
    assert!(!storage.is_empty(), "storage should have at least one disk");
}

// ============================================================================
// Filesystem Detail Subcommand Tests
// git, repo, language
// ============================================================================

#[test]
fn test_git_status_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "git-status"])
        .assert()
        .success()
        // Rich output format has Status and Meta sections
        .stdout(predicate::str::contains("Status"))
        .stdout(predicate::str::contains("Meta"));
}

#[test]
fn test_git_status_subcommand_with_history_flag() {
    // Test that the --history flag is accepted
    cargo_bin_cmd!("sniff")
        .args(["repo", "git-status", "--history", "3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status"));
}

#[test]
fn test_git_status_subcommand_compact_output() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "git-status", "--compact"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status"))
        .stdout(predicate::str::contains("\x1b[1m\x1b[4mMeta").not());
}

#[test]
fn test_git_status_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["repo", "git-status", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // `repo git-status --json` returns a focused `GitInfo` object — not the
    // full `RepoInfo` blob. Top-level keys mirror `GitInfo` fields.
    assert!(json.is_object(), "git-status JSON should be an object");
    assert!(
        json.get("repo_root").is_some(),
        "git-status JSON should have top-level `repo_root`: {json}"
    );
    assert!(
        json.get("status").is_some(),
        "git-status JSON should have top-level `status`: {json}"
    );
    assert!(
        json.get("recent").is_some(),
        "git-status JSON should have top-level `recent`: {json}"
    );
    assert!(
        json.get("branches").is_some(),
        "git-status JSON should have top-level `branches`: {json}"
    );

    // RepoInfo-only fields must not leak into git-status JSON.
    assert!(
        json.get("is_monorepo").is_none(),
        "git-status JSON should NOT contain RepoInfo `is_monorepo`: {json}"
    );
    assert!(
        json.get("packages").is_none(),
        "git-status JSON should NOT contain RepoInfo `packages`: {json}"
    );
}

#[test]
fn test_repo_subcommand_text_output() {
    cargo_bin_cmd!("sniff").arg("repo").assert().success();
}

#[test]
fn test_repo_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["repo", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should be object or null at top level
    assert!(
        json.is_object() || json.is_null(),
        "repo output should be object or null at top level"
    );

    // Should NOT have wrappers
    assert!(json.get("repo").is_none(), "repo wrapper should not exist");
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_language_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "language", "--breakdown"])
        .assert()
        .success();
}

#[test]
fn test_language_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["repo", "language", "--breakdown", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should be object or null at top level
    assert!(
        json.is_object() || json.is_null(),
        "language output should be object or null at top level"
    );

    // Should NOT have wrappers
    assert!(
        json.get("language").is_none(),
        "language wrapper should not exist"
    );
    assert!(json.get("filesystem").is_none());
}

// ============================================================================
// `sniff repo language` Subcommand Tests (review-plan-1, Phase 2)
// Pins:
//   - text output exact contract: `Rust\n` / empty + exit 1
//   - JSON output exact contract: `{"language":"Rust"}` / `{"language":null}` + exit 1
//   - `--base` works in all three placements (global pre, repo-nested, leaf)
// ============================================================================

#[test]
fn test_repo_language_text_returns_rust_for_rust_repo() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "language"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout, "Rust\n", "expected exact `Rust\\n` output");
}

#[test]
fn test_repo_language_json_returns_rust_for_rust_repo() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "language",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap().trim_end();
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("repo language --json must emit valid JSON");

    // Exact shape contract: object with single key "language" → "Rust".
    assert_eq!(parsed, serde_json::json!({ "language": "Rust" }));
}

#[test]
fn test_repo_language_base_flag_all_three_placements() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let base = path.to_str().unwrap();

    // Placement A: `sniff --base <repo> repo language` (global, before subcommand)
    let a = cargo_bin_cmd!("sniff")
        .args(["--base", base, "repo", "language"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        String::from_utf8(a).unwrap(),
        "Rust\n",
        "placement A failed"
    );

    // Placement B: `sniff repo --base <repo> language` (between repo and leaf)
    let b = cargo_bin_cmd!("sniff")
        .args(["repo", "--base", base, "language"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        String::from_utf8(b).unwrap(),
        "Rust\n",
        "placement B failed"
    );

    // Placement C: `sniff repo language --base <repo>` (after the leaf subcommand)
    let c = cargo_bin_cmd!("sniff")
        .args(["repo", "language", "--base", base])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        String::from_utf8(c).unwrap(),
        "Rust\n",
        "placement C failed"
    );
}

#[test]
fn test_repo_language_text_empty_repo_exits_one_with_no_stdout() {
    // create_test_repo creates a git repo with one empty initial commit
    // and no source files — primary language detection returns None.
    let (_dir, path) = create_test_repo();

    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "language"])
        .assert()
        .failure() // exit 1 by Phase 1 contract
        .code(1);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(
        stdout, "",
        "text mode must emit no stdout when no language detected"
    );
}

#[test]
fn test_repo_language_json_empty_repo_emits_null_and_exits_one() {
    let (_dir, path) = create_test_repo();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "language",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);

    let stdout = assert.get_output().stdout.clone();
    let json_str = std::str::from_utf8(&stdout).unwrap().trim_end();
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .expect("repo language --json must emit valid JSON even when null");
    assert_eq!(parsed, serde_json::json!({ "language": null }));
}

#[test]
fn test_repo_help_lists_language_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff repo language"));
}

// ============================================================================
// Programs Subcommand Tests
// programs, editors, utilities, language-package-managers, os-package-managers,
// tts-clients, terminal-apps, audio
// ============================================================================

#[test]
fn test_programs_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the programs table. Accept either the rendered table
    // or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .arg("programs")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_programs_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["programs", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let entries = json
        .as_array()
        .expect("programs --json should return an array");
    assert!(!entries.is_empty(), "programs JSON should not be empty");

    let first = entries[0]
        .as_object()
        .expect("programs JSON entries should be objects");
    assert!(first.contains_key("name"));
    assert!(first.contains_key("binary_name"));
    assert!(first.contains_key("description"));
    assert!(first.contains_key("website"));
}

#[test]
fn test_programs_subcommand_rejects_json_format_flag() {
    cargo_bin_cmd!("sniff")
        .args(["programs", "--json-format", "full"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unexpected argument '--json-format'",
        ));
}

#[test]
fn test_editors_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the editors table. Accept either the rendered table
    // or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .arg("editors")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_editors_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["editors", "--json"])
        .assert()
        .success();
}

#[test]
fn test_utilities_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the utilities table. Accept either the rendered table
    // or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .arg("utilities")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_utilities_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["utilities", "--json"])
        .assert()
        .success();
}

#[test]
fn test_language_package_managers_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the language-package-managers table. Accept either the
    // rendered table or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .arg("language-package-managers")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_language_package_managers_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["language-package-managers", "--json"])
        .assert()
        .success();
}

#[test]
fn test_os_package_managers_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("os-package-managers")
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
}

#[test]
fn test_os_package_managers_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["os-package-managers", "--json"])
        .assert()
        .success();
}

#[test]
fn test_tts_clients_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the tts-clients table. Accept either the rendered table
    // or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .arg("tts-clients")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_tts_clients_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["tts-clients", "--json"])
        .assert()
        .success();
}

#[test]
fn test_terminal_apps_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the terminal-apps table. Accept either the rendered table
    // or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .arg("terminal-apps")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_terminal_apps_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["terminal-apps", "--json"])
        .assert()
        .success();
}

#[test]
fn test_audio_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the audio-players table. Accept either the rendered table
    // or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .arg("audio-players")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_audio_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["audio-players", "--json"])
        .assert()
        .success();
}

// ============================================================================
// Services Subcommand Tests
// ============================================================================

#[test]
fn test_services_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("services")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Services ==="))
        .stdout(predicate::str::contains("Init System:"));
}

#[test]
fn test_services_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["services", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("init_system"))
        .stdout(predicate::str::contains("services"));
}

#[test]
fn test_services_state_all() {
    cargo_bin_cmd!("sniff")
        .args(["services", "--state", "all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Services:"));
}

#[test]
fn test_services_state_running() {
    cargo_bin_cmd!("sniff")
        .args(["services", "--state", "running"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Running Services:"));
}

#[test]
fn test_services_state_stopped() {
    cargo_bin_cmd!("sniff")
        .args(["services", "--state", "stopped"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped Services:"));
}

// ============================================================================
// Scoped Enrichment Flag Tests
// ============================================================================

#[test]
fn test_enrichment_flags_in_help() {
    // Top-level help should mention --plain and repo, but not --deep
    cargo_bin_cmd!("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--deep").not())
        .stdout(predicate::str::contains("--plain"))
        .stdout(predicate::str::contains("sniff repo"));
}

#[test]
fn test_filesystem_help_mentions_scoped_flags() {
    cargo_bin_cmd!("sniff")
        .args(["filesystem", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--refresh-remotes"))
        .stdout(predicate::str::contains("--latest-versions"));
}

#[test]
fn test_git_status_help_mentions_refresh_remotes() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "git-status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--refresh-remotes"))
        .stdout(predicate::str::contains("--compact"));
}

#[test]
fn test_repo_help_mentions_latest_versions() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest-versions"))
        .stdout(predicate::str::contains("--refresh-remotes").not());
}

#[test]
fn test_git_status_json_is_git_info() {
    // Verify JSON output is a `GitInfo` object — not the full `RepoInfo`
    // blob. The top-level `repo_root` field is unique to `GitInfo`'s shape
    // (RepoInfo serializes its root field as `root`).
    cargo_bin_cmd!("sniff")
        .args(["repo", "git-status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("repo_root"))
        .stdout(predicate::str::contains("\"is_monorepo\"").not())
        .stdout(predicate::str::contains("\"packages\"").not());
}

#[test]
fn test_repo_deps_help_mentions_ui() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "deps", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--ui"));
}

#[test]
fn test_invalid_refresh_remotes_on_remote_subcommand_fails() {
    // --refresh-remotes is only valid on git-status, not on remote
    cargo_bin_cmd!("sniff")
        .args(["repo", "remote", "origin", "--refresh-remotes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--refresh-remotes"));
}

// ============================================================================
// Verbose Flag Tests with Subcommands
// ============================================================================

#[test]
fn test_verbose_with_programs_adds_columns() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the verbose programs table. Accept either the rendered table
    // or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .args(["programs", "-v"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Binary")
                .or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_verbose_with_hardware_shows_details() {
    cargo_bin_cmd!("sniff")
        .args(["hardware", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Total:"));
}

// ============================================================================
// Invalid Subcommand Tests
// ============================================================================

#[test]
fn test_invalid_subcommand_fails() {
    cargo_bin_cmd!("sniff")
        .arg("invalid-subcommand")
        .assert()
        .failure();
}

#[test]
fn test_old_flag_syntax_fails() {
    // Old --hardware flag should not work (not a valid subcommand or flag)
    cargo_bin_cmd!("sniff").arg("--hardware").assert().failure();
}

// ============================================================================
// Remote Subcommand Tests
// ============================================================================

#[test]
fn test_repo_remote_help() {
    // Remote subcommand is documented via `sniff repo --help`
    cargo_bin_cmd!("sniff")
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("remote"));
}

#[test]
fn test_help_mentions_remote_via_repo() {
    // Remote inspection is now under `sniff repo --help`
    cargo_bin_cmd!("sniff")
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff repo remote origin"))
        .stdout(predicate::str::contains("Inspect the 'origin' remote"));
}

// ============================================================================
// Install Subcommand Tests
// ============================================================================

#[test]
fn test_editors_still_shows_table_without_install() {
    // Backward compat: `sniff editors` still produces table output
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the editors table. Accept either the rendered table
    // or the graceful width error message.
    cargo_bin_cmd!("sniff")
        .arg("editors")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_editors_install_invalid_name_fails() {
    cargo_bin_cmd!("sniff")
        .args(["editors", "install", "nonexistent-editor-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown editor"))
        .stderr(predicate::str::contains("Valid names:"));
}

#[test]
fn test_utilities_install_invalid_name_fails() {
    cargo_bin_cmd!("sniff")
        .args(["utilities", "install", "nonexistent-util-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown utility"))
        .stderr(predicate::str::contains("Valid names:"));
}

#[test]
fn test_programs_install_invalid_name_fails() {
    cargo_bin_cmd!("sniff")
        .args(["programs", "install", "nonexistent-program-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown program"));
}

#[test]
fn test_editors_install_help_works() {
    cargo_bin_cmd!("sniff")
        .args(["editors", "install", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Install a program"));
}

#[test]
fn test_help_mentions_install() {
    // Top-level help mentions editors with install support
    cargo_bin_cmd!("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff editors"));
}

#[test]
fn test_editors_json_still_works_with_install_subcommand() {
    // --json flag should still work for listing (no install action)
    cargo_bin_cmd!("sniff")
        .args(["editors", "--json"])
        .assert()
        .success();
}

// ============================================================================
// --plain flag tests
// ============================================================================

#[test]
fn test_plain_flag_strips_escape_codes() {
    let output = cargo_bin_cmd!("sniff")
        .args(["os", "--plain"])
        .output()
        .expect("failed to run sniff os --plain");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ANSI escape codes start with \x1b[
    assert!(
        !stdout.contains("\x1b["),
        "Plain output should not contain ANSI escape codes"
    );
}

#[test]
fn test_plain_with_json_ignores_plain() {
    // --plain --json should produce normal JSON (plain is irrelevant for JSON)
    cargo_bin_cmd!("sniff")
        .args(["os", "--plain", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""));
}

// ============================================================================
// ============================================================================
// Repo subcommand tests (Phase 1 verification)
// ============================================================================

#[test]
fn test_repo_git_status_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "git-status"])
        .assert()
        .success();
}

#[test]
fn test_repo_help_shows_examples() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff repo git-status"))
        .stdout(predicate::str::contains("sniff repo hash"))
        .stdout(predicate::str::contains("sniff repo staged-files"));
}

// ============================================================================
// Blast-radius CLI integration tests (temp-repo based)
// ============================================================================

/// Create a temp git repo with an initial commit.
fn create_test_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    let sig = repo.signature().unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Commit a file to the test repo.
fn test_commit_file(repo_path: &Path, relative: &str, content: &str) {
    test_commit_file_with_message(repo_path, relative, content, "add file");
}

/// Commit a file to the test repo with a custom commit message.
fn test_commit_file_with_message(repo_path: &Path, relative: &str, content: &str, message: &str) {
    let full = repo_path.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&full, content).unwrap();

    let repo = git2::Repository::open(repo_path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();

    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap();
}

/// Overwrite the loose object file for `sha` with garbage so any decode fails.
/// Git creates objects read-only, so make the file writable first.
fn corrupt_loose_object(repo_path: &Path, sha: &str) {
    let obj_path = repo_path
        .join(".git")
        .join("objects")
        .join(&sha[..2])
        .join(&sha[2..]);
    let mut perms = std::fs::metadata(&obj_path).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o644);
    }
    #[cfg(not(unix))]
    perms.set_readonly(false);
    std::fs::set_permissions(&obj_path, perms).unwrap();
    std::fs::write(&obj_path, b"garbage").unwrap();
}

/// Flip the trailing checksum byte of the index so a read detects the mismatch.
fn corrupt_index(repo_path: &Path) {
    let index_path = repo_path.join(".git").join("index");
    let mut bytes = std::fs::read(&index_path).unwrap();
    let len = bytes.len();
    assert!(len >= 20, "index must have a trailing checksum to corrupt");
    bytes[len - 1] = bytes[len - 1].wrapping_add(1);
    std::fs::write(&index_path, bytes).unwrap();
}

/// A corrupt index must surface as a CLI failure through `repo has-merge-conflict`,
/// not be reported as a clean "no conflicts" result.
#[test]
fn test_repo_has_merge_conflict_surfaces_corrupt_index() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    corrupt_index(&path);

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "has-merge-conflict",
        ])
        .assert()
        .failure();
}

/// A corrupt commit object must surface as a CLI failure through `repo hash`,
/// not be reported as "commit not found".
#[test]
fn test_repo_hash_surfaces_corrupt_commit_object() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let repo = git2::Repository::open(&path).unwrap();
    let sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    corrupt_loose_object(&path, &sha);

    cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "hash", &sha])
        .assert()
        .failure();
}

/// Corrupt the HEAD commit object of a freshly-built test repo and return its
/// path, so corruption surfaces through any history-reading command.
fn repo_with_corrupt_head() -> (tempfile::TempDir, PathBuf) {
    let (dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let repo = git2::Repository::open(&path).unwrap();
    let sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    corrupt_loose_object(&path, &sha);
    (dir, path)
}

/// A corrupt commit object must surface as a CLI failure through
/// `repo git-status`, not be reported as a clean, empty history.
#[test]
fn test_repo_git_status_surfaces_corrupt_history() {
    let (_dir, path) = repo_with_corrupt_head();

    cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "git-status"])
        .assert()
        .failure();
}

/// A corrupt commit object must surface as a CLI failure through
/// `repo recent-commits`, not produce a successful but empty list.
#[test]
fn test_repo_recent_commits_surfaces_corrupt_history() {
    let (_dir, path) = repo_with_corrupt_head();

    cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "recent-commits"])
        .assert()
        .failure();
}

/// A corrupt commit object must surface as a CLI failure through
/// `repo source-code-changes`, not produce a successful but empty report.
#[test]
fn test_repo_source_code_changes_surfaces_corrupt_history() {
    let (_dir, path) = repo_with_corrupt_head();

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
        ])
        .assert()
        .failure();
}

/// Stage a file in the test repo (no commit).
fn test_stage_file(repo_path: &Path, relative: &str, content: &str) {
    let full = repo_path.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&full, content).unwrap();

    let repo = git2::Repository::open(repo_path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

#[test]
fn test_repo_dirty_source_code_returns_source_files() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    // Create a dirty source file
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn test_repo_staged_source_code_returns_staged_only() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/a.rs", "a");
    test_commit_file(&path, "src/b.rs", "b");

    // Stage a change to a.rs only
    test_stage_file(&path, "src/a.rs", "a modified");
    // Modify b.rs without staging
    std::fs::write(path.join("src/b.rs"), "b modified").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "staged-source-code",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("a.rs"), "Should contain staged file a.rs");
    assert!(
        !stdout.contains("b.rs"),
        "Should not contain unstaged file b.rs"
    );
}

#[test]
fn test_repo_staged_files_uses_new_path() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "docs/guide.md", "# Guide");

    // Stage changes to both
    test_stage_file(&path, "src/main.rs", "fn main() { updated }");
    test_stage_file(&path, "docs/guide.md", "# Updated Guide");

    // staged-files should now go through the new path (all files, not just source)
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "staged-files",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("main.rs"), "Should contain source file");
    assert!(stdout.contains("guide.md"), "Should contain markdown file");
}

#[test]
fn test_repo_staged_files_json_uses_new_shape() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_stage_file(&path, "src/main.rs", "fn main() { updated }");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "staged-files",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(json["scope"], "staged", "scope should be lowercase");
    assert_eq!(json["kind"], "all_files", "kind should be snake_case");
    let paths = json["paths"].as_array().expect("paths should be an array");
    assert!(
        paths
            .iter()
            .any(|p| p.as_str().unwrap().contains("main.rs"))
    );
}

#[test]
fn test_repo_unstaged_files_json_uses_new_shape() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    // Modify without staging
    std::fs::write(path.join("src/main.rs"), "fn main() { updated }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "unstaged-files",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(json["scope"], "unstaged", "scope should be lowercase");
    assert_eq!(json["kind"], "all_files", "kind should be snake_case");
    let paths = json["paths"].as_array().expect("paths should be an array");
    assert!(
        paths
            .iter()
            .any(|p| p.as_str().unwrap().contains("main.rs"))
    );
}

#[test]
fn test_repo_untracked_files_json_uses_new_shape() {
    let (_dir, path) = create_test_repo();
    // Create a new file without adding it to git
    std::fs::write(path.join("new_file.rs"), "// new").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "untracked-files",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(json["scope"], "untracked", "scope should be lowercase");
    assert_eq!(json["kind"], "all_files", "kind should be snake_case");
    let paths = json["paths"].as_array().expect("paths should be an array");
    assert!(
        paths
            .iter()
            .any(|p| p.as_str().unwrap().contains("new_file.rs"))
    );
}

#[test]
fn test_repo_dirty_files_returns_all_file_types() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "config.json", "{}");

    // Dirty both files
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();
    std::fs::write(path.join("config.json"), "{\"key\": true}").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-files",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("config.json"));
}

#[test]
fn test_repo_file_list_no_results_exits_1() {
    let (_dir, path) = create_test_repo();
    // Commit a file, no dirty files
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
        ])
        .assert()
        .code(1);
}

#[test]
fn test_repo_file_list_no_error_exits_0() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--no-error",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_file_list_on_error_to_stderr() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--on-error",
            "No dirty source code found",
            "--plain",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No dirty source code found"));
}

#[test]
fn test_repo_file_list_on_error_plus_no_error_to_stdout() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--no-error",
            "--on-error",
            "clean!",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("clean!"));
}

#[test]
fn test_blast_radius_dirty_matches_documents() {
    let (_dir, path) = create_test_repo();
    // Commit source and doc
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);

    // Dirty the source file
    std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/guide.md"));
}

#[test]
fn test_blast_radius_staged_matches_documents() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);

    // Stage a modification
    test_stage_file(&path, "src/main.rs", "fn main() { staged }");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "staged",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/guide.md"));
}

#[test]
fn test_blast_radius_last_commit_matches_documents() {
    let (_dir, path) = create_test_repo();
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);
    // Commit the source file last (it will be in HEAD)
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "last-commit",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/guide.md"));
}

#[test]
fn test_blast_radius_no_matches_exits_1() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    // No dirty files -> no blast radius matches

    cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "blast-radius", "dirty"])
        .assert()
        .code(1);
}

#[test]
fn test_blast_radius_no_error_exits_0() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--no-error",
        ])
        .assert()
        .success();
}

#[test]
fn test_blast_radius_on_error_to_stderr() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--on-error",
            "No docs affected",
            "--plain",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No docs affected"));
}

#[test]
fn test_blast_radius_json_output() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);
    std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(json["scope"], "dirty", "scope should be lowercase");
    let docs = json["documents"]
        .as_array()
        .expect("documents should be an array");
    assert_eq!(docs.len(), 1);
    assert_eq!(
        docs[0].as_str().unwrap(),
        "docs/guide.md",
        "documents should be path strings"
    );
}

#[test]
fn test_blast_radius_list_format() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);
    std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--list",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/guide.md"));
}

#[test]
fn test_docs_stdout_stderr_split() {
    let (_dir, path) = create_test_repo();
    test_commit_file(
        &path,
        "docs/readme.md",
        "---\ntitle: Readme\n---\n# Readme\n",
    );

    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "docs", "--plain"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);

    // Header should be on stderr
    assert!(stderr.contains("Docs"), "Header should be on stderr");
    // Document list should be on stdout
    assert!(stdout.contains("readme.md"), "Doc list should be on stdout");
    // Footer should be on stderr
    assert!(stderr.contains("--verbose"), "Footer should be on stderr");
}

#[test]
fn test_docs_blast_radius_filter() {
    let (_dir, path) = create_test_repo();
    // Doc WITH blast_radius
    let doc_with = "---\ntitle: API Guide\nblast_radius:\n  - src/main.rs\n---\n# API Guide\n";
    test_commit_file(&path, "docs/api.md", doc_with);
    // Doc WITHOUT blast_radius
    let doc_without = "---\ntitle: Readme\n---\n# Readme\n";
    test_commit_file(&path, "docs/readme.md", doc_without);

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "docs",
            "--blast-radius",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("api.md"),
        "Should include doc with blast_radius"
    );
    assert!(
        !stdout.contains("readme.md"),
        "Should exclude doc without blast_radius"
    );
}

#[test]
fn test_repo_dirty_source_code_with_list_flag() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--list",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_repo_unstaged_source_code_returns_modified_only() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/a.rs", "a");
    test_commit_file(&path, "src/b.rs", "b");

    // Stage a.rs
    test_stage_file(&path, "src/a.rs", "a staged");
    // Modify b.rs without staging
    std::fs::write(path.join("src/b.rs"), "b modified").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "unstaged-source-code",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("b.rs"), "Should contain unstaged file b.rs");
    assert!(
        !stdout.contains("a.rs"),
        "Should not contain staged file a.rs"
    );
}

// ============================================================================
// Recent Commits CLI Integration Tests (Step 14)
// ============================================================================

#[test]
fn test_repo_recent_commits_default_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "recent-commits"])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_with_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "1d",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_with_count_period() {
    let (_dir, path) = create_test_repo();
    for i in 0..5 {
        test_commit_file(&path, &format!("src/file{i}.rs"), "fn main() {}");
    }

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "2",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let commits = json["commits"].as_array().expect("commits array");
    assert_eq!(commits.len(), 2, "expected exactly 2 commits");
    assert_eq!(
        json["period_label"].as_str().unwrap(),
        "last 2 commits",
        "period label should describe the count"
    );
}

#[test]
fn test_repo_source_code_changes_with_count_period() {
    let (_dir, path) = create_test_repo();
    for i in 0..3 {
        test_commit_file(&path, &format!("src/file{i}.rs"), "fn main() {}");
    }

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "2",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_with_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // JSON output should contain commit fields
    assert!(
        stdout.contains("\"commits\""),
        "JSON should have commits array"
    );
    assert!(
        stdout.contains("\"period_label\""),
        "JSON should have period_label"
    );
}

#[test]
fn test_repo_recent_commits_with_plain() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--plain",
        ])
        .output()
        .expect("failed to run sniff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Plain output should not have ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Plain output should not have ANSI escape codes"
    );
}

#[test]
fn test_repo_source_code_changes() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "1w",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_source_code_changes_with_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("\"commits\""),
        "JSON should have commits array"
    );
}

#[test]
fn test_repo_documentation_changes() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "docs/guide.md", "# Guide\n");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "documentation-changes",
            "1w",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_documentation_changes_with_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "docs/guide.md", "# Guide\n");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "documentation-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("\"commits\""),
        "JSON should have commits array"
    );
}

#[test]
fn test_source_code_changes_json_filters_commits_and_files() {
    // Two commits: one touches a source file, one touches only docs.
    // `source-code-changes --json` must keep only the source commit and
    // tag the payload with `"filter": "source_code"`.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "README.md", "# readme");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    assert_eq!(
        value["filter"], "source_code",
        "source-code-changes --json must include `filter: source_code`: {value}"
    );

    let commits = value["commits"].as_array().expect("commits must be array");
    // Only the source-touching commit should remain after filtering.
    assert_eq!(
        commits.len(),
        1,
        "expected exactly one commit after source-code filtering: {value}"
    );

    // All files left in the kept commit must look like source code.
    for commit in commits {
        let files = commit["files"].as_array().expect("files must be array");
        assert!(!files.is_empty(), "filtered commit must keep its files");
        for file in files {
            let path_str = file["path"].as_str().expect("path is a string");
            assert!(
                !path_str.ends_with(".md"),
                "source-code filter must not keep markdown: {path_str}"
            );
        }
    }
}

#[test]
fn test_documentation_changes_json_filters_commits_and_files() {
    // Two commits: one touches a source file, one touches docs.
    // `documentation-changes --json` must keep only doc commits and tag
    // the payload with `"filter": "documentation"`.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "README.md", "# readme");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "documentation-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    assert_eq!(
        value["filter"], "documentation",
        "documentation-changes --json must include `filter: documentation`: {value}"
    );

    let commits = value["commits"].as_array().expect("commits must be array");
    assert!(
        !commits.is_empty(),
        "expected at least one doc commit: {value}"
    );

    for commit in commits {
        let files = commit["files"].as_array().expect("files must be array");
        assert!(!files.is_empty(), "filtered commit must keep its files");
        for file in files {
            let path_str = file["path"].as_str().expect("path is a string");
            assert!(
                !path_str.ends_with(".rs"),
                "documentation filter must not keep .rs files: {path_str}"
            );
        }
    }
}

#[test]
fn test_filtered_commit_json_trims_packages() {
    // `source-code-changes` and `documentation-changes` should NOT include
    // the full `packages` metadata for brevity.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "README.md", "# readme");

    for (subcommand, label) in [
        ("source-code-changes", "source_code"),
        ("documentation-changes", "documentation"),
    ] {
        let assert = cargo_bin_cmd!("sniff")
            .args([
                "--base",
                path.to_str().unwrap(),
                "repo",
                subcommand,
                "--json",
            ])
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let value: Value = serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

        assert_eq!(
            value["filter"], label,
            "{subcommand} --json must include `filter: {label}`"
        );
        assert!(
            value.get("packages").is_none(),
            "{subcommand} --json must NOT include full `packages` metadata: {value}"
        );
    }
}

#[test]
fn test_recent_commits_json_unchanged() {
    // Regression guard — `recent-commits --json` must NOT include the
    // `filter` field that the filtered variants add.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "README.md", "# readme");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    let obj = value.as_object().expect("payload must be a JSON object");
    assert!(
        !obj.contains_key("filter"),
        "recent-commits --json must NOT include `filter`: {value}"
    );
    assert!(
        obj.contains_key("commits"),
        "recent-commits --json must include `commits`"
    );
    assert!(
        obj.contains_key("period_label"),
        "recent-commits --json must include `period_label`"
    );
}

#[test]
fn test_repo_recent_commits_no_error_flag() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    // Use a future date - valid period that returns no commits
    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "2099-01-01",
            "--no-error",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_invalid_period_error() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "invalid-period",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_recent_commits_on_error_flag() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "2099-01-01",
            "--on-error",
            "No recent commits",
            "--plain",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No recent commits"));
}

// ============================================================================
// Recent Commits CLI — Hash, Package, and Date routing tests
// ============================================================================

/// Create a monorepo-style test repo for CLI testing.
fn create_cli_monorepo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    // Create workspace Cargo.toml
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["pkg-a/lib", "pkg-b/lib"]
"#,
    )
    .unwrap();

    // Package A
    let pkg_a = dir.path().join("pkg-a/lib");
    std::fs::create_dir_all(pkg_a.join("src")).unwrap();
    std::fs::write(
        pkg_a.join("Cargo.toml"),
        r#"[package]
name = "pkg-a"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    std::fs::write(pkg_a.join("src/lib.rs"), "pub fn a() {}").unwrap();

    // Package B
    let pkg_b = dir.path().join("pkg-b/lib");
    std::fs::create_dir_all(pkg_b.join("src")).unwrap();
    std::fs::write(
        pkg_b.join("Cargo.toml"),
        r#"[package]
name = "pkg-b"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    std::fs::write(pkg_b.join("src/lib.rs"), "pub fn b() {}").unwrap();

    // Commit everything
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial monorepo", &tree, &[])
        .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

fn create_cli_monorepo_with_root_package() -> (tempfile::TempDir, PathBuf) {
    let (dir, path) = create_cli_monorepo();
    let repo = git2::Repository::open(&path).unwrap();

    std::fs::write(
        path.join("Cargo.toml"),
        r#"[package]
name = "root-tool"
version = "0.1.0"
edition = "2024"

[workspace]
members = ["pkg-a/lib", "pkg-b/lib", "."]
"#,
    )
    .unwrap();
    std::fs::create_dir_all(path.join("src")).unwrap();
    std::fs::write(path.join("src/lib.rs"), "pub fn root_tool() {}").unwrap();

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "add root workspace package",
        &tree,
        &[&parent],
    )
    .unwrap();

    (dir, path)
}

#[test]
fn test_repo_recent_commits_with_hash_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "src/lib.rs", "pub fn lib() {}");

    let repo = git2::Repository::open(&path).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    // Get the parent commit hash to use as boundary
    let parent = head.parent(0).unwrap();
    let parent_hash = parent.id().to_string();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            &parent_hash,
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "Hash-based query should produce output");
}

#[test]
fn test_repo_recent_commits_with_today_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "today",
            "--plain",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_with_date_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "2020-01-01",
            "--plain",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_action_filter_single_action() {
    let (_dir, path) = create_test_repo();
    test_commit_file_with_message(
        &path,
        "src/feature.rs",
        "pub fn feature() {}",
        "feat(cli): add action filter",
    );
    test_commit_file_with_message(
        &path,
        "src/fix.rs",
        "pub fn fix() {}",
        "fix(cli): tighten recent commit filtering",
    );

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--action",
            "feat",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    assert_eq!(commits.len(), 1, "Only feat commits should remain");
    assert_eq!(
        commits[0]["description"].as_str(),
        Some("feat(cli): add action filter")
    );
}

#[test]
fn test_repo_recent_commits_action_filter_or_semantics() {
    let (_dir, path) = create_test_repo();
    test_commit_file_with_message(
        &path,
        "src/feature.rs",
        "pub fn feature() {}",
        "feat(cli): add action filter",
    );
    test_commit_file_with_message(
        &path,
        "src/refactor.rs",
        "pub fn refactor() {}",
        "refactor(cli): simplify commit filtering",
    );
    test_commit_file_with_message(
        &path,
        "src/fix.rs",
        "pub fn fix() {}",
        "fix(cli): tighten recent commit filtering",
    );

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--action",
            "feat",
            "--action",
            "refactor",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");
    let descriptions: Vec<&str> = commits
        .iter()
        .filter_map(|commit| commit["description"].as_str())
        .collect();

    assert_eq!(
        descriptions.len(),
        2,
        "feat and refactor commits should remain"
    );
    assert!(descriptions.contains(&"feat(cli): add action filter"));
    assert!(descriptions.contains(&"refactor(cli): simplify commit filtering"));
    assert!(!descriptions.contains(&"fix(cli): tighten recent commit filtering"));
}

#[test]
fn test_repo_recent_commits_package_filter() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--package",
            "pkg-a",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        !stdout.is_empty(),
        "Package-filtered query should produce output"
    );
}

#[test]
fn test_repo_recent_commits_package_area_filter() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-b/lib/src/lib.rs", "pub fn b2() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--package-area",
            "pkg-b",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        !stdout.is_empty(),
        "Package-area filtered query should produce output"
    );
}

#[test]
fn test_repo_recent_commits_package_json_scoped() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--package",
            "pkg-a",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // The packages array should only contain the filtered package
    if let Some(packages) = json["packages"].as_array() {
        for pkg in packages {
            assert_eq!(
                pkg["name"], "pkg-a",
                "JSON packages should be scoped to the filter"
            );
        }
    }

    // No files from pkg-b should appear in any commit
    if let Some(commits) = json["commits"].as_array() {
        for commit in commits {
            if let Some(files) = commit["files"].as_array() {
                for file in files {
                    let f = file.as_str().unwrap_or("");
                    assert!(
                        !f.starts_with("pkg-b/"),
                        "Filtered JSON should not contain pkg-b files, got: {}",
                        f
                    );
                }
            }
        }
    }
}

#[test]
fn test_repo_recent_commits_unknown_package_error() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--package",
            "nonexistent",
        ])
        .assert()
        .failure();
}

// ============================================================================
// Recent Commits CLI — Empty commit and exact payload tests
// ============================================================================

#[test]
fn test_repo_recent_commits_json_includes_empty_commits() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    // Create an empty commit on top
    let repo = git2::Repository::open(&path).unwrap();
    let sig = repo.signature().unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "chore: empty marker",
        &tree,
        &[&head],
    )
    .unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    // Find the empty commit
    let empty = commits
        .iter()
        .find(|c| c["description"].as_str() == Some("chore: empty marker"));
    assert!(empty.is_some(), "Empty commit should appear in JSON output");
    let empty = empty.unwrap();
    let files = empty["files"].as_array().expect("Should have files array");
    assert!(files.is_empty(), "Empty commit should have files: []");
}

#[test]
fn test_repo_recent_commits_json_exact_commit_fields() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    // Should have at least 2 commits (initial + add file)
    assert!(
        commits.len() >= 2,
        "Should have at least 2 commits, got {}",
        commits.len()
    );

    // Verify each commit has required fields
    for commit in commits {
        assert!(commit["hash"].is_string(), "Commit should have hash");
        assert!(
            commit["datetime"].is_string(),
            "Commit should have datetime"
        );
        assert!(commit["files"].is_array(), "Commit should have files array");
        assert!(
            commit["description"].is_string(),
            "Commit should have description"
        );
        assert!(
            commit["bullet_points"].is_array(),
            "Commit should have bullet_points"
        );
    }
}

#[test]
fn test_repo_source_code_changes_json_exact_fields() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    // At least one commit should have a .rs file
    let has_rs_file = commits.iter().any(|c| {
        c["files"].as_array().is_some_and(|files| {
            files
                .iter()
                .any(|f| f["path"].as_str().is_some_and(|s| s.ends_with(".rs")))
        })
    });
    assert!(has_rs_file, "Source code changes should include .rs files");
}

#[test]
fn test_repo_documentation_changes_json_exact_fields() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "docs/guide.md", "# Guide\n");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "documentation-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    // At least one commit should have a .md file
    let has_md_file = commits.iter().any(|c| {
        c["files"].as_array().is_some_and(|files| {
            files
                .iter()
                .any(|f| f["path"].as_str().is_some_and(|s| s.ends_with(".md")))
        })
    });
    assert!(
        has_md_file,
        "Documentation changes should include .md files"
    );
}

#[test]
fn test_repo_recent_commits_plain_output_exact_structure() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--plain",
        ])
        .output()
        .expect("failed to run sniff");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Plain output should contain markdown structure
    assert!(
        stdout.contains("[") && stdout.contains("] at "),
        "Plain output should have `[hash] at TIME` commit markers, got:\n{stdout}"
    );
    assert!(
        stdout.contains("**Files Impacted:**"),
        "Plain output should have files section, got:\n{stdout}"
    );
    assert!(
        stdout.contains("add file"),
        "Plain output should include the commit description, got:\n{stdout}"
    );
    assert!(
        stdout.contains("src/main.rs"),
        "Plain output should list the committed file, got:\n{stdout}"
    );
}

// ============================================================================
// repo packages Subcommand Tests
// ============================================================================

#[test]
fn test_repo_packages_csv_default() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a, pkg-b");
}

#[test]
fn test_repo_packages_md_format() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--md",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "- pkg-a\n- pkg-b");
}

#[test]
fn test_repo_packages_list_format() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--list",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a\npkg-b");
}

#[test]
fn test_repo_packages_md_and_list_conflict() {
    let (_dir, path) = create_cli_monorepo();
    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--md",
            "--list",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_packages_package_area_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package-area",
            "pkg-b",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-b");
}

#[test]
fn test_repo_packages_verbose_shows_root_dir() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--list",
            "--verbose",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("pkg-a(./pkg-a/lib)"),
        "Verbose list output should include the package root, got:\n{stdout}"
    );
    assert!(
        stdout.contains("pkg-b(./pkg-b/lib)"),
        "Verbose list output should include the package root, got:\n{stdout}"
    );
}

#[test]
fn test_repo_packages_verbose_does_not_emit_tracing() {
    let (_dir, path) = create_cli_monorepo();
    let output = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--verbose",
        ])
        .output()
        .expect("failed to run sniff");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("performance stage complete"),
        "--verbose must not leak tracing output to stderr, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("INFO"),
        "--verbose must not emit INFO tracing, got:\n{stderr}"
    );
}

#[test]
fn test_repo_packages_json_output() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert_eq!(
        names
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["pkg-a", "pkg-b"]
    );
}

#[test]
fn test_repo_packages_no_error_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    // Filter that matches nothing — without --no-error should exit 1
    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--plain",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_packages_no_error_allows_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    // Filter that matches nothing — with --no-error should exit 0
    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--no-error",
            "--plain",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_packages_on_error_message() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--on-error",
            "nothing here",
            "--plain",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("nothing here"),
        "stderr should contain custom error message, got: {stderr}"
    );
}

#[test]
fn test_repo_packages_no_error_json_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert!(names.is_empty());
}

#[test]
fn test_repo_packages_no_error_json_with_flag() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--no-error",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert!(names.is_empty());
}

// ============================================================================
// repo package-areas Subcommand Tests
// ============================================================================

#[test]
fn test_repo_package_areas_csv_default() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a, pkg-b");
}

#[test]
fn test_repo_package_areas_md_format() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--md",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "- pkg-a\n- pkg-b");
}

#[test]
fn test_repo_package_areas_list_format() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--list",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a\npkg-b");
}

#[test]
fn test_repo_package_areas_md_and_list_conflict() {
    let (_dir, path) = create_cli_monorepo();
    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--md",
            "--list",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_package_areas_package_area_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--package-area",
            "pkg-b",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-b");
}

#[test]
fn test_repo_package_areas_positional_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "pkg-a",
            "--list",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a");
}

#[test]
fn test_repo_package_areas_positional_filter_negation() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "!pkg-a",
            "--list",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-b");
}

#[test]
fn test_repo_package_areas_verbose_shows_root_dir() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--list",
            "--verbose",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("pkg-a (./pkg-a)"),
        "Verbose list output should include the area root, got:\n{stdout}"
    );
    assert!(
        stdout.contains("pkg-b (./pkg-b)"),
        "Verbose list output should include the area root, got:\n{stdout}"
    );
}

#[test]
fn test_repo_package_areas_root_area_verbose_renders_dot_slash() {
    let (_dir, path) = create_cli_monorepo_with_root_package();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--list",
            "--verbose",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("root (./)"),
        "Root package area should render as the repo root, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("root (./root)"),
        "Root package area should not render a non-existent root directory, got:\n{stdout}"
    );
}

#[test]
fn test_repo_package_areas_verbose_does_not_emit_tracing() {
    let (_dir, path) = create_cli_monorepo();
    let output = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--verbose",
        ])
        .output()
        .expect("failed to run sniff");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("performance stage complete"),
        "--verbose must not leak tracing output to stderr, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("INFO"),
        "--verbose must not emit INFO tracing, got:\n{stderr}"
    );
}

#[test]
fn test_repo_package_areas_json_output() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert_eq!(
        names
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["pkg-a", "pkg-b"]
    );
}

#[test]
fn test_repo_package_areas_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--json",
            "--perf",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert_eq!(
        names
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["pkg-a", "pkg-b"]
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.trim().is_empty(),
        "perf output should be written to stderr"
    );
    assert!(
        stderr.contains("Performance") || stderr.contains("Total"),
        "stderr should contain performance timing text, got:\n{stderr}"
    );
}

#[test]
fn test_repo_package_areas_no_error_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--plain",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_package_areas_no_error_allows_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--no-error",
            "--plain",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_package_areas_on_error_message() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--on-error",
            "no areas",
            "--plain",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("no areas"),
        "stderr should contain custom error message, got: {stderr}"
    );
}

#[test]
fn test_repo_package_areas_no_error_json_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert!(names.is_empty());
}

#[test]
fn test_repo_package_areas_no_error_json_with_flag() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--no-error",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert!(names.is_empty());
}

#[test]
fn test_repo_root_json_perf_stdout_is_valid_json() {
    // `repo root --json --perf` must produce parseable JSON on stdout.
    let assert = cargo_bin_cmd!("sniff")
        .args(["repo", "root", "--json", "--perf"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert!(stdout.contains("root"), "should contain root key");
}

#[test]
fn test_repo_dirty_files_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-files",
            "--json",
            "--perf",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
}

#[test]
fn test_repo_recent_commits_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
            "--perf",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert!(stdout.contains("commits"), "should contain commits key");
}

#[test]
fn test_repo_has_merge_conflict_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "has-merge-conflict",
            "--json",
            "--perf",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
}

// ============================================================================
// Phase 2 — Stable JSON shape for `package` / `package-area` / `root`
// when the result resolves to empty.
// ============================================================================
//
// JSON consumers must always see a stable object even when the lookup
// resolves to nothing. Text mode emits prose via `handle_no_results`, but
// JSON mode emits `{ "name": "" }` (or `{ "root": "" }`) and exits 1.

#[test]
fn test_package_json_empty_name_stable_shape() {
    // A bare git repo with no packages — `repo package --json` must emit
    // `{ "name": "" }` instead of prose / no output.
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package",
            "--json",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String(String::new()));
}

#[test]
fn test_package_area_json_empty_name_stable_shape() {
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-area",
            "--json",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String(String::new()));
}

#[test]
fn test_root_json_outside_git_repo_stable_shape() {
    // Pointing `--base` at a non-git directory must still emit
    // `{ "root": "" }` so JSON consumers see a stable shape rather than
    // a Box<dyn Error> bubble.
    let dir = tempfile::tempdir().unwrap();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            dir.path().to_str().unwrap(),
            "repo",
            "root",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["root"], Value::String(String::new()));
}

// ============================================================================
// `repo {dirty,staged,unstaged}-{packages,package-areas} --json` Shape Tests
// ============================================================================
//
// Phase 3 of the `incorrect-json` feature: every package/area family
// subcommand returns `{ scope, kind, names }` instead of the full RepoInfo
// blob. Non-monorepo repos return an empty `names` array, NOT a prose
// "only intended to be used in a monorepo" error string.

fn assert_package_family_shape_when_non_monorepo(
    subcommand: &str,
    expected_scope: &str,
    expected_kind: &str,
) {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    // Modify so there's something to scan.
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            subcommand,
            "--json",
        ])
        .assert();

    // Accept either exit code: text mode exits 1 on empty rendered output,
    // but JSON mode is structurally well-formed regardless.
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected JSON for `{subcommand}`, got: {stdout:?} ({e})"));
    assert_eq!(
        json["scope"], expected_scope,
        "scope mismatch for {subcommand}"
    );
    assert_eq!(
        json["kind"], expected_kind,
        "kind mismatch for {subcommand}"
    );
    let names = json["names"]
        .as_array()
        .unwrap_or_else(|| panic!("expected `names` array for {subcommand}, got: {stdout}"));
    // Non-monorepo: empty array, not prose error.
    assert!(
        names.is_empty(),
        "non-monorepo repo should produce empty names; got {names:?}"
    );
}

#[test]
fn test_dirty_packages_json_shape() {
    assert_package_family_shape_when_non_monorepo("dirty-packages", "dirty", "packages");
}

#[test]
fn test_dirty_package_areas_json_shape() {
    assert_package_family_shape_when_non_monorepo("dirty-package-areas", "dirty", "package_areas");
}

#[test]
fn test_staged_packages_json_shape() {
    assert_package_family_shape_when_non_monorepo("staged-packages", "staged", "packages");
}

#[test]
fn test_staged_package_areas_json_shape() {
    assert_package_family_shape_when_non_monorepo(
        "staged-package-areas",
        "staged",
        "package_areas",
    );
}

#[test]
fn test_unstaged_packages_json_shape() {
    assert_package_family_shape_when_non_monorepo("unstaged-packages", "unstaged", "packages");
}

#[test]
fn test_unstaged_package_areas_json_shape() {
    assert_package_family_shape_when_non_monorepo(
        "unstaged-package-areas",
        "unstaged",
        "package_areas",
    );
}

#[test]
fn test_dirty_packages_json_does_not_emit_prose_error_for_non_monorepo() {
    // Regression: the legacy text-mode renderer returns
    // "- the \"--dirty-packages\" switch is only intended to be used in a monorepo"
    // for non-monorepos. JSON consumers must NEVER see that prose string —
    // they must see an empty `names` array.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-packages",
            "--json",
        ])
        .assert()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("only intended to be used in a monorepo"),
        "JSON output leaked the prose error string: {stdout}"
    );
    let json: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["names"], Value::Array(vec![]));
}

// ============================================================================
// Phase 5 — `deps --json` builder
// ============================================================================

/// `sniff repo deps --json` must return a `{ packages: [...] }` object,
/// not the full `RepoInfo` blob.
///
/// The created test repo is a non-monorepo so `packages` will be empty;
/// the assertion focuses on the top-level shape (object with `packages`
/// array) and the absence of `RepoInfo`-only fields like `is_monorepo`.
#[test]
fn test_repo_deps_json_shape() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "deps", "--json"])
        .assert()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected JSON, got: {stdout:?} ({e})"));

    assert!(
        json.is_object(),
        "deps --json must return an object, got: {json}"
    );
    assert!(
        json["packages"].is_array(),
        "deps --json must have `packages` array, got: {json}"
    );
    // Must NOT leak full RepoInfo blob fields.
    assert!(
        json.get("is_monorepo").is_none(),
        "deps --json must not include `is_monorepo`: {json}"
    );
}

// ============================================================================
// `repo pr` Subcommand Tests
// ============================================================================

#[test]
fn test_repo_pr_help_documents_bitbucket_draft_limitation() {
    // The --status flag's help text must call out the Bitbucket draft
    // limitation so users know `--status draft` returns nothing for
    // Bitbucket-hosted repositories.
    cargo_bin_cmd!("sniff")
        .args(["repo", "pr", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("Bitbucket"));
}

// ============================================================================
// Phase 4 — locator and boolean JSON shapes
// ============================================================================

/// `has-merge-conflict --json` on a clean repo must:
///   - exit 1 (no conflict)
///   - emit `{ "has_merge_conflict": false }` on stdout
#[test]
fn test_has_merge_conflict_json_false() {
    let (_dir, path) = create_test_repo();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "has-merge-conflict",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["has_merge_conflict"], Value::Bool(false));
}

/// `is-current-package-area-dirty --json` always emits a `{ "dirty": <bool> }`
/// object, even when invoked outside any package area (where text mode would
/// also exit 1).
///
/// Note: the underlying detection only consults `RepoStatus.dirty` /
/// `RepoStatus.untracked`, which are populated only when `--refresh-remotes`
/// (deep git mode) is in effect. This test pins the JSON contract for the
/// `false` case; the `true` case is exercised at the pure-helper layer in
/// `output::filesystem::tests::boolean_helpers`.
#[test]
fn test_is_current_package_area_dirty_json_outside_area_emits_false() {
    let (_dir, path) = create_test_repo();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "is-current-package-area-dirty",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["dirty"], Value::Bool(false));
}

/// `is-current-package-area-dirty --json` on a clean monorepo package area
/// must exit 1 and emit `{ "dirty": false }`.
#[test]
fn test_is_current_package_area_dirty_json_clean() {
    let (_dir, path) = create_cli_monorepo();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.join("pkg-a").to_str().unwrap(),
            "repo",
            "is-current-package-area-dirty",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["dirty"], Value::Bool(false));
}

/// `package-area-has-source-code-changes --json` on a clean monorepo
/// package emits `{ "has_source_code_changes": false }` and exits 1.
///
/// Note: like the dirty check, the underlying detection consults
/// `RepoStatus.dirty` / `RepoStatus.untracked`, which are populated only
/// in deep git mode (`--refresh-remotes`). The `true` case is exercised
/// at the pure-helper layer in
/// `output::filesystem::tests::boolean_helpers`.
#[test]
fn test_package_area_has_source_code_changes_json_clean() {
    let (_dir, path) = create_cli_monorepo();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.join("pkg-a").to_str().unwrap(),
            "repo",
            "package-area-has-source-code-changes",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["has_source_code_changes"], Value::Bool(false));
}

/// `package-root --json` inside a known package emits `{ "root": "<abs path>" }`.
#[test]
fn test_package_root_json_when_present() {
    let (_dir, path) = create_cli_monorepo();
    let pkg_a_lib = path.join("pkg-a/lib");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            pkg_a_lib.to_str().unwrap(),
            "repo",
            "package-root",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    let root = value["root"].as_str().expect("root must be a string");
    assert!(
        !root.is_empty(),
        "package-root must be non-empty inside a real package, got: {value}"
    );
    assert!(
        root.contains("pkg-a"),
        "package-root should resolve to the pkg-a directory, got: {root}"
    );
}

/// `package --json` inside a known package emits `{ "name": <pkg> }`.
#[test]
fn test_package_name_json() {
    let (_dir, path) = create_cli_monorepo();
    let pkg_a_lib = path.join("pkg-a/lib");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            pkg_a_lib.to_str().unwrap(),
            "repo",
            "package",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String("pkg-a".to_string()));
}

// ============================================================================
// Phase 7 — End-to-end regression: every `repo` subcommand emits a
// distinguishable JSON shape, and `--perf` keeps working alongside the
// new shapes.
// ============================================================================

/// Build a monorepo fixture whose package name differs from its area name.
///
/// Used by the Phase 7 distinctness matrix: the legacy `create_cli_monorepo`
/// helper places each package in `<area>/lib`, so `package` and `package-area`
/// resolve to the same string and produce identical `{ "name": ... }`
/// payloads. The `incorrect-json` contract is about distinct *shapes per
/// subcommand* under realistic input — pick a fixture where the values
/// differ so the matrix exercises real-world distinctness without needing
/// shape-only comparisons.
fn create_cli_monorepo_distinct_area_and_package() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["alpha/core", "alpha/cli", "beta/core"]
"#,
    )
    .unwrap();

    let members = [
        ("alpha/core", "alpha-core"),
        ("alpha/cli", "alpha-cli"),
        ("beta/core", "beta-core"),
    ];
    for (rel, name) in &members {
        let pkg = dir.path().join(rel);
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(
            pkg.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
"#
            ),
        )
        .unwrap();
        std::fs::write(pkg.join("src/lib.rs"), "pub fn entry() {}").unwrap();
    }

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial monorepo", &tree, &[])
        .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Distinctness matrix for `sniff repo <subcommand> --json`.
///
/// The `incorrect-json` feature was triggered by every subcommand returning
/// the same `RepoInfo` blob. This test pins the contract: no two subcommands
/// in the matrix produce identical stdout payloads under a realistic
/// monorepo fixture.
///
/// The matrix deliberately covers every subcommand whose JSON shape changed
/// in Phases 2-6 (`git-status`, `deps`, the package/area family, the
/// locator family, the boolean family, and the commit-family filtered
/// variants). Bare `repo` and `repo structure` are intentionally excluded —
/// they're meant to be identical (`structure` is the canonical alias).
///
/// The fixture is a monorepo where package names differ from area names
/// (`alpha-core` in area `alpha`, etc.) so `package` vs `package-area` and
/// `package-root` vs `package-area-root` resolve to different strings.
///
/// Some subcommands exit `1` when they have nothing to report; we accept
/// either exit code and only compare stdout.
#[test]
fn test_repo_subcommand_json_shapes_are_distinct() {
    let (_dir, path) = create_cli_monorepo_distinct_area_and_package();
    test_commit_file(&path, "alpha/core/src/lib.rs", "pub fn changed() {}");
    test_commit_file(&path, "README.md", "# readme");

    // Match groups intentionally avoid `structure` / bare `repo`. Each entry
    // is a tuple of (label, args after `repo`).
    let cases: &[(&str, &[&str])] = &[
        ("git-status", &["git-status"]),
        ("deps", &["deps"]),
        ("dirty-packages", &["dirty-packages"]),
        ("dirty-package-areas", &["dirty-package-areas"]),
        ("staged-packages", &["staged-packages"]),
        ("staged-package-areas", &["staged-package-areas"]),
        ("unstaged-packages", &["unstaged-packages"]),
        ("unstaged-package-areas", &["unstaged-package-areas"]),
        ("package-root", &["package-root"]),
        ("package-area-root", &["package-area-root"]),
        ("package", &["package"]),
        ("package-area", &["package-area"]),
        (
            "is-current-package-area-dirty",
            &["is-current-package-area-dirty"],
        ),
        (
            "package-area-has-source-code-changes",
            &["package-area-has-source-code-changes"],
        ),
        ("has-merge-conflict", &["has-merge-conflict"]),
        ("source-code-changes", &["source-code-changes"]),
        ("documentation-changes", &["documentation-changes"]),
    ];

    // Run from inside `alpha/core` so locator/boolean subcommands resolve
    // to a real package and area.
    let cwd = path.join("alpha/core");
    let mut payloads: Vec<(String, String)> = Vec::with_capacity(cases.len());

    for (label, sub_args) in cases {
        let mut args: Vec<&str> = vec!["--base", cwd.to_str().unwrap(), "repo"];
        args.extend_from_slice(sub_args);
        args.push("--json");
        let output = cargo_bin_cmd!("sniff")
            .args(&args)
            .assert()
            .get_output()
            .clone();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Every JSON payload must parse — that's a baseline contract even
        // for empty boolean/locator outputs.
        let _: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
            panic!(
                "stdout for `{label}` was not JSON: {e}\n--- stdout ---\n{stdout}\n--------------"
            )
        });
        payloads.push(((*label).to_string(), stdout));
    }

    // Cross-product distinctness check. We compare on raw stdout — the
    // shape, keys, and values combined are what consumers see.
    for i in 0..payloads.len() {
        for j in (i + 1)..payloads.len() {
            let (left_label, left) = &payloads[i];
            let (right_label, right) = &payloads[j];
            assert_ne!(
                left.trim(),
                right.trim(),
                "subcommands `{left_label}` and `{right_label}` returned identical JSON \
                 — every repo subcommand must emit a distinct shape:\n--- {left_label} ---\n{left}\n--- {right_label} ---\n{right}"
            );
        }
    }
}

/// `--perf --json` on a `git-status` invocation must inject a top-level
/// `performance` field into the existing object shape, leaving the rest of
/// the `GitInfo` payload intact.
///
/// Object-shaped payloads receive the perf data via `attach_performance`
/// inserting a sibling key — they are NOT wrapped in `{ data, performance }`.
#[test]
fn test_git_status_json_perf_attaches_performance_field() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--json",
            "--perf",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));

    // `GitInfo` fields stay at the top level.
    assert!(
        value.get("repo_root").is_some(),
        "git-status payload should still expose `repo_root`: {value}"
    );
    // `performance` should be a sibling key (not wrapped under `data`).
    assert!(
        value.get("performance").is_some(),
        "--perf must inject a `performance` field into object-shaped payloads: {value}"
    );
    assert!(
        value.get("data").is_none(),
        "object-shaped payloads must NOT be wrapped in `{{ data, ... }}`: {value}"
    );
}

/// `--perf --json` on a boolean subcommand still emits the boolean object
/// alongside the `performance` field, and still honours the boolean's
/// exit-code semantics (clean repo → exit 1 for `is-current-package-area-dirty`).
#[test]
fn test_is_current_package_area_dirty_json_perf_attaches_performance_field() {
    let (_dir, path) = create_cli_monorepo();
    let pkg_a = path.join("pkg-a/lib");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            pkg_a.to_str().unwrap(),
            "repo",
            "is-current-package-area-dirty",
            "--json",
            "--perf",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));

    assert_eq!(
        value["dirty"],
        Value::Bool(false),
        "boolean payload must remain intact alongside --perf: {value}"
    );
    assert!(
        value.get("performance").is_some(),
        "--perf must inject a `performance` field into boolean payloads: {value}"
    );
}

/// Phase 3 — `repo structure --json --filter` must scope the `packages`
/// array, matching text mode. Without `--filter` every workspace member is
/// listed; with `--filter pkg-a` only the matching package remains. The
/// non-`packages` `RepoInfo` fields (workspace tools, monorepo flag, root)
/// stay intact in both cases.
#[test]
fn test_repo_structure_filter_json_filters_packages() {
    let (_dir, path) = create_cli_monorepo();

    let assert_all = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "structure",
            "--json",
        ])
        .assert()
        .success();
    let stdout_all = String::from_utf8(assert_all.get_output().stdout.clone()).unwrap();
    let json_all: Value = serde_json::from_str(stdout_all.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout_all}\n---"));
    let all_packages = json_all["packages"]
        .as_array()
        .expect("packages must be array");
    assert_eq!(
        all_packages.len(),
        2,
        "unfiltered structure should list all 2 monorepo packages: {json_all}"
    );

    let assert_filtered = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "structure",
            "--json",
            "pkg-a",
        ])
        .assert()
        .success();
    let stdout_filtered = String::from_utf8(assert_filtered.get_output().stdout.clone()).unwrap();
    let json_filtered: Value = serde_json::from_str(stdout_filtered.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout_filtered}\n---"));
    let filtered_packages = json_filtered["packages"]
        .as_array()
        .expect("packages must be array");
    assert_eq!(
        filtered_packages.len(),
        1,
        "filter pkg-a should narrow to 1 package: {json_filtered}"
    );
    assert_eq!(filtered_packages[0]["name"], "pkg-a");

    // Non-packages fields must remain intact under the filter.
    assert!(
        json_filtered.get("root").is_some(),
        "filtered structure must preserve `root`: {json_filtered}"
    );
    assert_eq!(
        json_filtered["is_monorepo"],
        Value::Bool(true),
        "filtered structure must preserve `is_monorepo`: {json_filtered}"
    );
}

// ============================================================================
// Phase 4 — Targeted integration coverage for previously untested JSON paths
// ============================================================================
//
// These tests exercise the success branches of locator and boolean
// subcommands that were previously only covered by their empty/false
// branches, plus the `--package` scoping path on `git-status --json`.

/// `package-area --json` from inside a real package emits `{ "name": <area> }`
/// where the area name is distinct from the package name (the fixture
/// places `alpha-core` inside area `alpha`).
#[test]
fn test_package_area_json_resolves_to_real_area() {
    let (_dir, path) = create_cli_monorepo_distinct_area_and_package();
    let cwd = path.join("alpha/core");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            cwd.to_str().unwrap(),
            "repo",
            "package-area",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String("alpha".to_string()));
}

/// `package-area-root --json` from inside a known package area emits
/// `{ "root": <abs path containing the area name> }`.
#[test]
fn test_package_area_root_json_when_present() {
    let (_dir, path) = create_cli_monorepo_distinct_area_and_package();
    let cwd = path.join("alpha/core");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            cwd.to_str().unwrap(),
            "repo",
            "package-area-root",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    let root = value["root"].as_str().expect("root must be a string");
    assert!(
        !root.is_empty(),
        "package-area-root must be non-empty inside a real area, got: {value}"
    );
    assert!(
        root.contains("alpha"),
        "package-area-root should contain the `alpha` area segment, got: {root}"
    );
}

/// `git-status --package <name> --json` must scope `file_changes` to the
/// named package's path prefix while preserving the `GitInfo` shape (top-level
/// `repo_root` key).
#[test]
fn test_git_status_json_with_package_scope() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--package",
            "pkg-a",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));

    assert!(
        value.get("repo_root").is_some(),
        "package-scoped git-status must keep GitInfo shape (repo_root): {value}"
    );

    if let Some(file_changes) = value["file_changes"].as_array() {
        for fc in file_changes {
            let p = fc["path"].as_str().unwrap_or("");
            assert!(
                !p.starts_with("pkg-b/"),
                "pkg-a-scoped git-status must not contain pkg-b files, got: {p}"
            );
        }
    }
}

/// `is-current-package-area-dirty --json` from inside a package area whose
/// files are dirty must emit `{ "dirty": true }` and exit 0.
#[test]
fn test_is_current_package_area_dirty_json_true_branch() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a() {}");
    std::fs::write(path.join("pkg-a/lib/src/lib.rs"), "pub fn a() { dirty }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "is-current-package-area-dirty",
            "--json",
        ])
        .assert()
        .success()
        .code(0);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["dirty"],
        Value::Bool(true),
        "dirty area should emit dirty: true, got: {value}"
    );
}

/// `package-area-has-source-code-changes --json` from inside a package area
/// whose source files are dirty must emit
/// `{ "has_source_code_changes": true }` and exit 0, even in the normal
/// (non-deep) git request path where `RepoStatus.dirty` is empty.
///
/// Regression test for review-4 High finding: the helper used to read only
/// `git.status.dirty` / `git.status.untracked` and missed dirty files
/// surfaced via `git.file_changes`.
#[test]
fn test_package_area_has_source_code_changes_json_true_branch() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a() {}");
    std::fs::write(path.join("pkg-a/lib/src/lib.rs"), "pub fn a() { dirty }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "package-area-has-source-code-changes",
            "--json",
        ])
        .assert()
        .success()
        .code(0);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["has_source_code_changes"],
        Value::Bool(true),
        "dirty source file in the area should emit has_source_code_changes: true, got: {value}"
    );
}

/// `package-area-has-source-code-changes --json` must remain `false` when
/// only documentation files are dirty in the current package area, even
/// though those paths are reported via `git.file_changes` in the normal
/// CLI path.
#[test]
fn test_package_area_has_source_code_changes_json_docs_only_is_false() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/README.md", "# pkg-a");
    std::fs::write(path.join("pkg-a/lib/README.md"), "# pkg-a (dirty)").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "package-area-has-source-code-changes",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["has_source_code_changes"],
        Value::Bool(false),
        "docs-only dirty file must not flip has_source_code_changes, got: {value}"
    );
}

// ============================================================================
// Phase 4 — `repo worktree` CLI integration tests
// ============================================================================

/// Create a temp git repo with an initial commit and a linked worktree.
fn create_test_repo_with_worktree() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let (dir, repo_path) = create_test_repo();
    let repo = git2::Repository::open(&repo_path).unwrap();

    let worktree_path = repo_path.join("my-worktree");
    let _wt = repo.worktree("my-worktree", &worktree_path, None).unwrap();

    (dir, repo_path, worktree_path)
}

#[test]
fn test_repo_worktree_inside_linked_worktree_returns_name() {
    let (_dir, _repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            worktree_path.to_str().unwrap(),
            "repo",
            "worktree",
        ])
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "my-worktree");
}

#[test]
fn test_repo_worktree_inside_main_worktree_exits_1() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", repo_path.to_str().unwrap(), "repo", "worktree"])
        .assert()
        .failure()
        .code(1);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout, "");
}

#[test]
fn test_repo_worktree_no_error_exits_0() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktree",
            "--no-error",
        ])
        .assert()
        .success()
        .code(0);
}

#[test]
fn test_repo_worktree_on_error_to_stderr() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktree",
            "--on-error",
            "Not in a worktree",
            "--plain",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Not in a worktree"));
}

#[test]
fn test_repo_worktree_json_success() {
    let (_dir, _repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            worktree_path.to_str().unwrap(),
            "repo",
            "worktree",
            "--json",
        ])
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["worktree"], Value::String("my-worktree".to_string()));
}

#[test]
fn test_repo_worktree_json_failure_no_error() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktree",
            "--json",
            "--no-error",
        ])
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["worktree"], Value::Null);
}

#[test]
fn test_repo_worktree_verbose_includes_path() {
    let (_dir, _repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            worktree_path.to_str().unwrap(),
            "repo",
            "worktree",
            "-v",
        ])
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let trimmed = stdout.trim();
    assert!(trimmed.starts_with("my-worktree ["));
    assert!(trimmed.ends_with("]"));
    assert!(trimmed.contains(worktree_path.to_str().unwrap()));
}

#[test]
fn test_repo_worktree_help_mentions_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("worktree"));
}

// ============================================================================
// Phase 3 — `repo worktrees` CLI integration tests
// ============================================================================

#[test]
fn test_repo_worktrees_default_output() {
    let (_dir, repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", repo_path.to_str().unwrap(), "repo", "worktrees"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "expected main + 1 linked worktree: {stdout}"
    );
    assert!(
        lines.iter().any(|l| l.contains("my-worktree")),
        "should list linked worktree: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_md_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--md",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for line in stdout.trim().lines() {
        assert!(
            line.starts_with("- "),
            "md output should start with '- ': {line}"
        );
    }
}

#[test]
fn test_repo_worktrees_list_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--list",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    for line in &lines {
        assert!(
            !line.starts_with("- "),
            "list output should not use markdown bullets: {line}"
        );
    }
    assert!(
        lines.iter().any(|l| l.contains("my-worktree")),
        "list should contain worktree name: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_csv_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--csv",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let trimmed = stdout.trim();
    assert!(
        trimmed.contains("my-worktree"),
        "csv should contain worktree name: {stdout}"
    );
    assert!(
        !trimmed.contains('\n'),
        "csv should be single line: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_verbose_output() {
    let (_dir, repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "-v",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("my-worktree"),
        "verbose should list worktree name: {stdout}"
    );
    assert!(
        stdout.contains("located at"),
        "verbose should include path: {stdout}"
    );
    assert!(
        stdout.contains(worktree_path.to_str().unwrap()),
        "verbose should contain worktree path: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_json_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    let arr = value["worktrees"]
        .as_array()
        .expect("worktrees must be array");
    assert_eq!(arr.len(), 2, "expected main + 1 linked worktree");
    assert!(
        arr.iter().any(|w| w["name"] == "my-worktree"),
        "should include linked worktree: {value}"
    );
}

#[test]
fn test_repo_worktrees_plain_verbose_no_escape_codes() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "-v",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        !stdout.contains('\x1b'),
        "plain output must not contain escape codes: {stdout:?}"
    );
    assert!(
        stdout.contains("located at"),
        "plain verbose should still show words: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_current_marker_from_main_worktree() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .current_dir(&repo_path)
        .args(["repo", "worktrees"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    let current_line = lines
        .iter()
        .find(|l| l.starts_with("* "))
        .expect("should have a current marker line");
    assert!(
        current_line.contains(repo_path.file_name().unwrap().to_str().unwrap()),
        "current marker should be on main worktree: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_current_marker_from_linked_worktree() {
    let (_dir, _repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = cargo_bin_cmd!("sniff")
        .current_dir(&worktree_path)
        .args(["repo", "worktrees"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    let current_line = lines
        .iter()
        .find(|l| l.starts_with("* "))
        .expect("should have a current marker line");
    assert!(
        current_line.contains("my-worktree"),
        "current marker should be on linked worktree: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_detached_head() {
    let (dir, repo_path) = create_test_repo();
    let repo = git2::Repository::open(&repo_path).unwrap();

    let worktree_path = repo_path.join("detached-wt");
    let _wt = repo.worktree("detached-wt", &worktree_path, None).unwrap();

    // Detach HEAD in the linked worktree.
    let wt_repo = git2::Repository::open(&worktree_path).unwrap();
    let head_commit = wt_repo.head().unwrap().peel_to_commit().unwrap();
    wt_repo.set_head_detached(head_commit.id()).unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "-v",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("detached HEAD"),
        "verbose should show detached HEAD fallback: {stdout}"
    );
}

// ============================================================================
// Phase 5 — `--package` / `--package-area` flag matrix
//
// These tests pin the consistency contract for the new flag pair across the
// `repo` subcommand surface:
//
// 1. `--package` returns exactly one package.
// 2. `--package-area homelab` matches both `homelab` and `homelab/server`
//    (case-insensitive prefix semantics).
// 3. `--package` AND `--package-area` overlapping → success.
// 4. `--package` AND `--package-area` non-overlapping → hard error citing
//    the package's real area.
// 5. Unknown `--package` → error names valid package list.
// 6. Unknown `--package-area` → error names valid area list.
// 7. Positional `filter` plus `--package` → AND of both.
// 8. `-p` short flag works on `FileListArgs`-based commands.
// 9. `git-status -p <area-name>` no longer falls back to area matching —
//    must hard-error.
// ============================================================================

/// Build a monorepo with three areas where two share a common prefix
/// (`homelab` and `homelab/server`) and one is wholly distinct (`sniff`).
///
/// Layout:
///
/// - `homelab/lib`        → area `homelab`,        name `homelab-lib`
/// - `homelab/server/srv` → area `homelab/server`, name `homelab-srv`
/// - `sniff/cli`          → area `sniff`,          name `sniff-cli`
fn create_cli_monorepo_with_nested_areas() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["homelab/lib", "homelab/server/srv", "sniff/cli"]
"#,
    )
    .unwrap();

    let members = [
        ("homelab/lib", "homelab-lib"),
        ("homelab/server/srv", "homelab-srv"),
        ("sniff/cli", "sniff-cli"),
    ];
    for (rel, name) in &members {
        let pkg = dir.path().join(rel);
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(
            pkg.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
"#
            ),
        )
        .unwrap();
        std::fs::write(pkg.join("src/lib.rs"), "pub fn entry() {}").unwrap();
    }

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "initial nested-area monorepo",
        &tree,
        &[],
    )
    .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Spec §6.1 — `--package` returns exactly one package.
#[test]
fn test_repo_package_flag_returns_single_package() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package",
            "sniff-cli",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "sniff-cli");
}

/// Spec §6.2 — `--package-area homelab` matches both `homelab` and
/// `homelab/server` packages via prefix semantics.
#[test]
fn test_repo_package_area_flag_uses_prefix_semantics() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package-area",
            "homelab",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let mut names: Vec<&str> = stdout.trim().lines().collect();
    names.sort();
    assert_eq!(names, vec!["homelab-lib", "homelab-srv"]);
}

/// Spec §6.3 — `--package` AND `--package-area` overlap → intersection,
/// returns the package itself.
#[test]
fn test_repo_package_and_area_flags_overlap_succeeds() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package",
            "homelab-srv",
            "--package-area",
            "homelab",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "homelab-srv");
}

/// Spec §6.4 — `--package` AND `--package-area` non-overlapping → hard error
/// naming the package's real area and the requested area.
#[test]
fn test_repo_package_and_area_flags_non_overlap_errors() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package",
            "sniff-cli",
            "--package-area",
            "homelab",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("sniff-cli"),
        "intersection error must name the package, got:\n{stderr}"
    );
    assert!(
        stderr.contains("sniff"),
        "intersection error must name the package's real area, got:\n{stderr}"
    );
    assert!(
        stderr.contains("homelab"),
        "intersection error must name the requested area, got:\n{stderr}"
    );
}

/// Spec §6.5 — Unknown `--package` → error lists valid package names.
#[test]
fn test_repo_unknown_package_errors_with_valid_list() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package",
            "no-such-pkg",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("no-such-pkg"),
        "error must name the unknown package, got:\n{stderr}"
    );
    assert!(
        stderr.contains("Valid package names"),
        "error must mention valid package names, got:\n{stderr}"
    );
    assert!(
        stderr.contains("sniff-cli") && stderr.contains("homelab-lib"),
        "error must list the actual valid package names, got:\n{stderr}"
    );
}

/// Spec §6.6 — Unknown `--package-area` → error lists valid package areas.
#[test]
fn test_repo_unknown_package_area_errors_with_valid_list() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package-area",
            "no-such-area",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("no-such-area"),
        "error must name the unknown area, got:\n{stderr}"
    );
    assert!(
        stderr.contains("Valid package areas"),
        "error must mention valid package areas, got:\n{stderr}"
    );
    assert!(
        stderr.contains("sniff") && stderr.contains("homelab"),
        "error must list the actual valid areas, got:\n{stderr}"
    );
}

/// Spec §6.7 — Positional `filter` plus `--package` are AND-combined.
///
/// `homelab-lib` and `homelab-srv` are both in areas starting with `homelab`,
/// so the positional `@homelab` filter selects both. The `--package` flag
/// then narrows to the single named package.
#[test]
fn test_repo_positional_filter_and_package_flag_combine() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "@homelab",
            "--package",
            "homelab-lib",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "homelab-lib");
}

/// Spec §6.8 — `-p` short flag works on a `FileListArgs`-based command.
///
/// Stage and modify a file under `sniff/cli` so `dirty-files` has something
/// to report, then verify `-p sniff-cli` scopes the result to that package.
#[test]
fn test_repo_dirty_files_short_p_flag_scopes_to_package() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    // Mark the sniff-cli source file dirty (untracked modification of an
    // already-tracked file).
    std::fs::write(
        path.join("sniff/cli/src/lib.rs"),
        "pub fn entry() { let _x = 1; }",
    )
    .unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-files",
            "-p",
            "sniff-cli",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("sniff/cli"),
        "scoped dirty-files must include the sniff/cli path, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("homelab/"),
        "scoped dirty-files must not include unrelated areas, got:\n{stdout}"
    );
}

/// Spec §6.9 — Regression guard: `git-status -p <area-name>` (a real area
/// that is **not** a package name) must hard-error rather than fall back to
/// area matching.
#[test]
fn test_repo_git_status_package_with_area_name_errors() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "-p",
            "homelab",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("homelab"),
        "error must name the rejected input, got:\n{stderr}"
    );
    assert!(
        stderr.contains("Valid package names"),
        "error must list valid package names (not areas), got:\n{stderr}"
    );
}

// ============================================================================
// `repo area` Subcommand
// ============================================================================
//
// `sniff repo area` returns a single "area" name combining the notions of
// "package" and "package-area": package name when inside a package, else the
// surrounding area string (or "root").

#[test]
fn test_repo_area_inside_package_returns_package_name() {
    let (_dir, path) = create_cli_monorepo();
    let inside_pkg_a = path.join("pkg-a/lib/src");
    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", inside_pkg_a.to_str().unwrap(), "repo", "area"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a");
}

#[test]
fn test_repo_area_at_area_dir_returns_area_name() {
    let (_dir, path) = create_cli_monorepo();
    let area_dir = path.join("pkg-a");
    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", area_dir.to_str().unwrap(), "repo", "area"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a");
}

#[test]
fn test_repo_area_at_repo_root_returns_root() {
    let (_dir, path) = create_cli_monorepo();
    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "area"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "root");
}

#[test]
fn test_repo_area_json_emits_name_outcome() {
    let (_dir, path) = create_cli_monorepo();
    let inside_pkg_b = path.join("pkg-b/lib");
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            inside_pkg_b.to_str().unwrap(),
            "--json",
            "repo",
            "area",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String("pkg-b".to_string()));
}

#[test]
fn test_repo_area_non_monorepo_repo_silent_failure() {
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "area"])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stdout.is_empty(), "stdout must be empty, got: {stdout:?}");
    assert!(
        stderr.is_empty(),
        "stderr must be empty without --verbose, got: {stderr:?}"
    );
}

#[test]
fn test_repo_area_non_monorepo_verbose_message_on_stderr() {
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "--verbose",
            "--plain",
            "repo",
            "area",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stdout.is_empty(), "stdout must be empty, got: {stdout:?}");
    assert!(
        stderr.contains("you are in a repo but not a monorepo"),
        "verbose stderr must explain not-a-monorepo, got: {stderr:?}"
    );
}

#[test]
fn test_repo_area_not_in_repo_verbose_message_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            dir.path().to_str().unwrap(),
            "--verbose",
            "--plain",
            "repo",
            "area",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stdout.is_empty(), "stdout must be empty, got: {stdout:?}");
    assert!(
        stderr.contains("you are not in a repo"),
        "verbose stderr must explain not-in-repo, got: {stderr:?}"
    );
}

#[test]
fn test_repo_area_no_error_zero_exit_when_no_monorepo() {
    let (_dir, path) = create_test_repo();
    cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "area",
            "--no-error",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_area_on_error_prints_message_to_stdout() {
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.to_str().unwrap(),
            "--plain",
            "repo",
            "area",
            "--no-error",
            "--on-error",
            "n/a",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("n/a"),
        "--on-error message must reach stdout, got: {stdout:?}"
    );
}
