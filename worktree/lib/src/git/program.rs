//! The program every Git process is started from.
//!
//! On Windows, the `git` found on `PATH` is one of two files in a Git for
//! Windows installation `<top>`, and [`command`] treats them differently:
//!
//! - **The launcher**, `<top>\cmd\git.exe` (or `<top>\bin\git.exe`), which
//!   the installer puts on `PATH`. It is `compat/win32/git-wrapper.c` in
//!   git-for-windows/git: it sets four environment variables and then starts
//!   `<top>\<prefix>\bin\git.exe`, so every Git call costs two process
//!   creations. [`command`] starts that binary itself and sets the same
//!   environment the launcher would, which makes each call about 40% cheaper
//!   (100 `git --version`: 2.7 s through the launcher, 1.6 s direct).
//! - **The binary itself**, `<top>\<prefix>\bin\git.exe`, when `PATH` names
//!   that directory. [`command`] runs it with the inherited environment,
//!   exactly as `Command::new("git")` would: the launcher's variables belong
//!   to the launcher, and a binary `PATH` reaches directly never had them.
//!
//! `<prefix>` is the MSYS2 environment the installation was built for:
//! `mingw64` (x86_64 before Git for Windows 2.56), `ucrt64` (x86_64 from
//! 2.56), `clangarm64` (arm64), or `mingw32`. On every other platform, and
//! for any other layout, [`command`] is `Command::new("git")`.
//!
//! What the launcher sets for its child, and [`command`] sets the same way
//! for the launcher entry only:
//!
//! - `MSYSTEM` to the prefix's system (`MINGW64` for `mingw64`), always;
//! - `PLINK_PROTOCOL=ssh`, when unset;
//! - `HOME` to `%HOMEDRIVE%%HOMEPATH%` when that is a directory other than
//!   System32, else `%USERPROFILE%`, when unset;
//! - `PATH` prefixed with `<top>\<prefix>\bin;<top>\usr\bin;%HOME%\bin;`,
//!   so hooks, aliases, and `GIT_SSH_COMMAND` find Git's own `sh` and `ssh`.
//!
//! Git's own `git.exe` extends `PATH` only when `MSYSTEM` is unset, so with
//! these set it sees exactly what it sees under the launcher. The Windows
//! tests compare each entry with what it stands for: the launcher entry with
//! the sibling `cmd\git.exe`, the direct entry with the found binary run
//! unwrapped.

use std::process::Command;

/// A [`Command`] for Git: the real Git for Windows binary when a recognized
/// installation is found (see the module docs), else `git`.
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
/// Git. Does nothing off Windows, outside a recognized installation (either
/// entry in the module docs), or when `EXEPATH` or `GIT_EXEC_PATH` is
/// already set.
///
/// Without it, gix-path runs `git --exec-path` (through the launcher, when
/// that is what `PATH` names) with a hidden console once per process: up to
/// three process creations, about 40 ms of every `wt` run on Windows outside
/// Git Bash (which sets `EXEPATH` itself). Both routes name the same
/// `<top>\<prefix>`, since Git derives its exec path from that binary's
/// location unless `GIT_EXEC_PATH` overrides it. gix-path reads `EXEPATH`
/// for every prefix [`command`] recognizes, and asks Git when `<top>` holds
/// more than one, which [`command`] rejects too.
///
/// Neither entry sets `EXEPATH`, so [`command`] and
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

/// The real Git binary of a Git for Windows installation, how `PATH`
/// reached it, and what the launcher would add to its environment.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Direct {
    /// `<top>\<prefix>\bin\git.exe`
    executable: std::path::PathBuf,
    entry: Entry,
    /// `<top>\<prefix>\bin;<top>\usr\bin;`
    path_prefix: std::ffi::OsString,
    msystem: &'static str,
}

/// Which file of the installation `PATH` named; it decides whether
/// [`command`] adds the launcher's environment.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// `<top>\cmd\git.exe` or `<top>\bin\git.exe`
    Launcher,
    /// `<top>\<prefix>\bin\git.exe` itself
    Direct,
}

/// The MSYS2 prefixes a Git for Windows install can carry, with the
/// `MSYSTEM` its launcher is built to set.
#[cfg_attr(not(windows), allow(dead_code))]
const PREFIXES: [(&str, &str); 4] =
    [("mingw64", "MINGW64"), ("ucrt64", "UCRT64"), ("clangarm64", "CLANGARM64"), ("mingw32", "MINGW32")];

/// The installation around `found`, the `git.exe` that `PATH` names: the
/// launcher `<top>\cmd\git.exe` or `<top>\bin\git.exe`, or the binary
/// `<top>\<prefix>\bin\git.exe` itself, where `<top>` holds exactly one
/// prefix directory and that prefix's `bin\git.exe`. A direct entry must name
/// that one prefix. `None` for anything else, including a
/// `<top>\etc\git-bash.config`, which makes the launcher also edit `MSYS`.
#[cfg_attr(not(windows), allow(dead_code))]
fn launcher_layout(found: &std::path::Path) -> Option<Direct> {
    let is = |name: Option<&std::ffi::OsStr>, expected: &str| name.and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case(expected));
    if !is(found.file_name(), "git.exe") {
        return None;
    }
    let dir = found.parent()?;
    let named_prefix = if is(dir.file_name(), "bin") {
        let prefix_dir = dir.parent()?;
        PREFIXES.iter().find(|(prefix, _)| is(prefix_dir.file_name(), prefix)).map(|(prefix, _)| *prefix)
    } else {
        None
    };
    let (top, entry) = match named_prefix {
        Some(_) => (dir.parent()?.parent()?, Entry::Direct),
        None if is(dir.file_name(), "cmd") || is(dir.file_name(), "bin") => (dir.parent()?, Entry::Launcher),
        None => return None,
    };
    if top.join("etc").join("git-bash.config").exists() {
        return None;
    }
    // A prefix directory, not its binary, counts as installed: gix-path
    // picks the system prefix from `EXEPATH` by directory too, so a leftover
    // directory from another build must make the layout ambiguous here, or
    // the installation hint could lead gix to the leftover's configuration.
    let mut installed = PREFIXES.iter().filter(|(prefix, _)| top.join(prefix).is_dir());
    let &(prefix, msystem) = installed.next()?;
    if installed.next().is_some() || named_prefix.is_some_and(|named| named != prefix) {
        return None;
    }
    let bin = top.join(prefix).join("bin");
    let executable = bin.join("git.exe");
    if !executable.is_file() {
        return None;
    }
    let mut path_prefix = std::ffi::OsString::new();
    for dir in [bin, top.join("usr").join("bin")] {
        path_prefix.push(dir.as_os_str());
        path_prefix.push(";");
    }
    Some(Direct { executable, entry, path_prefix, msystem })
}

#[cfg(windows)]
mod windows {
    use std::ffi::{OsStr, OsString};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::OnceLock;

    use super::{Direct, Entry, launcher_layout};

    /// Resolved once per process: the installation's location does not
    /// change while `wt` runs.
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
            if self.entry == Entry::Direct {
                return command;
            }
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
    use std::path::{Path, PathBuf};

    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"").unwrap();
    }

    fn join(top: &Path, segments: &[&str]) -> PathBuf {
        segments.iter().fold(top.to_path_buf(), |path, segment| path.join(segment))
    }

    /// What [`launcher_layout`] finds for `prefix` installed under `top`.
    fn installation(top: &Path, prefix: &str, msystem: &'static str, entry: Entry) -> Direct {
        let bin = top.join(prefix).join("bin");
        let path_prefix = format!("{};{};", bin.display(), top.join("usr").join("bin").display());
        Direct { executable: bin.join("git.exe"), entry, path_prefix: path_prefix.into(), msystem }
    }

    /// One test per prefix and entry: `<top>` holds only `<prefix>\bin\git.exe`
    /// and `PATH` names `<top>\<entry...>\git.exe`.
    macro_rules! recognized {
        ($($name:ident: $prefix:literal, $msystem:literal, [$($entry_dir:expr),+] => $entry:ident;)*) => {
            $(
                #[test]
                fn $name() {
                    let top = tempfile::tempdir().unwrap();
                    touch(&join(top.path(), &[$prefix, "bin", "git.exe"]));
                    let found = join(top.path(), &[$($entry_dir),+, "git.exe"]);
                    assert_eq!(launcher_layout(&found), Some(installation(top.path(), $prefix, $msystem, Entry::$entry)));
                }
            )*
            /// `(prefix, entry directory)` of every row above.
            const RECOGNIZED: &[(&str, &[&str])] = &[$(($prefix, &[$($entry_dir),+])),*];
        };
    }

    recognized! {
        mingw64_cmd_launcher: "mingw64", "MINGW64", ["cmd"] => Launcher;
        mingw64_bin_launcher: "mingw64", "MINGW64", ["bin"] => Launcher;
        mingw64_direct: "mingw64", "MINGW64", ["mingw64", "bin"] => Direct;
        ucrt64_cmd_launcher: "ucrt64", "UCRT64", ["cmd"] => Launcher;
        ucrt64_bin_launcher: "ucrt64", "UCRT64", ["bin"] => Launcher;
        ucrt64_direct: "ucrt64", "UCRT64", ["ucrt64", "bin"] => Direct;
        clangarm64_cmd_launcher: "clangarm64", "CLANGARM64", ["cmd"] => Launcher;
        clangarm64_bin_launcher: "clangarm64", "CLANGARM64", ["bin"] => Launcher;
        clangarm64_direct: "clangarm64", "CLANGARM64", ["clangarm64", "bin"] => Direct;
        mingw32_cmd_launcher: "mingw32", "MINGW32", ["cmd"] => Launcher;
        mingw32_bin_launcher: "mingw32", "MINGW32", ["bin"] => Launcher;
        mingw32_direct: "mingw32", "MINGW32", ["mingw32", "bin"] => Direct;
    }

    /// Guards the table above against a prefix added to [`PREFIXES`] without
    /// its rows.
    #[test]
    fn every_prefix_has_a_row_per_entry() {
        for (prefix, msystem) in PREFIXES {
            assert_eq!(msystem, prefix.to_ascii_uppercase());
            for entry_dir in [&["cmd"][..], &["bin"], &[prefix, "bin"]] {
                assert!(RECOGNIZED.contains(&(prefix, entry_dir)), "no row for {prefix} via {entry_dir:?}");
            }
        }
    }

    /// One test per layout that keeps plain `git`: the files under `<top>`,
    /// then the `git` `PATH` names, relative to `<top>`.
    macro_rules! plain_git {
        ($($name:ident: [$($file:literal),*], [$($found:literal),+];)*) => {
            $(
                #[test]
                fn $name() {
                    let top = tempfile::tempdir().unwrap();
                    $(touch(&top.path().join($file));)*
                    assert_eq!(launcher_layout(&join(top.path(), &[$($found),+])), None);
                }
            )*
        };
    }

    plain_git! {
        // Two prefixes: which one the launcher was built for is unknown.
        two_prefixes_via_the_launcher: ["mingw64/bin/git.exe", "ucrt64/bin/git.exe"], ["cmd", "git.exe"];
        two_prefixes_via_a_direct_binary: ["mingw64/bin/git.exe", "ucrt64/bin/git.exe"], ["ucrt64", "bin", "git.exe"];
        // A leftover prefix directory from an earlier build is a second prefix.
        a_leftover_prefix_via_the_launcher: ["mingw64/etc/gitconfig", "ucrt64/bin/git.exe"], ["cmd", "git.exe"];
        a_leftover_prefix_via_a_direct_binary: ["mingw64/etc/gitconfig", "ucrt64/bin/git.exe"], ["ucrt64", "bin", "git.exe"];
        a_git_bash_config_via_the_launcher: ["mingw64/bin/git.exe", "etc/git-bash.config"], ["cmd", "git.exe"];
        a_git_bash_config_via_a_direct_binary: ["ucrt64/bin/git.exe", "etc/git-bash.config"], ["ucrt64", "bin", "git.exe"];
        a_shim_directory: ["mingw64/bin/git.exe", "shims/git.exe"], ["shims", "git.exe"];
        another_file_name: ["mingw64/bin/git.exe"], ["cmd", "git"];
        the_msys_git: ["mingw64/bin/git.exe", "usr/bin/git.exe"], ["usr", "bin", "git.exe"];
        a_git_core_binary: ["mingw64/bin/git.exe", "mingw64/libexec/git-core/git.exe"], ["mingw64", "libexec", "git-core", "git.exe"];
        an_unknown_prefix: ["clang64/bin/git.exe"], ["clang64", "bin", "git.exe"];
        no_prefix_behind_the_launcher: [], ["cmd", "git.exe"];
        a_prefix_without_its_binary_via_the_launcher: ["mingw64/etc/gitconfig"], ["cmd", "git.exe"];
        a_prefix_without_its_binary_via_a_direct_binary: ["mingw64/etc/gitconfig"], ["mingw64", "bin", "git.exe"];
        a_direct_binary_of_another_prefix: ["ucrt64/bin/git.exe"], ["mingw64", "bin", "git.exe"];
    }

    /// The real installation, through each entry on `PATH`. [`windows::resolved`]
    /// is cached once per process, so each entry runs a child test in a fresh
    /// copy of this test binary with that entry first on `PATH`; a child test
    /// started any other way (a plain run of the suite) asserts nothing.
    #[cfg(windows)]
    mod entries {
        use std::fs;
        use std::path::{Path, PathBuf};
        use std::process::Command;

        use super::super::{Entry, command, name_installation_for_gix, windows};

        /// Set by [`in_child`] to the entry the child must resolve.
        const ROLE: &str = "WT_GIT_PROGRAM_TEST_ENTRY";

        fn in_child(child: &str, entry: Entry) {
            let Some(host) = windows::resolved() else {
                let found = which::which("git").expect("git on PATH");
                panic!("expected a Git for Windows installation around {}", found.display());
            };
            let top = host.executable.ancestors().nth(3).expect("<top>\\<prefix>\\bin\\git.exe");
            let entry_dir = match entry {
                Entry::Launcher => top.join("cmd"),
                Entry::Direct => host.executable.parent().expect("bin").to_path_buf(),
            };
            let mut path = entry_dir.into_os_string();
            path.push(";");
            path.push(std::env::var_os("PATH").unwrap_or_default());
            let module = module_path!().split_once("::").expect("crate::module").1;
            let output = Command::new(std::env::current_exe().expect("test binary"))
                .args([format!("{module}::{child}").as_str(), "--exact", "--nocapture", "--test-threads=1"])
                .env(ROLE, format!("{entry:?}"))
                .env("PATH", path)
                .env_remove("EXEPATH")
                .env_remove("GIT_EXEC_PATH")
                .output()
                .expect("the test binary runs");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success() && stdout.contains("1 passed"), "{child} via {entry:?}:\n{stdout}\n{stderr}");
        }

        /// The entry [`in_child`] put first on `PATH`, and the installation
        /// resolved through it; `None` outside [`in_child`].
        fn role() -> Option<&'static super::super::Direct> {
            let entry = match std::env::var(ROLE).ok()?.as_str() {
                "Launcher" => Entry::Launcher,
                "Direct" => Entry::Direct,
                other => panic!("{ROLE}={other}"),
            };
            let direct = windows::resolved().expect("an installation through the entry on PATH");
            assert_eq!(direct.entry, entry);
            Some(direct)
        }

        /// The sorted `env` that an alias of a Git started by `git` sees.
        fn alias_env(dir: &Path, mut git: Command, name: &str) -> String {
            let config = dir.join("alias.gitconfig");
            // `env` output goes to a file: sh's stdout is the same either way.
            fs::write(&config, "[alias]\n\tdump = \"!env | sort > \\\"$DUMP\\\"\"\n").unwrap();
            let out = dir.join(name);
            let status = git
                .current_dir(dir)
                .arg("-c")
                .arg(format!("include.path={}", config.display()))
                .arg("dump")
                .env("DUMP", out.to_string_lossy().replace('\\', "/"))
                .status()
                .expect("git runs");
            assert!(status.success());
            fs::read_to_string(out).unwrap().replace(name, "DUMP")
        }

        #[test]
        fn through_the_launcher_the_named_installation_is_the_prefix_git_reports() {
            in_child("named_installation", Entry::Launcher);
        }

        #[test]
        fn through_the_direct_binary_the_named_installation_is_the_prefix_git_reports() {
            in_child("named_installation", Entry::Direct);
        }

        /// What `wt`'s `main` does: gix then takes the system prefix from
        /// `EXEPATH` without starting Git, and it is the one `git
        /// --exec-path` implies, while Git children never see the variable.
        #[test]
        fn named_installation() {
            if role().is_none() {
                return;
            }
            // SAFETY: this child process runs this one test on one thread.
            unsafe { name_installation_for_gix() };
            assert!(std::env::var_os("EXEPATH").is_some());
            // gix-path's fallbacks (`git --exec-path`, `git config
            // --show-origin`) derive the same prefix, so only an empty trace
            // directory shows that gix took it from the hint.
            let traces = tempfile::tempdir().unwrap();
            // SAFETY: as above.
            unsafe { std::env::set_var("GIT_TRACE2_EVENT", traces.path()) };
            let named = gix::path::env::system_prefix().expect("a system prefix");
            let _ = gix::path::env::installation_config();
            // SAFETY: as above.
            unsafe { std::env::remove_var("GIT_TRACE2_EVENT") };
            assert_eq!(fs::read_dir(traces.path()).unwrap().count(), 0, "gix started Git to find the installation");
            let exec_path = command().arg("--exec-path").output().expect("git runs");
            let exec_path = String::from_utf8(exec_path.stdout).unwrap();
            // `<prefix>/libexec/git-core`, as gix-path derives the prefix from it.
            let reported = Path::new(exec_path.trim()).ancestors().nth(2).expect("a prefix").to_path_buf();
            assert!(
                crate::remove::admin_entry::same_location(named, &reported),
                "{} vs {}",
                named.display(),
                reported.display()
            );
            let env = command().args(["-c", "alias.e=!env", "e"]).output().expect("git runs");
            let env = String::from_utf8_lossy(&env.stdout);
            assert!(env.lines().any(|line| line.starts_with("PATH=")), "the alias ran: {env}");
            assert!(!env.lines().any(|line| line.starts_with("EXEPATH=")), "{env}");
        }

        #[test]
        fn the_launcher_entry_sees_the_launchers_environment() {
            in_child("launcher_parity", Entry::Launcher);
        }

        /// The environment Git's hooks, aliases, and transports see is the
        /// same whether `wt` starts the launcher or the binary behind it.
        #[test]
        fn launcher_parity() {
            let Some(direct) = role() else {
                return;
            };
            let top = direct.executable.ancestors().nth(3).unwrap();
            let launcher: PathBuf = top.join("cmd").join("git.exe");
            assert!(launcher.is_file(), "{}", launcher.display());
            let dir = tempfile::tempdir().unwrap();
            let via_launcher = alias_env(dir.path(), Command::new(&launcher), "launcher.txt");
            assert_eq!(alias_env(dir.path(), command(), "direct.txt"), via_launcher);
        }

        #[test]
        fn the_direct_entry_sees_what_the_found_binary_sees() {
            in_child("direct_parity", Entry::Direct);
        }

        /// A binary `PATH` names directly gets nothing from `wt` but the
        /// removal of the installation hint `wt` itself set.
        #[test]
        fn direct_parity() {
            let Some(direct) = role() else {
                return;
            };
            // SAFETY: this child process runs this one test on one thread.
            unsafe { name_installation_for_gix() };
            assert!(std::env::var_os("EXEPATH").is_some());
            let dir = tempfile::tempdir().unwrap();
            let mut unwrapped = Command::new(&direct.executable);
            unwrapped.env_remove("EXEPATH");
            let via_binary = alias_env(dir.path(), unwrapped, "binary.txt");
            assert_eq!(alias_env(dir.path(), command(), "direct.txt"), via_binary);
        }
    }
}
