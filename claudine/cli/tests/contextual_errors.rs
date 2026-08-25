//! Integration coverage for the unified contextual-error redesign.
//!
//! Exercises the three headline failure paths described in
//! `claudine/features/2026-04-21-contextual-errors/spec.md` and asserts
//! that the CLI's top-level cause-chain walker surfaces a rich darkmatter
//! `BlockError` report (path, line number, hint) instead of flat text.

use std::fs;
use tempfile::tempdir;

mod common;
#[cfg(unix)]
use common::CliProcessFixture;
use common::{augmented_path, strip_ansi, write_executable};

#[cfg(windows)]
fn shim_name(name: &str) -> String {
    format!("{name}.cmd")
}

#[cfg(not(windows))]
fn shim_name(name: &str) -> String {
    name.to_string()
}

#[cfg(windows)]
fn successful_provider_shim() -> &'static str {
    "@echo off\r\nexit /b 0\r\n"
}

#[cfg(not(windows))]
fn successful_provider_shim() -> &'static str {
    "#!/bin/sh\nexit 0\n"
}

#[cfg(windows)]
fn failing_rustc_shim() -> &'static str {
    "@echo off\r\necho error: invalid value for --edition 1>&2\r\nexit /b 1\r\n"
}

#[cfg(not(windows))]
fn failing_rustc_shim() -> &'static str {
    "#!/bin/sh\nprintf '%s\\n' 'error: invalid value for --edition' >&2\nexit 1\n"
}

/// Shell-expansion failure during `claudine compose`.
///
/// Uses a blacklisted command (`rm -rf /`) so the test works deterministically
/// without needing a whitelist file or an approval handler: shell expansion
/// rejects it at pre-flight with a structured `Blacklisted` variant that the
/// darkmatter `BlockError` renderer turns into a rich report.
#[cfg(unix)]
#[test]
fn compose_shell_expansion_failure_renders_rich_block() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let md_file = workspace.path().join("compose.md");
    fs::write(
        &md_file,
        "---\ntitle: Shell demo\n---\n\nPre.\n\n::shell rm -rf /\n\nPost.\n",
    )
    .unwrap();

    // Fake claude binary so the wrapper pipeline picks a provider and
    // reaches shell-expansion resolution.
    write_executable(
        &path_dir.join(shim_name("claude")),
        successful_provider_shim(),
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--claude", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("blacklisted") || plain.contains("Blacklisted"),
        "expected rich shell-expansion report to mention blacklist; got:\n{plain}"
    );
    assert!(
        plain.contains("rm") || plain.contains("rm -rf"),
        "expected the denied command to appear in the report; got:\n{plain}"
    );
    assert!(
        !plain.contains("__claudine_pre_rendered__"),
        "legacy pre-render marker must never reach the user; got:\n{plain}"
    );
}

/// Real `ExecutionFailed` shell-expansion failure during `claudine compose`.
///
/// Uses a deterministic failing `rustc` fixture so the test exercises the full
/// boundary from Markdown composition through the CLI's top-level error walker,
/// including in archive-only environments without a Rust toolchain.
#[test]
fn compose_shell_execution_failure_renders_rich_block() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let md_file = workspace.path().join("compose.md");
    fs::write(
        &md_file,
        "---\ntitle: Shell demo\n---\n\nPre.\n\n::shell rustc --edition=invalid\n\nPost.\n",
    )
    .unwrap();

    // Fake claude binary so the wrapper pipeline picks a provider and
    // reaches shell-expansion resolution.
    write_executable(
        &path_dir.join(shim_name("claude")),
        successful_provider_shim(),
    );
    write_executable(
        &path_dir.join(shim_name("rustc")),
        failing_rustc_shim(),
    );

    // `--silent` suppresses the execution header so the only stderr output
    // is the structured error block, letting us assert the full captured
    // stderr is ANSI-free under `NO_COLOR=1`.
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--yolo", "--silent", "--claude", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr_raw = String::from_utf8_lossy(&assert.get_output().stderr).to_string();

    assert!(
        !stderr_raw.contains('\x1b'),
        "raw stderr must contain no ANSI escape bytes under NO_COLOR=1; got:\n{stderr_raw:?}"
    );

    let plain = strip_ansi(&stderr_raw);

    assert!(
        plain.contains("line 7") || plain.contains("> 7 │"),
        "expected file-relative line in diagnostic; got:\n{plain}"
    );
    assert!(
        plain.contains("error:"),
        "expected captured rustc stderr text in diagnostic; got:\n{plain}"
    );
    assert!(
        plain.contains("::shell"),
        "expected source excerpt in diagnostic; got:\n{plain}"
    );
    assert!(
        plain.contains("title:") || plain.contains("---"),
        "expected frontmatter block in diagnostic; got:\n{plain}"
    );
    assert!(
        !plain.contains("__claudine_pre_rendered__"),
        "legacy pre-render marker must never reach the user; got:\n{plain}"
    );
}

/// System-prompt composition failure via `--append-system-prompt`.
///
/// The system prompt file contains a blacklisted `::shell` directive.
/// `ClaudineError::SystemPromptComposition` now carries the typed
/// `MarkdownError`, so the top-level walker must surface the shell
/// expansion's structured `Blacklisted` report.
#[cfg(unix)]
#[test]
fn compose_system_prompt_shell_failure_renders_rich_block() {
    let fixture = CliProcessFixture::named("compose-system-prompt-shell-failure");

    let md_file = fixture.cwd().join("prompt.md");
    fs::write(&md_file, "---\ntitle: demo\n---\nHello compose\n").unwrap();

    // Broken system prompt: ::shell with a blacklisted command.
    let sp_file = fixture.cwd().join("append-system-prompt.md");
    fs::write(&sp_file, "Base system prompt.\n\n::shell rm -rf /\n").unwrap();

    // Fake codex binary so the wrapper pipeline picks a provider and
    // reaches system-prompt resolution.
    write_executable(&fixture.bin_dir().join("codex"), "#!/bin/sh\nexit 0\n");

    let assert = fixture
        .command()
        .args([
            "compose",
            "--codex",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("blacklisted") || plain.contains("Blacklisted"),
        "expected system-prompt shell failure to render blacklist report; got:\n{plain}"
    );
    assert!(
        plain.contains("rm"),
        "expected the denied command to appear in the report; got:\n{plain}"
    );
    assert!(
        !plain.contains("__claudine_pre_rendered__"),
        "legacy pre-render marker must never reach the user; got:\n{plain}"
    );
}

/// Transclusion cycle surfaced during `claudine compose`.
///
/// Two files transclude one another in a loop. The top-level walker should
/// surface darkmatter's `TransclusionError::CycleDetected` report with both
/// file paths in the chain.
#[test]
fn compose_transclusion_cycle_renders_file_chain() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let a = workspace.path().join("a.md");
    let b = workspace.path().join("b.md");
    fs::write(&a, "Start a.\n\n::file ./b.md\n\nEnd a.\n").unwrap();
    fs::write(&b, "Start b.\n\n::file ./a.md\n\nEnd b.\n").unwrap();

    // Fake claude binary so the wrapper pipeline picks a provider and
    // reaches transclusion resolution.
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--claude", a.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("cycle") || plain.contains("Cycle"),
        "expected transclusion-cycle report to mention a cycle; got:\n{plain}"
    );
    assert!(
        plain.contains("a.md") && plain.contains("b.md"),
        "expected both files in the cycle chain to be reported; got:\n{plain}"
    );
    assert!(
        !plain.contains("__claudine_pre_rendered__"),
        "legacy pre-render marker must never reach the user; got:\n{plain}"
    );
}
