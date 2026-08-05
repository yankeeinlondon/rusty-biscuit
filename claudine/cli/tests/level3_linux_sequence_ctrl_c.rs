//! Level 3 OS-keyboard-injection test for user Ctrl+C against a **sequence**
//! whose current step is a blocking `execution: parallel` group — Linux/X11.
//!
//! The Linux twin of `level3_sequence_ctrl_c.rs`. Same fixture shape, same four
//! user-facing claims, different injector: X11 XTEST via `xdotool` rather than
//! macOS Quartz via `cliclick`/System Events. Read the macOS file first; this
//! one documents only what differs.
//!
//! ## What it proves
//!
//! Pressing Ctrl+C while a parallel group is running must:
//!
//! - interrupt every task in the group,
//! - reap each task's *descendants*, not only its direct child,
//! - prevent the next sequence step from launching,
//! - return control to the shell with exit `130`.
//!
//! ## Why the keyboard tier is required here
//!
//! Every other Linux interrupt test in this area manufactures the condition —
//! `libc::kill(SIGINT)`, or setting the cooperative `AtomicBool` directly. Those
//! start downstream of the keyboard, the terminal's input encoder, and the tty's
//! foreground-process-group delivery. Only an XTEST key event enters where a
//! real keypress enters, so only it discharges a requirement phrased as "the
//! user presses Ctrl+C". The lower-level tests are retained for diagnosis: when
//! this test fails they are what distinguish a broken fan-out from a chord that
//! never landed.
//!
//! ## The fixture is deliberately SIGINT-immune
//!
//! A task running a plain `sleep` would risk proving nothing: a child sharing
//! Claudine's foreground process group would be killed by the tty-delivered
//! SIGINT directly, even with the fan-out entirely broken.
//!
//! Two independent things rule that out. `SystemTaskShell` spawns each task's
//! `sh -c` under `process_group(0)` (`lib/src/composition/sequence/task/shell.rs`),
//! so the children are not in the group the terminal broadcasts to; and each
//! task runs `trap '' INT` before blocking, so a delivered SIGINT would be
//! ignored anyway. The only thing left that can end a task is Claudine's own
//! machinery — the `signal_hook` handler sets the shared interrupt flag, each
//! task's wait loop observes it, and `ProcessTree::terminate` signals the task's
//! whole group with SIGTERM then an unignorable SIGKILL. A dead pid is therefore
//! positive evidence that the fan-out reached *that specific task*.
//!
//! ## The descendant is the point of the extra pid
//!
//! Each task also forks a background subshell that ignores SIGINT and blocks
//! forever. It inherits the task's process group but is *not* the process
//! Claudine holds a `Child` handle for, so killing only the direct child would
//! leave it running — the exact defect that motivated tree-scoped ownership.
//! Its death is the assertion that `ProcessTree::terminate` addresses the group
//! rather than the pid.
//!
//! Publishing the descendant's pid before the task's own also orders the
//! readiness barrier correctly: once `<task>.pid` exists, both processes are up.
//!
//! ## Skip-clean
//!
//! `WezTermHarness::available() && xdotool::available()` runs through
//! `require_level!(Level::L3, ...)`, which additionally skips unless
//! `RUN_LEVEL3=1`. `xdotool::available()` requires a live `DISPLAY`, so a
//! Wayland or headless host skips rather than red-failing on injection that
//! silently goes nowhere. `BISCUIT_TEST_LEVEL_REQUIRED=3` flips a missing
//! backend into a hard failure. Run via `just test-l3`.
//!
//! ## Execution status
//!
//! **Executed green once, headless-container X11 only** (2026-07-19, revision
//! `baba83844` + the uncommitted harness injector modules): Docker Linux
//! (aarch64, kernel 6.12.76), Xvfb + Openbox + WezTerm GUI, XTEST injection —
//! 1 passed, no retries, under `BISCUIT_TEST_LEVEL_REQUIRED=3`. See
//! `features/2026-07-11-sequence-plus/gate-run-2026-07-19-l3-linux.md`. An
//! **attended native-desktop Linux run is still owed** — the container has no
//! real desktop, focus contention, or WM variety. See
//! `features/2026-07-11-sequence-plus/l3-ctrl-c-runbook.md` for the procedure
//! and the four observations to record.
//!
//! ## This test takes the desktop
//!
//! It raises a GUI terminal to frontmost and injects real keystrokes into
//! whatever holds focus. Two guards bound that: `just test-l3` refuses to start
//! unattended (no TTY and no `BISCUIT_L3_TAKE_FOCUS=1` → hard error), and
//! `test_placement.rs::focus_stealing_apis_stay_in_keyboard_tier_files` keeps
//! `SpawnVisibility::Foreground` and `focus_spawned_pane` out of any file not
//! named `level3_*`.

#![cfg(target_os = "linux")]

mod common;
use common::write_executable;

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{SpawnVisibility, TerminalHarness, xdotool};
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

/// WezTerm replaces its OS window title with the foreground program's basename,
/// so once the sequence is running the pane's window is titled `claudine`. This
/// is what `xdotool search --name` matches on.
const WINDOW_TITLE_PATTERN: &str = "claudine";

/// Is `pid` still a live process?
///
/// `kill(pid, 0)` performs the permission and existence check without delivering
/// a signal. The tasks are grandchildren of the WezTerm pane, never of this test
/// process, so a reaped child cannot linger as a zombie that answers `true`.
fn pid_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

/// A fake `goose` so provider resolution succeeds without a network call. No
/// step in the fixture runs a prompt — every step is `shell:` — but the CLI
/// still requires a provider selection.
fn write_fake_goose(path: &Path) {
    write_executable(path, "#!/bin/sh\nprintf 'agent-said\\n'\nexit 0\n");
}

/// The sequence fixture: one blocking parallel group, then a step that must
/// never run.
///
/// `timeout: 300s` per task overrides the 30s `DEFAULT_COMMAND_TIMEOUT`. Without
/// it, a regression in which Ctrl+C did nothing would still see the children die
/// — to the ordinary timeout — and the assertions would pass for the wrong
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
fn level3_linux_sequence_ctrl_c_fans_out_to_parallel_children() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && xdotool::available(),
        "WezTerm + xdotool on X11",
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

    // Foreground spawn: XTEST injection goes to whichever window holds focus,
    // so the pane must be on the active workspace for `xdotool windowactivate`
    // to reach it. `with_expected_window_title` is a no-op on Linux (the AXRaise
    // path it feeds is macOS-only) but is set for symmetry with the macOS twin.
    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title(WINDOW_TITLE_PATTERN);
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    // Anchor CWD to the small temp workspace. A `wezterm cli spawn` pane
    // inherits the mux server's working directory — often a large repo — and
    // Claudine's startup repo detection walks the tree from CWD, which can stall
    // for tens of seconds so the pid files never appear. The tasks also write
    // their pid and marker files relative to this CWD.
    harness
        .send_command_with_env(&format!("cd '{}'", workspace.path().display()), &[])
        .expect("cd into workspace");

    // A short, shell-expanded `export` rather than an inline `PATH=<2500 chars>`
    // prefix: the full system PATH inlined into one typed line overflows
    // WezTerm's send-text into a multi-row wrap the shell then fails to run as a
    // single command.
    harness
        .send_command_with_env(
            &format!("export PATH='{}':\"$PATH\"", path_dir.display()),
            &[],
        )
        .expect("prepend PATH");

    // The chained `echo` carries Claudine's exit status, so one sentinel proves
    // both "the shell regained control" and "the exit code was 130". It is
    // printed if and only if Claudine exited.
    let sentinel = format!("L3LINSEQ_{}", SEQ.fetch_add(1, Ordering::Relaxed));
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
    // orchestrator registered its SIGINT handler and after each task's wait loop
    // began polling the shared interrupt flag. Injecting earlier would race the
    // handler.
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

    // Activate the pane inside WezTerm. On Linux this stamps the tab title and
    // focuses the pane; the OS-level window raise is xdotool's job below, so the
    // `Ok(None)` this returns off macOS is expected and carries no coordinates.
    harness
        .focus_spawned_pane()
        .expect("activate spawned WezTerm pane");

    // Genuine OS keyboard injection: focus the window, then inject Ctrl+C
    // through XTEST. WezTerm's input encoder turns the chord into ETX → SIGINT
    // for the foreground process group.
    xdotool::focus_then_ctrl_chord(WINDOW_TITLE_PATTERN, "c").expect(
        "inject OS Ctrl+C chord via xdotool (needs exactly one window titled \
         `claudine` and a window manager that honors programmatic activation)",
    );

    // Claims 3 + 4: the shell regained control, and Claudine's status was 130.
    //
    // 15s (matching the macOS twin) rather than a longer leash: the whole test
    // must finish inside nextest's 30s termination budget, so a chord that never
    // lands reports as this assertion — with the pane dump below — instead of an
    // opaque `TMT` that says nothing about why.
    let term_deadline = Instant::now() + Duration::from_secs(15);
    let mut last_plain = String::new();
    let mut observed_rc: Option<String> = None;
    // Match the *last* occurrence, not the first. The pane holds the echoed
    // command line as well as its output, so `<sentinel>rc=` appears twice
    // whenever the command has not yet scrolled off the visible capture: first
    // as the literal `; echo <sentinel>rc=$?` the shell echoed, then as the
    // result. Reading the first match yields `$?` — no digits — so the poll
    // would spin out its deadline while the sequence had in fact already exited
    // correctly, reporting a product failure that never happened.
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
             the shell within 15s. If the pane below shows the group still running \
             with no `^C`, the xdotool chord did not land (wrong window focused, or \
             a window manager that refused activation) rather than the fan-out \
             being broken.\npane:\n{last_plain}"
        )
    });
    assert_eq!(
        observed_rc, "130",
        "an interrupted sequence must exit 130.\npane:\n{last_plain}",
    );

    // Claims 1 + 2: every parallel child AND every descendant terminated. The
    // fixture ignores SIGINT throughout, so a dead pid can only be Claudine's
    // interrupt fan-out reaching that process group. Poll briefly — the shell
    // prints the sentinel as soon as Claudine exits, which can very slightly
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
        "Ctrl+C must fan out to every running parallel child and its descendants; \
         these survived (they ignore SIGINT, so surviving means the interrupt \
         never reached their process group): {survivors:?}\npane:\n{last_plain}",
    );

    // Claim 3: the step after the interrupted group never launched.
    assert!(
        !later_marker.exists(),
        "the sequence step after an interrupted group must not start, but it \
         wrote its marker.\npane:\n{last_plain}",
    );

    // `harness` Drop kills the pane; no explicit teardown needed.
}
