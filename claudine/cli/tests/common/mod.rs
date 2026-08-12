#![allow(dead_code)]

pub(crate) mod completion;
#[cfg(unix)]
pub(crate) mod pty;
pub(crate) mod wrap;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

/// Initialize the workspace tracing subscriber for this test binary so
/// `trace_phase!` spans become visible without setting `RUST_LOG` manually.
///
/// Idempotent (`test_toolkit::init_test_tracing` uses an internal `Once`),
/// so all helpers below can call it cheaply on every invocation.
fn ensure_test_tracing_initialized() {
    test_toolkit::init_test_tracing();
}

/// Public re-export so individual tests that bypass the helpers below can
/// still opt-in explicitly.
#[allow(unused_imports)]
pub use test_toolkit::init_test_tracing;

/// Absolute path to the `claudine` binary built for this test binary.
///
/// Resolved at run time rather than with `env!`, so a suite executed from a
/// relocated `cargo nextest archive` (the `wsl2-ubuntu` leg) still finds the
/// binary — see [`biscuit_test_harness::bin_exe`]. Returned as `&str` because
/// most call sites interpolate it into a shell command line.
pub fn claudine_bin() -> &'static str {
    static BIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    BIN.get_or_init(|| {
        biscuit_test_harness::bin_exe!("claudine")
            .to_string_lossy()
            .into_owned()
    })
}

pub struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        Self::named("claudine-test")
    }

    pub fn named(prefix: &str) -> Self {
        ensure_test_tracing_initialized();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("{prefix}-{}-{nonce}-{unique}", process::id()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Hermetic defaults for tests that execute the shipped `claudine` binary.
///
/// Repository topology, prompts, provider memory, and user configuration are
/// absent until a test writes them into the fixture explicitly.
pub struct CliProcessFixture {
    workspace: TestWorkspace,
    cwd: PathBuf,
    home: PathBuf,
    bin_dir: PathBuf,
}

impl CliProcessFixture {
    pub fn named(prefix: &str) -> Self {
        let workspace = TestWorkspace::named(prefix);
        let cwd = workspace.path().join("cwd");
        let home = workspace.path().join("home");
        let bin_dir = workspace.path().join("bin");
        for path in [&cwd, &home, &bin_dir] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            workspace,
            cwd,
            home,
            bin_dir,
        }
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    pub fn initialize_repository(&self) {
        assert!(
            init_git_repo(&self.cwd),
            "failed to initialize fixture repository at {}",
            self.cwd.display()
        );
    }

    pub fn seed_user_config(&self) {
        wrap::seed_minimal_config(&self.home);
    }

    pub fn write_root_system_prompt(&self, content: &str) -> PathBuf {
        let path = self.cwd.join("system-prompt.md");
        write(&path, content);
        path
    }

    pub fn write_user_system_prompt(&self, content: &str) -> PathBuf {
        let path = self.home.join(".claudine/system-prompt.md");
        write(&path, content);
        path
    }

    pub fn write_repo_appendix(&self, content: &str) -> PathBuf {
        let path = self.cwd.join(".claudine/non-interactive.md");
        write(&path, content);
        path
    }

    pub fn write_provider_memory(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.cwd.join(relative);
        write(&path, content);
        path
    }

    pub fn command(&self) -> assert_cmd::Command {
        let path = std::env::join_paths([self.bin_dir.as_path()])
            .expect("fake-only PATH should contain one valid path");
        let mut command = assert_cmd::Command::cargo_bin("claudine").unwrap();
        command
            .current_dir(&self.cwd)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env_remove("HOMEDRIVE")
            .env_remove("HOMEPATH")
            .env_remove("XDG_CONFIG_HOME")
            .env("APPDATA", &self.home)
            .env("LOCALAPPDATA", &self.home)
            .env("PATH", path)
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("NO_COLOR", "1");
        command
    }
}

pub fn write(path: &Path, content: &str) {
    ensure_test_tracing_initialized();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

pub fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    write(path, &serde_json::to_string_pretty(value).unwrap());
}

pub fn init_git_repo(path: &Path) -> bool {
    ensure_test_tracing_initialized();
    Command::new("git")
        .arg("init")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn augmented_path(fake_bin: &Path) -> std::ffi::OsString {
    ensure_test_tracing_initialized();
    let system_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = vec![fake_bin.to_path_buf()];
    paths.extend(std::env::split_paths(&system_path));
    std::env::join_paths(paths).expect("join_paths")
}

/// Clear an inherited `NO_COLOR` from a pane's interactive shell before a
/// color fixture runs in it.
///
/// `send_command_with_env` cannot express this: it renders env as `VAR='' cmd`,
/// and claudine treats `NO_COLOR` as *present* rather than as truthy
/// (`cli/src/log.rs` — `colors_disabled()` short-circuits `force_color_enabled()`),
/// so an empty value still disables color and `FORCE_COLOR=1` cannot out-vote
/// it. A host that exports `NO_COLOR=1` — as review environments legitimately
/// do — would otherwise turn every L2 color assertion into a false failure, or
/// worse, a vacuous pass.
///
/// The unset lands in the shell the caller then drives, so it survives any
/// `clear`/`cd` sent afterwards.
pub fn clear_no_color<H: biscuit_test_harness::TerminalHarness>(harness: &mut H) {
    harness
        .send_text(b"unset NO_COLOR\n")
        .expect("unset NO_COLOR");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
}

/// Assert that the captured pane row carrying `needle` is itself styled.
///
/// The weaker `frame.raw.contains('\u{1b}')` this replaces is satisfied by *any*
/// escape anywhere in the pane — a colored shell prompt, or tmux's own
/// re-emission — so it can hold on a run that rendered the diagnostic with no
/// styling at all. Anchoring on the row that carries the diagnostic text makes
/// the assertion fail when the thing under test loses its color.
pub fn assert_row_is_styled(raw: &str, needle: &str, what: &str) {
    let row = raw
        .lines()
        .find(|line| strip_ansi(line).contains(needle))
        .unwrap_or_else(|| panic!("no captured row contains {needle:?}.\nraw:\n{raw}"));
    assert!(
        row.contains('\u{1b}'),
        "{what}: the row carrying {needle:?} reached the pane unstyled.\nrow: {row:?}"
    );
}

pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next();
                    for code in chars.by_ref() {
                        if ('@'..='~').contains(&code) {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    chars.next();
                    for code in chars.by_ref() {
                        if code == '\u{7}' {
                            break;
                        }
                        if code == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        out.push(ch);
    }

    out
}

/// Probe for a usable pseudo-terminal on the host.
///
/// On Unix, attempts to open `/dev/ptmx` (the master multiplexer). Returns
/// `true` when allocation succeeds, which is the precondition every
/// `expectrl::Session::spawn` call in this crate's L2 PTY tests requires.
/// On non-Unix targets returns `false` (the PTY test files are themselves
/// `#[cfg(unix)]`, so this branch is unreachable from those tests).
#[allow(dead_code)]
pub fn pty_available() -> bool {
    #[cfg(unix)]
    {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ptmx")
            .is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn write_executable(path: &Path, content: &str) {
    ensure_test_tracing_initialized();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        write(path, content);
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
    #[cfg(not(unix))]
    {
        write(path, content);
    }
}

pub fn write_dry_run_provider_stub(bin_dir: &Path, binary: &str) {
    ensure_test_tracing_initialized();
    #[cfg(unix)]
    {
        write_executable(
            &bin_dir.join(binary),
            r#"#!/bin/sh
echo "SHOULD NOT RUN"
exit 1
"#,
        );
    }
    #[cfg(windows)]
    {
        write(
            &bin_dir.join(format!("{binary}.cmd")),
            "@echo off\r\necho SHOULD NOT RUN\r\nexit /b 1\r\n",
        );
    }
}
