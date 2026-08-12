//! Level 3 OS-keyboard-injection test for user Ctrl+C against a **sequence**
//! whose current step is a blocking `execution: parallel` group — Windows.
//!
//! The Windows twin of `level3_sequence_ctrl_c.rs` (macOS) and
//! `level3_linux_sequence_ctrl_c.rs`. Same four user-facing claims, different
//! injector and a different fixture vocabulary. Read the macOS file first; this
//! one documents only what differs.
//!
//! ## Why this exists alongside `level2_windows_sequence_ctrl_c.rs`
//!
//! That fixture drives `GenerateConsoleCtrlEvent`, which posts a console-control
//! notification straight to a process group. It is a real test of Claudine's
//! *signal path*, but no key is pressed and no terminal encodes one, so it
//! cannot discharge a requirement phrased as "the user presses Ctrl+C". It is
//! deliberately retained: when this Level-3 test fails, the Level-2 fixture is
//! what separates "the fan-out is broken" from "the chord never landed".
//!
//! This test injects through `SendKeys.SendWait`, which drives
//! `keybd_event`/`SendInput` — the keystroke enters the system input queue where
//! a physical keypress enters, is delivered to the focused window, and is
//! encoded by *that terminal* into the bytes Claudine finally sees.
//!
//! ## Why the pane runs `cmd.exe` directly rather than `spawn_shell`
//!
//! `TerminalHarness::spawn_shell` resolves a POSIX login shell (`bash`/`sh`
//! with `-l`) and joins `PATH` entries with `:`. Both are Unix-shaped. Rather
//! than depend on Git Bash being installed and on a `PATH` separator that is
//! wrong for this platform, the pane is spawned with `spawn_program("cmd.exe")`
//! and driven with raw `send_text`. This also keeps the fixture in the shell
//! Claudine itself uses for `shell:` tasks on Windows (`cmd /C`).
//!
//! ## Why the exit code is echoed only after the injection
//!
//! On Unix the whole thing is one line — `claudine … ; echo rc=$?` — because the
//! trailing `echo` survives the interrupt. Windows delivers `CTRL_C_EVENT` to
//! *every* process attached to the console, so any wrapper (`cmd /v:on /c "… &
//! echo …"`) would be terminated before it could report. An interactive `cmd`
//! prompt, by contrast, survives Ctrl+C and preserves `%ERRORLEVEL%` from the
//! command that was interrupted. The echo is therefore typed as a separate line
//! after the chord, which is also exactly what a user does.
//!
//! ## Descendants, and why liveness is not measured by pid
//!
//! Each task launches `start /b ping -n 60N 127.0.0.1` — a grandchild that
//! outlives the task's own loop, is *not* the process Claudine holds a `Child`
//! handle for, and deliberately **inherits the task's stdout pipe**. That last
//! detail is the hazard tree-scoped ownership exists for: a descendant holding
//! the pipe open could keep emitting frames after Claudine believes the command
//! is dead. The ownership contract answers it twice — `ProcessTree` terminates
//! the Job on interrupt and reaps any remainder at completion (which closes the
//! pipe), and the reader settle is bounded by `READER_SHUTDOWN_GRACE` so a
//! surviving handle costs seconds, not a hang. If the Job Object failed to reap
//! the tree, the descendant would outlive the sequence and the live-descendant
//! count below would report it as a survivor.
//!
//! The per-task `-n 60N` count makes each descendant's command line unique, so
//! it can be counted by `Win32_Process` without being confused with the
//! short-lived `ping` the tick loop uses as a sleep. Pids are avoided for the
//! task itself for the reason the Level-2 fixture documents: a `cmd /C` task
//! spawns its own grandchildren, so a pid check on any one process can report a
//! dead tree that is still ticking. A file that stops growing proves the whole
//! tree stopped.
//!
//! ## Skip-clean
//!
//! `WezTermHarness::available() && win_input::available()` runs through
//! `require_level!(Level::L3, ...)`, which additionally skips unless
//! `RUN_LEVEL3=1`. `BISCUIT_TEST_LEVEL_REQUIRED=3` flips a missing backend into
//! a hard failure. Run via `just test-l3`.
//!
//! ## Execution status
//!
//! **Not yet run.** Authored and type-checked only; no Windows Level-3 execution
//! evidence exists for this test. It requires an attended Windows desktop with
//! WezTerm running. See `features/2026-07-11-sequence-plus/l3-ctrl-c-runbook.md`
//! for the procedure and the four observations to record.
//!
//! ## This test takes the desktop
//!
//! It raises a GUI terminal to frontmost and injects real keystrokes into
//! whatever holds focus. Two guards bound that: `just test-l3` refuses to start
//! unattended (no TTY and no `BISCUIT_L3_TAKE_FOCUS=1` → hard error), and
//! `test_placement.rs::focus_stealing_apis_stay_in_keyboard_tier_files` keeps
//! `SpawnVisibility::Foreground` and `focus_spawned_pane` out of any file not
//! named `level3_*`.

#![cfg(windows)]

mod common;

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{SpawnVisibility, TerminalHarness, win_input};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, require_level};

/// The parallel tasks the group launches, each paired with the `ping -n` count
/// that makes its long-lived descendant's command line unique.
const TASKS: [(&str, u32); 3] = [("child-a", 601), ("child-b", 602), ("child-c", 603)];

/// WezTerm replaces its OS window title with the foreground program's basename,
/// so once the sequence is running the pane's window title contains `claudine`.
/// This is the token `win_input` matches on.
const WINDOW_TITLE_TOKEN: &str = "claudine";

/// This task's heartbeat file, appended to about once a second while any part of
/// its own loop is alive.
fn heartbeat(workspace: &Path, task: &str) -> PathBuf {
    workspace.join(format!("{task}.live"))
}

/// A fake `goose` so provider resolution succeeds without a network call. No
/// step in the fixture runs a prompt — every step is `shell:` — but the CLI
/// still requires a provider selection. `.cmd` because Windows program lookup
/// honors `PATHEXT`, not an executable bit.
fn write_fake_goose(dir: &Path) {
    fs::write(
        dir.join("goose.cmd"),
        "@echo off\r\necho agent-said\r\nexit /b 0\r\n",
    )
    .unwrap();
}

/// One blocking parallel group, then a step that must never run.
///
/// `timeout: 300s` per task overrides the 30s `DEFAULT_COMMAND_TIMEOUT`. Without
/// it, a regression in which Ctrl+C did nothing would still see the children die
/// — to the ordinary timeout — and the liveness assertions would pass for the
/// wrong reason.
fn sequence_document() -> String {
    let task_entries: String = TASKS
        .iter()
        .map(|(name, descendant_count)| {
            // `ping -n 2 127.0.0.1` is the portable `cmd` sleep, so the loop
            // ticks about once a second. The `start /b` ping ahead of it is the
            // long-lived descendant; it inherits stdout on purpose.
            format!(
                "        - name: {name}\n\
                 \x20         timeout: 300s\n\
                 \x20         shell: \"start /b ping -n {descendant_count} 127.0.0.1 & \
                 for /l %i in (1,1,600) do @(echo tick >> {name}.live & \
                 ping -n 2 127.0.0.1 > nul)\"\n"
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
         \x20   shell: \"echo ran > later-step-ran.txt\"\n\
         ---\n\
         \n\
         Body.\n"
    )
}

/// Counts live `ping` processes whose command line carries `-n <count>`.
///
/// Identifies one task's long-lived descendant without matching the tick loop's
/// short-lived `ping -n 2`. Returns `0` when the query itself fails, which would
/// surface as the cleanup assertion passing — acceptable only because the query
/// is also run *before* the injection, where a spurious `0` fails the fixture
/// sanity check loudly instead.
fn live_descendants(count: u32) -> u32 {
    let script = format!(
        "@(Get-CimInstance Win32_Process -Filter \"Name='PING.EXE'\" | \
         Where-Object {{ $_.CommandLine -like '*-n {count} *' }}).Count"
    );
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()
        .and_then(|out| String::from_utf8_lossy(&out.stdout).trim().parse().ok())
        .unwrap_or(0)
}

/// Types one line into the pane. `cmd.exe` needs CRLF to accept it.
fn send_line(harness: &mut WezTermHarness, line: &str) {
    harness
        .send_text(format!("{line}\r\n").as_bytes())
        .unwrap_or_else(|error| panic!("send {line:?} to cmd pane: {error}"));
}

/// A real OS Ctrl+C keystroke, delivered to a focused WezTerm window running a
/// sequence that is blocked inside a parallel group, must terminate every
/// parallel task and its descendants, suppress the following step, return
/// control to the shell, and exit `130`.
#[test]
#[serial(level3_keyboard)]
fn level3_windows_sequence_ctrl_c_fans_out_to_parallel_children() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && win_input::available(),
        "WezTerm + Windows key injection",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    common::wrap::seed_minimal_config(workspace.path());
    write_fake_goose(&path_dir);

    let md_file = workspace.path().join("seq.md");
    fs::write(&md_file, sequence_document()).unwrap();

    let later_marker = workspace.path().join("later-step-ran.txt");

    // Foreground spawn: the injected keystroke goes to whichever window holds
    // focus, so the pane must not be hidden in WezTerm's off-screen background
    // workspace. `spawn_program` rather than `spawn_shell` — see module docs.
    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title(WINDOW_TITLE_TOKEN);
    harness
        .spawn_program("cmd.exe", &[])
        .expect("spawn WezTerm cmd.exe pane");
    // `spawn_program` performs no prompt-readiness wait, so settle before typing.
    std::thread::sleep(Duration::from_secs(1));

    // Anchor CWD to the small temp workspace. A `wezterm cli spawn` pane
    // inherits the mux server's working directory — often a large repo — and
    // Claudine's startup repo detection walks the tree from CWD, which can stall
    // for tens of seconds so the heartbeats never appear. The tasks also write
    // their marker files relative to this CWD. `/d` is required to change drive
    // as well as directory.
    send_line(
        &mut harness,
        &format!("cd /d {}", workspace.path().display()),
    );
    send_line(
        &mut harness,
        &format!("set PATH={};%PATH%", path_dir.display()),
    );
    send_line(&mut harness, "set NO_COLOR=1");
    send_line(
        &mut harness,
        &format!("set HOME={}", workspace.path().display()),
    );
    send_line(
        &mut harness,
        &format!("set USERPROFILE={}", workspace.path().display()),
    );

    let claudine = common::claudine_bin();
    send_line(
        &mut harness,
        &format!(
            "\"{claudine}\" sequence --goose --yolo \"{md}\"",
            md = md_file.display(),
        ),
    );

    // Readiness barrier: every task has emitted its first heartbeat and its
    // descendant is running, so every child is in its loop — strictly after the
    // orchestrator registered its console-control handler and after each task's
    // wait loop began reading the shared interrupt flag. Injecting earlier would
    // race the handler installation.
    let ready_deadline = Instant::now() + Duration::from_secs(60);
    for (name, descendant_count) in TASKS {
        let marker = heartbeat(workspace.path(), name);
        loop {
            assert!(
                Instant::now() < ready_deadline,
                "parallel task {name} never started; the group did not reach its \
                 blocking state",
            );
            if fs::metadata(&marker).is_ok_and(|meta| meta.len() > 0)
                && live_descendants(descendant_count) > 0
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    // Activate the pane inside WezTerm. The OS-level window raise is
    // `win_input`'s job below, so the `Ok(None)` this returns off macOS is
    // expected and carries no coordinates.
    harness
        .focus_spawned_pane()
        .expect("activate spawned WezTerm pane");

    // Genuine OS keyboard injection. WezTerm's input encoder turns the chord
    // into ETX, which the console subsystem raises as CTRL_C_EVENT for the
    // attached process group.
    win_input::focus_then_ctrl_chord(WINDOW_TITLE_TOKEN, "c").expect(
        "inject OS Ctrl+C chord (needs exactly one window whose title contains \
         `claudine`, and a desktop session that permits foreground activation)",
    );

    // Give Claudine a moment to unwind before typing, so the echo lands at a
    // restored prompt rather than into a process still reading stdin. Typing
    // early is not incorrect — `cmd` parses each line as it reaches it, so
    // `%ERRORLEVEL%` still expands after Claudine exits — but it keeps the pane
    // capture readable.
    std::thread::sleep(Duration::from_secs(2));
    let sentinel = format!("L3WINSEQ_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    send_line(&mut harness, &format!("echo {sentinel}rc=%ERRORLEVEL%"));

    // Claims 3 + 4: the shell regained control, and Claudine's status was 130.
    //
    // A hang here is the failure mode tree-scoped ownership exists to prevent —
    // a descendant holding the stdout pipe open past the interrupt — so it is
    // bounded rather than awaited.
    let term_deadline = Instant::now() + Duration::from_secs(20);
    let mut last_plain = String::new();
    let mut observed_rc: Option<String> = None;
    // Match the *last* occurrence, not the first: the pane holds the echoed
    // command line as well as its output, so `<sentinel>rc=` appears twice while
    // the command is still on screen — first as the literal
    // `echo <sentinel>rc=%ERRORLEVEL%` that cmd echoed, then as the result.
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
             the shell within 20s. If the pane below shows the group still running, \
             the chord did not land (wrong window focused, or foreground \
             activation refused) rather than the fan-out being broken.\npane:\n{last_plain}"
        )
    });
    assert_eq!(
        observed_rc, "130",
        "an interrupted sequence must exit 130.\npane:\n{last_plain}",
    );

    // Claim 1a: every task's own loop stopped. Two heartbeat samples three
    // seconds apart (the loop ticks about once a second) must be identical.
    let ticks = |name: &str| {
        fs::metadata(heartbeat(workspace.path(), name))
            .expect("heartbeat file")
            .len()
    };
    let before: Vec<u64> = TASKS.iter().map(|(name, _)| ticks(name)).collect();
    std::thread::sleep(Duration::from_secs(3));
    for ((name, _), was) in TASKS.iter().zip(before) {
        assert_eq!(
            ticks(name),
            was,
            "task {name} was still running after the interrupt.\npane:\n{last_plain}",
        );
    }

    // Claim 1b: every task's long-lived descendant was reaped with it. This is
    // what distinguishes tree-scoped ownership from killing the direct child.
    let survivors: Vec<&str> = TASKS
        .iter()
        .filter(|(_, count)| live_descendants(*count) > 0)
        .map(|(name, _)| *name)
        .collect();
    assert!(
        survivors.is_empty(),
        "Ctrl+C must reap each interrupted task's whole process tree; these tasks \
         left a descendant running: {survivors:?}\npane:\n{last_plain}",
    );

    // Claim 2: the step after the interrupted group never launched.
    assert!(
        !later_marker.exists(),
        "the sequence step after an interrupted group must not start, but it wrote \
         its marker.\npane:\n{last_plain}",
    );

    // `harness` Drop kills the pane; no explicit teardown needed.
}
