//! Tests for the three spawn modes, split to match the module tree:
//! - [`inherited`] — [`run_child`] PID capture and wall-clock timeout.
//! - [`captured`] — [`run_child_capture`] PID/env capture, timeout, and the
//!   [`capture_stream_with_volume_cap`] helper.
//!
//! [`capture_stream_with_volume_cap`]: super::captured::capture_stream_with_volume_cap

use super::super::ChildIoOptions;
use super::*;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

mod captured;
mod inherited;

#[cfg(windows)]
#[test]
fn child_env_invariant_accepts_windows_key_spelling() {
    let env = HashMap::from([
        (
            OsString::from("Path"),
            OsString::from(r"C:\Windows\System32"),
        ),
        (
            OsString::from("UserProfile"),
            OsString::from(r"C:\Users\test"),
        ),
    ]);

    super::setup::debug_assert_child_env(&env);
}

/// Minimal child environment for real process fixtures.
///
/// The host's path is part of executable discovery on both platforms; home
/// falls back to the existing temporary directory when the host exposes no
/// conventional home variable.
fn minimal_env() -> HashMap<OsString, OsString> {
    let mut env = HashMap::new();
    env.insert(
        OsString::from("PATH"),
        std::env::var_os("PATH").expect("test host must provide PATH"),
    );
    env.insert(
        OsString::from("HOME"),
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .unwrap_or_else(|| std::env::temp_dir().into_os_string()),
    );
    #[cfg(windows)]
    if let Some(system_root) = std::env::var_os("SYSTEMROOT") {
        env.insert(OsString::from("SYSTEMROOT"), system_root);
    }
    env
}

fn test_cwd() -> PathBuf {
    std::env::temp_dir()
}

#[cfg(unix)]
fn test_shell_command(unix_script: &str, _windows_script: &str) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("/bin/sh"),
        vec!["-c".to_owned(), unix_script.to_owned()],
    )
}

#[cfg(windows)]
fn test_shell_command(_unix_script: &str, windows_script: &str) -> (PathBuf, Vec<String>) {
    (
        std::env::var_os("COMSPEC")
            .map(PathBuf::from)
            .expect("Windows test host must provide COMSPEC"),
        vec!["/D".to_owned(), "/C".to_owned(), windows_script.to_owned()],
    )
}

/// Locate a `sleep`-equivalent for the wall-clock-timeout tests. macOS and
/// Linux both ship `/bin/sleep`; some Linux distros only have
/// `/usr/bin/sleep`.
#[cfg(unix)]
fn sleep_binary() -> &'static Path {
    if Path::new("/bin/sleep").exists() {
        Path::new("/bin/sleep")
    } else {
        Path::new("/usr/bin/sleep")
    }
}
