use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::types::{
    McpOrigin, McpProviderState, ProviderScopeEntries, ProviderStateEntry, provider_state_path,
};
use crate::config::atomic::atomic_write;
use crate::error::Result;
use crate::provider::Provider;

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
        Ok(atomic_write(path, content.as_bytes())?)
    }

    /// Record an imported entry (origin = Imported).
    pub fn record_import(&mut self, provider: Provider, scope: &Scope, entry: ProviderStateEntry) {
        let entries = self.get_or_create_entries(provider, scope);
        Self::upsert_entry(entries, entry, scope);
    }

    /// Record a managed entry (origin = Managed).
    pub fn record_managed(&mut self, provider: Provider, scope: &Scope, entry: ProviderStateEntry) {
        let entries = self.get_or_create_entries(provider, scope);
        Self::upsert_entry(entries, entry, scope);
    }

    /// Get all entries for a provider at a given scope.
    pub fn entries_for_provider(
        &self,
        provider: Provider,
        scope: &Scope,
    ) -> Vec<&ProviderStateEntry> {
        let slug = provider.as_slug();
        match scope {
            Scope::User => self
                .state
                .providers
                .get(slug)
                .map(|e| e.user.iter().collect())
                .unwrap_or_default(),
            Scope::Repo(path) => self
                .resolve_repo_key(path)
                .as_ref()
                .and_then(|key| self.state.repos.get(key))
                .and_then(|repo| repo.providers.get(slug))
                .map(|e| e.repo.iter().collect())
                .unwrap_or_default(),
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

    /// Get a single entry for a provider/scope pair by catalog ID.
    pub fn entry_for_catalog_id(
        &self,
        provider: Provider,
        scope: &Scope,
        catalog_id: &str,
    ) -> Option<&ProviderStateEntry> {
        self.entries_for_provider(provider, scope)
            .into_iter()
            .find(|entry| entry.catalog_id == catalog_id)
    }

    /// Remove a specific entry by catalog_id.
    pub fn remove_entry(&mut self, provider: Provider, scope: &Scope, catalog_id: &str) {
        let slug = provider.as_slug();
        match scope {
            Scope::User => {
                if let Some(entries) = self.state.providers.get_mut(slug) {
                    entries.user.retain(|e| e.catalog_id != catalog_id);
                }
            }
            Scope::Repo(path) => {
                if let Some(repo_key) = self.resolve_repo_key(path)
                    && let Some(repo) = self.state.repos.get_mut(&repo_key)
                    && let Some(entries) = repo.providers.get_mut(slug)
                {
                    entries.repo.retain(|e| e.catalog_id != catalog_id);
                }
            }
        }
    }

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
            Scope::User => self.state.providers.entry(slug).or_default(),
            Scope::Repo(path) => {
                let path_str = self
                    .resolve_repo_key(path)
                    .unwrap_or_else(|| Self::preferred_repo_key(path));
                let repo = self.state.repos.entry(path_str).or_default();
                repo.providers.entry(slug).or_default()
            }
        }
    }

    fn upsert_entry(
        entries: &mut ProviderScopeEntries,
        mut entry: ProviderStateEntry,
        scope: &Scope,
    ) {
        entry.last_seen = Utc::now();
        let vec = match scope {
            Scope::User => &mut entries.user,
            Scope::Repo(_) => &mut entries.repo,
        };

        if let Some(existing) = vec.iter_mut().find(|e| e.catalog_id == entry.catalog_id) {
            if existing.origin == McpOrigin::Managed && entry.origin == McpOrigin::Imported {
                entry.origin = McpOrigin::Managed;
            }
            *existing = entry;
            return;
        }

        if let Some(existing) = vec.iter_mut().find(|e| e.native_name == entry.native_name) {
            if existing.origin == McpOrigin::Managed && entry.origin == McpOrigin::Imported {
                entry.origin = McpOrigin::Managed;
            }
            *existing = entry;
            return;
        }

        vec.push(entry);
    }

    fn resolve_repo_key(&self, path: &Path) -> Option<String> {
        let raw = path.to_string_lossy().to_string();
        if self.state.repos.contains_key(&raw) {
            return Some(raw);
        }

        if let Some(canonical) = Self::canonical_repo_key(path)
            && self.state.repos.contains_key(&canonical)
        {
            return Some(canonical);
        }

        let requested_canonical = Self::canonical_repo_path(path);
        self.state.repos.keys().find_map(|candidate| {
            let candidate_path = PathBuf::from(candidate);
            let matches = candidate_path == path
                || requested_canonical
                    .as_ref()
                    .zip(Self::canonical_repo_path(&candidate_path).as_ref())
                    .is_some_and(|(requested, candidate)| candidate == requested);
            matches.then(|| candidate.clone())
        })
    }

    fn preferred_repo_key(path: &Path) -> String {
        Self::canonical_repo_key(path).unwrap_or_else(|| path.to_string_lossy().to_string())
    }

    fn canonical_repo_key(path: &Path) -> Option<String> {
        Self::canonical_repo_path(path).map(|canonical| canonical.to_string_lossy().to_string())
    }

    fn canonical_repo_path(path: &Path) -> Option<PathBuf> {
        fs::canonicalize(path).ok()
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
    fn repo_scope_lookup_matches_canonical_and_noncanonical_paths() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        let nested = repo_root.join("nested");
        fs::create_dir_all(&nested).unwrap();

        let mut store =
            McpProviderStateStore::load_from(std::path::Path::new("/nonexistent")).unwrap();

        store.record_managed(
            Provider::Codex,
            &Scope::Repo(repo_root.clone()),
            ProviderStateEntry {
                catalog_id: "calendar".into(),
                native_name: "calendar-native".into(),
                source: repo_root.join(".codex/config.toml").display().to_string(),
                origin: McpOrigin::Managed,
                last_seen: Utc::now(),
            },
        );

        let lookup_scope = Scope::Repo(nested.join(".."));
        let entry = store
            .entry_for_catalog_id(Provider::Codex, &lookup_scope, "calendar")
            .expect("repo entry should resolve across path variants");
        assert_eq!(entry.native_name, "calendar-native");

        store.remove_entry(Provider::Codex, &lookup_scope, "calendar");
        assert!(
            store
                .entries_for_provider(Provider::Codex, &Scope::Repo(repo_root))
                .is_empty()
        );
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
        assert!(
            store
                .entries_for_provider(Provider::Codex, &Scope::User)
                .is_empty()
        );
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
