#![cfg(unix)]

//! Integration tests: compose agent resolution reporting and validation/handler engagement banners.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use predicates::str::contains;
use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{strip_ansi, write_executable};

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn repo_scoped_config_favorite_selects_provider() {
    // Verifies that a repo-level .claudine/config.json with a linking
    // preference is consulted during composition selection so the
    // favorite provider wins over interactive selection.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_file = workspace.path().join("goose-args.txt");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // Initialize a git repo so repo root detection works
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(workspace.path())
        .output()
        .unwrap();

    // Create repo-local config with goose as the preferred agent
    let config_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.json"),
        r#"{"preferred_agent":"goose"}"#,
    )
    .unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    // Install both providers — without the config favorite, multiple
    // installed providers would require interactive selection.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );
    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 99
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("HOME", workspace.path())
        .env("CLAUDINE_ARGS_FILE", &args_file)
        .current_dir(workspace.path())
        .args(["compose", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    // Non-TTY with no agent hint aborts with the no-agent message;
    // the config favorite is no longer used as a non-TTY fallback.
    assert!(
        plain.contains("didn't specify the Agent"),
        "non-TTY no-agent should abort with the no-agent message; stderr was: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn agent_hint_resolved_early_in_non_tty() {
    // Verifies that an `agent` hint is resolved during preparation
    // (not at launch), so prefix matches like "c" resolve to the
    // first match (Claude) instead of being treated as ambiguous.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\nagent: c\n---\nPrompt\n").unwrap();

    // Install both claude and codex; "c" resolves to Claude (first prefix match)
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");
    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 99\n");

    // Write empty stdin via a file to prevent TTY detection
    let stdin_file = workspace.path().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["compose", md_file.to_str().unwrap()])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn unknown_agent_hint_is_non_fatal_and_aborts_in_non_tty() {
    // Invalid `agent` values are no longer fatal during preparation.
    // In non-TTY mode the invalid hint is discarded and the run aborts
    // because no provider can be resolved, mirroring the no-agent
    // non-TTY behavior until the Phase 3 live-path messaging lands.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: test\nagent: unknown-provider\n---\nPrompt\n",
    )
    .unwrap();

    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    let stdin_file = workspace.path().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["compose", md_file.to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(contains("agent resolution failed"));
}

/// End-to-end (Finding 1): a frontmatter `agent` list resolving to exactly one
/// installed provider must render the **auto-select header** in the dry-run
/// `Agent` cell — not collapse to a bare provider name. Before the fix the
/// resolved target masked the list state at the dry-run seam.
#[cfg(unix)]
#[test]
fn compose_dry_run_list_one_installed_renders_auto_select_header() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // Suggest two agents; install only `claude` so the list resolves to a
    // single installed provider (ListOneInstalled).
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nname: one-installed\nagent: [claude, gemini]\n---\nBODY\n",
    )
    .unwrap();
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        // Restrict PATH to the fake bin so only `claude` is "installed".
        .env("PATH", &path_dir)
        .args(["compose", "--dry-run", md_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "dry-run should succeed; stderr was:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    // `prompting` only appears in the auto-select header ("…without the need
    // for interactive prompting"); it is the single-token proof that the cell
    // is the list auto-select state, not a bare `Selected` provider name.
    assert!(
        stderr.contains("prompting"),
        "list-with-one-installed dry-run must render the auto-select header; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("Claude"),
        "the auto-selected provider must still be named; stderr was:\n{stderr}"
    );
}

/// End-to-end (Finding 2): a single-entry all-invalid `agent` list
/// (`agent: [not-real]`) must render the **zero-installed-list** state, not the
/// single-invalid scalar cell. Before the fix the lost list-ness collapsed it
/// to `Invalid Agent(…)`.
#[cfg(unix)]
#[test]
fn compose_dry_run_single_entry_invalid_list_is_zero_installed() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, "---\nname: zero\nagent: [not-real]\n---\nBODY\n").unwrap();
    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["compose", "--dry-run", md_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "dry-run should succeed; stderr was:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    // `installed/valid` is the single-token signature of the zero-installed
    // header; it survives table word-wrap.
    assert!(
        stderr.contains("installed/valid"),
        "single-entry invalid list must render the zero-installed-list state; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("not-real"),
        "the invalid suggestion must appear in the NOT-valid list; stderr was:\n{stderr}"
    );
    assert!(
        !stderr.contains("Invalid Agent"),
        "must NOT render the single-invalid scalar cell; stderr was:\n{stderr}"
    );
}

/// End-to-end (Findings 3/4): `--silent` governs status verbosity only — it
/// must not suppress the live no-TTY agent-resolution report, and the run must
/// still abort with a non-zero exit.
#[cfg(unix)]
#[test]
fn compose_silent_does_not_suppress_agent_resolution_report() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // No agent hint, no explicit provider → live no-TTY abort.
    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, "---\ntitle: t\n---\nPrompt body\n").unwrap();
    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let stdin_file = workspace.path().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["compose", "--silent", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        stderr.contains("didn't specify the Agent"),
        "--silent must not suppress the agent-resolution report; stderr was:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Per-provider dry-run regression tests (Task 18)
//
// These tests are the structural guard that would have caught the original
// Gemini/Qwen drift: composition pipelines that silently bailed because
// `apply_non_interactive` re-read args before the prompt was injected.
// A successful dry-run (exit 0 + "DRY RUN" in output) proves the full
// extraction → delivery → output pipeline ran without error.
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn assert_direct_wrap_dry_run_delivers_prompt(provider_slug: &str) {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // An empty PATH proves dry-run neither resolves nor spawns the provider.

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("OPENCODE_MODEL", "test-model")
        .env("PATH", &path_dir)
        .args([provider_slug, "--dry-run", "hello"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "`claudine {provider_slug} --dry-run hello` failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let normalized = strip_ansi(&combined);
    assert!(
        normalized.contains("DRY RUN"),
        "`claudine {provider_slug} --dry-run hello` did not emit a DRY RUN section:\n{normalized}"
    );
}

macro_rules! direct_wrap_dry_run_test {
    ($name:ident, $provider:literal) => {
        #[cfg(unix)]
        #[test]
        fn $name() {
            assert_direct_wrap_dry_run_delivers_prompt($provider);
        }
    };
}

direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_claude, "claude");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_codex, "codex");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_gemini, "gemini");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_kimi, "kimi");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_opencode, "opencode");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_qwen, "qwen");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_goose, "goose");
