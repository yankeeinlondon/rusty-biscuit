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
        .stdout(predicate::str::contains("--output <OUTPUT>"))
        .stdout(predicate::str::contains("--show"))
        .stdout(predicate::str::contains("toc"))
        .stdout(predicate::str::contains("delta"));
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
//                       EXISTING BUG FIX REGRESSION TESTS
// =============================================================================

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
fn test_clean_save_rejects_stdin() {
    md_cmd()
        .args(["-", "--clean-save"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--clean"));
}

#[test]
fn test_clean_save_rejects_implicit_stdin() {
    md_cmd()
        .arg("--clean-save")
        .write_stdin("# Test")
        .assert()
        .failure();
}

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

    let contents = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        contents.contains("# Hello"),
        "File should still contain heading"
    );
}

#[test]
fn test_fm_merge_with_valid_json() {
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
    md_cmd()
        .args(["--fm-merge-with", r#"{"title":"New Title"}"#, "-"])
        .write_stdin("---\ntitle: Old Title\n---\n# Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: New Title"));
}

#[test]
fn test_fm_merge_with_invalid_json() {
    md_cmd()
        .args(["--fm-merge-with", "not valid json", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_fm_defaults_valid_json() {
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
    md_cmd()
        .args(["--fm-defaults", r#"{"title":"Default Title"}"#, "-"])
        .write_stdin("---\ntitle: Original\n---\n# Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Original"));
}

#[test]
fn test_fm_defaults_invalid_json() {
    md_cmd()
        .args(["--fm-defaults", "{bad", "-"])
        .write_stdin("---\ntitle: X\n---\n# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}
