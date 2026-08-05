//! Integration test: `clip watch --foreground` must refuse to run when
//! a clipper service is already alive.
//!
//! We drive the check by writing a `clipper.pid` file in a temp runtime
//! dir, with our own pid (which is guaranteed alive), then invoke the
//! `clip` binary via `cargo run`. The CLI is expected to exit non-zero
//! with a message that mentions the pid.

use std::process::Command;

use biscuit_test_harness::bin_exe;

#[test]
fn watch_foreground_refuses_when_service_pid_alive() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let runtime = tmp.path();
    // Spec: pid file lives at <runtime>/clipper.pid. Our own pid is alive.
    let our_pid = std::process::id();
    std::fs::write(runtime.join("clipper.pid"), format!("{our_pid}")).unwrap();

    let exe = bin_exe!("clip");

    let output = Command::new(exe)
        .arg("watch")
        .arg("--foreground")
        .env("CLIP_RUNTIME_DIR", runtime)
        .output()
        .expect("failed to invoke clip");

    assert!(
        !output.status.success(),
        "clip watch --foreground should fail when service is up; \
         stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("clipper service is running") || combined.contains("refusing to start"),
        "expected 'refusing' message; got: {combined}"
    );
}
