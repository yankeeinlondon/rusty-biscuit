//! Integration tests: wrapper CLI surface basics: help, flag/argv validation, env sanitization, exit-code propagation, dry-run, model flag, and PID injection.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{strip_ansi, write_dry_run_provider_stub, write_executable};

#[test]
fn help_lists_wrapper_subcommands() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["--help"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    insta::assert_snapshot!(strip_ansi(&stdout));
}

#[test]
fn wrapper_help_includes_expected_flags() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["codex", "--help"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let plain = strip_ansi(&stdout);
    assert!(
        plain.contains("Usage"),
        "help output should include usage information; stdout was: {plain}"
    );
    assert!(
        plain.contains("--help"),
        "help output should mention the help flag; stdout was: {plain}"
    );
    assert!(
        plain.contains("--model") || plain.contains("model"),
        "help output should describe model selection; stdout was: {plain}"
    );
    assert!(
        plain.contains("--edit"),
        "help output should describe prompt editing; stdout was: {plain}"
    );
    assert!(
        plain.contains("--perf"),
        "help output should describe performance reporting; stdout was: {plain}"
    );
}

#[test]
fn wrapper_rejects_edit_and_interactive_conflict() {
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["codex", "--edit", "--interactive"])
        .assert()
        .failure()
        .stderr(contains("--edit"))
        .stderr(contains("--interactive"))
        .stderr(contains("cannot be used"));
}

#[cfg(unix)]
#[test]
fn wrapper_preserves_passthrough_args_and_injects_env() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let args_path = workspace.path().join("args.txt");
    let env_path = workspace.path().join("env.txt");
    let stdin_path = workspace.path().join("stdin.txt");

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
/bin/cat > "$CLAUDINE_STDIN_FILE"
{
  printf 'AGENT=%s\n' "$AGENT"
  printf 'YOLO=%s\n' "$YOLO"
  printf 'INTERACTIVE=%s\n' "$INTERACTIVE"
  printf 'AGENT_PARAMS=%s\n' "$AGENT_PARAMS"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
        .current_dir(workspace.path())
        .args(["codex", "--yolo", "--", "--json", "summarize repo"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(
        args.len() >= 3,
        "expected at least exec + --json + --dangerously-bypass-approvals-and-sandbox, got {args:?}"
    );
    assert_eq!(args[0], "exec");
    assert_eq!(args[1], "--json");
    assert_eq!(args[2], "--dangerously-bypass-approvals-and-sandbox");
    // Non-interactive Codex sessions append the safety appendix via
    // -c developer_instructions="..." argv tokens; accept extra args.

    let stdin = fs::read_to_string(&stdin_path).unwrap();
    assert_eq!(stdin, "summarize repo");

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("AGENT=codex"));
    assert!(env_lines.contains("YOLO=true"));
    assert!(env_lines.contains("INTERACTIVE=false"));
    assert!(env_lines.contains("AGENT_PARAMS=["));
}

#[cfg(unix)]
#[test]
fn wrapper_rejects_edit_without_interactive_terminal_before_launch() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["codex", "summarize repo", "--edit"])
        .assert()
        .failure()
        .stderr(contains("--edit requires an interactive terminal"));

    assert!(
        !args_path.exists(),
        "provider should not launch when --edit is requested without a TTY"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_preserves_post_boundary_edit_passthrough() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["codex", "--", "--edit", "--version"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    assert!(
        args.lines().any(|line| line == "--edit"),
        "post-boundary --edit should reach the provider; args were: {args}"
    );
}

#[cfg(unix)]
#[test]
fn codex_wrapper_uses_shadow_home_for_repo_prompt_overlay_without_repo_flag() {
    let workspace = tempdir().unwrap();
    let repo_dir = workspace.path().join("repo");
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let env_path = workspace.path().join("env.txt");

    fs::create_dir_all(&repo_dir).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(&fake_home);
    fs::create_dir_all(fake_home.join(".codex")).unwrap();
    fs::create_dir_all(repo_dir.join(".claude/commands")).unwrap();
    fs::write(
        repo_dir.join(".claude/commands/review.md"),
        "---\ndescription: review\n---\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
{
  printf 'HOME=%s\n' "$HOME"
  if [ -L "$HOME/.codex/prompts/review.md" ]; then
    printf 'HAS_REPO_PROMPT=1\n'
  else
    printf 'HAS_REPO_PROMPT=0\n'
  fi
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .current_dir(&repo_dir)
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["codex", "--", "--version"])
        .assert()
        .success();

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains(&format!("HOME={}", fake_home.join(".claudine").display())));
    assert!(env_lines.contains("HAS_REPO_PROMPT=1"));
}

#[cfg(unix)]
#[test]
fn wrapper_reports_removed_sensitive_env_names() {
    let workspace = tempdir().unwrap();
    let cwd_dir = workspace.path().join("cwd");
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&cwd_dir).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(&fake_home);
    fs::create_dir_all(fake_home.join(".codex")).unwrap();

    // Seed a deterministic system-prompt.md next to the launch CWD so the
    // pre-flight token count is stable across repo state and machines.
    fs::write(
        cwd_dir.join("system-prompt.md"),
        "You are a test fixture system prompt.\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env_clear()
        .env("HOME", &fake_home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("TERM", "dumb")
        .env("TERM_WIDTH", "80")
        .env("OPENAI_API_KEY", "keep")
        .env("INTERNAL_TOKEN", "remove")
        .current_dir(&cwd_dir)
        .args(["codex", "--include", "OPENAI_API_KEY", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let redacted = redact_workspace_paths(
        workspace.path(),
        &redact_claudine_pid(&redact_temp_home(&redact_session_id(&strip_ansi(&stderr)))),
    );
    insta::assert_snapshot!(redacted);
}

#[cfg(unix)]
#[test]
fn wrapper_propagates_child_exit_code() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 17
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--", "--version"])
        .assert()
        .code(17);
}

#[cfg(unix)]
#[test]
fn wrapper_consumes_non_interactive_alias_from_passthrough() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let args_path = workspace.path().join("args.txt");
    let stdin_path = workspace.path().join("stdin.txt");

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
/bin/cat > "$CLAUDINE_STDIN_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
        .current_dir(workspace.path())
        .args(["codex", "--json", "summarize repo"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(
        args.len() >= 2,
        "expected at least exec + --json, got {args:?}"
    );
    assert_eq!(args[0], "exec");
    assert_eq!(args[1], "--json");
    // Non-interactive Codex sessions append the safety appendix via
    // -c developer_instructions="..." argv tokens; accept extra args.

    let stdin = fs::read_to_string(&stdin_path).unwrap();
    assert_eq!(stdin, "summarize repo");
}

#[cfg(unix)]
#[test]
fn wrapper_logs_are_written_to_stderr_not_stdout() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--", "--version"])
        .assert()
        .success()
        .stdout("")
        .stderr(contains("Claudine"))
        .stderr(contains("Environment Variables"));
}

#[cfg(unix)]
#[test]
fn wrapper_rejects_direct_provider_yolo_flag_with_guidance() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["claude", "--dangerously-skip-permissions", "hi"])
        .assert()
        .code(1)
        .stderr(contains("Error:"))
        .stderr(contains("do not pass"))
        .stderr(contains("--dangerously-skip-permissions"))
        .stderr(contains("use Claudine's"))
        .stderr(contains("--yolo"))
        .stderr(contains("-y"))
        .stderr(contains("switches instead"));
}

#[cfg(unix)]
#[test]
fn wrapper_sets_interactive_true_by_default() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    // Isolate HOME so the wrapper reads a seeded empty config instead of the
    // developer's real `~/.claudine/config.json`, whose lifecycle actions can
    // spawn detached side-effect processes that hold the stdout pipe open
    // (assert_cmd then blocks on the pipe and nextest reports leaked handles).
    seed_minimal_config(workspace.path());
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf 'INTERACTIVE=%s\n' "$INTERACTIVE" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["codex", "--", "--version"])
        .assert()
        .success();

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("INTERACTIVE=true"));
}

#[cfg(unix)]
#[test]
fn wrapper_header_shows_provider_name() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    let header_line = plain
        .lines()
        .find(|line| line.contains("Claudine"))
        .unwrap();
    assert!(
        header_line.contains("Codex"),
        "Header should contain 'Codex' but was: {header_line}"
    );
}

// ---------------------------------------------------------------------------
// Gemini wrapper (review 6.2)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn wrapper_dry_run_prints_command_and_exits_zero() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
echo "SHOULD NOT RUN"
exit 1
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--dry-run", "--", "--version"])
        .assert()
        .success()
        .stdout(""); // no stdout means child did not run

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("DRY RUN"));
    assert!(plain.contains("Command:"));
    assert!(plain.contains("codex"));
}

#[test]
fn codex_dry_run_discovered_replace_system_prompt_uses_model_instructions_file() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    fs::write(
        workspace.path().join("system-prompt.md"),
        "---\nmode: replace\n---\n\nUse the replacement prompt.\n",
    )
    .unwrap();

    write_dry_run_provider_stub(&path_dir, "codex");

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .current_dir(workspace.path())
        .args(["codex", "--dry-run", "inspect the repo"])
        .assert()
        .success()
        .stdout("");

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("model_instructions_file="),
        "discovered mode: replace should use Codex replace delivery; stderr was:\n{plain}"
    );
    assert!(
        !plain.contains("developer_instructions="),
        "discovered mode: replace should not use Codex append delivery; stderr was:\n{plain}"
    );
}

#[test]
fn codex_dry_run_discovered_replace_system_prompt_reports_effective_mode() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    fs::write(
        workspace.path().join("system-prompt.md"),
        "---\nmode: replace\n---\n\nUse the replacement prompt.\n",
    )
    .unwrap();

    write_dry_run_provider_stub(&path_dir, "codex");

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .current_dir(workspace.path())
        .args(["codex", "--dry-run", "inspect the repo"])
        .assert()
        .success()
        .stdout("");

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("System prompt:"),
        "dry-run output should include the system-prompt report; stderr was:\n{plain}"
    );
    assert!(
        plain.contains("mode: replace"),
        "dry-run output should report the effective replace mode; stderr was:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_quiet_suppresses_summary() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let system_prompt = workspace.path().join("system-prompt.md");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    fs::write(&system_prompt, "Quiet mode prompt").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    // --quiet shows header but suppresses env details and info
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args([
            "codex",
            "--quiet",
            "--append-system-prompt",
            system_prompt.to_str().unwrap(),
            "--",
            "--version",
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let stderr_plain = strip_ansi(&stderr);
    assert!(
        stderr_plain.contains("Claudine"),
        "Quiet mode should show header but stderr was: {stderr}"
    );
    assert!(
        !stderr_plain.contains("Environment Variables"),
        "Quiet mode should suppress env details but stderr was: {stderr}"
    );
    assert!(
        stderr_plain.contains("System Prompt (appended)"),
        "Quiet mode should still show the system prompt when set but stderr was: {stderr}"
    );

    // --silent suppresses everything
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--silent", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        !stderr.contains("Claudine"),
        "Silent mode should suppress all output but stderr was: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_missing_explicit_system_prompt_fails_visibly() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let missing_prompt = workspace.path().join("missing-prompt.md");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args([
            "codex",
            "--append-system-prompt",
            missing_prompt.to_str().unwrap(),
            "--",
            "--version",
        ])
        .assert()
        .code(1)
        // The path is named, but a `StatusBlock` word-wraps it at the terminal
        // width, so match the file name's tail rather than the whole path — the
        // assertion is "the operator is told which file", not "the path is on
        // one line".
        .stderr(contains("system prompt file not found"))
        .stderr(contains("prompt.md"))
        // `ClaudineError` reaches the walker through the diagnostic registry,
        // so this renders a coded block rather than the generic `Error:` line.
        .stderr(contains("io.read_failed"));
}

#[cfg(unix)]
#[test]
fn wrapper_universal_model_flag_passes_to_provider() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["claude", "--model", "claude-sonnet-4-6", "hi"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(args.contains(&"--model"));
    assert!(args.contains(&"claude-sonnet-4-6"));
}

// ---------------------------------------------------------------------------
// Security: sensitive pattern expansion (review 4.2)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn wrapper_removes_new_sensitive_env_patterns() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("SSH_PRIVATE_KEY", "secret")
        .env("AWS_ACCESS_KEY_ID", "secret")
        .env("DB_CREDENTIAL", "secret")
        .args(["codex", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("SSH_PRIVATE_KEY"));
    assert!(plain.contains("AWS_ACCESS_KEY_ID"));
    assert!(plain.contains("DB_CREDENTIAL"));
}

// ---------------------------------------------------------------------------
// Timeout requires non-interactive (review 3.2)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn wrapper_timeout_rejects_in_interactive_mode() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    // No prompt → interactive by default → --timeout should fail
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--timeout", "30s"])
        .assert()
        .code(1)
        .stderr(contains(
            "--timeout can only be used in non-interactive mode",
        ));

    // --interactive + --timeout → explicit conflict
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["codex", "--timeout", "30s", "-i", "--", "hello"])
        .assert()
        .code(1)
        .stderr(contains("--timeout cannot be used with --interactive mode"));
}

// ---------------------------------------------------------------------------
// AGENT_PARAMS redaction (review 4.6)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn wrapper_redacts_sensitive_args_in_agent_params() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf 'AGENT_PARAMS=%s\n' "$AGENT_PARAMS" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args([
            "codex",
            "--",
            "--api-key=sk-secret-key",
            "--token",
            "bearer-xyz",
            "--json",
            "task",
        ])
        .assert()
        .success();

    let env_lines = fs::read_to_string(&env_path).unwrap();
    // Secrets should be redacted
    assert!(!env_lines.contains("sk-secret-key"));
    assert!(!env_lines.contains("bearer-xyz"));
    // Non-secret args should be present
    assert!(env_lines.contains("--json"));
    assert!(env_lines.contains("task"));
    // Redacted markers should be present
    assert!(env_lines.contains("****"));
}

#[cfg(unix)]
#[test]
fn wrapper_injects_claudine_pid_into_provider_env() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
{
  printf 'CLAUDINE_PID=%s\n' "$CLAUDINE_PID"
  if [ -n "${AGENT_PID:-}" ]; then
    printf 'HAS_AGENT_PID=1\n'
  else
    printf 'HAS_AGENT_PID=0\n'
  fi
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["codex", "--", "--version"])
        .assert()
        .success();

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(
        env_lines.contains("CLAUDINE_PID="),
        "provider must receive CLAUDINE_PID; got: {env_lines}"
    );
    let claudine_pid_value = env_lines
        .lines()
        .find(|l| l.starts_with("CLAUDINE_PID="))
        .and_then(|l| l.strip_prefix("CLAUDINE_PID="))
        .unwrap_or("");
    assert!(
        !claudine_pid_value.is_empty(),
        "CLAUDINE_PID must not be empty; got: {env_lines}"
    );
    assert!(
        claudine_pid_value.parse::<u32>().is_ok(),
        "CLAUDINE_PID must be a valid PID; got: {env_lines}"
    );
    assert!(
        env_lines.contains("HAS_AGENT_PID=0"),
        "provider must not receive AGENT_PID in its environment; got: {env_lines}"
    );
}
