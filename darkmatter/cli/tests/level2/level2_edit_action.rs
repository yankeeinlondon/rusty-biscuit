//! The lifecycle `edit` action handing a real terminal to a terminal editor.
//!
//! An interactive shell in a detached tmux pane runs `md compose`; the
//! `darkmatter-fake-editor` fixture, installed as `vim`, is the terminal
//! editor. No window takes focus, and nothing is typed outside the pane.
//!
//! L1 can only prove the refusals, because a spawned `md` has no terminal.
//! These tests prove the handoff itself: the editor reads a line typed into
//! the pane, the next action runs, and the shell takes input again; and an
//! interrupt to `md` while the editor is open reaps the editor's process tree
//! and gives the terminal back. Unix only: native Windows shares the console
//! and has no foreground handoff, and its process-tree ownership is proven by
//! the L1 launcher tests.

#![cfg(unix)]

use crate::common;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use common::CliProcessFixture;
use common::level2::{link_or_copy, md_bin};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use test_toolkit::{Backend, Level, require_level};

const WAIT: Duration = Duration::from_secs(20);

/// An interactive shell in a fresh fixture whose `bin` holds `md` and the
/// fake editor as `vim`. The fixture owns the shell's home and outlives the
/// pane.
fn open_shell(prefix: &str, document: &str) -> (CliProcessFixture, TmuxHarness) {
    let fixture = CliProcessFixture::named(prefix);
    std::fs::write(fixture.cwd().join("doc.md"), document).expect("write the document");
    link_or_copy(md_bin(), &fixture.bin_dir().join("md")).expect("link md");
    let editor = biscuit_test_harness::bin_exe!("darkmatter-fake-editor");
    link_or_copy(&editor, &fixture.bin_dir().join("vim")).expect("link the fake editor");

    let path = format!("PATH={}:/usr/bin:/bin", fixture.bin_dir().display());
    let home = format!("HOME={}", fixture.home().display());
    let mut harness = TmuxHarness::new();
    harness
        .spawn_program("/usr/bin/env", &["-i", &path, &home, "TERM=xterm-256color", "LANG=C.UTF-8", "PS1=$ ", "sh", "-i"])
        .expect("spawn the shell");
    run_line(&mut harness, &format!("cd '{}'; echo in''place", fixture.cwd().display()), "inplace");
    (fixture, harness)
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

fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + WAIT;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{} never appeared", path.display());
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

/// The terminal settings before and after `md`, and its exit status, each in
/// a file; then a marker line once the shell has its terminal back.
fn compose_line(environment: &str) -> String {
    format!(
        "stty -a > before.stty; {environment} sh -c 'echo $$ > md.pid; exec md compose doc.md'; \
         echo $? > status; stty -a > after.stty; echo back''home"
    )
}

#[test]
fn level2_edit_action_a_terminal_editor_reads_the_pane_then_the_stack_continues() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (fixture, mut harness) = open_shell(
        "level2-edit-handoff",
        "---\ncomposed:\n  - edit: note.md\n  - info: \"after-edit\"\n---\nthe-document\n",
    );
    let cwd = fixture.cwd().to_path_buf();
    let environment = "EDITOR=vim FAKE_EDITOR_RECORD=record FAKE_EDITOR_READ_LINE=1 FAKE_EDITOR_STDIO=stdio.json";
    harness.send_text(compose_line(environment).as_bytes()).expect("type the line");
    harness.send_key("Enter").expect("press Enter");

    // The editor has started once its argument record exists; the line goes
    // to the terminal it now owns.
    wait_for_file(&cwd.join("record"));
    run_line(&mut harness, "typed-into-the-editor", "backhome");

    assert_eq!(read(cwd.join("status")).trim(), "0", "md succeeded");
    assert_eq!(read(cwd.join("note.md")), "typed-into-the-editor", "the editor read the pane");
    let stdio = test_toolkit::fake_editor::read_stdio_report(&cwd.join("stdio.json")).unwrap();
    assert!(stdio.stdin_tty && stdio.stdout_tty && stdio.stderr_tty, "{stdio:?}");
    let pane = harness.capture().expect("capture").plain;
    let after = pane.find("after-edit").unwrap_or_else(|| panic!("the next action ran:\n{pane}"));
    assert!(pane[after..].contains("the-document"), "the document follows the stack:\n{pane}");
    assert_eq!(settings(cwd.join("before.stty")), settings(cwd.join("after.stty")), "terminal settings restored");
    run_line(&mut harness, "echo still-$((1+1))", "still-2");
}

#[test]
fn level2_edit_action_an_interrupt_reaps_the_open_editor_and_returns_the_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (fixture, mut harness) = open_shell(
        "level2-edit-cancel",
        "---\ncomposed:\n  - edit:\n      file: note.md\n      no_error: true\n  - info: \"after-edit\"\n\
         cancelled:\n  - info: \"event-cancelled\"\n---\nthe-document\n",
    );
    let cwd = fixture.cwd().to_path_buf();
    let environment = "EDITOR=vim FAKE_EDITOR_SPAWN_CHILD=1 FAKE_EDITOR_PIDS=pids FAKE_EDITOR_HOLD=held";
    harness.send_text(compose_line(environment).as_bytes()).expect("type the line");
    harness.send_key("Enter").expect("press Enter");
    wait_for_file(&cwd.join("held"));
    let child: u32 = read(cwd.join("pids")).trim().parse().expect("the child's PID");
    let md: i32 = read(cwd.join("md.pid")).trim().parse().expect("md's PID");

    // The interrupt goes to `md` itself: the editor owns the terminal's
    // foreground, so a typed Ctrl-C would reach the editor instead.
    // SAFETY: plain `kill(2)` on the PID this test's pane started.
    unsafe {
        libc::kill(md, libc::SIGINT);
    }
    let pane = wait_for_pane(&mut harness, "backhome");

    assert_ne!(read(cwd.join("status")).trim(), "0", "a cancelled run fails:\n{pane}");
    assert!(pane.contains("event-cancelled"), "the run is reported as cancelled:\n{pane}");
    assert!(!pane.contains("after-edit"), "`no_error` never resumes a cancelled edit:\n{pane}");
    assert!(cwd.join("note.md").is_file(), "the file the action created is kept");
    let deadline = Instant::now() + WAIT;
    // SAFETY: signal 0 only checks that the PID exists.
    while unsafe { libc::kill(child as libc::pid_t, 0) } == 0 {
        assert!(Instant::now() < deadline, "the editor's child {child} survived");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(settings(cwd.join("before.stty")), settings(cwd.join("after.stty")), "terminal settings restored");
    run_line(&mut harness, "echo still-$((2+2))", "still-4");
}
