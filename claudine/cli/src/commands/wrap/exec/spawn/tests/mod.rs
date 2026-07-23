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
use std::path::Path;

mod captured;
mod inherited;

/// Minimal env that satisfies the `PATH` / `HOME` debug-asserts inside
/// every spawn function. Test-owned so we never depend on the host
/// shell's environment.
fn minimal_env() -> HashMap<OsString, OsString> {
    let mut env = HashMap::new();
    env.insert(OsString::from("PATH"), OsString::from("/usr/bin:/bin"));
    env.insert(OsString::from("HOME"), OsString::from("/tmp"));
    env
}

/// Find a working `/bin/true`-equivalent on the test host. macOS ships
/// `/usr/bin/true`; Linux distros typically have both `/bin/true` and
/// `/usr/bin/true`. We prefer `/usr/bin/true` (always present on macOS)
/// and fall back to `/bin/true`.
fn true_binary() -> &'static Path {
    if Path::new("/usr/bin/true").exists() {
        Path::new("/usr/bin/true")
    } else {
        Path::new("/bin/true")
    }
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
