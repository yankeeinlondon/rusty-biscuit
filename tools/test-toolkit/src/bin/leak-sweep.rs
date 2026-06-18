//! `leak-sweep` — run a command, then report child processes that outlived it.
//!
//! A cross-platform (macOS / Windows / Linux) post-run orphan detector. It
//! snapshots the live process set, runs the wrapped command to completion, then
//! snapshots again and reports any *new* process still alive whose executable
//! path or command line points inside the workspace root.
//!
//! This complements nextest's per-test `LEAK` status: nextest only flags
//! children still holding a test's stdout/stderr pipes, whereas this sweep also
//! catches detached orphans that closed or redirected those handles. It relies
//! on a snapshot diff rather than process-group/job membership, so it works
//! regardless of how the test runner groups or reparents processes.
//!
//! ## Examples
//!
//! ```text
//! leak-sweep -- cargo nextest run
//! leak-sweep --root /path/to/repo --settle-ms 500 -- just test claudine
//! ```
//!
//! ## Notes
//!
//! - Attribution is by workspace path, not parent PID, because orphan
//!   reparenting differs per OS (init/launchd on Unix; job semantics on
//!   Windows). A leaked process invoked purely by a bare system name (e.g.
//!   `git`) with no workspace path on its command line is not attributed.
//! - PID reuse during the run can mask a survivor whose PID matches a
//!   pre-existing one; this is rare over a single run.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System};

/// Exit code used when the wrapped command succeeded but leaked processes were
/// found. Distinct from `1` so callers can tell a leak from a test failure.
const LEAK_EXIT_CODE: i32 = 99;

#[derive(Parser)]
#[command(
    name = "leak-sweep",
    about = "Run a command, then report child processes that outlived it"
)]
struct Cli {
    /// Workspace root used to attribute survivors. A survivor is reported only
    /// when its executable path or command line points inside this directory.
    /// Defaults to the current working directory.
    #[arg(long)]
    root: Option<PathBuf>,

    /// Grace period (milliseconds) to wait after the command exits before the
    /// final snapshot, letting normal child cleanup finish.
    #[arg(long, default_value_t = 250)]
    settle_ms: u64,

    /// Report survivors but still exit with the wrapped command's status,
    /// instead of failing with a dedicated leak exit code.
    #[arg(long)]
    warn_only: bool,

    /// The command to run — every argument after the options, optionally
    /// preceded by `--`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    command: Vec<String>,
}

fn live_pids(sys: &mut System) -> HashSet<Pid> {
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.processes().keys().copied().collect()
}

/// Whether `proc`'s executable path or any command-line argument points inside
/// `root` — our heuristic for "this process belongs to the test run."
fn belongs_to_workspace(proc: &Process, root: &std::path::Path) -> bool {
    if proc.exe().is_some_and(|exe| exe.starts_with(root)) {
        return true;
    }
    let root = root.to_string_lossy();
    proc.cmd()
        .iter()
        .any(|arg| arg.to_string_lossy().contains(root.as_ref()))
}

fn describe(pid: Pid, proc: &Process) -> String {
    let exe = proc
        .exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| proc.name().to_string_lossy().into_owned());
    let args: Vec<String> = proc
        .cmd()
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    format!("pid {pid}  {exe}  [{}]", args.join(" "))
}

fn main() {
    let cli = Cli::parse();

    let root = cli
        .root
        .unwrap_or_else(|| std::env::current_dir().expect("current directory"));
    let root = root.canonicalize().unwrap_or(root);

    let mut sys = System::new();
    let before = live_pids(&mut sys);

    let (program, args) = cli.command.split_first().expect("clap requires a command");
    let status = Command::new(program).args(args).status().unwrap_or_else(|err| {
        eprintln!("leak-sweep: failed to launch {program:?}: {err}");
        std::process::exit(127);
    });

    if cli.settle_ms > 0 {
        sleep(Duration::from_millis(cli.settle_ms));
    }

    // `everything()` is required so `exe()`/`cmd()` are populated — the default
    // refresh kind leaves argv empty on macOS, which would defeat attribution.
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::everything());
    let mut leaks: Vec<String> = sys
        .processes()
        .iter()
        .filter(|(pid, _)| !before.contains(*pid))
        .filter(|(_, proc)| belongs_to_workspace(proc, &root))
        .map(|(pid, proc)| describe(*pid, proc))
        .collect();
    leaks.sort();

    if leaks.is_empty() {
        eprintln!("leak-sweep: no leaked processes detected.");
    } else {
        eprintln!(
            "leak-sweep: {} process(es) outlived the run (rooted at {}):",
            leaks.len(),
            root.display()
        );
        for leak in &leaks {
            eprintln!("  - {leak}");
        }
    }

    let command_code = status.code().unwrap_or(1);
    if !leaks.is_empty() && status.success() && !cli.warn_only {
        std::process::exit(LEAK_EXIT_CODE);
    }
    std::process::exit(command_code);
}
