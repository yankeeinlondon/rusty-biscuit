//! Support for tests that launch "the user's editor": the
//! `darkmatter-fake-editor` fixture installed under an editor's name, scoped
//! process environment, and process-liveness checks.
//!
//! Tests that use [`ScopedEnv`] change the test process's own environment
//! (`EDITOR`, `VISUAL`, the fake editor's `FAKE_EDITOR_*` variables, which the
//! launched editor inherits), so they are `#[serial]`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use darkmatter::editor::ResolvedEditor;
use sniff::programs::EditorSurface;

/// Sets and removes process environment variables, restoring them on drop.
pub struct ScopedEnv {
    saved: Vec<(String, Option<OsString>)>,
}

impl ScopedEnv {
    pub fn new() -> Self {
        Self { saved: Vec::new() }
    }

    pub fn set(&mut self, key: &str, value: impl AsRef<std::ffi::OsStr>) -> &mut Self {
        self.saved.push((key.to_string(), std::env::var_os(key)));
        // SAFETY: every test using this is `#[serial]`, and nextest runs each
        // test in its own process, so nothing reads the environment
        // concurrently.
        unsafe { std::env::set_var(key, value) };
        self
    }

    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.saved.push((key.to_string(), std::env::var_os(key)));
        // SAFETY: as in `set`.
        unsafe { std::env::remove_var(key) };
        self
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        for (key, value) in self.saved.iter().rev() {
            // SAFETY: as in `set`.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

/// A directory for installed editors. Its name has no space: `$EDITOR` is
/// split on whitespace, so an editor program path cannot contain one.
///
/// This is the system temp directory, not `CARGO_TARGET_TMPDIR`: that path is
/// fixed at compile time and does not exist where an archived test binary
/// runs (CI and `just cross-check`).
pub fn editor_dir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("fake-editors-")
        .tempdir()
        .expect("create the editor directory")
}

/// Installs the fake editor in `dir` as `name` (plus the platform's
/// executable suffix), so Sniff recognizes it as that editor.
pub fn install_fake_editor(dir: &Path, name: &str) -> PathBuf {
    let source = biscuit_test_harness::bin_exe!("darkmatter-fake-editor");
    let target = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    // Hard-link when the temp directory shares the fixture's filesystem, else
    // copy. Never copy over an existing install: it may be a hard link to the
    // fixture, and writing through it would truncate the fixture itself.
    if !target.exists() && std::fs::hard_link(&source, &target).is_err() {
        std::fs::copy(&source, &target).expect("install the fake editor");
    }
    target
}

/// A resolved editor that is the fake editor at `program`.
pub fn resolved(program: PathBuf, surface: Option<EditorSurface>, args: &[&str]) -> ResolvedEditor {
    ResolvedEditor {
        display: program.display().to_string(),
        program,
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        surface,
    }
}

/// Waits for the fake editor's ready marker (or any file) to appear.
pub fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !path.exists() {
        assert!(Instant::now() < deadline, "{} never appeared", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Waits until the fake editor has written `count` PIDs to `path`.
pub fn wait_for_pids(path: &Path, count: usize) -> Vec<u32> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(pids) = test_toolkit::fake_editor::read_pids(path)
            && pids.len() >= count
        {
            return pids;
        }
        assert!(Instant::now() < deadline, "{} never listed {count} PID(s)", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Polls until `pid` is gone; a killed process may take a moment to be
/// reaped by init after its parent died.
pub fn wait_until_dead(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while process_alive(pid) {
        assert!(Instant::now() < deadline, "process {pid} survived");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(unix)]
pub fn process_alive(pid: u32) -> bool {
    // SAFETY: signal 0 only checks that the PID exists.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[cfg(unix)]
pub fn kill_pid(pid: u32) {
    // SAFETY: plain `kill(2)` on a PID this test started.
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGKILL);
    }
}

#[cfg(windows)]
pub fn process_alive(pid: u32) -> bool {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .expect("run tasklist");
    String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
}

