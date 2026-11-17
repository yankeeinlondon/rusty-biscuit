//! The lifecycle `edit` action handing a real terminal to a terminal editor
//! from a Claudine event stack.
//!
//! An interactive shell in a detached tmux pane runs `claudine compose`; the
//! `claudine-fake-editor` fixture, installed as `vim`, is the terminal editor.
//! Each document's `initialize` stack edits a file and ends in `skip`, so no
//! provider launches (a fake `goose` on `PATH` records a run if one did). No
//! window takes focus, and nothing is typed outside the pane.
//!
//! L1 can only prove the refusals, because a spawned `claudine` has no
//! terminal. These tests prove the handoff itself: the editor reads a line
//! typed into the pane, the next action runs, and the shell takes input again;
//! and an interrupt to `claudine` while the editor is open reaps the editor's
//! process tree and gives the terminal back. Unix only: native Windows shares
//! the console and has no foreground handoff.

#![cfg(unix)]

use crate::common;
use common::wrap::seed_minimal_config;
use common::write_executable;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use test_toolkit::{Backend, Level, require_level};

const WAIT: Duration = Duration::from_secs(20);

/// An interactive shell whose home and working directory are a fresh
/// workspace holding `doc.md`, and whose `PATH` starts with a `bin` holding
/// `claudine`, the fake editor as `vim`, and a fake `goose`. The workspace outlives the
/// pane.
fn open_shell(document: &str) -> (TempDir, TmuxHarness) {
    let workspace = tempfile::tempdir().expect("create the workspace");
    let cwd = workspace.path();
    let bin = cwd.join("bin");
    std::fs::create_dir_all(&bin).expect("create bin");
    seed_minimal_config(cwd);
    std::fs::write(
        cwd.join("doc.md"),
        document.replace("@WS", &cwd.display().to_string()),
    )
    .expect("write the document");
    std::os::unix::fs::symlink(common::claudine_bin(), bin.join("claudine"))
        .expect("link claudine");
    let editor = biscuit_test_harness::bin_exe!("claudine-fake-editor");
    std::fs::copy(&editor, bin.join("vim")).expect("install the fake editor");
    write_executable(
        &bin.join("goose"),
        "#!/bin/sh\ncat > /dev/null\n: > provider-ran\n",
    );

    let path = format!("PATH={}:/usr/bin:/bin", bin.display());
    let home = format!("HOME={}", cwd.display());
    let mut harness = TmuxHarness::new();
    harness
        .spawn_program(
            "/usr/bin/env",
            &[
                "-i",
                &path,
                &home,
                "TERM=xterm-256color",
                "LANG=C.UTF-8",
                "NO_COLOR=1",
                "PS1=$ ",
                "sh",
                "-i",
            ],
        )
        .expect("spawn the shell");
    harness.resize(160, 50).expect("resize the pane");
    run_line(
        &mut harness,
        &format!("cd '{}'; echo in''place", cwd.display()),
        "inplace",
    );
    (workspace, harness)
}

fn run_line(harness: &mut TmuxHarness, line: &str, ready: &str) {
    harness.send_text(line.as_bytes()).expect("type the line");
    harness.send_key("Enter").expect("press Enter");
    wait_for_pane(harness, ready);
}

/// Waits until a pane line ends with `expected`.
fn wait_for_pane(harness: &mut TmuxHarness, expected: &str) -> String {
    let deadline = Instant::now() + WAIT;
    let mut last = String::new();
    while Instant::now() < deadline {
        last = harness.capture().expect("capture").plain;
        if last.lines().any(|line| line.trim_end().ends_with(expected)) {
            return last;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("no line ended with `{expected}` after {WAIT:?}. Pane:\n{last}");
}

/// Waits until `path` exists; the pane is in the panic, since a run that
/// never reaches the editor says why there.
fn wait_for_file(harness: &mut TmuxHarness, path: &Path) {
    let deadline = Instant::now() + WAIT;
    while !path.exists() {
        if Instant::now() >= deadline {
            let pane = harness.capture().expect("capture").plain;
            panic!("{} never appeared. Pane:\n{pane}", path.display());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// `stty -a`'s settings without `pendin`, which the kernel sets and clears
/// itself while input is pending, so it is not a setting anyone restores.
fn settings(path: PathBuf) -> Vec<String> {
    read(path)
        .split_whitespace()
        .filter(|token| token.trim_start_matches('-') != "pendin")
        .map(str::to_string)
        .collect()
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The terminal settings before and after `claudine`, and its exit status,
/// each in a file; then a marker line once the shell has its terminal back.
/// `sh -c … exec` records `claudine`'s own PID, so an interrupt reaches it
/// rather than the editor that owns the foreground.
fn compose_line(environment: &str) -> String {
    format!(
        "stty -a > before.stty; {environment} sh -c 'echo $$ > claudine.pid; exec claudine compose --goose doc.md'; \
         echo $? > status; stty -a > after.stty; echo back''home"
    )
}

#[test]
fn level2_edit_action_a_terminal_editor_reads_the_pane_then_the_stack_continues() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (workspace, mut harness) = open_shell(
        "---\ninitialize:\n  - edit: \"@WS/note.md\"\n  - info: \"after-edit\"\n  - skip\n---\nthe-document\n",
    );
    let cwd = workspace.path().to_path_buf();
    let environment =
        "EDITOR=vim FAKE_EDITOR_RECORD=record FAKE_EDITOR_READ_LINE=1 FAKE_EDITOR_STDIO=stdio.json";
    harness
        .send_text(compose_line(environment).as_bytes())
        .expect("type the line");
    harness.send_key("Enter").expect("press Enter");

    // The editor has started once its argument record exists; the line goes
    // to the terminal it now owns.
    wait_for_file(&mut harness, &cwd.join("record"));
    run_line(&mut harness, "typed-into-the-editor", "backhome");

    let pane = harness.capture().expect("capture").plain;
    assert_eq!(
        read(cwd.join("status")).trim(),
        "0",
        "claudine succeeded:\n{pane}"
    );
    assert_eq!(
        read(cwd.join("note.md")),
        "typed-into-the-editor",
        "the editor read the pane"
    );
    let stdio = test_toolkit::fake_editor::read_stdio_report(&cwd.join("stdio.json")).unwrap();
    assert!(
        stdio.stdin_tty && stdio.stdout_tty && stdio.stderr_tty,
        "{stdio:?}"
    );
    let typed = pane
        .find("typed-into-the-editor")
        .expect("the typed line is on the pane");
    assert!(
        pane[typed..].contains("after-edit"),
        "the next action ran after the editor:\n{pane}"
    );
    assert!(
        !cwd.join("provider-ran").exists(),
        "`skip` launched no provider"
    );
    assert_eq!(
        settings(cwd.join("before.stty")),
        settings(cwd.join("after.stty")),
        "terminal settings restored"
    );
    run_line(&mut harness, "echo still-$((1+1))", "still-2");
}

#[test]
fn level2_edit_action_an_interrupt_reaps_the_open_editor_and_returns_the_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (workspace, mut harness) = open_shell(
        "---\ninitialize:\n  - edit:\n      file: \"@WS/note.md\"\n      no_error: true\n  - info: \"after-edit\"\n  - skip\n\
         cancelled:\n  - info: \"event-cancelled\"\n---\nthe-document\n",
    );
    let cwd = workspace.path().to_path_buf();
    let environment =
        "EDITOR=vim FAKE_EDITOR_SPAWN_CHILD=1 FAKE_EDITOR_PIDS=pids FAKE_EDITOR_HOLD=held";
    harness
        .send_text(compose_line(environment).as_bytes())
        .expect("type the line");
    harness.send_key("Enter").expect("press Enter");
    wait_for_file(&mut harness, &cwd.join("held"));
    let child: libc::pid_t = read(cwd.join("pids"))
        .trim()
        .parse()
        .expect("the child's PID");
    let claudine: libc::pid_t = read(cwd.join("claudine.pid"))
        .trim()
        .parse()
        .expect("claudine's PID");

    // The interrupt goes to `claudine` itself: the editor owns the terminal's
    // foreground, so a typed Ctrl-C would reach the editor instead.
    // SAFETY: plain `kill(2)` on the PID this test's pane started.
    unsafe {
        libc::kill(claudine, libc::SIGINT);
    }
    let pane = wait_for_pane(&mut harness, "backhome");

    assert_ne!(
        read(cwd.join("status")).trim(),
        "0",
        "a cancelled run fails:\n{pane}"
    );
    assert!(
        pane.contains("event-cancelled"),
        "the run is reported as cancelled:\n{pane}"
    );
    assert!(
        !pane.contains("after-edit"),
        "`no_error` never resumes a cancelled edit:\n{pane}"
    );
    assert!(
        cwd.join("note.md").is_file(),
        "the file the action created is kept"
    );
    assert!(
        !cwd.join("provider-ran").exists(),
        "an interrupted run launched no provider"
    );
    let deadline = Instant::now() + WAIT;
    // SAFETY: signal 0 only checks that the PID exists.
    while unsafe { libc::kill(child, 0) } == 0 {
        assert!(
            Instant::now() < deadline,
            "the editor's child {child} survived"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        settings(cwd.join("before.stty")),
        settings(cwd.join("after.stty")),
        "terminal settings restored"
    );
    run_line(&mut harness, "echo still-$((2+2))", "still-4");
}
