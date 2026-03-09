use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::config::backup::create_backup;
use crate::config::atomic::atomic_write;
use crate::error::{ClaudineError, Result};
use crate::events::Provider;

use super::catalog::McpCatalogStore;
use super::state::{McpProviderStateStore, Scope};
use super::types::{McpServer, McpTransport};

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

// ---------------------------------------------------------------------------
// Exporter
// ---------------------------------------------------------------------------

/// Export/sync Claudine-managed MCP servers to native provider configs.
pub struct McpExporter<'a> {
    #[allow(dead_code)]
    catalog: &'a McpCatalogStore,
    state: &'a McpProviderStateStore,
}

impl<'a> McpExporter<'a> {
    pub fn new(catalog: &'a McpCatalogStore, state: &'a McpProviderStateStore) -> Self {
        Self { catalog, state }
    }

    /// Sync servers to a provider's native config.
    ///
    /// When `apply` is `false`, returns what would change without writing.
    pub fn sync_provider(
        &self,
        provider: Provider,
        scope: &Scope,
        servers: &[McpServer],
        apply: bool,
    ) -> Result<SyncReport> {
        let config_path = native_config_path(provider, scope)?;

        // Determine which entries are managed (we can remove them)
        let managed_ids: Vec<String> = self
            .state
            .managed_entries_for_provider(provider, scope)
            .iter()
            .map(|e| e.native_name.clone())
            .collect();

        let mut report = SyncReport {
            written: servers.iter().map(|s| s.id.clone()).collect(),
            removed: Vec::new(),
            preserved: Vec::new(),
            applied: apply,
        };

        if !apply {
            // Dry-run: compute what we'd remove
            if config_path.exists() {
                let existing = read_existing_native_servers(provider, &config_path)?;
                for name in existing {
                    if managed_ids.contains(&name)
                        && !servers.iter().any(|s| s.id == name || s.aliases.contains(&name))
                    {
                        report.removed.push(name.clone());
                    } else if !servers.iter().any(|s| s.id == name || s.aliases.contains(&name)) {
                        report.preserved.push(name);
                    }
                }
            }
            return Ok(report);
        }

        // Create backup if config exists
        if config_path.exists() {
            create_backup(&config_path, provider)?;
        }

        match provider {
            Provider::Claude => write_claude_mcp(servers, &config_path, &managed_ids)?,
            Provider::Codex => write_codex_mcp(servers, &config_path, &managed_ids)?,
            Provider::Gemini => write_gemini_mcp(servers, &config_path, &managed_ids)?,
            Provider::OpenCode => write_opencode_mcp(servers, &config_path, &managed_ids)?,
            Provider::RooCode => write_roo_mcp(servers, &config_path, &managed_ids)?,
            _ => {
                return Err(ClaudineError::McpProviderNotSupported {
                    provider: provider.as_slug().into(),
                    reason: "export not implemented".into(),
                });
            }
        }

        Ok(report)
    }
}

// ---------------------------------------------------------------------------
// Native config path resolution
// ---------------------------------------------------------------------------

fn native_config_path(provider: Provider, scope: &Scope) -> Result<PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    match (provider, scope) {
        (Provider::Claude, Scope::User) => Ok(home.join(".claude.json")),
        (Provider::Claude, Scope::Repo(root)) => Ok(root.join(".mcp.json")),
        (Provider::Codex, Scope::User) => Ok(home.join(".codex").join("config.toml")),
        (Provider::Codex, Scope::Repo(root)) => Ok(root.join(".codex").join("config.toml")),
        (Provider::Gemini, Scope::User) => Ok(home.join(".gemini").join("settings.json")),
        (Provider::Gemini, Scope::Repo(root)) => Ok(root.join(".gemini").join("settings.json")),
        (Provider::OpenCode, Scope::User) => {
            Ok(home.join(".config").join("opencode").join("opencode.json"))
        }
        (Provider::OpenCode, Scope::Repo(root)) => Ok(root.join("opencode.json")),
        (Provider::RooCode, Scope::Repo(root)) => Ok(root.join(".roo").join("mcp.json")),
        _ => Err(ClaudineError::McpProviderNotSupported {
            provider: provider.as_slug().into(),
            reason: "no config path for scope".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Read existing native servers (names only)
// ---------------------------------------------------------------------------

fn read_existing_native_servers(provider: Provider, config_path: &Path) -> Result<Vec<String>> {
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    match provider {
        Provider::Claude | Provider::Gemini | Provider::RooCode => {
            let content = fs::read_to_string(config_path)?;
            let doc: Value = serde_json::from_str(&content)?;
            Ok(doc
                .get("mcpServers")
                .and_then(|v| v.as_object())
                .map(|obj| obj.keys().cloned().collect())
                .unwrap_or_default())
        }
        Provider::Codex => {
            let content = fs::read_to_string(config_path)?;
            let doc: toml_edit::DocumentMut = content.parse().map_err(ClaudineError::TomlParse)?;
            Ok(doc
                .get("mcp_servers")
                .and_then(|v| v.as_table())
                .map(|t| t.iter().map(|(k, _)| k.to_string()).collect())
                .unwrap_or_default())
        }
        Provider::OpenCode => {
            let content = fs::read_to_string(config_path)?;
            let doc: Value = serde_json::from_str(&content)?;
            Ok(doc
                .get("mcp")
                .and_then(|v| v.as_object())
                .map(|obj| obj.keys().cloned().collect())
                .unwrap_or_default())
        }
        _ => Ok(Vec::new()),
    }
}

// ---------------------------------------------------------------------------
// Per-provider writers
// ---------------------------------------------------------------------------

/// Write MCP servers to Claude config.
///
/// Preserves non-MCP parts of the config; only removes managed entries.
fn write_claude_mcp(
    servers: &[McpServer],
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
        let mut entry = Map::new();
        if let Some(ref cmd) = server.command {
            entry.insert("command".into(), json!(cmd));
        }
        if !server.args.is_empty() {
            entry.insert("args".into(), json!(server.args));
        }
        if !server.env.is_empty() {
            entry.insert("env".into(), json!(server.env));
        }
        if let Some(ref cwd) = server.cwd {
            entry.insert("cwd".into(), json!(cwd));
        }
        if let Some(ref url) = server.url {
            entry.insert("url".into(), json!(url));
        }
        match server.transport {
            McpTransport::Sse => {
                entry.insert("type".into(), json!("sse"));
            }
            McpTransport::Http => {
                entry.insert("type".into(), json!("http"));
            }
            McpTransport::Stdio => {}
        }
        obj.insert(server.id.clone(), Value::Object(entry));
    }

    let output = serde_json::to_string_pretty(&doc)?;
    atomic_write(config_path, output.as_bytes())
}

/// Write MCP servers to Codex config (TOML).
fn write_codex_mcp(
    servers: &[McpServer],
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
        let mut table = toml_edit::Table::new();
        if let Some(ref cmd) = server.command {
            table["command"] = toml_edit::value(cmd.as_str());
        }
        if !server.args.is_empty() {
            let mut arr = toml_edit::Array::new();
            for arg in &server.args {
                arr.push(arg.as_str());
            }
            table["args"] = toml_edit::value(arr);
        }
        if !server.env.is_empty() {
            let mut env_table = toml_edit::Table::new();
            let mut sorted_env: Vec<_> = server.env.iter().collect();
            sorted_env.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in sorted_env {
                env_table[k.as_str()] = toml_edit::value(v.as_str());
            }
            table["env"] = toml_edit::Item::Table(env_table);
        }
        if let Some(ref url) = server.url {
            table["url"] = toml_edit::value(url.as_str());
        }
        mcp_table[&server.id] = toml_edit::Item::Table(table);
    }

    atomic_write(config_path, doc.to_string().as_bytes())
}

/// Write MCP servers to Gemini config (JSON).
fn write_gemini_mcp(
    servers: &[McpServer],
    config_path: &Path,
    managed_names: &[String],
) -> Result<()> {
    // Same JSON format as Claude
    write_claude_mcp(servers, config_path, managed_names)
}

/// Write MCP servers to OpenCode config (JSON).
fn write_opencode_mcp(
    servers: &[McpServer],
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
        let mut entry = Map::new();
        match server.transport {
            McpTransport::Stdio => {
                entry.insert("type".into(), json!("local"));
                if let Some(ref cmd) = server.command {
                    entry.insert("command".into(), json!(cmd));
                }
            }
            McpTransport::Http | McpTransport::Sse => {
                entry.insert("type".into(), json!("remote"));
                if let Some(ref url) = server.url {
                    entry.insert("url".into(), json!(url));
                }
            }
        }
        if !server.env.is_empty() {
            entry.insert("environment".into(), json!(server.env));
        }
        if !server.headers.is_empty() {
            entry.insert("headers".into(), json!(server.headers));
        }
        obj.insert(server.id.clone(), Value::Object(entry));
    }

    let output = serde_json::to_string_pretty(&doc)?;
    atomic_write(config_path, output.as_bytes())
}

/// Write MCP servers to Roo Code config (JSON).
fn write_roo_mcp(
    servers: &[McpServer],
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
        let mut entry = Map::new();
        if let Some(ref cmd) = server.command {
            entry.insert("command".into(), json!(cmd));
        }
        if !server.args.is_empty() {
            entry.insert("args".into(), json!(server.args));
        }
        if !server.env.is_empty() {
            entry.insert("env".into(), json!(server.env));
        }
        if let Some(ref url) = server.url {
            entry.insert("url".into(), json!(url));
        }
        match server.transport {
            McpTransport::Sse => {
                entry.insert("transportType".into(), json!("sse"));
            }
            McpTransport::Http => {
                entry.insert("transportType".into(), json!("streamableHttp"));
            }
            McpTransport::Stdio => {
                entry.insert("transportType".into(), json!("stdio"));
            }
        }
        obj.insert(server.id.clone(), Value::Object(entry));
    }

    let output = serde_json::to_string_pretty(&doc)?;
    atomic_write(config_path, output.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;

    fn make_stdio_server(id: &str) -> McpServer {
        use chrono::Utc;
        use super::super::types::McpServerMetadata;

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

    #[test]
    fn write_claude_creates_new_config() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("claude.json");

        write_claude_mcp(&[make_stdio_server("test")], &config, &[]).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
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

        write_claude_mcp(&[make_stdio_server("new-server")], &config, &[]).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
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

        write_claude_mcp(
            &[make_stdio_server("new")],
            &config,
            &["managed-one".into()],
        )
        .unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
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

        write_codex_mcp(&[server], &config, &[]).unwrap();

        let content = fs::read_to_string(&config).unwrap();
        let doc: toml_edit::DocumentMut = content.parse().unwrap();
        assert!(doc["mcp_servers"]["test"].is_table());
    }

    #[test]
    fn write_opencode_uses_correct_format() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("opencode.json");

        write_opencode_mcp(&[make_stdio_server("test")], &config, &[]).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        assert_eq!(content["mcp"]["test"]["type"], "local");
    }

    #[test]
    fn write_roo_uses_transport_type() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("mcp.json");

        write_roo_mcp(&[make_stdio_server("test")], &config, &[]).unwrap();

        let content: Value =
            serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["test"]["transportType"], "stdio");
    }
}
