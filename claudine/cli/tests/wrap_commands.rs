use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin_cmd;
use chrono::Local;
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

fn redact_session_id(input: &str) -> String {
    const PREFIX: &str = "CLAUDINE_SESSION_ID=";
    let Some(start) = input.find(PREFIX) else {
        return input.to_string();
    };
    let value_start = start + PREFIX.len();
    // UUID is 36 chars: 8-4-4-4-12
    let value_end = (value_start + 36).min(input.len());
    format!(
        "{}{}<redacted>{}",
        &input[..start],
        PREFIX,
        &input[value_end..]
    )
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

fn today_log_path(home: &Path) -> std::path::PathBuf {
    home.join(".claudine")
        .join("logs")
        .join(format!("{}.jsonl", Local::now().format("%Y-%m-%d")))
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
        .args(["codex", "--yolo", "--", "--json", "summarize repo"])
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
fn codex_wrapper_uses_shadow_home_for_repo_prompt_overlay_without_repo_flag() {
    let workspace = tempdir().unwrap();
    let repo_dir = workspace.path().join("repo");
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let env_path = workspace.path().join("env.txt");

    fs::create_dir_all(&repo_dir).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
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
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(fake_home.join(".codex")).unwrap();

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
        .env("TERM_WIDTH", "80")
        .env("OPENAI_API_KEY", "keep")
        .env("INTERNAL_TOKEN", "remove")
        .args(["codex", "--include", "OPENAI_API_KEY", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    insta::assert_snapshot!(redact_session_id(&strip_ansi(&stderr)));
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
        .args(["codex", "--json", "summarize repo"])
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
        .args(["opencode", "summarize"])
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
        .args(["opencode", "summarize"])
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
        .args(["opencode", "--model", "cli-selected", "summarize"])
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
        .args(["opencode", "-y", "hi"])
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
    let hint_index = plain
        .find("Opencode requires a model be specified")
        .unwrap();

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
        .args(["goose", "--yolo", "summarize"])
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
        .args(["goose", "summarize"])
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
        .args(["kimi", "hi"])
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

    // --quiet shows header but suppresses env details and info
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["codex", "--quiet", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("Claudine"),
        "Quiet mode should show header but stderr was: {stderr}"
    );
    assert!(
        !stderr.contains("Environment Variables"),
        "Quiet mode should suppress env details but stderr was: {stderr}"
    );

    // --silent suppresses everything
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
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
fn wrapper_timeout_rejects_in_interactive_mode() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    // No prompt → interactive by default → --timeout should fail
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["codex", "--timeout", "30"])
        .assert()
        .code(1)
        .stderr(contains(
            "--timeout can only be used in non-interactive mode",
        ));

    // --interactive + --timeout → explicit conflict
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["codex", "--timeout", "30", "-i", "--", "hello"])
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

#[cfg(unix)]
#[test]
fn codex_structured_mode_reconstructs_stdout_and_writes_summary_event() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
LAST=""
while [ $# -gt 0 ]; do
  case "$1" in
    --output-last-message)
      LAST="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
printf '%s\n' '{"type":"thread.started","thread_id":"thread-123"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"item.completed","item":{"id":"msg-1","type":"agent_message","text":"fallback from stream"}}'
printf '%s\n' '{"type":"turn.completed","usage":{"input_tokens":12,"output_tokens":5},"duration_ms":1000,"status":"completed"}'
printf '%s' 'Final assistant response' > "$LAST"
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["codex", "--model", "codex-mini", "summarize repo"])
        .assert()
        .success()
        .stdout("Final assistant response\n");

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        stderr.contains("Codex") && stderr.contains("session ID") && stderr.contains("thread-123")
    );
    assert!(stderr.contains("codex-mini"));
    assert!(stderr.contains("✓ 1.0s"));

    let args = fs::read_to_string(&args_path).unwrap();
    assert!(args.lines().any(|line| line == "--json"));
    assert!(args.lines().any(|line| line == "--output-last-message"));

    let log_path = today_log_path(&fake_home);
    let log_contents = fs::read_to_string(log_path).unwrap();
    assert_eq!(
        log_contents
            .lines()
            .filter(|line| line.contains("\"synthetic_kind\":\"stream_wrapper_summary\""))
            .count(),
        1
    );
    assert!(log_contents.contains("\"provider_summary\":{\"raw_summary\""));
}

#[cfg(unix)]
#[test]
fn structured_verbosity_controls_stream_stderr_lines() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"init","session_id":"gem-1","model":"gemini-2.5-pro"}'
printf '%s\n' 'not-json'
printf '%s\n' '{"type":"message","role":"assistant","content":"Hello"}'
printf '%s\n' '{"type":"error","severity":"warning","message":"Loop detected"}'
printf '%s\n' '{"type":"result","status":"success","stats":{"total_tokens":30,"input_tokens":20,"output_tokens":10,"cached":5,"input":15,"duration_ms":1500,"tool_calls":0}}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "say hi"])
        .assert()
        .success()
        .stdout("Hello\n");
    let default_stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(default_stderr.contains("session ID gem-1"));
    assert!(!default_stderr.contains("Malformed JSON"));
    assert!(default_stderr.contains("Loop detected"));
    assert!(default_stderr.contains("\n\n✓ 1.5s"));
    assert!(default_stderr.contains("20 input tokens"));
    assert!(default_stderr.contains("10 output tokens"));
    assert!(default_stderr.contains("5 cached tokens"));
    assert!(default_stderr.contains("no tool calls"));

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "--quiet", "say hi"])
        .assert()
        .success()
        .stdout("Hello\n");
    let quiet_stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!quiet_stderr.contains("session ID gem-1"));
    assert!(!quiet_stderr.contains("Malformed JSON"));
    assert!(quiet_stderr.contains("Loop detected"));
    assert!(quiet_stderr.contains("\n\n✓ 1.5s"));
    assert!(quiet_stderr.contains("20 input tokens"));
    assert!(quiet_stderr.contains("10 output tokens"));
    assert!(quiet_stderr.contains("5 cached tokens"));
    assert!(quiet_stderr.contains("no tool calls"));

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "--silent", "say hi"])
        .assert()
        .success()
        .stdout("Hello\n");
    let silent_stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!silent_stderr.contains("session ID gem-1"));
    assert!(!silent_stderr.contains("Malformed JSON"));
    assert!(!silent_stderr.contains("Loop detected"));
    assert!(!silent_stderr.contains("✓ 1.5s"));
}

#[cfg(unix)]
#[test]
fn gemini_structured_success_suppresses_provider_stderr_noise() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
cat >&2 <<'EOF'
    at throwErrorIfNotOK (file:///tmp/fake.mjs:1:1)
    at process.processTicksAndRejections (node:internal/process/task_queues:105:5)
EOF
printf '%s\n' '{"type":"init","session_id":"gem-err","model":"gemini-2.5-pro"}'
printf '%s\n' '{"type":"message","role":"assistant","content":"Recovered answer"}'
printf '%s\n' '{"type":"result","status":"success","stats":{"total_tokens":30,"input_tokens":20,"output_tokens":10,"cached":5,"input":15,"duration_ms":1500,"tool_calls":0}}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "--quiet", "say hi"])
        .assert()
        .success()
        .stdout("Recovered answer\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!stderr_plain.contains("throwErrorIfNotOK"));
    assert!(!stderr_plain.contains("processTicksAndRejections"));
    assert!(stderr_plain.contains("20 input tokens"));
    assert!(stderr_plain.contains("10 output tokens"));
    assert!(stderr_plain.contains("5 cached tokens"));
    assert!(stderr_plain.contains("no tool calls"));
}

#[cfg(unix)]
#[test]
fn structured_completion_summary_is_separated_on_stderr() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"init","session_id":"gem-2","model":"gemini-2.5-pro"}'
printf '%s\n' '{"type":"message","role":"assistant","content":"Hello without newline"}'
printf '%s\n' '{"type":"result","status":"success","stats":{"total_tokens":30,"input_tokens":20,"output_tokens":10,"cached":5,"input":15,"duration_ms":1500,"tool_calls":0}}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "--quiet", "say hi"])
        .assert()
        .success()
        .stdout("Hello without newline\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr_plain.contains("\n\n✓ 1.5s"));
    assert!(stderr_plain.contains("20 input tokens"));
    assert!(stderr_plain.contains("10 output tokens"));
    assert!(stderr_plain.contains("5 cached tokens"));
    assert!(stderr_plain.contains("no tool calls"));
}

#[cfg(unix)]
#[test]
fn structured_verbose_summary_restores_rich_prose_fields() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("gemini"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"init","session_id":"gem-3","model":"gemini-2.5-pro"}'
printf '%s\n' '{"type":"message","role":"assistant","content":"Verbose summary"}'
printf '%s\n' '{"type":"tool_use","tool_name":"search","tool_id":"tool-1","parameters":{"query":"sky"}}'
printf '%s\n' '{"type":"tool_result","tool_id":"tool-1","status":"success","output":{"hits":1}}'
printf '%s\n' '{"type":"result","status":"success","cost_usd":0.02,"stats":{"total_tokens":63,"input_tokens":3,"output_tokens":60,"cached":11,"input":3,"duration_ms":4600,"tool_calls":1}}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["gemini", "-v", "say hi"])
        .assert()
        .success()
        .stdout("Verbose summary\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr_plain.contains("4.6s"));
    assert!(stderr_plain.contains("3 input tokens"));
    assert!(stderr_plain.contains("60 output tokens"));
    assert!(stderr_plain.contains("11 cached tokens"));
    assert!(stderr_plain.contains("$0.02 cost basis"));
    assert!(stderr_plain.contains("1 tool call"));
    assert!(stderr_plain.contains("tools used: search"));
    assert!(stderr_plain.contains("model: gemini-2.5-pro"));
    assert!(stderr_plain.contains("stop reason: success"));
}

#[cfg(unix)]
#[test]
fn structured_quiet_verbose_uses_old_verbose_summary_renderer() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"claude-1","model":"claude-sonnet-4"}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"Quiet verbose summary"}]}}'
printf '%s\n' '{"type":"tool_use","name":"read_file","input":{"path":"sky.md"}}'
printf '%s\n' '{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":2,"duration_ms":4600,"usage":{"input_tokens":3,"output_tokens":60,"cache_read_input_tokens":11},"total_cost_usd":0.02}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["claude", "--quiet", "-v", "say hi"])
        .assert()
        .success()
        .stdout("Quiet verbose summary\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!stderr_plain.contains("claude session claude-1"));
    assert!(stderr_plain.contains("4.6s"));
    assert!(stderr_plain.contains("3 input tokens"));
    assert!(stderr_plain.contains("60 output tokens"));
    assert!(stderr_plain.contains("11 cached tokens"));
    assert!(stderr_plain.contains("$0.02 cost basis"));
    assert!(stderr_plain.contains("1 tool call"));
    assert!(stderr_plain.contains("tools used: read_file"));
    assert!(stderr_plain.contains("model: claude-sonnet-4"));
    assert!(stderr_plain.contains("turns: 2"));
    assert!(stderr_plain.contains("stop reason: end_turn"));
}

#[cfg(unix)]
#[test]
fn structured_verbose_summary_reports_no_tool_calls_when_absent() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"claude-2","model":"claude-sonnet-4"}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"No tools here"}],"role":"assistant"}}'
printf '%s\n' '{"type":"result","duration_ms":4600,"total_cost_usd":0.02,"usage":{"input_tokens":3,"output_tokens":60,"cache_read_input_tokens":11}}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["claude", "-v", "say hi"])
        .assert()
        .success()
        .stdout("No tools here\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr_plain.contains("no tool calls"));
}

#[cfg(unix)]
#[test]
fn codex_frontmatter_prompt_validates_agent_file_update() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let doc_path = workspace.path().join("note.md");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    fs::write(
        &doc_path,
        concat!(
            "---\n",
            "prompt: |-\n",
            "  Write a haiku about wrapping text.\n",
            "  Keep the YAML readable.\n",
            "---\n",
            "Old body\n",
        ),
    )
    .unwrap();

    // The fake agent writes directly to the target file (as a real agent would)
    let doc_path_str = doc_path.to_str().unwrap().replace('\'', "'\\''");
    write_executable(
        &path_dir.join("codex"),
        &format!(
            r#"#!/bin/sh
LAST=""
DOC='{doc_path_str}'
while [ $# -gt 0 ]; do
  case "$1" in
    --output-last-message)
      LAST="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
echo "provider stderr noise" >&2
printf '%s\n' '{{"type":"thread.started","thread_id":"thread-compose"}}'
printf '%s\n' '{{"type":"turn.started"}}'
# Agent writes to the target file directly (preserving frontmatter)
printf '%s\n' '---' > "$DOC"
printf '%s\n' 'prompt: |-' >> "$DOC"
printf '%s\n' '  Write a haiku about wrapping text.' >> "$DOC"
printf '%s\n' '  Keep the YAML readable.' >> "$DOC"
printf '%s\n' '---' >> "$DOC"
printf '%s' 'Fresh agent body' >> "$DOC"
printf '%s\n' '{{"type":"turn.completed","usage":{{"input_tokens":8,"output_tokens":6}},"duration_ms":900,"status":"completed"}}'
printf '%s' 'Summary of work done with enough words to require terminal-aware wrapping.' > "$LAST"
"#
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("COLUMNS", "36")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["codex", "--frontmatter-prompt", doc_path.to_str().unwrap()])
        .assert()
        .success();

    let updated = fs::read_to_string(&doc_path).unwrap();
    let stdout_plain = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));

    assert!(updated.contains("last_updated:"));
    assert!(updated.contains("prompt: |-"));
    assert!(updated.contains("  Write a haiku about wrapping text.\n  Keep the YAML readable.\n"));
    assert!(updated.contains("Fresh agent body"));
    assert!(!updated.contains("Old body"));
    assert!(stdout_plain.contains("Summary of work done"));
    assert!(stderr_plain.contains("\n\n✓ Codex agent completed successfully"));
}

#[cfg(unix)]
#[test]
fn codex_frontmatter_prompt_restores_original_frontmatter_layout_after_tamper() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let doc_path = workspace.path().join("note.md");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    fs::write(
        &doc_path,
        concat!(
            "---\n",
            "prompt: |-\n",
            "  Preserve this block scalar.\n",
            "  Do not fold it into one line.\n",
            "---\n",
            "Old body\n",
        ),
    )
    .unwrap();

    let doc_path_str = doc_path.to_str().unwrap().replace('\'', "'\\''");
    write_executable(
        &path_dir.join("codex"),
        &format!(
            r#"#!/bin/sh
LAST=""
DOC='{doc_path_str}'
while [ $# -gt 0 ]; do
  case "$1" in
    --output-last-message)
      LAST="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
printf '%s\n' '{{"type":"thread.started","thread_id":"thread-compose"}}'
printf '%s\n' '{{"type":"turn.started"}}'
# Agent tampers with the frontmatter representation.
printf '%s\n' '---' > "$DOC"
printf '%s\n' 'prompt: Preserve this block scalar. Do not fold it into one line.' >> "$DOC"
printf '%s\n' '---' >> "$DOC"
printf '%s' 'Tampered body replacement' >> "$DOC"
printf '%s\n' '{{"type":"turn.completed","usage":{{"input_tokens":8,"output_tokens":6}},"duration_ms":900,"status":"completed"}}'
printf '%s' 'Agent completed after tampering with frontmatter.' > "$LAST"
"#
        ),
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["codex", "--frontmatter-prompt", doc_path.to_str().unwrap()])
        .assert()
        .success();

    let updated = fs::read_to_string(&doc_path).unwrap();
    assert!(updated.contains("prompt: |-"));
    assert!(updated.contains("  Preserve this block scalar.\n  Do not fold it into one line.\n"));
    assert!(updated.contains("Tampered body replacement"));
    assert!(updated.contains("last_updated:"));
}

#[cfg(unix)]
#[test]
fn codex_frontmatter_prompt_does_not_overwrite_file_on_failure() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let doc_path = workspace.path().join("note.md");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    fs::write(&doc_path, "---\nprompt: Write a haiku\n---\nOld body\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
LAST=""
while [ $# -gt 0 ]; do
  case "$1" in
    --output-last-message)
      LAST="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
printf '%s\n' '{"type":"thread.started","thread_id":"thread-compose"}'
printf '%s\n' '{"type":"error","error_type":"test_failure","error_message":"structured failure"}'
printf '%s' 'Should not be applied' > "$LAST"
exit 9
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["codex", "--frontmatter-prompt", doc_path.to_str().unwrap()])
        .assert()
        .code(9)
        .stderr(contains("structured failure"));

    let updated = fs::read_to_string(&doc_path).unwrap();
    assert!(updated.contains("Old body"));
    assert!(!updated.contains("Should not be applied"));
}

#[cfg(unix)]
#[test]
fn codex_prompt_file_redirects_after_precheck_failure() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let prompt_path = workspace.path().join("prompt.md");
    let redirected_path = workspace.path().join("redirected.md");
    let run_count_path = workspace.path().join("runs.txt");
    let prompt_log_path = workspace.path().join("prompt-log.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    fs::write(
        &prompt_path,
        format!(
            concat!(
                "---\n",
                "pre_checks:\n",
                "  file_exists: ./missing.txt\n",
                "handle_file_exists:\n",
                "  redirect:\n",
                "    file: {}\n",
                "---\n",
                "Initial prompt body.\n",
            ),
            redirected_path.display()
        ),
    )
    .unwrap();
    fs::write(&redirected_path, "Redirected prompt body.\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
COUNT=0
if [ -f "$CLAUDINE_RUN_COUNT_FILE" ]; then
  COUNT=$(/bin/cat "$CLAUDINE_RUN_COUNT_FILE")
fi
COUNT=$((COUNT + 1))
printf '%s' "$COUNT" > "$CLAUDINE_RUN_COUNT_FILE"
PROMPT=$(/bin/cat)
printf '%s' "$PROMPT" > "$CLAUDINE_PROMPT_LOG"
printf '%s\n' 'redirected success'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("CLAUDINE_RUN_COUNT_FILE", &run_count_path)
        .env("CLAUDINE_PROMPT_LOG", &prompt_log_path)
        .args([
            "codex",
            "--prompt-file",
            prompt_path.to_str().unwrap(),
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout("redirected success\n");

    assert_eq!(fs::read_to_string(&run_count_path).unwrap(), "1");
    assert_eq!(
        fs::read_to_string(&prompt_log_path).unwrap(),
        "Redirected prompt body."
    );

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr.contains("redirected"));
}

#[cfg(unix)]
#[test]
fn codex_prompt_file_retry_applies_prompt_suffix_and_set_overlay() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let prompt_path = workspace.path().join("prompt.md");
    let run_count_path = workspace.path().join("runs.txt");
    let attempt_log_path = workspace.path().join("attempt-log.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    fs::write(
        &prompt_path,
        concat!(
            "---\n",
            "mode: first\n",
            "post_checks:\n",
            "  response_includes: FIXED\n",
            "handle_response_includes:\n",
            "  retry:\n",
            "    prompt: Add the word FIXED.\n",
            "    retries: 2\n",
            "    set:\n",
            "      mode: second\n",
            "---\n",
            "Base prompt body.\n",
        ),
    )
    .unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
COUNT=0
if [ -f "$CLAUDINE_RUN_COUNT_FILE" ]; then
  COUNT=$(/bin/cat "$CLAUDINE_RUN_COUNT_FILE")
fi
COUNT=$((COUNT + 1))
printf '%s' "$COUNT" > "$CLAUDINE_RUN_COUNT_FILE"
PROMPT=$(/bin/cat)
{
  printf 'ATTEMPT=%s\n' "$COUNT"
  printf 'MODE=%s\n' "$MODE"
  printf 'PROMPT=%s\n' "$PROMPT"
  printf '%s\n' '--'
} >> "$CLAUDINE_ATTEMPT_LOG"
if [ "$COUNT" -eq 1 ]; then
  printf '%s\n' 'missing keyword'
else
  printf '%s\n' 'Now FIXED'
fi
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("CLAUDINE_RUN_COUNT_FILE", &run_count_path)
        .env("CLAUDINE_ATTEMPT_LOG", &attempt_log_path)
        .args([
            "codex",
            "--prompt-file",
            prompt_path.to_str().unwrap(),
            "--output",
            "text",
        ])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&run_count_path).unwrap(), "2");
    let attempt_log = fs::read_to_string(&attempt_log_path).unwrap();
    assert!(attempt_log.contains("ATTEMPT=1\nMODE=first"));
    assert!(attempt_log.contains("ATTEMPT=2\nMODE=second"));
    assert!(attempt_log.contains("PROMPT=Base prompt body."));
    assert!(attempt_log.contains("Add the word FIXED."));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("missing keyword"));
    assert!(stdout.contains("Now FIXED"));

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr.contains("retry"));
}

#[cfg(unix)]
#[test]
fn claude_prompt_file_resume_uses_provider_resume_args() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let prompt_path = workspace.path().join("prompt.md");
    let run_count_path = workspace.path().join("runs.txt");
    let invocation_log_path = workspace.path().join("invocations.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    fs::write(
        &prompt_path,
        concat!(
            "---\n",
            "post_checks:\n",
            "  response_includes: continued\n",
            "handle_response_includes:\n",
            "  resume:\n",
            "    prompt: Continue and include continued.\n",
            "    retries: 2\n",
            "---\n",
            "Initial prompt body.\n",
        ),
    )
    .unwrap();

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
COUNT=0
if [ -f "$CLAUDINE_RUN_COUNT_FILE" ]; then
  COUNT=$(/bin/cat "$CLAUDINE_RUN_COUNT_FILE")
fi
COUNT=$((COUNT + 1))
printf '%s' "$COUNT" > "$CLAUDINE_RUN_COUNT_FILE"
INPUT=$(/bin/cat)
{
  printf 'INVOCATION=%s\n' "$COUNT"
  for arg in "$@"; do
    printf 'ARG=%s\n' "$arg"
  done
  printf 'STDIN=%s\n' "$INPUT"
  printf '%s\n' '--'
} >> "$CLAUDINE_INVOCATION_LOG"
printf '%s\n' '{"type":"system","subtype":"init","session_id":"resume-session","model":"claude-sonnet-4"}'
if [ "$1" = "-r" ]; then
  printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"continued answer"}],"role":"assistant"}}'
else
  printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"initial answer"}],"role":"assistant"}}'
fi
printf '%s\n' '{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":1,"duration_ms":40,"usage":{"input_tokens":3,"output_tokens":4},"total_cost_usd":0.0}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("CLAUDINE_RUN_COUNT_FILE", &run_count_path)
        .env("CLAUDINE_INVOCATION_LOG", &invocation_log_path)
        .args(["claude", "--prompt-file", prompt_path.to_str().unwrap()])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&run_count_path).unwrap(), "2");
    let invocation_log = fs::read_to_string(&invocation_log_path).unwrap();
    assert!(invocation_log.contains("INVOCATION=1"));
    assert!(invocation_log.contains("STDIN=Initial prompt body."));
    assert!(invocation_log.contains("INVOCATION=2\nARG=-r\nARG=resume-session\nARG=--print"));
    assert!(invocation_log.contains("STDIN=Continue and include continued."));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("initial answer"));
    assert!(stdout.contains("continued answer"));
}

#[cfg(unix)]
#[test]
fn codex_compose_response_validation_uses_captured_legacy_output() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let doc_path = workspace.path().join("compose.md");
    let prompt_log_path = workspace.path().join("compose-prompt.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    fs::write(
        &doc_path,
        concat!(
            "---\n",
            "post_checks:\n",
            "  response_includes: legacy needle\n",
            "---\n",
            "Compose prompt body.\n",
        ),
    )
    .unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
PROMPT=$(/bin/cat)
printf '%s' "$PROMPT" > "$CLAUDINE_PROMPT_LOG"
printf '%s\n' 'legacy needle from raw stdout'
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("CLAUDINE_PROMPT_LOG", &prompt_log_path)
        .args([
            "codex",
            "--compose",
            doc_path.to_str().unwrap(),
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout("legacy needle from raw stdout\n");

    assert!(
        fs::read_to_string(&prompt_log_path)
            .unwrap()
            .contains("Compose prompt body.")
    );
}

#[cfg(unix)]
#[test]
fn codex_frontmatter_prompt_retries_inline_recovery() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    let doc_path = workspace.path().join("note.md");
    let run_count_path = workspace.path().join("runs.txt");
    let prompt_log_path = workspace.path().join("prompt-log.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    fs::write(
        &doc_path,
        concat!(
            "---\n",
            "prompt: |-\n",
            "  Replace the body.\n",
            "post_checks:\n",
            "  response_includes: DONE\n",
            "handle_response_includes:\n",
            "  retry:\n",
            "    prompt: Make sure the response says DONE.\n",
            "    retries: 2\n",
            "---\n",
            "Old body\n",
        ),
    )
    .unwrap();

    let doc_path_str = doc_path.to_str().unwrap().replace('\'', "'\\''");
    write_executable(
        &path_dir.join("codex"),
        &format!(
            r#"#!/bin/sh
COUNT=0
if [ -f "$CLAUDINE_RUN_COUNT_FILE" ]; then
  COUNT=$(/bin/cat "$CLAUDINE_RUN_COUNT_FILE")
fi
COUNT=$((COUNT + 1))
printf '%s' "$COUNT" > "$CLAUDINE_RUN_COUNT_FILE"
PROMPT=$(/bin/cat)
{{
  printf 'ATTEMPT=%s\n' "$COUNT"
  printf 'PROMPT=%s\n' "$PROMPT"
  printf '%s\n' '--'
}} >> "$CLAUDINE_PROMPT_LOG"
DOC='{doc_path_str}'
if [ "$COUNT" -eq 1 ]; then
  printf '%s\n' '---' > "$DOC"
  printf '%s\n' 'prompt: |-' >> "$DOC"
  printf '%s\n' '  Replace the body.' >> "$DOC"
  printf '%s\n' 'post_checks:' >> "$DOC"
  printf '%s\n' '  response_includes: DONE' >> "$DOC"
  printf '%s\n' 'handle_response_includes:' >> "$DOC"
  printf '%s\n' '  retry:' >> "$DOC"
  printf '%s\n' '    prompt: Make sure the response says DONE.' >> "$DOC"
  printf '%s\n' '    retries: 2' >> "$DOC"
  printf '%s\n' '---' >> "$DOC"
  printf '%s' 'First attempt body' >> "$DOC"
  printf '%s\n' 'not yet'
else
  printf '%s\n' '---' > "$DOC"
  printf '%s\n' 'prompt: |-' >> "$DOC"
  printf '%s\n' '  Replace the body.' >> "$DOC"
  printf '%s\n' 'post_checks:' >> "$DOC"
  printf '%s\n' '  response_includes: DONE' >> "$DOC"
  printf '%s\n' 'handle_response_includes:' >> "$DOC"
  printf '%s\n' '  retry:' >> "$DOC"
  printf '%s\n' '    prompt: Make sure the response says DONE.' >> "$DOC"
  printf '%s\n' '    retries: 2' >> "$DOC"
  printf '%s\n' '---' >> "$DOC"
  printf '%s' 'DONE body' >> "$DOC"
  printf '%s\n' 'DONE'
fi
"#
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("CLAUDINE_RUN_COUNT_FILE", &run_count_path)
        .env("CLAUDINE_PROMPT_LOG", &prompt_log_path)
        .args([
            "codex",
            "--frontmatter-prompt",
            doc_path.to_str().unwrap(),
            "--output",
            "text",
        ])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&run_count_path).unwrap(), "2");
    let prompt_log = fs::read_to_string(&prompt_log_path).unwrap();
    assert!(prompt_log.contains("ATTEMPT=2"));
    assert!(prompt_log.contains("Make sure the response says DONE."));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("not yet"));
    assert!(stdout.contains("DONE"));

    let updated = fs::read_to_string(&doc_path).unwrap();
    assert!(updated.contains("DONE body"));
    assert!(updated.contains("last_updated:"));
}
