use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
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
    // A package-area root (no manifest of its own) holding `lib/` and `cli/`
    // child packages, mirroring layouts like `biscuit-terminal/`. Built in a
    // temp dir (outside the repo) so `--prelude` only walks these few files —
    // running against a real area would parse hundreds of source files just to
    // surface the handful of prelude-named functions.
    let area = tempfile::TempDir::new().unwrap();
    let root = area.path();

    // `lib` child: a function defined in a deep module and re-exported through
    // the child package's own prelude. Discovery must descend into `lib/` to
    // find this prelude and resolve `parse_width_spec`.
    std::fs::create_dir_all(root.join("lib/src/components")).unwrap();
    std::fs::write(
        root.join("lib/Cargo.toml"),
        "[package]\nname = \"prelude-area-lib\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("lib/src/lib.rs"),
        "pub mod components;\npub mod prelude;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("lib/src/prelude.rs"),
        "pub use crate::components::width::parse_width_spec;\n",
    )
    .unwrap();
    std::fs::write(root.join("lib/src/components/mod.rs"), "pub mod width;\n").unwrap();
    std::fs::write(
        root.join("lib/src/components/width.rs"),
        // `parse_width_spec` is prelude-exported; `not_in_prelude` is not.
        "pub fn parse_width_spec(spec: &str) -> usize { spec.len() }\npub fn not_in_prelude() {}\n",
    )
    .unwrap();

    // `cli` child: scanned as a second package but exports no prelude, so it
    // contributes no symbols. This keeps the multi-package code path live while
    // proving single-package output is not split with per-package headings.
    std::fs::create_dir_all(root.join("cli/src")).unwrap();
    std::fs::write(
        root.join("cli/Cargo.toml"),
        "[package]\nname = \"prelude-area-cli\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("cli/src/main.rs"),
        "pub fn run_cli() {}\nfn main() {}\n",
    )
    .unwrap();

    let mut cmd = hug_cmd();
    cmd.current_dir(root)
        .args(["functions", "--prelude", "--plain"])
        // A single child package supplies the prelude symbols, so output must
        // not be split into per-package sections.
        .assert()
        .success()
        .stdout(predicate::str::contains("\n\nlib\n\n").not())
        .stdout(predicate::str::contains("\n\ncli\n\n").not())
        // Discovered through `lib/`'s prelude.
        .stdout(predicate::str::contains("parse_width_spec"))
        // Not prelude-exported, so excluded.
        .stdout(predicate::str::contains("not_in_prelude").not())
        .stdout(predicate::str::contains("run_cli").not())
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

#[test]
fn test_lint_experimental_semantics_enables_semantic_diagnostics() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("probe.rs");
    std::fs::write(&path, "fn main() {\n    missing_symbol();\n}\n").unwrap();

    hug_cmd()
        .current_dir(dir.path())
        .args(["--no-cache", "--plain", "lint", "probe.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("undefined-symbol").not());

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "--no-cache",
            "--plain",
            "lint",
            "--experimental-semantics",
            "probe.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("undefined-symbol"));

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "--no-cache",
            "--plain",
            "lint",
            "--experimental-semantics",
            "--strict",
            "probe.rs",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("undefined-symbol"));
}

#[test]
fn test_lint_policy_selectors_affect_severity_and_exit_code() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("probe.rs");
    std::fs::write(&path, "fn main() {\n    Some(1).unwrap();\n}\n").unwrap();

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "--no-cache",
            "--plain",
            "lint",
            "--warn",
            "unwrap-call",
            "probe.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("warning"))
        .stdout(predicate::str::contains("[unwrap-call]"));

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "--no-cache",
            "--plain",
            "lint",
            "--deny",
            "unwrap-call",
            "probe.rs",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error"))
        .stdout(predicate::str::contains("[unwrap-call]"));

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "--no-cache",
            "--plain",
            "lint",
            "--deny",
            "category:suspicious",
            "probe.rs",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error"))
        .stdout(predicate::str::contains("[unwrap-call]"));

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "--no-cache",
            "--plain",
            "lint",
            "--allow",
            "unwrap-call",
            "probe.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("info"))
        .stdout(predicate::str::contains("[unwrap-call]"));
}

#[test]
fn test_lint_list_rules_exposes_rule_metadata() {
    hug_cmd()
        .args(["--plain", "lint", "--list-rules"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unwrap-call"))
        .stdout(predicate::str::contains("suspicious"))
        .stdout(predicate::str::contains("default-on"));
}

#[test]
fn test_lint_list_rules_json_exposes_rule_metadata() {
    let output = hug_cmd()
        .args(["--json", "lint", "--list-rules"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();
    let unwrap_rule = json
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["id"] == "unwrap-call")
        .unwrap();

    assert_eq!(unwrap_rule["category"], "suspicious");
    assert_eq!(unwrap_rule["enabled_by_default"], true);
}

#[test]
fn test_lint_cache_separates_experimental_semantics_policy() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("probe.rs");
    std::fs::write(&path, "fn main() {\n    missing_symbol();\n}\n").unwrap();

    hug_cmd()
        .current_dir(dir.path())
        .args(["--plain", "lint", "probe.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("undefined-symbol").not());

    hug_cmd()
        .current_dir(dir.path())
        .args(["--plain", "lint", "--experimental-semantics", "probe.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("undefined-symbol"));
}

#[test]
fn test_lint_json_includes_syntax_metadata() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("broken.rs");
    std::fs::write(&path, "fn main( {\n").unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["--no-cache", "--json", "lint", "broken.rs"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();
    let metadata = &json["files"][0]["syntax"][0]["metadata"];

    assert_eq!(metadata["source"], "SyntaxParser");
    assert_eq!(metadata["category"], "Correctness");
    assert_eq!(metadata["effective_severity"], "Error");
}

#[test]
fn test_lint_invokes_oxlint_for_javascript() {
    let dir = tempfile::TempDir::new().unwrap();
    let bin_dir = dir.path().join("bin");
    std::fs::create_dir(&bin_dir).unwrap();
    let oxlint = bin_dir.join("oxlint");
    std::fs::write(
        &oxlint,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "oxlint 1.0.0"
  exit 0
fi
cat <<'JSON'
{"messages":[{"message":"Avoid debugger","severity":2,"rule_id":"no-debugger","line":1,"column":1,"end_line":1,"end_column":9,"file_path":"probe.js","category":"correctness"}],"exit_code":1}
JSON
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&oxlint).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&oxlint, permissions).unwrap();

    let source = dir.path().join("probe.js");
    std::fs::write(&source, "debugger;\n").unwrap();
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = hug_cmd()
        .current_dir(dir.path())
        .env("PATH", path)
        .args(["--no-cache", "--json", "lint", "probe.js"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();
    let lint = json["files"][0]["lint"].as_array().unwrap();
    let oxlint_diagnostic = lint
        .iter()
        .find(|diagnostic| diagnostic["rule"] == "no-debugger")
        .unwrap();

    assert_eq!(oxlint_diagnostic["message"], "Avoid debugger");
    assert_eq!(oxlint_diagnostic["metadata"]["source"], "ExternalTool");
    assert_eq!(oxlint_diagnostic["severity"], "Error");
}

#[test]
fn test_lint_warns_when_oxlint_is_unavailable() {
    let dir = tempfile::TempDir::new().unwrap();
    let bin_dir = dir.path().join("bin");
    std::fs::create_dir(&bin_dir).unwrap();
    std::fs::write(dir.path().join("probe.js"), "const value = 1;\n").unwrap();

    hug_cmd()
        .current_dir(dir.path())
        .env("PATH", bin_dir)
        .args(["--plain", "lint", "probe.js"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "external lint adapter 'oxlint' is unavailable",
        ));
}

#[test]
fn test_lint_json_reports_unavailable_oxlint_metadata() {
    let dir = tempfile::TempDir::new().unwrap();
    let bin_dir = dir.path().join("bin");
    std::fs::create_dir(&bin_dir).unwrap();
    std::fs::write(dir.path().join("probe.js"), "const value = 1;\n").unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .env("PATH", bin_dir)
        .args(["--json", "lint", "probe.js"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();
    let metadata = &json["adapter_metadata"][0];

    assert_eq!(metadata["tool_name"], "oxlint");
    assert_eq!(metadata["tool_available"], false);
}
