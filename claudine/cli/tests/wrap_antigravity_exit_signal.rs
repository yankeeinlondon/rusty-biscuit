#![cfg(unix)]

//! Integration test: the wrapper-path exit-signal seam.
//!
//! `claudine antigravity <prompt>` routes through the non-wire branch
//! (`wrapper_exec.rs` → `exec::run_child_stream_semantic`). That path keeps a
//! bounded `stdout_tail_ring` of the last `EXIT_STDOUT_TAIL_LINES` (= 10) raw
//! stdout lines, then AFTER the child exits synthesizes the ratified exit
//! payload (`exit_source_payload`) and feeds it to the run's `SignalHub` (built
//! with the Antigravity detection table). The drained signals become
//! `stream_result.signals`, which `policy::emit_stream_summary` writes to the
//! SessionEnd summary row as `extra["signals"]`.
//!
//! A dedicated library test already proves a *manually-constructed* exit payload
//! fires the Antigravity `auth_invalid` record. This test closes the remaining
//! gap: it spawns a fake `agy` child through the *production* orchestration and
//! verifies the stdout ring → post-wait exit-payload synthesis → signal hub →
//! summary/reporting path preserves the `auth_invalid` signal end-to-end. The
//! surviving signal carries `source == "exit"`, pinning it to the seam under
//! test rather than to stream-side detection.

use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::write_executable;

#[cfg(unix)]
#[test]
fn antigravity_exit_auth_signal_survives_bounded_tail_into_summary_row() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    // The fake `agy` writes 13 filler stdout lines followed by the auth error as
    // the FINAL stdout line — more than the bounded 10-line `stdout_tail_ring`.
    // This proves the truncation still preserves the signal: the leading lines
    // (the envelope + the first filler lines) fall out of the retained tail, but
    // the auth line stays within the last 10 lines and so rides the synthesized
    // exit payload the seam matches against.
    //
    // The leading `{"status":"SUCCESS",…}` envelope is what keeps the buffered-
    // JSON parser's semantic path clean: it parses on the first line and marks
    // the parser `emitted`, so the trailing plain-text lines are never re-parsed
    // into a terminal error event. The exit-signal seam is independent of that
    // parse — it matches the RAW `stdout_tail` substring — so the auth line still
    // fires the `exit-auth_invalid-models-signin` record from `SignalSource::Exit`.
    write_executable(
        &path_dir.join("agy"),
        r#"#!/bin/sh
printf '%s\n' '{"status":"SUCCESS","conversation_id":"fake-antigravity-conv","response":"OK","num_turns":1}'
i=1
while [ "$i" -le 13 ]; do
  printf '%s\n' "boot line $i"
  i=$((i + 1))
done
printf '%s\n' 'Error: Please sign in to view available models. Launch the CLI without arguments to sign in.'
exit 1
"#,
    );

    // Antigravity's child exits nonzero, so the wrapper propagates a failure.
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["antigravity", "list models"])
        .assert()
        .failure();

    let log_path = today_log_path(&fake_home);
    let log_contents = fs::read_to_string(&log_path)
        .unwrap_or_else(|e| panic!("expected a JSONL log at {log_path:?}: {e}"));

    let summary_line = log_contents
        .lines()
        .find(|line| line.contains("\"synthetic_kind\":\"stream_wrapper_summary\""))
        .unwrap_or_else(|| panic!("no SessionEnd summary row in log:\n{log_contents}"));

    let summary: serde_json::Value = serde_json::from_str(summary_line).unwrap();

    // `extra` is a nested object on the EventMeta row (not flattened); the
    // drained run signals land under `extra["signals"]` with `kind`/`source`
    // hoisted beside the full tagged `event` payload (see `stream::reporting`).
    let signals = summary
        .get("extra")
        .and_then(|e| e.get("signals"))
        .and_then(|s| s.as_array())
        .unwrap_or_else(|| panic!("summary row missing extra.signals array; got: {summary_line}"));

    let auth_from_exit = signals.iter().any(|entry| {
        entry.get("kind").and_then(|k| k.as_str()) == Some("auth_invalid")
            && entry.get("source").and_then(|s| s.as_str()) == Some("exit")
    });

    assert!(
        auth_from_exit,
        "expected an `auth_invalid` signal with source `exit` (the exit-payload seam) in \
         extra.signals; got: {summary_line}"
    );
}
