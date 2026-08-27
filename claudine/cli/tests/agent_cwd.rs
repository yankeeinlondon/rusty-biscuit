use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

const CLAUDE_BEFORE_PROMPT: &str =
    r#"{"hook_event_name":"UserPromptSubmit","prompt":"hi","session_id":"agent-cwd-test"}"#;

#[test]
fn handle_rejects_present_non_absolute_agent_cwd() {
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), None);
    claudine_command(home.path())
        .current_dir(home.path())
        .env("AGENT_CWD", "relative/path")
        .args(["handle", "before_prompt", "--provider", "claude"])
        .write_stdin(CLAUDE_BEFORE_PROMPT)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "AGENT_CWD must be an absolute path for `claudine handle`; got `relative/path`",
        ));
}

#[test]
fn handle_without_agent_cwd_passes_its_absolute_entry_cwd_to_hook_action() {
    let home = tempfile::tempdir().unwrap();
    let entry = tempfile::tempdir().unwrap();
    let observed = home.path().join("observed.txt");
    let (command, params) = recording_action(&observed);
    write_config(home.path(), Some((&command, &params)));

    claudine_command(home.path())
        .current_dir(entry.path())
        .env_remove("AGENT_CWD")
        .args(["handle", "before_prompt", "--provider", "claude"])
        .write_stdin(CLAUDE_BEFORE_PROMPT)
        .assert()
        .success();

    assert_same_path(&read_path(&observed), entry.path());
}

#[test]
fn handle_retains_wrapper_launch_directory_across_provider_cwd() {
    let home = tempfile::tempdir().unwrap();
    let wrapper_launch = tempfile::tempdir().unwrap();
    let provider_cwd = tempfile::tempdir().unwrap();
    let observed = home.path().join("observed.txt");
    let (command, params) = recording_action(&observed);
    write_config(home.path(), Some((&command, &params)));

    claudine_command(home.path())
        .current_dir(provider_cwd.path())
        .env("AGENT_CWD", wrapper_launch.path())
        .args(["handle", "before_prompt", "--provider", "claude"])
        .write_stdin(CLAUDE_BEFORE_PROMPT)
        .assert()
        .success();

    assert_same_path(&read_path(&observed), wrapper_launch.path());
    assert_ne!(
        fs::canonicalize(read_path(&observed)).unwrap(),
        fs::canonicalize(provider_cwd.path()).unwrap()
    );
}

#[cfg(unix)]
#[test]
fn ordinary_nested_cli_overwrites_stale_agent_cwd_for_provider_child() {
    use std::os::unix::fs::PermissionsExt;

    let home = tempfile::tempdir().unwrap();
    let entry = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let observed = home.path().join("provider-observed.txt");
    write_config(home.path(), None);
    let provider = tools.path().join("codex");
    fs::write(
        &provider,
        "#!/bin/sh\nprintf %s \"$AGENT_CWD\" > \"$CLAUDINE_OBSERVED\"\nexit 0\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&provider).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&provider, permissions)
    .unwrap();

    claudine_command(home.path())
        .current_dir(entry.path())
        .env("PATH", tools.path())
        .env("CLAUDINE_OBSERVED", &observed)
        .env("AGENT_CWD", "/stale/parent")
        .args(["codex", "say hi"])
        .assert()
        .success();

    assert_same_path(&read_path(&observed), entry.path());
}

fn claudine_command(home: &Path) -> Command {
    let mut command = Command::cargo_bin("claudine").unwrap();
    command.env("HOME", home).env("USERPROFILE", home);
    command
}

fn write_config(home: &Path, action: Option<(&str, &str)>) {
    let directory = home.join(".claudine");
    fs::create_dir_all(&directory).unwrap();
    let actions = action.map_or_else(
        || serde_json::json!({}),
        |(command, params)| {
            serde_json::json!({
                "before_prompt": [{
                    "type": "bash",
                    "command": command,
                    "params": params
                }]
            })
        },
    );
    fs::write(
        directory.join("config.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "tts": false,
            "logging": false,
            "protect": false,
            "actions": actions
        }))
        .unwrap(),
    )
    .unwrap();
}

#[cfg(unix)]
fn recording_action(observed: &Path) -> (String, String) {
    (
        "sh".to_string(),
        format!("-c 'printf %s \"$AGENT_CWD\" > \"{}\"'", observed.display()),
    )
}

#[cfg(windows)]
fn recording_action(observed: &Path) -> (String, String) {
    (
        "cmd.exe".to_string(),
        format!(r#"/D /C "echo %AGENT_CWD%>\"{}\"""#, observed.display()),
    )
}

fn read_path(path: &Path) -> PathBuf {
    PathBuf::from(fs::read_to_string(path).unwrap().trim())
}

fn assert_same_path(actual: &Path, expected: &Path) {
    assert_eq!(
        fs::canonicalize(actual).unwrap_or_else(|_| actual.to_path_buf()),
        fs::canonicalize(expected).unwrap_or_else(|_| expected.to_path_buf())
    );
}
