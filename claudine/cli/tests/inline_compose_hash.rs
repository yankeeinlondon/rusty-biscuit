//! Integration test for inline-compose hash stamping.
//!
//! Verifies that a real `claudine inline-compose` run writes a valid
//! Darkmatter `Simple` hash and that `md hash --diff` exits 0 on the
//! resulting file.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::process::Command;
use tempfile::tempdir;
mod common;
use common::{augmented_path, write_executable};
use common::wrap::seed_minimal_config;

/// Locate the `md` binary in the workspace target directory.
///
/// `assert_cmd::cargo::cargo_bin_cmd!` works for binaries of the crate under
/// test; `md` lives in `darkmatter-cli`, so we resolve it relative to the
/// shared workspace target directory via `CARGO_BIN_EXE_md` when available
/// (cargo sets it when the binary is a dependency artifact) and fall back to
/// the first ancestor `Cargo.toml` that declares a `[workspace]` section.
fn md_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_md") {
        return path.into();
    }
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .ancestors()
        .find(|p| {
            let cargo_toml = p.join("Cargo.toml");
            cargo_toml.exists()
                && std::fs::read_to_string(&cargo_toml)
                    .map(|contents| contents.contains("[workspace]"))
                    .unwrap_or(false)
        })
        .expect("workspace root with [workspace] in Cargo.toml");
    let path = workspace_root.join("target/debug/md");
    assert!(
        path.exists(),
        "md binary not found at {path:?}; run `cargo build -p darkmatter-cli --bin md`"
    );
    path
}

#[cfg(unix)]
#[test]
fn inline_compose_writes_hash_that_passes_md_diff() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate the body\nlast_updated: 2026-01-01\n---\nOriginal body\n",
    )
    .unwrap();

    // Minimal provider: returns a deterministic replacement body.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf 'Replacement body content\n'
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("hash:"),
        "inline-compose must stamp a hash property; file:\n{final_content}"
    );
    assert!(
        final_content.contains("Replacement body content"),
        "inline-compose must write the replacement body; file:\n{final_content}"
    );

    // `md hash --diff` must report the stored hash matches the document.
    let output = Command::new(md_bin())
        .env("NO_COLOR", "1")
        .args(["hash", "--diff", md_file.to_str().unwrap()])
        .output()
        .expect("md hash --diff should execute");

    assert!(
        output.status.success(),
        "md hash --diff must exit 0; stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
