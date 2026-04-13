//! PTY regression test for the interactive install interview flow.
//!
//! This test is env-var-gated (`SNIFF_INTERACTIVE_PTY=1`) so it only runs
//! when explicitly requested. It verifies that the `sniff utilities install
//! <name> --dry-run --plain` path emits the announcement and success status
//! under a real PTY.
//!
//! A full failure-then-retry flow would require a deliberately-failing
//! fixture program; that exercise is left as a follow-up.

#![cfg(unix)]

use std::time::Duration;

use expectrl::{Expect, Regex, spawn};

#[test]
fn install_dry_run_emits_announcement_and_success_under_pty()
-> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("SNIFF_INTERACTIVE_PTY").is_none() {
        eprintln!("skipping: set SNIFF_INTERACTIVE_PTY=1 to enable");
        return Ok(());
    }

    let binary = assert_cmd::cargo::cargo_bin("sniff");
    let cmd = format!(
        "{} utilities install ripgrep --dry-run --plain --yes",
        binary.display()
    );
    let mut p = spawn(&cmd)?;
    p.set_expect_timeout(Some(Duration::from_secs(10)));

    p.expect(Regex("will be installed"))?;
    p.expect(Regex("installed successfully"))?;
    Ok(())
}
