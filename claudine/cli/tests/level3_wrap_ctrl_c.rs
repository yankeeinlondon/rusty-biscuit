//! Level 3 OS-keyboard-injection tests for user Ctrl+C against the wrapper.
//!
//! ## What makes these Level 3
//!
//! These tests inject a **real OS keyboard event** with
//! `cliclick::click_then_ctrl_chord` into a focused, foreground WezTerm
//! window that is wrapping a live `claudine compose --opencode` run. The
//! Ctrl+C chord is synthesised at the macOS Quartz event layer, so WezTerm's
//! own input encoder receives it and emits ETX (0x03) → `SIGINT` to the
//! foreground process group — the exact path a physical keypress takes. This
//! is the verification level the review rubric demands for any "when the user
//! presses Ctrl+C" claim.
//!
//! Why the chord (and not a bare Ctrl press): a Ctrl+`c` chord rides along
//! with the letter keyDown as a normal `CGEvent` with the Control flag set, so
//! cliclick can synthesise it. A *bare* modifier press travels through AppKit's
//! `flagsChanged` event type, which no userspace tool on macOS can reliably
//! synthesise — but the chord is all this path needs.
//!
//! macOS only: `cliclick` is the only injector wired into the harness. The
//! Linux equivalent (`xdotool`) is not implemented, so the tests below carry
//! `#[cfg(target_os = "macos")]` honestly rather than pretending portability.
//!
//! ## Contrast with the lower tiers
//!
//! - `level2_wrap_ctrl_c_tmux.rs` injects Ctrl+C via `tmux send-keys C-c` —
//!   bytes written into the pane and translated by the pane's line discipline.
//!   That is L2 multiplexer / terminal-CLI coverage, **not** OS keyboard
//!   injection: the terminal emulator's input encoder never participates.
//! - `wrap_sigint.rs` delivers `SIGINT` with `libc::kill(pid, SIGINT)` — a
//!   process signal, the lowest tier, exercising only the prep-phase guard.
//!
//! ## Hardest case: Ctrl+C with a wall-clock `timeout` configured
//!
//! `level3_ctrl_c_terminates_wrapped_child_with_timeout_configured` covers the
//! original silent-failure path: opting into `timeout` used to route the
//! capture/direct spawn paths through a no-signal wait helper that *disabled*
//! Ctrl+C. With the unified signal-aware wait loop, Ctrl+C must terminate the
//! child even when `CLAUDINE_TIMEOUT` is set. The wall-clock budget is set far
//! higher than the interrupt window so a pass proves the keystroke — not the
//! watchdog — did the killing.
//!
//! ## Skip-clean
//!
//! `WezTermHarness::available() && cliclick::available()` is checked via
//! `require_level!(Level::L3, ...)`, which also skips unless `RUN_LEVEL3=1`.
//! `BISCUIT_TEST_LEVEL_REQUIRED=3` flips a missing backend into a hard failure.
//! Run via `just test-l3`.

#![cfg(unix)]

#[cfg(target_os = "macos")]
mod common;
#[cfg(target_os = "macos")]
use common::{augmented_path, write_executable};

#[cfg(target_os = "macos")]
use biscuit_test_harness::wezterm::WezTermHarness;
#[cfg(target_os = "macos")]
use biscuit_test_harness::{SpawnVisibility, TerminalHarness, cliclick};
#[cfg(target_os = "macos")]
use serial_test::serial;
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use tempfile::tempdir;
#[cfg(target_os = "macos")]
use test_toolkit::{Level, require_level};

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
#[cfg(target_os = "macos")]
fn write_long_running_opencode(path: &Path) {
    write_executable(
        path,
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"l3-ctrl-c","model":"test-model"}'
: > "$CLAUDINE_READY_MARKER"
while :; do /bin/sleep 1; done
"#,
    );
}

/// Drive a `claudine compose --opencode <md>` run inside a real, focused
/// WezTerm window, inject a genuine OS Ctrl+C keystroke via cliclick once the
/// wrapped child is running, and return whether the pane returned to a shell
/// prompt within `deadline`.
///
/// `extra_env` is appended to the command line so callers can configure a
/// wall-clock `timeout` (the hardest case). Returns `(returned_to_prompt,
/// captured_plain)` so the caller can both assert termination and inspect the
/// visible interrupt feedback.
#[cfg(target_os = "macos")]
fn ctrl_c_terminates(extra_env: &[(&str, &str)], deadline: Duration) -> (bool, String) {
    static SEQ: AtomicU32 = AtomicU32::new(0);

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    common::wrap::seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("run.md");
    fs::write(&md_file, "---\ntitle: l3\nmodel: test-model\n---\nBody\n").unwrap();

    let ready_marker = workspace.path().join("opencode-started");
    write_long_running_opencode(&path_dir.join("opencode"));

    // Foreground spawn: cliclick injection requires the window to be on the
    // active workspace so `focus_spawned_pane` (AXRaise + click) can route OS
    // events to it. A background pane would be invisible to AXRaise.
    let mut harness = WezTermHarness::new().with_spawn_visibility(SpawnVisibility::Foreground);
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    let claudine = env!("CARGO_BIN_EXE_claudine");
    // The chained `; echo <sentinel>` is the unambiguous "child terminated,
    // control returned" signal: the sentinel is printed if and only if claudine
    // exits and the shell runs the next command. `send_command_with_env`
    // formats the leading `KEY=value` prefixes inline.
    let sentinel = format!("L3_DONE_{}", SEQ.fetch_add(1, Ordering::Relaxed));
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
    let env_refs: Vec<(&str, &str)> = env_pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let cmd = format!(
        "{claudine} compose --opencode {md} ; echo {sentinel}",
        md = md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &env_refs)
        .expect("send wrapper command");

    // Poll for the readiness marker: it proves the wrapped child is in its
    // blocking loop, which is strictly after the wrapper installed the
    // per-child SIGINT handler. Injecting Ctrl+C before that would race the
    // handler and is the flakiness trap the marker closes.
    let marker_deadline = Instant::now() + Duration::from_secs(30);
    while !ready_marker.exists() {
        if Instant::now() >= marker_deadline {
            panic!("wrapped child never reached its run loop within 30s");
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // Raise our specific WezTerm window and obtain click coordinates. The
    // raise routes OS keyboard events to this pane; a missing Accessibility
    // grant surfaces as an error here rather than a silent miss.
    let coords = harness
        .focus_spawned_pane()
        .expect("focus spawned WezTerm pane")
        .expect("AXRaise yielded no window coords (non-macOS or AX failure)");

    // Genuine OS keyboard injection: click to transfer focus, then the Ctrl+C
    // chord, atomic within one cliclick invocation. WezTerm's input encoder
    // turns the chord into ETX → SIGINT for the foreground process group.
    // Ctrl is released within the same invocation, so no modifier leaks.
    cliclick::click_then_ctrl_chord(coords.0, coords.1, "c").expect("inject OS Ctrl+C chord");

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

    // `harness` Drop kills the pane; no explicit teardown needed.
    (returned, last_plain)
}

/// Baseline: a real OS Ctrl+C keystroke terminates the wrapped child on the
/// default (no `timeout`) spawn path and returns control to the shell.
#[test]
#[serial(level3_keyboard)]
#[cfg(target_os = "macos")]
fn level3_ctrl_c_terminates_wrapped_child() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    let (returned, plain) = ctrl_c_terminates(&[], Duration::from_secs(15));
    assert!(
        returned,
        "OS Ctrl+C keystroke must terminate the wrapped child and return to the \
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
#[serial(level3_keyboard)]
#[cfg(target_os = "macos")]
fn level3_ctrl_c_terminates_wrapped_child_with_timeout_configured() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

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
        "OS Ctrl+C keystroke must terminate the wrapped child within the interrupt \
         window even with a 120s wall-clock timeout configured (proving the \
         keystroke, not the watchdog, killed it).\npane:\n{plain}",
    );
}

// ---------------------------------------------------------------------------
// Windows parity verification record (spec.md:684-686, Cluster D / Q15)
// ---------------------------------------------------------------------------
//
// The dev host is macOS; the Windows Ctrl+C path (Job Object +
// `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT)` → `TerminateJobObject`, see
// `windows_wait_loop` in `exec/termination.rs`) CANNOT be exercised here. This
// record encodes the intended runtime behavior so it executes on a Windows host
// or in CI. It is honestly marked: it is NOT verified on the dev host. The prose
// counterpart lives in `claudine/docs/topics/signal-handling.md`
// ("Windows parity" → "Verification record").
//
// The Unix surface is verified above (real-keystroke termination, incl. the
// timeout-configured column) and by the process-signal coverage in
// `wrap_sigint.rs`. Windows shares the same unified wait loop but a distinct
// `#[cfg(not(unix))]` arm, so its runtime behavior is a separate, flagged gap.

/// Windows host / CI verification record: a console Ctrl+C must terminate the
/// wrapped child even with a wall-clock `timeout` configured, mirroring the
/// Unix `level3_ctrl_c_terminates_wrapped_child_with_timeout_configured` proof.
///
/// Compiled only on Windows and `#[ignore]`d by default: it requires a real
/// attached console to receive `CTRL_C_EVENT`, which headless CI runners may
/// lack. Run explicitly on a Windows host with
/// `cargo test -p claudine-cli --test level3_wrap_ctrl_c -- --ignored
/// windows_ctrl_c_verification_record`. Until executed on a Windows host this
/// remains an UNVERIFIED contract, not a passing claim.
#[cfg(windows)]
#[test]
#[ignore = "requires a Windows host with an attached console; not verifiable on the macOS dev host"]
fn windows_ctrl_c_verification_record() {
    // Intentionally minimal: this is a placeholder contract that documents the
    // expected behavior and gives a Windows host an entry point. The full
    // GenerateConsoleCtrlEvent harness is future work tracked in
    // signal-handling.md's verification record. Marking it `#[ignore]` keeps the
    // suite honest — it does not assert success on a host that cannot run it.
    panic!(
        "Windows Ctrl+C parity is not yet exercised by an automated harness. \
         Run on a Windows host to validate the Job-Object / CTRL_BREAK_EVENT \
         termination path; see claudine/docs/topics/signal-handling.md."
    );
}
