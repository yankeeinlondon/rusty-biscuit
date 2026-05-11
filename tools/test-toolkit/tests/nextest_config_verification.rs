//! End-to-end verification that the repo's `.config/nextest.toml`
//! `slow-timeout` threshold is honored by `cargo nextest`.
//!
//! The verifier spawns `cargo nextest` against the slow fixture below and
//! asserts the resulting output contains nextest's `SLOW` marker. Both tests
//! are `#[ignore]` so the standard `cargo test` / `cargo nextest run` invocation
//! does not pay the ~6s sleep, and the recursive cargo invocation only fires
//! when explicitly opted in:
//!
//! ```bash
//! cargo nextest run -p test-toolkit --test nextest_config_verification \
//!     --run-ignored only
//! ```
//!
//! In CI, run with `--run-ignored only` on this specific test binary.

use std::process::Command;
use std::thread;
use std::time::Duration;

/// Slow fixture that sleeps past the `.config/nextest.toml` `slow-timeout`
/// threshold (5 s default profile, 10 s ci profile). The verifier below spawns
/// `cargo nextest` to run only this test and parses the output.
#[test]
#[ignore = "fixture for nextest slow-test highlighting verification"]
fn slow_fixture_for_nextest_verification() {
    thread::sleep(Duration::from_secs(6));
}

/// Spawns `cargo nextest run` against `slow_fixture_for_nextest_verification`
/// and asserts the output contains nextest's `SLOW` marker.
///
/// Skips with a clear stderr message if `cargo-nextest` is not installed so
/// development environments without the optional tool stay green.
#[test]
#[ignore = "spawns `cargo nextest` recursively (~7 s); run with --run-ignored only"]
fn cargo_nextest_flags_slow_test_in_output() {
    if !cargo_nextest_available() {
        eprintln!("skipping: `cargo nextest` is not installed on this system");
        return;
    }

    let output = Command::new("cargo")
        .args([
            "nextest",
            "run",
            "-p",
            "test-toolkit",
            "--test",
            "nextest_config_verification",
            "--run-ignored",
            "only",
            "--no-fail-fast",
            "-E",
            "test(=slow_fixture_for_nextest_verification)",
        ])
        .output()
        .expect("failed to spawn `cargo nextest run`");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let saw_slow_marker = stderr.contains("SLOW") || stdout.contains("SLOW");
    assert!(
        saw_slow_marker,
        "expected nextest output to contain the 'SLOW' marker for the slow \
         fixture; this proves `.config/nextest.toml` slow-timeout is honored.\n\
         --- stderr ---\n{stderr}\n--- stdout ---\n{stdout}",
    );
}

/// Probe whether `cargo nextest` is invokable on this system without paying
/// the cost of a real run.
fn cargo_nextest_available() -> bool {
    Command::new("cargo")
        .args(["nextest", "--version"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}
