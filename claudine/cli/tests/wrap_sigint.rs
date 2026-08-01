#![cfg(unix)]

//! Integration tests: SIGINT-during-prep exit-code behavior.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{augmented_path, strip_ansi, write_executable};

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn slow_compose_sigint_during_prep_exits_130_with_notice() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // Frontmatter `model` hint is required so the catalog refresh gate
    // (`refresh_for_model_validation`) actually invokes the dynamic source
    // for OpenCode. Without it, `hints.model.is_none()` short-circuits the
    // refresh and the slow `opencode models` subprocess never runs, leaving
    // this test's interrupt window non-deterministic.
    let md_file = workspace.path().join("slow.md");
    fs::write(
        &md_file,
        "---\ntitle: test\nmodel: test-model\n---\nPrompt body\n",
    )
    .unwrap();

    // Fake `opencode models` touches a readiness marker, then sleeps for
    // 10s so prep is slow enough to interrupt, and so a regression to the
    // uncancellable blocking path would clearly exceed the 4s
    // interrupt-to-exit budget below (it would have to wait ~9s for the
    // sleep to finish). The marker is the test's synchronization barrier:
    // it is written only once the model-validation refresh has reached the
    // `opencode models` subprocess, which happens *after* the SIGINT handler
    // is installed at the top of `compose`. The `opencode` provider binary
    // itself never runs.
    let ready_marker = workspace.path().join("opencode-models-started");
    write_executable(
        &path_dir.join("opencode"),
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

    let bin = env!("CARGO_BIN_EXE_claudine");
    let child = std::process::Command::new(bin)
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_READY_MARKER", &ready_marker)
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

    // Bounded interrupt latency: the cancellable refresh path returns
    // within ~50 ms of the interrupt poll under normal conditions. The
    // fake `opencode models` sleeps for 10s — anywhere near that means
    // we've regressed to the uncancellable `refresh_provider_blocking`
    // path. A 4-second ceiling sits comfortably below the 10s blocking
    // floor while leaving headroom for OS scheduling under contention.
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
