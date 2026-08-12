use std::fs;
use std::path::Path;

use crate::config::atomic::atomic_write;
use crate::error::Result;

use super::catalog::McpCatalogStore;
use super::types::{McpDefaults, defaults_path, repo_defaults_path};

/// Load user-scope defaults from disk.
pub fn load_user_defaults() -> Result<McpDefaults> {
    let path = defaults_path();
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(McpDefaults::default())
    }
}

/// Save user-scope defaults to disk.
pub fn save_user_defaults(defaults: &McpDefaults) -> Result<()> {
    let path = defaults_path();
    let content = serde_json::to_string_pretty(defaults)?;
    Ok(atomic_write(&path, content.as_bytes())?)
}

/// Set user-scope defaults (replacing any existing).
pub fn set_user_defaults(ids: Vec<String>) -> Result<()> {
    let defaults = McpDefaults {
        version: 1,
        defaults: ids,
    };
    save_user_defaults(&defaults)
}

/// Load repo-scope defaults from disk, if present.
pub fn load_repo_defaults(repo_root: &Path) -> Result<Option<McpDefaults>> {
    let path = repo_defaults_path(repo_root);
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        Ok(Some(serde_json::from_str(&content)?))
    } else {
        Ok(None)
    }
}

/// Save repo-scope defaults to disk.
pub fn save_repo_defaults(repo_root: &Path, defaults: &McpDefaults) -> Result<()> {
    let path = repo_defaults_path(repo_root);
    let content = serde_json::to_string_pretty(defaults)?;
    Ok(atomic_write(&path, content.as_bytes())?)
}

/// Set repo-scope defaults (replacing any existing).
pub fn set_repo_defaults(repo_root: &Path, ids: Vec<String>) -> Result<()> {
    let defaults = McpDefaults {
        version: 1,
        defaults: ids,
    };
    save_repo_defaults(repo_root, &defaults)
}

/// Compute the effective defaults: repo defaults replace user defaults.
///
/// Warns (via tracing) about IDs that don't exist in the catalog, but
/// does not fail — missing IDs are still returned.
pub fn effective_defaults(
    repo_root: Option<&Path>,
    catalog: &McpCatalogStore,
) -> Result<Vec<String>> {
    let defaults = if let Some(root) = repo_root {
        if let Some(repo_defaults) = load_repo_defaults(root)? {
            repo_defaults.defaults
        } else {
            load_user_defaults()?.defaults
        }
    } else {
        load_user_defaults()?.defaults
    };

    // Warn about missing catalog IDs
    for id in &defaults {
        if catalog.get_server(id).is_none() {
            tracing::warn!(id, "default MCP server not found in catalog");
        }
    }

    Ok(defaults)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn user_defaults_round_trip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("defaults.json");

        let defaults = McpDefaults {
            version: 1,
            defaults: vec!["a".into(), "b".into()],
        };
        let content = serde_json::to_string_pretty(&defaults).unwrap();
        atomic_write(&path, content.as_bytes()).unwrap();

        let loaded: McpDefaults =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.defaults, vec!["a", "b"]);
    }

    #[test]
    fn repo_defaults_replace_user_defaults() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path();

        // Create repo defaults
        let repo_defaults = McpDefaults {
            version: 1,
            defaults: vec!["repo-server".into()],
        };
        save_repo_defaults(repo_root, &repo_defaults).unwrap();

        let loaded = load_repo_defaults(repo_root).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().defaults, vec!["repo-server"]);
    }

    #[test]
    fn missing_repo_defaults_returns_none() {
        let tmp = TempDir::new().unwrap();
        let loaded = load_repo_defaults(tmp.path()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn empty_defaults_is_initial_state() {
        let defaults = McpDefaults::default();
        assert!(defaults.defaults.is_empty());
        assert_eq!(defaults.version, 1);
    }
}
