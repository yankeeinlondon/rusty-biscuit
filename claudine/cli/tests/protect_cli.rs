use serde_json::Value;
mod common;
use common::{CliProcessFixture, write};

#[test]
fn handle_json_includes_protect_decisions() {
    let fixture = CliProcessFixture::named("claudine-protect-it");

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
        &fixture.home().join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    let payload = r#"{
      "hook_event_name":"PreToolUse",
      "session_id":"protect-session-1",
      "tool_name":"Bash",
      "tool_input":{"command":"rm -rf /tmp/protect-test"}
    }"#;

    let output = fixture
        .command()
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
