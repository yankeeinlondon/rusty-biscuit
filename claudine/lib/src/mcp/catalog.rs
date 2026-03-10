use std::fs;

use crate::config::atomic::atomic_write;
use crate::error::{ClaudineError, Result};

use super::types::{McpCatalog, McpServer, catalog_path};

/// The tier a query resolved through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchTier {
    ExactId,
    ExactAlias,
    Normalized,
    Prefix,
    Substring,
}

/// A resolved server plus the tier that matched it.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedMatch<'a> {
    pub server: &'a McpServer,
    pub tier: MatchTier,
}

/// Catalog store for MCP server definitions.
///
/// All CRUD operations go through this struct to ensure consistent
/// file I/O with atomic writes.
pub struct McpCatalogStore {
    catalog: McpCatalog,
}

impl McpCatalogStore {
    /// Load the catalog from disk, creating an empty one if missing.
    pub fn load() -> Result<Self> {
        let path = catalog_path();
        let catalog = if path.exists() {
            let content = fs::read_to_string(&path)?;
            serde_json::from_str(&content)?
        } else {
            McpCatalog::default()
        };
        Ok(Self { catalog })
    }

    /// Load from a specific path (for testing or custom locations).
    pub fn load_from(path: &std::path::Path) -> Result<Self> {
        let catalog = if path.exists() {
            let content = fs::read_to_string(path)?;
            serde_json::from_str(&content)?
        } else {
            McpCatalog::default()
        };
        Ok(Self { catalog })
    }

    /// Save the catalog to disk atomically.
    pub fn save(&self) -> Result<()> {
        self.save_to(&catalog_path())
    }

    /// Save to a specific path.
    pub fn save_to(&self, path: &std::path::Path) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.catalog)?;
        atomic_write(path, content.as_bytes())
    }

    /// Add a server to the catalog. Overwrites if the ID already exists.
    pub fn add_server(&mut self, server: McpServer) {
        self.catalog.servers.insert(server.id.clone(), server);
    }

    /// Remove a server by ID, returning it if found.
    pub fn remove_server(&mut self, id: &str) -> Result<McpServer> {
        // Also clean up any aliases in other servers that pointed here
        self.catalog
            .servers
            .remove(id)
            .ok_or_else(|| ClaudineError::McpServerNotFound { id: id.into() })
    }

    /// Get a server by exact ID.
    pub fn get_server(&self, id: &str) -> Option<&McpServer> {
        self.catalog.servers.get(id)
    }

    /// Get a mutable server by exact ID.
    pub fn get_server_mut(&mut self, id: &str) -> Option<&mut McpServer> {
        self.catalog.servers.get_mut(id)
    }

    /// Find a server by its content fingerprint.
    pub fn find_by_fingerprint(&self, fingerprint: &str) -> Option<&McpServer> {
        self.catalog
            .servers
            .values()
            .find(|s| s.metadata.fingerprint == fingerprint)
    }

    /// Add an alias to a server.
    ///
    /// ## Errors
    ///
    /// - `McpServerNotFound` if `id` is not in the catalog
    /// - `McpAliasConflict` if `alias` matches an existing server ID or alias
    pub fn add_alias(&mut self, id: &str, alias: &str) -> Result<()> {
        // Reject if alias matches an existing server ID
        if self.catalog.servers.contains_key(alias) {
            return Err(ClaudineError::McpAliasConflict {
                alias: alias.into(),
                existing_id: alias.into(),
            });
        }

        // Reject if alias is already assigned to a different server
        for (sid, server) in &self.catalog.servers {
            if server.aliases.iter().any(|a| a == alias) {
                if sid == id {
                    return Ok(()); // already assigned to this server
                }
                return Err(ClaudineError::McpAliasConflict {
                    alias: alias.into(),
                    existing_id: sid.clone(),
                });
            }
        }

        let server = self
            .catalog
            .servers
            .get_mut(id)
            .ok_or_else(|| ClaudineError::McpServerNotFound { id: id.into() })?;
        server.aliases.push(alias.into());
        Ok(())
    }

    /// Remove an alias from whichever server owns it.
    pub fn remove_alias(&mut self, alias: &str) -> Result<()> {
        for server in self.catalog.servers.values_mut() {
            if let Some(pos) = server.aliases.iter().position(|a| a == alias) {
                server.aliases.remove(pos);
                return Ok(());
            }
        }
        Err(ClaudineError::McpServerNotFound {
            id: format!("alias `{alias}` not found"),
        })
    }

    /// Resolve a query to a server using 5-tier priority:
    ///
    /// 1. Exact catalog ID
    /// 2. Exact alias
    /// 3. Normalized match (case-insensitive, `-`/`_` equivalent)
    /// 4. Prefix match
    /// 5. Substring match
    ///
    /// Returns `McpAmbiguousMatch` if multiple candidates match at the same tier.
    pub fn resolve(&self, query: &str) -> Result<&McpServer> {
        self.resolve_match(query).map(|resolved| resolved.server)
    }

    /// Resolve a query and return both the matched server and match tier.
    pub fn resolve_match(&self, query: &str) -> Result<ResolvedMatch<'_>> {
        if let Some(server) = self.catalog.servers.get(query) {
            return Ok(ResolvedMatch {
                server,
                tier: MatchTier::ExactId,
            });
        }

        if let Some(server) = resolve_match_list(
            query,
            self.catalog
                .servers
                .values()
                .filter(|s| s.aliases.iter().any(|a| a == query))
                .collect(),
        )? {
            return Ok(ResolvedMatch {
                server,
                tier: MatchTier::ExactAlias,
            });
        }

        let normalized = normalize_for_match(query);

        if let Some(server) = resolve_match_list(
            query,
            self.catalog
                .servers
                .iter()
                .filter(|(id, s)| {
                    normalize_for_match(id) == normalized
                        || s.aliases
                            .iter()
                            .any(|a| normalize_for_match(a) == normalized)
                })
                .map(|(_, s)| s)
                .collect(),
        )? {
            return Ok(ResolvedMatch {
                server,
                tier: MatchTier::Normalized,
            });
        }

        if let Some(server) = resolve_match_list(
            query,
            self.catalog
                .servers
                .iter()
                .filter(|(id, s)| {
                    normalize_for_match(id).starts_with(&normalized)
                        || s.aliases
                            .iter()
                            .any(|a| normalize_for_match(a).starts_with(&normalized))
                })
                .map(|(_, s)| s)
                .collect(),
        )? {
            return Ok(ResolvedMatch {
                server,
                tier: MatchTier::Prefix,
            });
        }

        if let Some(server) = resolve_match_list(
            query,
            self.catalog
                .servers
                .iter()
                .filter(|(id, s)| {
                    normalize_for_match(id).contains(&normalized)
                        || s.aliases
                            .iter()
                            .any(|a| normalize_for_match(a).contains(&normalized))
                })
                .map(|(_, s)| s)
                .collect(),
        )? {
            return Ok(ResolvedMatch {
                server,
                tier: MatchTier::Substring,
            });
        }

        Err(ClaudineError::McpServerNotFound { id: query.into() })
    }

    /// List all servers in the catalog.
    pub fn list_servers(&self) -> Vec<&McpServer> {
        let mut servers: Vec<&McpServer> = self.catalog.servers.values().collect();
        servers.sort_by_key(|s| &s.id);
        servers
    }

    /// Returns a reference to the underlying catalog.
    pub fn catalog(&self) -> &McpCatalog {
        &self.catalog
    }

    /// Returns a mutable reference to the underlying catalog.
    pub fn catalog_mut(&mut self) -> &mut McpCatalog {
        &mut self.catalog
    }
}

fn resolve_match_list<'a>(
    query: &str,
    matches: Vec<&'a McpServer>,
) -> Result<Option<&'a McpServer>> {
    if matches.len() == 1 {
        return Ok(matches.into_iter().next());
    }

    if matches.len() > 1 {
        return Err(ClaudineError::McpAmbiguousMatch {
            query: query.into(),
            candidates: matches.iter().map(|server| server.id.clone()).collect(),
        });
    }

    Ok(None)
}

/// Normalize a string for matching: lowercase, replace `_` with `-`.
fn normalize_for_match(s: &str) -> String {
    s.to_ascii_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use tempfile::TempDir;

    use super::*;
    use crate::mcp::types::{McpServerMetadata, McpTransport};

    fn make_server(id: &str) -> McpServer {
        McpServer {
            id: id.into(),
            aliases: Vec::new(),
            transport: McpTransport::Stdio,
            command: Some("npx".into()),
            args: vec![format!("@test/{id}")],
            cwd: None,
            env: HashMap::new(),
            url: None,
            headers: HashMap::new(),
            enabled_tools: Vec::new(),
            disabled_tools: Vec::new(),
            required: false,
            metadata: McpServerMetadata {
                description: None,
                created_from: None,
                fingerprint: String::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            provider_overrides: HashMap::new(),
        }
    }

    #[test]
    fn add_and_get_server() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        let server = make_server("test-server");
        store.add_server(server);
        assert!(store.get_server("test-server").is_some());
        assert!(store.get_server("missing").is_none());
    }

    #[test]
    fn remove_server() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("to-remove"));
        let removed = store.remove_server("to-remove").unwrap();
        assert_eq!(removed.id, "to-remove");
        assert!(store.get_server("to-remove").is_none());
    }

    #[test]
    fn remove_missing_server_errors() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        assert!(store.remove_server("nope").is_err());
    }

    #[test]
    fn find_by_fingerprint() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        let mut server = make_server("fp-test");
        let fp = server.fingerprint();
        server.metadata.fingerprint = fp.clone();
        store.add_server(server);

        assert!(store.find_by_fingerprint(&fp).is_some());
        assert!(store.find_by_fingerprint("nonexistent").is_none());
    }

    #[test]
    fn alias_management() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("my-server"));
        store.add_server(make_server("other-server"));

        // Add alias
        store.add_alias("my-server", "ms").unwrap();
        assert!(
            store
                .get_server("my-server")
                .unwrap()
                .aliases
                .contains(&"ms".into())
        );

        // Duplicate alias to same server is ok
        store.add_alias("my-server", "ms").unwrap();

        // Alias conflict with another server's alias
        assert!(store.add_alias("other-server", "ms").is_err());

        // Alias conflict with existing server ID
        assert!(store.add_alias("my-server", "other-server").is_err());

        // Remove alias
        store.remove_alias("ms").unwrap();
        assert!(
            !store
                .get_server("my-server")
                .unwrap()
                .aliases
                .contains(&"ms".into())
        );
    }

    #[test]
    fn resolve_exact_id() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("google-calendar"));
        let s = store.resolve("google-calendar").unwrap();
        assert_eq!(s.id, "google-calendar");
    }

    #[test]
    fn resolve_exact_alias() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("google-calendar"));
        store.add_alias("google-calendar", "gcal").unwrap();
        let s = store.resolve("gcal").unwrap();
        assert_eq!(s.id, "google-calendar");
    }

    #[test]
    fn resolve_normalized() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("my-server"));
        // my_server matches my-server after normalization
        let s = store.resolve("my_server").unwrap();
        assert_eq!(s.id, "my-server");
    }

    #[test]
    fn resolve_prefix() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("sequential-thinking"));
        let s = store.resolve("seq").unwrap();
        assert_eq!(s.id, "sequential-thinking");
    }

    #[test]
    fn resolve_substring() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("google-calendar"));
        let s = store.resolve("calendar").unwrap();
        assert_eq!(s.id, "google-calendar");
    }

    #[test]
    fn resolve_ambiguous() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("google-calendar"));
        store.add_server(make_server("google-drive"));
        let result = store.resolve("google");
        assert!(matches!(
            result,
            Err(ClaudineError::McpAmbiguousMatch { .. })
        ));
    }

    #[test]
    fn resolve_not_found() {
        let store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        assert!(store.resolve("nothing").is_err());
    }

    #[test]
    fn save_and_reload() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("catalog.json");

        let mut store = McpCatalogStore::load_from(&path).unwrap();
        store.add_server(make_server("persisted"));
        store.save_to(&path).unwrap();

        let reloaded = McpCatalogStore::load_from(&path).unwrap();
        assert!(reloaded.get_server("persisted").is_some());
    }

    #[test]
    fn list_servers_sorted() {
        let mut store = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        store.add_server(make_server("z-last"));
        store.add_server(make_server("a-first"));
        store.add_server(make_server("m-middle"));

        let list = store.list_servers();
        let ids: Vec<&str> = list.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["a-first", "m-middle", "z-last"]);
    }
}
