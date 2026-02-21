use std::fs;
use std::path::Path;
use std::process::{self};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

fn test_root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("claudine-protect-it-{}-{nonce}", process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn init_quick_writes_protect_defaults() {
    let root = test_root();
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();

    cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["init", "--quick"])
        .assert()
        .success();

    let config_path = home.join(".claudine/config.json");
    let config: Value = serde_json::from_str(&fs::read_to_string(config_path).unwrap()).unwrap();
    assert_eq!(config["settings"]["protect"]["enabled"], true);
    assert_eq!(config["settings"]["protect"]["posture"], "balanced");
}

#[test]
fn dry_run_json_includes_protect_preview() {
    let root = test_root();
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();

    let config = serde_json::json!({
        "version": "1.0",
        "settings": {
            "protect": {
                "enabled": true,
                "posture": "balanced",
                "rules": {
                    "blocked_command_patterns": ["rm -rf"],
                    "ask_command_patterns": [],
                    "protected_paths": [],
                    "secret_patterns": []
                }
            }
        },
        "providers": {
            "claude": {
                "events": {
                    "before_tool": {
                        "enabled": true,
                        "actions": [
                            { "type": "report" }
                        ]
                    }
                }
            }
        }
    });
    write(
        &home.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    let output = cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["dry-run", "before_tool", "--provider", "claude", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["parsed"], true);
    assert!(parsed.get("protect").is_some());
}

#[test]
fn handle_json_includes_protect_decisions() {
    let root = test_root();
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();

    let config = serde_json::json!({
        "version": "1.0",
        "settings": {
            "protect": {
                "enabled": true,
                "posture": "balanced",
                "rules": {
                    "blocked_command_patterns": ["rm -rf"],
                    "ask_command_patterns": [],
                    "protected_paths": [],
                    "secret_patterns": []
                }
            }
        },
        "providers": {
            "claude": {
                "events": {
                    "before_tool": {
                        "enabled": true,
                        "actions": [
                            { "type": "report" }
                        ]
                    }
                }
            }
        }
    });
    write(
        &home.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    let payload = r#"{
      "hook_event_name":"PreToolUse",
      "session_id":"protect-session-1",
      "tool_name":"Bash",
      "tool_input":{"command":"rm -rf /tmp/protect-test"}
    }"#;

    let output = cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["handle", "before_tool", "--provider", "claude", "--json"])
        .write_stdin(payload)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert!(parsed.get("protect_pre").is_some());
}
