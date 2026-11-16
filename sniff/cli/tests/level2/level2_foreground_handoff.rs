//! `sniff::process::ForegroundHandoff` against a real controlling terminal.
//!
//! The library's unit tests script the guard's terminal calls; only a real
//! terminal shows that a child in its own process group actually becomes the
//! foreground group, reads the keyboard, and that the caller gets the
//! foreground and its terminal settings back afterwards. Each test opens its
//! own detached tmux pane running an interactive `sh`, which runs
//! `foreground_handoff_fixture` in one mode; the fixture records its
//! observations to files in a fresh marker directory (see the fixture's module
//! docs for the format). Nothing takes window focus, and nothing is typed
//! outside the pane.
//!
//! Unix only: native Windows has no foreground group, and its fixture path is
//! a compile-only control.

#![cfg(unix)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use tempfile::TempDir;
use test_toolkit::{Backend, Level, require_level};

const WAIT: Duration = Duration::from_secs(10);
const TYPED: &str = "typed-into-the-child";

/// A detached pane running an interactive `sh` (job control on, so the
/// fixture runs as its own foreground job) in a fresh marker directory. The
/// directory outlives the pane: the harness is dropped first.
struct Pane {
    harness: TmuxHarness,
    dir: TempDir,
}

impl Pane {
    fn open() -> Self {
        let dir = tempfile::tempdir().expect("create the marker directory");
        let mut harness = TmuxHarness::new();
        let home = format!("HOME={}", dir.path().display());
        harness
            .spawn_program(
                "/usr/bin/env",
                &[
                    "-i",
                    "PATH=/usr/bin:/bin",
                    &home,
                    "TERM=xterm-256color",
                    "PS1=$ ",
                    "sh",
                    "-i",
                ],
            )
            .expect("spawn the shell");
        let mut pane = Self { harness, dir };
        let line = format!("cd '{}'; echo in''place", pane.dir.path().display());
        pane.type_line(&line);
        pane.wait_for_line("inplace");
        pane
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn type_line(&mut self, line: &str) {
        self.harness.send_text(line.as_bytes()).expect("type the line");
        self.harness.send_key("Enter").expect("press Enter");
    }

    /// Runs the fixture in `mode`, recording the shell's PID and its terminal
    /// settings around it, then prints a marker once the shell has the
    /// terminal back.
    fn start_fixture(&mut self, mode: &str) {
        let fixture = biscuit_test_harness::bin_exe!("foreground_handoff_fixture");
        let line = format!(
            "echo $$ > shell.pid; stty -a > shell.before; '{}' {mode} .; echo $? > status; \
             stty -a > shell.after; echo back''home",
            fixture.display()
        );
        self.type_line(&line);
    }

    /// Waits until a pane line ends with `expected`.
    fn wait_for_line(&mut self, expected: &str) {
        let deadline = Instant::now() + WAIT;
        let mut last = String::new();
        while Instant::now() < deadline {
            last = self.harness.capture().expect("capture").plain;
            if last.lines().any(|line| line.trim_end().ends_with(expected)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("no line ended with `{expected}` after {WAIT:?}. Pane:\n{last}");
    }

    fn wait_for_file(&mut self, name: &str) {
        let path = self.path(name);
        let deadline = Instant::now() + WAIT;
        while !path.exists() {
            if Instant::now() >= deadline {
                let pane = self.harness.capture().expect("capture").plain;
                panic!("{} never appeared. Pane:\n{pane}", path.display());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn read(&self, name: &str) -> String {
        let path = self.path(name);
        std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    }

    /// The fixture's `key=value` observations.
    fn results(&self) -> HashMap<String, String> {
        self.read("results")
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    /// `stty -a` settings without `pendin`, which the kernel sets and clears
    /// itself while input is pending, so it is not a setting anyone restores.
    fn settings(&self, name: &str) -> Vec<String> {
        self.read(name)
            .split_whitespace()
            .filter(|token| token.trim_start_matches('-') != "pendin")
            .map(str::to_string)
            .collect()
    }

    /// The idle shell's `(pgid, tpgid)`, read from outside the pane so no job
    /// of the shell's own holds the foreground while it is observed.
    fn shell_groups(&self) -> Groups {
        let pid = self.read("shell.pid");
        let output = Command::new("ps")
            .args(["-o", "pgid=", "-o", "tpgid=", "-p", pid.trim()])
            .output()
            .expect("run ps");
        Groups::parse(&String::from_utf8_lossy(&output.stdout))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Groups {
    pgid: i64,
    tpgid: i64,
}

impl Groups {
    fn parse(text: &str) -> Self {
        let numbers: Vec<i64> = text
            .split_whitespace()
            .map(|number| number.parse().unwrap_or_else(|_| panic!("not a group: {text:?}")))
            .collect();
        let [pgid, tpgid] = numbers[..] else {
            panic!("expected `pgid tpgid`, got {text:?}");
        };
        Self { pgid, tpgid }
    }

    fn owns_foreground(self) -> bool {
        self.pgid == self.tpgid
    }
}

/// Starts `mode`, lets `interact` drive the pane while the fixture runs, and
/// returns the pane once the shell has the terminal back.
fn run_mode(mode: &str, interact: impl FnOnce(&mut Pane)) -> (Pane, HashMap<String, String>) {
    let mut pane = Pane::open();
    pane.start_fixture(mode);
    interact(&mut pane);
    pane.wait_for_line("backhome");
    let results = pane.results();
    (pane, results)
}

fn group(results: &HashMap<String, String>, key: &str) -> Groups {
    Groups::parse(results.get(key).unwrap_or_else(|| panic!("no `{key}` in {results:?}")))
}

/// The fixture held the foreground before the handoff and after restoration,
/// with the same terminal settings, and the shell has the foreground again
/// with its own settings unchanged. Whether restoration was an explicit call
/// or the guard's drop is each test's own assertion.
fn assert_host_restored(pane: &Pane, results: &HashMap<String, String>) {
    assert_eq!(pane.read("status").trim(), "0", "fixture exit status; results: {results:?}");
    assert_eq!(results.get("done").map(String::as_str), Some("1"), "{results:?}");
    assert_eq!(results.get("active").map(String::as_str), Some("true"), "{results:?}");
    let before = group(results, "host_before");
    assert!(before.owns_foreground(), "the fixture started in the foreground: {results:?}");
    assert_eq!(group(results, "host_after"), before, "foreground restored: {results:?}");
    assert_eq!(
        pane.settings("stty.before"),
        pane.settings("stty.after"),
        "the fixture's terminal settings were restored"
    );
    assert!(pane.shell_groups().owns_foreground(), "the shell has the foreground back");
    assert_eq!(
        pane.settings("shell.before"),
        pane.settings("shell.after"),
        "the shell's terminal settings are unchanged"
    );
}

/// The fixture called `restore()` and it succeeded.
fn assert_restore_called(results: &HashMap<String, String>) {
    assert_eq!(results.get("restore").map(String::as_str), Some("ok"), "{results:?}");
}

/// The child led its own group, and that group was the foreground group.
fn assert_child_had_foreground(pane: &Pane, results: &HashMap<String, String>) {
    let child = Groups::parse(&pane.read("child.fg"));
    assert!(child.owns_foreground(), "the child owned the foreground: {child:?}");
    assert_ne!(child.pgid, group(results, "host_before").pgid, "the child had its own group");
}

#[test]
fn level2_foreground_handoff_child_reads_the_terminal_then_the_host_gets_it_back() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (pane, results) = run_mode("success", |pane| {
        pane.wait_for_file("child.ready");
        pane.type_line(TYPED);
    });

    assert_eq!(results.get("child_exit").map(String::as_str), Some("0"), "{results:?}");
    assert_eq!(pane.read("child.line").trim(), TYPED, "the child read the typed line");
    assert_child_had_foreground(&pane, &results);
    assert_restore_called(&results);
    assert_host_restored(&pane, &results);
}

#[test]
fn level2_foreground_handoff_nonzero_exit_still_restores_the_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (pane, results) = run_mode("nonzero", |_| {});

    assert_eq!(results.get("child_exit").map(String::as_str), Some("7"), "{results:?}");
    assert!(pane.path("child.ready").exists(), "the child changed the settings");
    assert_child_had_foreground(&pane, &results);
    assert_restore_called(&results);
    assert_host_restored(&pane, &results);
}

#[test]
fn level2_foreground_handoff_failed_spawn_after_the_transfer_restores_the_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (pane, results) = run_mode("spawn-error", |_| {});

    let spawn = results.get("spawn").map(String::as_str).unwrap_or_default();
    assert!(spawn.starts_with("error:"), "the spawn failed: {results:?}");
    assert!(!pane.path("child.fg").exists(), "no child program ran");
    let host = group(&results, "host_before");
    assert_ne!(
        group(&results, "pre_restore").tpgid,
        host.pgid,
        "the failed child had taken the foreground before restoration: {results:?}"
    );
    assert_restore_called(&results);
    assert_host_restored(&pane, &results);
}

#[test]
fn level2_foreground_handoff_refuses_a_child_without_its_own_group() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (pane, results) = run_mode("unconfigured", |_| {});

    assert_eq!(results.get("spawn").map(String::as_str), Some("error:InvalidInput"), "{results:?}");
    assert!(!pane.path("child.fg").exists(), "no child program ran");
    assert_eq!(
        group(&results, "pre_restore"),
        group(&results, "host_before"),
        "the foreground never left the host: {results:?}"
    );
    assert_restore_called(&results);
    assert_host_restored(&pane, &results);
}

#[test]
fn level2_foreground_handoff_unwind_stops_the_tree_then_restores_the_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (pane, results) = run_mode("unwind", |_| {});

    assert_eq!(results.get("ready").map(String::as_str), Some("true"), "{results:?}");
    assert_eq!(results.get("unwind").map(String::as_str), Some("caught"), "{results:?}");
    assert_eq!(
        results.get("panic").map(String::as_str),
        Some("deliberate panic while the child owns the terminal"),
        "the fixture's own panic, not a failed setup step: {results:?}"
    );
    assert_child_had_foreground(&pane, &results);
    assert_eq!(
        group(&results, "host_unwound"),
        group(&results, "host_before"),
        "the foreground was back with the host when the panic was caught: {results:?}"
    );
    let exit = results.get("child_exit").map(String::as_str).unwrap_or_default();
    assert!(exit.starts_with("signal:"), "the tree's cleanup killed the child: {results:?}");
    assert_eq!(results.get("grandchild").map(String::as_str), Some("gone"), "{results:?}");
    assert!(!results.contains_key("restore"), "the guard's drop restored, not a call: {results:?}");
    assert_host_restored(&pane, &results);
}
