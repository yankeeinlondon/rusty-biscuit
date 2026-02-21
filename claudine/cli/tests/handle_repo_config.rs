use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;

struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("claudine-handle-repo-it-{}-{nonce}", process::id()));
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

fn init_git_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("init")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn handle_reads_repo_scoped_config_from_cwd_repo_root() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&repo_root).unwrap();

    // Skip if git is unavailable in the test environment.
    if !init_git_repo(&repo_root) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    let log_path = repo_root.join("events.jsonl");
    let config = serde_json::json!({
        "version": "1.0",
        "settings": {
            "linking": {
                "canonical_provider": {
                    "repo_skill": "claude"
                }
            }
        },
        "providers": {
            "claude": {
                "events": {
                    "session_start": {
                        "enabled": true,
                        "actions": [
                            {
                                "type": "log",
                                "target": {
                                    "type": "file",
                                    "path": log_path,
                                    "rotate_daily": false
                                }
                            }
                        ]
                    }
                }
            }
        }
    });
    write(
        &repo_root.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    cargo_bin_cmd!("claudine")
        .current_dir(&repo_root)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["handle", "session_start", "--provider", "claude"])
        .write_stdin(r#"{"hook_event_name":"SessionStart","session_id":"repo-cfg-123"}"#)
        .assert()
        .success();

    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("repo-cfg-123"));
}
