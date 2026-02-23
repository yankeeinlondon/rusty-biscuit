use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;

struct TestWorkspace {
    root: PathBuf,
}

static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

impl TestWorkspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "claudine-wrap-it-{}-{nonce}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
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

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;

    write(path, content);
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[test]
fn help_lists_wrapper_subcommands() {
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["--help"])
        .assert()
        .success()
        .stdout(contains("claude"))
        .stdout(contains("codex"))
        .stdout(contains("gemini"))
        .stdout(contains("kimi"))
        .stdout(contains("qwen"))
        .stdout(contains("opencode"))
        .stdout(contains("goose"));
}

#[test]
fn wrapper_help_includes_expected_flags() {
    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["codex", "--help"])
        .assert()
        .success()
        .stdout(contains("--include <ENV_NAME>"))
        .stdout(contains("--non-interactive"))
        .stdout(contains("--yolo"));
}

#[cfg(unix)]
#[test]
fn wrapper_preserves_passthrough_args_and_injects_env() {
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
        .env("OPENAI_API_KEY", "keep")
        .env("INTERNAL_TOKEN", "remove")
        .args(["codex", "--include", "OPENAI_API_KEY", "--", "--version"])
        .assert()
        .success()
        .stderr(contains("Environment Variables"))
        .stderr(contains("INTERNAL_TOKEN"))
        .stderr(contains("potentially dangerous ENV variables were removed"))
        .stderr(contains("--include"));
}

#[cfg(unix)]
#[test]
fn wrapper_propagates_child_exit_code() {
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
    let workspace = TestWorkspace::new();
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
