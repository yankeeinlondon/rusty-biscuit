#![cfg(unix)]

//! End-to-end tests for the prompt reporting feature (Phase 5).
//!
//! Verifies visual layout, token counting, markdown formatting, and
//! suppression behaviour across standard `compose` workflows.

use std::fs;
use tempfile::tempdir;

mod common;
use common::{augmented_path, strip_ansi, write_executable};

fn make_workspace_with_goose() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    let md_file = workspace.path().join("prompt.md");
    fs::write(&md_file, "# Compose body\nHello from compose.\n").unwrap();
    (workspace, path_dir, md_file)
}

fn make_workspace_with_goose_and_system_prompt() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let (workspace, path_dir, md_file) = make_workspace_with_goose();
    let sp_file = workspace.path().join("system-prompt.md");
    fs::write(
        &sp_file,
        "# Test System Prompt\n\nThis is a test system prompt for token counting.\n",
    )
    .unwrap();
    (workspace, path_dir, md_file, sp_file)
}

fn make_workspace_with_goose_and_verbose_system_prompt() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let (workspace, path_dir, md_file) = make_workspace_with_goose();
    let sp_file = workspace.path().join("system-prompt.md");
    fs::write(
        &sp_file,
        "---\nverbosity: verbose\n---\n\n# Test System Prompt\n\nThis is a test system prompt for token counting.\n",
    )
    .unwrap();
    (workspace, path_dir, md_file, sp_file)
}

#[cfg(unix)]
#[test]
fn compose_default_shows_summary_and_short_user_prompt() {
    let (workspace, path_dir, md_file, sp_file) = make_workspace_with_goose_and_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // System prompt header should appear
    assert!(
        plain.contains("System Prompt"),
        "default mode should show system prompt header; stderr:\n{plain}"
    );

    // Token count should appear in summary
    assert!(
        plain.contains("tokens"),
        "system prompt summary should show token count; stderr:\n{plain}"
    );

    // User prompt header should appear
    assert!(
        plain.contains("Agent Prompt"),
        "default mode should show agent prompt header; stderr:\n{plain}"
    );

    // User prompt body should appear (short prompt)
    assert!(
        plain.contains("Hello from compose."),
        "short user prompt should be shown in full; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_verbose_shows_full_prompts() {
    let (workspace, path_dir, md_file, sp_file) = make_workspace_with_goose_and_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            "--verbose",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // Full system prompt body should appear
    assert!(
        plain.contains("Test System Prompt"),
        "verbose mode should show full system prompt body; stderr:\n{plain}"
    );

    // Per spec, verbose mode renders both Summary and Full Prompt.
    assert!(
        plain.contains("tokens"),
        "verbose mode should also show the summary line with token count; stderr:\n{plain}"
    );
    assert!(
        plain.contains("The system prompt was"),
        "verbose mode should render the spec-format summary prose; stderr:\n{plain}"
    );

    // Full user prompt body should appear
    assert!(
        plain.contains("Hello from compose."),
        "verbose mode should show full user prompt; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_quiet_shows_system_summary_and_full_agent_prompt() {
    // Spec 6.3: `--quiet` forces the System Prompt to Summary mode but is a
    // no-op for the User Prompt — header and body still render.
    let (workspace, path_dir, md_file, sp_file) = make_workspace_with_goose_and_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            "--quiet",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // System prompt header + summary line.
    assert!(
        plain.contains("System Prompt"),
        "quiet mode should still show system prompt header; stderr:\n{plain}"
    );
    assert!(
        plain.contains("tokens"),
        "quiet mode should show token count in summary; stderr:\n{plain}"
    );
    // System body must NOT appear in quiet mode.
    assert!(
        !plain.contains("Test System Prompt"),
        "quiet mode should suppress system prompt body; stderr:\n{plain}"
    );

    // Agent Prompt MUST still render under --quiet (header + body).
    assert!(
        plain.contains("Agent Prompt"),
        "quiet mode is a no-op for the user prompt and must keep its header; stderr:\n{plain}"
    );
    assert!(
        plain.contains("Hello from compose."),
        "quiet mode must keep the user prompt body visible; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_silent_suppresses_all_prompt_reporting() {
    let (workspace, path_dir, md_file, sp_file) = make_workspace_with_goose_and_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            "--silent",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // System prompt should be suppressed
    assert!(
        !plain.contains("System Prompt"),
        "silent mode should suppress system prompt; stderr:\n{plain}"
    );

    // User prompt should be suppressed
    assert!(
        !plain.contains("Agent Prompt"),
        "silent mode should suppress agent prompt; stderr:\n{plain}"
    );

    // Token count should not appear
    assert!(
        !plain.contains("tokens"),
        "silent mode should suppress token count; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_long_user_prompt_uses_frontback_truncation() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    // Create a user prompt with >40 lines
    // List items (not a bare paragraph): consecutive non-blank Markdown lines
    // reflow into a single rendered row, but each list item renders on its own
    // row — which is what front/back row truncation operates on.
    let long_prompt: String = (1..=50)
        .map(|i| format!("- Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let md_file = workspace.path().join("prompt.md");
    fs::write(&md_file, format!("# Long Prompt\n\n{long_prompt}\n")).unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // Header should appear
    assert!(
        plain.contains("Agent Prompt"),
        "long user prompt should still show header; stderr:\n{plain}"
    );

    // First lines should appear
    assert!(
        plain.contains("Line 1"),
        "front portion of long prompt should be shown; stderr:\n{plain}"
    );

    // Last lines should appear (check for " 50" since wrapping may split "Line 50")
    assert!(
        plain.contains(" 50"),
        "back portion of long prompt should be shown; stderr:\n{plain}"
    );

    // Middle lines should be truncated
    assert!(
        !plain.contains("Line 25"),
        "middle lines should be truncated in FrontBack mode; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_system_prompt_summary_shows_token_count() {
    let (workspace, path_dir, md_file, sp_file) = make_workspace_with_goose_and_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // Token count should be present and look like a number followed by "tokens"
    let tokens_found = plain
        .split_whitespace()
        .any(|w| w.parse::<u64>().is_ok() && plain.contains(&format!("{w} tokens")));
    assert!(
        tokens_found,
        "system prompt summary should contain a numeric token count; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_env_var_verbose_shows_full_system_prompt() {
    let (workspace, path_dir, md_file, sp_file) = make_workspace_with_goose_and_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .env("CLAUDINE_SYSTEM_PROMPT", "verbose")
        .args([
            "compose",
            "--goose",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("Test System Prompt"),
        "CLAUDINE_SYSTEM_PROMPT=verbose should show full body; stderr:\n{plain}"
    );
    assert!(
        plain.contains("tokens"),
        "CLAUDINE_SYSTEM_PROMPT=verbose should include summary line; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_env_var_quiet_shows_summary_only() {
    let (workspace, path_dir, md_file, sp_file) = make_workspace_with_goose_and_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .env("CLAUDINE_SYSTEM_PROMPT", "quiet")
        .args([
            "compose",
            "--goose",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("System Prompt"),
        "CLAUDINE_SYSTEM_PROMPT=quiet should still show header; stderr:\n{plain}"
    );
    assert!(
        plain.contains("tokens"),
        "CLAUDINE_SYSTEM_PROMPT=quiet should show summary line; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("Test System Prompt"),
        "CLAUDINE_SYSTEM_PROMPT=quiet should suppress body; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_env_var_silent_suppresses_system_prompt() {
    let (workspace, path_dir, md_file, sp_file) = make_workspace_with_goose_and_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .env("CLAUDINE_SYSTEM_PROMPT", "silent")
        .args([
            "compose",
            "--goose",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        !plain.contains("System Prompt"),
        "CLAUDINE_SYSTEM_PROMPT=silent should suppress system prompt; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_user_prompt_at_40_lines_renders_full() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    // Build a prompt whose final body has exactly 40 lines. The compose
    // pipeline doesn't add lines for a body-only file, so 40 numbered lines
    // produce a 40-line user prompt.
    let body: String = (1..=40)
        .map(|i| format!("Line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let md_file = workspace.path().join("prompt.md");
    fs::write(&md_file, &body).unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("Agent Prompt"),
        "40-line user prompt should still show header; stderr:\n{plain}"
    );
    // Front+middle+last should all appear (no truncation at 40 lines).
    assert!(
        plain.contains("Line1"),
        "should show front of 40-line prompt"
    );
    assert!(
        plain.contains("Line20"),
        "should show middle of 40-line prompt (no truncation at 40)"
    );
    assert!(
        plain.contains("Line40"),
        "should show last line of 40-line prompt"
    );
}

#[cfg(unix)]
#[test]
fn compose_user_prompt_at_41_lines_uses_frontback() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
echo "Agent response"
exit 0
"#,
    );

    // List items so each renders on its own row (a bare paragraph would
    // reflow into a single row and never exceed the front/back row budget).
    let body: String = (1..=41)
        .map(|i| format!("- Line{i:03}"))
        .collect::<Vec<_>>()
        .join("\n");
    let md_file = workspace.path().join("prompt.md");
    fs::write(&md_file, &body).unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(plain.contains("Agent Prompt"));
    assert!(plain.contains("Line001"), "front portion should appear");
    assert!(plain.contains("Line041"), "back portion should appear");
    // Middle (around Line021) should be omitted in FrontBack mode (20/10).
    assert!(
        !plain.contains("Line025"),
        "middle lines should be truncated for >40-line prompt; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_frontmatter_verbose_shows_full_system_prompt() {
    let (workspace, path_dir, md_file, sp_file) =
        make_workspace_with_goose_and_verbose_system_prompt();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "compose",
            "--goose",
            "--append-system-prompt",
            sp_file.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // Full system prompt body should appear due to frontmatter verbosity
    assert!(
        plain.contains("Test System Prompt"),
        "frontmatter verbose should show full system prompt body; stderr:\n{plain}"
    );

    // Per spec, verbose renders Summary + Full Prompt — token line MUST appear.
    assert!(
        plain.contains("tokens"),
        "frontmatter verbose should also show the summary line with token count; stderr:\n{plain}"
    );
}
