use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::atomic::atomic_write;
use super::backup::create_backup;
use super::claudine_handle_command;
use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};
use crate::error::Result;
use crate::provider::Provider;

/// Top-level named-hook key Claudine owns in agy's `hooks.json`.
///
/// agy's hook config is `{ "<hook-name>": JSONHookSpec }` — a map from an
/// arbitrary hook name to a spec carrying per-event handler arrays. Claudine
/// owns exactly this one key, so registration is a whole-key replace and other
/// named hooks (user/plugin) are left untouched.
const CLAUDINE_HOOK_NAME: &str = "claudine";

/// Per-handler timeout in **seconds** (agy's unit; default 30).
const HANDLER_TIMEOUT_SECS: u64 = 30;

/// Minimal valid hooks.json (an empty object — no named hooks yet).
const MINIMAL_CONFIG: &str = "{}\n";

/// Configurator for Antigravity.
///
/// agy has a real file-based hook system whose subsystem loads during a
/// `--print` run (verified against agy 1.1.0: `hooks_manager.go` parses the
/// config live). Claudine registers command handlers into the global
/// `~/.gemini/config/hooks.json` under the single [`CLAUDINE_HOOK_NAME`] key,
/// using agy's schema: `PreToolUse`/`PostToolUse` take matcher-groups
/// (`[{matcher, hooks:[handler]}]`), while `PreInvocation`/`PostInvocation`/
/// `Stop` take direct handler arrays (`[handler]`). Each handler is
/// `{type: command, command: "claudine handle <event> --provider antigravity",
/// timeout: 30}`.
pub(crate) struct AntigravityConfigurator;

impl AgentConfigurator for AntigravityConfigurator {
    fn provider(&self) -> Provider {
        Provider::Antigravity
    }

    fn create_minimal_config(&self, config_dir: Option<&Path>) -> Result<()> {
        let path = config_path(config_dir);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&path, MINIMAL_CONFIG.as_bytes())?;
        Ok(())
    }

    fn register(
        &self,
        plan: &ProviderHookPlan,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        let path = config_path(config_dir);

        if !path.exists() {
            if self.is_cli_installed() {
                self.create_minimal_config(config_dir)?;
            } else {
                return Ok(RegistrationResult::Skipped(SkipReason::NotDetected));
            }
        }

        if plan.events.is_empty() {
            self.deregister(config_dir)?;
            return Ok(RegistrationResult::Registered { event_count: 0 });
        }

        create_backup(&path, Provider::Antigravity)?;

        let content = fs::read_to_string(&path)?;
        let mut root: Value = serde_json::from_str(&content).unwrap_or_else(|_| json!({}));
        if !root.is_object() {
            root = json!({});
        }
        let root_obj = root.as_object_mut().expect("root coerced to a JSON object");

        let handle_command = claudine_handle_command(Provider::Antigravity);
        let mut spec = serde_json::Map::new();
        let mut event_count = 0;
        for event in &plan.events {
            // Map the Claudine event to agy's native hook event via the catalog.
            let Some(native) = Provider::Antigravity.registration_native_event_name(event) else {
                continue;
            };
            let handler = json!({
                "type": "command",
                "command": handle_command(&event.to_string()),
                "timeout": HANDLER_TIMEOUT_SECS,
            });
            // PreToolUse/PostToolUse are matcher-scoped groups; the loop-control
            // events (PreInvocation/PostInvocation/Stop) are direct handler
            // arrays (matcher is ignored for them).
            let value = if matches!(native, "PreToolUse" | "PostToolUse") {
                json!([{ "matcher": "*", "hooks": [handler] }])
            } else {
                json!([handler])
            };
            spec.insert(native.to_string(), value);
            event_count += 1;
        }

        // Whole-key replace: Claudine owns exactly the `claudine` named hook.
        root_obj.insert(CLAUDINE_HOOK_NAME.to_string(), Value::Object(spec));

        let output = serde_json::to_string_pretty(&root)?;
        atomic_write(&path, output.as_bytes())?;
        Ok(RegistrationResult::Registered { event_count })
    }

    fn deregister(&self, config_dir: Option<&Path>) -> Result<()> {
        let path = config_path(config_dir);
        if !path.exists() {
            return Ok(());
        }
        create_backup(&path, Provider::Antigravity)?;
        let content = fs::read_to_string(&path)?;
        let mut root: Value = serde_json::from_str(&content).unwrap_or_else(|_| json!({}));
        if let Some(obj) = root.as_object_mut() {
            obj.remove(CLAUDINE_HOOK_NAME);
        }
        let output = serde_json::to_string_pretty(&root)?;
        atomic_write(&path, output.as_bytes())?;
        Ok(())
    }

    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool> {
        let path = config_path(config_dir);
        if !path.exists() {
            return Ok(false);
        }
        let content = fs::read_to_string(&path)?;
        let root: Value = serde_json::from_str(&content)?;
        Ok(root.get(CLAUDINE_HOOK_NAME).is_some())
    }

    fn registered_events(&self, config_dir: Option<&Path>) -> Result<Vec<String>> {
        let path = config_path(config_dir);
        if !path.exists() {
            return Ok(vec![]);
        }
        let content = fs::read_to_string(&path)?;
        let root: Value = serde_json::from_str(&content)?;
        let Some(spec) = root.get(CLAUDINE_HOOK_NAME).and_then(Value::as_object) else {
            return Ok(vec![]);
        };
        let mut events: Vec<String> = spec
            .keys()
            .filter_map(|native| claudine_event_for_native(native))
            .map(str::to_string)
            .collect();
        events.sort();
        events.dedup();
        Ok(events)
    }
}

/// Reverse the native-event → Claudine-event mapping for the five wired hooks.
///
/// Mirrors the `event_mapping` in `docs/providers/facts/antigravity.yaml`; kept
/// local (rather than a catalog reverse-lookup) so `registered_events` reads
/// only what this configurator writes.
fn claudine_event_for_native(native: &str) -> Option<&'static str> {
    match native {
        "PreToolUse" => Some("before_tool"),
        "PostToolUse" => Some("after_tool"),
        "PreInvocation" => Some("before_model"),
        "PostInvocation" => Some("after_model"),
        "Stop" => Some("turn_complete"),
        _ => None,
    }
}

/// Resolve the `hooks.json` path (the global customization dir agy reads in
/// print mode), using an override dir for tests.
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("hooks.json"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".gemini").join("config").join("hooks.json")
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
    fn register_writes_named_hook_with_agy_schema() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("hooks.json"), "{}\n").unwrap();

        let cfg = AntigravityConfigurator;
        let plan = test_plan(vec![
            AgenticEvent::BeforeTool,
            AgenticEvent::AfterModel,
            AgenticEvent::TurnComplete,
        ]);
        let result = cfg.register(&plan, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Registered { event_count: 3 }
        ));

        let root: Value =
            serde_json::from_str(&fs::read_to_string(tmp.path().join("hooks.json")).unwrap())
                .unwrap();
        let spec = &root["claudine"];
        // PreToolUse (before_tool) uses a matcher-group.
        assert_eq!(spec["PreToolUse"][0]["matcher"], "*");
        assert_eq!(spec["PreToolUse"][0]["hooks"][0]["type"], "command");
        assert!(
            spec["PreToolUse"][0]["hooks"][0]["command"]
                .as_str()
                .unwrap()
                .contains("handle before_tool --provider antigravity")
        );
        assert_eq!(spec["PreToolUse"][0]["hooks"][0]["timeout"], 30);
        // PostInvocation (after_model) is a direct handler array (no matcher).
        assert_eq!(spec["PostInvocation"][0]["type"], "command");
        assert!(spec["PostInvocation"][0].get("matcher").is_none());
        // Stop (turn_complete) is a direct handler array.
        assert!(
            spec["Stop"][0]["command"]
                .as_str()
                .unwrap()
                .contains("handle turn_complete")
        );
    }

    #[test]
    fn register_preserves_other_named_hooks() {
        let tmp = TempDir::new().unwrap();
        let initial = json!({ "user-thing": { "Stop": [ { "type": "command", "command": "echo hi" } ] } });
        fs::write(
            tmp.path().join("hooks.json"),
            serde_json::to_string_pretty(&initial).unwrap(),
        )
        .unwrap();

        let cfg = AntigravityConfigurator;
        cfg.register(&test_plan(vec![AgenticEvent::BeforeTool]), Some(tmp.path()))
            .unwrap();

        let root: Value =
            serde_json::from_str(&fs::read_to_string(tmp.path().join("hooks.json")).unwrap())
                .unwrap();
        assert!(root.get("user-thing").is_some(), "user hook preserved");
        assert!(root.get("claudine").is_some(), "claudine hook added");
    }

    #[test]
    fn deregister_removes_only_claudine_key() {
        let tmp = TempDir::new().unwrap();
        let initial = json!({
            "user-thing": { "Stop": [ { "type": "command", "command": "echo hi" } ] },
            "claudine": { "PreToolUse": [] }
        });
        fs::write(
            tmp.path().join("hooks.json"),
            serde_json::to_string_pretty(&initial).unwrap(),
        )
        .unwrap();

        let cfg = AntigravityConfigurator;
        cfg.deregister(Some(tmp.path())).unwrap();

        let root: Value =
            serde_json::from_str(&fs::read_to_string(tmp.path().join("hooks.json")).unwrap())
                .unwrap();
        assert!(root.get("claudine").is_none());
        assert!(root.get("user-thing").is_some());
    }

    #[test]
    fn is_registered_and_registered_events_roundtrip() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("hooks.json"), "{}\n").unwrap();
        let cfg = AntigravityConfigurator;
        assert!(!cfg.is_registered(Some(tmp.path())).unwrap());

        cfg.register(
            &test_plan(vec![AgenticEvent::BeforeTool, AgenticEvent::TurnComplete]),
            Some(tmp.path()),
        )
        .unwrap();
        assert!(cfg.is_registered(Some(tmp.path())).unwrap());
        let events = cfg.registered_events(Some(tmp.path())).unwrap();
        assert_eq!(events, vec!["before_tool", "turn_complete"]);
    }

    #[test]
    fn provider_returns_antigravity() {
        assert_eq!(AntigravityConfigurator.provider(), Provider::Antigravity);
    }
}
