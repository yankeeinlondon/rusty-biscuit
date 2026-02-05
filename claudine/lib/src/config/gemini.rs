use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::error::Result;
use crate::events::{AgenticEvent, HookerConfig, Provider};

use super::atomic::atomic_write;
use super::backup::create_backup;
use super::trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

/// Name prefix used to identify Claudine-managed hooks in Gemini config.
const CLAUDINE_NAME_PREFIX: &str = "claudine-";

pub(crate) struct GeminiConfigurator;

impl AgentConfigurator for GeminiConfigurator {
    fn provider(&self) -> Provider {
        Provider::Gemini
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

        create_backup(&settings_path, Provider::Gemini)?;

        let content = fs::read_to_string(&settings_path)?;
        let mut root: Value = serde_json::from_str(&content)?;
        let hooks = root
            .as_object_mut()
            .unwrap()
            .entry("hooks")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap();

        let mut event_count = 0;
        for event in config.events.keys() {
            let native_name = to_gemini_native(event);
            let snake = event.to_string();
            let hook_entry = json!({
                "name": format!("{CLAUDINE_NAME_PREFIX}{snake}"),
                "command": format!("claudine handle {snake}"),
                "timeout": 30000,
                "description": format!("Claudine handler for {snake}")
            });

            let existing = hooks
                .entry(&native_name)
                .or_insert_with(|| json!([]));
            if let Some(arr) = existing.as_array_mut() {
                // Remove existing Claudine entries
                arr.retain(|entry| !is_claudine_hook(entry));
                arr.push(hook_entry);
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

        create_backup(&settings_path, Provider::Gemini)?;

        let content = fs::read_to_string(&settings_path)?;
        let mut root: Value = serde_json::from_str(&content)?;

        if let Some(hooks) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) {
            let keys: Vec<String> = hooks.keys().cloned().collect();
            for key in keys {
                if let Some(arr) = hooks.get_mut(&key).and_then(|v| v.as_array_mut()) {
                    arr.retain(|entry| !is_claudine_hook(entry));
                }
            }
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
            for (_, hook_list) in hooks {
                if let Some(arr) = hook_list.as_array()
                    && arr.iter().any(is_claudine_hook) {
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
            for (_, hook_list) in hooks {
                if let Some(arr) = hook_list.as_array() {
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

/// Map Claudine event names to Gemini CLI native hook names.
fn to_gemini_native(event: &AgenticEvent) -> String {
    match event {
        AgenticEvent::BeforePrompt => "BeforeAgent".to_string(),
        AgenticEvent::TurnComplete => "AfterAgent".to_string(),
        AgenticEvent::BeforeCompact => "PreCompress".to_string(),
        AgenticEvent::SessionStart => "SessionStart".to_string(),
        AgenticEvent::SessionEnd => "SessionEnd".to_string(),
        AgenticEvent::BeforeTool => "BeforeTool".to_string(),
        AgenticEvent::AfterTool => "AfterTool".to_string(),
        AgenticEvent::ToolError => "ToolError".to_string(),
        AgenticEvent::PermissionRequest => "PermissionRequest".to_string(),
        AgenticEvent::SubagentStart => "SubagentStart".to_string(),
        AgenticEvent::SubagentStop => "SubagentStop".to_string(),
        AgenticEvent::Notification => "Notification".to_string(),
        AgenticEvent::TurnError => "TurnError".to_string(),
        AgenticEvent::BeforeModel => "BeforeModel".to_string(),
        AgenticEvent::AfterModel => "AfterModel".to_string(),
    }
}

/// Check if a hook entry has a Claudine name prefix.
fn is_claudine_hook(entry: &Value) -> bool {
    entry
        .get("name")
        .and_then(|n| n.as_str())
        .is_some_and(|name| name.starts_with(CLAUDINE_NAME_PREFIX))
}

/// Extract the event name from a Claudine hook (e.g., "claudine-before_tool" -> "before_tool").
fn extract_claudine_event(entry: &Value) -> Option<String> {
    entry
        .get("name")
        .and_then(|n| n.as_str())
        .and_then(|name| name.strip_prefix(CLAUDINE_NAME_PREFIX))
        .map(|s| s.to_string())
}

/// Resolve the settings.json path for Gemini.
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("settings.json"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".gemini").join("settings.json")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use crate::events::{EventBinding, GlobalSettings};

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
                    overrides: HashMap::new(),
                },
            );
        }
        HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            events: event_map,
        }
    }

    #[test]
    fn register_adds_gemini_hooks() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

        let configurator = GeminiConfigurator;
        let config = test_config(vec![AgenticEvent::BeforePrompt, AgenticEvent::TurnComplete]);

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

        // Check Gemini native names
        assert!(hooks.contains_key("BeforeAgent"));
        assert!(hooks.contains_key("AfterAgent"));
    }

    #[test]
    fn gemini_hooks_use_millisecond_timeout_and_name() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

        let configurator = GeminiConfigurator;
        let config = test_config(vec![AgenticEvent::TurnComplete]);
        configurator.register(&config, Some(tmp.path())).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let hook = &content["hooks"]["AfterAgent"][0];

        // Verify ms timeout
        assert_eq!(hook["timeout"], 30000);
        // Verify name field present
        let name = hook["name"].as_str().unwrap();
        assert!(name.starts_with("claudine-"));
        // Verify description
        assert!(hook["description"].as_str().unwrap().contains("Claudine"));
    }

    #[test]
    fn deregister_removes_only_claudine_gemini_hooks() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        let initial = json!({
            "hooks": {
                "AfterAgent": [
                    {"name": "claudine-turn_complete", "command": "claudine handle turn_complete", "timeout": 30000},
                    {"name": "my-custom-hook", "command": "echo done", "timeout": 5000}
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

        let configurator = GeminiConfigurator;
        configurator.deregister(Some(tmp.path())).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let arr = content["hooks"]["AfterAgent"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "my-custom-hook");
    }

    #[test]
    fn is_registered_detects_gemini_hooks() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(
            &settings,
            r#"{"hooks": {"AfterAgent": [{"name": "claudine-turn_complete", "command": "claudine handle turn_complete"}]}}"#,
        )
        .unwrap();

        let configurator = GeminiConfigurator;
        assert!(configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn event_name_mapping() {
        assert_eq!(to_gemini_native(&AgenticEvent::BeforePrompt), "BeforeAgent");
        assert_eq!(to_gemini_native(&AgenticEvent::TurnComplete), "AfterAgent");
        assert_eq!(
            to_gemini_native(&AgenticEvent::BeforeCompact),
            "PreCompress"
        );
    }
}
