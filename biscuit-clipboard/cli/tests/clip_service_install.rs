//! Phase 7: integration tests for `clip service install`.
//!
//! These exercise the `--dry-run` and write paths against a temp prefix
//! so the real `~/Library/LaunchAgents` / `~/.config/systemd/user` /
//! `%APPDATA%\\...\\Startup` directories are never touched.

use assert_cmd::Command;
use predicates::prelude::*;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn install_dry_run_prints_manifest_with_binary_path() {
    let prefix = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("clip").expect("locate clip binary");
    let assertion = cmd
        .args([
            "service",
            "install",
            "--dry-run",
            "--binary",
            "/usr/local/bin/clipper",
            "--prefix",
        ])
        .arg(prefix.path())
        .assert()
        .success();

    let output = assertion.get_output();
    let stdout = std::str::from_utf8(&output.stdout).unwrap();

    assert!(
        stdout.contains("/usr/local/bin/clipper") || stdout.contains("clipper"),
        "dry-run stdout should contain the binary path; got: {stdout}"
    );
    assert!(
        stdout.contains("manifest path"),
        "dry-run stdout should mention the manifest path; got: {stdout}"
    );
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn install_dry_run_does_not_write_to_disk() {
    let prefix = tempfile::tempdir().expect("tempdir");
    Command::cargo_bin("clip")
        .expect("locate clip binary")
        .args([
            "service",
            "install",
            "--dry-run",
            "--binary",
            "/usr/local/bin/clipper",
            "--prefix",
        ])
        .arg(prefix.path())
        .assert()
        .success();

    // Walk the prefix recursively — it should be empty.
    let mut count = 0usize;
    for entry in walk(prefix.path()) {
        if entry.is_file() {
            count += 1;
        }
    }
    assert_eq!(
        count,
        0,
        "dry-run must not write any files under {:?}",
        prefix.path()
    );
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn install_writes_manifest_then_uninstall_removes_it() {
    let prefix = tempfile::tempdir().expect("tempdir");

    // First install should write.
    Command::cargo_bin("clip")
        .expect("locate clip binary")
        .args([
            "service",
            "install",
            "--binary",
            "/usr/local/bin/clipper",
            "--prefix",
        ])
        .arg(prefix.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed autostart manifest"));

    // Second install should be idempotent.
    Command::cargo_bin("clip")
        .expect("locate clip binary")
        .args([
            "service",
            "install",
            "--binary",
            "/usr/local/bin/clipper",
            "--prefix",
        ])
        .arg(prefix.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already present"));

    // Sanity: at least one file lives under the prefix.
    let files: Vec<_> = walk(prefix.path()).filter(|p| p.is_file()).collect();
    assert!(
        !files.is_empty(),
        "install should have written at least one file"
    );

    // Uninstall removes it.
    Command::cargo_bin("clip")
        .expect("locate clip binary")
        .args(["service", "uninstall", "--prefix"])
        .arg(prefix.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed autostart manifest"));

    let after: Vec<_> = walk(prefix.path()).filter(|p| p.is_file()).collect();
    assert!(
        after.is_empty(),
        "uninstall should have removed the manifest, found: {after:?}"
    );
}

fn walk(root: &std::path::Path) -> impl Iterator<Item = std::path::PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut out = Vec::new();
    while let Some(p) = stack.pop() {
        if p.is_dir()
            && let Ok(rd) = std::fs::read_dir(&p)
        {
            for e in rd.flatten() {
                stack.push(e.path());
            }
        }
        out.push(p);
    }
    out.into_iter()
}
