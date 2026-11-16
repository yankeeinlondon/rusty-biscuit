//! A lifecycle `shell` command taking the terminal's foreground from the
//! interactive shell of a real (tmux) terminal, and giving it back.
//!
//! An interactive shell in a detached tmux pane runs `claudine compose`; each
//! document's `start` stack runs a script from the workspace that records the
//! process group it runs in beside the terminal's foreground group (`ps -o
//! pgid= -o tpgid=`), so "the command held the foreground" is observed from
//! inside it rather than inferred from the absence of a hang. A fake `goose`
//! on `PATH` is the provider and records that it ran. Nothing is typed outside
//! the pane, and no window takes focus.
//!
//! The L1 PTY tests (`level1_lifecycle_shell_terminal_pty.rs`) prove a
//! terminal-using command completes and a command ending by `SIGINT` cancels
//! the run. These prove what needs a shell with job control and a typed key:
//! the command reads a line typed into the pane, a typed Ctrl-C reaches the
//! command and not Claudine, nested runs hand the foreground down and back one
//! level at a time, a background `claudine` leaves the shell's foreground
//! alone, and afterwards the shell owns the terminal with its settings intact.
//! Unix only: native Windows shares the console and has no foreground group.

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
const TYPED: &str = "typed-into-the-command";

/// Every PID file a scenario may write, killed on drop so a failed assertion
/// never leaks a `sleep 60` or a stuck `claudine`.
const PID_FILES: &[&str] = &["claudine.pid", "root.pid", "bg.pid"];

struct Reaper(PathBuf);

impl Drop for Reaper {
    fn drop(&mut self) {
        for name in PID_FILES {
            let Ok(text) = std::fs::read_to_string(self.0.join(name)) else {
                continue;
            };
            let Ok(pid) = text.trim().parse::<libc::pid_t>() else {
                continue;
            };
            // SAFETY: plain `kill(2)` on a PID this test's pane started.
            unsafe {
                if libc::kill(pid, 0) == 0 {
                    libc::kill(pid, libc::SIGKILL);
                }
            }
        }
    }
}

/// A pane running an interactive `sh` whose home and working directory are a
/// fresh workspace holding `doc.md` and the given scripts, and whose `PATH`
/// starts with a `bin` holding `claudine` and a fake `goose`.
struct Pane {
    harness: TmuxHarness,
    _reaper: Reaper,
    workspace: TempDir,
}

impl Pane {
    fn open(document: &str, scripts: &[(&str, &str)]) -> Self {
        let workspace = tempfile::tempdir().expect("create the workspace");
        let root = workspace.path().to_path_buf();
        let cwd = root.as_path();
        let bin = cwd.join("bin");
        std::fs::create_dir_all(&bin).expect("create bin");
        seed_minimal_config(cwd);
        std::fs::write(cwd.join("doc.md"), document).expect("write the document");
        for (name, body) in scripts {
            write_executable(&cwd.join(name), body);
        }
        std::os::unix::fs::symlink(common::claudine_bin(), bin.join("claudine"))
            .expect("link claudine");
        write_executable(&bin.join("goose"), "#!/bin/sh\n: > provider-ran\nexit 0\n");

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
        let mut pane = Self {
            harness,
            _reaper: Reaper(cwd.to_path_buf()),
            workspace,
        };
        pane.run_line(
            &format!("cd '{}'; echo $$ > shell.pid; echo in''place", cwd.display()),
            "inplace",
        );
        pane
    }

    fn cwd(&self) -> &Path {
        self.workspace.path()
    }

    fn type_line(&mut self, line: &str) {
        self.harness.send_text(line.as_bytes()).expect("type the line");
        self.harness.send_key("Enter").expect("press Enter");
    }

    fn run_line(&mut self, line: &str, ready: &str) -> String {
        self.type_line(line);
        self.wait_for_pane(ready)
    }

    /// Waits until a pane line ends with `expected`.
    fn wait_for_pane(&mut self, expected: &str) -> String {
        let deadline = Instant::now() + WAIT;
        let mut last = String::new();
        while Instant::now() < deadline {
            last = self.harness.capture().expect("capture").plain;
            if last.lines().any(|line| line.trim_end().ends_with(expected)) {
                return last;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("no line ended with `{expected}` after {WAIT:?}. Pane:\n{last}");
    }

    /// Waits until the workspace file `name` exists; the pane is in the panic,
    /// since a run that never gets there says why on it.
    fn wait_for_file(&mut self, name: &str) {
        let path = self.cwd().join(name);
        let deadline = Instant::now() + WAIT;
        while !path.exists() {
            if Instant::now() >= deadline {
                let pane = self.harness.capture().expect("capture").plain;
                panic!("{name} never appeared. Pane:\n{pane}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Starts `claudine compose --goose {args} doc.md` from the pane, with
    /// the terminal settings before and after it and its exit status each in a
    /// file, then a marker line once the shell has its terminal back. `sh -c
    /// … exec` records `claudine`'s own PID. `redirect` follows the `sh -c`.
    fn start_compose(&mut self, args: &str, redirect: &str) {
        let line = format!(
            "stty -a > before.stty; sh -c 'echo $$ > claudine.pid; exec claudine compose --goose {args} doc.md'{redirect}; \
             echo $? > status; stty -a > after.stty; echo back''home"
        );
        self.type_line(&line);
    }

    fn read(&self, name: &str) -> String {
        read(&self.cwd().join(name))
    }

    fn pid(&self, name: &str) -> libc::pid_t {
        self.read(name)
            .trim()
            .parse()
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    /// The `pgid tpgid` pair a script recorded into `name`.
    fn groups(&self, name: &str) -> (libc::pid_t, libc::pid_t) {
        parse_groups(&self.read(name))
    }

    /// The run is over and the pane is back to normal: claudine exited with
    /// `status`, the shell is the terminal's foreground group again, its
    /// settings are the ones it had before, and it takes the next line.
    fn assert_shell_owns_the_terminal(&mut self, pane: &str, status: &str) {
        assert_eq!(self.read("status").trim(), status, "claudine's exit status:\n{pane}");
        let (pgid, tpgid) = ps_groups(self.pid("shell.pid"));
        assert_eq!(pgid, tpgid, "the pane's shell is the foreground group again:\n{pane}");
        assert_eq!(
            settings(&self.cwd().join("before.stty")),
            settings(&self.cwd().join("after.stty")),
            "terminal settings restored"
        );
        self.run_line("echo still-$((1+1))", "still-2");
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn parse_groups(text: &str) -> (libc::pid_t, libc::pid_t) {
    let numbers: Vec<libc::pid_t> = text
        .split_whitespace()
        .map(|word| word.parse().unwrap_or_else(|error| panic!("{text:?}: {error}")))
        .collect();
    assert_eq!(numbers.len(), 2, "a `pgid tpgid` pair: {text:?}");
    (numbers[0], numbers[1])
}

/// `pid`'s process group and its terminal's foreground group, as `ps` reports
/// them.
fn ps_groups(pid: libc::pid_t) -> (libc::pid_t, libc::pid_t) {
    let output = std::process::Command::new("ps")
        .args(["-o", "pgid=", "-o", "tpgid=", "-p", &pid.to_string()])
        .output()
        .expect("run ps");
    parse_groups(&String::from_utf8_lossy(&output.stdout))
}

/// `stty -a`'s settings without `pendin`, which the kernel sets and clears
/// itself while input is pending, so it is not a setting anyone restores.
fn settings(path: &Path) -> Vec<String> {
    read(path)
        .split_whitespace()
        .filter(|token| token.trim_start_matches('-') != "pendin")
        .map(str::to_string)
        .collect()
}

fn wait_dead(pid: libc::pid_t, what: &str) {
    let deadline = Instant::now() + WAIT;
    // SAFETY: signal 0 only checks that the PID exists.
    while unsafe { libc::kill(pid, 0) } == 0 {
        assert!(Instant::now() < deadline, "{what} ({pid}) survived");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Records the command's own and the foreground group, changes the
/// terminal's settings, and reads one typed line into `typed`.
const READ_TYPED_LINE: &str = "stty -echo\n\
    ps -o pgid= -o tpgid= -p $$ > command.fg\n\
    : > waiting\n\
    IFS= read -r line\n\
    printf '%s\\n' \"$line\" > typed\n";

/// A background descendant that ignores `SIGINT` and `SIGTERM`, so only the
/// owned tree's cleanup can stop it; `bg.pid` appears once it is ignoring them.
const STUBBORN_CHILD: &str = "set +m\n\
    sh -c 'trap \"\" INT TERM; echo $$ > bg.tmp; mv bg.tmp bg.pid; exec sleep 60' &\n\
    while [ ! -s bg.pid ]; do sleep 0.01; done\n";

/// Reads the typed line, leaves a stubborn child, then becomes a `sleep` that
/// only a signal ends.
fn interruptible_step() -> String {
    format!("#!/bin/sh\n{READ_TYPED_LINE}{STUBBORN_CHILD}echo $$ > root.pid\n: > ready\nexec sleep 60\n")
}

const INTERRUPTED_DOCUMENT: &str = "---\ntitle: t\nstart:\n  - shell:\n      command: \"./step.sh\"\n      no_error: true\n  - shell:\n      command: \"echo later >> events.log\"\n      no_error: true\n\
cancelled:\n  - shell: \"echo cancelled >> events.log\"\nfinalize:\n  - shell: \"echo finalize >> events.log\"\n---\nBody.\n";

/// The interrupted run's shared tail: the command read the typed line in the
/// foreground, its tree is gone, `cancelled` and `finalize` ran once each, the
/// tolerated `start` action after it did not, no provider launched, and the
/// shell has the terminal back with claudine's status 130.
fn assert_interrupted_run(pane: &mut Pane, transcript: &str) {
    assert_eq!(pane.read("typed").trim(), TYPED, "the command read the pane");
    let (pgid, tpgid) = pane.groups("command.fg");
    assert_eq!(pgid, tpgid, "the command held the foreground:\n{transcript}");
    wait_dead(pane.pid("root.pid"), "the command's root");
    wait_dead(pane.pid("bg.pid"), "the command's INT/TERM-ignoring child");
    assert_eq!(
        pane.read("events.log"),
        "cancelled\nfinalize\n",
        "cancelled and finalize ran once each and no later `start` action ran:\n{transcript}"
    );
    assert!(!pane.cwd().join("provider-ran").exists(), "an interrupted run launched no provider");
    pane.assert_shell_owns_the_terminal(transcript, "130");
}

#[test]
fn level2_lifecycle_foreground_handoff_typed_ctrl_c_reaches_the_command_and_cancels_the_run() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut pane = Pane::open(INTERRUPTED_DOCUMENT, &[("step.sh", interruptible_step().as_str())]);
    pane.start_compose("--yolo", "");
    pane.wait_for_file("waiting");
    pane.type_line(TYPED);
    pane.wait_for_file("ready");

    // The command's group owns the foreground, so the line discipline sends
    // this Ctrl-C's `SIGINT` to the command, never to claudine.
    pane.harness.send_key("C-c").expect("send C-c");
    let transcript = pane.wait_for_pane("backhome");

    assert_interrupted_run(&mut pane, &transcript);
}

#[test]
fn level2_lifecycle_foreground_handoff_sigint_to_claudine_cancels_the_foreground_command() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut pane = Pane::open(INTERRUPTED_DOCUMENT, &[("step.sh", interruptible_step().as_str())]);
    pane.start_compose("--yolo", "");
    pane.wait_for_file("waiting");
    pane.type_line(TYPED);
    pane.wait_for_file("ready");

    // SAFETY: plain `kill(2)` on the PID this test's pane started.
    unsafe {
        libc::kill(pane.pid("claudine.pid"), libc::SIGINT);
    }
    let transcript = pane.wait_for_pane("backhome");

    assert_interrupted_run(&mut pane, &transcript);
}

/// Reads the typed line in the foreground, leaves a background child, and
/// exits `code`.
fn completing_step(code: i32) -> String {
    format!("#!/bin/sh\n{READ_TYPED_LINE}sleep 60 &\necho $! > bg.pid\nexit {code}\n")
}

const COMPLETING_DOCUMENT: &str = "---\ntitle: t\nstart:\n  - shell: \"./step.sh\"\n  - shell: \"echo later >> events.log\"\n\
success:\n  - shell: \"echo success >> events.log\"\nfailure:\n  - shell: \"echo failure >> events.log\"\n\
blocked:\n  - shell: \"echo blocked >> events.log\"\nfinalize:\n  - shell: \"echo finalize >> events.log\"\n---\nBody.\n";

/// Runs `COMPLETING_DOCUMENT` with a step exiting `code`, types the line, and
/// returns the pane once the shell has its terminal back.
fn run_completing_step(code: i32) -> (Pane, String) {
    let mut pane = Pane::open(COMPLETING_DOCUMENT, &[("step.sh", completing_step(code).as_str())]);
    pane.start_compose("--yolo", "");
    pane.wait_for_file("waiting");
    pane.type_line(TYPED);
    let transcript = pane.wait_for_pane("backhome");
    assert_eq!(pane.read("typed").trim(), TYPED, "the command read the pane:\n{transcript}");
    let (pgid, tpgid) = pane.groups("command.fg");
    assert_eq!(pgid, tpgid, "the command held the foreground:\n{transcript}");
    wait_dead(pane.pid("bg.pid"), "the completed command's background child");
    (pane, transcript)
}

#[test]
fn level2_lifecycle_foreground_handoff_a_successful_command_returns_the_terminal_and_the_run_continues() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (mut pane, transcript) = run_completing_step(0);

    assert_eq!(
        pane.read("events.log"),
        "later\nsuccess\nfinalize\n",
        "the next `start` action and the provider's events ran:\n{transcript}"
    );
    assert!(pane.cwd().join("provider-ran").exists(), "the provider launched");
    pane.assert_shell_owns_the_terminal(&transcript, "0");
}

#[test]
fn level2_lifecycle_foreground_handoff_a_failing_command_returns_the_terminal_and_fails_the_run() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let (mut pane, transcript) = run_completing_step(3);

    assert_eq!(
        pane.read("events.log"),
        "failure\nfinalize\n",
        "the failed `start` action ended the event and failed the run before launch:\n{transcript}"
    );
    assert!(!pane.cwd().join("provider-ran").exists(), "a failed run launched no provider");
    pane.assert_shell_owns_the_terminal(&transcript, "1");
}

#[test]
fn level2_lifecycle_foreground_handoff_a_nested_claudine_hands_the_foreground_down_and_back() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let outer_step = "#!/bin/sh\n\
        ps -o pgid= -o tpgid= -p $$ > outer-before.fg\n\
        claudine compose --goose --yolo inner.md\n\
        echo $? > inner.status\n\
        ps -o pgid= -o tpgid= -p $$ > outer-after.fg\n";
    let inner_step = format!("#!/bin/sh\n{READ_TYPED_LINE}");
    let mut pane = Pane::open(
        "---\ntitle: outer\nstart:\n  - shell: \"./outer.sh\"\n---\nOuter.\n",
        &[("outer.sh", outer_step), ("inner.sh", inner_step.as_str())],
    );
    std::fs::write(
        pane.cwd().join("inner.md"),
        "---\ntitle: inner\nstart:\n  - shell: \"./inner.sh\"\n---\nInner.\n",
    )
    .expect("write the inner document");
    pane.start_compose("--yolo", "");
    pane.wait_for_file("waiting");
    pane.type_line(TYPED);
    let transcript = pane.wait_for_pane("backhome");

    assert_eq!(pane.read("typed").trim(), TYPED, "the inner command read the pane:\n{transcript}");
    assert_eq!(pane.read("inner.status").trim(), "0", "the inner claudine succeeded:\n{transcript}");
    let (outer, outer_foreground) = pane.groups("outer-before.fg");
    let (inner, inner_foreground) = pane.groups("command.fg");
    let (outer_after, outer_foreground_after) = pane.groups("outer-after.fg");
    assert_eq!(outer, outer_foreground, "the outer command held the foreground first");
    assert_ne!(inner, outer, "the inner command ran in a group of its own");
    assert_eq!(inner, inner_foreground, "the inner command held the foreground");
    assert_eq!(outer_after, outer);
    assert_eq!(
        outer_after, outer_foreground_after,
        "the inner claudine gave the foreground back to the outer command"
    );
    pane.assert_shell_owns_the_terminal(&transcript, "0");
}

#[test]
fn level2_lifecycle_foreground_handoff_a_structured_command_reads_the_typed_line() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let reader = format!("#!/bin/sh\nprintf '%s\\n' \"$1\" > argument\n{READ_TYPED_LINE}sleep 60 &\necho $! > bg.pid\n");
    let mut pane = Pane::open(
        "---\ntitle: t\ntag: structured\nstart:\n  - shell: \"reader {{ tag }}\"\n---\nBody.\n",
        &[],
    );
    write_executable(&pane.cwd().join("bin/reader"), &reader);
    // A shape approval alone, so the command runs as a filled native chain
    // rather than through the system shell.
    common::write_shell_permissions(pane.cwd(), "template reader {{ string }}\n");
    pane.start_compose("", "");
    pane.wait_for_file("waiting");
    pane.type_line(TYPED);
    let transcript = pane.wait_for_pane("backhome");

    assert_eq!(pane.read("argument").trim(), "structured", "the filled argument");
    assert_eq!(pane.read("typed").trim(), TYPED, "the command read the pane:\n{transcript}");
    let (pgid, tpgid) = pane.groups("command.fg");
    assert_eq!(pgid, tpgid, "the structured command held the foreground:\n{transcript}");
    wait_dead(pane.pid("bg.pid"), "the structured command's background child");
    pane.assert_shell_owns_the_terminal(&transcript, "0");
}

#[test]
fn level2_lifecycle_foreground_handoff_a_background_claudine_leaves_the_foreground_alone() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let step = "#!/bin/sh\nps -o pgid= -o tpgid= -p $$ > command.fg\nsleep 60 &\necho $! > bg.pid\n";
    let mut pane = Pane::open(
        "---\ntitle: t\nstart:\n  - shell: \"./step.sh\"\n---\nBody.\n",
        &[("step.sh", step)],
    );
    // The shell waits in the foreground while claudine runs as its job (`%1`,
    // since an interactive shell history-expands `$!`). Claudine's output goes
    // to a file: with stdout or stderr on the terminal, biscuit-terminal's
    // capability probe puts the terminal in raw mode, and a background job
    // doing that is stopped by `SIGTTOU` before any lifecycle command runs.
    // stdin stays the terminal, so the command still has it as its own.
    pane.start_compose("--yolo", " > claudine.log 2>&1 & wait %1");
    let transcript = pane.wait_for_pane("backhome");
    let log = pane.cwd().join("claudine.log");
    let log = std::fs::read_to_string(log).unwrap_or_default();
    assert_eq!(pane.read("status").trim(), "0", "claudine succeeded:\n{transcript}\n{log}");

    let (shell_group, _) = ps_groups(pane.pid("shell.pid"));
    let (command, foreground) = pane.groups("command.fg");
    assert_eq!(foreground, shell_group, "the shell kept the foreground:\n{transcript}");
    assert_ne!(command, foreground, "the command ran in the background:\n{transcript}");
    wait_dead(pane.pid("bg.pid"), "the command's background child");
    pane.assert_shell_owns_the_terminal(&transcript, "0");
}

#[test]
fn level2_lifecycle_foreground_handoff_a_command_reads_dev_tty_when_claudine_stdin_is_redirected() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let step = "#!/bin/sh\n\
        stty -echo < /dev/tty\n\
        ps -o pgid= -o tpgid= -p $$ > command.fg\n\
        : > waiting\n\
        IFS= read -r line < /dev/tty\n\
        printf '%s\\n' \"$line\" > typed\n";
    let mut pane = Pane::open(
        "---\ntitle: t\nstart:\n  - shell: \"./step.sh\"\n---\nBody.\n",
        &[("step.sh", step)],
    );
    pane.start_compose("--yolo", " < /dev/null");
    pane.wait_for_file("waiting");
    pane.type_line(TYPED);
    let transcript = pane.wait_for_pane("backhome");

    assert_eq!(pane.read("typed").trim(), TYPED, "the command read /dev/tty:\n{transcript}");
    let (pgid, tpgid) = pane.groups("command.fg");
    assert_eq!(pgid, tpgid, "the command held the foreground:\n{transcript}");
    pane.assert_shell_owns_the_terminal(&transcript, "0");
}
