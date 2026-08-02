#![cfg(unix)]

//! Level 1 integration tests for the `compose` / `inline-compose`
//! execution header as the first user-facing signal.
//!
//! The old `→ Composing <file>…` receipt banner was removed: the
//! execution line (`Claudine ▸ <Agent> … Compose … prompt sourced from
//! <file>`) is now rendered up front — the moment the agent is resolved,
//! before the expensive prepare/compose work — so it *is* the immediate
//! feedback. These tests pin that the banner is gone and the header is the
//! first line, and that `--silent` / `--quiet` behave as before.

use std::fs;
use tempfile::tempdir;

mod common;
use common::{augmented_path, strip_ansi, write_executable};

/// The removed banner token — must never reappear.
const BANNER_TOKEN: &str = "Composing";
/// The execution header carries the compose badge.
const HEADER_TOKEN: &str = "Compose";

fn make_workspace_with_goose() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    let md_file = workspace.path().join("prompt.md");
    fs::write(&md_file, "# Compose body\nHello from compose.\n").unwrap();
    (workspace, path_dir, md_file)
}

fn make_workspace_with_goose_inline() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf)
{
    // inline-compose requires a frontmatter `prompt:` property because
    // the prompt body is composed inline (a separate `prompt` block) and
    // injected into the agent invocation. The plain compose flow has no
    // such requirement.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    let md_file = workspace.path().join("prompt.md");
    fs::write(
        &md_file,
        "---\nprompt: |\n  Hello from inline compose.\n---\n# Body\n",
    )
    .unwrap();
    (workspace, path_dir, md_file)
}

#[cfg(unix)]
#[test]
fn compose_execution_header_is_the_first_signal() {
    let (workspace, path_dir, md_file) = make_workspace_with_goose();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        !plain.contains(BANNER_TOKEN),
        "the `→ Composing` receipt banner must be gone; stderr:\n{plain}"
    );

    let header_pos = plain
        .find(HEADER_TOKEN)
        .unwrap_or_else(|| panic!("missing execution header; stderr:\n{plain}"));
    let claudine_pos = plain
        .find("Claudine")
        .unwrap_or_else(|| panic!("missing `Claudine` in header; stderr:\n{plain}"));

    // The header is the first user-facing line: the first non-blank stderr
    // line is the `Claudine ▸ …` execution line.
    let first_nonblank = plain.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    assert!(
        first_nonblank.contains("Claudine"),
        "execution header must be the first non-blank stderr line; got: {first_nonblank:?}\nstderr:\n{plain}"
    );
    assert!(
        claudine_pos <= header_pos,
        "`Claudine` should lead the execution line; stderr:\n{plain}"
    );
    assert!(
        plain.contains("prompt.md"),
        "header should show the source file ref; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_silent_suppresses_the_execution_header() {
    let (workspace, path_dir, md_file) = make_workspace_with_goose();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", "--silent", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        !plain.contains(BANNER_TOKEN),
        "--silent: no banner; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("Claudine \u{25b8}"),
        "--silent must suppress the execution header; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_quiet_keeps_the_execution_header() {
    // `--quiet` suppresses env details and the prompt block but keeps the
    // execution header so the user still gets immediate feedback.
    let (workspace, path_dir, md_file) = make_workspace_with_goose();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", "--quiet", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        !plain.contains(BANNER_TOKEN),
        "--quiet: no banner; stderr:\n{plain}"
    );
    assert!(
        plain.contains("Claudine") && plain.contains(HEADER_TOKEN),
        "--quiet must keep the execution header; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_execution_header_is_the_first_signal() {
    let (workspace, path_dir, md_file) = make_workspace_with_goose_inline();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        !plain.contains(BANNER_TOKEN),
        "inline-compose: the receipt banner must be gone; stderr:\n{plain}"
    );
    let first_nonblank = plain.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    assert!(
        first_nonblank.contains("Claudine"),
        "inline-compose execution header must be the first non-blank stderr line; got: {first_nonblank:?}\nstderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_silent_suppresses_the_execution_header() {
    let (workspace, path_dir, md_file) = make_workspace_with_goose_inline();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "inline-compose",
            "--goose",
            "--silent",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        !plain.contains(BANNER_TOKEN),
        "--silent: no banner; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("Claudine \u{25b8}"),
        "--silent must suppress the inline-compose execution header; stderr:\n{plain}"
    );
}
