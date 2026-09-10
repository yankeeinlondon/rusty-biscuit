#![cfg(unix)]

//! Integration tests for `claudine inline-compose`.
//!
//! Phase 1 of the `2026-06-03-always-harness` feature. Pins current observable
//! behavior so divergence introduced by later phases fails loudly.

use std::fs;
mod common;
use common::{CliProcessFixture, init_git_repo, write, write_executable};

// ============================================================================
// Phase 1: convergence between non-harness and harness-enabled inline compose
// ============================================================================

/// Build a fake `opencode` binary that narrates, calls a tool, edits the
/// document, and then reports a final summary. The document is the deliverable;
/// the narration must reach neither the file nor the summary.
fn stage_opencode_inline_body_writer(path_dir: &std::path::Path, document: &std::path::Path) {
    write_executable(
        &path_dir.join("opencode"),
        &format!(
            r##"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{{"type":"init","session_id":"conv","model":"test-model"}}'
printf '%s\n' '{{"type":"step_start","sessionID":"conv"}}'
printf '%s\n' '{{"type":"text","text":"Let me look up the answer."}}'
printf '%s\n' '{{"type":"tool_start","part":{{"id":"t1","tool":"bash"}}}}'
CLAUDINE_DOC={document}
CLAUDINE_ADD=''
CLAUDINE_BODY='# Final Body

This is the replacement body.
'
{rewrite}printf '%s\n' '{{"type":"text","text":"Wrote the replacement body."}}'
printf '%s\n' '{{"type":"finish","sessionID":"conv"}}'
exit 0
"##,
            document = common::sh_quote(&document.display().to_string()),
            rewrite = common::INLINE_BODY_REWRITE,
        ),
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_writes_expected_final_body() {
    let fixture = CliProcessFixture::named("inline-compose-cli");

    let source = fixture.cwd().join("bare.md");
    write(
        &source,
        "---\nprompt: Generate the body\n---\nOriginal body.\n",
    );
    stage_opencode_inline_body_writer(fixture.bin_dir(), &source);

    fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .args(["inline-compose", "--opencode", source.to_str().unwrap()])
        .assert()
        .success();

    let doc = fs::read_to_string(&source).unwrap();
    // The agent's file is the deliverable; the narration it emitted around the
    // tool call must not appear in it.
    assert!(
        doc.contains("# Final Body"),
        "inline body must contain final response heading; doc:\n{doc}"
    );
    assert!(
        doc.contains("This is the replacement body."),
        "inline body must contain final response body; doc:\n{doc}"
    );
    assert!(
        !doc.contains("Let me look up the answer."),
        "inline body must not contain interstitial narration; doc:\n{doc}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_uses_source_doc_repository_not_launch_cwd() {
    let fixture = CliProcessFixture::named("inline-compose-cli");
    let source_root = fixture.cwd().join("source");
    let launch_root = fixture.cwd().join("launch");
    fs::create_dir_all(&source_root).unwrap();
    fs::create_dir_all(&launch_root).unwrap();
    assert!(init_git_repo(&source_root));
    assert!(init_git_repo(&launch_root));

    write(
        &source_root.join("snippet.md"),
        "SOURCE_REPOSITORY_INLINE_MARKER\n",
    );
    write(
        &launch_root.join("snippet.md"),
        "LAUNCH_REPOSITORY_INLINE_MARKER\n",
    );
    let source = source_root.join("prompts/inline.md");
    write(
        &source,
        "---\nprompt: |\n  CWD={{ ctx.cwd }}\n  ::file snippet.md\n---\nOriginal body.\n",
    );

    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    // The subject is the launch CWD losing to the source document's repository,
    // so the launch directory has to be a repository this test built.
    let assert = fixture
        .command_builder()
        .ambient_context(&launch_root)
        .build()
        .args([
            "inline-compose",
            "--goose",
            "--dry-run",
            source.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("SOURCE_REPOSITORY_INLINE_MARKER"),
        "inline prompt transclusion must use the source repository; stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("LAUNCH_REPOSITORY_INLINE_MARKER"),
        "launch repository must not leak into inline prompt transclusion; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "CWD={}",
            biscuit_file::to_portable_string(
                &launch_root.canonicalize().expect("canonical launch directory")
            )
        )),
        "inline composition must project the immutable launch directory; stdout:\n{stdout}"
    );
}
