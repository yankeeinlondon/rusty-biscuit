#![cfg(unix)]

//! Integration tests: inline-compose interactive collection, capability gating, and handler/readonly recovery.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use predicates::str::contains;
use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{write_executable};

#[cfg(unix)]
#[test]
fn inline_compose_interactive_is_capability_gated() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate content\n---\nOriginal body\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "inline-compose",
            "--interactive",
            "--gemini",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(contains(
            "inline-compose in interactive mode (from --interactive) is not supported",
        ));
}

#[cfg(unix)]
#[test]
fn inline_compose_interactive_codex_uses_captured_last_message() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate content\n---\nOriginal body\n",
    )
    .unwrap();

    // An interactive Codex session still edits the document itself; the
    // last-message file carries only the summary.
    common::InlineAgentStub::new(&md_file)
        .prelude(
            "CLAUDINE_LAST=''\n\
             prev=''\n\
             for arg in \"$@\"; do\n\
             if [ \"$prev\" = '--output-last-message' ]; then CLAUDINE_LAST=\"$arg\"; fi\n\
             prev=\"$arg\"\n\
             done\n\
             if [ -z \"$CLAUDINE_LAST\" ]; then exit 1; fi\n\
             printf 'Wrote the interactive body.\\n' > \"$CLAUDINE_LAST\"\n",
        )
        .body("Interactive body from codex\n")
        .install(&path_dir, "codex");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args([
            "inline-compose",
            "--interactive",
            "--codex",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("Interactive body from codex"),
        "interactive codex body should be applied; file: {final_content}"
    );
}

// ---------------------------------------------------------------------------
// Handler-engagement banner emission semantics
// ---------------------------------------------------------------------------
