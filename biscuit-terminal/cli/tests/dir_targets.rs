//! Integration tests for the `bt dir` cross-target render flags.
//!
//! Covers:
//! - Terminal default output (with `NO_COLOR`)
//! - `--md` (portable Markdown)
//! - `--md-plus` (MarkdownPlus)
//! - `--html` (HTML fragment)
//! - `--example` continues to work with terminal rendering
//! - Mutually exclusive target switches
//!
//! Uses a small on-disk fixture seeded via `tempfile::tempdir()` to keep the
//! tests hermetic.

use predicates::prelude::*;

fn build_fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("create tempdir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(root.join("README.md"), "# fixture\n").unwrap();
    temp
}

#[test]
fn dir_help_lists_target_switches() {
    assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .args(["dir", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--md"))
        .stdout(predicate::str::contains("--md-plus"))
        .stdout(predicate::str::contains("--html"));
}

#[test]
fn dir_md_emits_nested_markdown_list_without_box_drawing() {
    let temp = build_fixture();
    assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .args(["dir", "--md", "--skip-root"])
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("- "))
        .stdout(predicate::str::contains('├').not())
        .stdout(predicate::str::contains('└').not())
        .stdout(predicate::str::contains('│').not());
}

#[test]
fn dir_md_plus_emits_classed_icon_spans() {
    let temp = build_fixture();
    assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .args(["dir", "--md-plus", "--skip-root"])
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("class=\"fs-icon\""));
}

#[test]
fn dir_html_emits_nested_ul_li_with_fs_classes() {
    let temp = build_fixture();
    assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .args(["dir", "--html", "--skip-root"])
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("<ul"))
        .stdout(predicate::str::contains("<li"))
        .stdout(predicate::str::contains("fs-dir"))
        .stdout(predicate::str::contains("fs-file"));
}

#[test]
fn dir_md_and_html_are_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .args(["dir", "--md", "--html"])
        .assert()
        .failure();
}

#[test]
fn dir_md_and_md_plus_are_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .args(["dir", "--md", "--md-plus"])
        .assert()
        .failure();
}

#[test]
fn dir_example_terminal_default_still_works() {
    assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .env("NO_COLOR", "1")
        .args(["dir", "--example"])
        .assert()
        .success()
        // The example renders the repo's own dir with --depth 1 --filter ".rs"
        // and prints the example command afterwards.
        .stdout(predicate::str::contains(
            r#"bt dir . --depth 1 --filter ".rs""#,
        ));
}

#[test]
fn dir_example_with_md_still_prints_example_command() {
    // `--example --md` should run the example fixture and print the
    // example command line at the end, even when routing through Markdown.
    assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .args(["dir", "--example", "--md"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"bt dir . --depth 1 --filter ".rs""#,
        ));
}

#[test]
fn dir_terminal_default_renders_box_drawing() {
    // The terminal default path is the bespoke renderer; verify connectors
    // still appear when stdout is captured. `NO_COLOR` keeps the bytes
    // predictable.
    let temp = build_fixture();
    let output = assert_cmd::Command::cargo_bin("bt")
        .unwrap()
        .env("NO_COLOR", "1")
        .args(["dir", "--skip-root"])
        .arg(temp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&output);
    assert!(
        s.contains('├') || s.contains('└'),
        "expected box drawing in terminal output: {s}",
    );
}
