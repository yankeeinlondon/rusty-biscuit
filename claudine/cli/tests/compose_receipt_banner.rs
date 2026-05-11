//! Level 1 integration tests for the `compose` / `inline-compose` receipt
//! banner (W1 in `features/2026-05-09-perf/spec.md`).
//!
//! The banner — `→ Composing <file>…` — is the user's first signal that
//! the command was accepted, emitted before any prep work runs. These
//! tests pin its presence/order/suppression behaviour so a future change
//! that drops it (or moves it after the execution header) fails CI.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;

mod common;
use common::{augmented_path, strip_ansi, write_executable};

const RECEIPT_TOKEN: &str = "→ Composing";
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

fn make_workspace_with_goose_inline()
-> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
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
fn compose_receipt_banner_appears_before_execution_header() {
    let (workspace, path_dir, md_file) = make_workspace_with_goose();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    let receipt_pos = plain
        .find(RECEIPT_TOKEN)
        .unwrap_or_else(|| panic!("missing receipt banner; stderr:\n{plain}"));
    let header_pos = plain
        .find(HEADER_TOKEN)
        .unwrap_or_else(|| panic!("missing execution header; stderr:\n{plain}"));

    assert!(
        receipt_pos < header_pos,
        "receipt banner must appear before execution header.\n\
         receipt_pos={receipt_pos} header_pos={header_pos}\nstderr:\n{plain}"
    );
    assert!(
        plain.contains("prompt.md"),
        "receipt banner should include the file ref; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_silent_suppresses_receipt_banner_and_header() {
    let (workspace, path_dir, md_file) = make_workspace_with_goose();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", "--silent", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        !plain.contains(RECEIPT_TOKEN),
        "--silent must suppress the receipt banner; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_quiet_keeps_receipt_banner() {
    // Per spec W1, `--quiet` follows the existing execution header's
    // detail rules; the receipt banner is still emitted so the user has
    // immediate feedback that the command was accepted. Only `--silent`
    // suppresses the banner.
    let (workspace, path_dir, md_file) = make_workspace_with_goose();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", "--quiet", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains(RECEIPT_TOKEN),
        "--quiet must keep the receipt banner; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_receipt_banner_appears_before_execution_header() {
    let (workspace, path_dir, md_file) = make_workspace_with_goose_inline();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    let receipt_pos = plain
        .find(RECEIPT_TOKEN)
        .unwrap_or_else(|| panic!("missing receipt banner; stderr:\n{plain}"));
    let header_pos = plain
        .find(HEADER_TOKEN)
        .unwrap_or_else(|| panic!("missing execution header; stderr:\n{plain}"));

    assert!(
        receipt_pos < header_pos,
        "inline-compose receipt banner must appear before execution header.\n\
         stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_silent_suppresses_receipt_banner() {
    let (workspace, path_dir, md_file) = make_workspace_with_goose_inline();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
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
        !plain.contains(RECEIPT_TOKEN),
        "--silent must suppress the inline-compose receipt banner; stderr:\n{plain}"
    );
}
