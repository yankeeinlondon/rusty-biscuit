//! Level 3 OS-keyboard-injection test for user Ctrl+C against a **sequence**
//! whose current step is a blocking `execution: parallel` group.
//!
//! ## Why this exists separately from `level3_wrap_ctrl_c.rs`
//!
//! `level3_wrap_ctrl_c.rs` proves the shared wrapper receives one real OS
//! keyboard chord while running a standalone `claudine compose`. It says
//! nothing about sequence scheduling or group fan-out, because a standalone
//! compose has exactly one child and no later step to suppress. The spec's
//! concurrency claims (`spec.md` → "Timeouts, Guards, and Signals", and the
//! all-child Ctrl+C fan-out acceptance criterion) are about the sequence
//! orchestrator:
//!
//! - every running parallel child terminates, descendants included,
//! - no later sequence step launches,
//! - the shell regains control,
//! - the process exits `130`.
//!
//! Every other sequence-interrupt test in this area manufactures the condition
//! — `libc::kill(SIGINT)` or setting the cooperative `AtomicBool` directly.
//! That is Level 1: it starts *downstream* of the keyboard, the terminal's
//! input encoder, and the tty's foreground-process-group delivery. This test
//! starts at a synthesized macOS Quartz key event, which is the only tier that
//! discharges a "when the user presses Ctrl+C" claim.
//!
//! ## The fixture is deliberately SIGINT-immune
//!
//! The obvious fixture — a task running `sleep 300` — risks proving almost
//! nothing: a child sharing claudine's foreground process group would be killed
//! by the tty-delivered SIGINT directly, even if claudine's fan-out were
//! entirely broken, and the test would pass on a regression.
//!
//! Two independent things now rule that out. `SystemTaskShell` spawns each
//! task's `sh -c` under `process_group(0)` (see
//! `lib/src/composition/sequence/task/shell.rs`), so the children are no longer
//! in the foreground group the terminal broadcasts to; and each task
//! additionally runs `trap '' INT` before looping forever, so even a delivered
//! SIGINT would be ignored. The only remaining thing that can end a task is
//! claudine's own machinery: the sequence's `signal_hook` handler sets the
//! shared interrupt flag, every task's wait loop observes it, and
//! `ProcessTree::terminate` signals the task's whole group — SIGTERM, then an
//! unconditional, unignorable SIGKILL. Each dead pid is therefore positive
//! evidence that the fan-out reached *that specific task*, not a side effect of
//! the terminal's process-group broadcast.
//!
//! Each task publishes its own pid before blocking, which serves double duty:
//! the pid files are the readiness barrier (all children are in their loops
//! before the keystroke is injected, closing the handler-install race) and the
//! per-child identity used to prove each one died.
//!
//! ## The descendant is the point of the extra pid
//!
//! Each task also forks a background subshell that ignores SIGINT and blocks
//! forever. It inherits the task's process group but is *not* the process
//! Claudine holds a `Child` handle for, so killing only the direct child would
//! leave it running — the exact defect that motivated tree-scoped ownership,
//! now fail-closed with reap-on-completion (`ProcessTree` in
//! `lib/src/composition/sequence/task/shell.rs`). Its death is the assertion
//! that termination addresses the group rather than the pid.
//!
//! Publishing the descendant's pid before the task's own also orders the
//! readiness barrier correctly: once `<task>.pid` exists, both processes are up.
//!
//! ## One test, not several
//!
//! L3 is the most expensive tier and depends on a focused GUI window. This
//! file carries a single test that asserts all four spec claims from one
//! keystroke rather than paying the ~15s window cost per claim.
//!
//! ## Skip-clean
//!
//! `WezTermHarness::available() && cliclick::available()` is checked via
//! `require_level!(Level::L3, ...)`, which also skips unless `RUN_LEVEL3=1`.
//! `BISCUIT_TEST_LEVEL_REQUIRED=3` flips a missing backend into a hard
//! failure. Run via `just test-l3`. The `level3_` prefix keeps it out of the
//! `just test` (L1) and `just test-l2` filtersets.
//!
//! macOS only: `cliclick` is the only injector wired into the harness, so the
//! file is honestly `#[cfg(target_os = "macos")]` rather than pretending
//! portability.
//!
//! ## Execution status
//!
//! Observed passing on 2026-07-18: all three children reported `interrupted`,
//! the `must-not-run` step was suppressed, and the pane's exit marker read
//! `L3SEQ_0rc=130`. **That pass is stale**: it predates the task-shell
//! process-tree ownership rewrite (fail-closed `ProcessTree` with
//! reap-on-completion) and the descendant-cleanup assertion this fixture now
//! carries. Current evidence requires an attended re-run at the current
//! revision — see `features/2026-07-11-sequence-plus/l3-ctrl-c-runbook.md`.
//!
//! It did *not* pass when first authored — the cliclick chord never reached
//! WezTerm, matching the focus-transfer limit documented in
//! `level3_wrap_ctrl_c.rs`. Two `biscuit-test-harness` changes fixed delivery:
//! `focus_spawned_pane` now polls until WezTerm is genuinely the frontmost
//! application rather than sleeping a fixed 200ms after `AXRaise`, and the
//! Ctrl chord now goes through AppleScript `keystroke … using control down`,
//! which carries the modifier flag on the same key event instead of racing it
//! (measured 24/24 delivery versus 7/10 for cliclick's `kd:ctrl t:c ku:ctrl`).
//! A checkout without those harness changes reproduces the original failure.
//!
//! This test is deliberately NOT loosened to force a pass. It asserts real
//! termination and fails honestly when the OS event does not land — the
//! corroborating `tmux send-keys C-c` run over the same fixture lives at L2 and
//! does not substitute for the OS-keyboard leg.
//!
//! ## This test takes the desktop
//!
//! It raises a GUI terminal to frontmost and injects real keystrokes into
//! whatever holds focus. Two guards bound that: `just test-l3` refuses to start
//! unattended (no TTY and no `BISCUIT_L3_TAKE_FOCUS=1` → hard error, so agents
//! and CI cannot hijack an active session), and
//! `test_placement.rs::focus_stealing_apis_stay_in_keyboard_tier_files` keeps
//! `SpawnVisibility::Foreground` and `focus_spawned_pane` out of any file not
//! named `level3_*`.

#![cfg(target_os = "macos")]

mod common;
use common::write_executable;

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{SpawnVisibility, TerminalHarness, cliclick};
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, require_level};

/// The parallel tasks the group launches. Named so a failure message can say
/// which child survived.
const TASKS: [&str; 3] = ["child-a", "child-b", "child-c"];

/// Is `pid` still a live process?
///
/// `kill(pid, 0)` performs the permission and existence check without
/// delivering a signal. The tasks are grandchildren of the WezTerm pane, never
/// of this test process, so a reaped child cannot linger as a zombie that
/// answers `true` here.
fn pid_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

/// A fake `goose` so the sequence's provider resolution succeeds without a
/// network call. No step in the fixture actually runs a prompt — every step is
/// `shell:` — but the CLI still requires a provider selection.
fn write_fake_goose(path: &Path) {
    write_executable(path, "#!/bin/sh\nprintf 'agent-said\\n'\nexit 0\n");
}

/// The sequence fixture: one blocking parallel group, then a step that must
/// never run.
///
/// `timeout: 300s` per task overrides the 30s `DEFAULT_COMMAND_TIMEOUT`. Without
/// it, a regression in which Ctrl+C did nothing would still see the children
/// die — to the ordinary timeout — and the assertions would pass for the wrong
/// reason. 300s is far outside the interrupt window, so only the keystroke can
/// end them inside it.
fn sequence_document() -> String {
    let task_entries: String = TASKS
        .iter()
        .map(|name| {
            format!(
                "        - name: {name}\n\
                 \x20         timeout: 300s\n\
                 \x20         shell: \"trap '' INT; (trap '' INT; while :; do sleep 1; done) & \
                 echo $! > {name}.desc.pid; echo $$ > {name}.pid; while :; do sleep 1; done\"\n"
            )
        })
        .collect();

    format!(
        "---\n\
         sequence:\n\
         \x20 - name: blocking-group\n\
         \x20   group:\n\
         \x20     name: bundle\n\
         \x20     execution: parallel\n\
         \x20     tasks:\n\
         {task_entries}\
         \x20 - name: must-not-run\n\
         \x20   shell: \"printf 'ran\\\\n' > later-step-ran.txt\"\n\
         ---\n\
         \n\
         Body.\n"
    )
}

/// Blocks until `path` holds a parsable pid, or the deadline passes.
fn await_pid(path: &Path, deadline: Instant, what: &str) -> i32 {
    loop {
        if let Ok(text) = fs::read_to_string(path)
            && let Ok(pid) = text.trim().parse::<i32>()
        {
            return pid;
        }
        assert!(
            Instant::now() < deadline,
            "{what} never published a pid; the group did not reach its blocking state",
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// A real OS Ctrl+C keystroke, delivered to a focused WezTerm window running a
/// sequence that is blocked inside a parallel group, must terminate every
/// parallel child and its descendants, suppress the following step, return
/// control to the shell, and exit `130`.
#[test]
#[serial(level3_keyboard)]
fn level3_sequence_ctrl_c_fans_out_to_parallel_children() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    common::wrap::seed_minimal_config(workspace.path());
    write_fake_goose(&path_dir.join("goose"));

    let md_file = workspace.path().join("seq.md");
    fs::write(&md_file, sequence_document()).unwrap();

    let later_marker = workspace.path().join("later-step-ran.txt");

    // Foreground spawn: cliclick injection requires the window to be on the
    // active workspace so `focus_spawned_pane` (AXRaise + click) can route OS
    // events to it. `with_expected_window_title("claudine")` is needed because
    // WezTerm overrides the OS window title with the foreground program's
    // basename rather than the harness's stamped tab title.
    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    // Anchor CWD to the small temp workspace. A `wezterm cli spawn` pane
    // inherits the mux server's working directory — often a large repo — and
    // claudine's startup repo detection walks the tree from CWD, which can
    // stall for tens of seconds so the pid files never appear. The tasks also
    // write their pid and marker files relative to this CWD.
    harness
        .send_command_with_env(&format!("cd '{}'", workspace.path().display()), &[])
        .expect("cd into workspace");

    // A short, shell-expanded `export` rather than an inline `PATH=<2500
    // chars>` prefix: the full system PATH inlined into one typed line
    // overflows WezTerm's send-text into a multi-row wrap the shell then fails
    // to run as a single command.
    harness
        .send_command_with_env(
            &format!("export PATH='{}':\"$PATH\"", path_dir.display()),
            &[],
        )
        .expect("prepend PATH");

    // The chained `echo` carries claudine's exit status, so one sentinel proves
    // both "the shell regained control" and "the exit code was 130". It is
    // printed if and only if claudine exited.
    let sentinel = format!("L3SEQ_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let claudine = common::claudine_bin();
    let cmd = format!(
        "{claudine} sequence --goose --yolo {md} ; echo {sentinel}rc=$?",
        md = md_file.display(),
    );
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("NO_COLOR", "1"),
                ("HOME", &workspace.path().display().to_string()),
            ],
        )
        .expect("send sequence command");

    // Readiness barrier: every task has published its pid, so every child and
    // its descendant are inside their blocking loops — strictly after the
    // orchestrator registered its SIGINT handler and after each task's wait
    // loop began polling the shared interrupt flag. Injecting earlier would
    // race the handler.
    let marker_deadline = Instant::now() + Duration::from_secs(30);
    let mut pids: Vec<(String, i32)> = Vec::new();
    for name in TASKS {
        // `<task>.pid` is written last, so waiting on it also guarantees the
        // descendant's pid file is already complete.
        let task_pid = await_pid(
            &workspace.path().join(format!("{name}.pid")),
            marker_deadline,
            &format!("parallel task {name}"),
        );
        let descendant_pid = await_pid(
            &workspace.path().join(format!("{name}.desc.pid")),
            marker_deadline,
            &format!("descendant of {name}"),
        );
        pids.push((name.to_string(), task_pid));
        pids.push((format!("{name} descendant"), descendant_pid));
    }

    // Sanity: the group really is running concurrently right now. Without this,
    // a fixture that died on its own would make the termination assertions
    // vacuous.
    for (name, pid) in &pids {
        assert!(
            pid_alive(*pid),
            "{name} (pid {pid}) was not alive at injection time; the fixture is \
             not blocking as intended",
        );
    }

    // Raise our specific WezTerm window and obtain click coordinates. A missing
    // Accessibility grant, or a window-title mismatch, surfaces here rather
    // than as a silent miss.
    let coords = harness
        .focus_spawned_pane()
        .expect("focus spawned WezTerm pane (AXRaise: Accessibility grant + window-title match)")
        .expect("AXRaise yielded no window coords (non-macOS or AX failure)");

    // Genuine OS keyboard injection: click to transfer focus, then the Ctrl+C
    // chord, atomic within one cliclick invocation. WezTerm's input encoder
    // turns the chord into ETX → SIGINT for the foreground process group.
    cliclick::click_then_ctrl_chord(coords.0, coords.1, "c").expect("inject OS Ctrl+C chord");

    // Claim 3 + 4: the shell regained control, and claudine's status was 130.
    //
    // 15s (matching `level3_wrap_ctrl_c.rs`) rather than a longer leash: the
    // whole test must finish inside nextest's 30s termination budget, so a
    // chord that never lands reports as this assertion — with the pane dump
    // below — instead of an opaque `TMT` that says nothing about why.
    let term_deadline = Instant::now() + Duration::from_secs(15);
    let mut last_plain = String::new();
    let mut observed_rc: Option<String> = None;
    // Match the *last* occurrence, not the first. The pane holds the echoed
    // command line as well as its output, so `<sentinel>rc=` appears twice
    // whenever the command has not yet scrolled off the visible capture: first
    // as the literal `; echo <sentinel>rc=$?` the shell echoed, then as the
    // result. Reading the first match yields `$?` — no digits — so the poll
    // would spin out its deadline while the sequence had in fact already exited
    // correctly, reporting a product failure that never happened. Whether the
    // command line is still on screen depends on pane height and output volume,
    // which is why this only bit once the fixture grew.
    let needle = format!("{sentinel}rc=");
    while Instant::now() < term_deadline {
        if let Ok(frame) = harness.capture() {
            last_plain = frame.plain;
            if let Some(index) = last_plain.rfind(&needle) {
                let code: String = last_plain[index + needle.len()..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                if !code.is_empty() {
                    observed_rc = Some(code);
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let observed_rc = observed_rc.unwrap_or_else(|| {
        panic!(
            "OS Ctrl+C keystroke must terminate the sequence and return control to \
             the shell within 15s. If the pane below shows the group still \
             running with no `^C`, the cliclick chord did not land (the \
             documented focus-transfer limit) rather than the fan-out being \
             broken.\npane:\n{last_plain}"
        )
    });
    assert_eq!(
        observed_rc, "130",
        "an interrupted sequence must exit 130.\npane:\n{last_plain}",
    );

    // Claim 1: every parallel child AND every descendant terminated. The
    // fixture ignores SIGINT throughout, so a dead pid can only be claudine's
    // interrupt fan-out reaching that process group. Poll briefly — the shell
    // prints the sentinel as soon as claudine exits, which can very slightly
    // precede the last child's reaping.
    let reap_deadline = Instant::now() + Duration::from_secs(5);
    let survivors = loop {
        let survivors: Vec<&(String, i32)> =
            pids.iter().filter(|(_, pid)| pid_alive(*pid)).collect();
        if survivors.is_empty() || Instant::now() >= reap_deadline {
            break survivors
                .iter()
                .map(|(name, pid)| format!("{name} (pid {pid})"))
                .collect::<Vec<_>>();
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(
        survivors.is_empty(),
        "Ctrl+C must fan out to every running parallel child and its \
         descendants; these survived (they ignore SIGINT, so surviving means \
         the interrupt never reached their process group): \
         {survivors:?}\npane:\n{last_plain}",
    );

    // Claim 2: the step after the interrupted group never launched.
    assert!(
        !later_marker.exists(),
        "the sequence step after an interrupted group must not start, but it \
         wrote its marker.\npane:\n{last_plain}",
    );

    // `harness` Drop kills the pane; no explicit teardown needed.
}
