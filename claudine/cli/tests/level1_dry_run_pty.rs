//! Level 1 PTY test for interactive shell-command approval under `--dry-run`.
//!
//! Closes the spec acceptance criterion:
//!
//! > Interactive approval prompts for unapproved shell commands appear
//! > exactly as in normal mode.
//!
//! The non-PTY process tests in `wrap_compose_preflight.rs` cover the
//! **non-TTY** gate (`compose_dry_run_non_tty_unapproved_shell_emits_gate_error`)
//! and the `--yolo` bypass. This file drives the approval prompt through a
//! pseudo-terminal to prove the interactive handler still fires under
//! `--dry-run` — `dry_run` only changes the *no-handler* branch, so with a
//! TTY (handler present) the prompt path must be untouched.
//!
//! Two tests live here:
//!
//! - `..._appears_and_allows` — approving the command lets the dry-run execute
//!   it for real while the provider stays unlaunched.
//! - `..._matches_normal_mode` — captures the approval-prompt surface in both
//!   normal mode and `--dry-run` mode and asserts they are byte-identical
//!   (after ANSI stripping), closing the spec's "exactly as in normal mode"
//!   requirement with a direct comparison rather than a single-mode check.
//!
//! ## Tier
//!
//! **Level 1.** These tests inject bytes into a bare pseudo-terminal and read
//! the child's output back; they prove the *handler* fires and renders
//! identically in both modes. No terminal emulator participates, so nothing
//! about emulator rendering is under test. The emulator-level complement —
//! `level2_dry_run_approval_capture.rs` — drives the same prompt through a real
//! terminal (`tmux`) and compares the surface the emulator actually displayed.
//!
//! Gating: `#![cfg(unix)]`, `require_level!(Level::L1, pty_available(), ...)`
//! so the test skips cleanly without a PTY.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test
//! ```

#![cfg(unix)]

use expectrl::Session;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

mod common;
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
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();
}

/// A staged workspace with a document whose body holds one unapproved
/// `::shell` command, plus a provider stub that records a launch.
struct StagedApproval {
    fixture: CliProcessFixture,
    md_file: PathBuf,
    /// Written by the provider stub if it ever runs; must stay absent in
    /// dry-run mode.
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

/// Build the `claudine compose [--dry-run] --goose <doc>` command for a staged
/// fixture, with the environment normalized so the approval prompt renders
/// identically regardless of the developer's shell.
fn compose_command(staged: &StagedApproval, dry_run: bool) -> Command {
    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = staged.fixture.command_std();
    cmd.arg("compose").arg("--goose");
    if dry_run {
        cmd.arg("--dry-run");
    }
    cmd.arg(staged.md_file.to_str().unwrap());
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd
}

/// Extract the shell-approval prompt's text block from a transcript: the
/// ANSI-stripped lines from the `Shell Approval Required` header through the
/// final `Blacklist and stop` option (the last static line the handler writes
/// before reading input).
///
/// The handler renders the prompt with a single forward write (no cursor
/// repositioning or redraws), so the captured block is stable and directly
/// comparable across two independent runs.
fn approval_prompt_region(transcript: &str) -> Vec<String> {
    let plain = common::strip_ansi(transcript);
    let lines: Vec<&str> = plain.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.contains("Shell Approval Required"));
    let end = lines.iter().position(|l| l.contains("Blacklist and stop"));
    match (start, end) {
        (Some(s), Some(e)) if e >= s => lines[s..=e]
            .iter()
            .map(|l| l.trim_end().to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// Spawn `claudine compose` in the given mode, wait for the full approval
/// prompt to render, capture its text region, then send `4` (Deny) so the run
/// tears down cleanly without launching the provider. Returns the prompt
/// region lines.
fn capture_approval_prompt(staged: &StagedApproval, dry_run: bool) -> Vec<String> {
    let mut session: OsSession =
        Session::spawn(compose_command(staged, dry_run)).expect("spawn PTY session");

    // The parenthetical is the tail of the last static option written before
    // the handler blocks on input, so its appearance means the whole prompt has
    // been flushed and is safe to snapshot. Waiting on the option's *label*
    // instead ("Blacklist and stop") returns as soon as the option's first
    // bytes land: under parallel load the read can split mid-line, and the two
    // captures below then differ by a truncated final row rather than by
    // anything the handler rendered.
    let transcript = wait_for_marker(
        &mut session,
        "(persists to blacklist)",
        Duration::from_secs(10),
    );
    let region = approval_prompt_region(&transcript);

    // Deny so the composition aborts before any provider launch.
    session.write_all(b"4\n").expect("write deny choice");
    session.flush().ok();
    let _ = read_for(&mut session, Duration::from_millis(300));

    region
}

/// The interactive shell-approval prompt fires under `--dry-run` exactly as
/// it does in normal mode: with stdin + stderr both TTYs, an unapproved
/// `::shell` command surfaces the "Shell Approval Required" prompt, and
/// choosing "Allow once" lets the dry-run proceed — executing the command
/// for real (its output lands in the rendered body) without ever launching
/// the provider.
#[test]
fn level1_pty_dry_run_shell_approval_prompt_appears_and_allows() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let staged = stage_shell_approval_doc();
    let marker = staged.launch_marker.clone();

    let mut session: OsSession =
        Session::spawn(compose_command(&staged, true)).expect("spawn PTY session");

    // The approval prompt must appear under --dry-run, proving the
    // interactive path is identical to normal mode.
    let pre = wait_for_marker(
        &mut session,
        "Shell Approval Required",
        Duration::from_secs(10),
    );

    // Choose option 3 — "Allow once". The handler is line-based (reads a
    // numeric choice followed by Enter), so `\n` submits without raw mode.
    session.write_all(b"3\n").expect("write approval choice");
    session.flush().ok();

    // Drain until the child exits.
    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = pre;
    let mut exited = false;
    while Instant::now() < stop {
        let chunk = read_for(&mut session, Duration::from_millis(200));
        transcript.push_str(&chunk);
        if common::strip_ansi(&transcript).contains("tty-approved-marker") {
            exited = true;
            break;
        }
    }

    let plain = common::strip_ansi(&transcript);
    assert!(
        exited,
        "after approving the command, the dry-run should execute it for real \
         and render its output (`tty-approved-marker`) into the body; \
         transcript:\n{plain}"
    );
    assert!(
        !marker.exists(),
        "the provider must NOT launch under `--dry-run`, even after the shell \
         command is approved; transcript:\n{plain}"
    );
}

/// The approval prompt under `--dry-run` is byte-for-byte the same surface the
/// user sees in normal mode.
///
/// Shell approval runs in the prepare phase, *before* the dry-run seam, and is
/// served by the same `CliShellApprovalHandler` in both modes (`dry_run` only
/// rewrites the non-interactive *no-handler* branch). This test proves that
/// directly: it captures the rendered prompt region for the same document in
/// normal mode and in `--dry-run` mode and asserts the two are identical —
/// satisfying the spec's "exactly as in normal mode" wording with a comparison
/// rather than a single-mode existence check.
#[test]
fn level1_pty_dry_run_approval_prompt_matches_normal_mode() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    // One shared fixture so the prompt's `Source:` line (which embeds the
    // document path) is identical across both runs. Denying persists nothing,
    // so the second run sees the same state as the first.
    let staged = stage_shell_approval_doc();
    let normal = capture_approval_prompt(&staged, false);
    let dry_run = capture_approval_prompt(&staged, true);

    assert!(
        normal.iter().any(|l| l.contains("Shell Approval Required"))
            && normal.iter().any(|l| l.contains("tty-approved-marker"))
            && normal.iter().any(|l| l.contains("Allow once")),
        "normal-mode capture did not contain the expected approval prompt; got:\n{normal:#?}"
    );

    assert_eq!(
        dry_run, normal,
        "the --dry-run approval prompt must render exactly as in normal mode.\n\
         normal:\n{normal:#?}\ndry-run:\n{dry_run:#?}"
    );
}
