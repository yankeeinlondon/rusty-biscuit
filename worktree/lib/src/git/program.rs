//! The program every Git process is started from.
//!
//! On Windows, the `git` found on `PATH` is usually Git for Windows'
//! `cmd\git.exe` (or `bin\git.exe`). That file is a launcher
//! (`compat/win32/git-wrapper.c` in git-for-windows/git): it sets four
//! environment variables and then starts `<prefix>\mingw64\bin\git.exe`, so
//! every Git call costs two process creations. [`command`] starts the real
//! binary itself and sets the same environment the launcher would, which
//! makes each call about 40% cheaper there (100 `git --version`: 2.7 s through
//! the launcher, 1.6 s direct). On every other platform, and whenever the
//! layout is not the launcher's, it is `Command::new("git")`.
//!
//! What the launcher sets for its child, and [`command`] sets the same way:
//!
//! - `MSYSTEM` to the prefix's system (`MINGW64` for `mingw64`), always;
//! - `PLINK_PROTOCOL=ssh`, when unset;
//! - `HOME` to `%HOMEDRIVE%%HOMEPATH%` when that is a directory other than
//!   System32, else `%USERPROFILE%`, when unset;
//! - `PATH` prefixed with `<top>\mingw64\bin;<top>\usr\bin;%HOME%\bin;`, so
//!   hooks, aliases, and `GIT_SSH_COMMAND` find Git's own `sh` and `ssh`.
//!
//! Git's own `git.exe` extends `PATH` only when `MSYSTEM` is unset, so with
//! these set it sees exactly what it sees under the launcher. The
//! `the_direct_program_sees_the_launchers_environment` test compares the two.

use std::process::Command;

/// A [`Command`] for Git: the real Git for Windows binary behind its
/// launcher when that layout is found (see the module docs), else `git`.
///
/// Every production Git process is built here, and test fixtures use it too,
/// so both pay one process creation per call. Counting is unchanged: the
/// caller still spawns through `calls::output` or calls `calls::started`.
pub fn command() -> Command {
    #[cfg(windows)]
    if let Some(direct) = windows::resolved() {
        return direct.command();
    }
    Command::new("git")
}

/// Whether [`name_installation_for_gix`] set `EXEPATH` in this process.
static SET_EXEPATH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Names the Git for Windows installation in this process's `EXEPATH`, so
/// the in-process `gix` reads find the system configuration without asking
/// Git. Does nothing off Windows, outside the launcher layout, or when
/// `EXEPATH` or `GIT_EXEC_PATH` is already set.
///
/// Without it, gix-path runs `git --exec-path` through the launcher with a
/// hidden console once per process: three process creations, about 40 ms of
/// every `wt` run on Windows outside Git Bash (which sets `EXEPATH` itself).
/// Both routes name the same `<top>\<prefix>` for the launcher layout, since
/// Git derives its exec path from that binary's location unless
/// `GIT_EXEC_PATH` overrides it.
///
/// The launcher does not set `EXEPATH`, so [`command`] and
/// [`forget_installation_hint`] keep it from the processes `wt` starts.
///
/// ## Safety
///
/// It writes the process environment: call it before any other thread
/// starts, first thing in `main`.
pub unsafe fn name_installation_for_gix() {
    #[cfg(windows)]
    {
        let unset = |name| std::env::var_os(name).is_none_or(|value| value.is_empty());
        if !(unset("EXEPATH") && unset("GIT_EXEC_PATH")) {
            return;
        }
        // `<top>\<prefix>\bin\git.exe`
        let Some(top) = windows::resolved().and_then(|direct| direct.executable.ancestors().nth(3)) else {
            return;
        };
        // SAFETY: the caller guarantees no other thread is running.
        unsafe { std::env::set_var("EXEPATH", top) };
        SET_EXEPATH.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Removes the `EXEPATH` [`name_installation_for_gix`] set from `child`'s
/// environment, so a process `wt` starts sees what it would have inherited.
pub fn forget_installation_hint(child: &mut Command) {
    if SET_EXEPATH.load(std::sync::atomic::Ordering::Relaxed) {
        child.env_remove("EXEPATH");
    }
}

/// The real Git binary behind a Git for Windows launcher, and what the
/// launcher would add to its environment.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Direct {
    executable: std::path::PathBuf,
    /// `<top>\<prefix>\bin;<top>\usr\bin;`
    path_prefix: std::ffi::OsString,
    msystem: &'static str,
}

/// The MSYS2 prefixes a Git for Windows install can carry, with the
/// `MSYSTEM` its launcher is built to set.
#[cfg_attr(not(windows), allow(dead_code))]
const PREFIXES: [(&str, &str); 3] = [("mingw64", "MINGW64"), ("clangarm64", "CLANGARM64"), ("mingw32", "MINGW32")];

/// The launcher layout around `found`, the `git.exe` that `PATH` names:
/// `<top>\cmd\git.exe` or `<top>\bin\git.exe` beside exactly one
/// `<top>\<prefix>\bin\git.exe`. `None` for anything else, including a
/// `<top>\etc\git-bash.config`, which makes the launcher also edit `MSYS`.
#[cfg_attr(not(windows), allow(dead_code))]
fn launcher_layout(found: &std::path::Path) -> Option<Direct> {
    let is = |name: Option<&std::ffi::OsStr>, expected: &str| name.and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case(expected));
    if !is(found.file_name(), "git.exe") {
        return None;
    }
    let launcher_dir = found.parent()?;
    if !(is(launcher_dir.file_name(), "cmd") || is(launcher_dir.file_name(), "bin")) {
        return None;
    }
    let top = launcher_dir.parent()?;
    if top.join("etc").join("git-bash.config").exists() {
        return None;
    }
    let mut candidates = PREFIXES.iter().filter(|(prefix, _)| top.join(prefix).join("bin").join("git.exe").is_file());
    let (prefix, msystem) = candidates.next()?;
    if candidates.next().is_some() {
        return None;
    }
    let mut path_prefix = std::ffi::OsString::new();
    for dir in [top.join(prefix).join("bin"), top.join("usr").join("bin")] {
        path_prefix.push(dir.as_os_str());
        path_prefix.push(";");
    }
    Some(Direct { executable: top.join(prefix).join("bin").join("git.exe"), path_prefix, msystem })
}

#[cfg(windows)]
mod windows {
    use std::ffi::{OsStr, OsString};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::OnceLock;

    use super::{Direct, launcher_layout};

    /// Resolved once per process: the launcher's location does not change
    /// while `wt` runs.
    pub(super) fn resolved() -> Option<&'static Direct> {
        static DIRECT: OnceLock<Option<Direct>> = OnceLock::new();
        DIRECT.get_or_init(resolve).as_ref()
    }

    fn resolve() -> Option<Direct> {
        let found = which::which("git").ok()?;
        // `Command::new("git")` looks in the application's directory and the
        // system directories before `PATH`; a `git.exe` there is what plain
        // `git` would run, so keep plain `git`.
        let ahead_of_path = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)).into_iter().chain(
            std::env::var_os("SystemRoot").map(PathBuf::from).into_iter().flat_map(|root| [root.join("System32"), root]),
        );
        for dir in ahead_of_path {
            if dir.join("git.exe").is_file() {
                return None;
            }
        }
        launcher_layout(&found)
    }

    impl Direct {
        pub(super) fn command(&self) -> Command {
            let mut command = Command::new(&self.executable);
            super::forget_installation_hint(&mut command);
            command.env("MSYSTEM", self.msystem);
            if std::env::var_os("PLINK_PROTOCOL").is_none_or(|value| value.is_empty()) {
                command.env("PLINK_PROTOCOL", "ssh");
            }
            let home = std::env::var_os("HOME").filter(|home| !home.is_empty()).or_else(|| {
                let home = launcher_home()?;
                command.env("HOME", &home);
                Some(home)
            });
            let mut path = self.path_prefix.clone();
            if let Some(home) = home {
                path.push(Path::new(&home).join("bin").as_os_str());
                path.push(";");
            }
            if let Some(inherited) = std::env::var_os("PATH") {
                path.push(inherited);
            }
            command.env("PATH", path);
            command
        }
    }

    /// The launcher's `HOME` when none is set: `%HOMEDRIVE%%HOMEPATH%` if it
    /// is an existing directory other than System32, else `%USERPROFILE%`.
    fn launcher_home() -> Option<OsString> {
        let profile = || std::env::var_os("USERPROFILE").filter(|value| !value.is_empty());
        let Some(home_path) = std::env::var_os("HOMEPATH").filter(|value| !value.is_empty()) else {
            return profile();
        };
        let mut joined = std::env::var_os("HOMEDRIVE").unwrap_or_default();
        joined.push(&home_path);
        let system32 = std::env::var_os("SystemRoot").map(|root| PathBuf::from(root).join("System32"));
        let is_system32 = system32.is_some_and(|dir| eq_ignore_case(dir.as_os_str(), &joined));
        if !is_system32 && Path::new(&joined).is_dir() { Some(joined) } else { profile() }
    }

    fn eq_ignore_case(a: &OsStr, b: &OsStr) -> bool {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn a_cmd_or_bin_launcher_resolves_to_the_prefix_binary() {
        let top = tempfile::tempdir().unwrap();
        touch(&top.path().join("mingw64/bin/git.exe"));
        for launcher in ["cmd", "bin"] {
            let direct = launcher_layout(&top.path().join(launcher).join("git.exe")).expect("launcher layout");
            assert_eq!(direct.executable, top.path().join("mingw64").join("bin").join("git.exe"));
            assert_eq!(direct.msystem, "MINGW64");
            let expected = format!("{};{};", top.path().join("mingw64").join("bin").display(), top.path().join("usr").join("bin").display());
            assert_eq!(direct.path_prefix, std::ffi::OsString::from(expected));
        }
    }

    #[test]
    fn an_arm64_install_sets_its_own_msystem() {
        let top = tempfile::tempdir().unwrap();
        touch(&top.path().join("clangarm64/bin/git.exe"));
        assert_eq!(launcher_layout(&top.path().join("cmd").join("git.exe")).expect("layout").msystem, "CLANGARM64");
    }

    #[test]
    fn anything_but_the_launcher_layout_keeps_plain_git() {
        let top = tempfile::tempdir().unwrap();
        touch(&top.path().join("mingw64/bin/git.exe"));
        // The real binary itself, a shim directory, and another file name.
        assert_eq!(launcher_layout(&top.path().join("mingw64").join("bin").join("git.exe")), None);
        assert_eq!(launcher_layout(&top.path().join("shims").join("git.exe")), None);
        assert_eq!(launcher_layout(&top.path().join("cmd").join("git")), None);
        // Two prefixes: which one the launcher was built for is unknown.
        touch(&top.path().join("mingw32/bin/git.exe"));
        assert_eq!(launcher_layout(&top.path().join("cmd").join("git.exe")), None);
    }

    #[test]
    fn a_git_bash_config_keeps_plain_git() {
        let top = tempfile::tempdir().unwrap();
        touch(&top.path().join("mingw64/bin/git.exe"));
        touch(&top.path().join("etc/git-bash.config"));
        assert_eq!(launcher_layout(&top.path().join("cmd").join("git.exe")), None);
    }

    #[test]
    fn a_missing_prefix_binary_keeps_plain_git() {
        let top = tempfile::tempdir().unwrap();
        assert_eq!(launcher_layout(&top.path().join("cmd").join("git.exe")), None);
    }

    /// What `wt`'s `main` does, in this test's own process (nextest runs
    /// each test in one): gix then takes the system prefix from `EXEPATH`,
    /// and it is the one `git --exec-path` implies, while Git children never
    /// see the variable.
    #[cfg(windows)]
    #[test]
    fn the_named_installation_is_the_prefix_git_reports_and_stays_out_of_git_children() {
        // SAFETY: the test's own process; no other thread reads the environment.
        unsafe {
            std::env::remove_var("EXEPATH");
            std::env::remove_var("GIT_EXEC_PATH");
            name_installation_for_gix();
        }
        assert!(std::env::var_os("EXEPATH").is_some(), "the launcher layout is expected on a Windows test host");
        let exec_path = command().arg("--exec-path").output().expect("git runs");
        let exec_path = String::from_utf8(exec_path.stdout).unwrap();
        // `<prefix>/libexec/git-core`, as gix-path derives the prefix from it.
        let reported = Path::new(exec_path.trim()).ancestors().nth(2).expect("a prefix").to_path_buf();
        let named = gix::path::env::system_prefix().expect("a system prefix");
        assert!(
            crate::remove::admin_entry::same_location(named, &reported),
            "{} vs {}",
            named.display(),
            reported.display()
        );
        let env = command().args(["-c", "alias.e=!env", "e"]).output().expect("git runs");
        let env = String::from_utf8_lossy(&env.stdout);
        assert!(env.contains("MSYSTEM="), "the alias ran: {env}");
        assert!(!env.lines().any(|line| line.starts_with("EXEPATH=")), "{env}");
    }

    /// The environment Git's hooks, aliases, and transports see is the same
    /// whether `wt` starts the launcher or the binary behind it.
    #[cfg(windows)]
    #[test]
    fn the_direct_program_sees_the_launchers_environment() {
        let Some(direct) = windows::resolved() else {
            let found = which::which("git").expect("git on PATH");
            panic!("expected Git for Windows' launcher layout around {}", found.display());
        };
        let launcher = which::which("git").expect("git on PATH");
        assert_ne!(launcher, direct.executable);
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("alias.gitconfig");
        // `env` output goes to a file: sh's stdout is the same either way.
        fs::write(&config, "[alias]\n\tdump = \"!env | sort > \\\"$DUMP\\\"\"\n").unwrap();
        let dump = |mut command: Command, name: &str| {
            let out = dir.path().join(name);
            let status = command
                .current_dir(dir.path())
                .arg("-c")
                .arg(format!("include.path={}", config.display()))
                .arg("dump")
                .env("DUMP", out.to_string_lossy().replace('\\', "/"))
                .status()
                .expect("git runs");
            assert!(status.success());
            fs::read_to_string(out).unwrap().replace(name, "DUMP")
        };
        let via_launcher = dump(Command::new(&launcher), "launcher.txt");
        let direct_env = dump(command(), "direct.txt");
        assert_eq!(direct_env, via_launcher);
    }
}
