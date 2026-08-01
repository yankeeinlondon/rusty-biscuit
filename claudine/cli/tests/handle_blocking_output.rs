#![cfg(unix)]

#[cfg(unix)]
#[cfg(unix)]
use serde_json::Value;

#[cfg(unix)]
mod common;
#[cfg(unix)]
use common::{TestWorkspace, write};

#[cfg(unix)]
#[test]
fn handle_flushes_blocking_payload_before_nonzero_exit() {
    let workspace = TestWorkspace::named("claudine-handle-blocking-output");
    let home_dir = workspace.path().join("home");
    std::fs::create_dir_all(&home_dir).unwrap();

    let config = serde_json::json!({
        "preferred_agent": "gemini",
        "tts": false,
        "logging": false,
        "protect": { "enabled": false },
        "actions": {
            "turn_complete": [
                {
                    "type": "call",
                    "command": "sh",
                    "args": ["-c", "echo blocked by handler; exit 2"]
                }
            ]
        }
    });
    write(
        &home_dir.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["handle", "turn_complete", "--provider", "gemini"])
        .write_stdin(r#"{"hook_event_name":"AfterAgent","session_id":"flush-test-1"}"#)
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();

    let parsed: Value =
        serde_json::from_slice(&output).expect("blocking payload should be flushed");
    assert_eq!(
        parsed["reason"],
        Value::String("blocked by handler".to_string())
    );
    assert_eq!(parsed["clearContext"], Value::Bool(false));
}
