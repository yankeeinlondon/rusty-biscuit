//! Choosing the one Python interpreter every direct Python child uses.
//!
//! A name on `PATH` proves nothing on Windows, where `python3` can be an App
//! Execution Alias stub that exits non-zero, so each candidate is *run* with a
//! version probe and the first that answers 3.10 or newer is kept. Candidates
//! are tried in a fixed order: `python3`, `py -3` (Windows only), `python`.
//!
//! The selection reaches the gates as two variables (see
//! [`PythonCommand::gate_environment`]) rather than as shell text, so a Bash
//! recipe can rebuild the exact argv without evaluating anything.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::process::{CancelSignal, Cancellation, ChildCommand, Termination};
use super::PythonCommand;

/// How long one candidate gets to answer the version probe.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// The oldest Python the planner and evidence helpers support.
pub const MINIMUM_VERSION: (u32, u32) = (3, 10);

/// Prints `major.minor`; valid in every Python old enough to be rejected.
const VERSION_PROBE: &str = "import sys; print('%d.%d' % sys.version_info[:2])";

/// Enough for a version line or a launcher's complaint.
const PROBE_CAPTURE_LIMIT: usize = 4 * 1024;

/// The selected interpreter's native path.
pub const PYTHON_VARIABLE: &str = "BISCUIT_CI_PYTHON";
/// A file holding the interpreter's argv prefix, each argument followed by a
/// NUL byte (Bash reads it with `IFS= read -r -d ''`).
pub const PYTHON_ARGS_FILE_VARIABLE: &str = "BISCUIT_CI_PYTHON_ARGS_FILE";
const ARGS_FILE_NAME: &str = "python-args";

/// One interpreter spelling to try.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Candidate {
    name: &'static str,
    prefix: &'static [&'static str],
}

impl Candidate {
    fn label(self) -> String {
        std::iter::once(self.name)
            .chain(self.prefix.iter().copied())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(windows)]
const CANDIDATES: [Candidate; 3] = [
    Candidate {
        name: "python3",
        prefix: &[],
    },
    Candidate {
        name: "py",
        prefix: &["-3"],
    },
    Candidate {
        name: "python",
        prefix: &[],
    },
];

#[cfg(not(windows))]
const CANDIDATES: [Candidate; 2] = [
    Candidate {
        name: "python3",
        prefix: &[],
    },
    Candidate {
        name: "python",
        prefix: &[],
    },
];

/// Why one candidate was not selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    NotFound,
    FailedToStart(String),
    Exited { code: Option<i32>, stderr: String },
    TimedOut(Duration),
    TooOld { major: u32, minor: u32 },
    UnreadableVersion(String),
}

impl fmt::Display for Rejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::NotFound => formatter.write_str("not found on PATH"),
            Rejection::FailedToStart(error) => write!(formatter, "failed to start: {error}"),
            Rejection::Exited { code, stderr } => {
                match code {
                    Some(code) => write!(formatter, "version probe exited with status {code}")?,
                    None => formatter.write_str("version probe was killed by a signal")?,
                }
                if !stderr.is_empty() {
                    write!(formatter, ": {stderr}")?;
                }
                Ok(())
            }
            Rejection::TimedOut(timeout) => write!(
                formatter,
                "did not answer the version probe within {} ms",
                timeout.as_millis()
            ),
            Rejection::TooOld { major, minor } => write!(
                formatter,
                "is Python {major}.{minor}; {}.{} or newer is required",
                MINIMUM_VERSION.0, MINIMUM_VERSION.1
            ),
            Rejection::UnreadableVersion(output) => {
                write!(formatter, "answered the version probe with {output:?}")
            }
        }
    }
}

/// One candidate that was tried and rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    /// The spelling tried, such as `py -3`.
    pub candidate: String,
    /// The resolved executable, when one was found.
    pub executable: Option<PathBuf>,
    pub rejection: Rejection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonSelectionError {
    /// Every candidate, in the order tried.
    NoInterpreter(Vec<Attempt>),
    Cancelled(CancelSignal),
}

impl fmt::Display for PythonSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PythonSelectionError::NoInterpreter(attempts) => {
                write!(
                    formatter,
                    "no Python {}.{} or newer could be run; tried",
                    MINIMUM_VERSION.0, MINIMUM_VERSION.1
                )?;
                for (index, attempt) in attempts.iter().enumerate() {
                    let separator = if index == 0 { ":" } else { ";" };
                    write!(formatter, "{separator} {}", attempt.candidate)?;
                    if let Some(executable) = &attempt.executable {
                        write!(formatter, " ({})", executable.display())?;
                    }
                    write!(formatter, " {}", attempt.rejection)?;
                }
                Ok(())
            }
            PythonSelectionError::Cancelled(_) => {
                formatter.write_str("Python selection was cancelled")
            }
        }
    }
}

impl std::error::Error for PythonSelectionError {}

impl PythonCommand {
    /// The first candidate on `path` (a `PATH` value) that runs as Python
    /// 3.10 or newer within [`PROBE_TIMEOUT`], observing the process-wide
    /// cancellation.
    ///
    /// # Errors
    ///
    /// No candidate qualified (the error lists each and why), or the run was
    /// cancelled.
    pub fn select(path: Option<&OsStr>) -> Result<Self, PythonSelectionError> {
        Self::select_with(path, PROBE_TIMEOUT, &Cancellation::process())
    }

    /// [`PythonCommand::select`] with an explicit probe timeout and
    /// cancellation.
    ///
    /// # Errors
    ///
    /// As [`PythonCommand::select`].
    pub fn select_with(
        path: Option<&OsStr>,
        timeout: Duration,
        cancellation: &Cancellation,
    ) -> Result<Self, PythonSelectionError> {
        let mut attempts = Vec::new();
        for candidate in CANDIDATES {
            let Some(executable) = find_executable(path, candidate.name) else {
                attempts.push(Attempt {
                    candidate: candidate.label(),
                    executable: None,
                    rejection: Rejection::NotFound,
                });
                continue;
            };
            let command = PythonCommand {
                executable,
                prefix: candidate.prefix.iter().map(OsString::from).collect(),
            };
            match command.probe(timeout, cancellation)? {
                None => return Ok(command),
                Some(rejection) => attempts.push(Attempt {
                    candidate: candidate.label(),
                    executable: Some(command.executable),
                    rejection,
                }),
            }
        }
        Err(PythonSelectionError::NoInterpreter(attempts))
    }

    /// A supervised command for this interpreter, with its prefix already in
    /// argv; callers append the script and its arguments.
    pub fn command(&self) -> ChildCommand {
        ChildCommand::new(&self.executable).args(&self.prefix)
    }

    /// Writes the argv prefix into `staging_dir` and returns the variables
    /// that hand this interpreter to the gates.
    ///
    /// # Errors
    ///
    /// The file could not be written, or (Windows) a prefix argument is not
    /// valid Unicode and so has no UTF-8 spelling for Bash to read.
    pub fn gate_environment(
        &self,
        staging_dir: &Path,
    ) -> io::Result<[(&'static str, OsString); 2]> {
        let file = staging_dir.join(ARGS_FILE_NAME);
        std::fs::write(&file, encode_prefix(&self.prefix)?)?;
        Ok([
            (PYTHON_VARIABLE, self.executable.clone().into_os_string()),
            (PYTHON_ARGS_FILE_VARIABLE, file.into_os_string()),
        ])
    }

    /// `None` when the interpreter qualifies.
    fn probe(
        &self,
        timeout: Duration,
        cancellation: &Cancellation,
    ) -> Result<Option<Rejection>, PythonSelectionError> {
        let completed = match self
            .command()
            .args(["-c", VERSION_PROBE])
            .repository_independent()
            .capture_limit(PROBE_CAPTURE_LIMIT)
            .deadline(timeout)
            .cancellation(cancellation.clone())
            .run()
        {
            Ok(completed) => completed,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Some(Rejection::NotFound));
            }
            Err(error) => return Ok(Some(Rejection::FailedToStart(error.source.to_string()))),
        };
        let status = match completed.termination {
            Termination::Cancelled(signal) => return Err(PythonSelectionError::Cancelled(signal)),
            Termination::TimedOut => return Ok(Some(Rejection::TimedOut(timeout))),
            Termination::Exited(status) => status,
        };
        if !status.success() {
            return Ok(Some(Rejection::Exited {
                code: status.code(),
                stderr: String::from_utf8_lossy(&completed.stderr).trim().to_owned(),
            }));
        }
        Ok(match parse_version(&completed.stdout) {
            Some(version) if version >= MINIMUM_VERSION => None,
            Some((major, minor)) => Some(Rejection::TooOld { major, minor }),
            None => Some(Rejection::UnreadableVersion(
                String::from_utf8_lossy(&completed.stdout).trim().to_owned(),
            )),
        })
    }
}

/// The `major.minor` line the version probe prints.
pub fn parse_version(stdout: &[u8]) -> Option<(u32, u32)> {
    let text = std::str::from_utf8(stdout).ok()?.trim();
    let (major, minor) = text.split_once('.')?;
    let digits = |part: &str| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse().ok())
            .flatten()
    };
    Some((digits(major)?, digits(minor)?))
}

/// Each argument followed by a NUL byte. Arguments cannot contain NUL, so the
/// encoding is unambiguous; an empty prefix is an empty file.
///
/// # Errors
///
/// (Windows) an argument is not valid Unicode.
pub fn encode_prefix(prefix: &[OsString]) -> io::Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for arg in prefix {
        encoded.extend_from_slice(&native_bytes(arg)?);
        encoded.push(0);
    }
    Ok(encoded)
}

#[cfg(unix)]
fn native_bytes(arg: &OsStr) -> io::Result<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    Ok(arg.as_bytes().to_vec())
}

#[cfg(not(unix))]
fn native_bytes(arg: &OsStr) -> io::Result<Vec<u8>> {
    arg.to_str()
        .map(|text| text.as_bytes().to_vec())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Python argument {arg:?} is not valid Unicode"),
            )
        })
}

/// The first `name` on `path` that is an executable file, as an absolute
/// native path.
///
/// Relative `PATH` entries are skipped: they would resolve against whatever
/// directory a child runs in. On Windows only `name.exe` is accepted, because
/// a `.bat` or `.cmd` launcher runs through `cmd.exe`, which re-parses its
/// arguments.
pub fn find_executable(path: Option<&OsStr>, name: &str) -> Option<PathBuf> {
    let file_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    std::env::split_paths(path?)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(&file_name))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// Windows App Execution Aliases are reparse points whose metadata is not a
/// plain file, so existence is the test; the probe proves the rest.
#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| !metadata.is_dir())
}
