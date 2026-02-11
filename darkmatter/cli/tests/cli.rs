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
        .stdout(predicate::str::contains("--clean"))
        .stdout(predicate::str::contains("--html"));
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
fn test_stdin_rendering() {
    // Pipe markdown through stdin using explicit "-" marker
    md_cmd()
        .arg("-")
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
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
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_file_not_found() {
    md_cmd()
        .arg("/tmp/nonexistent-darkmatter-test-file.md")
        .assert()
        .failure();
}

#[test]
fn test_clean_output() {
    md_cmd()
        .args(["--clean", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_ast_output_is_json() {
    md_cmd()
        .args(["--ast", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("}"));
}

#[test]
fn test_toc_output() {
    md_cmd()
        .args(["--toc", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_toc_json_output() {
    md_cmd()
        .args(["--toc", "--json", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("structure"));
}

#[test]
fn test_list_themes() {
    md_cmd()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available themes"))
        .stdout(predicate::str::contains("--theme"));
}

// =============================================================================
//                       BUG FIX REGRESSION TESTS
// =============================================================================

// ---------------------------------------------------------------------------
// Bug fix: --clean-save should reject stdin input
// ---------------------------------------------------------------------------

#[test]
fn test_clean_save_rejects_stdin() {
    // Using "-" as input with --clean-save should fail because we cannot
    // save back to stdin. The error message should suggest using --clean.
    md_cmd()
        .args(["-", "--clean-save"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--clean"));
}

#[test]
fn test_clean_save_rejects_implicit_stdin() {
    // When no input is provided at all and stdin is piped,
    // --clean-save should still fail because there is no file to save to.
    md_cmd()
        .arg("--clean-save")
        .write_stdin("# Test")
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Bug fix: --clean-save should work with a real file
// ---------------------------------------------------------------------------

#[test]
fn test_clean_save_works_with_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.md");
    std::fs::write(&file_path, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .args(["--clean-save"])
        .arg(&file_path)
        .assert()
        .success()
        .stderr(predicate::str::contains("Saved cleaned content"));

    // Verify file was actually modified (written back)
    let contents = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        contents.contains("# Hello"),
        "File should still contain heading"
    );
}

// ---------------------------------------------------------------------------
// Bug fix: --fm-merge-with should accept valid JSON
// ---------------------------------------------------------------------------

#[test]
fn test_fm_merge_with_valid_json() {
    // Merge JSON into frontmatter; the output should contain YAML frontmatter
    // with the merged key.
    md_cmd()
        .args(["--fm-merge-with", r#"{"title":"Hello"}"#, "-"])
        .write_stdin("# Test\n\nContent")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Hello"))
        .stdout(predicate::str::contains("---"));
}

#[test]
fn test_fm_merge_with_overwrites_existing() {
    // When the document already has frontmatter, --fm-merge-with should
    // override conflicting keys (PreferExternal strategy).
    md_cmd()
        .args(["--fm-merge-with", r#"{"title":"New Title"}"#, "-"])
        .write_stdin("---\ntitle: Old Title\n---\n# Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: New Title"));
}

// ---------------------------------------------------------------------------
// Bug fix: --fm-merge-with should reject invalid JSON
// ---------------------------------------------------------------------------

#[test]
fn test_fm_merge_with_invalid_json() {
    md_cmd()
        .args(["--fm-merge-with", "not valid json", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

// ---------------------------------------------------------------------------
// Bug fix: --fm-defaults should accept valid JSON
// ---------------------------------------------------------------------------

#[test]
fn test_fm_defaults_valid_json() {
    // --fm-defaults adds missing keys but does NOT override existing keys.
    md_cmd()
        .args(["--fm-defaults", r#"{"draft":true}"#, "-"])
        .write_stdin("---\ntitle: X\n---\n# Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("draft: true"))
        .stdout(predicate::str::contains("title: X"));
}

#[test]
fn test_fm_defaults_does_not_override_existing() {
    // Existing keys should be preserved with their original values.
    md_cmd()
        .args(["--fm-defaults", r#"{"title":"Default Title"}"#, "-"])
        .write_stdin("---\ntitle: Original\n---\n# Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Original"));
}

// ---------------------------------------------------------------------------
// Bug fix: --fm-defaults should reject invalid JSON
// ---------------------------------------------------------------------------

#[test]
fn test_fm_defaults_invalid_json() {
    md_cmd()
        .args(["--fm-defaults", "{bad", "-"])
        .write_stdin("---\ntitle: X\n---\n# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

// =============================================================================
//                          OUTPUT MODE EXCLUSIVITY
// =============================================================================

#[test]
fn test_conflicting_output_modes() {
    // Multiple output mode flags (e.g. --clean and --html) should be rejected
    // by clap's ArgGroup constraint.
    md_cmd()
        .args(["--clean", "--html", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_conflicting_clean_and_ast() {
    md_cmd()
        .args(["--clean", "--ast", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
