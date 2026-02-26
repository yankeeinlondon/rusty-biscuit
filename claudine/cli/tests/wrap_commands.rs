use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::tempdir;

fn write_executable(path: &Path, content: &str) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, content).unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
    #[cfg(not(unix))]
    {
        fs::write(path, content).unwrap();
    }
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for code in chars.by_ref() {
                    if ('@'..='~').contains(&code) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(ch);
    }

    out
}

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

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    insta::assert_snapshot!(strip_ansi(&stdout));
}

#[cfg(unix)]
#[test]
fn wrapper_preserves_passthrough_args_and_injects_env() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let args_path = workspace.path().join("args.txt");
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
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
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args([
            "codex",
            "--non-interactive",
            "--yolo",
            "--",
            "--json",
            "summarize repo",
        ])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(
        args,
        vec![
            "exec",
            "--json",
            "summarize repo",
            "--dangerously-bypass-approvals-and-sandbox",
        ]
    );

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("AGENT=codex"));
    assert!(env_lines.contains("YOLO=true"));
    assert!(env_lines.contains("INTERACTIVE=false"));
    assert!(env_lines.contains("AGENT_PARAMS=["));
}

#[cfg(unix)]
#[test]
fn wrapper_reports_removed_sensitive_env_names() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env_clear()
        .env("HOME", std::env::var("HOME").unwrap())
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("TERM_WIDTH", "80")
        .env("OPENAI_API_KEY", "keep")
        .env("INTERNAL_TOKEN", "remove")
        .args(["codex", "--include", "OPENAI_API_KEY", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    insta::assert_snapshot!(strip_ansi(&stderr));
}

#[cfg(unix)]
#[test]
fn wrapper_propagates_child_exit_code() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 17
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
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
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["codex", "--json", "--ni", "summarize repo"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(args, vec!["exec", "--json", "summarize repo"]);
}

#[cfg(unix)]
#[test]
fn wrapper_logs_are_written_to_stderr_not_stdout() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
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

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["claude", "--dangerously-skip-permissions", "--ni", "hi"])
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
fn opencode_non_interactive_injects_default_model() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let args_path = workspace.path().join("args.txt");
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf 'MODEL=%s\n' "$MODEL" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["opencode", "--ni", "summarize"])
        .assert()
        .success()
        .stderr(contains("Opencode requires a model be specified"));

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(args.contains(&"run"));
    assert!(args.contains(&"--model"));
    assert!(args.contains(&"minimax/MiniMax-M2.5-highspeed"));
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("MODEL=minimax/MiniMax-M2.5-highspeed"));
}

#[cfg(unix)]
#[test]
fn opencode_non_interactive_model_precedence_uses_env_overrides() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let args_path = workspace.path().join("args.txt");
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf 'MODEL=%s\n' "$MODEL" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .env("MODEL", "from-model")
        .env("OPENCODE_MODEL", "from-opencode")
        .args(["opencode", "--non-interactive", "summarize"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    let model_index = args.iter().position(|arg| *arg == "--model").unwrap();
    assert_eq!(args.get(model_index + 1), Some(&"from-opencode"));
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("MODEL=from-opencode"));
}

#[cfg(unix)]
#[test]
fn opencode_non_interactive_explicit_cli_model_sets_model_env() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let args_path = workspace.path().join("args.txt");
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf 'MODEL=%s\n' "$MODEL" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["opencode", "--ni", "--model", "cli-selected", "summarize"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    assert!(args.lines().any(|line| line == "cli-selected"));
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("MODEL=cli-selected"));
}

#[cfg(unix)]
#[test]
fn opencode_post_summary_messages_are_logged_after_summary_block() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["opencode", "-y", "--ni", "hi"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(plain.starts_with('\n'));
    assert!(plain.contains("\nClaudine"));
    let summary_index = plain.find("Environment Variables:").unwrap();
    let warning_index = plain
        .find("- Warning: --yolo is not supported for 'opencode' and was ignored")
        .unwrap();
    let hint_index = plain.find("Opencode requires a model be specified").unwrap();

    assert!(warning_index > summary_index);
    assert!(hint_index > summary_index);
    let header_line = plain
        .lines()
        .find(|line| line.contains("Claudine"))
        .unwrap();
    assert!(!header_line.contains("YOLO"));
    assert!(plain.contains("YOLO=false"));
    assert!(!plain.contains("`--model`"));
}

// ---------------------------------------------------------------------------
// Provider header shows provider name (review 5.3)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn wrapper_header_shows_provider_name() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
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
fn gemini_wrapper_applies_yolo_as_approval_mode() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["gemini", "--yolo", "--", "-p", "summarize"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(args.contains(&"--approval-mode"));
    assert!(args.contains(&"yolo"));
}

// ---------------------------------------------------------------------------
// Goose wrapper (review 6.2)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn goose_wrapper_yolo_sets_env_var() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let env_path = workspace.path().join("env.txt");

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf 'GOOSE_MODE=%s\n' "$GOOSE_MODE" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["goose", "--yolo", "--ni", "summarize"])
        .assert()
        .success();

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("GOOSE_MODE=auto"));
}

#[cfg(unix)]
#[test]
fn goose_wrapper_non_interactive_prepends_run() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["goose", "--ni", "summarize"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(args.first(), Some(&"run"));
    assert!(args.contains(&"summarize"));
}

// ---------------------------------------------------------------------------
// Kimi wrapper (review 6.2)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn kimi_wrapper_non_interactive_appends_print() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &path_dir.join("kimi"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["kimi", "--ni", "hi"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(args.contains(&"--print"));
    assert!(args.contains(&"hi"));
}

// ---------------------------------------------------------------------------
// Qwen wrapper (review 6.2)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn qwen_wrapper_rejects_direct_approval_mode_yolo() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("qwen"),
        r#"#!/bin/sh
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["qwen", "--approval-mode", "yolo", "--", "-p", "hi"])
        .assert()
        .code(1)
        .stderr(contains("do not pass"))
        .stderr(contains("--approval-mode yolo"))
        .stderr(contains("--yolo"));
}

// ---------------------------------------------------------------------------
// New universal flags (review 3.2)
// ---------------------------------------------------------------------------

#[test]
fn wrapper_help_includes_new_universal_flags() {
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["claude", "--help"])
        .assert()
        .success()
        .stdout(contains("--model <MODEL>"))
        .stdout(contains("--output <FORMAT>"))
        .stdout(contains("--system-prompt"))
        .stdout(contains("--timeout <SECONDS>"))
        .stdout(contains("--dry-run"))
        .stdout(contains("--quiet"))
        .stdout(contains("--sandbox"));
}

#[cfg(unix)]
#[test]
fn wrapper_dry_run_prints_command_and_exits_zero() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
echo "SHOULD NOT RUN"
exit 1
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
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

#[cfg(unix)]
#[test]
fn wrapper_quiet_suppresses_summary() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["codex", "--quiet", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        !stderr.contains("Claudine"),
        "Quiet mode should suppress summary but stderr was: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_universal_model_flag_passes_to_provider() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
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
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["claude", "--model", "claude-sonnet-4-6", "--ni", "hi"])
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

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
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
fn wrapper_timeout_rejects_without_non_interactive() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["codex", "--timeout", "30", "--", "hello"])
        .assert()
        .code(1)
        .stderr(contains("--timeout can only be used with --non-interactive"));
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
