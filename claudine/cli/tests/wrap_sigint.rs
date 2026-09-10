#![cfg(unix)]

//! Integration tests: SIGINT-during-prep exit-code behavior.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

mod common;
use common::{CliProcessFixture, strip_ansi, write, write_executable};

// Was `slow_compose_sigint_during_prep_exits_130_with_notice`. The `slow_`
// prefix is a tier marker, and `_tier_filter L1` excludes it from `just test`,
// `just test-cli` and every CI leg — inclusion needs `l1-include-slow`, which
// only darkmatter's packages declare. So the SIGINT-during-prep contract ran in
// no recipe at all. Nothing about the test is slow: the *fake provider* sleeps
// for 10 s, which is the floor the 4 s latency assertion is measured against,
// and the test itself completes in 0.18 s because interrupting that sleep is
// the whole point.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_sigint_during_prep_exits_130_with_notice() {
    let fixture = CliProcessFixture::named("wrap-sigint");
    fixture.seed_user_config();

    // Frontmatter `model` hint is required so the catalog refresh gate
    // (`refresh_for_model_validation`) actually invokes the dynamic source
    // for OpenCode. Without it, `hints.model.is_none()` short-circuits the
    // refresh and the slow `opencode models` subprocess never runs, leaving
    // this test's interrupt window non-deterministic.
    //
    // The hint must be *valid*: frontmatter models are validated against
    // the baseline catalog (`resolve_model_with_env` step 4) and an invalid
    // hint falls through to the provider default (`None`), which fails
    // OpenCode's non-TTY model requirement with exit 1 before the interrupt
    // path is reached. `llamacpp/…` matches via `offering_sources` prefix,
    // so validity is structural and immune to baseline offering churn.
    let md_file = fixture.cwd().join("slow.md");
    write(
        &md_file,
        "---\ntitle: test\nmodel: llamacpp/test-model\n---\nPrompt body\n",
    );

    // Fake `opencode models` touches a readiness marker, then sleeps for
    // 10s so prep is slow enough to interrupt, and so a regression to the
    // uncancellable blocking path would clearly exceed the 4s
    // interrupt-to-exit budget below (it would have to wait ~9s for the
    // sleep to finish). The marker is the test's synchronization barrier:
    // it is written only once the model-validation refresh has reached the
    // `opencode models` subprocess, which happens *after* the SIGINT handler
    // is installed at the top of `compose`. The `opencode` provider binary
    // itself never runs.
    let ready_marker = fixture.cwd().join("opencode-models-started");
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  : > "$CLAUDINE_READY_MARKER"
  /bin/sleep 10
  printf '%s\n' '["test-model"]'
  exit 0
fi
exit 0
"#,
    );

    // `CLAUDINE_BACKGROUND_REFRESH=0` forces the caller-blocking refresh
    // path (the documented escape hatch). The default W3 refresh is
    // detached, so prep would race ahead of the `opencode models`
    // subprocess and this test's interrupt window would not exist; the
    // blocking path is the one whose SIGINT cancellability is under test.
    //
    // The child has to stay alive to receive the signal, so this is the
    // builder's raw-command surface; the call site keeps only its subject —
    // signal delivery, output draining, and reaping.
    let child = fixture
        .command_std()
        .env("CLAUDINE_READY_MARKER", &ready_marker)
        .env("CLAUDINE_BACKGROUND_REFRESH", "0")
        .args([
            "compose",
            "--opencode",
            md_file.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let pid = child.id() as i32;

    // Poll for the readiness marker rather than sleeping a fixed interval:
    // the marker proves the child reached the cancellable `opencode models`
    // refresh, which is strictly after the SIGINT handler is installed. A
    // fixed sleep was flaky under full-suite contention — slow wrapper
    // startup could push handler installation past the deadline, so SIGINT
    // hit the default disposition (exit 130 but no clean notice).
    let marker_deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !ready_marker.exists() {
        assert!(
            std::time::Instant::now() < marker_deadline,
            "child never reached the opencode models refresh within 30s"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let interrupt_sent_at = std::time::Instant::now();
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }

    let output = child.wait_with_output().unwrap();
    let interrupt_to_exit = interrupt_sent_at.elapsed();

    // Exit code 130 = 128 + SIGINT(2)
    assert_eq!(
        output.status.code(),
        Some(130),
        "SIGINT during prep must yield exit code 130"
    );

    // Bounded interrupt latency: the cancellable fetch (`kill_on_drop`
    // plus the interrupt-flag race in `fetch_shell_command_models`)
    // returns within ~50 ms of the interrupt under normal conditions. The
    // fake `opencode models` sleeps for 10s — anywhere near that means
    // the fetch stopped responding to the interrupt flag. A 4-second
    // ceiling sits comfortably below the 10s blocking floor while leaving
    // headroom for OS scheduling under contention.
    assert!(
        interrupt_to_exit < std::time::Duration::from_secs(4),
        "SIGINT-to-exit latency exceeded 4s ({:?}); blocked-prep regression",
        interrupt_to_exit,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("User interrupted compose operation"),
        "stderr must contain the clean interrupt notice; got: {plain}"
    );
}
