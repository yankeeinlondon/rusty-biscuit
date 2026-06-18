mod common;

use common::md_cmd;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    md_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("markdown"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("compose"))
        .stdout(predicate::str::contains("toc"))
        .stdout(predicate::str::contains("delta"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("hash"))
        .stdout(predicate::str::contains("graph"));
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


#[test]
fn test_list_themes() {
    md_cmd()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available themes"))
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("solarized"));
}

#[test]
fn test_completions_bash() {
    md_cmd()
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_zsh() {
    md_cmd()
        .args(["--completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_fish() {
    md_cmd()
        .args(["--completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

