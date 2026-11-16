//! Test-only binary that drives `sniff::process::ForegroundHandoff` against a
//! real controlling terminal.
//!
//! The Level 2 test `level2_foreground_handoff` runs it from an interactive
//! shell in a detached tmux pane. It calls only Sniff's public guard and tree
//! API, in the order the guard documents, and writes what it observed to files
//! in a marker directory, never to the terminal:
//!
//! ```text
//! foreground_handoff_fixture <mode> <marker-dir>
//! ```
//!
//! `<marker-dir>/results` holds one `key=value` line per observation. The
//! fixture's own process group and the terminal's foreground group are
//! `host_before` / `host_after` (`"<pgid> <tpgid>"`, from `ps`), and the
//! terminal settings are `stty -a` output in `stty.before` / `stty.after`.
//! A child script records its own groups in `child.fg` and signals readiness
//! with `child.ready` once it has changed the terminal settings.
//!
//! Modes:
//!
//! - `success`: the child reads one typed line into `child.line` and exits 0;
//! - `nonzero`: the child exits 7;
//! - `spawn-error`: a `pre_exec` hook after the handoff's fails, so the spawn
//!   fails after the child took the foreground;
//! - `unconfigured`: the command is not given its own process group, so the
//!   handoff refuses the spawn;
//! - `unwind`: a panic inside the guard's and tree's scope, caught outside it.
//!
//! Native Windows has no foreground group: every mode records the inactive
//! guard and spawns nothing.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Appends `key=value` lines to `<marker-dir>/results`, flushing each, so a
/// fixture that dies part-way leaves what it had observed.
struct Results {
    path: PathBuf,
}

impl Results {
    fn record(&self, key: &str, value: impl std::fmt::Display) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .unwrap_or_else(|error| panic!("open {}: {error}", self.path.display()));
        writeln!(file, "{key}={value}")
            .unwrap_or_else(|error| panic!("write {}: {error}", self.path.display()));
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [mode, dir] = args.as_slice() else {
        eprintln!("usage: foreground_handoff_fixture <mode> <marker-dir>");
        return ExitCode::from(2);
    };
    let dir = Path::new(dir);
    if let Err(error) = fs::create_dir_all(dir) {
        eprintln!("create {}: {error}", dir.display());
        return ExitCode::from(2);
    }
    let results = Results {
        path: dir.join("results"),
    };
    let Some(mode) = Mode::parse(mode) else {
        eprintln!("unknown mode `{mode}`");
        return ExitCode::from(2);
    };
    platform::run(mode, dir, &results);
    results.record("done", 1);
    ExitCode::SUCCESS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Success,
    NonZero,
    SpawnError,
    Unconfigured,
    Unwind,
}

impl Mode {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "success" => Self::Success,
            "nonzero" => Self::NonZero,
            "spawn-error" => Self::SpawnError,
            "unconfigured" => Self::Unconfigured,
            "unwind" => Self::Unwind,
            _ => return None,
        })
    }
}

#[cfg(not(unix))]
mod platform {
    use std::path::Path;
    use std::process::Command;

    use sniff::process::{ForegroundHandoff, TreeConfig};

    use super::{Mode, Results};

    /// The terminal-free control path: preparation must leave the command
    /// unchanged and restoration must do nothing.
    pub(super) fn run(mode: Mode, _dir: &Path, results: &Results) {
        results.record("mode", format!("{mode:?}"));
        let mut command = Command::new("cmd");
        if mode != Mode::Unconfigured {
            TreeConfig::process_group().configure(&mut command);
        }
        match ForegroundHandoff::prepare(&mut command) {
            Ok(mut handoff) => {
                results.record("active", handoff.is_active());
                results.record("restore", outcome(handoff.restore()));
            }
            Err(error) => results.record("prepare", format!("error:{:?}", error.kind())),
        }
    }

    fn outcome(result: std::io::Result<()>) -> String {
        match result {
            Ok(()) => "ok".to_string(),
            Err(error) => format!("error:{:?}", error.kind()),
        }
    }
}

#[cfg(unix)]
mod platform {
    use std::io;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::panic::{self, AssertUnwindSafe};
    use std::path::Path;
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::time::{Duration, Instant};

    use sniff::process::{CleanupScan, ForegroundHandoff, ProcessTree, TreeConfig};

    use super::{Mode, Results};

    /// How long a child may run before the fixture stops its tree. A child
    /// that never got the foreground stops on its first terminal access, so
    /// this bound is what turns a failed handoff into a recorded `timeout`
    /// rather than a hung pane.
    const CHILD_DEADLINE: Duration = Duration::from_secs(3);
    const POLL: Duration = Duration::from_millis(10);
    const DELIBERATE_PANIC: &str = "deliberate panic while the child owns the terminal";

    /// Kills and reaps the held child when the fixture's scope ends, so no
    /// exit path, including a failed recording, leaves it running. For a child
    /// already waited for, both calls return without signaling anything.
    struct Reaper(Option<Child>);

    impl Drop for Reaper {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    pub(super) fn run(mode: Mode, dir: &Path, results: &Results) {
        results.record("mode", format!("{mode:?}"));
        results.record("host_before", groups(std::process::id()));
        stty(&dir.join("stty.before"));
        match mode {
            Mode::Success | Mode::NonZero => owned_child(mode, dir, results),
            Mode::SpawnError | Mode::Unconfigured => refused_spawn(mode, dir, results),
            Mode::Unwind => unwind(dir, results),
        }
        results.record("host_after", groups(std::process::id()));
        stty(&dir.join("stty.after"));
    }

    /// The child's script. Every script records the child's own groups first,
    /// so `child.fg` exists only when a child actually ran.
    fn child_command(mode: Mode, dir: &Path) -> Command {
        let script = match mode {
            Mode::NonZero => "stty -icanon -echo; echo ready > \"$MARKERS/child.ready\"; exit 7",
            Mode::Unwind => {
                "stty -echo; sleep 10 & echo $! > \"$MARKERS/grandchild.pid\"; \
                 echo ready > \"$MARKERS/child.ready\"; wait"
            }
            _ => {
                "stty -echo; echo ready > \"$MARKERS/child.ready\"; \
                 IFS= read -r line; printf '%s\\n' \"$line\" > \"$MARKERS/child.line\"; exit 0"
            }
        };
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg(format!(
                "ps -o pgid= -o tpgid= -p $$ > \"$MARKERS/child.fg\"; {script}"
            ))
            .env("MARKERS", dir);
        command
    }

    fn owned_child(mode: Mode, dir: &Path, results: &Results) {
        let mut command = child_command(mode, dir);
        TreeConfig::process_group().configure(&mut command);
        let mut handoff = match ForegroundHandoff::prepare(&mut command) {
            Ok(handoff) => handoff,
            Err(error) => return results.record("prepare", describe(&error)),
        };
        results.record("active", handoff.is_active());
        let mut reaper = Reaper(None);
        match command.spawn() {
            Err(error) => results.record("spawn", describe(&error)),
            Ok(child) => {
                results.record("spawn", "ok");
                let child = reaper.0.insert(child);
                match ProcessTree::attach(child) {
                    Err(error) => results.record("attach", describe(&error)),
                    Ok(tree) => {
                        let exit = wait_and_clean_up(child, tree, results);
                        results.record("child_exit", exit);
                    }
                }
            }
        }
        drop(reaper);
        results.record("restore", outcome(handoff.restore()));
    }

    /// Waits for the child, then stops what is left of its tree; a child that
    /// outlives the deadline is stopped and recorded as `timeout:<status>`.
    fn wait_and_clean_up(child: &mut Child, mut tree: ProcessTree, results: &Results) -> String {
        if let Some(status) = wait_bounded(child, &mut tree) {
            results.record("cleanup", outcome(tree.terminate(CleanupScan::WhenObserved)));
            return describe_status(status);
        }
        let stopped = tree.terminate_gracefully(Duration::from_millis(200), child);
        results.record("cleanup", outcome(stopped.cleanup));
        let status = stopped.status.map_or_else(|error| describe(&error), describe_status);
        format!("timeout:{status}")
    }

    /// `spawn-error` lets the handoff hook take the foreground and then fails
    /// the spawn; `unconfigured` never gives the child its own group, which
    /// the handoff hook refuses before touching the terminal.
    fn refused_spawn(mode: Mode, dir: &Path, results: &Results) {
        let mut command = child_command(Mode::Success, dir);
        if mode == Mode::SpawnError {
            TreeConfig::process_group().configure(&mut command);
        }
        let mut handoff = match ForegroundHandoff::prepare(&mut command) {
            Ok(handoff) => handoff,
            Err(error) => return results.record("prepare", describe(&error)),
        };
        results.record("active", handoff.is_active());
        if mode == Mode::SpawnError {
            // SAFETY: the hook only constructs an error value; it allocates
            // nothing and calls no function that is unsafe after `fork`.
            unsafe {
                command.pre_exec(|| Err(io::Error::from(io::ErrorKind::Other)));
            }
        }
        match command.spawn() {
            Ok(child) => {
                results.record("spawn", "ok");
                drop(Reaper(Some(child)));
            }
            Err(error) => results.record("spawn", describe(&error)),
        }
        results.record("pre_restore", groups(std::process::id()));
        results.record("restore", outcome(handoff.restore()));
    }

    /// Panics while the child owns the terminal. The guard is declared before
    /// the tree, so the unwind stops the tree first and then restores the
    /// terminal; the direct child is held outside the closure and reaped after
    /// the panic is caught.
    fn unwind(dir: &Path, results: &Results) {
        let mut reaper = Reaper(None);
        // The panic is recorded rather than printed: the terminal belongs to
        // the child when it happens.
        let panics = Results {
            path: results.path.clone(),
        };
        panic::set_hook(Box::new(move |info| {
            let payload = info.payload();
            let message = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("non-string payload");
            panics.record("panic", message);
        }));
        let caught = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut command = child_command(Mode::Unwind, dir);
            TreeConfig::process_group().configure(&mut command);
            let handoff = ForegroundHandoff::prepare(&mut command).expect("prepare the handoff");
            results.record("active", handoff.is_active());
            let child = reaper.0.insert(command.spawn().expect("spawn the child"));
            let _tree = ProcessTree::attach(child).expect("attach the tree");
            results.record("ready", wait_for(&dir.join("child.ready")));
            panic!("{DELIBERATE_PANIC}");
        }));
        let _ = panic::take_hook();
        results.record("unwind", if caught.is_err() { "caught" } else { "returned" });
        // Observed before reaping, so it shows the state the unwind left.
        results.record("host_unwound", groups(std::process::id()));
        if let Some(child) = reaper.0.as_mut() {
            let exit = poll_exit(child).map_or_else(|| "running".to_string(), describe_status);
            results.record("child_exit", exit);
        }
        drop(reaper);
        let grandchild = std::fs::read_to_string(dir.join("grandchild.pid"))
            .map(|pid| if process_gone(pid.trim()) { "gone" } else { "alive" })
            .unwrap_or("unrecorded");
        results.record("grandchild", grandchild);
    }

    fn wait_bounded(child: &mut Child, tree: &mut ProcessTree) -> Option<ExitStatus> {
        let deadline = Instant::now() + CHILD_DEADLINE;
        while Instant::now() < deadline {
            if let Ok(Some(status)) = child.try_wait() {
                return Some(status);
            }
            tree.sample();
            std::thread::sleep(POLL);
        }
        None
    }

    fn poll_exit(child: &mut Child) -> Option<ExitStatus> {
        let deadline = Instant::now() + CHILD_DEADLINE;
        while Instant::now() < deadline {
            if let Ok(Some(status)) = child.try_wait() {
                return Some(status);
            }
            std::thread::sleep(POLL);
        }
        None
    }

    fn wait_for(path: &Path) -> bool {
        let deadline = Instant::now() + CHILD_DEADLINE;
        while !path.exists() {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(POLL);
        }
        true
    }

    /// Whether `pid` no longer exists; its new parent reaps it shortly after
    /// it is killed, so this polls within the child deadline.
    fn process_gone(pid: &str) -> bool {
        let deadline = Instant::now() + CHILD_DEADLINE;
        loop {
            let alive = Command::new("kill")
                .args(["-0", pid])
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if !alive {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(POLL);
        }
    }

    /// `"<pgid> <tpgid>"` for `pid`, as `ps` reports them.
    fn groups(pid: u32) -> String {
        match Command::new("ps")
            .args(["-o", "pgid=", "-o", "tpgid=", "-p", &pid.to_string()])
            .output()
        {
            Ok(output) => String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            Err(error) => describe(&error),
        }
    }

    /// Writes `stty -a` for the inherited terminal to `path`.
    fn stty(path: &Path) {
        let output = Command::new("stty")
            .arg("-a")
            .stdin(Stdio::inherit())
            .output()
            .map(|output| output.stdout)
            .unwrap_or_else(|error| describe(&error).into_bytes());
        std::fs::write(path, output)
            .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    }

    fn describe(error: &io::Error) -> String {
        format!("error:{:?}", error.kind())
    }

    fn describe_status(status: ExitStatus) -> String {
        match (status.code(), status.signal()) {
            (Some(code), _) => code.to_string(),
            (None, Some(signal)) => format!("signal:{signal}"),
            (None, None) => "unknown".to_string(),
        }
    }

    fn outcome(result: io::Result<()>) -> String {
        result.map_or_else(|error| describe(&error), |()| "ok".to_string())
    }
}
