use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::config::atomic::atomic_write;
use crate::error::Result;
use crate::events::Provider;

use super::types::{
    McpOrigin, McpProviderState, McpScope, ProviderScopeEntries, ProviderStateEntry,
    provider_state_path,
};

/// Scope for state operations — either user-global or a specific repo.
#[derive(Debug, Clone)]
pub enum Scope {
    User,
    Repo(std::path::PathBuf),
}

/// Provider-state store for tracking import provenance and sync ownership.
pub struct McpProviderStateStore {
    state: McpProviderState,
}

impl McpProviderStateStore {
    /// Load from the default path, creating empty state if missing.
    pub fn load() -> Result<Self> {
        Self::load_from(&provider_state_path())
    }

    /// Load from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        let state = if path.exists() {
            let content = fs::read_to_string(path)?;
            serde_json::from_str(&content)?
        } else {
            McpProviderState::default()
        };
        Ok(Self { state })
    }

    /// Save to the default path.
    pub fn save(&self) -> Result<()> {
        self.save_to(&provider_state_path())
    }

    /// Save to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.state)?;
        atomic_write(path, content.as_bytes())
    }

    /// Record an imported entry (origin = Imported).
    pub fn record_import(
        &mut self,
        provider: Provider,
        scope: &Scope,
        entry: ProviderStateEntry,
    ) {
        let entries = self.get_or_create_entries(provider, scope);
        Self::upsert_entry(entries, entry, McpScope::User, scope);
    }

    /// Record a managed entry (origin = Managed).
    pub fn record_managed(
        &mut self,
        provider: Provider,
        scope: &Scope,
        entry: ProviderStateEntry,
    ) {
        let entries = self.get_or_create_entries(provider, scope);
        Self::upsert_entry(entries, entry, McpScope::User, scope);
    }

    /// Get all entries for a provider at a given scope.
    pub fn entries_for_provider(&self, provider: Provider, scope: &Scope) -> Vec<&ProviderStateEntry> {
        let slug = provider.as_slug();
        match scope {
            Scope::User => self
                .state
                .providers
                .get(slug)
                .map(|e| e.user.iter().collect())
                .unwrap_or_default(),
            Scope::Repo(path) => {
                let path_str = path.to_string_lossy();
                self.state
                    .repos
                    .get(path_str.as_ref())
                    .and_then(|repo| repo.providers.get(slug))
                    .map(|e| e.repo.iter().collect())
                    .unwrap_or_default()
            }
        }
    }

    /// Get only managed entries for a provider at a given scope.
    pub fn managed_entries_for_provider(
        &self,
        provider: Provider,
        scope: &Scope,
    ) -> Vec<&ProviderStateEntry> {
        self.entries_for_provider(provider, scope)
            .into_iter()
            .filter(|e| e.origin == McpOrigin::Managed)
            .collect()
    }

    /// Remove a specific entry by catalog_id.
    pub fn remove_entry(
        &mut self,
        provider: Provider,
        scope: &Scope,
        catalog_id: &str,
    ) {
        let slug = provider.as_slug();
        match scope {
            Scope::User => {
                if let Some(entries) = self.state.providers.get_mut(slug) {
                    entries.user.retain(|e| e.catalog_id != catalog_id);
                }
            }
            Scope::Repo(path) => {
                let path_str = path.to_string_lossy().to_string();
                if let Some(repo) = self.state.repos.get_mut(&path_str)
                    && let Some(entries) = repo.providers.get_mut(slug)
                {
                    entries.repo.retain(|e| e.catalog_id != catalog_id);
                }
            }
        }
    }

    /// Returns a reference to the underlying state.
    pub fn state(&self) -> &McpProviderState {
        &self.state
    }

    fn get_or_create_entries(
        &mut self,
        provider: Provider,
        scope: &Scope,
    ) -> &mut ProviderScopeEntries {
        let slug = provider.as_slug().to_string();
        match scope {
            Scope::User => self
                .state
                .providers
                .entry(slug)
                .or_default(),
            Scope::Repo(path) => {
                let path_str = path.to_string_lossy().to_string();
                let repo = self
                    .state
                    .repos
                    .entry(path_str)
                    .or_default();
                repo.providers.entry(slug).or_default()
            }
        }
    }

    fn upsert_entry(
        entries: &mut ProviderScopeEntries,
        mut entry: ProviderStateEntry,
        _mcp_scope: McpScope,
        scope: &Scope,
    ) {
        entry.last_seen = Utc::now();
        let vec = match scope {
            Scope::User => &mut entries.user,
            Scope::Repo(_) => &mut entries.repo,
        };
        if let Some(existing) = vec.iter_mut().find(|e| e.catalog_id == entry.catalog_id) {
            *existing = entry;
        } else {
            vec.push(entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn make_entry(catalog_id: &str, origin: McpOrigin) -> ProviderStateEntry {
        ProviderStateEntry {
            catalog_id: catalog_id.into(),
            native_name: catalog_id.into(),
            source: "~/.codex/config.toml".into(),
            origin,
            last_seen: Utc::now(),
        }
    }

    #[test]
    fn record_and_retrieve_import() {
        let mut store =
            McpProviderStateStore::load_from(std::path::Path::new("/nonexistent")).unwrap();

        let entry = make_entry("google-calendar", McpOrigin::Imported);
        store.record_import(Provider::Codex, &Scope::User, entry);

        let entries = store.entries_for_provider(Provider::Codex, &Scope::User);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].catalog_id, "google-calendar");
        assert_eq!(entries[0].origin, McpOrigin::Imported);
    }

    #[test]
    fn record_managed_entry() {
        let mut store =
            McpProviderStateStore::load_from(std::path::Path::new("/nonexistent")).unwrap();

        let entry = make_entry("linear", McpOrigin::Managed);
        store.record_managed(Provider::Gemini, &Scope::User, entry);

        let managed = store.managed_entries_for_provider(Provider::Gemini, &Scope::User);
        assert_eq!(managed.len(), 1);
        assert_eq!(managed[0].catalog_id, "linear");
    }

    #[test]
    fn repo_scope_tracking() {
        let tmp = TempDir::new().unwrap();
        let mut store =
            McpProviderStateStore::load_from(std::path::Path::new("/nonexistent")).unwrap();

        let scope = Scope::Repo(tmp.path().to_path_buf());
        let entry = make_entry("github", McpOrigin::Managed);
        store.record_managed(Provider::Gemini, &scope, entry);

        let entries = store.entries_for_provider(Provider::Gemini, &scope);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].catalog_id, "github");
    }

    #[test]
    fn remove_entry() {
        let mut store =
            McpProviderStateStore::load_from(std::path::Path::new("/nonexistent")).unwrap();

        store.record_import(
            Provider::Codex,
            &Scope::User,
            make_entry("to-remove", McpOrigin::Imported),
        );
        assert_eq!(
            store
                .entries_for_provider(Provider::Codex, &Scope::User)
                .len(),
            1
        );

        store.remove_entry(Provider::Codex, &Scope::User, "to-remove");
        assert!(store
            .entries_for_provider(Provider::Codex, &Scope::User)
            .is_empty());
    }

    #[test]
    fn upsert_updates_existing() {
        let mut store =
            McpProviderStateStore::load_from(std::path::Path::new("/nonexistent")).unwrap();

        let entry1 = make_entry("server-a", McpOrigin::Imported);
        store.record_import(Provider::Claude, &Scope::User, entry1);

        let mut entry2 = make_entry("server-a", McpOrigin::Managed);
        entry2.native_name = "updated-name".into();
        store.record_managed(Provider::Claude, &Scope::User, entry2);

        let entries = store.entries_for_provider(Provider::Claude, &Scope::User);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].native_name, "updated-name");
    }

    #[test]
    fn save_and_reload() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("state.json");

        let mut store = McpProviderStateStore::load_from(&path).unwrap();
        store.record_import(
            Provider::Codex,
            &Scope::User,
            make_entry("test", McpOrigin::Imported),
        );
        store.save_to(&path).unwrap();

        let reloaded = McpProviderStateStore::load_from(&path).unwrap();
        assert_eq!(
            reloaded
                .entries_for_provider(Provider::Codex, &Scope::User)
                .len(),
            1
        );
    }
}
