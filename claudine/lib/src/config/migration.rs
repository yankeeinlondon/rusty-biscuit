use std::path::{Path, PathBuf};

use crate::error::Result;

/// Known provider keys that appear in the old per-provider config format.
const OLD_PROVIDER_KEYS: &[&str] = &[
    "claude",
    "codex",
    "gemini",
    "goose",
    "kimi_code",
    "opencode",
    "qwen_code",
];

/// Returns `true` if `value` looks like the old per-provider config format.
///
/// The old format is identified by either:
/// - A root object containing both `"version"` and `"providers"` keys, or
/// - A root object containing any known provider key at the root level.
///
/// Returns `false` for non-objects.
pub fn is_old_format(value: &serde_json::Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };

    if obj.contains_key("version") && obj.contains_key("providers") {
        return true;
    }

    OLD_PROVIDER_KEYS.iter().any(|key| obj.contains_key(*key))
}

/// Renames the config file at `config_path` to `<config_path>.bak`.
///
/// ## Returns
///
/// The backup path on success.
///
/// ## Errors
///
/// Returns [`crate::error::ClaudineError::Io`] if the rename fails.
pub fn backup_old_config(config_path: &Path) -> Result<PathBuf> {
    let backup_path = {
        let mut s = config_path.as_os_str().to_os_string();
        s.push(".bak");
        PathBuf::from(s)
    };

    tracing::info!(
        original = %config_path.display(),
        backup = %backup_path.display(),
        "backing up old config format"
    );

    std::fs::rename(config_path, &backup_path)?;
    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_old_version_providers_format() {
        let value = json!({
            "version": "1.0",
            "settings": {},
            "providers": { "claude": {} }
        });
        assert!(is_old_format(&value));
    }

    #[test]
    fn detects_old_provider_keys_at_root() {
        let value = json!({ "claude": {}, "gemini": {} });
        assert!(is_old_format(&value));
    }

    #[test]
    fn rejects_new_format() {
        let value = json!({
            "tts": true,
            "logging": true,
            "protect": true,
            "preferred_agent": "claude",
            "actions": {}
        });
        assert!(!is_old_format(&value));
    }

    #[test]
    fn rejects_non_object() {
        assert!(!is_old_format(&json!("a string")));
        assert!(!is_old_format(&json!(42)));
        assert!(!is_old_format(&json!(null)));
    }

    #[test]
    fn backup_renames_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("claudine.json");
        std::fs::write(&config_path, b"{}").expect("write temp config");

        let backup_path = backup_old_config(&config_path).expect("backup");

        assert!(!config_path.exists(), "original should be gone");
        assert!(backup_path.exists(), ".bak should exist");
        assert_eq!(backup_path, dir.path().join("claudine.json.bak"));
    }
}
