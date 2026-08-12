use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::atomic::atomic_write;
use super::backup::create_backup;
use super::claudine_handle_command;
use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};
use crate::error::Result;
use crate::provider::Provider;

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
/// The executable must be the first shell token. Full paths and quoted paths
/// are accepted; the Windows `.exe` basename is matched case-insensitively.
fn is_claudine_command(cmd: &str) -> bool {
    claudine_command_args(cmd).is_some_and(|args| !args.is_empty())
}

fn claudine_command_args(cmd: &str) -> Option<&str> {
    let cmd = cmd.trim_start();
    let (executable, tail) = match cmd.as_bytes().first().copied() {
        Some(quote @ (b'\'' | b'"')) => {
            let closing = cmd[1..].find(char::from(quote))? + 1;
            (&cmd[1..closing], &cmd[closing + 1..])
        }
        Some(_) => {
            let boundary = cmd.find(char::is_whitespace).unwrap_or(cmd.len());
            (&cmd[..boundary], &cmd[boundary..])
        }
        None => return None,
    };
    let basename = executable.rsplit(['/', '\\']).next()?;
    (basename == "claudine" || basename.eq_ignore_ascii_case("claudine.exe"))
        .then(|| tail.trim_start())
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
/// Handles short and full-path forms, with or without a native Windows
/// executable suffix.
fn extract_claudine_event(entry: &Value) -> Option<String> {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .and_then(|hooks| {
            hooks.iter().find_map(|hook| {
                hook.get("command")
                    .and_then(|c| c.as_str())
                    .and_then(|cmd| {
                        let mut args = claudine_command_args(cmd)?.split_whitespace();
                        args.next();
                        args.next().map(str::to_string)
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
mod tests;
