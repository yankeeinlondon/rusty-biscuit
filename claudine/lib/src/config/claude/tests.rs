use tempfile::TempDir;

use crate::events::AgenticEvent;

use super::*;

fn test_plan(events: Vec<AgenticEvent>) -> ProviderHookPlan {
    ProviderHookPlan {
        events,
        canonical_for: None,
    }
}

#[test]
fn register_adds_hooks() {
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

    let configurator = ClaudeConfigurator;
    let config = test_plan(vec![AgenticEvent::BeforeTool, AgenticEvent::TurnComplete]);

    let result = configurator.register(&config, Some(tmp.path())).unwrap();
    match result {
        RegistrationResult::Registered { event_count } => {
            assert_eq!(event_count, 2);
        }
        _ => panic!("expected Registered"),
    }

    let content: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    let hooks = content.get("hooks").unwrap().as_object().unwrap();

    // Check mapped names
    assert!(hooks.contains_key("PreToolUse"));
    assert!(hooks.contains_key("Stop"));

    // Verify command content (may be full path or just "claudine")
    let pre_tool = &hooks["PreToolUse"];
    let cmd = pre_tool[0]["hooks"][0]["command"].as_str().unwrap();
    assert!(cmd.contains("claudine") && cmd.contains("handle"));
    assert!(cmd.contains("before_tool"));
}

#[test]
fn register_handles_missing_config() {
    // When config doesn't exist:
    // - If CLI is installed → creates minimal config and registers hooks
    // - If CLI is not installed → returns NotDetected
    let tmp = TempDir::new().unwrap();
    let configurator = ClaudeConfigurator;
    let config = test_plan(vec![AgenticEvent::BeforeTool]);

    let result = configurator.register(&config, Some(tmp.path())).unwrap();

    if configurator.is_cli_installed() {
        // CLI installed: should create config and register
        assert!(
            matches!(result, RegistrationResult::Registered { event_count: 1 }),
            "Expected Registered when CLI is installed, got {:?}",
            result
        );
        // Verify config was created
        let settings_path = tmp.path().join("settings.json");
        assert!(settings_path.exists(), "Config file should be created");
    } else {
        // CLI not installed: should skip
        assert!(
            matches!(result, RegistrationResult::Skipped(SkipReason::NotDetected)),
            "Expected NotDetected when CLI is not installed, got {:?}",
            result
        );
    }
}

#[test]
fn create_minimal_config_creates_valid_json() {
    let tmp = TempDir::new().unwrap();
    let configurator = ClaudeConfigurator;

    configurator
        .create_minimal_config(Some(tmp.path()))
        .unwrap();

    let settings_path = tmp.path().join("settings.json");
    assert!(settings_path.exists());

    // Verify it's valid JSON and has the schema
    let content = fs::read_to_string(&settings_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(
        json.get("$schema")
            .and_then(|s| s.as_str())
            .is_some_and(|s| s.contains("claude-code-settings"))
    );
}

#[test]
fn create_minimal_config_creates_parent_directory() {
    let tmp = TempDir::new().unwrap();
    let nested = tmp.path().join("nested").join("dir");
    let configurator = ClaudeConfigurator;

    configurator.create_minimal_config(Some(&nested)).unwrap();

    assert!(nested.join("settings.json").exists());
}

#[test]
fn register_skips_when_already_registered() {
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    fs::write(
        &settings,
        r#"{"hooks": {"PreToolUse": [{"hooks": [{"type": "command", "command": "claudine handle before_tool --provider claude", "timeout": 30}]}]}}"#,
    )
    .unwrap();

    let configurator = ClaudeConfigurator;
    let config = test_plan(vec![AgenticEvent::BeforeTool]);

    let result = configurator.register(&config, Some(tmp.path())).unwrap();
    assert!(matches!(
        result,
        RegistrationResult::Skipped(SkipReason::AlreadyRegistered)
    ));
}

#[test]
fn deregister_removes_only_claudine_hooks() {
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    let initial = json!({
        "hooks": {
            "PreToolUse": [
                {"hooks": [{"type": "command", "command": "claudine handle before_tool --provider claude", "timeout": 30}]},
                {"hooks": [{"type": "command", "command": "other-tool check", "timeout": 10}]}
            ]
        }
    });
    fs::write(&settings, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

    let configurator = ClaudeConfigurator;
    configurator.deregister(Some(tmp.path())).unwrap();

    let content: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    let hooks = content.get("hooks").unwrap().as_object().unwrap();
    let pre_tool = hooks["PreToolUse"].as_array().unwrap();

    // Only the non-Claudine hook remains
    assert_eq!(pre_tool.len(), 1);
    let cmd = pre_tool[0]["hooks"][0]["command"].as_str().unwrap();
    assert_eq!(cmd, "other-tool check");
}

#[test]
fn deregister_removes_empty_hook_arrays() {
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    let initial = json!({
        "hooks": {
            "PreToolUse": [
                {"hooks": [{"type": "command", "command": "claudine handle before_tool --provider claude", "timeout": 30}]}
            ]
        }
    });
    fs::write(&settings, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

    let configurator = ClaudeConfigurator;
    configurator.deregister(Some(tmp.path())).unwrap();

    let content: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    let hooks = content.get("hooks").unwrap().as_object().unwrap();
    assert!(!hooks.contains_key("PreToolUse"));
}

#[test]
fn is_registered_detects_claudine_hooks() {
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    fs::write(
        &settings,
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "claudine handle turn_complete --provider claude", "timeout": 30}]}]}}"#,
    )
    .unwrap();

    let configurator = ClaudeConfigurator;
    assert!(configurator.is_registered(Some(tmp.path())).unwrap());
}

#[test]
fn is_registered_false_without_claudine() {
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    fs::write(
        &settings,
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "other-tool stop"}]}]}}"#,
    )
    .unwrap();

    let configurator = ClaudeConfigurator;
    assert!(!configurator.is_registered(Some(tmp.path())).unwrap());
}

#[test]
fn preserves_existing_json_fields() {
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    fs::write(
        &settings,
        r#"{"permissions": {"allow": ["Read"]}, "hooks": {}}"#,
    )
    .unwrap();

    let configurator = ClaudeConfigurator;
    let config = test_plan(vec![AgenticEvent::TurnComplete]);
    configurator.register(&config, Some(tmp.path())).unwrap();

    let content: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    // Original field preserved
    assert!(content.get("permissions").is_some());
}

#[test]
fn event_name_mapping() {
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::BeforeTool),
        Some("PreToolUse")
    );
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::AfterTool),
        Some("PostToolUse")
    );
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::ToolError),
        Some("PostToolUseFailure")
    );
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::BeforePrompt),
        Some("UserPromptSubmit")
    );
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::TurnComplete),
        Some("Stop")
    );
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::BeforeCompact),
        Some("PreCompact")
    );
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::SessionStart),
        Some("SessionStart")
    );
}

#[test]
fn unsupported_events_return_none() {
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::BeforeModel),
        None
    );
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::AfterModel),
        None
    );
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::TurnError),
        None
    );
    // HumanInTheLoop is derived from PreToolUse, not a standalone hook
    assert_eq!(
        Provider::Claude.registration_native_event_name(&AgenticEvent::HumanInTheLoop),
        None
    );
}

#[test]
fn human_in_the_loop_supported_via_hook_for_claude() {
    // HumanInTheLoop is captured via PreToolUse hook with AskUserQuestion tool matcher
    assert!(
        Provider::Claude
            .event_support_level(&AgenticEvent::HumanInTheLoop)
            .is_hook()
    );
}

#[test]
fn re_register_adds_new_events() {
    // Regression test: re-running init with more events should register them
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

    let configurator = ClaudeConfigurator;

    // First register with one event
    let config1 = test_plan(vec![AgenticEvent::BeforeTool]);
    configurator.register(&config1, Some(tmp.path())).unwrap();

    // Verify initial state
    let events = configurator.registered_events(Some(tmp.path())).unwrap();
    assert_eq!(events, vec!["before_tool"]);

    // Re-register with two events
    let config2 = test_plan(vec![AgenticEvent::BeforeTool, AgenticEvent::TurnComplete]);
    let result = configurator.register(&config2, Some(tmp.path())).unwrap();

    // Should register (not skip)
    match result {
        RegistrationResult::Registered { event_count } => {
            assert_eq!(event_count, 2);
        }
        _ => panic!("expected Registered, got AlreadyRegistered"),
    }

    // Verify both events are now registered
    let events = configurator.registered_events(Some(tmp.path())).unwrap();
    assert!(events.contains(&"before_tool".to_string()));
    assert!(events.contains(&"turn_complete".to_string()));
}

#[test]
fn re_register_removes_stale_events() {
    // Regression test: removing events from config should remove them from settings
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

    let configurator = ClaudeConfigurator;

    // First register with two events
    let config1 = test_plan(vec![AgenticEvent::BeforeTool, AgenticEvent::TurnComplete]);
    configurator.register(&config1, Some(tmp.path())).unwrap();

    // Verify initial state
    let events = configurator.registered_events(Some(tmp.path())).unwrap();
    assert_eq!(events.len(), 2);

    // Re-register with only one event
    let config2 = test_plan(vec![AgenticEvent::BeforeTool]);
    let result = configurator.register(&config2, Some(tmp.path())).unwrap();

    // Should register (not skip)
    match result {
        RegistrationResult::Registered { event_count } => {
            assert_eq!(event_count, 1);
        }
        _ => panic!("expected Registered"),
    }

    // Verify only before_tool remains
    let events = configurator.registered_events(Some(tmp.path())).unwrap();
    assert_eq!(events, vec!["before_tool"]);

    // Verify Stop hook was removed from JSON
    let content: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    let hooks = content.get("hooks").unwrap().as_object().unwrap();
    assert!(!hooks.contains_key("Stop"));
}

#[test]
fn already_in_sync_skips_registration() {
    // Verify that identical config doesn't cause unnecessary writes
    let tmp = TempDir::new().unwrap();
    let settings = tmp.path().join("settings.json");
    fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

    let configurator = ClaudeConfigurator;

    // First register
    let config = test_plan(vec![AgenticEvent::BeforeTool]);
    configurator.register(&config, Some(tmp.path())).unwrap();

    // Re-register with identical config
    let result = configurator.register(&config, Some(tmp.path())).unwrap();

    assert!(matches!(
        result,
        RegistrationResult::Skipped(SkipReason::AlreadyRegistered)
    ));
}
