//! Windows-host integration test for a sequence interrupted while blocked
//! inside a parallel group.
//!
//! ## What it proves
//!
//! The spec's interruption contract is one behavior with two platform
//! producers. On Unix a `signal_hook` SIGINT handler sets the sequence's shared
//! `interrupted` flag; on Windows a process-scoped console-control handler does
//! (`wrap/exec/termination/windows.rs`). Everything downstream — step-boundary
//! suppression, cooperative shell-task termination, the `130` exit — reads only
//! that flag, so this test drives the Windows producer and asserts the same four
//! user-observable claims the Unix Level-3 test asserts:
//!
//! - every parallel child terminates,
//! - no later sequence step launches,
//! - the shell regains control,
//! - the process exits `130`.
//!
//! ## Level
//!
//! **Level 1**, gated by `#[cfg(windows)]` so it runs on the Windows leg of the
//! ordinary matrix and nowhere else — the same way `wrap_sigint.rs` is gated by
//! `#[cfg(unix)]`. A real Claudine process runs a real sequence with real child
//! processes, and the interrupt arrives through the real Win32 console-control
//! path rather than by setting an `AtomicBool`. It is not Level 3 because the
//! event is synthesized with `GenerateConsoleCtrlEvent` instead of an OS
//! keyboard chord — it starts downstream of the keyboard and the terminal's
//! input encoder. It is not Level 2 either: it needs no terminal harness, only a
//! console. The separate `level3_windows_sequence_ctrl_c` fixture uses the
//! harness's Windows input injector to cover that keyboard boundary.
//!
//! It carried a `level2_` prefix until 2026-07-27, which excluded it from the
//! Windows L1 leg while CI's L2 job runs only on Linux — so it never executed
//! anywhere (`features/2026-07-11-sequence-plus/validation-matrix.md` recorded
//! it as "type-checked, never executed").
//!
//! ## Why the marker file and the exit code carry the weight
//!
//! Claudine is spawned in `CREATE_NEW_PROCESS_GROUP`, so the injected
//! `CTRL_BREAK_EVENT` reaches its group and no other. Each task's `cmd /C`
//! child is created suspended in its own process group, assigned to a
//! kill-on-close Job Object, and then resumed (see
//! `lib/src/composition/sequence/task/shell.rs`). The shared interrupt flag
//! therefore drives each task shell to terminate its owned Job tree. The
//! per-task `interrupted` label is deliberately not asserted: whether the task's
//! poll loop observes the shared flag or the already-terminated child first is
//! a genuine race on this platform.
//!
//! Liveness is measured by heartbeat rather than by pid. A `cmd /C` task can
//! spawn grandchildren, so a pid check on any one process could report a dead
//! process while other members of the Job tree are still ticking. A file that
//! stops growing proves the fixture's work stopped.
//!
//! The two assertions that a broken implementation cannot pass are the ones
//! that read the shared flag and nothing else: `later-step-ran.txt` must not
//! exist, and the exit code must be `130`. Before the Windows console
//! coordinator existed, nothing in the process could set that flag — the
//! sequence would have run its second step and exited on the group's own
//! status.

#![cfg(windows)]

mod common;

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::tempdir;

/// `CREATE_NEW_PROCESS_GROUP` — makes Claudine's pid a console process-group id
/// that `GenerateConsoleCtrlEvent` can target without hitting this test.
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// `CTRL_BREAK_EVENT`. `CTRL_C_EVENT` cannot target a specific group, and a
/// process created with `CREATE_NEW_PROCESS_GROUP` has Ctrl+C disabled by
/// default, so Ctrl+Break is the deliverable chord — the same rung Claudine
/// sends its own children.
const CTRL_BREAK_EVENT: u32 = 1;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GenerateConsoleCtrlEvent(dwCtrlEvent: u32, dwProcessGroupId: u32) -> i32;
}

/// The parallel tasks the group launches. Named so a failure message can say
/// which child survived.
const TASKS: [&str; 3] = ["child-a", "child-b", "child-c"];

/// This task's heartbeat file, appended to about once a second while any part
/// of its process tree is alive.
fn heartbeat(workspace: &Path, task: &str) -> std::path::PathBuf {
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
/// `timeout: 300s` per task overrides the 30s `DEFAULT_COMMAND_TIMEOUT`.
/// Without it, a regression in which Ctrl+Break did nothing would still see the
/// children die — to the ordinary timeout — and the child-liveness assertion
/// would pass for the wrong reason.
fn sequence_document() -> String {
    let task_entries: String = TASKS
        .iter()
        .map(|name| {
            // `ping -n 2 127.0.0.1` is the portable `cmd` sleep, so the loop
            // ticks about once a second for up to 600s.
            format!(
                "        - name: {name}\n\
                 \x20         timeout: 300s\n\
                 \x20         shell: \"for /l %i in (1,1,600) do @(echo tick >> {name}.live & ping -n 2 127.0.0.1 > nul)\"\n"
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

/// Wait until every task has emitted its first heartbeat, so every child is
/// running — strictly after the orchestrator registered its console handler and
/// after each task's poll loop began reading the shared flag. Injecting earlier
/// would race the handler installation.
fn await_all_tasks_running(workspace: &Path) {
    let deadline = Instant::now() + Duration::from_secs(60);
    for name in TASKS {
        let marker = heartbeat(workspace, name);
        loop {
            assert!(Instant::now() < deadline, "task {name} never started");
            if fs::metadata(&marker).is_ok_and(|m| m.len() > 0) {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

#[test]
fn sequence_ctrl_c_fans_out_to_parallel_children_on_windows() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    common::wrap::seed_minimal_config(workspace.path());
    write_fake_goose(&path_dir);

    let md_file = workspace.path().join("seq.md");
    fs::write(&md_file, sequence_document()).unwrap();

    let path = format!(
        "{};{}",
        path_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let mut claudine = Command::new(env!("CARGO_BIN_EXE_claudine"))
        .args(["sequence", "--goose", "--yolo"])
        .arg(&md_file)
        .current_dir(workspace.path())
        .env("PATH", path)
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("USERPROFILE", workspace.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .expect("spawn claudine sequence");

    await_all_tasks_running(workspace.path());

    let sent = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, claudine.id()) };
    assert_ne!(sent, 0, "GenerateConsoleCtrlEvent failed");

    // The shell regains control: Claudine must return on its own well inside
    // the tasks' 300s timeout. A hang here is the failure mode the whole
    // coordinator exists to prevent, so it is bounded rather than awaited.
    let exit_deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = claudine.try_wait().expect("poll claudine") {
            break status;
        }
        if Instant::now() >= exit_deadline {
            let _ = claudine.kill();
            panic!("claudine did not exit after CTRL_BREAK_EVENT");
        }
        std::thread::sleep(Duration::from_millis(200));
    };

    // Every task tree stopped: two heartbeat samples three seconds apart (the
    // loop ticks about once a second) must be identical.
    let ticks = |name: &str| {
        fs::metadata(heartbeat(workspace.path(), name))
            .expect("heartbeat file")
            .len()
    };
    let before: Vec<u64> = TASKS.iter().map(|name| ticks(name)).collect();
    std::thread::sleep(Duration::from_secs(3));
    for (name, was) in TASKS.iter().zip(before) {
        assert_eq!(
            ticks(name),
            was,
            "task {name} was still running after the interrupt"
        );
    }

    assert!(
        !workspace.path().join("later-step-ran.txt").exists(),
        "the step after the interrupted group ran",
    );

    assert_eq!(
        status.code(),
        Some(130),
        "interrupted sequence must exit 130",
    );
}
