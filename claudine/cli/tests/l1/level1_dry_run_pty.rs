//! Level 1 PTY test: `--dry-run` on a terminal asks for no shell approval.
//!
//! The dry run is a passive check: it runs no shell command, so it has
//! nothing to approve, even when stdin and stderr are both TTYs and an
//! approval handler would be available to a real run. This file drives the
//! dry run through a pseudo-terminal to prove the TTY branch stays passive:
//! no `Shell Approval Required` prompt, the `::shell` directive is reported as
//! not checked and previewed unevaluated, and the provider never launches.
//! The normal-mode approval prompt itself is covered by
//! `level1_approval_order_pty.rs` and `shell_approval_prompt.rs`.
//!
//! ## Tier
//!
//! **Level 1.** The test reads the child's output back from a bare
//! pseudo-terminal; no terminal emulator participates.
//!
//! Gating: `#![cfg(unix)]` is the only exclusion, because `expectrl`'s
//! `OsSession` is Unix-only. On a selected platform
//! `expect_level!(Level::L1, pty_available(), ...)` **fails** when the PTY is
//! missing rather than skipping: Level 1 is the mandatory suite, where a skip
//! is indistinguishable from a pass.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test
//! ```

use expectrl::Session;
use expectrl::session::OsSession;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use test_toolkit::{Level, expect_level};

use crate::common;
use common::{CliProcessFixture, pty_available, write_executable};

/// Drain bytes from the PTY for a window so each `try_read` collects
/// whatever the child has flushed so far.
fn read_for(session: &mut OsSession, total_deadline: Duration) -> String {
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    let deadline = Instant::now() + total_deadline;
    session.set_expect_timeout(Some(Duration::from_millis(150)));
    while Instant::now() < deadline {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&scratch[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Wait for a substring (after ANSI stripping) to appear in the cumulative
/// transcript. Returns the full transcript on success; panics on timeout.
fn wait_for_marker(session: &mut OsSession, marker: &str, deadline: Duration) -> String {
    let stop = Instant::now() + deadline;
    let mut transcript = String::new();
    let mut scratch = [0u8; 4096];
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    while Instant::now() < stop {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                transcript.push_str(&String::from_utf8_lossy(&scratch[..n]));
                if common::strip_ansi(&transcript).contains(marker) {
                    return transcript;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    panic!("marker {marker:?} did not appear within {deadline:?}; transcript:\n{transcript}");
}

/// Pre-stage a minimal claudine config so the first-run setup wizard does
/// not intercept the input we send to the approval prompt.
fn stage_default_config(home_dir: &Path) {
    let claudine_dir = home_dir.join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.yaml"), "{}").unwrap();
}

/// A staged workspace with a document whose body holds one unapproved
/// `::shell` command, plus a provider stub that records a launch.
struct StagedApproval {
    fixture: CliProcessFixture,
    md_file: PathBuf,
    /// Written by the provider stub if it ever runs; must stay absent.
    launch_marker: PathBuf,
}

/// Stage the shared approval fixture: default config, a `goose` provider stub
/// that touches `launch_marker` when launched, and a document whose body is a
/// single unapproved `::shell echo tty-approved-marker` command.
fn stage_shell_approval_doc() -> StagedApproval {
    let fixture = CliProcessFixture::named("level1-dry-run-pty");
    stage_default_config(fixture.home());

    let launch_marker = fixture.cwd().join("launched.flag");
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho 'launched' > {marker}\nexit 0\n",
            marker = launch_marker.display()
        ),
    );

    let md_file = fixture.cwd().join("doc.md");
    fs::write(
        &md_file,
        "---\ntitle: dry-run approval\n---\n::shell echo tty-approved-marker\n",
    )
    .unwrap();

    StagedApproval {
        fixture,
        md_file,
        launch_marker,
    }
}

/// Build the `claudine compose --goose --dry-run <doc>` command for a staged
/// fixture, with a color terminal environment so the TTY branch is the one
/// under test regardless of the developer's shell.
fn compose_command(staged: &StagedApproval) -> Command {
    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = staged.fixture.command_std();
    cmd.arg("compose").arg("--goose").arg("--dry-run");
    cmd.arg(staged.md_file.to_str().unwrap());
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd
}

/// On a terminal, `compose --dry-run` over an unapproved `::shell` command
/// shows no approval prompt and runs nothing: the report names the directive
/// as not checked, the preview leaves it unevaluated, the command's marker
/// output never appears, the provider never launches, and the check exits 0.
#[test]
fn level1_pty_dry_run_asks_no_shell_approval_and_runs_nothing() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let staged = stage_shell_approval_doc();
    let mut session: OsSession =
        Session::spawn(compose_command(&staged)).expect("spawn PTY session");

    let mut transcript = wait_for_marker(&mut session, "Previewed 1 of 1 task.", Duration::from_secs(10));
    transcript.push_str(&read_for(&mut session, Duration::from_millis(500)));
    let status = session.get_process().wait().expect("wait for the dry run");

    let plain = common::strip_ansi(&transcript);
    assert!(
        matches!(status, expectrl::process::unix::WaitStatus::Exited(_, 0)),
        "a clean dry run exits 0, got {status:?}; transcript:\n{plain}"
    );
    assert!(!plain.contains("Shell Approval Required"), "the dry run asks for no approval; transcript:\n{plain}");
    assert!(plain.contains("not checked: `::shell` directive"), "transcript:\n{plain}");
    assert!(plain.contains("[not evaluated in a dry run]"), "transcript:\n{plain}");
    assert!(
        !plain.lines().any(|line| line.trim() == "tty-approved-marker"),
        "the command's output never appears; transcript:\n{plain}"
    );
    assert!(!staged.launch_marker.exists(), "the provider must NOT launch under `--dry-run`; transcript:\n{plain}");
}
