use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;

/// Helper to create a `md` command from cargo bin.
fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

// =============================================================================
//                          BASIC FUNCTIONALITY TESTS
// =============================================================================

#[test]
fn test_help_flag() {
    md_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("markdown"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("compose"))
        .stdout(predicate::str::contains("toc"))
        .stdout(predicate::str::contains("delta"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("hash"));
}

#[test]
fn test_version_flag() {
    md_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("md"));
}

#[test]
fn test_stdin_rendering_auto_non_tty_outputs_markdown() {
    md_cmd()
        .arg("-")
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_file_rendering() {
    // Create a temporary markdown file and render it
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "# Test File\n\nSome content here.").unwrap();

    md_cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test File"));
}

#[test]
fn test_file_not_found() {
    md_cmd()
        .arg("/tmp/nonexistent-darkmatter-test-file.md")
        .assert()
        .failure();
}

#[test]
fn test_output_markdown_alias_text() {
    md_cmd()
        .args(["--output", "text", "-"])
        .write_stdin("# Alias Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Alias Test"));
}

#[test]
fn test_output_html() {
    md_cmd()
        .args(["--output", "html", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<style>"))
        .stdout(predicate::str::contains("<h1>Hello</h1>"));
}

#[test]
fn test_output_json_alias_ast() {
    md_cmd()
        .args(["--output", "ast", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\""));
}

#[test]
fn test_show_option_with_markdown_output() {
    md_cmd()
        .env("MD_DRY_RUN", "1")
        .args(["--output", "markdown", "--show", "-"])
        .write_stdin("# Show Test")
        .assert()
        .success();
}

#[test]
fn test_removed_flags_are_rejected() {
    for flag in [
        "--html",
        "--show-html",
        "--ast",
        "--json",
        "--no-images",
        "--toc",
        "--delta",
        "--clean",
        "--clean-save",
        "--fm-merge-with",
        "--fm-defaults",
    ] {
        md_cmd()
            .args([flag, "-"])
            .write_stdin("# Test")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }
}

#[test]
fn test_subcommand_rejects_render_options() {
    md_cmd()
        .args(["--output", "html", "toc", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommands cannot be combined"));
}

// =============================================================================
//                          SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_toc_subcommand_output() {
    md_cmd()
        .args(["toc", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Top"))
        .stdout(predicate::str::contains("Section A"));
}

#[test]
fn test_toc_subcommand_json_output() {
    md_cmd()
        .args(["toc", "--json", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"structure\""));
}

#[test]
fn test_delta_subcommand_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified"));
}

#[test]
fn test_delta_subcommand_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"classification\""));
}

// =============================================================================
//                          CLEAN SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_clean_subcommand_stdin() {
    md_cmd()
        .args(["clean", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_clean_subcommand_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

// =============================================================================
//                          READ SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_read_explicit() {
    md_cmd()
        .args(["read", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_read_default_backward_compat() {
    // md file.md (no subcommand) still works
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "# Backward\n\nCompat test.").unwrap();

    md_cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Backward"));
}

#[test]
fn test_read_explicit_with_output() {
    md_cmd()
        .args(["read", "--output", "html", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1>Hello</h1>"));
}

// =============================================================================
//                          COMPOSE SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_compose_basic() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_compose_with_state() {
    md_cmd()
        .args(["compose", "-", "--state", r#"{"name":"Alice"}"#])
        .write_stdin("# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Alice"));
}

#[test]
fn test_compose_output_html() {
    md_cmd()
        .args(["compose", "-", "--output", "html"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1>Hello</h1>"));
}

#[test]
fn test_compose_strips_frontmatter() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"))
        .stdout(predicate::str::contains("---").not());
}

#[test]
fn test_compose_invalid_state() {
    md_cmd()
        .args(["compose", "-", "--state", "bad json"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_compose_state_requires_json_object() {
    md_cmd()
        .args(["compose", "-", "--state", "[1,2,3]"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a JSON object"));
}

// =============================================================================
//                          HASH SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_hash_default_outputs_two_hashes() {
    // Default mode: frontmatter_hash-body_hash (each 16 hex chars)
    md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_body_only() {
    md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_frontmatter_only() {
    md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_no_frontmatter() {
    // Document with no frontmatter should still produce a valid hash pair
    md_cmd()
        .args(["hash", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_deterministic() {
    // Same input should produce the same hash
    let result1 = md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_frontmatter_reordering() {
    // Frontmatter with different key ordering should produce the same hash
    let result1 = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Hello\nauthor: Alice\n---\n# Content")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\nauthor: Alice\ntitle: Hello\n---\n# Content")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_body_whitespace_insensitive() {
    // Body with different whitespace should produce the same hash (non-strict)
    let result1 = md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_whitespace_sensitive() {
    // With --strict, different whitespace should produce different hashes
    let result1 = md_cmd()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_ne!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_frontmatter_differs_from_normalized() {
    // Strict and non-strict use different serialization strategies, so their
    // hashes should differ (strict uses serde_yaml, non-strict uses sorted canonical JSON)
    let input = "---\ntitle: Hello\nauthor: Alice\n---\n# Content";
    let strict = md_cmd()
        .args(["hash", "--frontmatter", "--strict", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    let normal = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin(input)
        .output()
        .unwrap();

    assert_ne!(strict.stdout, normal.stdout);
}

#[test]
fn test_hash_from_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "---\ntitle: File Test\n---\n# Hello\n\nWorld").unwrap();

    md_cmd()
        .arg("hash")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

// =============================================================================
//                      HASH DIRECTORY MODE TESTS
// =============================================================================

/// Helper: create a temp directory with markdown files and return the dir.
fn create_hash_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.md"),
        "---\ntitle: Beta\n---\n# Beta\n\nSecond file.",
    )
    .unwrap();
    // Nested subdirectory
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(
        dir.path().join("sub/c.md"),
        "---\ntitle: Gamma\n---\n# Gamma\n\nThird file.",
    )
    .unwrap();
    // Non-markdown file (should be ignored)
    std::fs::write(dir.path().join("notes.txt"), "not markdown").unwrap();
    dir
}

#[test]
fn test_hash_directory_default() {
    let dir = create_hash_dir();

    md_cmd()
        .arg("hash")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_body_only() {
    let dir = create_hash_dir();

    md_cmd()
        .args(["hash", "--body"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_frontmatter_only() {
    let dir = create_hash_dir();

    md_cmd()
        .args(["hash", "--frontmatter"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_deterministic() {
    let dir = create_hash_dir();

    let result1 = md_cmd()
        .arg("hash")
        .arg(dir.path())
        .output()
        .unwrap();
    let result2 = md_cmd()
        .arg("hash")
        .arg(dir.path())
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_directory_differs_from_single_file() {
    let dir = create_hash_dir();

    let dir_result = md_cmd()
        .arg("hash")
        .arg(dir.path())
        .output()
        .unwrap();
    let file_result = md_cmd()
        .arg("hash")
        .arg(dir.path().join("a.md"))
        .output()
        .unwrap();

    // Directory aggregate should differ from any single file hash
    assert_ne!(dir_result.stdout, file_result.stdout);
}

#[test]
fn test_hash_directory_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), "# Visible").unwrap();
    std::fs::create_dir(dir.path().join(".hidden")).unwrap();
    std::fs::write(dir.path().join(".hidden/secret.md"), "# Secret").unwrap();

    // Hash with only visible file
    let with_hidden = md_cmd()
        .arg("hash")
        .arg(dir.path())
        .output()
        .unwrap();

    // Hash a dir that only has the visible file (no hidden dir)
    let dir2 = tempfile::tempdir().unwrap();
    std::fs::write(dir2.path().join("a.md"), "# Visible").unwrap();

    let without_hidden = md_cmd()
        .arg("hash")
        .arg(dir2.path())
        .output()
        .unwrap();

    assert_eq!(with_hidden.stdout, without_hidden.stdout);
}

#[test]
fn test_hash_directory_strict() {
    let dir = create_hash_dir();

    let normal = md_cmd()
        .arg("hash")
        .arg(dir.path())
        .output()
        .unwrap();
    let strict = md_cmd()
        .args(["hash", "--strict"])
        .arg(dir.path())
        .output()
        .unwrap();

    // Strict and normal should produce different hashes (different normalization)
    assert_ne!(normal.stdout, strict.stdout);
}

#[test]
fn test_hash_directory_ignores_non_markdown() {
    // A directory with only non-markdown files should still produce a valid hash
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), "not markdown").unwrap();
    std::fs::write(dir.path().join("data.json"), "{}").unwrap();

    md_cmd()
        .arg("hash")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

// =============================================================================
//                          GET SUBCOMMAND TESTS
// =============================================================================

const FM_DOC: &str = "---\ntitle: Hello World\nauthor: Alice\ncount: 42\ntags:\n  - rust\n  - cli\n---\n# Content\n\nBody text.";

#[test]
fn test_get_single_property_string() {
    md_cmd()
        .args(["get", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Hello World\""));
}

#[test]
fn test_get_single_property_number() {
    md_cmd()
        .args(["get", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_single_property_array() {
    md_cmd()
        .args(["get", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_get_missing_property_returns_empty_string() {
    md_cmd()
        .args(["get", "-", "nonexistent"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_multiple_properties_returns_object() {
    md_cmd()
        .args(["get", "-", "title", "author"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"author\""))
        .stdout(predicate::str::contains("\"Alice\""));
}

#[test]
fn test_get_multiple_with_missing_includes_empty_string() {
    md_cmd()
        .args(["get", "-", "title", "missing"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"missing\""))
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_json5_output() {
    md_cmd()
        .args(["get", "--json5", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        // JSON5 uses unquoted keys for valid identifiers
        .stdout(predicate::str::contains("title:"))
        .stdout(predicate::str::contains("count:"));
}

#[test]
fn test_get_yaml_output() {
    md_cmd()
        .args(["get", "--yaml", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello World"));
}

#[test]
fn test_get_toml_output() {
    md_cmd()
        .args(["get", "--toml", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("Hello World"))
        .stdout(predicate::str::contains("count"))
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_from_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "---\nversion: 2\n---\n# Doc").unwrap();

    md_cmd()
        .args(["get"])
        .arg(tmp.path())
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_get_no_frontmatter_returns_empty_string() {
    md_cmd()
        .args(["get", "-", "title"])
        .write_stdin("# No frontmatter")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_requires_at_least_one_prop() {
    md_cmd()
        .args(["get", "-"])
        .write_stdin(FM_DOC)
        .assert()
        .failure();
}
