use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::DocumentMut;

use super::atomic::atomic_write;
use super::backup::create_backup;
use super::claudine_handle_command;
use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};
use crate::error::Result;
use crate::events::AgenticEvent;
use crate::provider::Provider;

/// Minimal valid config.toml for Codex CLI.
///
/// An empty TOML file (or one with just a comment) is valid for Codex.
/// Note: TOML requires root keys to appear before tables.
const MINIMAL_CONFIG: &str =
    "# Codex CLI configuration\n# Created by claudine for hook registration\n";

pub(crate) struct CodexConfigurator;

impl AgentConfigurator for CodexConfigurator {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn create_minimal_config(&self, config_dir: Option<&Path>) -> crate::error::Result<()> {
        let toml_path = config_path(config_dir);

        // Create parent directory if needed
        if let Some(parent) = toml_path.parent() {
            fs::create_dir_all(parent)?;
        }

        atomic_write(&toml_path, MINIMAL_CONFIG.as_bytes())?;
        Ok(())
    }

    fn register(
        &self,
        plan: &ProviderHookPlan,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        let toml_path = config_path(config_dir);

        // If config doesn't exist but CLI is installed, create minimal config
        if !toml_path.exists() {
            if self.is_cli_installed() {
                self.create_minimal_config(config_dir)?;
            } else {
                return Ok(RegistrationResult::Skipped(SkipReason::NotDetected));
            }
        }

        // Check if already in sync (Codex only supports turn_complete)
        if self.is_in_sync(plan, config_dir)? {
            return Ok(RegistrationResult::Skipped(SkipReason::AlreadyRegistered));
        }

        // Check if we should register (turn_complete is enabled in config)
        let should_register = plan.events.contains(&AgenticEvent::TurnComplete);

        if !should_register {
            // Config exists but turn_complete is disabled - deregister
            self.deregister(config_dir)?;
            return Ok(RegistrationResult::Registered { event_count: 0 });
        }

        create_backup(&toml_path, Provider::Codex)?;

        let content = fs::read_to_string(&toml_path)?;
        let mut doc: DocumentMut = content.parse()?;

        // Check for existing notify value, but ignore claudine's own
        // direct-notify or wrapper entries — those aren't user commands.
        let existing_notify = doc
            .get("notify")
            .and_then(|v| {
                // Extract existing command from array or string
                if let Some(arr) = v.as_array() {
                    let parts: Vec<String> = arr
                        .iter()
                        .filter_map(|item| item.as_str().map(String::from))
                        .collect();
                    if !parts.is_empty() {
                        Some(parts.join(" "))
                    } else {
                        None
                    }
                } else {
                    v.as_str().map(String::from)
                }
            })
            .filter(|_cmd| {
                // Don't preserve claudine's own commands as "original"
                !is_claudine_notify(&doc) && !is_claudine_wrapper(&doc, config_dir)
            });

        let wrapper_path = wrapper_script_path(config_dir);
        create_wrapper_script(&wrapper_path, existing_notify.as_deref())?;

        let mut arr = toml_edit::Array::new();
        arr.push(wrapper_path.to_string_lossy().to_string());
        doc["notify"] = toml_edit::value(arr);

        atomic_write(&toml_path, doc.to_string().as_bytes())?;

        // Only TurnComplete is available in Codex
        Ok(RegistrationResult::Registered { event_count: 1 })
    }

    fn deregister(&self, config_dir: Option<&Path>) -> Result<()> {
        let toml_path = config_path(config_dir);
        if !toml_path.exists() {
            return Ok(());
        }

        create_backup(&toml_path, Provider::Codex)?;

        let content = fs::read_to_string(&toml_path)?;
        let mut doc: DocumentMut = content.parse()?;

        // Check if the notify points to our wrapper
        let is_wrapper = is_claudine_wrapper(&doc, config_dir);

        if is_wrapper {
            // Check if wrapper exists and extract original command
            let wrapper_path = wrapper_script_path(config_dir);
            if let Ok(script) = fs::read_to_string(&wrapper_path) {
                if let Some(original) = extract_original_from_wrapper(&script) {
                    // Restore original notify
                    let parts: Vec<&str> = original.split_whitespace().collect();
                    let mut arr = toml_edit::Array::new();
                    for part in parts {
                        arr.push(part);
                    }
                    doc["notify"] = toml_edit::value(arr);
                } else {
                    doc.remove("notify");
                }
                let _ = fs::remove_file(&wrapper_path);
            } else {
                doc.remove("notify");
            }
        } else if is_claudine_notify(&doc) {
            doc.remove("notify");
        }

        atomic_write(&toml_path, doc.to_string().as_bytes())?;

        Ok(())
    }

    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool> {
        let toml_path = config_path(config_dir);
        if !toml_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&toml_path)?;
        let doc: DocumentMut = content.parse()?;

        Ok(is_claudine_notify(&doc) || is_claudine_wrapper(&doc, config_dir))
    }

    fn registered_events(&self, config_dir: Option<&Path>) -> Result<Vec<String>> {
        // Codex only supports turn_complete (via notify)
        if self.is_registered(config_dir)? {
            Ok(vec!["turn_complete".to_string()])
        } else {
            Ok(vec![])
        }
    }

    fn registerable_events(&self) -> Option<Vec<crate::events::AgenticEvent>> {
        // Codex can only register turn_complete via the notify option
        Some(vec![crate::events::AgenticEvent::TurnComplete])
    }
}

impl CodexConfigurator {
    /// Check if the registered hooks match the expected events from config.
    ///
    /// Beyond checking registered vs. should-be-registered, this also detects
    /// a legacy direct-notify registration (`["claudine", "handle"]`) that
    /// is missing the wrapper script. The wrapper is required because Codex
    /// passes the JSON payload as argv[1] but `claudine handle` reads stdin.
    fn is_in_sync(&self, plan: &ProviderHookPlan, config_dir: Option<&Path>) -> Result<bool> {
        let is_registered = self.is_registered(config_dir)?;

        // Codex only supports turn_complete
        let should_be_registered = plan.events.contains(&AgenticEvent::TurnComplete);

        if is_registered != should_be_registered {
            return Ok(false);
        }

        // If registered, verify the wrapper script exists. A direct-notify
        // registration (legacy) won't have a wrapper, which means the payload
        // never reaches stdin — treat that as out-of-sync so `register()`
        // creates the wrapper.
        if is_registered {
            let toml_path = config_path(config_dir);
            let content = fs::read_to_string(&toml_path)?;
            let doc: DocumentMut = content.parse()?;

            if is_claudine_notify(&doc) && !is_claudine_wrapper(&doc, config_dir) {
                // Direct notify without wrapper — needs re-registration
                return Ok(false);
            }

            // Also check the wrapper script file actually exists on disk
            let wrapper = wrapper_script_path(config_dir);
            if is_claudine_wrapper(&doc, config_dir) && !wrapper.exists() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Check if notify is set directly to claudine.
///
/// Matches both short form `["claudine", "handle"]` and full path
/// `["/path/to/claudine", "handle"]`.
fn is_claudine_notify(doc: &DocumentMut) -> bool {
    doc.get("notify")
        .and_then(|v| v.as_array())
        .is_some_and(|arr| {
            let parts: Vec<&str> = arr.iter().filter_map(|item| item.as_str()).collect();
            // Check: first part ends with "claudine", second part is "handle"
            parts.len() == 2
                && (parts[0] == "claudine" || parts[0].ends_with("/claudine"))
                && parts[1] == "handle"
        })
}

/// Check if notify points to a claudine wrapper script.
fn is_claudine_wrapper(doc: &DocumentMut, config_dir: Option<&Path>) -> bool {
    let notify_cmd = doc.get("notify").and_then(|v| {
        if let Some(arr) = v.as_array() {
            arr.get(0).and_then(|item| item.as_str()).map(String::from)
        } else {
            v.as_str().map(String::from)
        }
    });

    if let Some(cmd) = notify_cmd {
        let wrapper_path = wrapper_script_path(config_dir);
        cmd == wrapper_path.to_string_lossy()
    } else {
        false
    }
}

/// Generate the wrapper script that calls both the original and claudine.
fn create_wrapper_script(path: &Path, original_command: Option<&str>) -> Result<()> {
    let handle_command = claudine_handle_command(Provider::Codex)("turn_complete");
    let original_line = original_command
        .map(|command| format!("{command} \"$@\"\n"))
        .unwrap_or_default();
    let script = format!(
        "#!/bin/bash\n\
         # Claudine wrapper for Codex notify\n\
         # Codex appends the JSON payload as argv[1], so feed it to stdin.\n\
         {original_line}\
         printf '%s' \"$1\" | {handle_command}\n"
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, script.as_bytes())?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// Check if a line in a script is a claudine command.
fn is_claudine_line(line: &str) -> bool {
    line.contains("claudine handle") || line.contains("claudine ")
}

/// Extract the original command from a Claudine wrapper script.
fn extract_original_from_wrapper(script: &str) -> Option<String> {
    for line in script.lines() {
        let trimmed = line.trim();
        // Skip shebang, comments, empty lines, and claudine lines
        if trimmed.is_empty() || trimmed.starts_with('#') || is_claudine_line(trimmed) {
            continue;
        }
        // First non-comment, non-claudine command is the original
        return Some(trimmed.trim_end_matches(" \"$@\"").to_string());
    }
    None
}

/// Resolve the config.toml path for Codex.
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("config.toml"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".codex").join("config.toml")
        }
    }
}

/// Resolve the wrapper script path.
fn wrapper_script_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("codex-notify-wrapper.sh"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".claudine").join("codex-notify-wrapper.sh")
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::events::AgenticEvent;

    use super::*;

    fn test_plan() -> ProviderHookPlan {
        ProviderHookPlan {
            events: vec![AgenticEvent::TurnComplete],
            canonical_for: None,
        }
    }

    #[test]
    fn register_sets_notify_in_toml() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        fs::write(&config, "# Codex config\nmodel = \"o3\"\n").unwrap();

        let configurator = CodexConfigurator;
        let plan = test_plan();

        let result = configurator.register(&plan, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Registered { event_count: 1 }
        ));

        let content = fs::read_to_string(&config).unwrap();
        assert!(content.contains("notify"));
        assert!(content.contains("codex-notify-wrapper.sh"));
        let wrapper = fs::read_to_string(tmp.path().join("codex-notify-wrapper.sh")).unwrap();
        assert!(wrapper.contains("handle turn_complete --provider codex"));
        assert!(wrapper.contains("printf '%s' \"$1\""));
    }

    #[test]
    fn register_preserves_toml_comments() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        fs::write(
            &config,
            "# My Codex configuration\nmodel = \"o3\"\n# End of config\n",
        )
        .unwrap();

        let configurator = CodexConfigurator;
        let plan = test_plan();
        configurator.register(&plan, Some(tmp.path())).unwrap();

        let content = fs::read_to_string(&config).unwrap();
        // toml_edit preserves comments
        assert!(content.contains("# My Codex configuration"));
        assert!(content.contains("model = \"o3\""));
    }

    #[test]
    fn register_creates_wrapper_when_existing_notify() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        fs::write(&config, "notify = [\"my-tool\", \"done\"]\n").unwrap();

        let configurator = CodexConfigurator;
        let plan = test_plan();
        configurator.register(&plan, Some(tmp.path())).unwrap();

        // Wrapper script should be created
        let wrapper_path = tmp.path().join("codex-notify-wrapper.sh");
        assert!(wrapper_path.exists());

        let script = fs::read_to_string(&wrapper_path).unwrap();
        let handle_command = claudine_handle_command(Provider::Codex)("turn_complete");
        assert!(script.contains("my-tool done"));
        assert!(script.contains(&handle_command));
        assert!(script.contains("printf '%s' \"$1\""));
    }

    #[test]
    fn deregister_removes_claudine_notify() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        fs::write(
            &config,
            "model = \"o3\"\nnotify = [\"claudine\", \"handle\"]\n",
        )
        .unwrap();

        let configurator = CodexConfigurator;
        configurator.deregister(Some(tmp.path())).unwrap();

        let content = fs::read_to_string(&config).unwrap();
        assert!(!content.contains("notify"));
        assert!(content.contains("model = \"o3\""));
    }

    #[test]
    fn is_registered_detects_claudine() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        fs::write(&config, "notify = [\"claudine\", \"handle\"]\n").unwrap();

        let configurator = CodexConfigurator;
        assert!(configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn is_registered_false_without_claudine() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        fs::write(&config, "model = \"o3\"\n").unwrap();

        let configurator = CodexConfigurator;
        assert!(!configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn wrapper_script_is_valid_bash() {
        let tmp = TempDir::new().unwrap();
        let wrapper_path = tmp.path().join("wrapper.sh");
        create_wrapper_script(&wrapper_path, Some("my-tool done")).unwrap();

        let script = fs::read_to_string(&wrapper_path).unwrap();
        let expected_pipeline = format!(
            "printf '%s' \"$1\" | {}",
            claudine_handle_command(Provider::Codex)("turn_complete")
        );
        assert!(script.starts_with("#!/bin/bash"));
        assert!(script.contains("my-tool done \"$@\""));
        assert!(script.contains(&expected_pipeline));
    }

    #[test]
    fn register_handles_missing_config() {
        // When config doesn't exist:
        // - If CLI is installed → creates minimal config and registers hooks
        // - If CLI is not installed → returns NotDetected
        let tmp = TempDir::new().unwrap();
        let configurator = CodexConfigurator;
        let plan = test_plan();

        let result = configurator.register(&plan, Some(tmp.path())).unwrap();

        if configurator.is_cli_installed() {
            // CLI installed: should create config and register
            assert!(
                matches!(result, RegistrationResult::Registered { event_count: 1 }),
                "Expected Registered when CLI is installed, got {:?}",
                result
            );
            // Verify config was created
            let config_path = tmp.path().join("config.toml");
            assert!(config_path.exists(), "Config file should be created");
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
    fn create_minimal_config_creates_valid_toml() {
        let tmp = TempDir::new().unwrap();
        let configurator = CodexConfigurator;

        configurator
            .create_minimal_config(Some(tmp.path()))
            .unwrap();

        let config_path = tmp.path().join("config.toml");
        assert!(config_path.exists());

        // Verify it's valid TOML
        let content = fs::read_to_string(&config_path).unwrap();
        let _doc: toml_edit::DocumentMut = content.parse().expect("Should be valid TOML");
    }

    #[test]
    fn sync_detects_direct_notify_without_wrapper_as_out_of_sync() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        // Legacy direct-notify: claudine handle (no wrapper script)
        fs::write(
            &config,
            "model = \"o3\"\nnotify = [\"/usr/bin/claudine\", \"handle\"]\n",
        )
        .unwrap();

        let configurator = CodexConfigurator;
        let plan = test_plan();

        // is_registered sees the direct notify → true
        assert!(configurator.is_registered(Some(tmp.path())).unwrap());
        // But is_in_sync should detect missing wrapper → false
        assert!(!configurator.is_in_sync(&plan, Some(tmp.path())).unwrap());
    }

    #[test]
    fn sync_detects_missing_wrapper_file_as_out_of_sync() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        let wrapper = tmp.path().join("codex-notify-wrapper.sh");
        // Config points to wrapper path, but wrapper file doesn't exist
        let wrapper = toml::Value::String(wrapper.to_string_lossy().into_owned());
        fs::write(&config, format!("notify = [{wrapper}]\n")).unwrap();

        let configurator = CodexConfigurator;
        let plan = test_plan();

        // Wrapper path matches but file is missing → out of sync
        assert!(!configurator.is_in_sync(&plan, Some(tmp.path())).unwrap());
    }

    #[test]
    fn sync_considers_proper_wrapper_registration_in_sync() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        fs::write(&config, "model = \"o3\"\n").unwrap();

        let configurator = CodexConfigurator;
        let plan = test_plan();

        // Register properly (creates wrapper + updates config)
        configurator.register(&plan, Some(tmp.path())).unwrap();

        // Now should be in sync
        assert!(configurator.is_in_sync(&plan, Some(tmp.path())).unwrap());
    }

    #[test]
    fn create_minimal_config_creates_parent_directory() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("nested").join("dir");
        let configurator = CodexConfigurator;

        configurator.create_minimal_config(Some(&nested)).unwrap();

        assert!(nested.join("config.toml").exists());
    }
}
