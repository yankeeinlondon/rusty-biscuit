//! The `path` check: whether the current process, and on native Windows
//! each other installed shell started without its startup files, can find
//! `claudine`.
//!
//! The current process looks the name up exactly as Sniff's PATH-only
//! executable index does, inheriting its executable-mode and native-suffix
//! rules; nothing is launched. On native Windows the installed PowerShell,
//! MSYS (Git Bash or MSYS2), and Cygwin shells are found and asked by
//! `sniff::programs::windows_shells`, which never runs `claudine` or a
//! profile. WSL2 checks only its own Linux environment.

use std::path::{Path, PathBuf};

use sniff::programs::windows_shells::{InstalledShell, ShellLookup};

use super::report::{Finding, Scope, display_path};
use super::{CheckOutcome, Pass, Severity};

/// The executable every lookup asks for.
const PROGRAM: &str = "claudine";

/// The registry callback.
pub(crate) fn check_path(pass: &Pass<'_>) -> CheckOutcome {
    let shells = native_shell_lookups(pass.environment().launch_dir());
    assess(current_process_lookup().as_deref(), &shells)
}

/// Where this process's `PATH` finds `claudine`.
fn current_process_lookup() -> Option<PathBuf> {
    sniff::programs::ExecutableIndex::build_path_only().find(PROGRAM)
}

#[cfg(windows)]
fn native_shell_lookups(working_dir: &Path) -> Vec<(InstalledShell, ShellLookup)> {
    use sniff::programs::windows_shells;

    // `test-fixtures` only: a CLI test of another row names this variable so
    // its child starts none of the host's real shells, which cost seconds per
    // run. Every probe outcome is tested through `assess`.
    #[cfg(feature = "test-fixtures")]
    {
        const TEST_SKIP_SHELL_PROBES_ENV: &str = "CLAUDINE_TEST_DOCTOR_SKIP_SHELL_PROBES";
        if std::env::var_os(TEST_SKIP_SHELL_PROBES_ENV).is_some() {
            return Vec::new();
        }
    }

    windows_shells::detect()
        .into_iter()
        .filter_map(|shell| {
            let lookup = windows_shells::probe(&shell, PROGRAM, working_dir).ok()?;
            Some((shell, lookup))
        })
        .collect()
}

/// Only native Windows has other shells to ask.
#[cfg(not(windows))]
fn native_shell_lookups(_working_dir: &Path) -> Vec<(InstalledShell, ShellLookup)> {
    Vec::new()
}

/// The finding for a current-process lookup that found `current`, and for
/// each installed shell's answer. A shell that is not installed has no
/// entry and no finding.
pub(super) fn assess(current: Option<&Path>, shells: &[(InstalledShell, ShellLookup)]) -> CheckOutcome {
    let mut outcome = CheckOutcome::new(match current {
        Some(found) => format!("this process finds `claudine` at {}", display_path(found)),
        None => String::new(),
    });
    if current.is_none() {
        outcome.push(
            Finding::new(Severity::Error, Scope::General, "this process cannot find `claudine` on its PATH")
                .remedy("add the directory that holds the installed `claudine` executable to PATH (for a Cargo install, `~/.cargo/bin`)"),
        );
    }
    for (shell, lookup) in shells {
        outcome.push(shell_finding(shell, *lookup));
    }
    outcome
}

fn shell_finding(shell: &InstalledShell, lookup: ShellLookup) -> Finding {
    let name = shell.kind.display_name();
    let fresh = format!("a fresh {name} without your startup files");
    let restart = format!("check that {name} starts without its profile; doctor never runs startup files");
    let finding = match lookup {
        ShellLookup::Found => Finding::new(Severity::Ok, Scope::General, format!("{fresh} finds `claudine`")),
        ShellLookup::Missing => Finding::new(Severity::Warn, Scope::General, format!("{fresh} cannot find `claudine`"))
            .remedy(format!("add the directory that holds `claudine` to the user or system PATH {name} starts with, not only to a profile script")),
        ShellLookup::TimedOut => Finding::new(Severity::Warn, Scope::General, format!("{fresh} did not answer the lookup for `claudine` within 2 seconds"))
            .remedy(restart),
        ShellLookup::SpawnFailed { .. } => Finding::new(Severity::Warn, Scope::General, format!("{name} could not be started to look for `claudine`"))
            .remedy(format!("check that the installed {name} executable runs, or remove the broken installation")),
        ShellLookup::Failed { exit_code } => {
            let ended = exit_code.map_or_else(|| "was terminated".to_string(), |code| format!("exited with status {code}"));
            Finding::new(Severity::Warn, Scope::General, format!("{fresh} {ended} before answering the lookup for `claudine`"))
                .remedy(restart)
        }
    };
    finding.note("asked without its startup files: no profile, rc file, or BASH_ENV").at(&shell.executable)
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;
