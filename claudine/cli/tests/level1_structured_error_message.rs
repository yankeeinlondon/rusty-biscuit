//! Level 1 process-integration test for structured provider error propagation.
//!
//! Proves the **full seam** that the "real-error-messages" feature restored: a
//! real provider subprocess that emits a *structured* stream error event
//! carrying its own `error_message` (e.g. `"upstream timeout"`) must have that
//! exact provider-authored text reach BOTH the lifecycle `failure` event's
//! `err.msg` AND the failed-`finalize` event's `err.msg` — winning over the
//! generic `agent exited with error code N` exit-code fallback.
//!
//! ## The seam this crosses (and why nothing else covers it)
//!
//! The provider's `error_message` travels a long path: the opencode stream
//! parser writes it to `StreamExecutionSummary::error_message`; the wrapper
//! copies `summary.error_message` into `AttemptOutcome::error_message`
//! (`harness_orch/attempt.rs`); the loop then calls
//! `claudine::harness::failure_message`, which prefers `error_message` as its
//! top-priority cascade branch, and builds the `LifecycleErrorInfo` (`err.msg`)
//! from that same string (`harness_orch/loop_control.rs`). Isolated parser
//! unit tests and isolated `failure_message` builder unit tests both exist, but
//! **no test crossed the whole seam through a real binary invocation**. A
//! regression that dropped `summary.error_message` in `execute_harness_attempt`
//! would leave every isolated test green while restoring the original
//! user-facing bug (users saw `agent exited with error code 1` instead of the
//! provider's real reason). This test fails loudly if that propagation breaks,
//! because it asserts the *content* of the message, not merely that a failure
//! occurred.
//!
//! ## Why Level 1 (subprocess), not Level 2 (tmux/terminal capture)
//!
//! The observable is **lifecycle data written to a file** (`events.log`), not
//! terminal glyphs, SGR sequences, or scrollback. There is nothing to capture
//! from a real terminal here — the proof is a deterministic on-disk record of
//! what the lifecycle stacks observed. So this uses the lighter
//! `assert_cmd::Command::cargo_bin("claudine").unwrap()` subprocess harness (from `loop_cli.rs`) rather
//! than the heavier `level2_*` tmux harness (which exists to assert real-
//! terminal rendering). It runs under the standard integration-test recipe;
//! it needs no tmux/WezTerm backend.
//!
//! ## Side-effect path resolution
//!
//! The `append_line` effect resolves its first argument against the effect
//! engine's mutation root (`repo_root`, else the launch CWD). The workspace is
//! initialized as a git repo and the command runs from the workspace root, so
//! the mutation root is the workspace root and a plain relative `'events.log'`
//! lands at `<workspace>/events.log`, which the test reads back.

#![cfg(unix)]

use std::fs;
use std::time::Duration;
use tempfile::tempdir;

mod common;
use common::wrap::seed_minimal_config;
use common::{augmented_path, init_git_repo, write_executable};

/// The provider-authored error text. Deliberately a phrase a provider stream
/// would emit — and one that shares no substring with the generic exit-code
/// fallback (`agent exited with error code N`), so an assertion on this string
/// can only pass when the structured `error_message` actually propagated.
const PROVIDER_ERROR: &str = "upstream timeout";

/// A real provider subprocess emitting a structured stream error must land its
/// own `error_message` in both `failure.err.msg` and `finalize.err.msg`.
///
/// The fake `opencode`:
/// - answers the `models` probe (so provider preflight passes), then
/// - emits `{"type":"error","error_type":"api_timeout","error_message":"upstream timeout"}`
///   and exits non-zero.
///
/// The compose document declares a `failure` stack (unconditionally appending
/// `failure-err-msg={{ err.msg }}`) and a `when: "err"`-guarded `finalize` stack
/// (appending `finalize-err-msg={{ err.msg }}`). Both markers must carry the
/// provider text — proving the structured `error_message` reached each event —
/// and neither may fall back to the generic exit-code message.
#[test]
fn structured_provider_error_message_reaches_failure_and_finalize_err_msg() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    // Git repo so the effect engine's mutation root resolves to the workspace
    // root; `append_line "events.log"` then lands at `<workspace>/events.log`.
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");

    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        r#"---
title: structured provider error propagation
failure:
  stack:
    - action: {append_line: ["events.log", "{{ 'failure-err-msg=' + err.msg }}"]}
finalize:
  stack:
    - when: "err"
      action: {append_line: ["events.log", "{{ 'finalize-err-msg=' + err.msg }}"]}
    - action: {append_line: ["events.log", "finalize"]}
---
Structured error seam probe.
"#,
    )
    .unwrap();

    // Fake opencode: pass the `models` preflight, drain the composed prompt off
    // stdin, then emit a structured error carrying `error_message` and exit 1.
    write_executable(
        &path_dir.join("opencode"),
        &format!(
            r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
cat > /dev/null
printf '%s\n' '{{"type":"init","session_id":"err-seam","model":"test-model"}}'
printf '%s\n' '{{"type":"step_start","sessionID":"err-seam"}}'
printf '%s\n' '{{"type":"error","error_type":"api_timeout","error_message":"{msg}"}}'
exit 1
"#,
            msg = PROVIDER_ERROR,
        ),
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .current_dir(workspace.path())
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(30))
        .assert()
        .failure();

    let log = fs::read_to_string(&events_log).unwrap_or_default();
    let lines: Vec<&str> = log.lines().map(str::trim).filter(|l| !l.is_empty()).collect();

    // 1. The `failure` stack observed the provider's structured error text.
    let failure_line = lines
        .iter()
        .find(|l| l.starts_with("failure-err-msg="))
        .unwrap_or_else(|| {
            panic!("no `failure-err-msg=` line in events.log; the failure stack never observed err.msg. events.log was {lines:?}")
        });
    assert!(
        failure_line.contains(PROVIDER_ERROR),
        "failure.err.msg must carry the provider's structured error_message \
         ({PROVIDER_ERROR:?}); got {failure_line:?}. events.log was {lines:?}"
    );

    // 2. The failed-`finalize` stack observed the same provider text.
    let finalize_line = lines
        .iter()
        .find(|l| l.starts_with("finalize-err-msg="))
        .unwrap_or_else(|| {
            panic!("no `finalize-err-msg=` line in events.log; the failed-finalize payload never observed err.msg (the `when: \"err\"` guard should be truthy on a failure path). events.log was {lines:?}")
        });
    assert!(
        finalize_line.contains(PROVIDER_ERROR),
        "finalize.err.msg must carry the provider's structured error_message \
         ({PROVIDER_ERROR:?}); got {finalize_line:?}. events.log was {lines:?}"
    );

    // 3. The provider text won over the generic exit-code fallback: a
    //    regression that dropped `summary.error_message` would leave the
    //    fallback `agent exited with error code 1` here instead.
    assert!(
        !failure_line.contains("agent exited with error code"),
        "failure.err.msg must be the provider message, NOT the generic exit-code \
         fallback (structured error_message must win); got {failure_line:?}"
    );
    assert!(
        !finalize_line.contains("agent exited with error code"),
        "finalize.err.msg must be the provider message, NOT the generic exit-code \
         fallback; got {finalize_line:?}"
    );
}
