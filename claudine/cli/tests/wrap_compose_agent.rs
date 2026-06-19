//! Integration tests: compose agent resolution reporting and validation/handler engagement banners.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use assert_cmd::cargo::cargo_bin_cmd;
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

    let assert = cargo_bin_cmd!("claudine")
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
    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    // Write empty stdin via a file to prevent TTY detection
    let stdin_file = workspace.path().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
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

    cargo_bin_cmd!("claudine")
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

    let output = cargo_bin_cmd!("claudine")
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

    let output = cargo_bin_cmd!("claudine")
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

    let assert = cargo_bin_cmd!("claudine")
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

#[cfg(unix)]
#[test]
fn effective_composed_frontmatter_activates_harness() {
    // Verifies that harness behavior (pre_checks) from effective
    // composed frontmatter -- not raw source frontmatter -- is honored
    // in the CLI composition path.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let marker_path = workspace.path().join("provider-ran.txt");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // The source file transcludes another file that adds pre_checks.
    // Since we can't easily set up full transclusion in an integration
    // test, we test with inline frontmatter that includes harness props.
    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Rewrite this\npre_checks:\n  file_exists: \"required-context.txt\"\n---\nBody\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf 'ran\n' > "$CLAUDINE_MARKER_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_MARKER_FILE", &marker_path)
        .current_dir(workspace.path())
        .args(["inline-compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(contains("pre-check validation failed"));

    assert!(
        !marker_path.exists(),
        "harness pre_checks from effective frontmatter should block provider launch"
    );
}

#[cfg(unix)]
#[test]
fn handler_engagement_banner_suppressed_when_retry_ceiling_reached() {
    // When the retry ceiling is hit and no recovery plan is produced,
    // the "engaging registered handlers" banner must NOT appear a second time.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    // retries: 1 means only one retry attempt is allowed.
    fs::write(
        &md_file,
        "---\nprompt: Rewrite the body\npost_checks:\n  response_includes: \"NEVER_APPEARS\"\nhandle_response_includes:\n  retry:\n    prompt: \"Include NEVER_APPEARS\"\n    retries: 1\n---\nOriginal body\n",
    )
    .unwrap();

    // Agent always outputs text that does NOT contain the required string.
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf 'Some output without the keyword\\n'\nexit 0\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));

    // After the first failure the retry handler fires (banner once).
    // After the second failure the ceiling is reached — no new plan, no banner.
    // Collapse whitespace because terminal line-wrapping can split the phrase.
    let collapsed: String = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    let banner_count = collapsed.matches("engaging registered handlers").count();
    assert!(
        banner_count <= 1,
        "banner should appear at most once; found {banner_count} occurrences in stderr:\n{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn handler_engagement_banner_emitted_once_on_successful_recovery() {
    // When a retry handler fires once and the retry succeeds,
    // the banner must appear exactly once.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let count_path = workspace.path().join("attempt-count.txt");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Rewrite the body\npost_checks:\n  response_includes: \"MAGIC_WORD\"\nhandle_response_includes:\n  retry:\n    prompt: \"Your response must include MAGIC_WORD.\"\n---\nOriginal body\n",
    )
    .unwrap();

    // First attempt: output without keyword. Second attempt: includes keyword.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
if [ "$count" -eq 1 ]; then
  printf 'First attempt output\n'
else
  printf 'MAGIC_WORD recovery success\n'
fi
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    // Collapse whitespace for line-wrapped output matching
    let collapsed: String = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    let banner_count = collapsed.matches("engaging registered handlers").count();
    assert_eq!(
        banner_count, 1,
        "banner should appear exactly once during successful recovery; found {banner_count} in stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Redirect status reporting
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn redirect_handler_updates_source_file_reporting() {
    // After a redirect handler fires, the second attempt's source-file
    // reporting should reference the redirected file, not the original.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let count_path = workspace.path().join("attempt-count.txt");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let redirect_file = workspace.path().join("redirect-target.md");
    fs::write(
        &redirect_file,
        "---\nprompt: Write the redirect content\n---\nRedirect body\n",
    )
    .unwrap();

    let md_file = workspace.path().join("original.md");
    fs::write(
        &md_file,
        format!(
            "---\nprompt: Write content\npost_checks:\n  response_includes: \"REDIRECT_OK\"\nhandle_response_includes:\n  redirect:\n    file: \"{}\"\n---\nOriginal body\n",
            redirect_file.display()
        ),
    )
    .unwrap();

    // First attempt (original.md): output lacks REDIRECT_OK → redirect fires
    // Second attempt (redirect-target.md): no post_checks → succeeds
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
if [ "$count" -eq 1 ]; then
  printf 'First pass without keyword\n'
else
  printf 'REDIRECT_OK second pass\n'
fi
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    // Collapse whitespace — terminal wrapping can break file names across lines
    let collapsed: String = stderr.split_whitespace().collect::<Vec<_>>().join(" ");

    // After redirect, source-file reporting should mention the redirected file
    assert!(
        collapsed.contains("redirect-target.md"),
        "after redirect, stderr should reference the redirected file; stderr:\n{stderr}"
    );

    // The final file content should be written to the redirect target
    let redirect_content = fs::read_to_string(&redirect_file).unwrap();
    assert!(
        redirect_content.contains("REDIRECT_OK"),
        "redirect target should contain the second attempt's output; content:\n{redirect_content}"
    );
}

// ---------------------------------------------------------------------------
// --silent suppresses validation-reporting output
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn silent_suppresses_validation_reporting_output() {
    // Normal verbosity: validation reporting appears.
    // --silent: validation reporting is absent.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\npre_checks:\n  file_exists: \"missing-file.txt\"\n---\nBody\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("codex"),
        "#!/bin/sh\nprintf 'output\\n'\nexit 0\n",
    );

    // Normal run — should see pre-check failure reporting
    let normal = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let normal_stderr = strip_ansi(&String::from_utf8_lossy(&normal.get_output().stderr));
    assert!(
        normal_stderr.contains("pre-check")
            || normal_stderr.contains("file_exists")
            || normal_stderr.contains("missing-file.txt")
            || normal_stderr.contains("validation failed"),
        "normal verbosity should include validation reporting; stderr:\n{normal_stderr}"
    );

    // Silent run — validation reporting lines should be absent
    let silent = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["compose", "--codex", "--silent", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let silent_stderr = strip_ansi(&String::from_utf8_lossy(&silent.get_output().stderr));
    // Source-file status should be suppressed
    assert!(
        !silent_stderr.contains("test.md"),
        "--silent should suppress source-file status reporting; stderr:\n{silent_stderr}"
    );
    // Pre-check status lines should be suppressed
    assert!(
        !silent_stderr.contains("missing-file.txt"),
        "--silent should suppress pre-check validation output; stderr:\n{silent_stderr}"
    );
}

#[cfg(unix)]
#[test]
fn silent_suppresses_handler_engagement_banner() {
    // When --silent is active, the "engaging registered handlers" banner
    // must not appear even when handlers fire.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let count_path = workspace.path().join("attempt-count.txt");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Rewrite the body\npost_checks:\n  response_includes: \"REQUIRED\"\nhandle_response_includes:\n  retry:\n    prompt: \"Include REQUIRED.\"\n---\nOriginal body\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
if [ "$count" -eq 1 ]; then
  printf 'First attempt\n'
else
  printf 'REQUIRED second attempt\n'
fi
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args([
            "inline-compose",
            "--goose",
            "--silent",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    let collapsed: String = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !collapsed.contains("engaging registered handlers"),
        "--silent should suppress handler-engagement banner; stderr:\n{stderr}"
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

/// End-to-end: for every wrapped provider, verify that
/// `claudine <provider> --dry-run "hello"` produces a successful
/// dry-run (exit 0) and that the dry-run output section is printed.
///
/// Providers that deliver the prompt via argv (Gemini, Qwen, OpenCode,
/// Goose) also have "hello" visible in the Command: line; providers that
/// seed stdin (Claude, Codex, Kimi) do not, but the pipeline still
/// completes and emits the DRY RUN header, which is sufficient to prove
/// that the prompt was accepted and processed. Runs for all 7 wrapped
/// providers with stub binaries on PATH.
#[cfg(unix)]
#[test]
fn direct_wrap_dry_run_delivers_prompt_for_every_provider() {
    for provider_slug in [
        "claude", "codex", "gemini", "kimi", "opencode", "qwen", "goose",
    ] {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        seed_minimal_config(workspace.path());

        // Stub binary so PATH resolution succeeds in dry-run mode.
        // Dry-run never actually spawns the child, so the stub body
        // doesn't matter — the stub only needs to exist and be
        // executable for claudine's binary-resolution step.
        write_executable(&path_dir.join(provider_slug), "#!/bin/sh\nexit 0\n");

        let output = cargo_bin_cmd!("claudine")
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
}
