#![cfg(unix)]

#[cfg(unix)]
#[cfg(unix)]
use serde_json::Value;

#[cfg(unix)]
mod common;
#[cfg(unix)]
use common::{CliProcessFixture, write};

#[cfg(unix)]
#[test]
fn handle_flushes_blocking_payload_before_nonzero_exit() {
    let fixture = CliProcessFixture::named("claudine-handle-blocking-output");

    let config = serde_json::json!({
        "preferred_agent": "gemini",
        "tts": false,
        "logging": false,
        "protect": { "enabled": false },
        "actions": {
            "turn_complete": [
                {
                    "type": "call",
                    "command": "/bin/sh",
                    "args": ["-c", "echo blocked by handler; exit 2"]
                }
            ]
        }
    });
    write(
        &fixture.home().join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    let output = fixture
        .command()
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
