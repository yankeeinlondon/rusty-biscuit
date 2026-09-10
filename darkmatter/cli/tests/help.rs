mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    let fixture = CliProcessFixture::named("test_help_flag");
    fixture
        .command()
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
    let fixture = CliProcessFixture::named("test_version_flag");
    fixture
        .command()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("md"));
}

#[test]
fn test_removed_flags_are_rejected() {
    let fixture = CliProcessFixture::named("test_removed_flags_are_rejected");
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
        fixture
            .command()
            .args([flag, "-"])
            .write_stdin("# Test")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }
}

#[test]
fn test_subcommand_rejects_render_options() {
    let fixture = CliProcessFixture::named("test_subcommand_rejects_render_options");
    fixture
        .command()
        .args(["--output", "html", "toc", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommands cannot be combined"));
}

#[test]
fn test_list_themes() {
    let fixture = CliProcessFixture::named("test_list_themes");
    fixture
        .command()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available themes"))
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("solarized"));
}

#[test]
fn test_completions_bash() {
    let fixture = CliProcessFixture::named("test_completions_bash");
    fixture
        .command()
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_zsh() {
    let fixture = CliProcessFixture::named("test_completions_zsh");
    fixture
        .command()
        .args(["--completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_fish() {
    let fixture = CliProcessFixture::named("test_completions_fish");
    fixture
        .command()
        .args(["--completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}
