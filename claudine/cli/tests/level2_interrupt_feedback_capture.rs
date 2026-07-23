//! Level 2 real-terminal capture test for the visible per-interrupt feedback.
//!
//! The interrupt-feedback strings live in
//! `claudine/cli/src/commands/wrap/exec/termination.rs`
//! (`INTERRUPT_FEEDBACK_FIRST` / `INTERRUPT_FEEDBACK_REPEAT`). Unit tests prove
//! the *mapping* (`escalation_signal`), but nothing proved the line actually
//! **renders** during a real terminal run. This closes that gap: it drives the
//! real `claudine` wrapper inside a tmux pane, raises a `SIGINT` against the
//! running wrapper, and asserts the acknowledgement appears in the bytes the
//! terminal displayed (`frame.raw`).
//!
//! ## Level: L2 (rendering), not L3 (input encoder)
//!
//! This test verifies what the terminal *displayed*, not what bytes a keypress
//! *encodes* — so it is L2. It delivers `SIGINT` directly to the wrapper PID
//! (which the fake provider records) rather than injecting a keystroke; the
//! genuine-keystroke path is the separate `level3_wrap_ctrl_c.rs` suite. Both
//! reach the same per-child SIGINT handler that emits the feedback line.
//!
//! ## Semantic substring (comment-quality rule #4)
//!
//! The assertion targets the stable substring `interrupt received` rather than
//! the exact glyph (`⚠`) or em-dash punctuation, which are presentation detail
//! that may drift. `FORCE_COLOR=1` routes claudine through an optimistic
//! terminal so the capture reflects production styling. The match runs over
//! the capture with newlines removed: the feedback is written asynchronously
//! from the SIGINT handler, so it can land mid-line and be hard-wrapped by
//! the pane at any column — including inside the asserted substring.
//!
//! ## Pane height (no scrollback)
//!
//! `capture-pane` returns only the visible pane. The wrapper emits little
//! output before the interrupt, so a 50-row pane keeps the feedback line on
//! screen (per the "Harness capture has no scrollback" caveat).
//!
//! Skip-clean: `TmuxHarness::available()` is checked first. Run via
//! `just test-l2`.

#![cfg(unix)]

mod common;
use common::{augmented_path, clear_no_color, write_executable};

use biscuit_test_harness::tmux::{kill_session_by_name, TmuxHarness};
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{require_level, Level};

/// Drive a wrapped, long-running child in a tmux pane, deliver one `SIGINT` to
/// the wrapper once it is running, and assert the visible interrupt feedback
/// rendered. The wrapper runs interactively (a real pane), so its escalation
/// ladder keeps the graceful first rung — exactly the press the feedback line
/// acknowledges.
#[test]
#[serial(level2_terminal)]
fn level2_interrupt_feedback_renders_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    static SEQ: AtomicU32 = AtomicU32::new(0);

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    common::wrap::seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("run.md");
    fs::write(&md_file, "---\ntitle: l2-fb\nmodel: test-model\n---\nBody\n").unwrap();

    // Fake provider records the claudine wrapper PID (its own parent) into a
    // file once it reaches its run loop, then blocks forever. The parent PID is
    // the wrapper process whose SIGINT handler emits the feedback line. Reading
    // the marker also proves the per-child handler is installed before the
    // signal is sent (closing the same race the L3 suite guards against).
    let pid_marker = workspace.path().join("claudine-pid");
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"l2-fb","model":"test-model"}'
printf '%s\n' "$PPID" > "$CLAUDINE_PID_MARKER"
while :; do /bin/sleep 1; done
"#,
    );

    let session = format!(
        "biscuit_l2_fb_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    // POSIX shell (bash/sh), not the developer's `$SHELL`: a custom login
    // prompt (e.g. Starship's `❯`) never ends in `$`/`#`/`%`, so
    // `wait_for_prompt` would never match and burn its full timeout.
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "120",
            "-y",
            "50",
            &format!("{shell} -l"),
        ])
        .env("FORCE_COLOR", "1")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);

    let claudine = env!("CARGO_BIN_EXE_claudine");
    let env_pairs: [(&str, String); 6] = [
        ("FORCE_COLOR", "1".to_string()),
        ("HOME", workspace.path().display().to_string()),
        ("PATH", augmented_path(&path_dir).to_string_lossy().into_owned()),
        ("OPENCODE_MODEL", "test-model".to_string()),
        ("CLAUDINE_PID_MARKER", pid_marker.display().to_string()),
        // Keep the wrapper from racing a watchdog: leave timeouts generous so
        // only our SIGINT drives termination during the window.
        ("CLAUDINE_STEP_TIMEOUT", "0s".to_string()),
    ];
    let env_prefix: String = env_pairs
        .iter()
        .map(|(k, v)| format!("{k}='{}' ", v.replace('\'', "'\\''")))
        .collect();
    let cmd = format!(
        "{env_prefix}{claudine} compose --opencode {md}",
        md = md_file.display(),
    );
    harness.send_command_with_env(&cmd, &[]).expect("send wrapper command");

    // Wait until the wrapper PID is recorded — the child is in its run loop and
    // the per-child SIGINT handler is installed.
    let marker_deadline = Instant::now() + Duration::from_secs(30);
    while !pid_marker.exists() {
        if Instant::now() >= marker_deadline {
            kill_session_by_name(&session);
            panic!("wrapped child never recorded the wrapper PID within 30s");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let wrapper_pid: i32 = fs::read_to_string(&pid_marker)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or_else(|| {
            kill_session_by_name(&session);
            panic!("could not parse wrapper PID from marker");
        });

    // One SIGINT to the wrapper: its handler emits the visible feedback line to
    // stderr, which the pane renders.
    unsafe {
        libc::kill(wrapper_pid, libc::SIGINT);
    }

    // Poll the captured frame for the rendered feedback.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last_raw = String::new();
    let mut rendered = false;
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            last_raw = frame.raw;
            // Semantic substring: stable across glyph/punctuation drift.
            // Joined across pane hard-wraps (see module doc).
            if last_raw.replace('\n', "").contains("interrupt received") {
                rendered = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    kill_session_by_name(&session);
    assert!(
        rendered,
        "the per-interrupt feedback line (substring `interrupt received`) must \
         render in the real terminal after a SIGINT to the wrapper.\nraw:\n{last_raw}",
    );
}
