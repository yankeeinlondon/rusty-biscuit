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
