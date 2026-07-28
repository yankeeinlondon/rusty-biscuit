//! Level 2 multiplexer test: a **second** Ctrl+C force-exits a compose loop
//! that is wedged in a blocking synchronous call **between iterations**.
//!
//! ## What this covers (the gap the sibling test does not)
//!
//! [`level2_wrap_ctrl_c_tmux.rs`] proves Ctrl+C terminates the wrapped agent
//! child *while a child wait loop is running* — the window where the
//! per-child SIGINT→SIGTERM→SIGKILL ladder is installed. This test covers the
//! complementary, previously-broken window: the moment **between two loop
//! iterations**, where no agent child is running and therefore no wait loop is
//! installed. There the only handler is the compose-scoped guard, which used
//! to merely set the interrupt flag and print a one-shot notice — so a
//! synchronous call wedged with no interrupt checkpoint (a hung TTS
//! subprocess, a stalled messenger send, an unresponsive shell expansion)
//! would ignore Ctrl+C entirely, exactly the lock-up this test reproduces.
//!
//! The fix gives a repeated press teeth: a second SIGINT that lands while
//! `crate::output::wait_loop_active()` is `false` force-exits with
//! `libc::_exit(130)`. This test asserts the wrapper returns control to the
//! shell with **exit code 130** within the interrupt window.
//!
//! ## Why a `::shell` wedge (and not a real hung TTS)
//!
//! The genuinely blocking lifecycle side effects (TTS, sound effect) are now
//! timeout-bounded, so they cannot hang indefinitely — they would mask the
//! force-exit behind their own budget. A `--yolo`-approved `::shell` directive
//! is the controllable, *unbounded* synchronous wedge that runs during each
//! iteration's prep (between iterations 1 and 2 here), outside any wait loop —
//! the same class of blocking call the fix is meant to escape. The wedge
//! script also `trap`s `INT`/`TERM` so the first Ctrl+C delivered to the
//! foreground process group cannot resolve the hang by killing the child:
//! only Claudine's own second-press `_exit(130)` can break the run free,
//! which is precisely the behavior under test.
//!
//! ## Skip-clean
//!
//! `TmuxHarness::available()` is checked via `require_level!`; the test skips
//! when tmux is absent. `BISCUIT_TEST_LEVEL_REQUIRED=2` flips a missing
//! backend into a hard failure. Run via `just test-l2`.

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

/// A fake `opencode` provider for a looping compose: each agent run emits one
/// init event, **arms** the wedge by creating `armed`, then exits `0` so the
/// iteration is classified a success and the loop advances. `models` returns
/// the catalog so model validation resolves without a network call.
///
/// The agent runs *after* prep, so iteration 1's `::shell` wedge (which only
/// fires once `armed` exists) stays inert during iteration 1 and only hangs on
/// iteration 2's prep — landing the wedge strictly *between* iterations.
fn write_arming_opencode(path: &Path, armed: &Path) {
    write_executable(
        path,
        &format!(
            r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{{"type":"init","session_id":"l2-loop","model":"test-model"}}'
: > '{armed}'
exit 0
"#,
            armed = armed.display()
        ),
    );
}

/// The wedge invoked by the prompt's `::shell` directive on every compose
/// pass. It is inert until the agent has armed it (so it hangs only between
/// iterations), then ignores `INT`/`TERM`/`HUP` and loops forever — proving
/// the first Ctrl+C cannot resolve the hang and only the wrapper's
/// second-press `_exit(130)` can.
fn write_wedge_script(path: &Path, armed: &Path, wedged: &Path) {
    write_executable(
        path,
        &format!(
            r#"#!/bin/sh
if [ ! -f '{armed}' ]; then
  exit 0
fi
# Ignore the signals tmux delivers to the foreground process group so the
# FIRST Ctrl+C cannot kill this child and free the run. The respawning sleep
# loop keeps the `::shell` capture blocking until Claudine itself exits.
trap '' INT TERM HUP
: > '{wedged}'
while : ; do /bin/sleep 1; done
"#,
            armed = armed.display(),
            wedged = wedged.display()
        ),
    );
}

/// Drive a looping `claudine compose --opencode <md>` run inside a real tmux
/// pane. Wait until the run is wedged in iteration 2's prep, inject two `C-c`
/// keystrokes, and report whether the pane returned to the shell carrying the
/// expected exit code within `deadline`.
///
/// Returns `(returned_with_code, captured_plain)`.
fn double_ctrl_c_force_exits(deadline: Duration) -> (bool, String) {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    common::wrap::seed_minimal_config(workspace.path());

    let armed = workspace.path().join("wedge-armed");
    let wedged = workspace.path().join("wedge-active");
    let wedge_sh = workspace.path().join("wedge.sh");
    write_arming_opencode(&path_dir.join("opencode"), &armed);
    write_wedge_script(&wedge_sh, &armed, &wedged);

    // A two-phase loop. Iteration 1 succeeds (agent exits 0, arms the wedge);
    // iteration 2's prep runs the now-armed `::shell` wedge and hangs.
    //
    // `::timeout:600` is load-bearing for the *negative* case: `::shell` has
    // its own internal execution timeout (~10s), which — combined with the
    // loop forcing exit 130 once the interrupt flag is set — would otherwise
    // unwedge the run and synthesize a 130 even WITHOUT the force-exit fix,
    // masking it. Pushing the directive timeout far past the test deadline
    // leaves the second-press `_exit(130)` as the only way the run can return
    // within the window, so the test fails if the fix regresses.
    let md_file = workspace.path().join("loop.md");
    fs::write(
        &md_file,
        format!(
             "---\ntitle: l2-loop-wedge\nmodel: test-model\ntotal_phases: 2\nphase: 1\n\
             loop:\n  until: \"phase > total_phases\"\n  action: \"increment(phase)\"\n---\n\
             Loop body.\n\n::shell /bin/sh {wedge} ::timeout:600\n",
            wedge = wedge_sh.display()
        ),
    )
    .unwrap();

    let session = format!("biscuit_l2_loopwedge_{}_{}", std::process::id(), seq);
    // POSIX shell, not `$SHELL`: a custom login prompt would never match
    // `wait_for_prompt` and would burn its full timeout.
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
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let claudine = env!("CARGO_BIN_EXE_claudine");
    // The sentinel carries claudine's exit code (`$?` after the `;`), so a
    // match proves not just "control returned" but "returned with exit 130" —
    // the force-exit code. `;` (not `&&`) runs the echo regardless of code.
    let sentinel = format!("L2WEDGE_{seq}");
    let env_pairs: Vec<(&str, String)> = vec![
        ("NO_COLOR", "1".to_string()),
        ("HOME", workspace.path().display().to_string()),
        (
            "PATH",
            augmented_path(&path_dir).to_string_lossy().into_owned(),
        ),
        ("OPENCODE_MODEL", "test-model".to_string()),
        // Auto-approve the `::shell` wedge without an interactive handler.
        ("CLAUDINE_YOLO", "true".to_string()),
    ];
    let env_prefix: String = env_pairs
        .iter()
        .map(|(k, v)| format!("{k}='{}' ", v.replace('\'', "'\\''")))
        .collect();
    let cmd = format!(
        "{env_prefix}{claudine} compose --opencode {md} ; echo {sentinel}_$?",
        md = md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send wrapper command");

    // Poll for the `wedged` marker: it is created by the wedge script the
    // instant iteration 2's prep enters the hang, which is strictly *after*
    // iteration 1's agent armed it and exited. Its appearance is the
    // synchronization barrier proving "we are now wedged between iterations".
    let wedge_deadline = Instant::now() + Duration::from_secs(30);
    while !wedged.exists() {
        if Instant::now() >= wedge_deadline {
            kill_session_by_name(&session);
            panic!("compose never wedged in iteration-2 prep within 30s");
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // First press: registers the interrupt (flag + one-shot notice) but cannot
    // break the wedge — the wedge child ignores SIGINT and no interrupt
    // checkpoint follows the blocking call. Second press: with no wait loop
    // active, the compose guard force-exits the wrapper with `_exit(130)`.
    // Standard signals do not queue, so space the presses so both are
    // delivered as distinct SIGINTs.
    harness.send_key("C-c").expect("inject first Ctrl+C");
    std::thread::sleep(Duration::from_millis(600));
    harness.send_key("C-c").expect("inject second Ctrl+C");

    // The sentinel with the `130` suffix means claudine force-exited and the
    // shell ran the chained echo — Ctrl+C broke the between-iterations wedge.
    let needle = format!("{sentinel}_130");
    let term_deadline = Instant::now() + deadline;
    let mut last_plain = String::new();
    let mut returned = false;
    while Instant::now() < term_deadline {
        if let Ok(frame) = harness.capture() {
            last_plain = frame.plain;
            if last_plain.contains(&needle) {
                returned = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    kill_session_by_name(&session);
    (returned, last_plain)
}

/// A second Ctrl+C must force-exit a compose loop wedged in a blocking
/// synchronous call between iterations, returning exit code 130 promptly.
#[test]
#[serial(level2_tmux_ctrlc)]
fn level2_double_ctrl_c_force_exits_loop_wedged_between_iterations() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let (returned, plain) = double_ctrl_c_force_exits(Duration::from_secs(15));
    assert!(
        returned,
        "a second Ctrl+C must force-exit (code 130) a compose loop wedged in a \
         blocking call between iterations, where no child wait loop is active \
         and the interrupt flag alone cannot break in.\npane:\n{plain}",
    );
}
