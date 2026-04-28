use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::error::Result;
use crate::provider::Provider;
use super::atomic::atomic_write;
use super::backup::create_backup;
use super::claudine_handle_command;
use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};

pub(crate) struct ClaudeConfigurator;

/// Minimal valid settings.json for Claude Code.
///
/// An empty object with just the schema URL is sufficient for Claude Code
/// to function and for hooks to be registered.
const MINIMAL_CONFIG: &str = r#"{
  "$schema": "https://json.schemastore.org/claude-code-settings.json"
}
"#;

impl AgentConfigurator for ClaudeConfigurator {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn create_minimal_config(&self, config_dir: Option<&Path>) -> crate::error::Result<()> {
        let settings_path = config_path(config_dir);

        // Create parent directory if needed
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent)?;
        }

        atomic_write(&settings_path, MINIMAL_CONFIG.as_bytes())?;
        Ok(())
    }

    fn register(
        &self,
        plan: &ProviderHookPlan,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        let settings_path = config_path(config_dir);

        // If config doesn't exist but CLI is installed, create minimal config
        if !settings_path.exists() {
            if self.is_cli_installed() {
                self.create_minimal_config(config_dir)?;
            } else {
                return Ok(RegistrationResult::Skipped(SkipReason::NotDetected));
            }
        }

        // Check if already in sync (same events registered as in plan)
        if self.is_in_sync(plan, config_dir)? {
            return Ok(RegistrationResult::Skipped(SkipReason::AlreadyRegistered));
        }

        if plan.events.is_empty() {
            self.deregister(config_dir)?;
            return Ok(RegistrationResult::Registered { event_count: 0 });
        }

        create_backup(&settings_path, Provider::Claude)?;

        let content = fs::read_to_string(&settings_path)?;
        let mut root: Value = serde_json::from_str(&content)?;
        let hooks = root
            .as_object_mut()
            .unwrap()
            .entry("hooks")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap();

        // Build set of native event names we want to keep
        let expected_natives: std::collections::HashSet<String> = plan
            .events
            .iter()
            .filter_map(|event| {
                Provider::Claude
                    .registration_native_event_name(event)
                    .map(str::to_string)
            })
            .collect();

        // First pass: remove claudine hooks for events NOT in plan
        let hook_keys: Vec<String> = hooks.keys().cloned().collect();
        for native_name in hook_keys {
            if !expected_natives.contains(&native_name)
                && let Some(arr) = hooks.get_mut(&native_name).and_then(|v| v.as_array_mut())
            {
                arr.retain(|entry| !is_claudine_hook_group(entry));
            }
        }
        // Clean up empty hook arrays
        hooks.retain(|_, v| v.as_array().is_none_or(|a| !a.is_empty()));

        // Second pass: add/update hooks for events in plan
        let handle_command = claudine_handle_command(Provider::Claude);
        let mut event_count = 0;
        for event in &plan.events {
            // Skip events that Claude Code doesn't support
            let Some(native_name) = Provider::Claude.registration_native_event_name(event) else {
                continue;
            };

            let snake = event.to_string();
            let hook_entry = json!([{
                "hooks": [{
                    "type": "command",
                    "command": handle_command(&snake),
                    "timeout": 30
                }]
            }]);

            // Merge with existing hooks for this event
            let existing = hooks
                .entry(native_name.to_string())
                .or_insert_with(|| json!([]));
            if let Some(arr) = existing.as_array_mut() {
                arr.retain(|entry| !is_claudine_hook_group(entry));
                if let Some(new_arr) = hook_entry.as_array() {
                    arr.extend(new_arr.iter().cloned());
                }
            }
            event_count += 1;
        }

        let output = serde_json::to_string_pretty(&root)?;
        atomic_write(&settings_path, output.as_bytes())?;

        Ok(RegistrationResult::Registered { event_count })
    }

    fn deregister(&self, config_dir: Option<&Path>) -> Result<()> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(());
        }

        create_backup(&settings_path, Provider::Claude)?;

        let content = fs::read_to_string(&settings_path)?;
        let mut root: Value = serde_json::from_str(&content)?;

        if let Some(hooks) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) {
            let keys: Vec<String> = hooks.keys().cloned().collect();
            for key in keys {
                if let Some(arr) = hooks.get_mut(&key).and_then(|v| v.as_array_mut()) {
                    arr.retain(|entry| !is_claudine_hook_group(entry));
                }
            }
            // Remove empty hook arrays
            hooks.retain(|_, v| v.as_array().is_none_or(|a| !a.is_empty()));
        }

        let output = serde_json::to_string_pretty(&root)?;
        atomic_write(&settings_path, output.as_bytes())?;

        Ok(())
    }

    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&settings_path)?;
        let root: Value = serde_json::from_str(&content)?;

        if let Some(hooks) = root.get("hooks").and_then(|h| h.as_object()) {
            for (_, hook_groups) in hooks {
                if let Some(arr) = hook_groups.as_array()
                    && arr.iter().any(is_claudine_hook_group)
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn registered_events(&self, config_dir: Option<&Path>) -> Result<Vec<String>> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&settings_path)?;
        let root: Value = serde_json::from_str(&content)?;

        let mut events = Vec::new();
        if let Some(hooks) = root.get("hooks").and_then(|h| h.as_object()) {
            for (_, hook_groups) in hooks {
                if let Some(arr) = hook_groups.as_array() {
                    for entry in arr {
                        if let Some(event_name) = extract_claudine_event(entry) {
                            events.push(event_name);
                        }
                    }
                }
            }
        }

        events.sort();
        events.dedup();
        Ok(events)
    }
}

impl ClaudeConfigurator {
    /// Check if the registered hooks match the expected events from the plan.
    ///
    /// Returns true only if both sets contain exactly the same events.
    fn is_in_sync(&self, plan: &ProviderHookPlan, config_dir: Option<&Path>) -> Result<bool> {
        use std::collections::HashSet;

        let registered: HashSet<String> = self.registered_events(config_dir)?.into_iter().collect();

        let expected: HashSet<String> = plan
            .events
            .iter()
            .filter(|event| {
                Provider::Claude
                    .registration_native_event_name(event)
                    .is_some()
            })
            .map(|event| event.to_string())
            .collect();

        Ok(registered == expected)
    }
}

/// Check if a command string is a Claudine-managed hook.
///
/// Matches both short form ("claudine handle ...") and full path
/// ("/path/to/claudine handle ...").
fn is_claudine_command(cmd: &str) -> bool {
    // Check for "claudine handle" anywhere (covers full paths)
    cmd.contains("claudine handle") || cmd.contains("claudine ")
}

/// Check if a hook group entry contains a Claudine-managed command.
fn is_claudine_hook_group(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .is_some_and(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(is_claudine_command)
            })
        })
}

/// Extract the event name from a Claudine hook entry.
///
/// Handles both short form ("claudine handle before_tool --provider claude")
/// and full path ("/path/to/claudine handle before_tool --provider claude")
/// -> "before_tool".
fn extract_claudine_event(entry: &Value) -> Option<String> {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .and_then(|hooks| {
            hooks.iter().find_map(|hook| {
                hook.get("command")
                    .and_then(|c| c.as_str())
                    .and_then(|cmd| {
                        // Find "claudine handle " and extract event after it
                        if let Some(idx) = cmd.find("claudine handle ") {
                            let after_handle = &cmd[idx + "claudine handle ".len()..];
                            let event = after_handle.split_whitespace().next()?;
                            return Some(event.to_string());
                        }
                        // Fallback for old format "claudine <subcommand> <event>"
                        if let Some(idx) = cmd.find("claudine ") {
                            let after_claudine = &cmd[idx + "claudine ".len()..];
                            // Skip subcommand, get event
                            let mut parts = after_claudine.split_whitespace();
                            parts.next(); // skip subcommand
                            return parts.next().map(|s| s.to_string());
                        }
                        None
                    })
            })
        })
}

/// Resolve the settings.json path, using override dir for tests.
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("settings.json"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".claude").join("settings.json")
        }
    }
}

#[cfg(test)]
mod tests {
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
        use crate::provider::EventSupportLevel;
        // HumanInTheLoop is captured via PreToolUse hook with AskUserQuestion tool matcher
        assert_eq!(
            Provider::Claude.event_support_level(&AgenticEvent::HumanInTheLoop),
            EventSupportLevel::Hook
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
}
