use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

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
        .stdout(predicate::str::contains("SUBCOMMANDS"))
        .stdout(predicate::str::contains("sniff os"))
        .stdout(predicate::str::contains("sniff cpu"))
        .stdout(predicate::str::contains("sniff hardware"));
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
        .stdout(predicate::str::contains("complete --keep-order --exclusive --command sniff"));
}

#[test]
fn test_help_mentions_completions() {
    cargo_bin_cmd!("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--completions"))
        .stdout(predicate::str::contains("SHELL COMPLETIONS"));
}

// ============================================================================
// Output Mode Tests
// No subcommand = JSON output (all data)
// With subcommand = text output by default, --json for JSON
// ============================================================================

#[test]
fn test_no_subcommand_outputs_json() {
    // Without a subcommand, the output should be JSON
    cargo_bin_cmd!("sniff")
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
        .stdout(predicate::str::contains("=== OS ==="));
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
fn test_base_flag_before_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["-b", ".", "filesystem"])
        .assert()
        .success();
}

#[test]
fn test_base_flag_after_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["filesystem", "-b", "."])
        .assert()
        .success();
}

#[test]
fn test_deep_flag_before_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["--deep", "git"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Git ==="));
}

#[test]
fn test_deep_flag_after_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["git", "--deep"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Git ==="));
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
        .stdout(predicate::str::contains("=== OS ==="))
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
        .stdout(predicate::str::contains("=== Network ==="));
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
    assert!(json.is_array(), "GPU output should be an array at top level");
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
    assert!(storage.len() > 0, "storage should have at least one disk");
}

// ============================================================================
// Filesystem Detail Subcommand Tests
// git, repo, language
// ============================================================================

#[test]
fn test_git_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("git")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Git ==="));
}

#[test]
fn test_git_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["git", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have git fields at top level (flattened)
    assert!(
        json.get("repo_root").is_some() || json.get("current_branch").is_some(),
        "git data should exist at top level"
    );

    // Should NOT have wrappers
    assert!(json.get("git").is_none(), "git wrapper should not exist");
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_repo_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("repo")
        .assert()
        .success();
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
        .arg("language")
        .assert()
        .success();
}

#[test]
fn test_language_subcommand_json_output() {
    let output = cargo_bin_cmd!("sniff")
        .args(["language", "--json"])
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
// Programs Subcommand Tests
// programs, editors, utilities, language-package-managers, os-package-managers,
// tts-clients, terminal-apps, audio
// ============================================================================

#[test]
fn test_programs_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("programs")
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
}

#[test]
fn test_programs_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["programs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("editors"))
        .stdout(predicate::str::contains("utilities"));
}

#[test]
fn test_programs_subcommand_markdown_flag() {
    cargo_bin_cmd!("sniff")
        .args(["programs", "--markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
}

#[test]
fn test_programs_markdown_conflicts_with_json() {
    cargo_bin_cmd!("sniff")
        .args(["programs", "--markdown", "--json"])
        .assert()
        .failure();
}

#[test]
fn test_editors_subcommand_text_output() {
    cargo_bin_cmd!("sniff")
        .arg("editors")
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
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
    cargo_bin_cmd!("sniff")
        .arg("utilities")
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
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
    cargo_bin_cmd!("sniff")
        .arg("language-package-managers")
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
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
    cargo_bin_cmd!("sniff")
        .arg("tts-clients")
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
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
    cargo_bin_cmd!("sniff")
        .arg("terminal-apps")
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
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
    cargo_bin_cmd!("sniff")
        .arg("audio")
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
}

#[test]
fn test_audio_subcommand_json_output() {
    cargo_bin_cmd!("sniff")
        .args(["audio", "--json"])
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
// Deep Flag Tests
// ============================================================================

#[test]
fn test_deep_flag_in_help() {
    cargo_bin_cmd!("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--deep"))
        .stdout(predicate::str::contains("remote").or(predicate::str::contains("git")));
}

#[test]
fn test_deep_flag_with_filesystem_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["--deep", "filesystem"])
        .assert()
        .success();
}

#[test]
fn test_deep_flag_with_git_subcommand_json() {
    cargo_bin_cmd!("sniff")
        .args(["--deep", "git", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("recent"));
}

#[test]
fn test_git_json_contains_new_fields() {
    // Verify JSON output contains the new git-related fields
    cargo_bin_cmd!("sniff")
        .args(["git", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("recent"))
        .stdout(predicate::str::contains("in_worktree"))
        .stdout(predicate::str::contains("worktrees"));
}

#[test]
fn test_deep_and_verbose_combined() {
    cargo_bin_cmd!("sniff")
        .args(["--deep", "git", "-vv"])
        .assert()
        .success();
}

// ============================================================================
// Verbose Flag Tests with Subcommands
// ============================================================================

#[test]
fn test_verbose_with_programs_markdown_adds_columns() {
    cargo_bin_cmd!("sniff")
        .args(["programs", "--markdown", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Binary"))
        .stdout(predicate::str::contains("Path"));
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
    cargo_bin_cmd!("sniff")
        .arg("--hardware")
        .assert()
        .failure();
}
