//! A native fake text editor, driven by environment variables.
//!
//! Tests that launch "the user's editor" point `EDITOR` at this program
//! instead of a real editor or a shell script, so the evidence (which
//! arguments arrived, which streams were terminals, what was written, which
//! processes stayed alive) is the same on macOS, Linux, native Windows, and
//! WSL2.
//!
//! The body lives here; each consuming package declares one thin `[[bin]]`
//! whose `main` calls [`main`] (as with [`argv_recorder`](crate::argv_recorder)),
//! and its tests find that binary with `biscuit_test_harness::bin_exe!`.
//!
//! ## Behavior
//!
//! The target file is the **last** argument. Every variable is optional, and a
//! run with none set exits `0` without touching anything. Set variables act in
//! this order:
//!
//! 1. [`FAKE_EDITOR_RECORD`] appends one [argv record](crate::argv_recorder#record-format)
//!    of every argument.
//! 2. [`FAKE_EDITOR_OUT_TEXT`] / [`FAKE_EDITOR_ERR_TEXT`] are printed to
//!    stdout / stderr.
//! 3. [`FAKE_EDITOR_READ_LINE`] reads one stdin line and writes it (without
//!    its line ending) to the target.
//! 4. [`FAKE_EDITOR_STDIO`] writes a [`StdioReport`] (read back with
//!    [`read_stdio_report`]).
//! 5. [`FAKE_EDITOR_WRITE`] copies the named file's bytes into the target in
//!    place; [`FAKE_EDITOR_REPLACE`] does the same through a temporary file
//!    renamed over the target, the way editors save atomically.
//! 6. [`FAKE_EDITOR_DELETE`] removes the target.
//! 7. [`FAKE_EDITOR_SPAWN_CHILD`] starts a copy of this program that sleeps
//!    until killed and, once that copy is running, appends its PID, one per
//!    line, to [`FAKE_EDITOR_PIDS`].
//! 8. [`FAKE_EDITOR_PAUSE`] creates the named marker file and waits until the
//!    test removes it, then carries on, so a test can change the target while
//!    the editor is "open" and still see a normal exit.
//! 9. [`FAKE_EDITOR_HOLD`] creates the named ready-marker file and sleeps
//!    until killed.
//! 10. The process exits with [`FAKE_EDITOR_EXIT`] (default `0`).
//!
//! [`FAKE_EDITOR_IGNORE_TERM`] (Unix only) ignores `SIGTERM` before any of
//! this, in the editor and in a spawned child, so a test can prove a caller
//! escalates to `SIGKILL`.
//!
//! A fixture failure (an unreadable content file, an unwritable record) exits
//! `2` with a message on stderr, so a misconfigured test fails loudly.
//!
//! ## Notes
//!
//! A sleeping process (hold, or a spawned child) exits on its own after
//! [`SLEEP_CEILING`], so a test that fails before killing it cannot leak a
//! process that outlives the test run.
//!
//! A spawned child's standard streams are not the editor's (stdin and
//! stderr are null, stdout is a private pipe for its readiness line), but on
//! Windows it still inherits the editor's own standard handles. A caller that captures the
//! editor's output through pipes therefore reads until the child exits or is
//! killed; capture nothing, or kill the whole tree, when spawning a child.

use std::ffi::OsString;
use std::io::{BufRead as _, IsTerminal as _, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

/// Path of a file each run appends one argv record to.
pub const FAKE_EDITOR_RECORD: &str = "FAKE_EDITOR_RECORD";
/// Path of a file whose bytes are written into the target in place.
pub const FAKE_EDITOR_WRITE: &str = "FAKE_EDITOR_WRITE";
/// Exit code of the run (decimal, `0`–`255`).
pub const FAKE_EDITOR_EXIT: &str = "FAKE_EDITOR_EXIT";
/// Any non-empty value removes the target.
pub const FAKE_EDITOR_DELETE: &str = "FAKE_EDITOR_DELETE";
/// Path of a file whose bytes replace the target through a temporary file and
/// a rename (an atomic save).
pub const FAKE_EDITOR_REPLACE: &str = "FAKE_EDITOR_REPLACE";
/// Path the run writes its [`StdioReport`] to.
pub const FAKE_EDITOR_STDIO: &str = "FAKE_EDITOR_STDIO";
/// Text printed to stdout.
pub const FAKE_EDITOR_OUT_TEXT: &str = "FAKE_EDITOR_OUT_TEXT";
/// Text printed to stderr.
pub const FAKE_EDITOR_ERR_TEXT: &str = "FAKE_EDITOR_ERR_TEXT";
/// Any non-empty value reads one stdin line into the target.
pub const FAKE_EDITOR_READ_LINE: &str = "FAKE_EDITOR_READ_LINE";
/// Any non-empty value spawns a sleeping child copy of this program.
pub const FAKE_EDITOR_SPAWN_CHILD: &str = "FAKE_EDITOR_SPAWN_CHILD";
/// Path of a file spawned children's PIDs are appended to, one per line.
pub const FAKE_EDITOR_PIDS: &str = "FAKE_EDITOR_PIDS";
/// Any non-empty value ignores `SIGTERM` (Unix only; no effect elsewhere).
pub const FAKE_EDITOR_IGNORE_TERM: &str = "FAKE_EDITOR_IGNORE_TERM";
/// Path of a marker file to create, then wait for a test to remove before
/// continuing.
pub const FAKE_EDITOR_PAUSE: &str = "FAKE_EDITOR_PAUSE";
/// Path of a ready-marker file to create before sleeping until killed.
pub const FAKE_EDITOR_HOLD: &str = "FAKE_EDITOR_HOLD";

/// Marks the sleeping child a spawning run starts; not part of the contract.
const CHILD_MARKER: &str = "FAKE_EDITOR_INTERNAL_SLEEPING_CHILD";

/// How long a held editor or a spawned child sleeps before exiting on its own.
pub const SLEEP_CEILING: Duration = Duration::from_secs(120);

/// What a run observed about its standard streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioReport {
    /// Whether stdin was a terminal.
    pub stdin_tty: bool,
    /// Whether stdout was a terminal.
    pub stdout_tty: bool,
    /// Whether stderr was a terminal.
    pub stderr_tty: bool,
    /// What stdin yielded: the line [`FAKE_EDITOR_READ_LINE`] read, or
    /// everything up to end of input. `None` when stdin was a terminal and
    /// nothing asked to read it (reading would wait for a person).
    pub stdin: Option<String>,
}

/// The fake editor's entry point, for a consuming package's thin `[[bin]]`.
pub fn main() -> ExitCode {
    let var = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if var(CHILD_MARKER).is_some() {
        if var(FAKE_EDITOR_IGNORE_TERM).is_some() {
            ignore_sigterm();
        }
        // Tell the spawning run this copy is set up; it reads one line.
        println!("ready");
        let _ = std::io::stdout().flush();
        sleep_until_killed();
        return ExitCode::SUCCESS;
    }
    match run(&var) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("fake editor: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(var: &dyn Fn(&str) -> Option<OsString>) -> Result<u8, String> {
    if var(FAKE_EDITOR_IGNORE_TERM).is_some() {
        ignore_sigterm();
    }
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let target = args.last().map(PathBuf::from);
    let target = || target.as_deref().ok_or_else(|| "no target file argument".to_string());

    if let Some(record) = var(FAKE_EDITOR_RECORD) {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&record)
            .and_then(|mut file| file.write_all(&crate::argv_recorder::encode_record(&args)))
            .map_err(|error| format!("cannot write {}: {error}", Path::new(&record).display()))?;
    }
    if let Some(text) = var(FAKE_EDITOR_OUT_TEXT) {
        print!("{}", text.to_string_lossy());
        std::io::stdout().flush().map_err(|error| format!("flush stdout: {error}"))?;
    }
    if let Some(text) = var(FAKE_EDITOR_ERR_TEXT) {
        eprint!("{}", text.to_string_lossy());
        std::io::stderr().flush().map_err(|error| format!("flush stderr: {error}"))?;
    }

    let mut stdin_yield = None;
    if var(FAKE_EDITOR_READ_LINE).is_some() {
        let mut line = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|error| format!("read stdin: {error}"))?;
        let line = line.trim_end_matches(['\r', '\n']).to_string();
        write_file(target()?, line.as_bytes())?;
        stdin_yield = Some(line);
    }
    if let Some(report_path) = var(FAKE_EDITOR_STDIO) {
        let stdin_tty = std::io::stdin().is_terminal();
        if stdin_yield.is_none() && !stdin_tty {
            let mut bytes = Vec::new();
            std::io::stdin()
                .read_to_end(&mut bytes)
                .map_err(|error| format!("read stdin: {error}"))?;
            stdin_yield = Some(String::from_utf8_lossy(&bytes).into_owned());
        }
        let report = StdioReport {
            stdin_tty,
            stdout_tty: std::io::stdout().is_terminal(),
            stderr_tty: std::io::stderr().is_terminal(),
            stdin: stdin_yield,
        };
        write_file(Path::new(&report_path), encode_stdio_report(&report).as_bytes())?;
    }

    if let Some(content) = var(FAKE_EDITOR_WRITE) {
        write_file(target()?, &read_file(Path::new(&content))?)?;
    }
    if let Some(content) = var(FAKE_EDITOR_REPLACE) {
        let target = target()?;
        let bytes = read_file(Path::new(&content))?;
        let mut temporary = target.as_os_str().to_owned();
        temporary.push(".fake-editor-save");
        let temporary = PathBuf::from(temporary);
        write_file(&temporary, &bytes)?;
        std::fs::rename(&temporary, target)
            .map_err(|error| format!("rename over {}: {error}", target.display()))?;
    }
    if var(FAKE_EDITOR_DELETE).is_some() {
        let target = target()?;
        std::fs::remove_file(target).map_err(|error| format!("remove {}: {error}", target.display()))?;
    }

    if var(FAKE_EDITOR_SPAWN_CHILD).is_some() {
        let pids = var(FAKE_EDITOR_PIDS).ok_or_else(|| format!("{FAKE_EDITOR_SPAWN_CHILD} needs {FAKE_EDITOR_PIDS}"))?;
        let exe = std::env::current_exe().map_err(|error| format!("current exe: {error}"))?;
        // No inherited streams: a child holding the caller's pipes would keep
        // a test's output reader open after the editor itself exits. Its
        // stdout is a private pipe it reports readiness on, so the PID is
        // recorded only once the child has, for example, ignored `SIGTERM`.
        let mut child = Command::new(exe)
            .env(CHILD_MARKER, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("spawn child: {error}"))?;
        let mut ready = String::new();
        std::io::BufReader::new(child.stdout.take().expect("piped child stdout"))
            .read_line(&mut ready)
            .map_err(|error| format!("wait for child: {error}"))?;
        if ready.trim() != "ready" {
            return Err("the spawned child exited before it was ready".to_string());
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&pids)
            .and_then(|mut file| writeln!(file, "{}", child.id()))
            .map_err(|error| format!("cannot write {}: {error}", Path::new(&pids).display()))?;
    }
    if let Some(marker) = var(FAKE_EDITOR_PAUSE) {
        let marker = Path::new(&marker);
        write_file(marker, b"paused")?;
        let started = Instant::now();
        while marker.exists() && started.elapsed() < SLEEP_CEILING {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    if let Some(marker) = var(FAKE_EDITOR_HOLD) {
        write_file(Path::new(&marker), b"ready")?;
        sleep_until_killed();
    }

    match var(FAKE_EDITOR_EXIT) {
        None => Ok(0),
        Some(code) => code
            .to_string_lossy()
            .trim()
            .parse::<u8>()
            .map_err(|error| format!("{FAKE_EDITOR_EXIT} is not 0-255: {error}")),
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

fn sleep_until_killed() {
    let started = Instant::now();
    while started.elapsed() < SLEEP_CEILING {
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
fn ignore_sigterm() {
    // SAFETY: installing `SIG_IGN` for `SIGTERM` has no handler code to race
    // with; it runs before this single-threaded program does anything else.
    unsafe {
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn ignore_sigterm() {}

fn encode_stdio_report(report: &StdioReport) -> String {
    serde_json::json!({
        "stdin_tty": report.stdin_tty,
        "stdout_tty": report.stdout_tty,
        "stderr_tty": report.stderr_tty,
        "stdin": report.stdin,
    })
    .to_string()
}

/// The [`StdioReport`] a run wrote to [`FAKE_EDITOR_STDIO`].
///
/// ## Errors
///
/// The read error (a missing file is [`std::io::ErrorKind::NotFound`]:
/// nothing ran), or [`std::io::ErrorKind::InvalidData`] for a file that is not
/// a report.
pub fn read_stdio_report(path: &Path) -> std::io::Result<StdioReport> {
    let invalid = |message: &str| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("fake editor stdio report: {message}"))
    };
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path)?).map_err(|error| invalid(&error.to_string()))?;
    let flag = |key: &str| value.get(key).and_then(serde_json::Value::as_bool).ok_or_else(|| invalid(key));
    let stdin = match value.get("stdin") {
        Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(text)) => Some(text.clone()),
        _ => return Err(invalid("stdin")),
    };
    Ok(StdioReport {
        stdin_tty: flag("stdin_tty")?,
        stdout_tty: flag("stdout_tty")?,
        stderr_tty: flag("stderr_tty")?,
        stdin,
    })
}

/// The PIDs spawned children appended to a [`FAKE_EDITOR_PIDS`] file.
///
/// ## Errors
///
/// The read error, or [`std::io::ErrorKind::InvalidData`] for a line that is
/// not a PID.
pub fn read_pids(path: &Path) -> std::io::Result<Vec<u32>> {
    std::fs::read_to_string(path)?
        .lines()
        .map(|line| {
            line.trim().parse::<u32>().map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, format!("fake editor pid file: {error}"))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdio_reports_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stdio.json");
        for report in [
            StdioReport { stdin_tty: true, stdout_tty: false, stderr_tty: true, stdin: None },
            StdioReport { stdin_tty: false, stdout_tty: true, stderr_tty: false, stdin: Some("a\nb".into()) },
        ] {
            std::fs::write(&path, encode_stdio_report(&report)).unwrap();
            assert_eq!(read_stdio_report(&path).unwrap(), report);
        }
    }

    #[test]
    fn a_malformed_stdio_report_is_invalid_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stdio.json");
        std::fs::write(&path, r#"{"stdin_tty": "yes"}"#).unwrap();
        assert_eq!(read_stdio_report(&path).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }
}
