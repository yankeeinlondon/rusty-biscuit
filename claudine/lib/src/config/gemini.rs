use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::error::Result;
use crate::events::{HookerConfig, Provider};

use super::atomic::atomic_write;
use super::backup::create_backup;
use super::claudine_handle_command;
use super::trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

/// Name prefix used to identify Claudine-managed hooks in Gemini config.
const CLAUDINE_NAME_PREFIX: &str = "claudine-";

/// Minimal valid settings.json for Gemini CLI.
///
/// An empty JSON object is sufficient for Gemini CLI to function
/// and for hooks to be registered.
const MINIMAL_CONFIG: &str = "{}\n";

pub(crate) struct GeminiConfigurator;

impl AgentConfigurator for GeminiConfigurator {
    fn provider(&self) -> Provider {
        Provider::Gemini
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
        config: &HookerConfig,
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

        // Check if already in sync (same events registered as in config)
        if self.is_in_sync(config, config_dir)? {
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

        // Get events configured for this provider
        let provider_config = match config.providers.get(&Provider::Gemini) {
            Some(pc) => pc,
            None => {
                // No config for Gemini - remove all claudine hooks
                self.deregister(config_dir)?;
                return Ok(RegistrationResult::Registered { event_count: 0 });
            }
        };

        // Build set of native event names we want to keep
        let expected_natives: std::collections::HashSet<String> = provider_config
            .events
            .iter()
            .filter(|(_, binding)| binding.enabled)
            .filter_map(|(event, _)| {
                Provider::Gemini
                    .registration_native_event_name(event)
                    .map(str::to_string)
            })
            .collect();

        // First pass: remove claudine hooks for events NOT in config
        let hook_keys: Vec<String> = hooks.keys().cloned().collect();
        for native_name in hook_keys {
            if !expected_natives.contains(&native_name)
                && let Some(arr) = hooks.get_mut(&native_name).and_then(|v| v.as_array_mut())
            {
                arr.retain(|entry| !is_claudine_hook(entry));
            }
        }
        // Clean up empty hook arrays
        hooks.retain(|_, v| v.as_array().is_none_or(|a| !a.is_empty()));

        // Second pass: add/update hooks for events in config
        let handle_command = claudine_handle_command(Provider::Gemini);
        let mut event_count = 0;
        for (event, binding) in &provider_config.events {
            // Skip disabled events
            if !binding.enabled {
                continue;
            }
            // Skip events that Gemini doesn't support
            let Some(native_name) = Provider::Gemini.registration_native_event_name(event) else {
                continue;
            };

            let snake = event.to_string();
            let hook_entry = json!({
                "hooks": [{
                    "type": "command",
                    "name": format!("{CLAUDINE_NAME_PREFIX}{snake}"),
                    "command": handle_command(&snake),
                    "timeout": 30000,
                    "description": format!("Claudine handler for {snake}")
                }]
            });

            let existing = hooks
                .entry(native_name.to_string())
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
                    && arr.iter().any(is_claudine_hook)
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

impl GeminiConfigurator {
    /// Check if the registered hooks match the expected events from config.
    ///
    /// Returns false if any legacy flat-format entries exist, forcing
    /// re-registration to clean them up.
    fn is_in_sync(&self, config: &HookerConfig, config_dir: Option<&Path>) -> Result<bool> {
        use std::collections::HashSet;

        // Check for legacy flat-format entries that need cleanup
        if self.has_legacy_entries(config_dir)? {
            return Ok(false);
        }

        let registered: HashSet<String> = self.registered_events(config_dir)?.into_iter().collect();

        let expected: HashSet<String> = config
            .providers
            .get(&Provider::Gemini)
            .map(|p| {
                p.events
                    .iter()
                    .filter(|(event, binding)| {
                        binding.enabled
                            && Provider::Gemini
                                .registration_native_event_name(event)
                                .is_some()
                    })
                    .map(|(event, _)| event.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(registered == expected)
    }

    /// Check if any claudine hook entries use the legacy flat format
    /// (missing nested `hooks` array).
    fn has_legacy_entries(&self, config_dir: Option<&Path>) -> Result<bool> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&settings_path)?;
        let root: Value = serde_json::from_str(&content)?;

        if let Some(hooks) = root.get("hooks").and_then(|h| h.as_object()) {
            for (_, hook_list) in hooks {
                if let Some(arr) = hook_list.as_array() {
                    for entry in arr {
                        // Legacy format: has claudine name at top level (no nested hooks array)
                        if entry.get("hooks").is_none() && is_claudine_hook(entry) {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }
}

/// Check if a hook definition contains a Claudine-managed hook.
///
/// Detects both the correct nested format (`{hooks: [{name: "claudine-..."}]}`)
/// and the legacy flat format (`{name: "claudine-..."}`) to ensure old entries
/// get cleaned up during re-registration.
fn is_claudine_hook(entry: &Value) -> bool {
    let has_claudine_name = |v: &Value| {
        v.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|name| name.starts_with(CLAUDINE_NAME_PREFIX))
    };

    // Nested format: {hooks: [{name: "claudine-..."}]}
    if let Some(hooks) = entry.get("hooks").and_then(|h| h.as_array()) {
        if hooks.iter().any(has_claudine_name) {
            return true;
        }
    }

    // Legacy flat format: {name: "claudine-..."}
    has_claudine_name(entry)
}

/// Extract the event name from a Claudine hook definition.
///
/// Supports both nested (`{hooks: [{name: "claudine-..."}]}`) and legacy flat
/// (`{name: "claudine-..."}`) formats.
fn extract_claudine_event(entry: &Value) -> Option<String> {
    let extract_from = |v: &Value| {
        v.get("name")
            .and_then(|n| n.as_str())
            .and_then(|name| name.strip_prefix(CLAUDINE_NAME_PREFIX))
            .map(|s| s.to_string())
    };

    // Try nested format first
    if let Some(hooks) = entry.get("hooks").and_then(|h| h.as_array()) {
        if let Some(event) = hooks.iter().find_map(extract_from) {
            return Some(event);
        }
    }

    // Fall back to legacy flat format
    extract_from(entry)
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

    use crate::events::{AgenticEvent, EventBinding, GlobalSettings, ProviderConfig};

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
        providers.insert(Provider::Gemini, ProviderConfig { events: event_map });
        HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers,
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

        let content: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let hooks = content.get("hooks").unwrap().as_object().unwrap();

        // Check Gemini native names
        assert!(hooks.contains_key("BeforeAgent"));
        assert!(hooks.contains_key("AfterAgent"));
    }

    #[test]
    fn gemini_hooks_use_nested_structure_with_type() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

        let configurator = GeminiConfigurator;
        let config = test_config(vec![AgenticEvent::TurnComplete]);
        configurator.register(&config, Some(tmp.path())).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let definition = &content["hooks"]["AfterAgent"][0];

        // Verify nested hooks array structure
        let inner_hook = &definition["hooks"][0];
        assert_eq!(inner_hook["type"], "command");
        assert_eq!(inner_hook["timeout"], 30000);
        let name = inner_hook["name"].as_str().unwrap();
        assert!(name.starts_with("claudine-"));
        assert!(
            inner_hook["description"]
                .as_str()
                .unwrap()
                .contains("Claudine")
        );
    }

    #[test]
    fn deregister_removes_only_claudine_gemini_hooks() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        let initial = json!({
            "hooks": {
                "AfterAgent": [
                    {"hooks": [{"type": "command", "name": "claudine-turn_complete", "command": "claudine handle turn_complete --provider gemini", "timeout": 30000}]},
                    {"hooks": [{"type": "command", "name": "my-custom-hook", "command": "echo done", "timeout": 5000}]}
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

        let configurator = GeminiConfigurator;
        configurator.deregister(Some(tmp.path())).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let arr = content["hooks"]["AfterAgent"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["hooks"][0]["name"], "my-custom-hook");
    }

    #[test]
    fn is_registered_detects_gemini_hooks() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(
            &settings,
            r#"{"hooks": {"AfterAgent": [{"hooks": [{"type": "command", "name": "claudine-turn_complete", "command": "claudine handle turn_complete --provider gemini"}]}]}}"#,
        )
        .unwrap();

        let configurator = GeminiConfigurator;
        assert!(configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn event_name_mapping() {
        assert_eq!(
            Provider::Gemini.registration_native_event_name(&AgenticEvent::BeforePrompt),
            Some("BeforeAgent")
        );
        assert_eq!(
            Provider::Gemini.registration_native_event_name(&AgenticEvent::TurnComplete),
            Some("AfterAgent")
        );
        assert_eq!(
            Provider::Gemini.registration_native_event_name(&AgenticEvent::BeforeCompact),
            Some("PreCompress")
        );
    }

    #[test]
    fn unsupported_events_return_none() {
        assert_eq!(
            Provider::Gemini.registration_native_event_name(&AgenticEvent::ToolError),
            None
        );
        assert_eq!(
            Provider::Gemini.registration_native_event_name(&AgenticEvent::PermissionRequest),
            None
        );
        assert_eq!(
            Provider::Gemini.registration_native_event_name(&AgenticEvent::TurnError),
            None
        );
        assert_eq!(
            Provider::Gemini.registration_native_event_name(&AgenticEvent::SubagentStart),
            None
        );
        assert_eq!(
            Provider::Gemini.registration_native_event_name(&AgenticEvent::SubagentStop),
            None
        );
    }

    #[test]
    fn re_register_adds_new_events() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

        let configurator = GeminiConfigurator;

        // First register with one event
        let config1 = test_config(vec![AgenticEvent::BeforePrompt]);
        configurator.register(&config1, Some(tmp.path())).unwrap();

        // Re-register with two events
        let config2 = test_config(vec![AgenticEvent::BeforePrompt, AgenticEvent::TurnComplete]);
        let result = configurator.register(&config2, Some(tmp.path())).unwrap();

        match result {
            RegistrationResult::Registered { event_count } => {
                assert_eq!(event_count, 2);
            }
            _ => panic!("expected Registered"),
        }

        let events = configurator.registered_events(Some(tmp.path())).unwrap();
        assert!(events.contains(&"before_prompt".to_string()));
        assert!(events.contains(&"turn_complete".to_string()));
    }

    #[test]
    fn re_register_removes_stale_events() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"hooks": {}}"#).unwrap();

        let configurator = GeminiConfigurator;

        // First register with two events
        let config1 = test_config(vec![AgenticEvent::BeforePrompt, AgenticEvent::TurnComplete]);
        configurator.register(&config1, Some(tmp.path())).unwrap();

        // Re-register with only one event
        let config2 = test_config(vec![AgenticEvent::BeforePrompt]);
        configurator.register(&config2, Some(tmp.path())).unwrap();

        // Verify only before_prompt remains
        let events = configurator.registered_events(Some(tmp.path())).unwrap();
        assert_eq!(events, vec!["before_prompt"]);
    }

    #[test]
    fn register_handles_missing_config() {
        // When config doesn't exist:
        // - If CLI is installed → creates minimal config and registers hooks
        // - If CLI is not installed → returns NotDetected
        let tmp = TempDir::new().unwrap();
        let configurator = GeminiConfigurator;
        let config = test_config(vec![AgenticEvent::BeforePrompt]);

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
        let configurator = GeminiConfigurator;

        configurator
            .create_minimal_config(Some(tmp.path()))
            .unwrap();

        let settings_path = tmp.path().join("settings.json");
        assert!(settings_path.exists());

        // Verify it's valid JSON
        let content = fs::read_to_string(&settings_path).unwrap();
        let _json: serde_json::Value =
            serde_json::from_str(&content).expect("Should be valid JSON");
    }

    #[test]
    fn create_minimal_config_creates_parent_directory() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("nested").join("dir");
        let configurator = GeminiConfigurator;

        configurator.create_minimal_config(Some(&nested)).unwrap();

        assert!(nested.join("settings.json").exists());
    }
}
