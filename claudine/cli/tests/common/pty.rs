//! Shared PTY harness for the Level 2 schema-prompt and sequence-overlay
//! interactive tests.
//!
//! The `level2_schema_prompt_pty.rs` god file was split into a schema /
//! inline-compose binary and a `level2_sequence_overlay_pty.rs` binary.
//! The draining loop (`read_for`), marker waiters (`wait_for_marker`,
//! `wait_for_raw_mode`), and the config / goose-stub stagers are shared by
//! both, so they live here **verbatim**. Gated `#[cfg(unix)]` at the
//! `mod pty;` site because `expectrl::session::OsSession` is Unix-only.

#![allow(dead_code)]

use super::{strip_ansi, write_executable};
use expectrl::session::OsSession;
use std::fs;
use std::time::{Duration, Instant};

/// Drain bytes from the PTY for a generous window so each `try_read`
/// call collects whatever the child has flushed so far. Returns the
/// concatenated transcript bytes as `String::from_utf8_lossy` so stray
/// ANSI fragments never panic.
pub(crate) fn read_for(session: &mut OsSession, total_deadline: Duration) -> String {
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    let deadline = Instant::now() + total_deadline;
    session.set_expect_timeout(Some(Duration::from_millis(150)));
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

/// Wait for a substring (after ANSI stripping) to appear in the cumulative
/// transcript, draining the PTY incrementally. Returns the full ANSI-bearing
/// transcript on success; panics with the accumulated transcript on
/// timeout for easier diagnosis.
pub(crate) fn wait_for_marker(session: &mut OsSession, marker: &str, deadline: Duration) -> String {
    let stop = Instant::now() + deadline;
    let mut transcript = String::new();
    let mut scratch = [0u8; 4096];
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    while Instant::now() < stop {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                transcript.push_str(&String::from_utf8_lossy(&scratch[..n]));
                if strip_ansi(&transcript).contains(marker) {
                    return transcript;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    panic!(
        "marker {marker:?} did not appear within {deadline:?}; transcript:\n{transcript}"
    );
}

/// Alternate-screen enter sequence crossterm emits via `EnterAlternateScreen`.
pub(crate) const ALT_SCREEN_ENTER: &str = "\x1b[?1049h";

/// Block until the interactive widget has switched the PTY into raw mode,
/// then return the accumulated raw transcript.
///
/// The prompt's property name renders in the pre-prompt status report —
/// which `run_standalone` prints to stderr *before* it calls
/// `enable_raw_mode()`. Sending a keystroke as soon as that name appears
/// races the line discipline: until raw mode disables `ICRNL`, a carriage
/// return (`\r`, the byte that means Enter) is rewritten to a line feed
/// (`\n`), which crossterm parses as `Ctrl+J` — not `Enter` — so the
/// prompt never submits and the test hangs to its deadline.
///
/// `prepare_terminal` enables raw mode and *then* emits
/// [`ALT_SCREEN_ENTER`], so observing that sequence in the transcript
/// proves raw mode is already active and any `\r` we write next survives
/// as a carriage return. Gate every keystroke on this marker.
///
/// `seed` is the transcript already drained by the preceding
/// [`wait_for_marker`] call — the sequence may have arrived in the same
/// read as the status report.
pub(crate) fn wait_for_raw_mode(session: &mut OsSession, seed: String, deadline: Duration) -> String {
    if seed.contains(ALT_SCREEN_ENTER) {
        return seed;
    }
    let stop = Instant::now() + deadline;
    let mut transcript = seed;
    let mut scratch = [0u8; 4096];
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    while Instant::now() < stop {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                transcript.push_str(&String::from_utf8_lossy(&scratch[..n]));
                if transcript.contains(ALT_SCREEN_ENTER) {
                    return transcript;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    panic!(
        "raw-mode marker {ALT_SCREEN_ENTER:?} did not appear within {deadline:?}; \
         transcript:\n{transcript}"
    );
}

/// Pre-stage a minimal claudine config at `$HOME/.claudine/config.json`.
///
/// Under a PTY, claudine detects an interactive stdin and runs the
/// first-run setup wizard when no user config exists. The wizard would
/// intercept any input we send to the schema prompt and block the test
/// indefinitely. Writing an empty JSON object satisfies the loader (every
/// `ClaudineConfig` field has a `#[serde(default)]` derivation) and
/// inherits the defaults — notably `prompt_for_missing = true`.
pub(crate) fn stage_default_config(home_dir: &std::path::Path) {
    let claudine_dir = home_dir.join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();
}

/// Stage a stub `goose` binary that records that the provider was
/// launched (so the test can prove the interactive prompt successfully
/// satisfied the schema and execution proceeded).
///
/// The stub writes the marker file FIRST and then exits without reading
/// stdin. Under PTY, the wrapper's stdin remains attached to the master
/// side, so a `cat > /dev/null` stub would block on EOF that never
/// arrives. Writing the marker first lets the test observe the launch
/// even if the wrapper still has stdin open.
pub(crate) fn stage_goose_stub(bin_dir: &std::path::Path, marker_file: &std::path::Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho 'launched' > {marker}\nexit 0\n",
            marker = marker_file.display()
        ),
    );
}
