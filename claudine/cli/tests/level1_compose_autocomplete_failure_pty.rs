//! Level 1 PTY tests for ENTER-path autocomplete failure modes.
//!
//! The ENTER autocomplete path has three observable failure modes: no matches,
//! over `MAX_CANDIDATES`, and non-TTY. The non-TTY path is already covered by
//! `wrap_compose_validation.rs`; this file exercises the two interactive
//! failures through a real pseudo-terminal so the CLI route from an unresolved
//! `claudine compose <partial>` file reference into autocomplete is verified
//! end-to-end.
//!
//! Each test stages a workspace with a detected repo root (`.git`) and a
//! `prompts/` scope, spawns `claudine compose --goose <query>` under a PTY so
//! `stdin` and `stderr` are TTYs, and asserts the rendered error contains the
//! expected message, the process exits non-zero, and the provider stub is never
//! launched.

#![cfg(unix)]

use expectrl::Session;
use expectrl::process::unix::WaitStatus;
use expectrl::session::OsSession;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

mod common;
use common::{CliProcessFixture, pty_available, strip_ansi, write_executable};

const MAX_CANDIDATES: usize = 500;

/// Drain all currently available bytes from the PTY master until a quiet period
/// or the overall deadline elapses.
fn drain_available(session: &mut OsSession, total_deadline: Duration) -> String {
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    let deadline = Instant::now() + total_deadline;
    session.set_expect_timeout(Some(Duration::from_millis(300)));
    while Instant::now() < deadline {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&scratch[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Build a workspace ready for an ENTER-path compose invocation:
///
/// - `.git/` so `sniff` detects a repo root and `prompts/` is in scope.
/// - Empty user config so the first-run wizard does not intercept input.
/// - A fake `goose` provider that records any launch with a marker file and
///   stderr output (so the test can prove the provider was never reached).
fn stage_workspace() -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named("level1-compose-autocomplete-pty");
    let prompts_dir = fixture.cwd().join("prompts");
    fs::create_dir_all(&prompts_dir).unwrap();
    fs::create_dir_all(fixture.cwd().join(".git")).unwrap();

    let claudine_dir = fixture.home().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    let marker = fixture.cwd().join("provider-launched.flag");
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho 'PROVIDER_WAS_LAUNCHED' > {marker}\nexit 1\n",
            marker = marker.display()
        ),
    );

    (fixture, prompts_dir)
}

/// Build a PTY-spawnable `claudine compose --goose <query>` command inside the
/// staged workspace.
///
/// `expectrl` needs a live `std::process::Command`, so this is the builder's
/// raw surface; the call site keeps only what the PTY itself requires.
fn compose_command(fixture: &CliProcessFixture, query: &str) -> Command {
    let mut cmd = fixture.command_std();
    cmd.args(["compose", "--goose", query]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("TERM_WIDTH", "80");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd
}

#[test]
fn level1_pty_compose_autocomplete_no_match_errors() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, prompts_dir) = stage_workspace();
    fs::write(prompts_dir.join("alpha.md"), "---\n---\nbody\n").unwrap();
    fs::write(prompts_dir.join("beta.md"), "---\n---\nbody\n").unwrap();

    let cmd = compose_command(&fixture, "nomatch");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");
    let transcript = drain_available(&mut session, Duration::from_secs(5));

    let status = session
        .get_process()
        .wait()
        .expect("wait for claudine compose to exit");
    let failed = !matches!(status, WaitStatus::Exited(_, 0));
    assert!(failed, "no-match autocomplete must exit non-zero");

    let plain = strip_ansi(&transcript).replace('\r', "");
    assert!(
        plain.contains("No files matched autocomplete query"),
        "expected no-match message; transcript:\n{plain}"
    );
    assert!(
        plain.contains("nomatch"),
        "expected query name in no-match error; transcript:\n{plain}"
    );

    let marker = fixture.cwd().join("provider-launched.flag");
    assert!(
        !marker.exists(),
        "provider must not launch when autocomplete finds no matches"
    );
    assert!(
        !plain.contains("PROVIDER_WAS_LAUNCHED"),
        "provider stdout must not appear; transcript:\n{plain}"
    );
}

#[test]
fn level1_pty_compose_autocomplete_over_cap_errors() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, prompts_dir) = stage_workspace();
    for i in 0..MAX_CANDIDATES + 1 {
        let name = format!("match{:03}.md", i + 1);
        fs::write(prompts_dir.join(&name), "---\n---\nbody\n").unwrap();
    }

    let cmd = compose_command(&fixture, "match");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");
    let transcript = drain_available(&mut session, Duration::from_secs(10));

    let status = session
        .get_process()
        .wait()
        .expect("wait for claudine compose to exit");
    let failed = !matches!(status, WaitStatus::Exited(_, 0));
    assert!(failed, "over-cap autocomplete must exit non-zero");

    let plain = strip_ansi(&transcript).replace('\r', "");
    assert!(
        plain.contains("500"),
        "expected cap value in over-cap error; transcript:\n{plain}"
    );
    assert!(
        plain.contains("matched autocomplete query"),
        "expected over-cap message body; transcript:\n{plain}"
    );
    assert!(
        plain.contains("match"),
        "expected query name in over-cap error; transcript:\n{plain}"
    );
    assert!(
        plain.contains("narrow") || plain.contains("Type more characters"),
        "expected narrowing hint in over-cap error; transcript:\n{plain}"
    );

    let marker = fixture.cwd().join("provider-launched.flag");
    assert!(
        !marker.exists(),
        "provider must not launch when autocomplete exceeds the candidate cap"
    );
    assert!(
        !plain.contains("PROVIDER_WAS_LAUNCHED"),
        "provider stdout must not appear; transcript:\n{plain}"
    );
}
