//! CLI-boundary integration tests for the `RemovedValidationKey` diagnostic.
//!
//! Verifies that a prompt file declaring a retired validation/handler DSL key
//! (`pre_checks`, `post_checks`, `handle`, `handle_*`, or `deviate`) is rejected
//! with the typed `CompositionError::RemovedValidationKey` diagnostic across all
//! three composition modes — `compose`, `inline-compose`, and `sequence` — before
//! the provider is ever launched.
//!
//! Covers review iteration 1 of the `2026-06-21-remove-validations` feature,
//! which found the diagnostic was only exercised by scanner-level unit tests and
//! not at the user-observable CLI boundary.

use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, strip_ansi, write_executable};

// ============================================================================
// Shared assertion harness
// ============================================================================

/// Collapse runs of whitespace (including line wraps) to single spaces so
/// substring assertions survive the prose renderer's terminal-width wrapping.
/// The block-report vertical bar (`┃`) that prefixes each continuation line is
/// treated as whitespace so a phrase split across a wrap still matches.
fn normalize_ws(s: &str) -> String {
    s.replace('┃', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Drive a single removed-key rejection case end to end.
///
/// Writes `md_content` to a temp file, stages a `goose` provider stub that
/// appends to a marker file on any invocation, then runs `claudine <subcommand>
/// --goose <file>`. Asserts the process exits 1, stderr carries the
/// `removed validation/handler key` header, names `expected_key`, includes the
/// `expected_replacement_substr` guidance, and — critically — that the marker
/// file was never written (the provider was not launched).
#[cfg(unix)]
fn assert_removed_key_rejected(
    md_content: &str,
    subcommand: &str,
    expected_key: &str,
    expected_replacement_substr: &str,
) {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let marker = workspace.path().join("provider-launched.txt");

    let md = workspace.path().join("prompt.md");
    fs::write(&md, md_content).unwrap();

    // Any invocation records to the marker so we can prove the stub never ran.
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {m}\nexit 0\n",
            m = marker.display()
        ),
    );

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([subcommand, "--goose", md.to_str().unwrap()])
        .output()
        .unwrap();

    let plain_err = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let normalized = normalize_ws(&plain_err);
    assert_eq!(
        output.status.code(),
        Some(1),
        "`{subcommand}` must exit 1 for removed key `{expected_key}`; stderr:\n{plain_err}"
    );
    assert!(
        normalized.contains("removed validation/handler key"),
        "stderr must carry the diagnostic header for `{expected_key}`; stderr:\n{plain_err}"
    );
    assert!(
        normalized.contains(expected_key),
        "stderr must name the offending key `{expected_key}`; stderr:\n{plain_err}"
    );
    assert!(
        normalized.contains(expected_replacement_substr),
        "stderr must carry the lifecycle replacement guidance for `{expected_key}`; stderr:\n{plain_err}"
    );
    assert!(
        !marker.exists(),
        "provider stub must not be launched when `{expected_key}` is present; marker was written"
    );
}

// ============================================================================
// Direct compose (`claudine compose`)
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_rejects_pre_checks_key() {
    assert_removed_key_rejected(
        "---\npre_checks:\n  - command: test\n---\nBody\n",
        "compose",
        "pre_checks",
        "lifecycle stack instead",
    );
}

#[cfg(unix)]
#[test]
fn compose_rejects_post_checks_key() {
    assert_removed_key_rejected(
        "---\npost_checks:\n  - command: test\n---\nBody\n",
        "compose",
        "post_checks",
        "lifecycle stack instead",
    );
}

#[cfg(unix)]
#[test]
fn compose_rejects_handle_key() {
    assert_removed_key_rejected(
        "---\nhandle: shell fix\n---\nBody\n",
        "compose",
        "handle",
        "other lifecycle action",
    );
}

#[cfg(unix)]
#[test]
fn compose_rejects_handle_timeout_key() {
    assert_removed_key_rejected(
        "---\nhandle_timeout:\n  - action: retry\n---\nBody\n",
        "compose",
        "handle_timeout",
        "recovery actions instead",
    );
}

#[cfg(unix)]
#[test]
fn compose_rejects_handle_inline_body_unchanged_key() {
    assert_removed_key_rejected(
        "---\nhandle_inline_body_unchanged:\n  - action: retry\n---\nBody\n",
        "compose",
        "handle_inline_body_unchanged",
        "recovery actions instead",
    );
}

#[cfg(unix)]
#[test]
fn compose_rejects_deviate_key() {
    assert_removed_key_rejected(
        "---\ndeviate: shell fix\n---\nBody\n",
        "compose",
        "deviate",
        "plus a recovery action",
    );
}

// ============================================================================
// Inline compose (`claudine inline-compose`)
// ============================================================================

#[cfg(unix)]
#[test]
fn inline_compose_rejects_pre_checks_key() {
    // inline-compose requires a `prompt` frontmatter property; the removed key
    // must still be caught alongside it.
    assert_removed_key_rejected(
        "---\nprompt: Generate the body\npre_checks:\n  - command: test\n---\nBody\n",
        "inline-compose",
        "pre_checks",
        "lifecycle stack instead",
    );
}

// ============================================================================
// Sequence (`claudine sequence`)
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_rejects_deviate_key() {
    // The sequence executor prepares each step through the same scan path, so a
    // removed key in the document frontmatter is caught before step 1 launches.
    assert_removed_key_rejected(
        "---\nsequence:\n  - alpha\ndeviate: shell fix\n---\nRun step {{state}}\n",
        "sequence",
        "deviate",
        "plus a recovery action",
    );
}
