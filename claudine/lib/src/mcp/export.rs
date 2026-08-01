use std::fs;
use std::path::Path;

use chrono::Utc;
use serde_json::{Map, Value, json};

use crate::config::atomic::atomic_write;
use crate::config::backup::create_backup;
use crate::error::{ClaudineError, Result};
use crate::provider::Provider;
use crate::provider::provider_info;

use super::catalog::McpCatalogStore;
use super::state::{McpProviderStateStore, Scope};
use super::types::{McpOrigin, McpServer, McpTransport, ProviderStateEntry};

// ---------------------------------------------------------------------------
// Sync report
// ---------------------------------------------------------------------------

/// Report summarizing a sync/export operation.
#[derive(Debug)]
pub struct SyncReport {
    /// Servers written to the native config.
    pub written: Vec<String>,
    /// Managed entries removed (no longer in defaults).
    pub removed: Vec<String>,
    /// Foreign entries preserved (not created by Claudine).
    pub preserved: Vec<String>,
    /// Whether changes were actually applied.
    pub applied: bool,
}

/// A server prepared for export to a provider's native MCP config.
///
/// Carries the catalog ID, the native (per-provider) server name resolved
/// from provider state, and a reference to the canonical [`McpServer`]
/// definition.
#[derive(Debug, Clone)]
pub struct ExportServer<'a> {
    /// Canonical catalog ID.
    pub catalog_id: &'a str,
    /// Provider-native server name (may differ from `catalog_id`).
    pub native_name: String,
    /// Reference to the catalog's canonical server definition.
    pub server: &'a McpServer,
}

// ---------------------------------------------------------------------------
// Exporter
// ---------------------------------------------------------------------------

/// Export/sync Claudine-managed MCP servers to native provider configs.
pub struct McpExporter<'a> {
    #[allow(dead_code)]
    catalog: &'a McpCatalogStore,
    state: &'a mut McpProviderStateStore,
}

impl<'a> McpExporter<'a> {
    pub fn new(catalog: &'a McpCatalogStore, state: &'a mut McpProviderStateStore) -> Self {
        Self { catalog, state }
    }

    /// Sync servers to a provider's native config.
    ///
    /// When `apply` is `false`, returns what would change without writing.
    pub fn sync_provider(
        &mut self,
        provider: Provider,
        scope: &Scope,
        servers: &[McpServer],
        apply: bool,
    ) -> Result<SyncReport> {
        let info = provider_info(provider);
        let config_path = info.mcp.native_config_path(scope).ok_or_else(|| {
            ClaudineError::McpProviderNotSupported {
                provider,
                reason: "no config path for scope".into(),
            }
        })?;
        let existing = info.mcp.read_existing_native_servers(&config_path)?;
        let managed_entries: Vec<_> = self
            .state
            .managed_entries_for_provider(provider, scope)
            .into_iter()
            .cloned()
            .collect();
        let managed_names: Vec<String> = managed_entries
            .iter()
            .map(|entry| entry.native_name.clone())
            .collect();
        let export_servers = build_export_servers(provider, scope, self.state, servers);
        let desired_names: std::collections::HashSet<String> = export_servers
            .iter()
            .map(|entry| entry.native_name.clone())
            .collect();

        let mut report = SyncReport {
            written: export_servers
                .iter()
                .map(|entry| entry.native_name.clone())
                .collect(),
            removed: managed_entries
                .iter()
                .filter(|entry| !desired_names.contains(&entry.native_name))
                .map(|entry| entry.native_name.clone())
                .collect(),
            preserved: existing
                .iter()
                .filter(|name| !desired_names.contains(*name) && !managed_names.contains(*name))
                .cloned()
                .collect(),
            applied: apply,
        };
        report.written.sort();
        report.removed.sort();
        report.preserved.sort();

        if !apply {
            return Ok(report);
        }

        // Create backup if config exists
        if config_path.exists() {
            create_backup(&config_path, provider)?;
        }

        info.mcp
            .write_native_config(&export_servers, &config_path, &managed_names)?;

        let source = biscuit_file::to_portable_string(&config_path);
        for removed in managed_entries
            .iter()
            .filter(|entry| !desired_names.contains(&entry.native_name))
        {
            self.state
                .remove_entry(provider, scope, &removed.catalog_id);
        }

        for entry in &export_servers {
            self.state.record_managed(
                provider,
                scope,
                ProviderStateEntry {
                    catalog_id: entry.catalog_id.to_string(),
                    native_name: entry.native_name.clone(),
                    source: source.clone(),
                    origin: McpOrigin::Managed,
                    last_seen: Utc::now(),
                },
            );
        }

        Ok(report)
    }
}

fn build_export_servers<'a>(
    provider: Provider,
    scope: &Scope,
    state: &McpProviderStateStore,
    servers: &'a [McpServer],
) -> Vec<ExportServer<'a>> {
    servers
        .iter()
        .map(|server| ExportServer {
            catalog_id: server.id.as_str(),
            native_name: state
                .entry_for_catalog_id(provider, scope, &server.id)
                .map(|entry| entry.native_name.clone())
                .unwrap_or_else(|| server.id.clone()),
            server,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Read existing native servers (names only)
// ---------------------------------------------------------------------------

/// Helper for behavior trait implementations: read MCP server names from a
/// config file with a top-level `mcpServers` JSON object (Claude, Gemini).
pub(crate) fn read_existing_json_mcp_servers(config_path: &Path) -> Result<Vec<String>> {
    if !config_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(config_path)?;
    let doc: Value = serde_json::from_str(&content)?;
    Ok(doc
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default())
}

/// Helper: read MCP server names from a TOML config file with `[mcp_servers]`
/// (Codex).
pub(crate) fn read_existing_codex_mcp_servers(config_path: &Path) -> Result<Vec<String>> {
    if !config_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(config_path)?;
    let doc: toml_edit::DocumentMut = content.parse().map_err(ClaudineError::TomlParse)?;
    Ok(doc
        .get("mcp_servers")
        .and_then(|v| v.as_table())
        .map(|t| t.iter().map(|(k, _)| k.to_string()).collect())
        .unwrap_or_default())
}

/// Helper: read MCP server names from an OpenCode JSON config (`mcp` key).
pub(crate) fn read_existing_opencode_mcp_servers(config_path: &Path) -> Result<Vec<String>> {
    if !config_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(config_path)?;
    let doc: Value = serde_json::from_str(&content)?;
    Ok(doc
        .get("mcp")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Per-provider writers
// ---------------------------------------------------------------------------

/// Write MCP servers to Claude config.
///
/// Preserves non-MCP parts of the config; only removes managed entries.
pub(crate) fn write_claude_mcp(
    servers: &[ExportServer<'_>],
    config_path: &Path,
    managed_names: &[String],
) -> Result<()> {
    let mut doc = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        serde_json::from_str::<Value>(&content)?
    } else {
        json!({})
    };

    let mcp_servers = doc
        .as_object_mut()
        .unwrap()
        .entry("mcpServers")
        .or_insert_with(|| json!({}));

    let obj = mcp_servers.as_object_mut().unwrap();

    // Remove managed entries
    for name in managed_names {
        obj.remove(name);
    }

    // Write new servers
    for server in servers {
        let server_def = server.server;
        let mut entry = Map::new();
        if let Some(ref cmd) = server_def.command {
            entry.insert("command".into(), json!(cmd));
        }
        if !server_def.args.is_empty() {
            entry.insert("args".into(), json!(server_def.args));
        }
        if !server_def.env.is_empty() {
            entry.insert("env".into(), json!(server_def.env));
        }
        if let Some(ref cwd) = server_def.cwd {
            entry.insert("cwd".into(), json!(cwd));
        }
        if let Some(ref url) = server_def.url {
            entry.insert("url".into(), json!(url));
        }
        match server_def.transport {
            McpTransport::Sse => {
                entry.insert("type".into(), json!("sse"));
            }
            McpTransport::Http => {
                entry.insert("type".into(), json!("http"));
            }
            McpTransport::Stdio => {}
        }
        obj.insert(server.native_name.clone(), Value::Object(entry));
    }

    let output = serde_json::to_string_pretty(&doc)?;
    Ok(atomic_write(config_path, output.as_bytes())?)
}

/// Write MCP servers to Codex config (TOML).
pub(crate) fn write_codex_mcp(
    servers: &[ExportServer<'_>],
    config_path: &Path,
    managed_names: &[String],
) -> Result<()> {
    let mut doc: toml_edit::DocumentMut = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        content.parse().map_err(ClaudineError::TomlParse)?
    } else {
        toml_edit::DocumentMut::new()
    };

    // Get or create mcp_servers table
    if doc.get("mcp_servers").is_none() {
        doc["mcp_servers"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    let mcp_table = doc["mcp_servers"].as_table_mut().unwrap();

    // Remove managed entries
    for name in managed_names {
        mcp_table.remove(name);
    }

    // Write new servers
    for server in servers {
        let server_def = server.server;
        let mut table = toml_edit::Table::new();
        if let Some(ref cmd) = server_def.command {
            table["command"] = toml_edit::value(cmd.as_str());
        }
        if !server_def.args.is_empty() {
            let mut arr = toml_edit::Array::new();
            for arg in &server_def.args {
                arr.push(arg.as_str());
            }
            table["args"] = toml_edit::value(arr);
        }
        if !server_def.env.is_empty() {
            let mut env_table = toml_edit::Table::new();
            let mut sorted_env: Vec<_> = server_def.env.iter().collect();
            sorted_env.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in sorted_env {
                env_table[k.as_str()] = toml_edit::value(v.as_str());
            }
            table["env"] = toml_edit::Item::Table(env_table);
        }
        if let Some(ref url) = server_def.url {
            table["url"] = toml_edit::value(url.as_str());
        }
        if let Some(ref cwd) = server_def.cwd {
            table["cwd"] = toml_edit::value(cwd.to_string_lossy().as_ref());
        }
        if !server_def.headers.is_empty() {
            let mut header_table = toml_edit::Table::new();
            for (key, value) in sorted_string_pairs(&server_def.headers) {
                header_table[key.as_str()] = toml_edit::value(value.as_str());
            }
            table["http_headers"] = toml_edit::Item::Table(header_table);
        }
        if !server_def.enabled_tools.is_empty() {
            table["enabled_tools"] = toml_edit::value(to_toml_array(&server_def.enabled_tools));
        }
        if !server_def.disabled_tools.is_empty() {
            table["disabled_tools"] = toml_edit::value(to_toml_array(&server_def.disabled_tools));
        }
        if server_def.required {
            table["required"] = toml_edit::value(true);
        }
        if let Some(enabled) = provider_override_bool(server_def, "codex", "enabled") {
            table["enabled"] = toml_edit::value(enabled);
        }
        if let Some(value) = provider_override_string_array(server_def, "codex", "env_vars")
            && !value.is_empty()
        {
            table["env_vars"] = toml_edit::value(to_toml_array(&value));
        }
        if let Some(value) = provider_override_string(server_def, "codex", "bearer_token_env_var") {
            table["bearer_token_env_var"] = toml_edit::value(value.as_str());
        }
        if let Some(value) = provider_override_string_map(server_def, "codex", "env_http_headers")
            && !value.is_empty()
        {
            let mut header_table = toml_edit::Table::new();
            for (key, value) in sorted_string_pairs(&value) {
                header_table[key.as_str()] = toml_edit::value(value.as_str());
            }
            table["env_http_headers"] = toml_edit::Item::Table(header_table);
        }
        if let Some(value) = provider_override_i64(server_def, "codex", "startup_timeout_sec") {
            table["startup_timeout_sec"] = toml_edit::value(value);
        }
        if let Some(value) = provider_override_i64(server_def, "codex", "tool_timeout_sec") {
            table["tool_timeout_sec"] = toml_edit::value(value);
        }

        mcp_table[&server.native_name] = toml_edit::Item::Table(table);
    }

    Ok(atomic_write(config_path, doc.to_string().as_bytes())?)
}

/// Write MCP servers to Gemini config (JSON).
pub(crate) fn write_gemini_mcp(
    servers: &[ExportServer<'_>],
    config_path: &Path,
    managed_names: &[String],
) -> Result<()> {
    let mut doc = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        serde_json::from_str::<Value>(&content)?
    } else {
        json!({})
    };

    let mcp_servers = doc
        .as_object_mut()
        .unwrap()
        .entry("mcpServers")
        .or_insert_with(|| json!({}));
    let obj = mcp_servers.as_object_mut().unwrap();

    for name in managed_names {
        obj.remove(name);
    }

    for server in servers {
        let server_def = server.server;
        let mut entry = Map::new();
        if let Some(ref cmd) = server_def.command {
            entry.insert("command".into(), json!(cmd));
        }
        if !server_def.args.is_empty() {
            entry.insert("args".into(), json!(server_def.args));
        }
        if !server_def.env.is_empty() {
            entry.insert("env".into(), json!(server_def.env));
        }
        if let Some(ref url) = server_def.url {
            entry.insert("url".into(), json!(url));
        }
        if !server_def.enabled_tools.is_empty() {
            entry.insert("include-tools".into(), json!(server_def.enabled_tools));
        }
        if !server_def.disabled_tools.is_empty() {
            entry.insert("exclude-tools".into(), json!(server_def.disabled_tools));
        }
        for field in ["description", "timeout", "trust"] {
            if let Some(value) = provider_override_value(server_def, "gemini", field) {
                entry.insert(field.to_string(), value.clone());
            }
        }

        obj.insert(server.native_name.clone(), Value::Object(entry));
    }

    let output = serde_json::to_string_pretty(&doc)?;
    Ok(atomic_write(config_path, output.as_bytes())?)
}

/// Write MCP servers to OpenCode config (JSON).
pub(crate) fn write_opencode_mcp(
    servers: &[ExportServer<'_>],
    config_path: &Path,
    managed_names: &[String],
) -> Result<()> {
    let mut doc = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        serde_json::from_str::<Value>(&content)?
    } else {
        json!({})
    };

    let mcp = doc
        .as_object_mut()
        .unwrap()
        .entry("mcp")
        .or_insert_with(|| json!({}));
    let obj = mcp.as_object_mut().unwrap();

    // Remove managed entries
    for name in managed_names {
        obj.remove(name);
    }

    // Write new servers
    for server in servers {
        let server_def = server.server;
        let mut entry = Map::new();
        match server_def.transport {
            McpTransport::Stdio => {
                entry.insert("type".into(), json!("local"));
                if let Some(ref cmd) = server_def.command {
                    let command = std::iter::once(cmd.clone())
                        .chain(server_def.args.iter().cloned())
                        .collect::<Vec<_>>();
                    entry.insert("command".into(), json!(command));
                }
            }
            McpTransport::Http | McpTransport::Sse => {
                entry.insert("type".into(), json!("remote"));
                if let Some(ref url) = server_def.url {
                    entry.insert("url".into(), json!(url));
                }
            }
        }
        if !server_def.env.is_empty() {
            entry.insert("environment".into(), json!(server_def.env));
        }
        if !server_def.headers.is_empty() {
            entry.insert("headers".into(), json!(server_def.headers));
        }
        for field in ["enabled", "oauth", "timeout"] {
            if let Some(value) = provider_override_value(server_def, "opencode", field) {
                entry.insert(field.to_string(), value.clone());
            }
        }
        obj.insert(server.native_name.clone(), Value::Object(entry));
    }

    let output = serde_json::to_string_pretty(&doc)?;
    Ok(atomic_write(config_path, output.as_bytes())?)
}

fn provider_override_value<'a>(
    server: &'a McpServer,
    provider_slug: &str,
    key: &str,
) -> Option<&'a Value> {
    server
        .provider_override_object(provider_slug)
        .and_then(|map| map.get(key))
}

fn provider_override_bool(server: &McpServer, provider_slug: &str, key: &str) -> Option<bool> {
    provider_override_value(server, provider_slug, key).and_then(Value::as_bool)
}

fn provider_override_i64(server: &McpServer, provider_slug: &str, key: &str) -> Option<i64> {
    provider_override_value(server, provider_slug, key).and_then(Value::as_i64)
}

fn provider_override_string(server: &McpServer, provider_slug: &str, key: &str) -> Option<String> {
    provider_override_value(server, provider_slug, key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn provider_override_string_array(
    server: &McpServer,
    provider_slug: &str,
    key: &str,
) -> Option<Vec<String>> {
    provider_override_value(server, provider_slug, key).map(|value| {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect()
    })
}

fn provider_override_string_map(
    server: &McpServer,
    provider_slug: &str,
    key: &str,
) -> Option<std::collections::HashMap<String, String>> {
    provider_override_value(server, provider_slug, key).map(|value| {
        value
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
            .collect()
    })
}

fn sorted_string_pairs(map: &std::collections::HashMap<String, String>) -> Vec<(String, String)> {
    let mut pairs: Vec<_> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

fn to_toml_array(values: &[String]) -> toml_edit::Array {
    let mut arr = toml_edit::Array::new();
    for value in values {
        arr.push(value.as_str());
    }
    arr
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use tempfile::TempDir;

    use super::*;
    use crate::mcp::catalog::McpCatalogStore;
    use crate::mcp::state::{McpProviderStateStore, Scope};
    use crate::mcp::types::ProviderStateEntry;

    fn make_stdio_server(id: &str) -> McpServer {
        use super::super::types::McpServerMetadata;
        use chrono::Utc;

        McpServer {
            id: id.into(),
            aliases: Vec::new(),
            transport: McpTransport::Stdio,
            command: Some("npx".into()),
            args: vec!["-y".into(), format!("@test/{id}")],
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

    fn export_servers<'a>(servers: &'a [McpServer]) -> Vec<ExportServer<'a>> {
        servers
            .iter()
            .map(|server| ExportServer {
                catalog_id: server.id.as_str(),
                native_name: server.id.clone(),
                server,
            })
            .collect()
    }

    #[test]
    fn write_claude_creates_new_config() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("claude.json");
        let servers = vec![make_stdio_server("test")];

        write_claude_mcp(&export_servers(&servers), &config, &[]).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        assert!(content["mcpServers"]["test"].is_object());
    }

    #[test]
    fn write_claude_preserves_existing() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("claude.json");
        fs::write(
            &config,
            r#"{"mcpServers":{"existing":{"command":"other"}},"otherKey":"preserved"}"#,
        )
        .unwrap();
        let servers = vec![make_stdio_server("new-server")];

        write_claude_mcp(&export_servers(&servers), &config, &[]).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        // existing server preserved
        assert!(content["mcpServers"]["existing"].is_object());
        // new server added
        assert!(content["mcpServers"]["new-server"].is_object());
        // non-MCP key preserved
        assert_eq!(content["otherKey"], "preserved");
    }

    #[test]
    fn write_claude_removes_managed_only() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("claude.json");
        fs::write(
            &config,
            r#"{"mcpServers":{"managed-one":{"command":"old"},"foreign":{"command":"keep"}}}"#,
        )
        .unwrap();
        let servers = vec![make_stdio_server("new")];

        write_claude_mcp(&export_servers(&servers), &config, &["managed-one".into()]).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        // managed entry removed
        assert!(content["mcpServers"]["managed-one"].is_null());
        // foreign entry preserved
        assert!(content["mcpServers"]["foreign"].is_object());
        // new entry added
        assert!(content["mcpServers"]["new"].is_object());
    }

    #[test]
    fn write_codex_creates_valid_toml() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config.toml");

        let mut server = make_stdio_server("test");
        server.env.insert("TOKEN".into(), "secret".into());
        let servers = vec![server];

        write_codex_mcp(&export_servers(&servers), &config, &[]).unwrap();

        let content = fs::read_to_string(&config).unwrap();
        let doc: toml_edit::DocumentMut = content.parse().unwrap();
        assert!(doc["mcp_servers"]["test"].is_table());
    }

    #[test]
    fn write_opencode_uses_correct_format() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("opencode.json");
        let servers = vec![make_stdio_server("test")];

        write_opencode_mcp(&export_servers(&servers), &config, &[]).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        assert_eq!(content["mcp"]["test"]["type"], "local");
    }

    #[test]
    fn sync_provider_uses_recorded_native_name_and_promotes_managed_ownership() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        let config_dir = repo_root.join(".codex");
        let config = config_dir.join("config.toml");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            &config,
            r#"
[mcp_servers.calendar]
command = "npx"
args = ["-y", "@test/google-calendar"]
"#,
        )
        .unwrap();

        let mut catalog =
            McpCatalogStore::load_from(tmp.path().join("catalog.json").as_path()).unwrap();
        let server = make_stdio_server("google-calendar");
        catalog.add_server(server.clone());

        let mut state =
            McpProviderStateStore::load_from(tmp.path().join("state.json").as_path()).unwrap();
        let scope = Scope::Repo(repo_root.clone());
        state.record_import(
            Provider::Codex,
            &scope,
            ProviderStateEntry {
                catalog_id: "google-calendar".into(),
                native_name: "calendar".into(),
                source: config.to_string_lossy().into(),
                origin: McpOrigin::Imported,
                last_seen: Utc::now(),
            },
        );

        let mut exporter = McpExporter::new(&catalog, &mut state);
        let report = exporter
            .sync_provider(Provider::Codex, &scope, &[server], true)
            .unwrap();

        assert_eq!(report.written, vec!["calendar"]);

        let doc: toml_edit::DocumentMut = fs::read_to_string(&config).unwrap().parse().unwrap();
        assert!(doc["mcp_servers"]["calendar"].is_table());
        assert!(doc["mcp_servers"].get("google-calendar").is_none());

        let entry = state
            .entry_for_catalog_id(Provider::Codex, &scope, "google-calendar")
            .unwrap();
        assert_eq!(entry.native_name, "calendar");
        assert_eq!(entry.origin, McpOrigin::Managed);
    }

    #[test]
    fn sync_provider_removes_stale_managed_entries_only() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        let config = repo_root.join(".mcp.json");
        fs::create_dir_all(&repo_root).unwrap();
        fs::write(
            &config,
            r#"{"mcpServers":{"managed-one":{"command":"old"},"foreign":{"command":"keep"}}}"#,
        )
        .unwrap();

        let mut catalog =
            McpCatalogStore::load_from(tmp.path().join("catalog.json").as_path()).unwrap();
        let server = make_stdio_server("new");
        catalog.add_server(server.clone());

        let mut state =
            McpProviderStateStore::load_from(tmp.path().join("state.json").as_path()).unwrap();
        let scope = Scope::Repo(repo_root.clone());
        state.record_managed(
            Provider::Claude,
            &scope,
            ProviderStateEntry {
                catalog_id: "old".into(),
                native_name: "managed-one".into(),
                source: config.to_string_lossy().into(),
                origin: McpOrigin::Managed,
                last_seen: Utc::now(),
            },
        );

        let mut exporter = McpExporter::new(&catalog, &mut state);
        let report = exporter
            .sync_provider(Provider::Claude, &scope, &[server], true)
            .unwrap();

        assert_eq!(report.removed, vec!["managed-one"]);
        assert_eq!(report.preserved, vec!["foreign"]);

        let content: Value = serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        assert!(content["mcpServers"]["managed-one"].is_null());
        assert!(content["mcpServers"]["foreign"].is_object());
        assert!(content["mcpServers"]["new"].is_object());
        assert!(
            state
                .entry_for_catalog_id(Provider::Claude, &scope, "old")
                .is_none()
        );
    }
}
