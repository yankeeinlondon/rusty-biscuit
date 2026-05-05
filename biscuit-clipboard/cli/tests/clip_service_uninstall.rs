//! Phase 7: integration tests for `clip service uninstall`.
//!
//! Companion to `clip_service_install.rs`. The install round-trip is
//! covered there; this file focuses on the cases where there is nothing
//! to remove.

use assert_cmd::Command;
use predicates::prelude::*;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn uninstall_dry_run_reports_what_would_be_removed() {
    let prefix = tempfile::tempdir().expect("tempdir");
    Command::cargo_bin("clip")
        .expect("locate clip binary")
        .args(["service", "uninstall", "--dry-run", "--prefix"])
        .arg(prefix.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("would remove"));
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn uninstall_when_not_installed_is_a_noop() {
    let prefix = tempfile::tempdir().expect("tempdir");
    Command::cargo_bin("clip")
        .expect("locate clip binary")
        .args(["service", "uninstall", "--prefix"])
        .arg(prefix.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No autostart manifest present"));
}
