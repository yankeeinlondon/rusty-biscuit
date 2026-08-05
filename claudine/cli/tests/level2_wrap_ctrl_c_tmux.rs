//! Level 2 multiplexer Ctrl+C tests for the wrapper.
//!
//! ## What level this is (and is not)
//!
//! These tests inject Ctrl+C with `tmux send-keys C-c`: tmux writes the ETX
//! (0x03) byte into the pane, and the pane's line discipline turns it into
//! `SIGINT` for the foreground process group. The terminal emulator's own
//! input encoder never participates — this is **L2 multiplexer /
//! terminal-CLI byte injection, NOT OS keyboard injection**. It proves that a
//! Ctrl+C delivered through a real pane's line discipline terminates the
//! wrapped child, which is one rung below a synthesized OS keystroke.
//!
//! For genuine OS keyboard injection — a real Quartz Ctrl+C chord that
//! WezTerm's input encoder must translate, the level the review rubric
//! demands for "when the user presses Ctrl+C" claims — see
//! `level3_wrap_ctrl_c.rs` (cliclick + a real WezTerm window). For the
//! lowest tier, the prep-phase process-signal guard, see `wrap_sigint.rs`
//! (`libc::kill(pid, SIGINT)`).
//!
//! ## Hardest case: Ctrl+C with a wall-clock `timeout` configured
//!
//! `level2_ctrl_c_terminates_wrapped_child_with_timeout_configured` covers the
//! original silent-failure path: opting into `timeout` used to route the
//! capture/direct spawn paths through a no-signal wait helper that *disabled*
//! Ctrl+C. With the unified signal-aware wait loop, Ctrl+C must terminate the
//! child even when `CLAUDINE_TIMEOUT` is set. The wall-clock budget is set far
//! higher than the interrupt window so a pass proves the keystroke — not the
//! watchdog — did the killing.
//!
//! ## Skip-clean
//!
//! `TmuxHarness::available()` is checked via `require_level!(Level::L2, ...)`,
//! which skips when tmux is absent. `BISCUIT_TEST_LEVEL_REQUIRED=2` flips a
//! missing backend into a hard failure. Run via `just test-l2`.

#![cfg(unix)]

mod common;
use common::{augmented_path, write_executable};

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name};
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Backend, Level, require_level};

/// A fake `opencode` provider that emits one init event then runs forever.
///
/// The infinite loop is the deterministic long-running child the test wraps and
/// interrupts. `models` returns the catalog so the wrapper's model-validation
/// refresh resolves without a network call. The child writes a readiness marker
/// once it reaches its blocking loop so the test can poll for "child is running"
/// rather than racing a fixed sleep. The marker is the synchronization barrier:
/// it proves the wrapper has spawned the agent child and the per-child SIGINT
/// handler is installed before the keystroke is injected.
///
/// Per the "L2 probe must not be a production bin" memory note, the
/// long-running fixture is a shell script on `PATH` (the established `wrap_*`
/// pattern), never a `src/bin` target re-exec.
fn write_long_running_opencode(path: &Path) {
    write_executable(
        path,
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"l2-ctrl-c","model":"test-model"}'
: > "$CLAUDINE_READY_MARKER"
while :; do /bin/sleep 1; done
"#,
    );
}

/// Drive a `claudine compose --opencode <md>` run inside a real tmux pane,
/// inject a `C-c` keystroke through tmux's translator once the wrapped child is
/// running, and return whether the pane returned to a shell prompt within
/// `deadline`.
///
/// `extra_env` is appended to the command line so callers can configure a
/// wall-clock `timeout` (the hardest case). Returns `(returned_to_prompt,
/// captured_plain)` so the caller can both assert termination and inspect the
/// visible interrupt feedback.
fn ctrl_c_terminates(extra_env: &[(&str, &str)], deadline: Duration) -> (bool, String) {
    static SEQ: AtomicU32 = AtomicU32::new(0);

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    common::wrap::seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("run.md");
    fs::write(&md_file, "---\ntitle: l2\nmodel: test-model\n---\nBody\n").unwrap();

    let ready_marker = workspace.path().join("opencode-started");
    write_long_running_opencode(&path_dir.join("opencode"));

    let session = format!(
        "biscuit_l2_ctrlc_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    // POSIX shell (bash/sh), not the developer's `$SHELL`: a custom login
    // prompt (e.g. Starship's `❯`) never ends in `$`/`#`/`%`, so
    // `wait_for_prompt` would never match and burn its full timeout.
    let shell = biscuit_test_harness::detect_shell();
    // A wide-but-short pane: a unique sentinel below makes the prompt-return
    // check robust without needing scrollback.
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
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let claudine = common::claudine_bin();
    // Build one command line: env prefixes, the wrapper invocation, then a
    // chained marker echo. When (and only when) claudine exits and the pane
    // returns to the shell, the sentinel is echoed — the unambiguous
    // "child terminated, control returned" signal in a no-scrollback capture.
    let sentinel = format!("L2_DONE_{}", SEQ.load(Ordering::Relaxed));
    let mut env_pairs: Vec<(&str, String)> = vec![
        ("NO_COLOR", "1".to_string()),
        ("HOME", workspace.path().display().to_string()),
        (
            "PATH",
            augmented_path(&path_dir).to_string_lossy().into_owned(),
        ),
        ("OPENCODE_MODEL", "test-model".to_string()),
        ("CLAUDINE_READY_MARKER", ready_marker.display().to_string()),
    ];
    for (k, v) in extra_env {
        env_pairs.push((k, v.to_string()));
    }
    let env_prefix: String = env_pairs
        .iter()
        .map(|(k, v)| format!("{k}='{}' ", v.replace('\'', "'\\''")))
        .collect();
    let cmd = format!(
        "{env_prefix}{claudine} compose --opencode {md} ; echo {sentinel}",
        md = md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send wrapper command");

    // Poll for the readiness marker: it proves the wrapped child is in its
    // blocking loop, which is strictly after the wrapper installed the
    // per-child SIGINT handler. Injecting C-c before that would race the
    // handler and is the flakiness trap the marker closes.
    let marker_deadline = Instant::now() + Duration::from_secs(30);
    while !ready_marker.exists() {
        if Instant::now() >= marker_deadline {
            kill_session_by_name(&session);
            panic!("wrapped child never reached its run loop within 30s");
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // L2 multiplexer injection: tmux translates `C-c` to ETX, which the pane's
    // line discipline turns into SIGINT for the foreground process group. The
    // terminal emulator's input encoder is not exercised — that is the L3
    // distinction (see level3_wrap_ctrl_c.rs).
    harness.send_key("C-c").expect("inject Ctrl+C keystroke");

    // Poll the captured pane for the sentinel. Its appearance means claudine
    // exited and the shell ran the chained echo — i.e. Ctrl+C terminated the
    // wrapped child and returned control.
    let term_deadline = Instant::now() + deadline;
    let mut last_plain = String::new();
    let mut returned = false;
    while Instant::now() < term_deadline {
        if let Ok(frame) = harness.capture() {
            last_plain = frame.plain;
            if last_plain.contains(&sentinel) {
                returned = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    kill_session_by_name(&session);
    (returned, last_plain)
}

/// Baseline: a tmux `C-c` keystroke terminates the wrapped child on the
/// default (no `timeout`) spawn path and returns control to the shell.
#[test]
#[serial(level2_tmux_ctrlc)]
fn level2_ctrl_c_terminates_wrapped_child() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let (returned, plain) = ctrl_c_terminates(&[], Duration::from_secs(15));
    assert!(
        returned,
        "Ctrl+C keystroke must terminate the wrapped child and return to the \
         shell prompt.\npane:\n{plain}",
    );
}

/// Hardest case (spec.md:623, 684-686): Ctrl+C must terminate the wrapped child
/// even when a wall-clock `timeout` is configured. The budget (`120s`) is far
/// larger than the ~15s interrupt window, so a pass proves the keystroke — not
/// the timeout watchdog — did the killing. This is the original silent-failure
/// path: a configured `timeout` once routed the spawn through a no-signal wait
/// helper that disabled Ctrl+C.
#[test]
#[serial(level2_tmux_ctrlc)]
fn level2_ctrl_c_terminates_wrapped_child_with_timeout_configured() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let (returned, plain) = ctrl_c_terminates(
        &[
            ("CLAUDINE_TIMEOUT", "120s"),
            // Disable stream-silence so only the keystroke (or, on regression,
            // the 120s wall-clock) can stop the child — well beyond our window.
            ("CLAUDINE_STEP_TIMEOUT", "0s"),
            ("CLAUDINE_WATCHDOG_INTERVAL", "1s"),
            ("CLAUDINE_KILL_GRACE", "1s"),
        ],
        Duration::from_secs(15),
    );
    assert!(
        returned,
        "Ctrl+C keystroke must terminate the wrapped child within the interrupt \
         window even with a 120s wall-clock timeout configured (proving the \
         keystroke, not the watchdog, killed it).\npane:\n{plain}",
    );
}
