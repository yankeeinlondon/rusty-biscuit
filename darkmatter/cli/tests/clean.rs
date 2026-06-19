mod common;

use common::{md_cmd, md_file};
use predicates::prelude::*;
use std::io::Write;

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
    let tmp = md_file("# Hello \n\nWorld  \n");

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_clean_subcommand_indent() {
    md_cmd()
        .args(["clean", "-", "--indent", "4"])
        .write_stdin("- Parent\n  - Child\n    - Grandchild\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\n    - Child"))
        .stdout(predicate::str::contains("\n        - Grandchild"));
}

#[test]
fn test_clean_subcommand_rejects_invalid_indent() {
    md_cmd()
        .args(["clean", "-", "--indent", "3"])
        .write_stdin("- Parent\n  - Child\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("indent must be one of: 2, 4, 8"));
}

#[test]
fn test_clean_subcommand_save_in_place_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace changes only"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("# Hello"));
    assert!(updated.contains("World"));
    assert!(!updated.contains("# Hello "));
    assert!(!updated.contains("World  "));
    assert!(updated.ends_with('\n'));
    assert!(!updated.ends_with("\n\n"));
}

#[test]
fn test_clean_subcommand_save_verbose_after_subcommand_shows_visual_diff() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .args(["clean", "--save", "-v"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace only"))
        .stdout(predicate::str::contains("original"))
        .stdout(predicate::str::contains("updated"))
        .stdout(predicate::str::contains("Content Visual Diff:").not());
}

#[test]
fn test_save_shorthand_cleans_in_place_and_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace changes only"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("# Hello"));
    assert!(updated.contains("World"));
    assert!(!updated.contains("# Hello "));
    assert!(!updated.contains("World  "));
}

#[test]
fn test_clean_save_rejects_stdin() {
    md_cmd()
        .args(["clean", "-", "--save"])
        .write_stdin("# Hello\n\nWorld\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--save requires an input file path (stdin is not supported)",
        ));
}

