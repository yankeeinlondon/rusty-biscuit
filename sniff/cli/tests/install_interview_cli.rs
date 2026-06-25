//! End-to-end smoke test for the install-interview flow in dry-run mode.
//!
//! `--dry-run` skips execution so the test never mutates host state.
//! `NO_COLOR=1` and `--plain` strip escape codes so assertions are stable.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn install_dry_run_plain_emits_announcement_and_success_status() {
    let mut cmd = Command::cargo_bin("sniff").unwrap();
    cmd.env("NO_COLOR", "1")
        .args([
            "software",
            "utilities",
            "install",
            "ripgrep",
            "--dry-run",
            "--yes",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("will be installed"))
        .stdout(predicate::str::contains("successfully"));
}
