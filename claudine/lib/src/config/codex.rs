use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::DocumentMut;

use crate::error::Result;
use crate::events::{HookerConfig, Provider};

use super::atomic::atomic_write;
use super::backup::create_backup;
use super::claudine_command;
use super::trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

pub(crate) struct CodexConfigurator;

impl AgentConfigurator for CodexConfigurator {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn register(
        &self,
        _config: &HookerConfig,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        let config_path = config_path(config_dir);
        if !config_path.exists() {
            return Ok(RegistrationResult::Skipped(SkipReason::NotDetected));
        }

        if self.is_registered(config_dir)? {
            return Ok(RegistrationResult::Skipped(SkipReason::AlreadyRegistered));
        }

        create_backup(&config_path, Provider::Codex)?;

        let content = fs::read_to_string(&config_path)?;
        let mut doc: DocumentMut = content.parse()?;

        // Check for existing notify value
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
            });

        let claudine_bin = claudine_command();

        if let Some(original_cmd) = existing_notify {
            // Need to create a wrapper script
            let wrapper_path = wrapper_script_path(config_dir);
            create_wrapper_script(&wrapper_path, &original_cmd, &claudine_bin)?;

            // Set notify to the wrapper
            let mut arr = toml_edit::Array::new();
            arr.push(wrapper_path.to_string_lossy().to_string());
            doc["notify"] = toml_edit::value(arr);
        } else {
            // No existing notify, set directly
            let mut arr = toml_edit::Array::new();
            arr.push(&claudine_bin);
            arr.push("handle");
            doc["notify"] = toml_edit::value(arr);
        }

        atomic_write(&config_path, doc.to_string().as_bytes())?;

        // Only TurnComplete is available in Codex
        Ok(RegistrationResult::Registered { event_count: 1 })
    }

    fn deregister(&self, config_dir: Option<&Path>) -> Result<()> {
        let config_path = config_path(config_dir);
        if !config_path.exists() {
            return Ok(());
        }

        create_backup(&config_path, Provider::Codex)?;

        let content = fs::read_to_string(&config_path)?;
        let mut doc: DocumentMut = content.parse()?;

        // Check if the notify points to our wrapper
        let is_wrapper = doc
            .get("notify")
            .and_then(|v| {
                if let Some(arr) = v.as_array() {
                    arr.get(0).and_then(|item| item.as_str()).map(|s| s.to_string())
                } else {
                    v.as_str().map(String::from)
                }
            })
            .is_some_and(|cmd| cmd.contains("claudine"));

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

        atomic_write(&config_path, doc.to_string().as_bytes())?;

        Ok(())
    }

    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool> {
        let config_path = config_path(config_dir);
        if !config_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&config_path)?;
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
    let notify_cmd = doc
        .get("notify")
        .and_then(|v| {
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
fn create_wrapper_script(path: &Path, original_command: &str, claudine_bin: &str) -> Result<()> {
    let script = format!(
        "#!/bin/bash\n\
         # Claudine wrapper for Codex notify\n\
         # Calls both original and claudine\n\
         {original_command} \"$@\"\n\
         {claudine_bin} handle turn_complete \"$@\"\n"
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
    use std::collections::HashMap;

    use tempfile::TempDir;

    use crate::events::{AgenticEvent, EventBinding, GlobalSettings, ProviderConfig};

    use super::*;

    fn test_config() -> HookerConfig {
        let mut events = HashMap::new();
        events.insert(
            AgenticEvent::TurnComplete,
            EventBinding {
                enabled: true,
                actions: vec![],
                matcher: None,
            },
        );
        let mut providers = HashMap::new();
        providers.insert(
            Provider::Codex,
            ProviderConfig { events },
        );
        HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers,
        }
    }

    #[test]
    fn register_sets_notify_in_toml() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");
        fs::write(&config, "# Codex config\nmodel = \"o3\"\n").unwrap();

        let configurator = CodexConfigurator;
        let hooker_config = test_config();

        let result = configurator
            .register(&hooker_config, Some(tmp.path()))
            .unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Registered { event_count: 1 }
        ));

        let content = fs::read_to_string(&config).unwrap();
        assert!(content.contains("notify"));
        assert!(content.contains("claudine"));
        assert!(content.contains("handle"));
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
        let hooker_config = test_config();
        configurator
            .register(&hooker_config, Some(tmp.path()))
            .unwrap();

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
        let hooker_config = test_config();
        configurator
            .register(&hooker_config, Some(tmp.path()))
            .unwrap();

        // Wrapper script should be created
        let wrapper_path = tmp.path().join("codex-notify-wrapper.sh");
        assert!(wrapper_path.exists());

        let script = fs::read_to_string(&wrapper_path).unwrap();
        assert!(script.contains("my-tool done"));
        assert!(script.contains("claudine handle turn_complete"));
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
        create_wrapper_script(&wrapper_path, "my-tool done", "claudine").unwrap();

        let script = fs::read_to_string(&wrapper_path).unwrap();
        assert!(script.starts_with("#!/bin/bash"));
        assert!(script.contains("my-tool done \"$@\""));
        assert!(script.contains("claudine handle turn_complete \"$@\""));
    }
}
