use std::fs;
use std::time::{Duration, Instant};

use serial_test::serial;

mod common;
use common::TestWorkspace;

/// Regression guard: a plain `claudine handle turn_complete` with no config
/// and a representative Gemini payload must complete in well under the 15s
/// default deadline. This locks in the 2026-04-14 fix for the 30s hook hang.
#[test]
#[serial]
fn handle_turn_complete_fast_path_completes_under_3s() {
    let workspace = TestWorkspace::named("claudine-handle-deadline-it");
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("cwd");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    let payload = serde_json::json!({
        "hook_event_name": "AfterAgent",
        "session_id": "regression-turn-complete",
        "turn_number": 3,
        "elapsed_ms": 12345
    })
    .to_string();

    let start = Instant::now();
    let assertion = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .env("CLAUDINE_HANDLE_DEADLINE_SECONDS", "5")
        .args(["handle", "turn_complete", "--provider", "gemini"])
        .write_stdin(payload)
        .assert()
        .success();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(3),
        "fast-path turn_complete should finish in <3s; took {elapsed:?}"
    );

    drop(assertion);
}

/// Verify the deadline itself fires: with a 1s deadline and stdin left open
/// (parent never sends EOF), the handler must exit inside the grace window
/// with a stderr diagnostic. Uses a shorter deadline than the 15s default
/// so the test stays fast.
#[test]
#[serial]
fn handle_exits_on_deadline() {
    use std::process::{Command, Stdio};

    let workspace = TestWorkspace::named("claudine-handle-deadline-hang");
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("cwd");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    let bin = env!("CARGO_BIN_EXE_claudine");
    let mut child = Command::new(bin)
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .env("CLAUDINE_HANDLE_DEADLINE_SECONDS", "1")
        .args(["handle", "session_end", "--provider", "claude"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn claudine handle");

    // Take stdin out of the child so `wait_with_output` doesn't auto-close it.
    // Keep the handle alive so the child blocks on read_to_string until the
    // deadline fires.
    let stdin_held = child.stdin.take().expect("child stdin");

    let start = Instant::now();
    let output = child.wait_with_output().expect("wait");
    let elapsed = start.elapsed();
    drop(stdin_held);

    assert!(
        elapsed < Duration::from_secs(4),
        "handle should exit within deadline + grace; took {elapsed:?}"
    );
    assert!(
        !output.status.success(),
        "expected non-zero exit on deadline; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("deadline exceeded"),
        "expected 'deadline exceeded' in stderr: {stderr}"
    );
}
