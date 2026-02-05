use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::error::Result;
use crate::events::{AgenticEvent, HookerConfig, Provider};

use super::atomic::atomic_write;
use super::backup::create_backup;
use super::claudine_command;
use super::trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

pub(crate) struct ClaudeConfigurator;

impl AgentConfigurator for ClaudeConfigurator {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn register(
        &self,
        config: &HookerConfig,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(RegistrationResult::Skipped(SkipReason::NotDetected));
        }

        if self.is_registered(config_dir)? {
            return Ok(RegistrationResult::Skipped(SkipReason::AlreadyRegistered));
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

        // Get events configured for this provider
        let provider_config = match config.providers.get(&Provider::Claude) {
            Some(pc) => pc,
            None => return Ok(RegistrationResult::Registered { event_count: 0 }),
        };

        let claudine_bin = claudine_command();
        let mut event_count = 0;
        for event in provider_config.events.keys() {
            // Skip events that Claude Code doesn't support
            let Some(native_name) = to_claude_native(event) else {
                continue;
            };

            let snake = event.to_string();
            let hook_entry = json!([{
                "hooks": [{
                    "type": "command",
                    "command": format!("{claudine_bin} handle {snake}"),
                    "timeout": 30
                }]
            }]);

            // Merge with existing hooks for this event
            let existing = hooks.entry(&native_name).or_insert_with(|| json!([]));
            if let Some(arr) = existing.as_array_mut() {
                // Remove any existing Claudine entries first
                arr.retain(|entry| !is_claudine_hook_group(entry));
                // Add new Claudine entry
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
            hooks.retain(|_, v| {
                v.as_array().is_none_or(|a| !a.is_empty())
            });
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
                    && arr.iter().any(is_claudine_hook_group) {
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

/// Map Claudine event names to Claude Code native hook names.
///
/// Returns `None` for events that Claude Code doesn't support natively.
/// Unsupported events: BeforeModel, AfterModel, TurnError.
fn to_claude_native(event: &AgenticEvent) -> Option<String> {
    match event {
        AgenticEvent::BeforeTool => Some("PreToolUse".to_string()),
        AgenticEvent::AfterTool => Some("PostToolUse".to_string()),
        AgenticEvent::ToolError => Some("PostToolUseFailure".to_string()),
        AgenticEvent::BeforePrompt => Some("UserPromptSubmit".to_string()),
        AgenticEvent::TurnComplete => Some("Stop".to_string()),
        AgenticEvent::BeforeCompact => Some("PreCompact".to_string()),
        AgenticEvent::SessionStart => Some("SessionStart".to_string()),
        AgenticEvent::SessionEnd => Some("SessionEnd".to_string()),
        AgenticEvent::PermissionRequest => Some("PermissionRequest".to_string()),
        AgenticEvent::SubagentStart => Some("SubagentStart".to_string()),
        AgenticEvent::SubagentStop => Some("SubagentStop".to_string()),
        AgenticEvent::Notification => Some("Notification".to_string()),
        // Unsupported events - Claude Code doesn't have these hook types
        AgenticEvent::BeforeModel | AgenticEvent::AfterModel | AgenticEvent::TurnError => None,
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
/// Handles both short form ("claudine handle before_tool") and full path
/// ("/path/to/claudine handle before_tool") -> "before_tool".
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
    use std::collections::HashMap;

    use tempfile::TempDir;

    use crate::events::{EventBinding, GlobalSettings, ProviderConfig};

    use super::*;

    fn test_config(events: Vec<AgenticEvent>) -> HookerConfig {
        let mut event_map = HashMap::new();
        for event in events {
            event_map.insert(
                event,
                EventBinding {
                    enabled: true,
                    actions: vec![],
                    matcher: None,
                },
            );
        }
        let mut providers = HashMap::new();
        providers.insert(
            Provider::Claude,
            ProviderConfig { events: event_map },
        );
        HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers,
        }
    }

    #[test]
    fn register_adds_hooks() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

        let configurator = ClaudeConfigurator;
        let config = test_config(vec![AgenticEvent::BeforeTool, AgenticEvent::TurnComplete]);

        let result = configurator.register(&config, Some(tmp.path())).unwrap();
        match result {
            RegistrationResult::Registered { event_count } => {
                assert_eq!(event_count, 2);
            }
            _ => panic!("expected Registered"),
        }

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
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
    fn register_skips_when_not_detected() {
        let tmp = TempDir::new().unwrap();
        let configurator = ClaudeConfigurator;
        let config = test_config(vec![AgenticEvent::BeforeTool]);

        let result = configurator.register(&config, Some(tmp.path())).unwrap();
        assert!(matches!(result, RegistrationResult::Skipped(SkipReason::NotDetected)));
    }

    #[test]
    fn register_skips_when_already_registered() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(
            &settings,
            r#"{"hooks": {"PreToolUse": [{"hooks": [{"type": "command", "command": "claudine handle before_tool", "timeout": 30}]}]}}"#,
        )
        .unwrap();

        let configurator = ClaudeConfigurator;
        let config = test_config(vec![AgenticEvent::BeforeTool]);

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
                    {"hooks": [{"type": "command", "command": "claudine handle before_tool", "timeout": 30}]},
                    {"hooks": [{"type": "command", "command": "other-tool check", "timeout": 10}]}
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

        let configurator = ClaudeConfigurator;
        configurator.deregister(Some(tmp.path())).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
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
                    {"hooks": [{"type": "command", "command": "claudine handle before_tool", "timeout": 30}]}
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

        let configurator = ClaudeConfigurator;
        configurator.deregister(Some(tmp.path())).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let hooks = content.get("hooks").unwrap().as_object().unwrap();
        assert!(!hooks.contains_key("PreToolUse"));
    }

    #[test]
    fn is_registered_detects_claudine_hooks() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(
            &settings,
            r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "claudine handle turn_complete", "timeout": 30}]}]}}"#,
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
        let config = test_config(vec![AgenticEvent::TurnComplete]);
        configurator.register(&config, Some(tmp.path())).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        // Original field preserved
        assert!(content.get("permissions").is_some());
    }

    #[test]
    fn event_name_mapping() {
        assert_eq!(
            to_claude_native(&AgenticEvent::BeforeTool),
            Some("PreToolUse".to_string())
        );
        assert_eq!(
            to_claude_native(&AgenticEvent::AfterTool),
            Some("PostToolUse".to_string())
        );
        assert_eq!(
            to_claude_native(&AgenticEvent::ToolError),
            Some("PostToolUseFailure".to_string())
        );
        assert_eq!(
            to_claude_native(&AgenticEvent::BeforePrompt),
            Some("UserPromptSubmit".to_string())
        );
        assert_eq!(
            to_claude_native(&AgenticEvent::TurnComplete),
            Some("Stop".to_string())
        );
        assert_eq!(
            to_claude_native(&AgenticEvent::BeforeCompact),
            Some("PreCompact".to_string())
        );
        assert_eq!(
            to_claude_native(&AgenticEvent::SessionStart),
            Some("SessionStart".to_string())
        );
    }

    #[test]
    fn unsupported_events_return_none() {
        assert_eq!(to_claude_native(&AgenticEvent::BeforeModel), None);
        assert_eq!(to_claude_native(&AgenticEvent::AfterModel), None);
        assert_eq!(to_claude_native(&AgenticEvent::TurnError), None);
    }
}
