use std::fs;

mod common;
use common::{TestWorkspace, init_git_repo, write};

#[test]
fn handle_reads_repo_scoped_config_from_cwd_repo_root() {
    let workspace = TestWorkspace::named("claudine-handle-repo-it");
    let home_dir = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&repo_root).unwrap();

    // Skip if git is unavailable in the test environment.
    if !init_git_repo(&repo_root) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    // Minimal user config (required by load_claudine_config before repo merge).
    let user_config = serde_json::json!({
        "preferred_agent": "claude",
        "tts": false,
        "logging": false,
        "protect": { "enabled": false }
    });
    write(
        &home_dir.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&user_config).unwrap(),
    );

    // Repo override config (only contains overridable fields).
    let repo_config = serde_json::json!({
        "actions": {
            "session_start": [
                {
                    "type": "report",
                    "handler": {
                        "format": "json"
                    }
                }
            ]
        }
    });
    write(
        &repo_root.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&repo_config).unwrap(),
    );

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&repo_root)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["handle", "session_start", "--provider", "claude"])
        .write_stdin(r#"{"hook_event_name":"SessionStart","session_id":"repo-cfg-123"}"#)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    assert!(
        stdout.contains("SessionStart") || stdout.contains("session_start"),
        "report output should contain event name"
    );
}

#[test]
fn handle_logs_wrapper_package_context_from_env() {
    let workspace = TestWorkspace::named("claudine-handle-repo-it");
    let home_dir = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&repo_root).unwrap();

    if !init_git_repo(&repo_root) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    // Minimal user config (required by load_claudine_config before repo merge).
    let user_config = serde_json::json!({
        "preferred_agent": "claude",
        "tts": false,
        "logging": false,
        "protect": { "enabled": false }
    });
    write(
        &home_dir.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&user_config).unwrap(),
    );

    // Repo override config (only contains overridable fields).
    let repo_config = serde_json::json!({
        "actions": {
            "session_start": [
                {
                    "type": "report",
                    "handler": {
                        "format": "json"
                    }
                }
            ]
        }
    });
    write(
        &repo_root.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&repo_config).unwrap(),
    );

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&repo_root)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .env("PACKAGE_AREA", "claudine")
        .env("PACKAGE", "claudine-cli")
        .args(["handle", "session_start", "--provider", "claude"])
        .write_stdin(r#"{"hook_event_name":"SessionStart","session_id":"pkg-env-123"}"#)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    assert!(
        stdout.contains("package_area") || stdout.contains("claudine"),
        "report output should contain package context"
    );
}
