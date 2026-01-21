use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

#[allow(deprecated)] // We need Command struct to set current_dir
fn hug_cmd() -> Command {
    let mut cmd = Command::cargo_bin("hug").unwrap();
    // Set working directory to repo root (2 levels up from cli/tests/)
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    cmd.current_dir(repo_root);
    cmd
}

#[test]
fn test_help_flag() {
    hug_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Tree Hugger diagnostics"));
}

#[test]
fn test_version_flag() {
    hug_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("hug"));
}

#[test]
fn test_subcommand_help() {
    hug_cmd()
        .args(["symbols", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List all symbols"));
}

// ============================================================================
// Regression tests for --json flag support
// Bug: The CLI was missing --json flag support for the symbols command
// ============================================================================

#[test]
fn test_json_flag_exists_in_help() {
    // Regression test: --json flag should appear in help output
    hug_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--json"));
}

#[test]
fn test_json_flag_in_subcommand_help() {
    // Regression test: --json flag should appear in subcommand help (global flag)
    hug_cmd()
        .args(["symbols", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--json"));
}

#[test]
fn test_no_format_flag() {
    // Regression test: --format flag should NOT exist (replaced by --json)
    hug_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--format").not());
}

#[test]
fn test_symbols_json_output() {
    // Regression test: symbols command with --json should produce JSON output
    hug_cmd()
        .args(["symbols", "tree-hugger/cli/src/main.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"root_dir\""))
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn test_symbols_pretty_output_default() {
    // Regression test: symbols command without --json should produce pretty output
    // Note: Pretty output now includes type composition like `{ field: Type }`
    // so we check for the language marker and that it's NOT JSON format
    hug_cmd()
        .args(["symbols", "tree-hugger/cli/src/main.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(Rust)"))
        // Check it's not JSON - JSON would have "root_dir" field
        .stdout(predicate::str::contains("\"root_dir\"").not());
}

// ============================================================================
// Regression tests for flag ordering flexibility
// Bug: Flags had to be placed in specific positions relative to subcommand
// ============================================================================

#[test]
fn test_json_flag_before_subcommand() {
    // Regression test: --json should work before the subcommand
    hug_cmd()
        .args(["--json", "symbols", "tree-hugger/cli/src/main.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"root_dir\""));
}

#[test]
fn test_json_flag_after_subcommand() {
    // Regression test: --json should work after the subcommand
    hug_cmd()
        .args(["symbols", "--json", "tree-hugger/cli/src/main.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"root_dir\""));
}

#[test]
fn test_json_flag_at_end() {
    // Regression test: --json should work at the very end
    hug_cmd()
        .args(["symbols", "tree-hugger/cli/src/main.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"root_dir\""));
}

#[test]
fn test_language_flag_ordering() {
    // Regression test: --language flag should work in any position
    hug_cmd()
        .args(["--language", "rust", "symbols", "tree-hugger/cli/src/main.rs"])
        .assert()
        .success();

    hug_cmd()
        .args(["symbols", "--language", "rust", "tree-hugger/cli/src/main.rs"])
        .assert()
        .success();

    hug_cmd()
        .args(["symbols", "tree-hugger/cli/src/main.rs", "--language", "rust"])
        .assert()
        .success();
}

#[test]
fn test_multiple_flags_different_positions() {
    // Regression test: Multiple flags in various positions should all work
    hug_cmd()
        .args([
            "--json",
            "symbols",
            "--language",
            "rust",
            "tree-hugger/cli/src/main.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"root_dir\""))
        .stdout(predicate::str::contains("\"language\": \"Rust\""));
}

// ============================================================================
// Tests for all subcommands with --json
// ============================================================================

#[test]
fn test_functions_json_output() {
    hug_cmd()
        .args(["functions", "tree-hugger/cli/src/main.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn test_types_json_output() {
    hug_cmd()
        .args(["types", "tree-hugger/cli/src/main.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn test_exports_json_output() {
    hug_cmd()
        .args(["exports", "tree-hugger/cli/src/main.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn test_imports_json_output() {
    hug_cmd()
        .args(["imports", "tree-hugger/cli/src/main.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""));
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_multiple_files_json() {
    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/cli/src/main.rs",
            "tree-hugger/lib/src/lib.rs",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn test_glob_pattern_json() {
    hug_cmd()
        .args(["symbols", "tree-hugger/cli/src/*.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""));
}

// ============================================================================
// Phase 7: CLI Output Enhancement tests
// ============================================================================

#[test]
fn test_plain_flag_exists_in_help() {
    // Regression test: --plain flag should appear in help output
    hug_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--plain"));
}

#[test]
fn test_plain_flag_suppresses_ansi() {
    // Test that --plain output contains no ANSI escape codes
    let output = hug_cmd()
        .args(["symbols", "tree-hugger/lib/tests/fixtures/sample.rs", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // ANSI escape codes start with ESC (0x1B)
    assert!(
        !stdout.contains('\x1b'),
        "Output contains ANSI escape codes: {}",
        stdout
    );
}

#[test]
fn test_json_output_has_no_escape_codes() {
    // Test that --json output contains no ANSI escape codes
    let output = hug_cmd()
        .args(["symbols", "tree-hugger/lib/tests/fixtures/sample.rs", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // ANSI escape codes start with ESC (0x1B)
    assert!(
        !stdout.contains('\x1b'),
        "JSON output contains ANSI escape codes: {}",
        stdout
    );
}

#[test]
fn test_json_contains_doc_comment() {
    // Test that JSON output includes doc_comment field for documented symbols
    hug_cmd()
        .args(["functions", "tree-hugger/lib/tests/fixtures/sample.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"doc_comment\""))
        .stdout(predicate::str::contains("Greets a person by name"));
}

#[test]
fn test_json_contains_signature() {
    // Test that JSON output includes signature field for functions
    hug_cmd()
        .args(["functions", "tree-hugger/lib/tests/fixtures/sample.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"signature\""))
        .stdout(predicate::str::contains("\"parameters\""));
}

#[test]
fn test_json_signature_has_parameter_names() {
    // Test that function signatures include parameter names
    hug_cmd()
        .args(["functions", "tree-hugger/lib/tests/fixtures/sample.rs", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"name\""));
}

#[test]
fn test_plain_flag_with_json_flag() {
    // Test that --json takes precedence over --plain (produces JSON)
    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--plain",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"root_dir\""));
}
