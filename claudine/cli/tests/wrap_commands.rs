use assert_cmd::cargo::cargo_bin_cmd;
use chrono::Local;
use claudine::mcp::types::{
    McpCatalog, McpDefaults, McpProviderState, McpServer, McpServerMetadata, McpTransport,
};
use predicates::str::contains;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
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

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
}

fn make_server(id: &str) -> McpServer {
    McpServer {
        id: id.into(),
        aliases: Vec::new(),
        transport: McpTransport::Stdio,
        command: Some("npx".into()),
        args: vec!["-y".into(), format!("@test/{id}")],
        cwd: None,
        env: HashMap::new(),
        url: None,
        headers: HashMap::new(),
        enabled_tools: Vec::new(),
        disabled_tools: Vec::new(),
        required: false,
        metadata: McpServerMetadata {
            description: None,
            created_from: Some("codex:user".into()),
            fingerprint: format!("fp-{id}"),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        provider_overrides: HashMap::new(),
    }
}

fn seed_catalog(home: &Path, servers: &[McpServer]) {
    write_json(
        &home.join(".claudine/mcp/catalog.json"),
        &McpCatalog {
            version: 1,
            servers: servers
                .iter()
                .cloned()
                .map(|server| (server.id.clone(), server))
                .collect(),
        },
    );
}

fn seed_defaults(home: &Path, ids: &[&str]) {
    write_json(
        &home.join(".claudine/mcp/defaults.json"),
        &McpDefaults {
            version: 1,
            defaults: ids.iter().map(|id| (*id).to_string()).collect(),
        },
    );
}

fn seed_empty_provider_state(home: &Path) {
    write_json(
        &home.join(".claudine/mcp/provider-state.json"),
        &McpProviderState {
            version: 1,
            providers: HashMap::new(),
            repos: HashMap::new(),
        },
    );
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

// ===========================================================================
// Composition tests
// ===========================================================================

#[test]
fn compose_requires_file_argument() {
    // When no file is provided, clap shows usage which includes the expected flags
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose"])
        .assert()
        .code(2);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("FILE"), "usage should show FILE argument");
}

#[test]
fn inline_compose_requires_file_argument() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["inline-compose"])
        .assert()
        .code(2);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("FILE"), "usage should show FILE argument");
}

#[test]
fn compose_rejects_nonexistent_file() {
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose", "/nonexistent/path/to/file.md"])
        .assert()
        .code(1);
}

#[test]
fn compose_rejects_non_markdown_file() {
    let workspace = tempdir().unwrap();
    let txt_file = workspace.path().join("file.txt");
    fs::write(&txt_file, "hello").unwrap();

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose", txt_file.to_str().unwrap()])
        .assert()
        .code(1);
}

#[test]
fn inline_compose_rejects_missing_prompt_property() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: No prompt\n---\nBody\n").unwrap();

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["inline-compose", md_file.to_str().unwrap()])
        .assert()
        .code(1);
}

#[cfg(unix)]
#[test]
fn explicit_provider_flag_bypasses_chooser() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );
    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
exit 99
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args([
            "compose",
            "--codex",
            "--exclude",
            "codex",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("explicit"),
        "should indicate explicit provider selection but stderr was: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_uses_wrapper_grade_execution() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello compose\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf 'AGENT=%s\n' "$AGENT" >&2
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .success();

    // Wrapper-grade execution injects AGENT env
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("AGENT=codex"),
        "compose should inject AGENT env via wrapper pipeline; stderr was: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_restores_repo_harness_for_plain_prompts() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let marker_path = workspace.path().join("provider-ran.txt");
    fs::create_dir_all(&path_dir).unwrap();

    fs::write(
        workspace.path().join("CLAUDE.md"),
        "---\npre_checks:\n  file_exists: \"missing.txt\"\n---\nRepo harness\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf 'ran\n' > "$CLAUDINE_MARKER_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_MARKER_FILE", &marker_path)
        .current_dir(workspace.path())
        .args(["claude", "--", "summarize the repo"])
        .assert()
        .code(1)
        .stderr(contains("pre-check validation failed"));

    assert!(
        !marker_path.exists(),
        "plain wrapper harness should block launch before the provider runs"
    );
}

#[cfg(unix)]
#[test]
fn compose_harness_pre_check_blocks_provider_launch() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let marker_path = workspace.path().join("provider-ran.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("compose.md");
    fs::write(
        &md_file,
        "---\npre_checks:\n  file_exists: \"missing.txt\"\n---\nFinish the brief.\n",
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
        .env("PATH", &path_dir)
        .env("CLAUDINE_MARKER_FILE", &marker_path)
        .current_dir(workspace.path())
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(contains("pre-check validation failed"));

    assert!(
        !marker_path.exists(),
        "compose harness should block launch before the provider runs"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_rejects_empty_captured_output() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate content\n---\nOriginal body\n",
    )
    .unwrap();

    // Agent that produces no replacement body.
    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    {
        let assert = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .env("PATH", &path_dir)
            .args(["inline-compose", "--codex", md_file.to_str().unwrap()])
            .assert();

        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        let plain = strip_ansi(&stderr);
        assert!(
            plain.contains("valid replacement body") || plain.contains("empty response"),
            "should report invalid captured output; stderr was: {plain}"
        );
    }
}

#[cfg(unix)]
#[test]
fn inline_compose_harness_retries_after_post_check_failure() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let count_path = workspace.path().join("attempt-count.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Rewrite the body\npost_checks:\n  response_includes: \"Updated brief\"\nhandle_response_includes:\n  retry:\n    prompt: \"Your final response must explicitly say 'Updated brief'.\"\n---\nOriginal body\n",
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
  printf 'Replacement body without the required phrase\n'
else
  printf 'Updated brief\n\nFinal replacement body\n'
fi
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let attempts = fs::read_to_string(&count_path).unwrap();
    assert_eq!(attempts.trim(), "2", "expected exactly one retry");

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("Updated brief"),
        "inline retry should apply the successful replacement body; file: {final_content}"
    );
    assert!(
        final_content.contains("Final replacement body"),
        "inline retry should preserve the successful second attempt body; file: {final_content}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_preserves_frontmatter() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    let original = "---\nprompt: Generate content\nlast_updated: 2026-01-01\n---\nOriginal body\n";
    fs::write(&md_file, original).unwrap();

    // Agent returns frontmatter + body on stdout. Claudine should strip the
    // accidental frontmatter wrapper and preserve the original source frontmatter.
    let provider_output =
        "---\nprompt: CHANGED\nlast_updated: 2099-01-01\n---\nNew body from agent\n";
    let escaped = provider_output.replace('\'', "'\\''");

    write_executable(
        &path_dir.join("goose"),
        &format!("#!/bin/sh\nprintf '%s' '{}'\nexit 0\n", escaped),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert();

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("prompt: Generate content"),
        "original frontmatter prompt should be preserved; file: {final_content}"
    );

    let today = Local::now().format("%Y-%m-%d").to_string();
    assert!(
        final_content.contains(&format!("last_updated: {today}")),
        "last_updated should be today; file: {final_content}"
    );

    // Body should be updated
    assert!(
        final_content.contains("New body from agent"),
        "body should be from agent; file: {final_content}"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Preserved original frontmatter"),
        "should report Claudine-managed frontmatter preservation; stderr was: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_no_overwrite_on_failure() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    let original = "---\nprompt: Generate content\n---\nOriginal body\n";
    fs::write(&md_file, original).unwrap();

    // Agent that exits with error and does not modify the file
    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 1
"#,
    );

    let _assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["inline-compose", "--codex", md_file.to_str().unwrap()])
        .assert();

    // File should not have been modified
    let final_content = fs::read_to_string(&md_file).unwrap();
    assert_eq!(
        final_content, original,
        "file should not be modified on agent failure"
    );
}

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

    cargo_bin_cmd!("claudine")
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
            "inline-compose with --interactive is not supported",
        ));
}

#[cfg(unix)]
#[test]
fn inline_compose_interactive_codex_uses_captured_last_message() {
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
        &path_dir.join("codex"),
        r#"#!/bin/sh
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output-last-message" ]; then
    shift
    printf 'Interactive body from codex\n' > "$1"
    exit 0
  fi
  shift
done
exit 1
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
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

#[cfg(unix)]
#[test]
fn compose_interactive_claude_seeds_prompt_as_positional_arg() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello Claude\n").unwrap();

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
        .args([
            "compose",
            "--interactive",
            "--claude",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    assert!(
        args.lines().any(|line| line == "Hello Claude"),
        "interactive compose should pass Claude the composed prompt as a positional arg; args: {args}"
    );
}

#[cfg(unix)]
#[test]
fn compose_interactive_kimi_seeds_prompt_with_prompt_flag() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello Kimi\n").unwrap();

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
        .args([
            "compose",
            "--interactive",
            "--kimi",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();
    assert!(
        collected
            .windows(2)
            .any(|window| window == ["--prompt", "Hello Kimi"]),
        "interactive compose should seed Kimi via --prompt; args: {args}"
    );
}

#[cfg(unix)]
#[test]
fn compose_supports_mcp_runtime_and_tag_cleanup() {
    let workspace = tempdir().unwrap();
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    let stdin_path = workspace.path().join("stdin.txt");
    let env_path = workspace.path().join("env.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(home.join(".codex")).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: test\n---\nUse #calendar for this task\n",
    )
    .unwrap();

    seed_catalog(&home, &[make_server("calendar")]);
    seed_defaults(&home, &["calendar"]);
    seed_empty_provider_state(&home);

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
cat > "$CLAUDINE_STDIN_FILE"
{
  printf 'HOME=%s\n' "$HOME"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["compose", "--codex", "--mcp", md_file.to_str().unwrap()])
        .assert()
        .success();

    let prompt = fs::read_to_string(&stdin_path).unwrap();
    assert!(
        !prompt.contains("#calendar"),
        "MCP tags should be stripped before prompt delivery; prompt: {prompt}"
    );

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(
        env_lines.contains(&format!("HOME={}", home.join(".claudine").display())),
        "runtime MCP for codex should use a shadow HOME; env: {env_lines}"
    );
}

#[cfg(unix)]
#[test]
fn no_cross_provider_retry_after_launch() {
    // Verifies that after a provider is launched and fails, Claudine
    // does NOT automatically retry with another provider. The exit code
    // from the single provider invocation is returned directly.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    // Provider that exits with error code 42
    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 42
"#,
    );

    // Also install a "claude" that succeeds -- if retry happened, we'd see code 0
    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
exit 0
"#,
    );

    // Explicitly select codex. It exits 42. No fallback to claude.
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .code(42);
}

#[test]
fn old_compose_inline_command_is_unknown() {
    // Verify that the old `compose-inline` command no longer exists
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose-inline", "file.md"])
        .assert()
        .code(2); // clap returns 2 for unrecognized subcommands
}
