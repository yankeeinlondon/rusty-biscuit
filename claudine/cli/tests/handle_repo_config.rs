mod common;
use common::{CliProcessFixture, init_git_repo, write};

#[test]
fn handle_reads_repo_scoped_config_from_cwd_repo_root() {
    let fixture = CliProcessFixture::named("handle-repo-config");
    let home_dir = fixture.home().to_path_buf();
    let repo_root = fixture.cwd().to_path_buf();

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

    let output = fixture
        .command()
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
    let fixture = CliProcessFixture::named("handle-repo-package-env");
    let home_dir = fixture.home().to_path_buf();
    let repo_root = fixture.cwd().to_path_buf();

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

    let output = fixture
        .command()
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
