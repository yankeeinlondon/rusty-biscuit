use assert_cmd::cargo::cargo_bin_cmd;
use chrono::Local;
use claudine::mcp::types::{
    McpCatalog, McpDefaults, McpProviderState, McpServer, McpServerMetadata, McpTransport,
};
use predicates::str::contains;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
mod common;
use common::{augmented_path, init_git_repo, strip_ansi, write, write_executable, write_json};

fn create_claudine_monorepo(workspace: &Path) -> Option<(PathBuf, PathBuf, PathBuf)> {
    let repo_root = workspace.join("repo");
    let launch_dir = repo_root.join("claudine/cli");
    let lib_dir = repo_root.join("claudine/lib");
    let bin_dir = repo_root.join("bin");

    fs::create_dir_all(launch_dir.join("src")).unwrap();
    fs::create_dir_all(lib_dir.join("src")).unwrap();
    fs::create_dir_all(&bin_dir).unwrap();

    write(
        &repo_root.join("Cargo.toml"),
        r#"[workspace]
resolver = "2"
members = ["claudine/lib", "claudine/cli"]
"#,
    );
    write(
        &lib_dir.join("Cargo.toml"),
        r#"[package]
name = "claudine"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(&lib_dir.join("src/lib.rs"), "");
    write(
        &launch_dir.join("Cargo.toml"),
        r#"[package]
name = "claudine-cli"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(&launch_dir.join("src/main.rs"), "fn main() {}\n");

    if !init_git_repo(&repo_root) {
        return None;
    }

    Some((repo_root, launch_dir, bin_dir))
}

fn redact_session_id(input: &str) -> String {
    let result = redact_temp_home(input);
    const PREFIX: &str = "CLAUDINE_SESSION_ID=";
    let Some(start) = result.find(PREFIX) else {
        return result;
    };
    let value_start = start + PREFIX.len();
    let value_end = (value_start + 36).min(result.len());
    format!(
        "{}{}<redacted>{}",
        &result[..start],
        PREFIX,
        &result[value_end..]
    )
}

fn redact_temp_home(input: &str) -> String {
    const MARKER: &str = "HOME=/var/folders/";
    let Some(start) = input.find(MARKER) else {
        return input.to_string();
    };
    let value_start = start + 5;
    let after = &input[value_start..];
    let end = after.find('\n').unwrap_or(after.len());
    format!("{}HOME=<redacted>{}", &input[..start], &after[end..])
}

fn today_log_path(home: &Path) -> std::path::PathBuf {
    home.join(".claudine")
        .join("logs")
        .join(format!("{}.jsonl", Local::now().format("%Y-%m-%d")))
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
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
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
            "--dangerously-bypass-approvals-and-sandbox",
        ]
    );

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
        .env("TERM", "dumb")
        .env("TERM_WIDTH", "80")
        .env("OPENAI_API_KEY", "keep")
        .env("INTERNAL_TOKEN", "remove")
        .args(["codex", "--include", "OPENAI_API_KEY", "--", "--version"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    insta::assert_snapshot!(redact_temp_home(&redact_session_id(&strip_ansi(&stderr))));
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
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
        .args(["codex", "--json", "summarize repo"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(args, vec!["exec", "--json"]);

    let stdin = fs::read_to_string(&stdin_path).unwrap();
    assert_eq!(stdin, "summarize repo");
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
fn opencode_non_interactive_requires_model_when_missing() {
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
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["opencode", "summarize"])
        .assert()
        .failure()
        .stderr(contains("No model specified!"))
        .stderr(contains("OPENCODE_MODEL"));

    assert!(
        !args_path.exists(),
        "child process should not launch when no non-interactive model is available"
    );
    assert!(
        !env_path.exists(),
        "child process should not launch when no non-interactive model is available"
    );
}

#[cfg(unix)]
#[test]
fn opencode_launches_child_from_repo_root() {
    let workspace = tempdir().unwrap();
    let Some((repo_root, launch_dir, bin_dir)) = create_claudine_monorepo(workspace.path()) else {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    };
    let pwd_path = workspace.path().join("pwd.txt");
    let env_path = workspace.path().join("env.txt");
    let args_path = workspace.path().join("args.txt");

    write_executable(
        &bin_dir.join("opencode"),
        r#"#!/bin/sh
pwd > "$CLAUDINE_PWD_FILE"
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
{
  printf 'PACKAGE=%s\n' "$PACKAGE"
  printf 'PACKAGE_AREA=%s\n' "$PACKAGE_AREA"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .current_dir(&launch_dir)
        .env("NO_COLOR", "1")
        .env("OPENCODE_MODEL", "test-model")
        .env("PATH", &bin_dir)
        .env("CLAUDINE_PWD_FILE", &pwd_path)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["opencode", "summarize"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&pwd_path).unwrap().trim(),
        repo_root.canonicalize().unwrap().display().to_string()
    );
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("PACKAGE=claudine-cli"));
    assert!(env_lines.contains("PACKAGE_AREA=claudine"));
    let args = fs::read_to_string(&args_path).unwrap();
    assert!(args.lines().any(|line| line == "summarize"));
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

    // Interactive mode (-i) still warns that OpenCode doesn't support --yolo
    // in interactive sessions (refined copy). This keeps the deferred-warning
    // ordering test meaningful after non-interactive forwards the flag.
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "-i", "-y"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(plain.starts_with('\n'));
    assert!(plain.contains("\nClaudine"));
    let summary_index = plain.find("Environment Variables:").unwrap();
    // Prose `<i>` tags render as ANSI italics which NO_COLOR strips, leaving
    // the word without markup in the plain-text output.
    let warning_needle = "--yolo mode is not supported in OpenCode interactive sessions";
    let warning_index = plain
        .find(warning_needle)
        .unwrap_or_else(|| panic!("expected refined interactive warning in stderr; got:\n{plain}"));

    assert!(warning_index > summary_index);
    // In interactive mode YOLO stays marked unsupported for OpenCode, so the
    // header should not advertise a YOLO badge.
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
    let stdin_path = workspace.path().join("stdin.txt");

    write_executable(
        &path_dir.join("kimi"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
/bin/cat > "$CLAUDINE_STDIN_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
        .args(["kimi", "hi"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert!(args.contains(&"--print"));
    let stdin = fs::read_to_string(&stdin_path).unwrap();
    assert_eq!(stdin, "hi");
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
fn wrapper_perf_emits_report_to_stderr_only() {
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
        .args(["codex", "--perf", "--", "--version"])
        .assert()
        .success()
        .stdout("");

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("CLI Overhead"),
        "stderr should contain CLI Overhead section; got: {plain}"
    );
    assert!(
        plain.contains("Agent Execution"),
        "stderr should contain Agent Execution section; got: {plain}"
    );
    assert!(
        plain.contains("launches:"),
        "stderr should show launch count; got: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_dry_run_perf_emits_report_with_skipped_note() {
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
        .args(["codex", "--dry-run", "--perf", "--", "--version"])
        .assert()
        .success()
        .stdout("");

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("CLI Overhead"),
        "stderr should contain CLI Overhead section; got: {plain}"
    );
    assert!(
        plain.contains("dry run") || plain.contains("skipped"),
        "stderr should mention dry run; got: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_quiet_suppresses_summary() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let system_prompt = workspace.path().join("system-prompt.md");
    fs::create_dir_all(&path_dir).unwrap();
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
        stderr_plain.contains("System Prompt(appended):"),
        "Quiet mode should still show the system prompt when set but stderr was: {stderr}"
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
fn wrapper_missing_explicit_system_prompt_fails_visibly() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let missing_prompt = workspace.path().join("missing-prompt.md");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
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
        .stderr(contains("missing-prompt.md"));
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
    // Agent session ID is always emitted regardless of --quiet/--silent
    assert!(quiet_stderr.contains("session ID gem-1"));
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
    // Agent session ID is always emitted regardless of --quiet/--silent
    assert!(silent_stderr.contains("session ID gem-1"));
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
fn compose_requires_positional_arg() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose"])
        .assert()
        .code(2);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("ARG"), "usage should show ARG positional");
}

#[test]
fn inline_compose_requires_positional_arg() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["inline-compose"])
        .assert()
        .code(2);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("ARG"), "usage should show ARG positional");
}

#[test]
fn compose_missing_file_with_setter_only() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose", "key=val"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("missing file reference"),
        "expected missing-file error, got: {plain}"
    );
}

#[test]
fn compose_empty_key_setter_errors() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose", "=foo"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("setter key must not be empty"),
        "expected empty-key setter error, got: {plain}"
    );
}

#[test]
fn compose_multiple_file_candidates_errors() {
    let workspace = tempdir().unwrap();
    let a = workspace.path().join("a.md");
    let b = workspace.path().join("b.md");
    fs::write(&a, "---\n---\nbody\n").unwrap();
    fs::write(&b, "---\n---\nbody\n").unwrap();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose", a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("multiple"),
        "expected multiple-file error, got: {plain}"
    );
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

#[cfg(unix)]
#[test]
fn compose_missing_explicit_system_prompt_fails_visibly() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let md_file = workspace.path().join("prompt.md");
    let missing_prompt = workspace.path().join("missing-system-prompt.md");
    fs::create_dir_all(&path_dir).unwrap();
    fs::write(&md_file, "---\ntitle: test\n---\nHello compose\n").unwrap();

    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "compose",
            "--codex",
            "--append-system-prompt",
            missing_prompt.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(contains("missing-system-prompt.md"));
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
        plain.contains("Codex") && plain.contains("Compose"),
        "should show Codex provider in the Claudine header but stderr was: {plain}"
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
fn compose_preflight_error_includes_source_provenance() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    // Markdown with a ::shell directive that is NOT whitelisted.
    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: provenance test\n---\n::shell curl https://example.com\n",
    )
    .unwrap();

    // Provider binary (should never be reached — preflight should abort first).
    write_executable(
        &path_dir.join("codex"),
        "#!/bin/sh\necho 'ERROR: provider should not run' >&2\nexit 99\n",
    );

    // Run without --interactive so preflight has no approval handler →
    // the non-whitelisted command triggers a clear error with provenance.
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    // Error message should mention the source file name (provenance).
    assert!(
        plain.contains("template.md"),
        "preflight error should include the source file name for provenance; stderr was:\n{plain}"
    );
    // Error message should mention the denied command.
    assert!(
        plain.contains("curl"),
        "preflight error should identify the denied command; stderr was:\n{plain}"
    );
    // Provider should NOT have run.
    assert!(
        !plain.contains("ERROR: provider should not run"),
        "provider binary should not execute when preflight fails; stderr was:\n{plain}"
    );
}

/// Proves `--interactive` flag is wired up for compose preflight with a
/// whitelisted command.  This covers the "interactive + whitelisted = success"
/// path.  Full interactive-prompt coverage (PTY + answer prompt + assert
/// provenance in the displayed prompt) remains a future improvement — the
/// library-level `interactive_handler_is_invoked_for_non_whitelisted_command`
/// test in `preflight.rs` covers the handler invocation path.
#[cfg(unix)]
#[test]
fn compose_interactive_preflight_with_whitelisted_command() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: interactive test\n---\n::shell echo whitelisted\n",
    )
    .unwrap();

    // Whitelist "echo" so the ::shell directive passes preflight.
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();

    // Also create .git so the whitelist is found (policy root = git root).
    fs::create_dir_all(workspace.path().join(".git")).unwrap();

    write_executable(
        &path_dir.join("codex"),
        "#!/bin/sh\ncat > /dev/null\necho 'provider-launched' >&2\nexit 0\n",
    );

    // Include system dirs so shell expansion can find `echo`.
    let full_path = format!("{}:/usr/bin:/bin", path_dir.display());

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &full_path)
        .args([
            "compose",
            "--interactive",
            "--codex",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("provider-launched"),
        "provider should run after --interactive preflight passes; stderr was:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_skips_shell_hidden_by_false_block() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    // The ::shell is inside a ::block when="false" — Darkmatter's composition
    // excludes it, so preflight never discovers the un-whitelisted command.
    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: false block test\n---\n\
         Safe content here.\n\n\
         ::block when=\"false\"\n\
         ::shell curl https://evil.example.com\n\
         ::end-block\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("codex"),
        "#!/bin/sh\necho 'provider-launched' >&2\nexit 0\n",
    );

    // No whitelist for curl — if it were discovered, preflight would fail.
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("provider-launched"),
        "provider should launch — ::shell inside ::block when=\"false\" must be hidden; stderr was:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_shell_preflight_passes_with_whitelisted_commands() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let marker_path = workspace.path().join("provider-ran.txt");
    fs::create_dir_all(&path_dir).unwrap();

    // Markdown with a whitelisted ::shell command AND a harness shell pre-check.
    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: full flow test\npre_checks:\n  shell_command: \"echo precheck-ok\"\n---\n\
         ::shell echo composed-ok\n",
    )
    .unwrap();

    // Whitelist "echo" so both the ::shell directive and the harness pre-check pass.
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();
    fs::create_dir_all(workspace.path().join(".git")).unwrap();

    write_executable(
        &path_dir.join("codex"),
        &format!(
            "#!/bin/sh\nprintf 'ran\\n' > \"{}\"\nexit 0\n",
            marker_path.display()
        ),
    );

    // Include system dirs so shell expansion can find `echo`.
    let full_path = format!("{}:/usr/bin:/bin", path_dir.display());

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &full_path)
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    // The provider should have run (marker file exists).
    assert!(
        marker_path.exists(),
        "provider should launch after shell preflight + harness audit pass; stderr was:\n{plain}"
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
#[serial_test::serial]
fn inline_compose_file_changed_post_check_sees_closure_artifact() {
    // Verifies that file-state post_checks like `file_changed` evaluate
    // AFTER inline closure has rewritten the document, not before.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    // Initialize a git repo so @-prefixed path resolution works.
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(workspace.path())
        .output()
        .unwrap();

    let md_file = workspace.path().join("test.md");
    // The `file_changed` post-check points at the target document itself.
    // Before the fix, closure ran after post-checks, so the file was still
    // unchanged when the check ran, causing a spurious failure.
    fs::write(
        &md_file,
        "---\nprompt: Rewrite the body\npost_checks:\n  file_changed: \"@test.md\"\n---\nOriginal body\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf 'Brand new replacement body\\n'\nexit 0\n",
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("Brand new replacement body"),
        "closure should have rewritten the document; file: {final_content}"
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
fn compose_opencode_non_interactive_passes_prompt_as_positional_arg() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello OpenCode\n").unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();
    let run_index = collected
        .iter()
        .position(|arg| *arg == "run")
        .expect("compose should use the OpenCode run entrypoint");
    let format_index = collected
        .iter()
        .position(|arg| *arg == "--format")
        .expect("compose should request OpenCode JSON format");
    let json_index = format_index + 1;
    let prompt_index = collected
        .iter()
        .position(|arg| *arg == "Hello OpenCode")
        .expect("compose should pass the composed prompt as a positional arg for OpenCode");

    assert!(run_index < format_index, "args: {args}");
    assert_eq!(collected.get(json_index), Some(&"json"), "args: {args}");
    assert!(
        json_index < prompt_index,
        "OpenCode flags must precede the positional prompt so structured output is enabled; args: {args}"
    );
}

#[cfg(unix)]
#[test]
fn compose_opencode_launches_child_from_repo_root() {
    let workspace = tempdir().unwrap();
    let Some((repo_root, launch_dir, bin_dir)) = create_claudine_monorepo(workspace.path()) else {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    };
    let pwd_path = workspace.path().join("pwd.txt");
    let env_path = workspace.path().join("env.txt");
    let args_path = workspace.path().join("args.txt");
    let md_file = repo_root.join("prompts/test.md");
    write(&md_file, "---\ntitle: test\n---\nHello OpenCode\n");

    write_executable(
        &bin_dir.join("opencode"),
        r#"#!/bin/sh
pwd > "$CLAUDINE_PWD_FILE"
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
{
  printf 'PACKAGE=%s\n' "$PACKAGE"
  printf 'PACKAGE_AREA=%s\n' "$PACKAGE_AREA"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .current_dir(&launch_dir)
        .env("NO_COLOR", "1")
        .env("OPENCODE_MODEL", "test-model")
        .env("PATH", &bin_dir)
        .env("CLAUDINE_PWD_FILE", &pwd_path)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&pwd_path).unwrap().trim(),
        repo_root.canonicalize().unwrap().display().to_string()
    );
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("PACKAGE=claudine-cli"));
    assert!(env_lines.contains("PACKAGE_AREA=claudine"));
    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();
    assert!(collected.contains(&"run"));
    assert!(collected.contains(&"Hello OpenCode"));
}

#[cfg(unix)]
#[test]
fn compose_opencode_launches_from_repo_root_for_package_prompt_refs() {
    let workspace = tempdir().unwrap();
    let Some((repo_root, _launch_dir, bin_dir)) = create_claudine_monorepo(workspace.path()) else {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    };
    let package_root = repo_root.join("claudine");
    let pwd_path = workspace.path().join("pwd-package.txt");
    let env_path = workspace.path().join("env-package.txt");
    let args_path = workspace.path().join("args-package.txt");
    let md_file = package_root.join("prompts/test.md");
    write(&md_file, "---\ntitle: test\n---\nHello OpenCode\n");

    write_executable(
        &bin_dir.join("opencode"),
        r#"#!/bin/sh
pwd > "$CLAUDINE_PWD_FILE"
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
{
  printf 'PACKAGE=%s\n' "$PACKAGE"
  printf 'PACKAGE_AREA=%s\n' "$PACKAGE_AREA"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .current_dir(&package_root)
        .env("NO_COLOR", "1")
        .env("OPENCODE_MODEL", "test-model")
        .env("PATH", &bin_dir)
        .env("CLAUDINE_PWD_FILE", &pwd_path)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["compose", "--opencode", "@prompts/test.md"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&pwd_path).unwrap().trim(),
        repo_root.canonicalize().unwrap().display().to_string()
    );
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("PACKAGE_AREA=claudine"));
    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();
    assert!(collected.contains(&"run"));
    assert!(collected.contains(&"Hello OpenCode"));
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
/bin/cat > "$CLAUDINE_STDIN_FILE"
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
fn codex_structured_compose_filters_stdin_banner() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello Codex\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
last_message=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output-last-message" ]; then
    last_message="$arg"
  fi
  prev="$arg"
done

printf '%s\n' 'Reading prompt from stdin...' >&2
/bin/cat > /dev/null
if [ -n "$last_message" ]; then
  printf '%s\n' 'Recovered answer' > "$last_message"
fi
printf '%s\n' '{"type":"thread.started","thread_id":"codex-1"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"turn.completed","usage":{"input_tokens":20,"output_tokens":10,"cached_input_tokens":5}}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["compose", "--codex", "--quiet", md_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout("Recovered answer\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!stderr_plain.contains("Reading prompt from stdin..."));
}

#[cfg(unix)]
#[test]
fn codex_structured_compose_surfaces_live_tool_progress() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello Codex\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
last_message=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output-last-message" ]; then
    last_message="$arg"
  fi
  prev="$arg"
done

/bin/cat > /dev/null
printf '%s\n' '{"type":"thread.started","thread_id":"codex-1"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"item.started","item":{"id":"t1","type":"command_exec","tool_name":"shell","input":{"cmd":"git status"}}}'
printf '%s\n' '{"type":"item.completed","item":{"id":"t1","type":"command_exec","tool_name":"shell","output":"ok"}}'
printf '%s\n' '{"type":"item.started","item":{"id":"t2","type":"view_image","tool_name":"view_image"}}'
printf '%s\n' '{"type":"item.completed","item":{"id":"t2","type":"view_image","output":"ok"}}'
printf '%s\n' '{"type":"turn.completed","usage":{"input_tokens":20,"output_tokens":10,"cached_input_tokens":5}}'
if [ -n "$last_message" ]; then
  printf '%s\n' 'Recovered answer' > "$last_message"
fi
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["compose", "--codex", "--quiet", md_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout("Recovered answer\n");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    // The sink now renders the first non-empty string value as the tool
    // input preview instead of a truncated JSON blob (Plan 3 hardening).
    // Post response-refinement humanization (Task 1.2), tool names are
    // title-cased (`shell` → `Shell`, `view_image` → `View Image`).
    // Per the 2026-04-16 more-is-more Phase 1 change, tool call lines
    // render as `Name(summary)` instead of `Name · summary`.
    assert!(
        stderr_plain.contains("Shell(git status)"),
        "missing Shell call with 'git status' preview in parens in:\n{stderr_plain}"
    );
    assert!(
        stderr_plain.contains("View Image"),
        "missing View Image line in:\n{stderr_plain}"
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

#[test]
fn retired_compose_flag_rejected_in_wrapper() {
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["claude", "--compose", "file.md"])
        .assert()
        .failure()
        .stderr(contains("--compose has been retired"))
        .stderr(contains("claudine compose"));
}

#[test]
fn retired_frontmatter_prompt_flag_rejected_in_wrapper() {
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["claude", "--frontmatter-prompt", "file.md"])
        .assert()
        .failure()
        .stderr(contains("--frontmatter-prompt has been retired"))
        .stderr(contains("claudine inline-compose"));
}

#[test]
fn retired_prompt_file_flag_rejected_in_wrapper() {
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["claude", "--prompt-file", "file.md"])
        .assert()
        .failure()
        .stderr(contains("--prompt-file has been retired"))
        .stderr(contains("claudine compose"));
}

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
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Goose"),
        "repo config favorite should select Goose; stderr was: {plain}"
    );
    assert!(
        args_file.exists(),
        "goose should have been invoked via the config favorite"
    );
}

#[cfg(unix)]
#[test]
fn ambiguous_agent_hint_no_tty_returns_error() {
    // Verifies that an ambiguous `agent` hint with no TTY returns an error
    // instead of hanging on interactive selection.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    // "agent: c" matches both claude and codex via prefix matching,
    // producing an ambiguous hint that requires interactive selection.
    fs::write(&md_file, "---\ntitle: test\nagent: c\n---\nPrompt\n").unwrap();

    // Install both claude and codex so "c" is ambiguous
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");
    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    // Write empty stdin via a file to prevent TTY detection
    let stdin_file = workspace.path().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["compose", md_file.to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(contains("ambiguous"));
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
fn inline_closure_unchanged_body_retries_via_handler() {
    // Verifies that when an inline composition provider returns the
    // unchanged body, the failure is routed through the harness handler
    // system and a retry handler can recover.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let count_path = workspace.path().join("attempt-count.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Rewrite the body\nhandle_inline_body_unchanged:\n  retry:\n    prompt: \"The body must change. Rewrite it differently.\"\n---\nOriginal body\n",
    )
    .unwrap();

    // First attempt: echo back the original body (unchanged).
    // Second attempt: return a different body.
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
  printf 'Original body\n'
else
  printf 'Revised and improved body\n'
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
    assert_eq!(
        attempts.trim(),
        "2",
        "expected retry after unchanged body failure"
    );

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("Revised and improved body"),
        "retry should apply the changed body; file: {final_content}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_readonly_file_fails_without_harness() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("readonly.md");
    fs::write(&md_file, "---\nprompt: Generate\n---\nBody\n").unwrap();

    // Make the file read-only
    let mut perms = fs::metadata(&md_file).unwrap().permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&md_file, perms).unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'should not run'\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("insufficient file permissions")
            || stderr.contains("Permission denied")
            || stderr.contains("permission"),
        "should report a permission error for read-only files; stderr: {stderr}"
    );

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&md_file).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&md_file, perms).unwrap();
}

#[cfg(unix)]
#[test]
fn inline_compose_harness_writability_pre_check_fires() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let marker_path = workspace.path().join("provider-ran.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("locked.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate\npre_checks:\n  file_exists: \"locked.md\"\n---\nBody\n",
    )
    .unwrap();

    // Make the file read-only so the system writability check fails
    let mut perms = fs::metadata(&md_file).unwrap().permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&md_file, perms).unwrap();

    write_executable(
        &path_dir.join("goose"),
        &format!("#!/bin/sh\ntouch \"{}\"\n", marker_path.display()),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("pre-check validation failed"),
        "harness should report a pre-check failure for the writability check; stderr: {stderr}"
    );
    assert!(
        !marker_path.exists(),
        "provider should not have been launched when the writability pre-check fails"
    );

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&md_file).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&md_file, perms).unwrap();
}

// ---------------------------------------------------------------------------
// Handler-engagement banner emission semantics
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn handler_engagement_banner_suppressed_when_retry_ceiling_reached() {
    // When the retry ceiling is hit and no recovery plan is produced,
    // the "engaging registered handlers" banner must NOT appear a second time.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

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

        // Stub binary so PATH resolution succeeds in dry-run mode.
        // Dry-run never actually spawns the child, so the stub body
        // doesn't matter — the stub only needs to exist and be
        // executable for claudine's binary-resolution step.
        write_executable(&path_dir.join(provider_slug), "#!/bin/sh\nexit 0\n");

        let output = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
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

/// End-to-end: for every wrapped provider, verify that
/// `claudine sequence compose.md --<provider> --dry-run` runs cleanly
/// through the composition pipeline with a trivial markdown body.
///
/// Regression guard for the composition-path drift: if a provider's
/// `apply_entrypoint` / `apply_non_interactive_flags` / `prompt_delivery`
/// chain silently bails when the prompt arrives via the composition
/// body, this test fails.
#[cfg(unix)]
#[test]
fn sequence_composition_dry_run_for_every_provider() {
    for provider_slug in [
        "claude", "codex", "gemini", "kimi", "opencode", "qwen", "goose",
    ] {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();

        write_executable(&path_dir.join(provider_slug), "#!/bin/sh\nexit 0\n");

        let compose_file = workspace.path().join("compose.md");
        fs::write(
            &compose_file,
            "---\nsequence:\n  - step_one\n---\ncomposed body text\n",
        )
        .unwrap();

        let output = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .env("OPENCODE_MODEL", "test-model")
            .env("PATH", &path_dir)
            .current_dir(workspace.path())
            .args([
                "sequence",
                "compose.md",
                &format!("--{provider_slug}"),
                "--dry-run",
            ])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "`claudine sequence compose.md --{provider_slug} --dry-run` failed: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// ---------------------------------------------------------------------------
// OpenCode model resolution (Phase 7 integration tests)
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod opencode_model_integration {
    use super::*;

    #[test]
    fn no_model_provided_renders_blockquote_without_text_above() {
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
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .args(["opencode", "summarize"])
            .assert()
            .code(1);

        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        let plain = strip_ansi(&stderr);

        assert!(plain.contains("No model specified!"));
        assert!(plain.contains("OPENCODE_MODEL"));
        assert!(plain.contains("--model"));
        assert!(plain.contains("opencode models"));

        let block_quote_start = plain.find("┃").unwrap();
        let before_block = &plain[..block_quote_start];
        let trimmed_before = before_block.trim();
        assert!(
            trimmed_before.is_empty(),
            "no text should appear above the BlockQuote; got: '{trimmed_before}'"
        );
    }

    #[test]
    fn cli_model_proceeds_past_resolver() {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        let args_path = workspace.path().join("args.txt");

        write_executable(
            &path_dir.join("opencode"),
            r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
        );

        cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .env("PATH", &path_dir)
            .env("CLAUDINE_ARGS_FILE", &args_path)
            .args(["opencode", "--model", "test-model", "summarize"])
            .assert()
            .success();

        let args = fs::read_to_string(&args_path).unwrap();
        assert!(args.lines().any(|line| line == "test-model"));
    }

    #[test]
    fn invalid_model_error_shows_suggestions() {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();

        write_executable(
            &path_dir.join("opencode"),
            r#"#!/bin/sh
printf 'Error: ProviderModelNotFoundError: model bad-model not found\nsuggestions: ["provider/a", "provider/b"]\n' >&2
exit 1
"#,
        );

        let assert = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .env("OPENCODE_MODEL", "bad-model")
            .args(["opencode", "summarize"])
            .assert()
            .code(1);

        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        let plain = strip_ansi(&stderr);

        assert!(
            plain.contains("Invalid model specified")
                || plain.contains("ProviderModelNotFoundError"),
            "expected model-not-found error in stderr; got:\n{plain}"
        );
        assert!(
            plain.contains("provider/a"),
            "expected suggestion 'provider/a' in stderr; got:\n{plain}"
        );
        assert!(
            plain.contains("provider/b"),
            "expected suggestion 'provider/b' in stderr; got:\n{plain}"
        );
    }

    #[test]
    fn config_file_model_resolves_successfully() {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        let args_path = workspace.path().join("args.txt");
        let env_path = workspace.path().join("env.txt");

        let config_dir = workspace.path().join(".config/opencode");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.json"),
            r#"{"model":"config-default-model"}"#,
        )
        .unwrap();

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
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .env("CLAUDINE_ARGS_FILE", &args_path)
            .env("CLAUDINE_ENV_FILE", &env_path)
            .args(["opencode", "summarize"])
            .assert()
            .success();

        let env_lines = fs::read_to_string(&env_path).unwrap();
        assert!(env_lines.contains("MODEL=config-default-model"));

        let args = fs::read_to_string(&args_path).unwrap();
        assert!(
            !args.lines().any(|line| line == "--model"),
            "ConfigDefault should NOT push --model to child args"
        );
    }
}

/// End-to-end regression test for the OpenCode structured wrap pipeline.
///
/// Motivated by review-2 finding #4: the OpenCode assistant-text bug that
/// inspired this feature was in the wrapped pipeline, not just the parser.
/// This test mirrors the Codex/Gemini `*_structured_*` coverage so the
/// end-to-end OpenCode path has the same regression surface.
///
/// Asserts that a real child process, stream parser, and live sink
/// cooperate to produce:
///
/// 1. The assistant text reaches stdout (regression guard for the missing-
///    assistant-text bug).
/// 2. The tool completion renders as a canonical incoming `←` line with
///    a humanized tool name — NOT a synthesized outgoing `→` line and
///    NOT via the `⚙` Info glyph.
/// 3. No two consecutive blank lines appear in the combined stdout +
///    stderr rendered output (Child 3 section-spacing contract).
/// 4. The session-id marker and the trailer render on stderr.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_structured_e2e_stdout_and_section_spacing() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    // Minimal OpenCode fake binary. Uses the simple parser-compatible
    // event shapes (matching the `step_start` / `text` / `tool_end` /
    // `step_complete` parser unit tests in opencode_semantic.rs) rather
    // than the full nested `.part` payload, which keeps the test
    // resilient to stream-protocol evolution while still exercising the
    // sink end-to-end.
    let args_path = workspace.path().join("args.txt");
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf '%s\n' '{"type":"step_start","sessionID":"ses_oc_e2e"}'
printf '%s\n' '{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok","tool_name":"bash"}}'
printf '%s\n' '{"type":"text","text":"The answer is 42."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15},"cost_usd":0.001,"duration_ms":1500}'
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_ARGS_FILE", &args_path)
        // NOTE: intentionally do NOT pass `--format json` here — claudine
        // treats an explicit native output request as a signal to skip
        // structured streaming and forward raw bytes, which would bypass
        // every behavior this test is guarding.
        .args(["opencode", "what is the answer"])
        .assert()
        .success();

    let args_captured = fs::read_to_string(&args_path).unwrap_or_default();
    assert!(
        !args_captured.is_empty(),
        "fake opencode should have been invoked (CLAUDINE_ARGS_FILE empty)"
    );

    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr_raw = String::from_utf8_lossy(&out.stderr);
    let stderr = strip_ansi(&stderr_raw);

    // 1. Assistant text reaches stdout.
    assert!(
        stdout.contains("The answer is 42."),
        "expected assistant text on stdout; stdout={stdout:?}"
    );

    // 2. Incoming `←` line exists; no synthesized outgoing `→` line; no
    //    `⚙` Info glyph routing.
    assert!(
        stderr.contains('\u{2190}'),
        "expected incoming ← arrow for tool completion; stderr={stderr}"
    );
    assert!(
        !stderr.contains('\u{2192}'),
        "OpenCode must NOT synthesize an outgoing → line; stderr={stderr}"
    );
    assert!(
        !stderr.contains('\u{2699}'),
        "tool result must NOT be rendered via the ⚙ Info glyph; stderr={stderr}"
    );
    assert!(
        stderr.contains("Bash"),
        "humanized tool name must render; stderr={stderr}"
    );

    // 3. Session ID + trailer render on stderr.
    assert!(
        stderr.contains("session ID") && stderr.contains("ses_oc_e2e"),
        "expected session ID marker; stderr={stderr}"
    );
    assert!(
        stderr.contains("1.5s") || stderr.contains("1.500s") || stderr.contains("1,500ms"),
        "expected trailer duration (1.5s); stderr={stderr}"
    );

    // 4. No two consecutive blank lines in combined stdout+stderr. The
    //    stderr side is line-structured already; the stdout side is a
    //    single chunk so we just concat and split on '\n'.
    let combined = format!("{stdout}{stderr}");
    let mut prev_blank = false;
    for line in combined.lines() {
        let is_blank = line.trim().is_empty();
        assert!(
            !(is_blank && prev_blank),
            "two consecutive blank lines in combined rendered output:\n---\n{combined}\n---"
        );
        prev_blank = is_blank;
    }
}

// ---------------------------------------------------------------------------
// OpenCode stderr log bridge integration (Phase 6 integration scenarios)
//
// These tests drive claudine's structured OpenCode wrapper path end-to-end
// with a fake `opencode` binary that emits NDJSON on stdout and structured
// log records on stderr. They assert against the persisted JSONL log row
// produced by the summary event emitter in
// `claudine::stream::reporting::summary_to_event_meta(...)` so the
// verification does not depend on terminal rendering or ANSI behavior.
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn read_summary_row(home: &Path) -> serde_json::Value {
    let log_path = today_log_path(home);
    let contents =
        fs::read_to_string(&log_path).expect("today's JSONL log should exist after wrap run");
    let row = contents
        .lines()
        .find(|line| line.contains("\"synthetic_kind\":\"stream_wrapper_summary\""))
        .unwrap_or_else(|| panic!("no stream_wrapper_summary row in log:\n{contents}"));
    serde_json::from_str::<serde_json::Value>(row)
        .unwrap_or_else(|e| panic!("failed to parse summary row as JSON ({e}): {row}"))
}

/// Phase 6 scenario: stderr rate limit arrives before any stdout semantic
/// event. The bridge should signal early termination, kill the child, and
/// synthesize a `usage_limit_reached` failure summary.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_stderr_rate_limit_before_stdout_forces_early_termination() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    // The fake binary only writes the rate-limit ERROR to stderr, then
    // sleeps so the bridge has to abort it. If claudine fails to terminate
    // early, the assert_cmd timeout would trip and the test would fail.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}' >&2
sleep 30
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .timeout(std::time::Duration::from_secs(30))
        .args(["opencode", "describe the thing"])
        .assert()
        .failure();

    let output = assert.get_output();
    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        1,
        "pre-stream rate limit must map to exit_code=1; got {exit_code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let row = read_summary_row(&fake_home);
    assert_eq!(row["extra"]["exit_code"], serde_json::json!(1));
    assert_eq!(
        row["error"].as_str().unwrap_or(""),
        row["error"].as_str().unwrap_or(""),
        "error field should carry the rate-limit message",
    );
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["rate_limit_events"],
        serde_json::json!(1),
        "stderr diagnostics should record the rate-limit event: row={row}",
    );
    let rate_limit = &row["extra"]["provider_summary"]["rate_limit"];
    assert_eq!(
        rate_limit["is_throttled"],
        serde_json::json!(true),
        "rate_limit.is_throttled should be true: row={row}",
    );
    assert!(
        rate_limit["reset_at"].is_string(),
        "rate_limit.reset_at should be populated: row={row}",
    );
}

/// Phase 6 scenario: a malformed asset stderr line during an otherwise-
/// successful run should surface as a Warning event (rendered once per
/// line) without failing the session. Per the 2026-04-18 OpenCode
/// reporting contract, the diagnostics counter is preserved while the
/// trailer Config badge is suppressed.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_stderr_malformed_asset_records_diagnostic_without_config_badge() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'ERROR 2026-04-15T21:28:30 +315ms service=config command=/Users/ken/.config/opencode/commands/catalog.md err=ENOENT: no such file or directory, open '"'"'/Users/ken/.config/opencode/commands/catalog.md'"'"' failed to load command' >&2
printf '%s\n' '{"type":"step_start","sessionID":"ses_cfg_ok"}'
printf '%s\n' '{"type":"text","text":"Warnings are non-fatal."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":200}'
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "just classify"])
        .assert()
        .success();

    let row = read_summary_row(&fake_home);
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["malformed_asset_events"],
        serde_json::json!(1),
        "stderr diagnostics should record one malformed asset: row={row}",
    );
    // Per the 2026-04-18 OpenCode reporting contract, malformed-asset
    // events do not produce a trailer Config badge — the per-line
    // Warning surface is the authoritative reporting channel. The
    // `badges` field may be absent or empty; either is acceptable as
    // long as no Config-category badge is present.
    let badges = row["extra"]["badges"].as_array();
    if let Some(b) = badges {
        assert!(
            !b.iter()
                .any(|b| b["category"] == serde_json::json!("config")),
            "Config trailer badge must be absent — malformed assets are surfaced once per Warning line: {b:?}",
        );
    }
    assert_eq!(
        row["extra"]["exit_code"],
        serde_json::json!(0),
        "malformed assets should not fail the session: row={row}",
    );
}

/// Phase 6 scenario: mixed structured stderr plus an ANSI-wrapped raw
/// `Error:` line plus benign chatter. The bridge should consume only
/// classified lines while raw lines still reach the user unchanged.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_stderr_mixed_shapes_only_consume_classified_lines() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    // `bare chatter line` should flow through as raw stderr because it
    // doesn't match the header regex and isn't an ANSI `Error:` block.
    // The ERROR/skill line is classified as MalformedAsset.
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'ERROR 2026-04-15T21:28:30 +0ms service=config skill=/tmp/s.md err=ENOENT failed to load skill' >&2
printf '%s\n' 'bare chatter line from the provider' >&2
printf '%s\n' '{"type":"step_start","sessionID":"ses_mix"}'
printf '%s\n' '{"type":"text","text":"All good."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":100}'
exit 0
"#,
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "mixed run"])
        .assert()
        .success();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    // Structured ERROR line must NOT reach raw stderr passthrough
    // (the bridge consumed it, then re-emitted it as a Warning event).
    assert!(
        !stderr.contains("failed to load skill"),
        "classified stderr must be suppressed from raw passthrough; stderr={stderr}",
    );
    // Bare chatter is unclassified so it must continue to surface.
    assert!(
        stderr.contains("bare chatter line"),
        "unclassified stderr must still passthrough to the operator; stderr={stderr}",
    );

    let row = read_summary_row(&fake_home);
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["malformed_asset_events"],
        serde_json::json!(1),
        "stderr diagnostics should record the malformed asset: row={row}",
    );
    assert!(
        row["extra"]["provider_summary"]["stderr_diagnostics"]["log_records_parsed"]
            .as_u64()
            .unwrap_or(0)
            >= 1,
        "log_records_parsed should count the structured record: row={row}",
    );
}

/// Phase 6 scenario: the final summary must contain merged `stderr_text`,
/// `stderr_diagnostics`, and recomputed `badges`. Guards the wrapper-layer
/// summary merge step that runs after both reader threads join.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_structured_summary_merges_stderr_diagnostics_and_badges() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'ERROR 2026-04-15T21:28:30 +0ms service=config command=/tmp/a.md err=ENOENT failed to load command' >&2
printf '%s\n' 'ERROR 2026-04-15T21:28:30 +0ms service=config agent=/tmp/b.md err=ENOENT failed to load agent' >&2
printf '%s\n' '{"type":"step_start","sessionID":"ses_merge"}'
printf '%s\n' '{"type":"text","text":"Merge ok."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":50}'
exit 0
"#,
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &fake_home)
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "merge probe"])
        .assert()
        .success();

    let row = read_summary_row(&fake_home);
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["malformed_asset_events"],
        serde_json::json!(2),
        "both malformed asset events should be accumulated: row={row}",
    );
    assert!(
        diagnostics["log_records_parsed"].as_u64().unwrap_or(0) >= 2,
        "log_records_parsed should count both records: row={row}",
    );

    // Per the 2026-04-18 OpenCode reporting contract, malformed-asset
    // events do not produce a trailer Config badge — the per-line
    // Warning surface is the authoritative reporting channel. The
    // `badges` field may be absent or empty; either is acceptable as
    // long as no Config-category badge is present.
    let badges = row["extra"]["badges"].as_array();
    if let Some(b) = badges {
        assert!(
            !b.iter()
                .any(|b| b["category"] == serde_json::json!("config")),
            "Config trailer badge must be absent after stderr merge — diagnostics counter still records the events: {b:?}",
        );
    }
}

// ============================================================================
// Performance flag tests
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_perf_emits_report_to_stderr() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: perf test\n---\n# Hello\n").unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'Agent response'\nexit 0\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", "--perf", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("CLI Overhead"),
        "stderr should contain CLI Overhead section; got: {plain}"
    );
    assert!(
        plain.contains("Agent Execution"),
        "stderr should contain Agent Execution section; got: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_perf_stdout_matches_non_perf() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: perf test\n---\n# Hello\n").unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'Agent response'\nexit 0\n",
    );

    let perf_assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", "--perf", md_file.to_str().unwrap()])
        .assert()
        .success();

    let plain_assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let perf_stdout = String::from_utf8_lossy(&perf_assert.get_output().stdout);
    let plain_stdout = String::from_utf8_lossy(&plain_assert.get_output().stdout);

    assert_eq!(
        perf_stdout, plain_stdout,
        "stdout must be identical between --perf and non-perf runs"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_perf_emits_report_to_stderr() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: inline perf\nprompt: say hello\n---\n# Body\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'Replacement body'\nexit 0\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "inline-compose",
            "--goose",
            "--perf",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("CLI Overhead"),
        "stderr should contain CLI Overhead section; got: {plain}"
    );
    assert!(
        plain.contains("Agent Execution"),
        "stderr should contain Agent Execution section; got: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_perf_stdout_matches_non_perf() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file_perf = workspace.path().join("test-perf.md");
    let md_file_plain = workspace.path().join("test-plain.md");
    let content = "---\ntitle: inline perf\nprompt: say hello\n---\n# Body\n";
    fs::write(&md_file_perf, content).unwrap();
    fs::write(&md_file_plain, content).unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'Replacement body'\nexit 0\n",
    );

    let perf_assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "inline-compose",
            "--goose",
            "--perf",
            md_file_perf.to_str().unwrap(),
        ])
        .assert()
        .success();

    let plain_assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["inline-compose", "--goose", md_file_plain.to_str().unwrap()])
        .assert()
        .success();

    let perf_stdout = String::from_utf8_lossy(&perf_assert.get_output().stdout);
    let plain_stdout = String::from_utf8_lossy(&plain_assert.get_output().stdout);

    assert_eq!(
        perf_stdout, plain_stdout,
        "stdout must be identical between --perf and non-perf runs"
    );
}

#[cfg(unix)]
#[test]
fn compose_dry_run_perf_renders_report_without_agent_execution() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: dry run perf\n---\n# Hello\n").unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'should not run'\nexit 0\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "compose",
            "--goose",
            "--perf",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("dry run"),
        "perf report should note dry run; got: {plain}"
    );

    // Provider should NOT have run in dry-run mode.
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        !stdout.contains("should not run"),
        "provider should not execute in dry-run mode"
    );
}

/// Verifies that the `arg parsing:` timing in the perf report captures
/// the full pipeline including `argv::normalize` and `parse_cli_from`.
/// This is a smoke test: the exact duration is environment-dependent, but
/// the line must appear with a formatted duration.
#[cfg(unix)]
#[test]
fn perf_arg_parsing_includes_clap_time() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: arg parse perf\n---\n# Hello\n").unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'Agent response'\nexit 0\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", "--perf", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // The arg parsing line must be present and show a duration.
    // We allow 0µs because timer resolution varies, but the line must exist.
    assert!(
        plain.contains("arg parsing:"),
        "perf report must include arg parsing timing; got: {plain}"
    );

    // Ensure the other startup timings are also present, confirming the
    // full CLI Overhead section is rendered.
    assert!(
        plain.contains("config loading:"),
        "perf report must include config loading timing; got: {plain}"
    );
    assert!(
        plain.contains("tracing init:"),
        "perf report must include tracing init timing; got: {plain}"
    );
    assert!(
        plain.contains("environment setup:"),
        "perf report must include environment setup timing; got: {plain}"
    );
}
