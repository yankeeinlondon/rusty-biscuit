//! Opt-in real-provider smoke tests for inline write grants outside the launch workspace.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::tempdir;

mod common;
use common::init_git_repo;

fn real_enabled() -> bool {
    std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() == Ok("1")
}

fn seed_repo(workspace: &Path) -> Option<PathBuf> {
    let repo = workspace.join("repo");
    fs::create_dir_all(&repo).unwrap();
    fs::write(repo.join("README.md"), "# Write-grant smoke test\n").unwrap();
    init_git_repo(&repo).then_some(repo)
}

fn run_external_inline_write(provider_flag: &str, binary: &str) {
    if !real_enabled() {
        eprintln!("skipping real inline write grant (set CLAUDINE_CONTRACT_REAL=1 to run)");
        return;
    }
    if which::which(binary).is_err() {
        eprintln!("skipping real inline write grant (`{binary}` is not on PATH)");
        return;
    }

    let workspace = tempdir().unwrap();
    let external = tempdir().unwrap();
    let Some(repo) = seed_repo(workspace.path()) else {
        eprintln!("skipping real inline write grant (`git init` unavailable)");
        return;
    };
    let document = external.path().join("outside-workspace.md");
    fs::write(
        &document,
        "---\nprompt: Replace this document's body with exactly the single byte x. Do not add a heading, explanation, or trailing punctuation.\n---\nplaceholder\n",
    )
    .unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .current_dir(&repo)
        .env("NO_COLOR", "1")
        .args([
            "inline-compose",
            provider_flag,
            "--no-interactive",
            "--timeout",
            "4m",
            document.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(5 * 60))
        .assert()
        .success();

    let markdown = darkmatter::markdown::Markdown::try_from(document.as_path()).unwrap_or_else(
        |error| {
            panic!(
                "{provider_flag} left an unreadable document: {error}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&assert.get_output().stdout),
                String::from_utf8_lossy(&assert.get_output().stderr)
            )
        },
    );
    assert_eq!(
        markdown.content().trim(),
        "x",
        "{provider_flag} did not write the requested byte outside the launch workspace"
    );
}

#[test]
#[serial_test::serial]
fn real_claude_inline_write_grant_allows_external_document() {
    run_external_inline_write("--claude", "claude");
}

#[test]
#[serial_test::serial]
fn real_codex_inline_write_grant_allows_external_document() {
    run_external_inline_write("--codex", "codex");
}
