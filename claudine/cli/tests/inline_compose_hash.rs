//! Integration test for inline-compose hash stamping.
//!
//! Verifies that a real `claudine inline-compose` run writes a valid
//! Darkmatter `Simple` hash and that `md hash --diff` exits 0 on the
//! resulting file.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
mod common;
use common::augmented_path;
use common::wrap::seed_minimal_config;

/// Locate the `md` binary in the workspace target directory.
///
/// `assert_cmd::cargo::cargo_bin` works for binaries of the crate under test;
/// `md` lives in `darkmatter-cli`, so use `CARGO_BIN_EXE_md` when Cargo provides
/// it and otherwise resolve the sibling binary next to this test's profile
/// directory. Deriving that directory from `current_exe` respects custom
/// target directories and the platform executable suffix.
fn md_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_md") {
        return path.into();
    }
    let test_exe = std::env::current_exe().expect("current test executable");
    let profile_dir = test_exe
        .parent()
        .and_then(std::path::Path::parent)
        .expect("integration test executable under <profile>/deps");
    let path = profile_dir.join(format!("md{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.exists(),
        "md binary not found at {path:?}; run `cargo build -p darkmatter-cli --bin md`"
    );
    path
}

/// Write a fake `goose` provider that prints a fixed, deterministic replacement
/// body and exits 0, discoverable on `PATH` on every platform.
///
/// Uses a shell script on Unix and compiles a tiny native executable on
/// Windows. A batch file is not a valid stand-in here because the composed
/// prompt intentionally contains newlines. The replacement body is
/// intentionally *dirty* — a heading immediately followed by a paragraph with no
/// blank line between them — so the test also covers the cleanup→hash
/// consistency path (the document is normalized before the hash is stamped, and
/// `md hash --diff` must still match the normalized result).
fn write_goose_provider(bin_dir: &Path) {
    #[cfg(unix)]
    {
        common::write_executable(
            &bin_dir.join("goose"),
            "#!/bin/sh\nprintf '# Replacement heading\\nReplacement body content\\n'\nexit 0\n",
        );
    }
    #[cfg(windows)]
    {
        let source = bin_dir.join("goose-fixture.rs");
        common::write(
            &source,
            r##"fn main() {
    println!("# Replacement heading");
    println!("Replacement body content");
}
"##,
        );
        let output = Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
            .arg("-o")
            .arg(bin_dir.join("goose.exe"))
            .output()
            .expect("rustc must build the Windows provider fixture");
        assert!(
            output.status.success(),
            "provider fixture compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

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

    write_goose_provider(&path_dir);

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        // `dirs::home_dir()` reads `HOME` on Unix and `USERPROFILE` on Windows;
        // set both so the wrapper's config home resolves to the temp workspace
        // on every platform.
        .env("HOME", workspace.path())
        .env("USERPROFILE", workspace.path())
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
