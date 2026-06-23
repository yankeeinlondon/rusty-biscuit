//! Integration tests: inline-compose interactive collection, capability gating, and handler/readonly recovery.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{write_executable};

#[cfg(unix)]
#[test]
fn inline_compose_interactive_is_capability_gated() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate content\n---\nOriginal body\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "inline-compose",
            "--interactive",
            "--gemini",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(contains(
            "inline-compose in interactive mode (from --interactive) is not supported",
        ));
}

#[cfg(unix)]
#[test]
fn inline_compose_interactive_codex_uses_captured_last_message() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate content\n---\nOriginal body\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output-last-message" ]; then
    shift
    printf 'Interactive body from codex\n' > "$1"
    exit 0
  fi
  shift
done
exit 1
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args([
            "inline-compose",
            "--interactive",
            "--codex",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("Interactive body from codex"),
        "interactive codex body should be applied; file: {final_content}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_readonly_file_fails_without_harness() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("readonly.md");
    fs::write(&md_file, "---\nprompt: Generate\n---\nBody\n").unwrap();

    // Make the file read-only
    let mut perms = fs::metadata(&md_file).unwrap().permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&md_file, perms).unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'should not run'\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("insufficient file permissions")
            || stderr.contains("Permission denied")
            || stderr.contains("permission"),
        "should report a permission error for read-only files; stderr: {stderr}"
    );

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&md_file).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&md_file, perms).unwrap();
}

// ---------------------------------------------------------------------------
// Handler-engagement banner emission semantics
// ---------------------------------------------------------------------------
