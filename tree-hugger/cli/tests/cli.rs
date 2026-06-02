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

#[test]
fn test_filter_help_describes_path_wildcard_and_exact_matching() {
    hug_cmd()
        .args(["functions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("File path or symbol-name filters"))
        .stdout(predicate::str::contains("File-like filters (paths such as"))
        .stdout(predicate::str::contains(
            "Remaining filters are symbol-name filters",
        ))
        .stdout(predicate::str::contains("fuzzy"))
        .stdout(predicate::str::contains("contains match"))
        .stdout(predicate::str::contains("wildcard match"))
        .stdout(predicate::str::contains("exact symbol name match"));
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
    // Default symbol output is now flattened (not file-grouped) and non-JSON.
    hug_cmd()
        .args(["symbols", "tree-hugger/cli/src/main.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tree-hugger/cli/src/main.rs:"))
        // Check it's not JSON - JSON would have "root_dir" field
        .stdout(predicate::str::contains("\"files\"").not());
}

#[test]
fn test_symbols_include_container_context_for_methods() {
    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "method greet(self, name: &str) -> String [in impl Greeter]",
        ));
}

#[test]
fn test_rust_structs_render_as_struct_not_type() {
    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- struct Greeter"))
        .stdout(predicate::str::contains("- type Greeter").not());
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
        .args([
            "--language",
            "rust",
            "symbols",
            "tree-hugger/cli/src/main.rs",
        ])
        .assert()
        .success();

    hug_cmd()
        .args([
            "symbols",
            "--language",
            "rust",
            "tree-hugger/cli/src/main.rs",
        ])
        .assert()
        .success();

    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/cli/src/main.rs",
            "--language",
            "rust",
        ])
        .assert()
        .success();
}

#[test]
fn test_language_help_uses_expected_value_names() {
    hug_cmd()
        .args(["functions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("javascript"))
        .stdout(predicate::str::contains("typescript"))
        .stdout(predicate::str::contains("c++"))
        .stdout(predicate::str::contains("c#"))
        .stdout(predicate::str::contains("java-script").not())
        .stdout(predicate::str::contains("type-script").not());
}

#[test]
fn test_language_aliases_parse_successfully() {
    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.js",
            "--language",
            "js",
            "--plain",
        ])
        .assert()
        .success();

    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.ts",
            "--language",
            "ts",
            "--plain",
        ])
        .assert()
        .success();

    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.cpp",
            "--language",
            "c++",
            "--plain",
        ])
        .assert()
        .success();

    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.cs",
            "--language",
            "c#",
            "--plain",
        ])
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
    // `exports` subcommand was removed in the CLI refactor.
    hug_cmd()
        .args(["exports", "tree-hugger/cli/src/main.rs", "--json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
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

#[test]
fn test_symbols_without_filters_scans_all_sources() {
    hug_cmd()
        .args(["symbols", "tree-hugger/cli/src/*.rs", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tree-hugger/cli/src/main.rs:"));
}

#[test]
fn test_symbol_name_filters_auto_wrap_and_use_or_semantics() {
    hug_cmd()
        .args([
            "functions",
            "greet",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet"))
        .stdout(predicate::str::contains("greet_many"));
}

#[test]
fn test_symbol_name_filters_preserve_explicit_wildcard() {
    hug_cmd()
        .args([
            "functions",
            "*many",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet_many"))
        .stdout(predicate::str::contains("greet(").not());
}

#[test]
fn test_symbol_name_filters_trailing_bang_means_exact_match() {
    hug_cmd()
        .args([
            "functions",
            "greet!",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet(name: &str)"))
        .stdout(predicate::str::contains("greet_many").not());
}

#[test]
fn test_exclude_symbols_glob_filters_output() {
    hug_cmd()
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--exclude-symbols",
            "*many",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet(name: &str)"))
        .stdout(predicate::str::contains("greet_many").not());
}

#[test]
fn test_exclude_symbols_trailing_bang_means_exact_match() {
    hug_cmd()
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--exclude-symbols",
            "greet!",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet(name: &str)").not())
        .stdout(predicate::str::contains("greet_many"));
}

#[test]
fn test_exclude_files_glob_filters_scanned_files() {
    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/cli/src/main.rs",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--exclude-files",
            "tree-hugger/cli/**",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "tree-hugger/lib/tests/fixtures/sample.rs",
        ))
        .stdout(predicate::str::contains("tree-hugger/cli/src/main.rs").not());
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
fn test_comments_flag_exists_in_help() {
    hug_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--comments"));
}

#[test]
fn test_exclude_flags_exist_in_help() {
    hug_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--exclude-files"))
        .stdout(predicate::str::contains("--exclude-symbols"))
        .stdout(predicate::str::contains("--ignore").not());
}

#[test]
fn test_plain_flag_suppresses_ansi() {
    // Test that --plain output contains no ANSI escape codes
    let output = hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--plain",
        ])
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
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--json",
        ])
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
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"doc_comment\""))
        .stdout(predicate::str::contains("Greets a person by name"));
}

#[test]
fn test_json_contains_signature() {
    // Test that JSON output includes signature field for functions
    hug_cmd()
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"signature\""))
        .stdout(predicate::str::contains("\"parameters\""));
}

#[test]
fn test_json_signature_has_parameter_names() {
    // Test that function signatures include parameter names
    hug_cmd()
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--json",
        ])
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

#[test]
fn test_comments_flag_renders_symbol_doc_comments() {
    hug_cmd()
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--plain",
            "--comments",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Greets a person by name."));
}

// ============================================================================
// Classes command tests
// ============================================================================

#[test]
fn test_classes_command_help() {
    hug_cmd()
        .args(["classes", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List classes and their members"));
}

#[test]
fn test_classes_command_json_output() {
    // Test that classes command with --json produces JSON output with class structure
    hug_cmd()
        .args([
            "classes",
            "tree-hugger/lib/tests/fixtures/signatures.java",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"class\""))
        .stdout(predicate::str::contains("\"static_methods\""))
        .stdout(predicate::str::contains("\"instance_methods\""));
}

#[test]
fn test_classes_command_pretty_output() {
    // Test that classes command without --json produces pretty output
    hug_cmd()
        .args([
            "classes",
            "tree-hugger/lib/tests/fixtures/signatures.java",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("(Java)"))
        .stdout(predicate::str::contains("Signatures"));
}

#[test]
fn test_classes_command_name_filter() {
    // Test --name filter
    hug_cmd()
        .args([
            "classes",
            "tree-hugger/lib/tests/fixtures/signatures.java",
            "--name",
            "Signatures",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Signatures\""));
}

#[test]
fn test_classes_command_static_only_filter() {
    // Test --static-only filter
    hug_cmd()
        .args([
            "classes",
            "tree-hugger/lib/tests/fixtures/signatures.java",
            "--static-only",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"static_methods\""))
        // Should have empty instance_methods array
        .stdout(predicate::str::contains("\"instance_methods\": []"));
}

#[test]
fn test_classes_command_instance_only_filter() {
    // Test --instance-only filter
    hug_cmd()
        .args([
            "classes",
            "tree-hugger/lib/tests/fixtures/signatures.java",
            "--instance-only",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"instance_methods\""))
        // Should have empty static_methods array
        .stdout(predicate::str::contains("\"static_methods\": []"));
}

#[test]
fn test_classes_command_csharp() {
    // Test classes command with C# file
    hug_cmd()
        .args([
            "classes",
            "tree-hugger/lib/tests/fixtures/signatures.cs",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"class\""))
        .stdout(predicate::str::contains("\"Greeter\""));
}

#[test]
fn test_classes_command_typescript_uses_real_class_symbols() {
    hug_cmd()
        .args([
            "classes",
            "tree-hugger/lib/tests/fixtures/sample.ts",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Greeter\""))
        .stdout(predicate::str::contains("\"GreetFn\"").not());
}

// ============================================================================
// Imports command grouping tests
// ============================================================================

#[test]
fn test_imports_typescript_grouped_output() {
    hug_cmd()
        .args([
            "imports",
            "tree-hugger/lib/tests/fixtures/imports.ts",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "tree-hugger/lib/tests/fixtures/imports.ts (TypeScript)",
        ))
        .stdout(predicate::str::contains(
            "- import { readFile as read, writeFile as write } from \"fs/promises\" [7:22, 41]",
        ));
}

#[test]
fn test_imports_rust_grouped_output() {
    hug_cmd()
        .args([
            "imports",
            "tree-hugger/lib/tests/fixtures/imports.rs",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "tree-hugger/lib/tests/fixtures/imports.rs (Rust)",
        ))
        .stdout(predicate::str::contains(
            "- use std::process::{Child, Command, Stdio}",
        ));
}

// ============================================================================
// --exported and --prelude filter tests
// ============================================================================

#[test]
fn test_exported_flag_filters_functions() {
    // sample.rs has pub fn greet and pub fn greet_many — both should appear
    hug_cmd()
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--exported",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet"))
        .stdout(predicate::str::contains("greet_many"));
}

#[test]
fn test_exported_flag_filters_symbols() {
    // sample.rs has pub struct Greeter — should appear under --exported
    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--exported",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Greeter"))
        .stdout(predicate::str::contains("greet"));
}

#[test]
fn test_exported_flag_excludes_private_rust_constants() {
    hug_cmd()
        .args([
            "symbols",
            "tree-hugger/lib/src/builtins.rs",
            "--exported",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ZSH_BUILTINS").not());
}

#[test]
fn test_exported_flag_json_output() {
    hug_cmd()
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--exported",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""))
        .stdout(predicate::str::contains("greet"));
}

#[test]
fn test_exported_and_prelude_conflict() {
    // --exported and --prelude are mutually exclusive
    hug_cmd()
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--exported",
            "--prelude",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_prelude_env_var_filters_symbols() {
    // PRELUDE env var should limit results to named symbols
    hug_cmd()
        .env("PRELUDE", "greet")
        .args([
            "functions",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--prelude",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet"))
        // greet_many should NOT appear (name is "greet_many", not "greet")
        .stdout(predicate::str::contains("greet_many").not());
}

#[test]
fn test_prelude_env_var_multiple_names() {
    // Multiple comma-separated names in PRELUDE
    hug_cmd()
        .env("PRELUDE", "greet, Greeter")
        .args([
            "symbols",
            "tree-hugger/lib/tests/fixtures/sample.rs",
            "--prelude",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet"))
        .stdout(predicate::str::contains("Greeter"))
        .stdout(predicate::str::contains("greet_many").not());
}

#[test]
fn test_prelude_flag_with_real_prelude_file() {
    // biscuit-terminal/lib has a real prelude.rs
    // Run from biscuit-terminal/lib to pick up its prelude
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let bt_lib = repo_root.join("biscuit-terminal/lib");

    let mut cmd = hug_cmd();
    cmd.current_dir(&bt_lib)
        .args(["symbols", "src/terminal.rs", "--prelude", "--plain"])
        .assert()
        .success()
        // Terminal is re-exported in prelude.rs
        .stdout(predicate::str::contains("Terminal"));
}

#[test]
fn test_symbols_prelude_reports_direct_prelude_exports() {
    let fixture_pkg = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/prelude_pkg");

    let mut cmd = hug_cmd();
    cmd.current_dir(&fixture_pkg)
        .args(["symbols", "--prelude", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("VerboseType"))
        .stdout(predicate::str::contains("ElevatedEnum"))
        // Private `use` should not be treated as a prelude export.
        .stdout(predicate::str::contains("VerboseEnum").not())
        // Prelude export view should not expand underlying type fields.
        .stdout(predicate::str::contains("field_a").not())
        // Output should point to the symbol definition file, not prelude.rs.
        .stdout(predicate::str::contains("src/alpha.rs"))
        .stdout(predicate::str::contains("src/prelude.rs").not());
}

#[test]
fn test_symbols_prelude_comments_show_resolved_doc_comments() {
    let fixture_pkg = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/prelude_pkg");

    let mut cmd = hug_cmd();
    cmd.current_dir(&fixture_pkg)
        .args(["symbols", "--prelude", "--plain", "--comments"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Type exported through the fixture prelude for comment rendering tests.",
        ))
        .stdout(predicate::str::contains(
            "Enum exported through the fixture prelude with aliasing.",
        ));
}

#[test]
fn test_functions_prelude_from_package_area_root_discovers_child_package_prelude() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let package_area_root = repo_root.join("biscuit-terminal");

    let mut cmd = hug_cmd();
    cmd.current_dir(&package_area_root)
        .args(["functions", "--prelude", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\n\nbiscuit-terminal/cli\n\n").not())
        .stdout(predicate::str::contains("\n\nbiscuit-terminal/lib\n\n").not())
        .stdout(predicate::str::contains("parse_width_spec"))
        .stdout(predicate::str::contains("(no symbols)").not());
}

#[test]
fn test_exported_flag_in_help() {
    hug_cmd()
        .args(["functions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--exported"))
        .stdout(predicate::str::contains("--prelude"));
}

#[test]
fn test_exported_flag_classes() {
    // Classes command should also support --exported
    hug_cmd()
        .args(["classes", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--exported"))
        .stdout(predicate::str::contains("--prelude"));
}

#[test]
fn test_language_override_parses_extensionless_file() {
    // A forced language should parse an explicitly named file even when its
    // extension does not map to that language (here, no extension at all).
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("myscript");
    std::fs::write(&path, "pub fn collect_widgets() {}\n").unwrap();

    hug_cmd()
        .args([
            "symbols",
            path.to_str().unwrap(),
            "--language",
            "rust",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("collect_widgets"));
}

#[test]
fn test_language_override_parses_bare_extensionless_file_from_cwd() {
    // Regression: a bare, slashless, extensionless token run from the file's own
    // directory must still resolve as an explicit file under `--language`.
    // Previously it was misclassified as a symbol glob and returned NoSourceFiles.
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("myscript"), "pub fn collect_widgets() {}\n").unwrap();

    hug_cmd()
        .current_dir(dir.path())
        .args(["symbols", "myscript", "--language", "rust", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("collect_widgets"));
}

#[test]
fn test_language_override_explicit_file_respects_exclude_files() {
    // Regression: `--exclude-files` must apply to explicitly resolved files, not
    // only to directory/glob scans.
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("alpha"), "pub fn alpha_fn() {}\n").unwrap();
    std::fs::write(dir.path().join("beta"), "pub fn beta_fn() {}\n").unwrap();

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "symbols",
            "alpha",
            "beta",
            "--language",
            "rust",
            "--exclude-files",
            "beta",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("alpha_fn"))
        .stdout(predicate::str::contains("beta_fn").not());
}

#[test]
fn test_language_override_parses_tsx_as_typescript() {
    // Forcing TypeScript on a .tsx file must use the TSX grammar so JSX parses
    // and symbols are still extracted through the CLI path.
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("component.tsx");
    std::fs::write(
        &path,
        "export function AppRoot() {\n  return <div>hi</div>;\n}\n",
    )
    .unwrap();

    hug_cmd()
        .args([
            "symbols",
            path.to_str().unwrap(),
            "--language",
            "typescript",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("AppRoot"));
}
