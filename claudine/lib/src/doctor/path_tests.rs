//! The `path` row: injected shell answers for every outcome, and the
//! current-process lookup in a child of this test binary whose `PATH` names
//! only a temporary directory, so the real suffix and executable-mode rules
//! decide.

use std::path::{Path, PathBuf};
use std::process::Command;

use sniff::programs::windows_shells::{InstalledShell, ShellKind, ShellLookup};

use super::super::report::row;
use super::super::{DoctorRow, Severity};
use super::assess;

fn path_row(current: Option<&Path>, shells: &[(InstalledShell, ShellLookup)]) -> DoctorRow {
    row("path", assess(current, shells), None)
}

fn shell(kind: ShellKind) -> InstalledShell {
    InstalledShell { kind, executable: PathBuf::from(format!("C:/Shells/{kind:?}/shell.exe")) }
}

#[test]
fn a_found_executable_passes_and_names_where_it_is() {
    let row = path_row(Some(Path::new("/opt/bin/claudine")), &[]);
    assert_eq!(row.severity, Severity::Ok);
    assert_eq!(row.summary, "this process finds `claudine` at /opt/bin/claudine");
    assert_eq!(row.detail, "", "an absent shell adds nothing");
    assert!(!row.fixable);
}

#[test]
fn a_miss_in_this_process_is_an_error_with_a_remedy() {
    let row = path_row(None, &[]);
    assert_eq!(row.severity, Severity::Error);
    assert_eq!(row.summary, "this process cannot find `claudine` on its PATH");
    assert!(row.detail.contains("Remedy: add the directory that holds the installed `claudine` executable to PATH"), "{row:#?}");
}

#[test]
fn each_shell_outcome_is_distinct_and_never_more_than_a_warning() {
    let cases = [
        (ShellLookup::Found, Severity::Ok, "a fresh Git Bash without your startup files finds `claudine`", None),
        (
            ShellLookup::Missing,
            Severity::Warn,
            "a fresh Git Bash without your startup files cannot find `claudine`",
            Some("Remedy: add the directory that holds `claudine` to the user or system PATH Git Bash starts with"),
        ),
        (
            ShellLookup::TimedOut,
            Severity::Warn,
            "a fresh Git Bash without your startup files did not answer the lookup for `claudine` within 2 seconds",
            Some("Remedy: check that Git Bash starts without its profile"),
        ),
        (
            ShellLookup::SpawnFailed { kind: std::io::ErrorKind::PermissionDenied },
            Severity::Warn,
            "Git Bash could not be started to look for `claudine`",
            Some("Remedy: check that the installed Git Bash executable runs"),
        ),
        (
            ShellLookup::Failed { exit_code: Some(127) },
            Severity::Warn,
            "a fresh Git Bash without your startup files exited with status 127 before answering",
            Some("Remedy: check that Git Bash starts without its profile"),
        ),
        (
            ShellLookup::Failed { exit_code: None },
            Severity::Warn,
            "a fresh Git Bash without your startup files was terminated before answering",
            Some("Remedy: check that Git Bash starts without its profile"),
        ),
    ];
    for (lookup, severity, summary, remedy) in cases {
        let row = path_row(Some(Path::new("/opt/bin/claudine")), &[(shell(ShellKind::GitBash), lookup)]);
        assert_eq!(row.severity, severity, "{lookup:?}: {row:#?}");
        assert!(row.detail.contains(&format!("C:/Shells/GitBash/shell.exe: {summary}")), "{lookup:?}: {row:#?}");
        assert!(row.detail.contains("asked without its startup files: no profile, rc file, or BASH_ENV"), "{row:#?}");
        match remedy {
            Some(remedy) => assert!(row.detail.contains(remedy), "{lookup:?}: {row:#?}"),
            None => assert!(!row.detail.contains("Remedy:"), "{row:#?}"),
        }
    }
}

#[test]
fn shell_findings_follow_path_order_and_a_current_miss_still_leads() {
    let shells = [
        (shell(ShellKind::Pwsh), ShellLookup::Found),
        (shell(ShellKind::GitBash), ShellLookup::Missing),
        (shell(ShellKind::Cygwin), ShellLookup::TimedOut),
    ];
    let row = path_row(None, &shells);
    assert_eq!(row.severity, Severity::Error);
    assert_eq!(row.summary, "this process cannot find `claudine` on its PATH");
    let lines: Vec<&str> = row.detail.lines().filter(|line| !line.starts_with("Remedy") && !line.starts_with("asked")).collect();
    assert_eq!(
        lines,
        [
            "this process cannot find `claudine` on its PATH",
            "C:/Shells/Cygwin/shell.exe: a fresh Cygwin Bash without your startup files did not answer the lookup for `claudine` within 2 seconds",
            "C:/Shells/GitBash/shell.exe: a fresh Git Bash without your startup files cannot find `claudine`",
            "C:/Shells/Pwsh/shell.exe: a fresh PowerShell 7 without your startup files finds `claudine`",
        ]
    );

    let row = path_row(Some(Path::new("/opt/bin/claudine")), &shells);
    assert_eq!(row.severity, Severity::Warn, "another shell's miss cannot fail the run");
    assert_eq!(row.summary, "a fresh Git Bash without your startup files cannot find `claudine` (+1 more)");
}

// ------------------------------------------- current-process lookup (F2)

const LOOKUP_CHILD: &str = "doctor::path::tests::child_reports_its_lookup";

#[test]
#[ignore = "subprocess fixture invoked by the current-process lookup tests"]
fn child_reports_its_lookup() {
    match super::current_process_lookup() {
        Some(found) => eprintln!("found:{}", found.display()),
        None => eprintln!("missing"),
    }
}

/// What the lookup reports (on stderr, apart from the harness's progress
/// lines) in a child whose `PATH` is exactly `dir`.
fn lookup_with_path(dir: &Path) -> String {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([LOOKUP_CHILD, "--exact", "--ignored", "--nocapture", "--test-threads=1"])
        .env("PATH", dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    stderr
        .lines()
        .find_map(|line| line.strip_prefix("found:").map(|path| format!("found:{path}")).or_else(|| (line == "missing").then(|| line.to_string())))
        .unwrap_or_else(|| panic!("no lookup line in {stderr:?}"))
}

#[cfg(unix)]
fn install_executable(dir: &Path, name: &str, mode: u32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
    path
}

#[test]
fn an_empty_path_directory_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(lookup_with_path(dir.path()), "missing");
}

#[cfg(unix)]
#[test]
fn an_executable_on_path_is_found_and_a_non_executable_file_is_not() {
    let dir = tempfile::tempdir().unwrap();
    let not_executable = install_executable(dir.path(), "claudine", 0o644);
    assert_eq!(lookup_with_path(dir.path()), "missing");

    std::fs::remove_file(&not_executable).unwrap();
    let executable = install_executable(dir.path(), "claudine", 0o755);
    assert_eq!(lookup_with_path(dir.path()), format!("found:{}", executable.display()));
}

#[cfg(windows)]
#[test]
fn a_native_suffix_is_required_on_windows() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("claudine"), b"").unwrap();
    assert_eq!(lookup_with_path(dir.path()), "missing");

    let executable = dir.path().join("claudine.exe");
    std::fs::write(&executable, b"").unwrap();
    let found = lookup_with_path(dir.path());
    let found = found.strip_prefix("found:").unwrap_or_else(|| panic!("expected a hit, got {found}"));
    assert!(found.to_ascii_lowercase().ends_with("claudine.exe"), "{found}");
}
