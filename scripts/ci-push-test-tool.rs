//! Test fixture: one executable that stands in for `python3`, `py`, `python`,
//! `gh`, `just`, `cargo`, `git`, and `sniff` in `ci-push`'s tests, and for a
//! bare remote's `pre-receive` hook.
//!
//! A test copies this binary into a fixture `bin` directory under each tool's
//! name (with `.exe` on Windows; never a symlink or a script) and puts that
//! directory first on `PATH`. Most tests build `PATH` from fixture directories
//! only, so a missing stub can never fall through to a real tool; the
//! end-to-end parity tests append only the real tools they run (Git, Python,
//! and for the `ci-local` recipe `just`, `jq`, and Bash).
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
//! - `dispatch`: `[{ "args": [...], "behavior": {...} }, ...]`; the first
//!   entry whose `args` are a prefix of argv replaces this behavior, and acts
//!   on the rest of argv (a `just` answering `pre-push`, `_lint`, and `_test
//!   <pkg>` differently). With no match, the keys below apply.
//! - `write_files`: `{ "relative/or/absolute": "content" }`, relative to the
//!   cwd, parents created (staged reports). A path starting with `$NAME`
//!   starts with that environment variable's value instead
//!   (`"$BISCUIT_CI_REPORTS_OUT/L1/alpha.xml"`); an unset variable panics.
//! - `append_files`: as `write_files`, appending (manifest lines).
//! - `copy_files`: `{ "destination": "source" }`, both spelled as in
//!   `write_files`: copies a file byte for byte, parents created (a gate
//!   writing back the plan it was fed).
//! - `stdout`, `stderr`: text to print; `stdout_fill`, `stderr_fill`: a byte
//!   count to write, interleaved in 64 KiB chunks.
//! - `signal_parent` (Unix): a signal number sent to the parent process
//!   (a gate interrupting the hook that runs it).
//! - `spawn_grandchild`: a path; start a copy of this tool that writes its pid
//!   there (atomically, by rename) and then hangs, inside this process's
//!   group or Job.
//! - `escape_grandchild` (Unix): as `spawn_grandchild`, but the grandchild
//!   starts its own process group, outside the supervisor's ownership, while
//!   still holding this process's stdout.
//! - `supervise`: a path; install `ci-push`'s cancellation handlers, run a
//!   hanging grandchild (pid written as above) through `ChildCommand`, and
//!   exit with the cancellation exit code when a signal ends it.
//! - `delegate`: `{ "program": path, "args": [...] }`; run the program with
//!   `args` and then the rest of argv, inheriting cwd, environment, stdin,
//!   stdout, and stderr, and exit with its status (128 + the signal when a
//!   signal ended it) — a stub `just` that hands `pre-push` to the real one.
//! - `refuse_stdin_containing`: with `read_stdin`, exit 1 when the input
//!   contains this text (a remote `pre-receive` refusing one ref).
//! - `hang`: wait until killed (capped at 30 s so a supervision bug cannot
//!   leave an immortal process).
//! - `forward`: a program path; run it with every argument after the first
//!   `--` (a `cargo run ... -- <args>` stand-in), in this cwd and environment,
//!   with this process's stdin (the recorded bytes when `read_stdin` consumed
//!   them), and exit with its status. Its stdout and stderr are inherited.
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
    write_files: BTreeMap<String, String>,
    stdout: Option<String>,
    stderr: Option<String>,
    stdout_fill: Option<usize>,
    stderr_fill: Option<usize>,
    spawn_grandchild: Option<PathBuf>,
    escape_grandchild: Option<PathBuf>,
    supervise: Option<PathBuf>,
    forward: Option<PathBuf>,
    #[serde(default)]
    dispatch: Vec<Dispatch>,
    #[serde(default)]
    append_files: BTreeMap<String, String>,
    #[serde(default)]
    copy_files: BTreeMap<String, String>,
    signal_parent: Option<i32>,
    delegate: Option<Delegate>,
    refuse_stdin_containing: Option<String>,
    #[serde(default)]
    hang: bool,
    #[serde(default)]
    exit: u8,
    #[serde(default)]
    record_env: Vec<String>,
    #[serde(default)]
    read_stdin: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dispatch {
    args: Vec<String>,
    behavior: Behavior,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Delegate {
    program: PathBuf,
    #[serde(default)]
    args: Vec<String>,
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
        bytes
    });
    if let Some(log) = control.as_ref().and_then(|control| control.log.as_deref()) {
        let text = stdin.as_deref().map(|bytes| String::from_utf8_lossy(bytes).into_owned());
        record(log, &name, &argv, behavior, text);
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
    act(&exe, behavior, &argv, stdin.as_deref())
}

// The grandchildren are never waited for: ending them is the job of the
// supervisor under test, and a reap here would hide its failure.
#[allow(clippy::zombie_processes)]
fn act(exe: &Path, behavior: &Behavior, argv: &[String], stdin: Option<&[u8]>) -> ExitCode {
    if let Some(entry) = behavior
        .dispatch
        .iter()
        .find(|entry| argv.starts_with(&entry.args))
    {
        return act(exe, &entry.behavior, &argv[entry.args.len()..], stdin);
    }
    if let Some(version) = &behavior.python_version {
        if argv.iter().any(|arg| arg == "-c") {
            println!("{version}");
            return ExitCode::SUCCESS;
        }
    }
    for (path, content) in &behavior.write_files {
        stage(path, content, false);
    }
    for (path, content) in &behavior.append_files {
        stage(path, content, true);
    }
    for (destination, source) in &behavior.copy_files {
        let source = expand(source);
        let content = std::fs::read(&source)
            .unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
        stage_bytes(destination, &content, false);
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

    #[cfg(unix)]
    if let Some(signal) = behavior.signal_parent {
        // SAFETY: plain syscalls on this process's parent.
        unsafe { libc::kill(libc::getppid(), signal) };
    }
    #[cfg(not(unix))]
    if behavior.signal_parent.is_some() {
        eprintln!("ci-push-test-tool: signal_parent is Unix-only");
        return ExitCode::from(96);
    }
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
    if let Some(program) = &behavior.forward {
        return forward(program, argv, stdin);
    }
    if let Some(delegate) = &behavior.delegate {
        return run_delegate(delegate, argv);
    }
    if let Some(needle) = &behavior.refuse_stdin_containing {
        let input = String::from_utf8_lossy(stdin.unwrap_or_default());
        if input.contains(needle.as_str()) {
            return ExitCode::from(1);
        }
    }
    if behavior.hang {
        hang();
    }
    ExitCode::from(behavior.exit)
}

fn forward(program: &Path, argv: &[String], stdin: Option<&[u8]>) -> ExitCode {
    let args = argv
        .iter()
        .position(|arg| arg == "--")
        .map_or(&[][..], |at| &argv[at + 1..]);
    let mut command = std::process::Command::new(program);
    command.args(args);
    if stdin.is_some() {
        command.stdin(std::process::Stdio::piped());
    }
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("forward to {}: {error}", program.display()));
    if let Some(bytes) = stdin {
        // Dropping the handle closes the pipe, so the child reads end of file.
        let mut pipe = child.stdin.take().expect("a piped stdin");
        pipe.write_all(bytes).expect("forward stdin");
    }
    let status = child.wait().expect("wait for the forwarded program");
    // A signal-ended child has no code; report it as an interruption.
    ExitCode::from(status.code().map_or(130, |code| code.clamp(0, 255) as u8))
}

fn run_delegate(delegate: &Delegate, argv: &[String]) -> ExitCode {
    let status = std::process::Command::new(&delegate.program)
        .args(&delegate.args)
        .args(argv)
        .status()
        .unwrap_or_else(|error| panic!("delegate to {}: {error}", delegate.program.display()));
    #[cfg(unix)]
    if let Some(signal) = std::os::unix::process::ExitStatusExt::signal(&status) {
        return ExitCode::from((128 + signal).clamp(0, 255) as u8);
    }
    ExitCode::from(status.code().map_or(1, |code| code.clamp(0, 255) as u8))
}

/// Writes or appends `content` at `path`, after expanding a leading `$NAME`.
fn stage(path: &str, content: &str, append: bool) {
    stage_bytes(path, content.as_bytes(), append);
}

fn stage_bytes(path: &str, content: &[u8], append: bool) {
    let path = expand(path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).expect("create a staged file's directory");
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(&path)
        .unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
    file.write_all(content)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

fn expand(path: &str) -> PathBuf {
    let Some(rest) = path.strip_prefix('$') else {
        return PathBuf::from(path);
    };
    let (name, tail) = rest.split_once('/').unwrap_or((rest, ""));
    let value =
        std::env::var_os(name).unwrap_or_else(|| panic!("ci-push-test-tool: ${name} is not set"));
    let mut expanded = PathBuf::from(value);
    if !tail.is_empty() {
        expanded.push(tail);
    }
    expanded
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
