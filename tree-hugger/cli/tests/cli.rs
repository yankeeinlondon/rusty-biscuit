use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
#[cfg(unix)]
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
    // --language is a subcommand-local flag, so it must follow the subcommand.
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
        .args(["symbols", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--comments"));
}

#[test]
fn test_exclude_flags_exist_in_help() {
    hug_cmd()
        .args(["symbols", "--help"])
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
        .args(["lint", "--no-cache", "--plain", "probe.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("undefined-symbol").not());

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "lint",
            "--no-cache",
            "--plain",
            "--experimental-semantics",
            "probe.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("undefined-symbol"));

    hug_cmd()
        .current_dir(dir.path())
        .args([
            "lint",
            "--no-cache",
            "--plain",
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
            "lint",
            "--no-cache",
            "--plain",
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
            "lint",
            "--no-cache",
            "--plain",
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
            "lint",
            "--no-cache",
            "--plain",
            "--deny",
            "category:restriction",
            "probe.rs",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error"))
        .stdout(predicate::str::contains("[unwrap-call]"));

    // `--allow` demotes severity but does not enable a default-off rule, so it
    // must be paired with an enabling selector. `--allow` wins over `--warn`.
    hug_cmd()
        .current_dir(dir.path())
        .args([
            "lint",
            "--no-cache",
            "--plain",
            "--warn",
            "unwrap-call",
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
fn test_lint_restriction_rules_off_by_default() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("probe.rs");
    std::fs::write(&path, "fn main() {\n    Some(1).unwrap();\n}\n").unwrap();

    // unwrap-call is a `restriction` rule: silent unless explicitly enabled.
    hug_cmd()
        .current_dir(dir.path())
        .args(["lint", "--no-cache", "--plain", "probe.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[unwrap-call]").not());

    // --strict escalates enabled warnings but must not enable a default-off rule.
    hug_cmd()
        .current_dir(dir.path())
        .args(["lint", "--no-cache", "--plain", "--strict", "probe.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[unwrap-call]").not());
}

#[test]
fn test_lint_scan_hides_clean_files_and_excludes_fixtures() {
    let area = tempfile::TempDir::new().unwrap();
    let root = area.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"scan-noise\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    // Clean file: must not appear at all during a multi-file scan.
    std::fs::write(root.join("src/clean.rs"), "pub fn ok() {}\n").unwrap();
    // Dirty file: dbg-macro is on by default, so this is the only file shown.
    std::fs::write(root.join("src/dirty.rs"), "fn run() {\n    dbg!(1);\n}\n").unwrap();
    // Fixture: an unused import that must be excluded from the default scan.
    std::fs::create_dir_all(root.join("tests/fixtures")).unwrap();
    std::fs::write(
        root.join("tests/fixtures/sample.rs"),
        "use std::io::Write;\n",
    )
    .unwrap();

    hug_cmd()
        .current_dir(root)
        .args(["lint", "--no-cache", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dirty.rs"))
        .stdout(predicate::str::contains("[dbg-macro]"))
        .stdout(predicate::str::contains("clean.rs").not())
        .stdout(predicate::str::contains("(no diagnostics)").not())
        .stdout(predicate::str::contains("fixtures").not());
}

#[test]
fn test_lint_explicit_fixture_path_overrides_default_exclude() {
    let area = tempfile::TempDir::new().unwrap();
    let root = area.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"explicit-fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("tests/fixtures")).unwrap();
    std::fs::write(
        root.join("tests/fixtures/sample.rs"),
        "use std::io::Write;\n",
    )
    .unwrap();

    // Naming the fixture explicitly re-includes it despite the default exclude
    // (`unused-import` is off by default, so opt in to get a visible signal).
    hug_cmd()
        .current_dir(root)
        .args([
            "lint",
            "--no-cache",
            "--plain",
            "--warn",
            "unused-import",
            "tests/fixtures/sample.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[unused-import]"));
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

    assert_eq!(unwrap_rule["category"], "restriction");
    assert_eq!(unwrap_rule["enabled_by_default"], false);
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
fn test_lint_cache_reuse_respects_policy_selector_changes() {
    // Policy is applied after the symbol cache is consulted, so reusing a cached
    // entry must not pin the earlier run's severity. First run warms the cache
    // with a `--warn unwrap-call` warning; the second run reuses it under
    // `--deny` and must still escalate to an error and fail.
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("probe.rs");
    std::fs::write(&path, "fn main() {\n    Some(1).unwrap();\n}\n").unwrap();

    hug_cmd()
        .current_dir(dir.path())
        .args(["--plain", "lint", "--warn", "unwrap-call", "probe.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("warning"))
        .stdout(predicate::str::contains("[unwrap-call]"));

    hug_cmd()
        .current_dir(dir.path())
        .args(["--plain", "lint", "--deny", "unwrap-call", "probe.rs"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error"))
        .stdout(predicate::str::contains("[unwrap-call]"));
}

#[test]
fn test_lint_json_includes_syntax_metadata() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("broken.rs");
    std::fs::write(&path, "fn main( {\n").unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["lint", "--no-cache", "--json", "broken.rs"])
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

// The shim below is a `#!/bin/sh` script made executable via mode bits and put on a
// `:`-separated PATH — three Unix-only mechanics, so the test cannot run on Windows.
#[cfg(unix)]
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
        .args(["lint", "--no-cache", "--json", "probe.js"])
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

// ============================================================================
// God-files command tests (Phase 6)
// ============================================================================

#[test]
fn test_god_files_help() {
    hug_cmd()
        .args(["god-files", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Identify oversized source files"))
        .stdout(predicate::str::contains("--high-risk"));
}

#[test]
fn test_god_files_json_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let content = "x = 1\n".repeat(1000);
    std::fs::write(dir.path().join("big.py"), content).unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let arr = json.as_array().expect("json array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["relative_path"], "big.py");
    assert_eq!(arr[0]["risk"], "High");
    assert!(arr[0]["effective_sloc"].as_u64().unwrap() >= 1000);
    // A cleanly-parsed file carries no diagnostic note.
    assert!(arr[0].get("note").is_none());
}

#[test]
fn test_god_files_json_top_level_excludes_locals() {
    // Public-boundary regression: ten top-level functions, each with one
    // parameter and many local assignments. The structural summary must count
    // the ten functions only — locals and parameters are not top-level — and
    // the ManyUnrelatedTopLevel hint must report 10, not hundreds.
    let dir = tempfile::TempDir::new().unwrap();
    let mut content = String::new();
    for f in 0..10 {
        content.push_str(&format!("def f{f}(arg):\n"));
        for v in 0..44 {
            content.push_str(&format!("    local_{f}_{v} = arg + {v}\n"));
        }
    }
    std::fs::write(dir.path().join("functions.py"), content).unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let arr = json.as_array().expect("json array");
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0]["top_level_symbol_count"].as_u64().unwrap(),
        10,
        "only the ten functions are top-level"
    );
    let histogram = &arr[0]["kind_histogram"];
    assert_eq!(
        histogram["Function"].as_u64().unwrap(),
        10,
        "histogram must tally the ten top-level functions"
    );
    assert!(
        histogram.get("Variable").is_none() && histogram.get("Parameter").is_none(),
        "locals and parameters must not appear in the top-level histogram: {histogram}"
    );
    let hints = arr[0]["refactor_hints"].as_array().expect("hints array");
    assert!(
        hints.iter().any(|h| h["kind"] == "many_unrelated_top_level"
            && h["count"].as_u64() == Some(10)),
        "ManyUnrelatedTopLevel hint count must be 10, got {hints:?}"
    );
}

#[test]
fn test_god_files_json_reports_control_flow_nesting() {
    // Public-boundary regression: a single function with eight nested `if`
    // blocks must surface deep control-flow nesting and a DeeplyNested hint.
    // Symbol-span nesting reported a trivial depth here and emitted no hint.
    let dir = tempfile::TempDir::new().unwrap();
    let mut content = String::from("def handler(value):\n");
    for level in 0..8 {
        let indent = "    ".repeat(level + 1);
        content.push_str(&format!("{indent}if value > {level}:\n"));
    }
    content.push_str(&format!("{}return value\n", "    ".repeat(9)));
    for i in 0..420 {
        content.push_str(&format!("x{i} = {i}\n"));
    }
    std::fs::write(dir.path().join("deep.py"), content).unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let arr = json.as_array().expect("json array");
    assert_eq!(arr.len(), 1);
    assert!(
        arr[0]["max_nesting_depth"].as_u64().unwrap() >= 8,
        "expected control-flow nesting depth >= 8, got {}",
        arr[0]["max_nesting_depth"]
    );
    let hints = arr[0]["refactor_hints"].as_array().expect("hints array");
    assert!(
        hints.iter().any(|h| h["kind"] == "deeply_nested"),
        "deep control flow must emit a DeeplyNested hint, got {hints:?}"
    );
}

#[test]
fn test_god_files_json_reports_nesting_non_python_forms() {
    // Public-boundary regression for the iteration-9 cross-language gap: the
    // four grammar-specific forms that previously reported `max_nesting_depth: 0`
    // at the CLI boundary — Perl `unless`, C++ range-for, Java
    // try-with-resources, and Swift repeat-while. Each file nests eight of the
    // construct and must surface depth >= 8 with a `deeply_nested` hint, padded
    // past the god-file SLOC floor.
    fn nest(open: impl Fn(usize) -> String, close: impl Fn(usize) -> String, body: &str) -> String {
        let mut s = String::new();
        for level in 0..8 {
            s.push_str(&open(level));
        }
        s.push_str(body);
        for level in (0..8).rev() {
            s.push_str(&close(level));
        }
        s
    }

    let perl = {
        let mut s = nest(
            |l| format!("{}unless ($value > {l}) {{\n", "  ".repeat(l)),
            |l| format!("{}}}\n", "  ".repeat(l)),
            &format!("{}$value = 1;\n", "  ".repeat(8)),
        );
        for i in 0..420 {
            s.push_str(&format!("my $x{i} = {i};\n"));
        }
        ("deep.pl", s)
    };
    let cpp = {
        let mut s = String::from("void handler(int v[]) {\n");
        s.push_str(&nest(
            |l| format!("{}for (auto a{l} : v) {{\n", "  ".repeat(l + 1)),
            |l| format!("{}}}\n", "  ".repeat(l + 1)),
            &format!("{}int z = 1;\n", "  ".repeat(9)),
        ));
        s.push_str("}\n");
        for i in 0..420 {
            s.push_str(&format!("int g{i} = {i};\n"));
        }
        ("deep.cpp", s)
    };
    let java = {
        let mut s = String::from("class Handler {\n  void run() throws Exception {\n");
        s.push_str(&nest(
            |l| format!("{}try (var r{l} = open()) {{\n", "    ".repeat(l + 1)),
            |l| format!("{}}}\n", "    ".repeat(l + 1)),
            &format!("{}use();\n", "    ".repeat(9)),
        ));
        s.push_str("  }\n");
        for i in 0..420 {
            s.push_str(&format!("  int g{i} = {i};\n"));
        }
        s.push_str("}\n");
        ("deep.java", s)
    };
    let swift = {
        let mut s = String::from("func handler() {\n");
        s.push_str(&nest(
            |l| format!("{}repeat {{\n", "  ".repeat(l + 1)),
            |l| format!("{}}} while c{l}\n", "  ".repeat(l + 1)),
            &format!("{}work()\n", "  ".repeat(9)),
        ));
        s.push_str("}\n");
        for i in 0..420 {
            s.push_str(&format!("let g{i} = {i}\n"));
        }
        ("deep.swift", s)
    };
    let zsh = {
        let mut s = nest(
            |l| format!("{}select x{l} in a b; do\n", "  ".repeat(l)),
            |l| format!("{}done\n", "  ".repeat(l)),
            &format!("{}:;\n", "  ".repeat(8)),
        );
        for i in 0..420 {
            s.push_str(&format!("local g{i}={i}\n"));
        }
        ("deep.zsh", s)
    };

    for (name, content) in [perl, cpp, java, swift, zsh] {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(name), content).unwrap();

        let output = hug_cmd()
            .current_dir(dir.path())
            .args(["god-files", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json: Value = serde_json::from_slice(&output).unwrap();
        let arr = json.as_array().expect("json array");
        assert_eq!(arr.len(), 1, "{name}: expected one god-file record");
        assert!(
            arr[0]["max_nesting_depth"].as_u64().unwrap() >= 8,
            "{name}: expected control-flow nesting depth >= 8, got {}",
            arr[0]["max_nesting_depth"]
        );
        let hints = arr[0]["refactor_hints"].as_array().expect("hints array");
        assert!(
            hints.iter().any(|h| h["kind"] == "deeply_nested"),
            "{name}: deep control flow must emit a DeeplyNested hint, got {hints:?}"
        );
    }
}

#[test]
fn test_god_files_high_risk_filters_json() {
    // `--high-risk` must filter the JSON payload, not only the rendered report.
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("high.py"), "x = 1\n".repeat(1000)).unwrap();
    std::fs::write(dir.path().join("moderate.py"), "x = 1\n".repeat(500)).unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--high-risk", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let arr = json.as_array().expect("json array");
    assert_eq!(arr.len(), 1, "only the high-risk file should be emitted");
    assert_eq!(arr[0]["relative_path"], "high.py");
    assert_eq!(arr[0]["risk"], "High");
    assert!(
        !arr.iter().any(|a| a["risk"] == "Moderate"),
        "no moderate-risk records may appear under --high-risk"
    );
}

#[test]
fn test_god_files_plain_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let content = "x = 1\n".repeat(1000);
    std::fs::write(dir.path().join("big.py"), content).unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !stdout.contains('\x1b'),
        "plain output must not contain ANSI escape codes"
    );
    // Report heading reports both band counts (spec §5.3).
    assert!(stdout.contains("There are 0 files with moderate risk of being considered god files"));
    assert!(stdout.contains("There are 1 files with high risk of being considered god files"));
    // High-risk section heading and the file item.
    assert!(stdout.contains("High risk"));
    assert!(stdout.contains("the big.py file is 1000 lines of code"));
}

#[test]
fn test_god_files_high_risk_suppresses_moderate_section() {
    let dir = tempfile::TempDir::new().unwrap();
    let high_content = "x = 1\n".repeat(1000);
    let moderate_content = "x = 1\n".repeat(500);
    std::fs::write(dir.path().join("high.py"), high_content).unwrap();
    std::fs::write(dir.path().join("moderate.py"), moderate_content).unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--high-risk", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // Under --high-risk the moderate count line is suppressed; only the
    // high-risk count is reported.
    assert!(
        !stdout.contains("moderate risk of being considered god files"),
        "moderate count line should be absent with --high-risk"
    );
    assert!(stdout.contains("There are 1 files with high risk of being considered god files"));
    // High body present.
    assert!(stdout.contains("the high.py file is 1000 lines of code"));
    // The moderate section heading and its body are both suppressed (spec §5.1).
    assert!(
        !stdout.contains("Moderate risk"),
        "moderate section heading should be absent with --high-risk"
    );
    assert!(
        !stdout.contains("moderate.py"),
        "moderate body should be suppressed with --high-risk"
    );
}

#[test]
fn test_god_files_empty_scan() {
    let dir = tempfile::TempDir::new().unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // Both counts are visible as 0/0 (spec §7).
    assert!(stdout.contains("There are 0 files with moderate risk of being considered god files"));
    assert!(stdout.contains("There are 0 files with high risk of being considered god files"));
    // Both risk sections are omitted for empty scans.
    assert!(
        !stdout.contains("High risk"),
        "empty scan should omit the high-risk section heading"
    );
    assert!(
        !stdout.contains("Moderate risk"),
        "empty scan should omit the moderate-risk section heading"
    );
    assert!(
        !stdout.contains("lines of code"),
        "empty scan should list no files"
    );
}

#[test]
fn test_god_files_osc8_links_to_scan_root() {
    // Regression: the OSC8 target must resolve against the *scan root*, not the
    // process working directory. Scan an explicit directory from a different
    // CWD and confirm the hyperlink points at the scanned file.
    let scan_dir = tempfile::TempDir::new().unwrap();
    let cwd_dir = tempfile::TempDir::new().unwrap();
    let content = "x = 1\n".repeat(1000);
    std::fs::write(scan_dir.path().join("big.py"), content).unwrap();

    let scan_suffix = scan_dir
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.rsplit('.').next())
        .expect("scan dir name suffix");

    let output = hug_cmd()
        .current_dir(cwd_dir.path())
        .env("CLICOLOR_FORCE", "1")
        .args(["god-files", scan_dir.path().to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // An OSC8 file hyperlink is emitted, targeting the scan root.
    assert!(stdout.contains("\x1b]8;;file://"), "expected an OSC8 file link");
    assert!(
        stdout.contains(scan_suffix),
        "OSC8 target should point at the scan root, got: {stdout:?}"
    );
}

#[test]
fn test_god_files_unparseable_note() {
    let dir = tempfile::TempDir::new().unwrap();
    // >400 physical lines with an invalid UTF-8 byte so TreeFile::new fails and
    // the candidate falls back to physical-line banding with a diagnostic note.
    let mut content = Vec::new();
    for _ in 0..600 {
        content.extend_from_slice(b"x = 1\n");
    }
    content[3] = 0xFF;
    std::fs::write(dir.path().join("broken.py"), &content).unwrap();

    // Plain output surfaces the note.
    let plain = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plain = String::from_utf8_lossy(&plain);
    assert!(plain.contains("note:"), "plain output should show the note");
    assert!(plain.contains("could not parse"));

    // JSON carries the structured note field.
    let json_out = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&json_out).unwrap();
    let arr = json.as_array().expect("json array");
    assert_eq!(arr.len(), 1);
    assert!(
        arr[0]["note"].as_str().is_some_and(|s| s.contains("could not parse")),
        "json should carry the diagnostic note"
    );
}

#[test]
fn test_god_files_nonexistent_dir_errors() {
    // A missing scan root is an invocation error, not a successful empty scan.
    let dir = tempfile::TempDir::new().unwrap();
    let missing = dir.path().join("does-not-exist");

    hug_cmd()
        .args(["god-files", missing.to_str().unwrap(), "--plain"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_god_files_file_valued_dir_errors() {
    // A scan root that resolves to a file (not a directory) must error rather
    // than silently report 0/0.
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("not_a_dir.py");
    std::fs::write(&file, "x = 1\n".repeat(1000)).unwrap();

    hug_cmd()
        .args(["god-files", file.to_str().unwrap(), "--plain"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn test_god_files_default_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    let content = "x = 1\n".repeat(1000);
    std::fs::write(dir.path().join("big.py"), content).unwrap();

    let output = hug_cmd()
        .current_dir(dir.path())
        .args(["god-files", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let arr = json.as_array().expect("json array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["relative_path"], "big.py");
}
