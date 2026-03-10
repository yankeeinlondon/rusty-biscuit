use std::collections::HashMap;
use std::path::Component;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

/// MCP transport protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
}

// ---------------------------------------------------------------------------
// Server metadata
// ---------------------------------------------------------------------------

/// Provenance and bookkeeping metadata for a catalog entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerMetadata {
    /// Human-readable description of what the server does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Where this entry was originally created from (e.g. `"codex:user"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_from: Option<String>,

    /// Content hash of the normalized definition (for deduplication).
    pub fingerprint: String,

    /// When this entry was first added to the catalog.
    pub created_at: DateTime<Utc>,

    /// When this entry was last updated.
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Server definition
// ---------------------------------------------------------------------------

/// A normalized MCP server definition — the superset of all provider formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Stable slug identifier (e.g. `"sequential-thinking"`).
    pub id: String,

    /// Alternative names that also resolve to this server.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,

    /// Transport protocol.
    pub transport: McpTransport,

    /// Executable command (Stdio transport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Command arguments (Stdio transport).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// Working directory for the command (Stdio transport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,

    /// Environment variables passed to the command (Stdio transport).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    /// Server URL (Http / Sse transport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// HTTP headers sent with requests.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Allowlist of tool names the client should expose.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_tools: Vec<String>,

    /// Denylist of tool names the client should hide.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_tools: Vec<String>,

    /// Whether the server is considered required (failure = session failure).
    #[serde(default)]
    pub required: bool,

    /// Provenance and bookkeeping.
    pub metadata: McpServerMetadata,

    /// Escape-hatch overrides keyed by provider slug.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub provider_overrides: HashMap<String, serde_json::Value>,
}

impl McpServer {
    /// Compute a deterministic fingerprint from the normalized definition.
    ///
    /// Fields excluded: `id`, `aliases`, `metadata`, `provider_overrides`.
    /// The remaining fields are serialized as sorted JSON and hashed with BLAKE3.
    pub fn fingerprint(&self) -> String {
        use serde_json::json;

        let mut env_sorted: Vec<(&String, &String)> = self.env.iter().collect();
        env_sorted.sort_by_key(|(k, _)| *k);

        let mut headers_sorted: Vec<(&String, &String)> = self.headers.iter().collect();
        headers_sorted.sort_by_key(|(k, _)| *k);

        let canonical = json!({
            "transport": self.transport,
            "command": self.command,
            "args": self.args,
            "cwd": self.cwd,
            "env": env_sorted,
            "url": self.url,
            "headers": headers_sorted,
            "enabled_tools": self.enabled_tools,
            "disabled_tools": self.disabled_tools,
            "required": self.required,
        });

        let serialized = serde_json::to_string(&canonical).expect("fingerprint serialization");
        biscuit_hash::blake3_hash(&serialized)
    }

    /// Return provider-specific override data as an object, if present.
    pub fn provider_override_object(&self, provider_slug: &str) -> Option<&Map<String, Value>> {
        self.provider_overrides
            .get(provider_slug)
            .and_then(Value::as_object)
    }

    /// Set a provider-specific override field, creating the provider object.
    pub fn set_provider_override(&mut self, provider_slug: &str, key: &str, value: Value) {
        let entry = self
            .provider_overrides
            .entry(provider_slug.to_string())
            .or_insert_with(|| Value::Object(Map::new()));

        let object = entry.as_object_mut().expect("provider override object");
        object.insert(key.to_string(), value);
    }
}

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

/// Top-level catalog file (`~/.claudine/mcp/catalog.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCatalog {
    /// Schema version (always 1 for now).
    pub version: u32,

    /// Server definitions keyed by ID.
    pub servers: HashMap<String, McpServer>,
}

impl Default for McpCatalog {
    fn default() -> Self {
        Self {
            version: 1,
            servers: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

/// A defaults file listing active server IDs.
///
/// Used for both user-scope (`~/.claudine/mcp/defaults.json`) and
/// repo-scope (`<repo>/.claudine/mcp.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDefaults {
    /// Schema version (always 1 for now).
    pub version: u32,

    /// Ordered list of catalog IDs that are active by default.
    pub defaults: Vec<String>,
}

impl Default for McpDefaults {
    fn default() -> Self {
        Self {
            version: 1,
            defaults: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Provider state
// ---------------------------------------------------------------------------

/// Whether a provider-state entry was imported or is Claudine-managed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpOrigin {
    /// Imported from a native provider config.
    Imported,
    /// Created and owned by Claudine.
    Managed,
}

/// A single provider-state entry tracking the relationship between a catalog
/// entry and a native provider config entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStateEntry {
    /// The catalog ID this entry maps to.
    pub catalog_id: String,

    /// The name used in the native provider config.
    pub native_name: String,

    /// Path to the native config file.
    pub source: String,

    /// Whether this was imported or is Claudine-managed.
    pub origin: McpOrigin,

    /// When this entry was last observed in the native config.
    pub last_seen: DateTime<Utc>,
}

/// Scope for provider-state entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpScope {
    User,
    Repo,
}

/// Per-scope entries for a single provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderScopeEntries {
    /// User-scope entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user: Vec<ProviderStateEntry>,

    /// Repo-scope entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repo: Vec<ProviderStateEntry>,
}

/// Per-repo provider state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoProviderState {
    /// Provider entries for this repo.
    pub providers: HashMap<String, ProviderScopeEntries>,
}

/// Top-level provider-state file (`~/.claudine/mcp/provider-state.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProviderState {
    /// Schema version.
    pub version: u32,

    /// User-scope provider entries (keyed by provider slug).
    #[serde(default)]
    pub providers: HashMap<String, ProviderScopeEntries>,

    /// Per-repo provider entries (keyed by absolute repo path).
    #[serde(default)]
    pub repos: HashMap<String, RepoProviderState>,
}

impl Default for McpProviderState {
    fn default() -> Self {
        Self {
            version: 1,
            providers: HashMap::new(),
            repos: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Returns the MCP data directory (`~/.claudine/mcp/`).
pub fn mcp_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claudine")
        .join("mcp")
}

/// Returns the catalog path (`~/.claudine/mcp/catalog.json`).
pub fn catalog_path() -> PathBuf {
    mcp_dir().join("catalog.json")
}

/// Returns the user defaults path (`~/.claudine/mcp/defaults.json`).
pub fn defaults_path() -> PathBuf {
    mcp_dir().join("defaults.json")
}

/// Returns the provider-state path (`~/.claudine/mcp/provider-state.json`).
pub fn provider_state_path() -> PathBuf {
    mcp_dir().join("provider-state.json")
}

/// Returns the repo-scope defaults path (`<repo_root>/.claudine/mcp.json`).
pub fn repo_defaults_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".claudine").join("mcp.json")
}

// ---------------------------------------------------------------------------
// Slug generation
// ---------------------------------------------------------------------------

/// Generate a stable slug ID from a server name.
///
/// Converts to lowercase, replaces non-alphanumeric with hyphens,
/// collapses consecutive hyphens, and trims leading/trailing hyphens.
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut prev_hyphen = true; // suppress leading hyphens

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }

    // trim trailing hyphen
    if slug.ends_with('-') {
        slug.pop();
    }

    slug
}

/// Derive a stable server name when a provider does not supply one explicitly.
///
/// The derivation order is:
/// 1. explicit name, when present
/// 2. launcher-aware executable or script name for local servers
/// 3. xxHash fallback of the normalized server definition
pub fn derive_server_name(server: &McpServer, explicit_name: Option<&str>) -> String {
    if let Some(name) = explicit_name.map(str::trim).filter(|value| !value.is_empty()) {
        return name.to_string();
    }

    if let Some(name) = derive_launcher_aware_name(server.command.as_deref(), &server.args) {
        return name;
    }

    let canonical = serde_json::json!({
        "transport": server.transport,
        "command": server.command,
        "args": server.args,
        "cwd": server.cwd,
        "env": server.env,
        "url": server.url,
        "headers": server.headers,
        "enabled_tools": server.enabled_tools,
        "disabled_tools": server.disabled_tools,
        "required": server.required,
    });

    let hash = biscuit_hash::xx_hash(&canonical.to_string());
    format!("mcp-{hash:016x}")
}

/// Derive a stable catalog ID from an optional explicit name or the server definition.
pub fn derive_server_id(server: &McpServer, explicit_name: Option<&str>) -> String {
    slugify(&derive_server_name(server, explicit_name))
}

fn derive_launcher_aware_name(command: Option<&str>, args: &[String]) -> Option<String> {
    let command = command?;
    let executable = Path::new(command)
        .components()
        .next_back()
        .and_then(component_name)
        .unwrap_or(command);
    let lowered = executable.to_ascii_lowercase();

    let launchers = [
        "uv", "uvx", "python", "python3", "node", "npm", "npx", "pnpm", "yarn", "bun",
    ];

    if launchers.contains(&lowered.as_str()) {
        for arg in args {
            if arg.starts_with('-') {
                continue;
            }

            let candidate = if lowered == "npm"
                || lowered == "npx"
                || lowered == "pnpm"
                || lowered == "yarn"
                || lowered == "bun"
            {
                arg.rsplit('/').next().unwrap_or(arg)
            } else {
                Path::new(arg)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or(arg)
            };

            if !candidate.trim().is_empty() {
                return Some(candidate.to_string());
            }
        }
    }

    if executable.trim().is_empty() {
        None
    } else {
        Some(executable.to_string())
    }
}

fn component_name(component: Component<'_>) -> Option<&str> {
    component.as_os_str().to_str()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_deterministic() {
        let server = test_server("test-server");
        let fp1 = server.fingerprint();
        let fp2 = server.fingerprint();
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn fingerprint_ignores_id_and_aliases() {
        let mut s1 = test_server("server-a");
        let mut s2 = test_server("server-b");
        s2.aliases = vec!["alias-x".into()];

        // Same definition, different id/aliases → same fingerprint
        assert_eq!(s1.fingerprint(), s2.fingerprint());

        // Different command → different fingerprint
        s1.command = Some("different-cmd".into());
        assert_ne!(s1.fingerprint(), s2.fingerprint());
    }

    #[test]
    fn fingerprint_ignores_metadata() {
        let s1 = test_server("a");
        let mut s2 = test_server("b");
        s2.metadata.description = Some("totally different description".into());
        s2.metadata.created_from = Some("codex:user".into());

        assert_eq!(s1.fingerprint(), s2.fingerprint());
    }

    #[test]
    fn fingerprint_env_order_independent() {
        let mut s1 = test_server("a");
        s1.env.insert("A".into(), "1".into());
        s1.env.insert("B".into(), "2".into());

        let mut s2 = test_server("b");
        s2.env.insert("B".into(), "2".into());
        s2.env.insert("A".into(), "1".into());

        assert_eq!(s1.fingerprint(), s2.fingerprint());
    }

    #[test]
    fn serde_round_trip_catalog() {
        let mut catalog = McpCatalog::default();
        catalog.servers.insert("test".into(), test_server("test"));

        let json = serde_json::to_string_pretty(&catalog).unwrap();
        let parsed: McpCatalog = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, 1);
        assert!(parsed.servers.contains_key("test"));
    }

    #[test]
    fn serde_round_trip_defaults() {
        let defaults = McpDefaults {
            version: 1,
            defaults: vec!["a".into(), "b".into()],
        };
        let json = serde_json::to_string(&defaults).unwrap();
        let parsed: McpDefaults = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.defaults, vec!["a", "b"]);
    }

    #[test]
    fn serde_round_trip_provider_state() {
        let state = McpProviderState::default();
        let json = serde_json::to_string(&state).unwrap();
        let parsed: McpProviderState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, 1);
    }

    #[test]
    fn path_helpers_return_expected_locations() {
        let mcp = mcp_dir();
        assert!(mcp.ends_with(".claudine/mcp"));

        assert!(catalog_path().ends_with("catalog.json"));
        assert!(defaults_path().ends_with("defaults.json"));
        assert!(provider_state_path().ends_with("provider-state.json"));

        let repo = repo_defaults_path(Path::new("/tmp/my-repo"));
        assert_eq!(repo, PathBuf::from("/tmp/my-repo/.claudine/mcp.json"));
    }

    #[test]
    fn slugify_basic_cases() {
        assert_eq!(slugify("Google Calendar"), "google-calendar");
        assert_eq!(slugify("my_server"), "my-server");
        assert_eq!(slugify("  spaces  "), "spaces");
        assert_eq!(slugify("UPPER-CASE"), "upper-case");
        assert_eq!(slugify("a--b"), "a-b");
        assert_eq!(slugify("sequential-thinking"), "sequential-thinking");
    }

    #[test]
    fn derive_server_name_prefers_explicit_name() {
        let server = test_server("ignored");
        assert_eq!(derive_server_name(&server, Some("Calendar MCP")), "Calendar MCP");
    }

    #[test]
    fn derive_server_name_uses_launcher_aware_argument() {
        let mut server = test_server("ignored");
        server.command = Some("npx".into());
        server.args = vec!["-y".into(), "@upstash/context7-mcp".into()];
        assert_eq!(
            derive_server_name(&server, None),
            "context7-mcp".to_string()
        );
        assert_eq!(derive_server_id(&server, None), "context7-mcp");
    }

    #[test]
    fn derive_server_name_falls_back_to_hash() {
        let mut server = test_server("ignored");
        server.command = None;
        server.args.clear();
        server.url = Some("https://example.com/mcp".into());

        let derived = derive_server_name(&server, None);
        assert!(derived.starts_with("mcp-"));
        assert_eq!(derived.len(), 20);
    }

    fn test_server(id: &str) -> McpServer {
        McpServer {
            id: id.into(),
            aliases: Vec::new(),
            transport: McpTransport::Stdio,
            command: Some("npx".into()),
            args: vec!["-y".into(), "@test/server".into()],
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
}
