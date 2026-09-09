//! End-to-end smoke test for the install-interview flow in dry-run mode.
//!
//! `--dry-run` skips execution so the test never mutates host state.
//! `NO_COLOR=1` and `--plain` strip escape codes so assertions are stable.

use predicates::prelude::*;

mod common;

#[test]
fn install_dry_run_plain_emits_announcement_and_success_status() {
    let fixture = common::SniffCliFixture::named("sniff-install-interview");
    // This case verifies the host-selected install plan without executing it.
    let mut cmd = fixture.command_builder().host_path().build();
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
