//! The interactive shells a native Windows user may open, and whether each
//! one, started without its startup files, can find a program.
//!
//! Each of PowerShell, the MSYS family (Git Bash, otherwise a standalone
//! MSYS2), and Cygwin can derive a different `PATH` when it starts, so a
//! program the current process finds may be missing in one of them.
//! [`detect`] finds at most one installed shell per [`ShellFamily`] from
//! local metadata and the registry alone; [`probe`] asks one of them for a
//! program in a bounded, windowless subprocess that reads no profile, rc
//! file, or `BASH_ENV`.
//!
//! [`detect`] returns nothing off native Windows: WSL2 and every Unix host
//! has only its own environment to check.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::process::{ProcessError, TreeConfig, WINDOWS_CREATE_NO_WINDOW, run_command_in_tree, timeouts};

/// A group of shells that share one way of deriving `PATH`; [`detect`]
/// reports at most one shell of each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShellFamily {
    /// PowerShell 7 (`pwsh`), otherwise Windows PowerShell.
    PowerShell,
    /// Git Bash, otherwise a standalone MSYS2 (both run on `msys-2.0.dll`).
    Msys,
    /// Cygwin's Bash (`cygwin1.dll`).
    Cygwin,
}

/// Which installed shell a [`InstalledShell`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellKind {
    /// PowerShell 7 or later.
    Pwsh,
    /// Windows PowerShell 5.1.
    WindowsPowerShell,
    /// Git for Windows' Bash.
    GitBash,
    /// A standalone MSYS2 Bash.
    Msys2,
    /// Cygwin's Bash.
    Cygwin,
}

impl ShellKind {
    /// The family this shell belongs to.
    #[must_use]
    pub const fn family(self) -> ShellFamily {
        match self {
            Self::Pwsh | Self::WindowsPowerShell => ShellFamily::PowerShell,
            Self::GitBash | Self::Msys2 => ShellFamily::Msys,
            Self::Cygwin => ShellFamily::Cygwin,
        }
    }

    /// The shell's name as a user knows it (`Git Bash`, `PowerShell 7`, …).
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Pwsh => "PowerShell 7",
            Self::WindowsPowerShell => "Windows PowerShell",
            Self::GitBash => "Git Bash",
            Self::Msys2 => "MSYS2 Bash",
            Self::Cygwin => "Cygwin Bash",
        }
    }

    const fn is_powershell(self) -> bool {
        matches!(self.family(), ShellFamily::PowerShell)
    }
}

/// One installed shell, by its absolute executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledShell {
    pub kind: ShellKind,
    pub executable: PathBuf,
}

/// What a startup-free shell answered when asked for a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellLookup {
    /// The lookup completed and found the program.
    Found,
    /// The lookup completed and found no such program.
    Missing,
    /// The shell did not answer within its deadline and was terminated.
    TimedOut,
    /// The shell could not be started.
    SpawnFailed { kind: std::io::ErrorKind },
    /// The shell ended without the lookup's answer: any exit status other
    /// than found or missing, or termination by a signal (`None`).
    Failed { exit_code: Option<i32> },
}

/// The program name given to [`probe`] is not a plain file name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a shell lookup takes a plain program name of ASCII letters, digits, '.', '_' and '-'")]
pub struct InvalidProgramName;

/// The installed shells of a native Windows host, at most one per family,
/// in family order. Empty on every other OS, WSL2 included.
///
/// Reads only file metadata, the process environment, and two read-only
/// registry keys; nothing is started.
#[must_use]
pub fn detect() -> Vec<InstalledShell> {
    #[cfg(windows)]
    {
        discover(&DiscoveryInputs::from_host())
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Asks `shell`, started without its startup files in `working_dir`, whether
/// `program` is on the `PATH` it derives. The shell inherits this process's
/// environment, less `BASH_ENV` and `ENV`.
///
/// The probe runs the discovered absolute executable directly (never
/// `program`) with null stdin, drained output, no console window, and a
/// two-second deadline, after which its process tree is terminated.
///
/// ## Errors
///
/// [`InvalidProgramName`] when `program` is not a plain name; nothing is
/// started.
pub fn probe(shell: &InstalledShell, program: &str, working_dir: &Path) -> Result<ShellLookup, InvalidProgramName> {
    let mut command = probe_command(shell, program, working_dir)?;
    Ok(run_probe(&mut command, timeouts::SHELL_LOOKUP))
}

/// The exit status a probe script uses for "found".
const FOUND: i32 = 0;
/// The exit status a probe script uses for "missing"; anything else is a
/// failed probe.
const MISSING: i32 = 3;

/// The tree a probe runs in: its own process group, and no console window
/// on Windows.
const PROBE_TREE: TreeConfig = TreeConfig::process_group().windows_creation_flags(WINDOWS_CREATE_NO_WINDOW);

/// Variables a Bash or POSIX shell sources at startup even without rc files.
const STARTUP_VARIABLES: [&str; 2] = ["BASH_ENV", "ENV"];

fn probe_command(shell: &InstalledShell, program: &str, working_dir: &Path) -> Result<Command, InvalidProgramName> {
    let plain = !program.is_empty()
        && program.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !program.starts_with(['-', '.']);
    if !plain {
        return Err(InvalidProgramName);
    }
    let mut command = Command::new(&shell.executable);
    command.current_dir(working_dir);
    if shell.kind.is_powershell() {
        // Module auto-loading stays off and `Get-Command` is module-qualified,
        // so no module on `PSModulePath` is imported to answer.
        let script = format!(
            "$PSModuleAutoLoadingPreference = 'None'; \
             if (Microsoft.PowerShell.Core\\Get-Command -CommandType Application -Name '{program}' -ErrorAction SilentlyContinue) \
             {{ exit {FOUND} }}; exit {MISSING}"
        );
        command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &script]);
    } else {
        // `-p` also stops Bash reading `BASH_ENV`; the name travels as `$1`,
        // never as script text.
        let script = format!("type -P -- \"$1\" >/dev/null 2>&1 && exit {FOUND}; exit {MISSING}");
        command.args(["--noprofile", "--norc", "-p", "-c", &script, "sniff-shell-lookup", program]);
        for variable in STARTUP_VARIABLES {
            command.env_remove(variable);
        }
    }
    Ok(command)
}

fn run_probe(command: &mut Command, timeout: Duration) -> ShellLookup {
    match run_command_in_tree(command, timeout, PROBE_TREE) {
        Ok(output) => match output.status.code() {
            Some(FOUND) => ShellLookup::Found,
            Some(MISSING) => ShellLookup::Missing,
            exit_code => ShellLookup::Failed { exit_code },
        },
        Err(ProcessError::Timeout) => ShellLookup::TimedOut,
        Err(ProcessError::Spawn(error)) => ShellLookup::SpawnFailed { kind: error.kind() },
    }
}

/// Everything [`discover`] reads from the host, gathered by
/// [`DiscoveryInputs::from_host`] so the selection rules can be tested
/// against a fake layout on any OS.
#[derive(Debug, Default)]
#[cfg_attr(not(windows), allow(dead_code))]
struct DiscoveryInputs {
    /// `pwsh` from the full executable index.
    pwsh: Option<PathBuf>,
    /// `powershell` from the full executable index.
    powershell: Option<PathBuf>,
    /// `git` from the full executable index.
    git: Option<PathBuf>,
    /// `bash` from the full executable index (`PATH` first).
    bash: Option<PathBuf>,
    system_root: Option<PathBuf>,
    program_files: Option<PathBuf>,
    /// `SystemDrive` as written (`C:`), not yet a root.
    system_drive: Option<OsString>,
    /// Cygwin `rootdir` values from the registry, in lookup order.
    cygwin_roots: Vec<PathBuf>,
}

#[cfg(windows)]
impl DiscoveryInputs {
    fn from_host() -> Self {
        let index = crate::executable_index::ExecutableIndex::build();
        let bash = index.find_with_source("bash").map(|(path, _)| path);
        Self {
            pwsh: index.find_with_source("pwsh").map(|(path, _)| path),
            powershell: index.find_with_source("powershell").map(|(path, _)| path),
            git: index.find_with_source("git").map(|(path, _)| path),
            bash,
            system_root: std::env::var_os("SystemRoot").map(PathBuf::from),
            program_files: std::env::var_os("ProgramFiles").map(PathBuf::from),
            system_drive: std::env::var_os("SystemDrive"),
            cygwin_roots: cygwin_registry_roots(),
        }
    }
}

/// `SOFTWARE\Cygwin\setup` `rootdir`, from the current user then the
/// machine, each in the native then the 32-bit registry view. Read-only.
#[cfg(windows)]
fn cygwin_registry_roots() -> Vec<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};

    let mut roots = Vec::new();
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(r"SOFTWARE\Cygwin\setup", KEY_READ | view) else {
                continue;
            };
            if let Ok(root) = key.get_value::<String, _>("rootdir") {
                if !root.is_empty() {
                    roots.push(PathBuf::from(root));
                }
            }
        }
    }
    roots
}

/// The runtime a Bash executable is built on, read from the DLL beside it;
/// `None` for any other `bash.exe`, such as Windows' WSL launcher.
fn bash_runtime(executable: &Path) -> Option<ShellFamily> {
    if !executable.is_file() {
        return None;
    }
    let dir = executable.parent()?;
    if dir.join("msys-2.0.dll").is_file() {
        Some(ShellFamily::Msys)
    } else if dir.join("cygwin1.dll").is_file() {
        Some(ShellFamily::Cygwin)
    } else {
        None
    }
}

/// `drive` (`C:`) as the absolute root of its volume (`C:\`).
fn volume_root(drive: &OsStr) -> PathBuf {
    let mut root = drive.to_os_string();
    if !root.to_string_lossy().ends_with(['/', '\\']) {
        root.push(std::path::MAIN_SEPARATOR_STR);
    }
    PathBuf::from(root)
}

fn join(root: &Path, segments: &[&str]) -> PathBuf {
    segments.iter().fold(root.to_path_buf(), |path, segment| path.join(segment))
}

/// One place a shell may be installed, in preference order within its
/// family.
struct Candidate {
    kind: ShellKind,
    executable: PathBuf,
}

/// Picks at most one shell per family from `inputs`.
///
/// PowerShell 7 wins over Windows PowerShell. In the MSYS family Git Bash
/// wins over a standalone MSYS2; within each kind, and among Cygwin
/// installations, the indexed `bash` (`PATH` first) wins over a known
/// install layout. The first existing candidate of a family is its shell,
/// so a layout that is also the indexed `bash` counts once.
#[cfg_attr(not(windows), allow(dead_code))]
fn discover(inputs: &DiscoveryInputs) -> Vec<InstalledShell> {
    let mut shells = Vec::new();

    let windows_powershell = inputs
        .system_root
        .as_deref()
        .map(|root| join(root, &["System32", "WindowsPowerShell", "v1.0", "powershell.exe"]));
    let powershell = [
        inputs.pwsh.clone().map(|executable| Candidate { kind: ShellKind::Pwsh, executable }),
        inputs.powershell.clone().map(|executable| Candidate { kind: ShellKind::WindowsPowerShell, executable }),
        windows_powershell.map(|executable| Candidate { kind: ShellKind::WindowsPowerShell, executable }),
    ];
    if let Some(shell) = first_installed(powershell.into_iter().flatten(), |path| path.is_file()) {
        shells.push(shell);
    }

    let system_drive = inputs.system_drive.as_deref().map(volume_root);
    let indexed_bash = inputs.bash.as_deref();

    let mut git_roots: Vec<PathBuf> = inputs.git.as_deref().and_then(super::git_program::installation_root).into_iter().collect();
    git_roots.extend(inputs.program_files.as_deref().map(|files| files.join("Git")));
    let git_bash_layouts = git_roots.iter().map(|root| join(root, &["usr", "bin", "bash.exe"]));
    let msys2_layouts = system_drive
        .iter()
        .flat_map(|drive| [join(drive, &["msys64", "usr", "bin", "bash.exe"]), join(drive, &["msys32", "usr", "bin", "bash.exe"])]);

    // An indexed Bash on `msys-2.0.dll` is Git Bash when its installation
    // carries Git's launcher (`<root>\usr\bin\bash.exe` beside
    // `<root>\cmd\git.exe`), and a standalone MSYS2 otherwise.
    let indexed_msys = indexed_bash.filter(|path| bash_runtime(path) == Some(ShellFamily::Msys)).map(|path| {
        let root = path.parent().and_then(Path::parent).and_then(Path::parent);
        let is_git = root.is_some_and(|root| join(root, &["cmd", "git.exe"]).is_file());
        (if is_git { ShellKind::GitBash } else { ShellKind::Msys2 }, path.to_path_buf())
    });
    let indexed_of = |kind: ShellKind| indexed_msys.iter().filter(move |(found, _)| *found == kind).map(|(_, path)| path.clone());
    let msys = indexed_of(ShellKind::GitBash)
        .chain(git_bash_layouts)
        .map(|executable| Candidate { kind: ShellKind::GitBash, executable })
        .chain(indexed_of(ShellKind::Msys2).chain(msys2_layouts).map(|executable| Candidate { kind: ShellKind::Msys2, executable }));
    if let Some(shell) = first_installed(msys, |path| bash_runtime(path) == Some(ShellFamily::Msys)) {
        shells.push(shell);
    }

    let indexed_cygwin = indexed_bash.filter(|path| bash_runtime(path) == Some(ShellFamily::Cygwin));
    let cygwin_layouts = inputs.cygwin_roots.iter().cloned().chain(
        system_drive.iter().flat_map(|drive| [drive.join("cygwin64"), drive.join("cygwin")]),
    );
    let cygwin = indexed_cygwin
        .map(Path::to_path_buf)
        .into_iter()
        .chain(cygwin_layouts.map(|root| join(&root, &["bin", "bash.exe"])))
        .map(|executable| Candidate { kind: ShellKind::Cygwin, executable });
    if let Some(shell) = first_installed(cygwin, |path| bash_runtime(path) == Some(ShellFamily::Cygwin)) {
        shells.push(shell);
    }

    shells
}

fn first_installed(candidates: impl Iterator<Item = Candidate>, installed: impl Fn(&Path) -> bool) -> Option<InstalledShell> {
    candidates
        .into_iter()
        .find(|candidate| installed(&candidate.executable))
        .map(|candidate| InstalledShell { kind: candidate.kind, executable: candidate.executable })
}

#[cfg(test)]
#[path = "windows_shells_tests.rs"]
mod tests;
