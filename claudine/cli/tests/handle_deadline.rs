use std::time::{Duration, Instant};

use serial_test::serial;

mod common;
use common::CliProcessFixture;

/// Regression guard: a plain `claudine handle turn_complete` with no config
/// and a representative Gemini payload must complete in well under the 15s
/// default deadline. This locks in the 2026-04-14 fix for the 30s hook hang.
#[test]
#[serial]
fn handle_turn_complete_fast_path_completes_under_3s() {
    // The no-config fast path excludes host config (USERPROFILE on Windows)
    // and best-effort rendezvous reporting, neither of which is timed here.
    let fixture = CliProcessFixture::named("claudine-handle-deadline-it");

    let payload = serde_json::json!({
        "hook_event_name": "AfterAgent",
        "session_id": "regression-turn-complete",
        "turn_number": 3,
        "elapsed_ms": 12345
    })
    .to_string();

    let mut command = fixture.command();
    command
        .env("CLAUDINE_HANDLE_DEADLINE_SECONDS", "5")
        .args(["handle", "turn_complete", "--provider", "gemini", "--json"])
        .write_stdin(payload)
        .timeout(Duration::from_secs(5));
    let start = Instant::now();
    let assertion = command.assert().success();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(3),
        "fast-path turn_complete should finish in <3s; took {elapsed:?}"
    );

    let output: serde_json::Value = serde_json::from_slice(&assertion.get_output().stdout).unwrap();
    assert_eq!(output["provider"], "gemini");
    assert_eq!(output["event"], "turn_complete");
    assert!(output["response"].is_null());
}

/// Verify the deadline itself fires: with a 1s deadline and stdin left open
/// (parent never sends EOF), the handler must exit inside the grace window
/// with a stderr diagnostic. Uses a shorter deadline than the 15s default
/// so the test stays fast.
#[test]
#[serial]
fn handle_exits_on_deadline() {
    use std::process::Stdio;

    let fixture = CliProcessFixture::named("claudine-handle-deadline-hang");

    // The child has to stay alive across the deadline, so this is the builder's
    // raw-command surface rather than `command()`; the policy is the same one.
    let mut child = fixture
        .command_std()
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
