//! Test fixture: one executable that stands in for `python3`, `py`, `python`,
//! `gh`, `just`, `cargo`, and `git` in `ci-push`'s tests.
//!
//! A test copies this binary into a fixture `bin` directory under each tool's
//! name (with `.exe` on Windows; never a symlink or a script) and builds `PATH`
//! from fixture directories only, so a missing stub can never fall through to
//! a real tool.
//!
//! # Choosing a behavior
//!
//! The behavior name is the executable's file stem, or `<name>` when the first
//! two arguments are `--scenario <name>` (those two are then not part of the
//! recorded argv). Behaviors are read from `ci-push-test-tool.json` beside the
//! executable, so each fixture directory configures its own copies:
//!
//! ```json
//! {
//!   "log": "/abs/invocations.jsonl",
//!   "tools": {
//!     "python3": { "exit": 1, "stderr": "Python was not found" },
//!     "python": { "python_version": "3.12" },
//!     "gh": { "stdout": "[]", "record_env": ["GH_NO_UPDATE_NOTIFIER"], "read_stdin": true }
//!   }
//! }
//! ```
//!
//! Every invocation appends one JSON line to `log` (when set) before acting:
//! `tool`, `argv`, `cwd`, `pid`, `env` (each `record_env` name, `null` when
//! unset), and `stdin` (only with `read_stdin`). A missing control file or a
//! name with no entry exits 97 or 98 after logging.
//!
//! Behavior keys, applied in this order:
//!
//! - `python_version`: when argv contains `-c`, print it and exit 0 (the
//!   version probe); other invocations fall through to the keys below.
//! - `write_files`: `{ "relative/or/absolute": "content" }`, relative to the
//!   cwd, parents created (staged reports).
//! - `stdout`, `stderr`: text to print; `stdout_fill`, `stderr_fill`: a byte
//!   count to write, interleaved in 64 KiB chunks.
//! - `spawn_grandchild`: a path; start a copy of this tool that writes its pid
//!   there (atomically, by rename) and then hangs, inside this process's
//!   group or Job.
//! - `escape_grandchild` (Unix): as `spawn_grandchild`, but the grandchild
//!   starts its own process group, outside the supervisor's ownership, while
//!   still holding this process's stdout.
//! - `supervise`: a path; install `ci-push`'s cancellation handlers, run a
//!   hanging grandchild (pid written as above) through `ChildCommand`, and
//!   exit with the cancellation exit code when a signal ends it.
//! - `hang`: wait until killed (capped at 30 s so a supervision bug cannot
//!   leave an immortal process).
//! - `exit`: the exit code (default 0).

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{json, Value};

const CONTROL_FILE: &str = "ci-push-test-tool.json";
const HANG_LIMIT: Duration = Duration::from_secs(30);
/// The internal behavior of a spawned grandchild: write the pid, then hang.
const GRANDCHILD: &str = "__grandchild";

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    log: Option<PathBuf>,
    #[serde(default)]
    tools: BTreeMap<String, Behavior>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Behavior {
    python_version: Option<String>,
    #[serde(default)]
    write_files: BTreeMap<PathBuf, String>,
    stdout: Option<String>,
    stderr: Option<String>,
    stdout_fill: Option<usize>,
    stderr_fill: Option<usize>,
    spawn_grandchild: Option<PathBuf>,
    escape_grandchild: Option<PathBuf>,
    supervise: Option<PathBuf>,
    #[serde(default)]
    hang: bool,
    #[serde(default)]
    exit: u8,
    #[serde(default)]
    record_env: Vec<String>,
    #[serde(default)]
    read_stdin: bool,
}

fn main() -> ExitCode {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    let exe = std::env::current_exe().expect("the fixture knows its own path");
    let name = if argv.first().map(String::as_str) == Some("--scenario") && argv.len() >= 2 {
        let name = argv[1].clone();
        argv.drain(..2);
        name
    } else {
        exe.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    };

    if name == GRANDCHILD {
        let pid_file = argv
            .first()
            .map(PathBuf::from)
            .expect("grandchild pid path");
        write_atomically(&pid_file, std::process::id().to_string().as_bytes());
        hang();
    }

    let control_path = exe.with_file_name(CONTROL_FILE);
    let control: Option<Control> = std::fs::read(&control_path).ok().map(|bytes| {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("{} is malformed: {error}", control_path.display()))
    });
    let behavior = control
        .as_ref()
        .and_then(|control| control.tools.get(&name));

    let stdin = behavior.filter(|behavior| behavior.read_stdin).map(|_| {
        let mut bytes = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    });
    if let Some(log) = control.as_ref().and_then(|control| control.log.as_deref()) {
        record(log, &name, &argv, behavior, stdin);
    }

    let Some(control) = control else {
        eprintln!(
            "ci-push-test-tool: no {CONTROL_FILE} beside {}",
            exe.display()
        );
        return ExitCode::from(97);
    };
    let Some(behavior) = control.tools.get(&name) else {
        eprintln!("ci-push-test-tool: no behavior configured for {name:?}");
        return ExitCode::from(98);
    };
    act(&exe, behavior, &argv)
}

// The grandchildren are never waited for: ending them is the job of the
// supervisor under test, and a reap here would hide its failure.
#[allow(clippy::zombie_processes)]
fn act(exe: &Path, behavior: &Behavior, argv: &[String]) -> ExitCode {
    if let Some(version) = &behavior.python_version {
        if argv.iter().any(|arg| arg == "-c") {
            println!("{version}");
            return ExitCode::SUCCESS;
        }
    }
    for (path, content) in &behavior.write_files {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).expect("create a staged file's directory");
        }
        std::fs::write(path, content).expect("write a staged file");
    }
    if let Some(text) = &behavior.stdout {
        print!("{text}");
    }
    if let Some(text) = &behavior.stderr {
        eprint!("{text}");
    }
    fill(
        behavior.stdout_fill.unwrap_or(0),
        behavior.stderr_fill.unwrap_or(0),
    );
    let _ = std::io::stdout().flush();

    if let Some(pid_file) = &behavior.spawn_grandchild {
        std::process::Command::new(exe)
            .args(["--scenario", GRANDCHILD])
            .arg(pid_file)
            .spawn()
            .expect("spawn the grandchild");
    }
    if let Some(pid_file) = &behavior.escape_grandchild {
        let mut command = std::process::Command::new(exe);
        command.args(["--scenario", GRANDCHILD]).arg(pid_file);
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        command.spawn().expect("spawn the escaping grandchild");
    }
    if let Some(pid_file) = &behavior.supervise {
        return supervise(exe, pid_file);
    }
    if behavior.hang {
        hang();
    }
    ExitCode::from(behavior.exit)
}

/// Alternates chunks so a parent that drains one pipe at a time deadlocks.
fn fill(mut stdout: usize, mut stderr: usize) {
    const CHUNK: usize = 64 * 1024;
    let out_chunk = [b'o'; CHUNK];
    let err_chunk = [b'e'; CHUNK];
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    while stdout > 0 || stderr > 0 {
        let take = stdout.min(CHUNK);
        out.write_all(&out_chunk[..take]).expect("write stdout");
        out.flush().expect("flush stdout");
        stdout -= take;
        let take = stderr.min(CHUNK);
        err.write_all(&err_chunk[..take]).expect("write stderr");
        stderr -= take;
    }
}

fn supervise(exe: &Path, pid_file: &Path) -> ExitCode {
    use repo_deps::ci_push::process::{
        exit_code_for_cancellation, install_cancellation_handlers, ChildCommand, Termination,
    };

    install_cancellation_handlers().expect("install the cancellation handlers");
    let completed = ChildCommand::new(exe)
        .args(["--scenario", GRANDCHILD])
        .arg(pid_file)
        .run()
        .expect("run the supervised grandchild");
    match completed.termination {
        Termination::Cancelled(_) => {
            ExitCode::from(exit_code_for_cancellation().expect("a recorded signal"))
        }
        other => {
            eprintln!("ci-push-test-tool: supervised child ended without cancellation: {other:?}");
            ExitCode::from(99)
        }
    }
}

fn record(
    log: &Path,
    name: &str,
    argv: &[String],
    behavior: Option<&Behavior>,
    stdin: Option<String>,
) {
    let env: serde_json::Map<String, Value> = behavior
        .map(|behavior| behavior.record_env.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|key| {
            let value = std::env::var_os(key).map(|value| value.to_string_lossy().into_owned());
            (key.clone(), json!(value))
        })
        .collect();
    let cwd = std::env::current_dir()
        .map(|cwd| cwd.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut line = json!({
        "tool": name,
        "argv": argv,
        "cwd": cwd,
        "pid": std::process::id(),
        "env": env,
    });
    if let Some(stdin) = stdin {
        line["stdin"] = json!(stdin);
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .expect("open the invocation log");
    // One write per line: appends from concurrent invocations never interleave
    // within a line.
    file.write_all(format!("{line}\n").as_bytes())
        .expect("append to the invocation log");
}

fn write_atomically(path: &Path, bytes: &[u8]) {
    let staging = path.with_extension("partial");
    std::fs::write(&staging, bytes).expect("write the staging file");
    std::fs::rename(&staging, path).expect("publish the file");
}

fn hang() -> ! {
    let started = Instant::now();
    while started.elapsed() < HANG_LIMIT {
        std::thread::sleep(Duration::from_millis(50));
    }
    std::process::exit(3);
}
