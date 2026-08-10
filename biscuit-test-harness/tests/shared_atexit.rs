#![cfg(unix)]

//! Verifies that `SharedHarness`'s `libc::atexit` hook actually fires
//! the inner harness's `Drop` impl when the process exits.
//!
//! This is the core value proposition of `SharedHarness` over a raw
//! `Mutex<Option<T>>`: Rust skips `Drop` on `static` values, so without
//! the atexit hook the harness leaks its pane / window / session at
//! process exit. The unit tests in `src/shared.rs` exercise the
//! initialization and `take()` paths but cannot observe the atexit hook
//! firing — that only happens in a real process exit.
//!
//! This integration test spawns the test binary as a subprocess. The
//! child initializes a `SharedHarness<DropFlag>` whose `Drop` impl
//! writes a sentinel byte string to a file on disk, then calls
//! `std::process::exit(0)`. The parent waits for the child to exit and
//! asserts the sentinel was written — proving the atexit hook ran.

use biscuit_test_harness::shared::SharedHarness;
use std::path::PathBuf;
use std::process::Command;

const HELPER_ENV: &str = "BISCUIT_SHARED_ATEXIT_HELPER_PATH";
const SENTINEL: &[u8] = b"atexit-dropped";

struct DropFlag {
    path: PathBuf,
}

impl Drop for DropFlag {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.path, SENTINEL);
    }
}

static SHARED: SharedHarness<DropFlag> = SharedHarness::new();

#[test]
fn shared_harness_atexit_drops_at_process_exit() {
    // Child mode: run only when the helper env var is set. We initialize
    // the shared slot (which registers the atexit hook) and then exit
    // cleanly so libc fires the hook before the process terminates.
    if let Ok(path_str) = std::env::var(HELPER_ENV) {
        let path = PathBuf::from(path_str);
        let guard = SHARED.get_or_init(|| DropFlag { path: path.clone() });
        assert!(guard.is_some(), "child: harness must be initialized");
        drop(guard);
        // libc::exit runs registered atexit handlers in reverse order.
        std::process::exit(0);
    }

    // Parent mode: spawn this very test binary with the helper env var
    // pointing at a temp file, wait for it to exit, and assert the file
    // contains the sentinel written by `DropFlag::drop`.
    let tmp = tempfile::tempdir().expect("create tempdir");
    let flag_path = tmp.path().join("drop_flag");
    assert!(!flag_path.exists(), "flag file must not exist before child run");

    let exe = std::env::current_exe().expect("locate current test executable");
    let status = Command::new(&exe)
        .env(HELPER_ENV, &flag_path)
        // Filter to just this test so any additions to this file in the
        // future do not run twice or interfere with the child's exit
        // path.
        .args(["--exact", "shared_harness_atexit_drops_at_process_exit"])
        .status()
        .expect("spawn child test binary");
    assert!(
        status.success(),
        "child exited non-zero (status: {status:?}); flag exists = {}",
        flag_path.exists(),
    );

    let contents =
        std::fs::read(&flag_path).expect("flag file should have been written by atexit hook");
    assert_eq!(
        contents.as_slice(),
        SENTINEL,
        "atexit hook did not write the expected sentinel to the flag file",
    );
}
