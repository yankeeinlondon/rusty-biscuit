#![cfg(unix)]

//! Integration tests: inline-compose interactive collection, capability gating, and handler/readonly recovery.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use predicates::str::contains;
use std::fs;
mod common;
use common::{CliProcessFixture, write_executable};

#[cfg(unix)]
#[test]
fn inline_compose_interactive_is_capability_gated() {
    let fixture = CliProcessFixture::named("inline-compose-interactive-gated");

    let md_file = fixture.cwd().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate content\n---\nOriginal body\n",
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("gemini"),
        r#"#!/bin/sh
exit 0
"#,
    );

    fixture
        .command()
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
    let fixture = CliProcessFixture::named("inline-compose-interactive-codex");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
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
        .install(fixture.bin_dir(), "codex");

    fixture
        .command()
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
