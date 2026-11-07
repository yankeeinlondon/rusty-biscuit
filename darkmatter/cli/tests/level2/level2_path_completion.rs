//! Real Tab presses in real shells: `style.pa<Tab>` through `ce<Tab>` to
//! `style.page.alignment=center`, then Enter.
//!
//! The Level-1 tests prove the words `md` receives and prints. Only a shell can
//! prove what it does with them: whether readline replaces the right piece, and
//! whether a space lands after a word that was meant to continue. Each test owns a
//! detached tmux session with a fixed environment, so no window takes focus and no
//! developer configuration reaches the shell.
//!
//! A stray space at any step shows up in the finished line, which is asserted in
//! full. A shell this host does not have reports itself unproven; the implementation
//! log records which ran.

use crate::common;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use common::CliProcessFixture;
use common::level2::{link_or_copy, md_bin};
use std::process::Command;
use std::time::{Duration, Instant};
use test_toolkit::{Backend, Level, require_level};

const DOCUMENT: &str = "---\nstyle:\n  page:\n    alignment: left\n---\nAlign: {{ style.page.alignment }}!\n";
const WAIT: Duration = Duration::from_secs(20);

/// Waits until a line of the pane ends with `expected`. `capture-pane` trims trailing
/// spaces, so this alone cannot see a stray one; the finished line can.
fn wait_for_line(harness: &mut TmuxHarness, expected: &str) {
    let deadline = Instant::now() + WAIT;
    let mut last = String::new();
    while Instant::now() < deadline {
        last = harness.capture().expect("capture").plain;
        if last.lines().any(|line| line.trim_end().ends_with(expected)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("no line ended with `{expected}` after {WAIT:?}. Pane:\n{last}");
}

fn type_text(harness: &mut TmuxHarness, text: &str) {
    harness.send_text(text.as_bytes()).expect("send text");
}

fn press_tab(harness: &mut TmuxHarness) {
    harness.send_key("Tab").expect("send Tab");
}

fn press_enter(harness: &mut TmuxHarness) {
    harness.send_key("Enter").expect("send Enter");
}

/// Runs one line at the shell's prompt and waits until it has finished.
fn run_line(harness: &mut TmuxHarness, line: &str, ready: &str) {
    type_text(harness, line);
    press_enter(harness);
    wait_for_line(harness, ready);
}

/// The Tab sequence, in `program`, loading `md`'s script with `setup`.
fn assert_round_trip(program: &str, flags: &[&str], setup: &str) {
    let fixture = CliProcessFixture::named("level2-path-completion");
    std::fs::write(fixture.cwd().join("doc.md"), DOCUMENT).expect("write document");
    let md = fixture.bin_dir().join("md");
    link_or_copy(md_bin(), &md).expect("link md");

    let path = format!("PATH={}:/usr/bin:/bin", fixture.bin_dir().display());
    let home = format!("HOME={}", fixture.home().display());
    let mut arguments = vec!["-i", &path, &home, "TERM=xterm-256color", "LANG=C.UTF-8", program];
    arguments.extend_from_slice(flags);

    let mut harness = TmuxHarness::new();
    harness.spawn_program("/usr/bin/env", &arguments).expect("spawn shell");
    run_line(&mut harness, &format!("cd '{}'; printf 'in%s\\n' place", fixture.cwd().display()), "inplace");
    run_line(&mut harness, &format!("{setup}; printf 'set%s\\n' up"), "setup");

    type_text(&mut harness, "md compose doc.md style.pa");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md style.page");
    type_text(&mut harness, ".");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md style.page.");
    type_text(&mut harness, "al");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md style.page.alignment=");
    type_text(&mut harness, "ce");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md style.page.alignment=center");

    type_text(&mut harness, "zz=1");
    wait_for_line(&mut harness, "md compose doc.md style.page.alignment=center zz=1");
    press_enter(&mut harness);
    wait_for_line(&mut harness, "Align: center!");
}

fn bash_major(program: &str) -> Option<u32> {
    let output = Command::new(program).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let version = text.split("version ").nth(1)?;
    version.split('.').next()?.parse().ok()
}

/// The first bash on the usual paths whose major version `accepts`.
fn find_bash(accepts: impl Fn(u32) -> bool) -> Option<&'static str> {
    ["/opt/homebrew/bin/bash", "/usr/local/bin/bash", "/usr/bin/bash", "/bin/bash"]
        .into_iter()
        .find(|program| bash_major(program).is_some_and(&accepts))
}

const BASH_SETUP: &str = "eval \"$(COMPLETE=bash md)\"";
const BASH_FLAGS: &[&str] = &["--noprofile", "--norc", "-i"];

#[test]
fn a_tab_round_trip_in_modern_bash() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let Some(bash) = find_bash(|major| major >= 4) else {
        eprintln!("unproven: no bash 4 or later on this host");
        return;
    };
    assert_round_trip(bash, BASH_FLAGS, BASH_SETUP);
}

/// Bash 3.2 keeps `x=ce` as one word and has no `compopt`, which is why it has its
/// own test; in practice it is the `/bin/bash` of every Mac.
#[test]
fn a_tab_round_trip_in_bash_3() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let Some(bash) = find_bash(|major| major == 3) else {
        eprintln!("unproven: no bash 3 on this host");
        return;
    };
    assert_round_trip(bash, BASH_FLAGS, BASH_SETUP);
}

fn installed(program: &str) -> Option<String> {
    ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"]
        .into_iter()
        .map(|directory| format!("{directory}/{program}"))
        .find(|candidate| std::path::Path::new(candidate).exists())
}

#[test]
fn a_tab_round_trip_in_zsh() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let Some(zsh) = installed("zsh") else {
        eprintln!("unproven: zsh is not installed on this host");
        return;
    };
    assert_round_trip(&zsh, &["-f", "-i"], "autoload -Uz compinit; compinit -u -D; eval \"$(COMPLETE=zsh md)\"");
}

#[test]
fn a_tab_round_trip_in_fish() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let Some(fish) = installed("fish") else {
        eprintln!("unproven: fish is not installed on this host");
        return;
    };
    assert_round_trip(&fish, &["--no-config", "-i"], "COMPLETE=fish md | source");
}
