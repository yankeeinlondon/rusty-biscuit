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

use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn nextest_config() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("test-toolkit lives under <repo>/tools/test-toolkit")
        .join(".config/nextest.toml");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// The CI profile must not retry failures, either globally or through a tier
/// override. A test that passes only on retry still represents a failed run.
#[test]
fn ci_profile_disables_all_retries() {
    let config = nextest_config();
    let ci_start = config
        .find("[profile.ci]")
        .expect("`.config/nextest.toml` must define [profile.ci]");
    // The blanket retry setting lives between `[profile.ci]` and its first override.
    let ci_head_end = config[ci_start..]
        .find("[[profile.ci.overrides]]")
        .map(|rel| ci_start + rel)
        .unwrap_or(config.len());
    let ci_head = &config[ci_start..ci_head_end];
    assert!(
        ci_head.contains("retries = 0"),
        "[profile.ci] must set `retries = 0` so a deterministic L1 failure runs once"
    );
    for tier in ["level2_", "browser_"] {
        let marker = format!("filter = 'test(/{tier}/)'");
        let override_start = config[ci_start..]
            .find(&marker)
            .map(|relative| ci_start + relative)
            .unwrap_or_else(|| panic!("[profile.ci] must define the {tier} override"));
        let override_end = config[override_start..]
            .find("[[profile.ci.overrides]]")
            .map(|relative| override_start + relative)
            .unwrap_or(config.len());
        assert!(
            config[override_start..override_end].contains("retries = 0"),
            "the {tier} CI override must keep retries disabled"
        );
    }
    assert!(
        config.contains(r#"junit = { path = "test-results.xml" }"#),
        "the CI profile must emit JUnit for dashboards and per-shard artifacts"
    );
}

/// Slow fixture that sleeps past the `default` profile's `slow-timeout` period
/// (5 s). The verifier below pins that profile explicitly, so this sleep does
/// not have to track the `ci` profile's period, which is deliberately far
/// longer (30 s) to avoid killing correct-but-contended tests.
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

    // `-P default` explicitly: without it the child inherits an ambient
    // `NEXTEST_PROFILE`, which CI sets to `ci`. This assertion would then be
    // measuring a 6-second sleep against the ci profile's 30-second period and
    // fail for a reason that has nothing to do with the configuration being
    // verified.
    let output = Command::new("cargo")
        .args([
            "nextest",
            "run",
            "-P",
            "default",
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
