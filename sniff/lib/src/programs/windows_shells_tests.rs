//! Shell discovery against fake install layouts, the probe's argument and
//! environment policy, and the probe's outcomes through child fixtures
//! (this test binary re-executed), so no real shell or profile ever runs.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::*;

/// Creates `path` (and its parents) as an empty file.
fn touch(path: &Path) -> PathBuf {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create parents");
    fs::write(path, b"").expect("write file");
    path.to_path_buf()
}

/// A Bash at `root/segments…` with the runtime DLL `dll` beside it.
fn bash_with(root: &Path, segments: &[&str], dll: Option<&str>) -> PathBuf {
    let bash = touch(&join(root, segments));
    if let Some(dll) = dll {
        touch(&bash.parent().expect("bin").join(dll));
    }
    bash
}

fn kinds(shells: &[InstalledShell]) -> Vec<(ShellKind, PathBuf)> {
    shells.iter().map(|shell| (shell.kind, shell.executable.clone())).collect()
}

// ---------------------------------------------------------------- discovery

#[test]
fn nothing_installed_finds_no_shell() {
    let root = tempfile::tempdir().expect("tempdir");
    let inputs = DiscoveryInputs {
        system_root: Some(root.path().join("Windows")),
        program_files: Some(root.path().join("Program Files")),
        system_drive: Some(root.path().as_os_str().to_os_string()),
        ..DiscoveryInputs::default()
    };
    assert_eq!(discover(&inputs), Vec::new());
}

#[cfg(not(windows))]
#[test]
fn detect_finds_nothing_off_native_windows() {
    assert_eq!(detect(), Vec::new());
}

#[test]
fn pwsh_is_preferred_over_windows_powershell() {
    let root = tempfile::tempdir().expect("tempdir");
    let pwsh = touch(&root.path().join("pwsh/pwsh.exe"));
    let powershell = touch(&root.path().join("ps/powershell.exe"));
    let inputs = DiscoveryInputs { pwsh: Some(pwsh.clone()), powershell: Some(powershell), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::Pwsh, pwsh)]);
}

#[test]
fn indexed_windows_powershell_is_used_without_pwsh() {
    let root = tempfile::tempdir().expect("tempdir");
    let powershell = touch(&root.path().join("ps/powershell.exe"));
    let inputs = DiscoveryInputs {
        pwsh: Some(root.path().join("gone/pwsh.exe")),
        powershell: Some(powershell.clone()),
        ..DiscoveryInputs::default()
    };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::WindowsPowerShell, powershell)]);
}

#[test]
fn windows_powershell_falls_back_to_its_system_root_layout() {
    let root = tempfile::tempdir().expect("tempdir");
    let system_root = root.path().join("Windows");
    let powershell = touch(&join(&system_root, &["System32", "WindowsPowerShell", "v1.0", "powershell.exe"]));
    let inputs = DiscoveryInputs { system_root: Some(system_root), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::WindowsPowerShell, powershell)]);
}

#[test]
fn git_bash_is_found_from_the_git_launcher_root() {
    let root = tempfile::tempdir().expect("tempdir");
    let git_root = root.path().join("Tools/Git");
    let git = touch(&join(&git_root, &["cmd", "git.exe"]));
    let bash = bash_with(&git_root, &["usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let inputs = DiscoveryInputs { git: Some(git), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::GitBash, bash)]);
}

#[test]
fn git_bash_is_found_under_program_files() {
    let root = tempfile::tempdir().expect("tempdir");
    let files = root.path().join("Program Files");
    let bash = bash_with(&files.join("Git"), &["usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let inputs = DiscoveryInputs { program_files: Some(files), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::GitBash, bash)]);
}

#[test]
fn git_bash_is_preferred_over_a_standalone_msys2() {
    let root = tempfile::tempdir().expect("tempdir");
    let files = root.path().join("Program Files");
    let git_bash = bash_with(&files.join("Git"), &["usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let drive = root.path().join("drive");
    let msys2 = bash_with(&drive, &["msys64", "usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let inputs = DiscoveryInputs {
        program_files: Some(files),
        system_drive: Some(drive.into_os_string()),
        bash: Some(msys2),
        ..DiscoveryInputs::default()
    };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::GitBash, git_bash)]);
}

#[test]
fn standalone_msys2_layouts_are_checked_in_order() {
    let root = tempfile::tempdir().expect("tempdir");
    let drive = root.path().join("drive");
    let msys32 = bash_with(&drive, &["msys32", "usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let inputs = DiscoveryInputs { system_drive: Some(drive.clone().into_os_string()), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::Msys2, msys32)]);

    let msys64 = bash_with(&drive, &["msys64", "usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::Msys2, msys64)]);
}

#[test]
fn the_indexed_bash_is_preferred_within_its_kind() {
    let root = tempfile::tempdir().expect("tempdir");
    let files = root.path().join("Program Files");
    bash_with(&files.join("Git"), &["usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let portable = root.path().join("PortableGit");
    touch(&join(&portable, &["cmd", "git.exe"]));
    let portable_bash = bash_with(&portable, &["usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let inputs = DiscoveryInputs { program_files: Some(files), bash: Some(portable_bash.clone()), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::GitBash, portable_bash)]);

    let drive = root.path().join("drive");
    bash_with(&drive, &["msys64", "usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let custom_msys2 = bash_with(&root.path().join("tools/msys2"), &["usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let inputs = DiscoveryInputs { system_drive: Some(drive.into_os_string()), bash: Some(custom_msys2.clone()), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::Msys2, custom_msys2)]);
}

#[test]
fn a_bash_without_an_msys_or_cygwin_runtime_is_not_a_shell() {
    let root = tempfile::tempdir().expect("tempdir");
    let system_root = root.path().join("Windows");
    let wsl_launcher = touch(&join(&system_root, &["System32", "bash.exe"]));
    let files = root.path().join("Program Files");
    bash_with(&files.join("Git"), &["usr", "bin", "bash.exe"], None);
    let inputs = DiscoveryInputs { bash: Some(wsl_launcher), program_files: Some(files), ..DiscoveryInputs::default() };
    assert_eq!(discover(&inputs), Vec::new());
}

#[test]
fn cygwin_registry_roots_precede_the_drive_layouts() {
    let root = tempfile::tempdir().expect("tempdir");
    let drive = root.path().join("drive");
    let cygwin = bash_with(&drive, &["cygwin", "bin", "bash.exe"], Some("cygwin1.dll"));
    let inputs = DiscoveryInputs { system_drive: Some(drive.clone().into_os_string()), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::Cygwin, cygwin)]);

    let cygwin64 = bash_with(&drive, &["cygwin64", "bin", "bash.exe"], Some("cygwin1.dll"));
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::Cygwin, cygwin64)]);

    let registered = root.path().join("opt/cyg");
    let registered_bash = bash_with(&registered, &["bin", "bash.exe"], Some("cygwin1.dll"));
    let inputs = DiscoveryInputs {
        system_drive: Some(drive.into_os_string()),
        cygwin_roots: vec![root.path().join("missing"), registered],
        ..DiscoveryInputs::default()
    };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::Cygwin, registered_bash)]);
}

#[test]
fn the_indexed_cygwin_bash_is_preferred_and_needs_its_runtime() {
    let root = tempfile::tempdir().expect("tempdir");
    let drive = root.path().join("drive");
    bash_with(&drive, &["cygwin64", "bin", "bash.exe"], Some("cygwin1.dll"));
    let indexed = bash_with(&root.path().join("cyg"), &["bin", "bash.exe"], Some("cygwin1.dll"));
    let inputs = DiscoveryInputs { system_drive: Some(drive.into_os_string()), bash: Some(indexed.clone()), ..DiscoveryInputs::default() };
    assert_eq!(kinds(&discover(&inputs)), vec![(ShellKind::Cygwin, indexed)]);

    let without_runtime = bash_with(&root.path().join("fake"), &["bin", "bash.exe"], None);
    let inputs = DiscoveryInputs { bash: Some(without_runtime), cygwin_roots: vec![root.path().join("fake")], ..DiscoveryInputs::default() };
    assert_eq!(discover(&inputs), Vec::new());
}

#[test]
fn every_family_is_reported_once_in_family_order() {
    let root = tempfile::tempdir().expect("tempdir");
    let drive = root.path().join("drive");
    let cygwin = bash_with(&drive, &["cygwin64", "bin", "bash.exe"], Some("cygwin1.dll"));
    let files = root.path().join("Program Files");
    let git_bash = bash_with(&files.join("Git"), &["usr", "bin", "bash.exe"], Some("msys-2.0.dll"));
    let system_root = root.path().join("Windows");
    let powershell = touch(&join(&system_root, &["System32", "WindowsPowerShell", "v1.0", "powershell.exe"]));
    let inputs = DiscoveryInputs {
        system_root: Some(system_root),
        program_files: Some(files),
        system_drive: Some(drive.into_os_string()),
        // The Program Files Git Bash is also the indexed `bash`: one entry.
        bash: Some(git_bash.clone()),
        ..DiscoveryInputs::default()
    };
    assert_eq!(
        kinds(&discover(&inputs)),
        vec![(ShellKind::WindowsPowerShell, powershell), (ShellKind::GitBash, git_bash), (ShellKind::Cygwin, cygwin)]
    );
    let families: Vec<ShellFamily> = discover(&inputs).iter().map(|shell| shell.kind.family()).collect();
    assert_eq!(families, vec![ShellFamily::PowerShell, ShellFamily::Msys, ShellFamily::Cygwin]);
}

#[test]
fn a_system_drive_is_joined_as_a_volume_root() {
    let separator = std::path::MAIN_SEPARATOR_STR;
    assert_eq!(volume_root(OsStr::new("C:")), PathBuf::from(format!("C:{separator}")));
    assert_eq!(volume_root(OsStr::new("C:\\")), PathBuf::from("C:\\"));
    assert_eq!(volume_root(OsStr::new("/mnt/c/")), PathBuf::from("/mnt/c/"));
}

// ------------------------------------------------------------ probe policy

fn args_of(command: &Command) -> Vec<String> {
    command.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect()
}

#[test]
fn a_powershell_probe_runs_no_profile_and_no_module_auto_loading() {
    let working_dir = tempfile::tempdir().expect("tempdir");
    for kind in [ShellKind::Pwsh, ShellKind::WindowsPowerShell] {
        let shell = InstalledShell { kind, executable: PathBuf::from("C:/ps/pwsh.exe") };
        let command = probe_command(&shell, "claudine", working_dir.path()).expect("plain name");
        assert_eq!(command.get_program(), OsStr::new("C:/ps/pwsh.exe"));
        assert_eq!(command.get_current_dir(), Some(working_dir.path()));
        let args = args_of(&command);
        assert_eq!(&args[..4], ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"]);
        assert_eq!(
            args[4],
            "$PSModuleAutoLoadingPreference = 'None'; \
             if (Microsoft.PowerShell.Core\\Get-Command -CommandType Application -Name 'claudine' -ErrorAction SilentlyContinue) \
             { exit 0 }; exit 3"
        );
        assert_eq!(args.len(), 5);
        assert_eq!(command.get_envs().count(), 0, "PowerShell inherits the environment unchanged");
    }
}

#[test]
fn a_bash_probe_reads_no_startup_file_and_passes_the_name_as_an_argument() {
    let working_dir = tempfile::tempdir().expect("tempdir");
    for kind in [ShellKind::GitBash, ShellKind::Msys2, ShellKind::Cygwin] {
        let shell = InstalledShell { kind, executable: PathBuf::from("C:/Git/usr/bin/bash.exe") };
        let command = probe_command(&shell, "claudine", working_dir.path()).expect("plain name");
        assert_eq!(command.get_program(), OsStr::new("C:/Git/usr/bin/bash.exe"));
        assert_eq!(command.get_current_dir(), Some(working_dir.path()));
        assert_eq!(
            args_of(&command),
            [
                "--noprofile",
                "--norc",
                "-p",
                "-c",
                "type -P -- \"$1\" >/dev/null 2>&1 && exit 0; exit 3",
                "sniff-shell-lookup",
                "claudine",
            ]
        );
        let removed: Vec<(String, bool)> =
            command.get_envs().map(|(key, value)| (key.to_string_lossy().into_owned(), value.is_none())).collect();
        assert_eq!(removed, [("BASH_ENV".to_string(), true), ("ENV".to_string(), true)]);
    }
}

#[test]
fn a_probe_refuses_anything_but_a_plain_program_name() {
    let shell = InstalledShell { kind: ShellKind::GitBash, executable: PathBuf::from("bash.exe") };
    for name in ["", "-c", ".hidden", "two words", "a;b", "a'b", "../claudine", "a/b", "a\\b", "$(id)", "é"] {
        assert_eq!(probe_command(&shell, name, Path::new(".")).err(), Some(InvalidProgramName), "{name:?}");
        assert_eq!(probe(&shell, name, Path::new(".")), Err(InvalidProgramName), "{name:?}");
    }
    for name in ["claudine", "claudine.exe", "my_tool-2"] {
        assert!(probe_command(&shell, name, Path::new(".")).is_ok(), "{name:?}");
    }
}

#[test]
fn a_probe_runs_windowless_in_its_own_process_group() {
    assert_eq!(PROBE_TREE, TreeConfig::process_group().windows_creation_flags(WINDOWS_CREATE_NO_WINDOW));
    assert_eq!(timeouts::SHELL_LOOKUP, Duration::from_secs(2));
}

// --------------------------------------------------------- probe outcomes

const EXIT_CHILD: &str = "programs::windows_shells::tests::child_exits_with_requested_code";
const SLEEP_CHILD: &str = "programs::windows_shells::tests::child_records_its_pid_and_sleeps";
#[cfg(unix)]
const SIGNAL_CHILD: &str = "programs::windows_shells::tests::child_kills_itself";
const EXIT_CODE_VAR: &str = "SNIFF_SHELL_PROBE_EXIT";
const PID_FILE_VAR: &str = "SNIFF_SHELL_PROBE_PID_FILE";

fn fixture(name: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command.args([name, "--exact", "--ignored", "--nocapture"]);
    command
}

#[test]
#[ignore = "subprocess fixture invoked by the shell probe tests"]
fn child_exits_with_requested_code() {
    let code = std::env::var(EXIT_CODE_VAR).ok().and_then(|code| code.parse().ok()).unwrap_or(99);
    std::process::exit(code);
}

#[test]
#[ignore = "subprocess fixture invoked by the shell probe tests"]
fn child_records_its_pid_and_sleeps() {
    if let Some(path) = std::env::var_os(PID_FILE_VAR) {
        fs::write(path, std::process::id().to_string()).expect("write pid");
    }
    std::thread::sleep(Duration::from_secs(30));
}

#[cfg(unix)]
#[test]
#[ignore = "subprocess fixture invoked by the shell probe tests"]
fn child_kills_itself() {
    // SAFETY: signals this fixture process itself.
    unsafe { libc::kill(libc::getpid(), libc::SIGKILL) };
}

fn outcome_for_exit(code: i32) -> ShellLookup {
    let mut command = fixture(EXIT_CHILD);
    command.env(EXIT_CODE_VAR, code.to_string());
    run_probe(&mut command, Duration::from_secs(20))
}

#[test]
fn exit_zero_is_found() {
    assert_eq!(outcome_for_exit(0), ShellLookup::Found);
}

#[test]
fn exit_three_is_missing() {
    assert_eq!(outcome_for_exit(3), ShellLookup::Missing);
}

#[test]
fn any_other_exit_is_a_failed_probe() {
    assert_eq!(outcome_for_exit(1), ShellLookup::Failed { exit_code: Some(1) });
    assert_eq!(outcome_for_exit(2), ShellLookup::Failed { exit_code: Some(2) });
}

#[cfg(unix)]
#[test]
fn a_terminated_shell_is_a_failed_probe_without_an_exit_code() {
    let mut command = fixture(SIGNAL_CHILD);
    assert_eq!(run_probe(&mut command, Duration::from_secs(20)), ShellLookup::Failed { exit_code: None });
}

#[test]
fn a_shell_that_cannot_start_is_a_spawn_failure() {
    let root = tempfile::tempdir().expect("tempdir");
    let shell = InstalledShell { kind: ShellKind::GitBash, executable: root.path().join("gone").join("bash.exe") };
    assert_eq!(
        probe(&shell, "claudine", root.path()),
        Ok(ShellLookup::SpawnFailed { kind: std::io::ErrorKind::NotFound })
    );
}

/// The deadline is injected short; the outcome must be the timeout branch,
/// and the terminated child must be gone when the probe returns.
#[test]
fn a_hung_shell_times_out_and_is_terminated() {
    let root = tempfile::tempdir().expect("tempdir");
    let pid_file = root.path().join("pid");
    let mut command = fixture(SLEEP_CHILD);
    command.env(PID_FILE_VAR, &pid_file);
    let start = Instant::now();

    assert_eq!(run_probe(&mut command, Duration::from_millis(1500)), ShellLookup::TimedOut);
    assert!(start.elapsed() < Duration::from_secs(10), "terminated at the deadline, not after the child's 30 s");

    let pid: u32 = fs::read_to_string(&pid_file).expect("the child recorded its pid").trim().parse().expect("numeric pid");
    let pid = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    assert!(system.process(pid).is_none(), "the timed-out child was reaped");
}
