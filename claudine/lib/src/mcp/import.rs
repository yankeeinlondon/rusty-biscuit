use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;
use serde_json::{Value, json};

use crate::error::Result;
use crate::provider::Provider;
use crate::provider::provider_info;

use super::catalog::McpCatalogStore;
use super::state::{McpProviderStateStore, Scope};
use super::types::{
    McpOrigin, McpServer, McpServerMetadata, McpTransport, ProviderStateEntry, slugify,
};

// ---------------------------------------------------------------------------
// Import report types
// ---------------------------------------------------------------------------

/// A server that was newly added to the catalog.
#[derive(Debug, Serialize)]
pub struct ImportedEntry {
    pub catalog_id: String,
    pub provider: Provider,
    pub native_name: String,
}

/// A server that matched an existing catalog entry by fingerprint.
#[derive(Debug, Serialize)]
pub struct MergedEntry {
    pub catalog_id: String,
    pub provider: Provider,
    pub native_name: String,
    pub alias_added: Option<String>,
}

/// A naming conflict (same name, different fingerprint).
#[derive(Debug, Serialize)]
pub struct ConflictEntry {
    pub name: String,
    pub provider: Provider,
    pub new_catalog_id: String,
}

/// A server that was skipped (already tracked).
#[derive(Debug, Serialize)]
pub struct SkippedEntry {
    pub catalog_id: String,
    pub provider: Provider,
    pub native_name: String,
}

/// An error during import of a single server.
#[derive(Debug, Serialize)]
pub struct ImportError {
    pub provider: Provider,
    pub native_name: String,
    pub reason: String,
}

/// Report summarizing an import operation.
#[derive(Debug, Default, Serialize)]
pub struct ImportReport {
    pub imported: Vec<ImportedEntry>,
    pub merged: Vec<MergedEntry>,
    pub conflicts: Vec<ConflictEntry>,
    pub skipped: Vec<SkippedEntry>,
    pub errors: Vec<ImportError>,
}

// ---------------------------------------------------------------------------
// Importer
// ---------------------------------------------------------------------------

/// Import MCP server definitions from native provider configs into the catalog.
pub struct McpImporter<'a> {
    catalog: &'a mut McpCatalogStore,
    state: &'a mut McpProviderStateStore,
}

impl<'a> McpImporter<'a> {
    pub fn new(catalog: &'a mut McpCatalogStore, state: &'a mut McpProviderStateStore) -> Self {
        Self { catalog, state }
    }

    /// Import from all detected provider configs.
    pub fn import_all(&mut self, repo_root: Option<&Path>) -> ImportReport {
        let mut report = ImportReport::default();

        for info in crate::provider::all_providers() {
            if !info.mcp.supported() {
                continue;
            }
            let provider_report = self.import_provider(info.provider, repo_root);
            report.imported.extend(provider_report.imported);
            report.merged.extend(provider_report.merged);
            report.conflicts.extend(provider_report.conflicts);
            report.skipped.extend(provider_report.skipped);
            report.errors.extend(provider_report.errors);
        }

        report
    }

    /// Import from a specific provider's config.
    pub fn import_provider(
        &mut self,
        provider: Provider,
        repo_root: Option<&Path>,
    ) -> ImportReport {
        let mut report = ImportReport::default();
        let info = provider_info(provider);

        let configs = info.mcp.discover_configs(repo_root);

        for (config_path, scope) in configs {
            let parsed = match info.mcp.parse_config(&config_path) {
                Ok(servers) => servers,
                Err(e) => {
                    report.errors.push(ImportError {
                        provider,
                        native_name: config_path.to_string_lossy().into(),
                        reason: e.to_string(),
                    });
                    continue;
                }
            };

            for (native_name, server) in parsed {
                self.process_import(
                    provider,
                    &scope,
                    &config_path,
                    &native_name,
                    server,
                    &mut report,
                );
            }
        }

        report
    }

    fn process_import(
        &mut self,
        provider: Provider,
        scope: &Scope,
        config_path: &Path,
        native_name: &str,
        mut server: McpServer,
        report: &mut ImportReport,
    ) {
        let fingerprint = server.fingerprint();
        server.metadata.fingerprint = fingerprint.clone();

        // Check if fingerprint already exists in catalog
        if let Some(existing) = self.catalog.find_by_fingerprint(&fingerprint) {
            let existing_id = existing.id.clone();

            // Merge: same fingerprint, potentially different name
            let alias_added = if native_name != existing_id
                && !existing.aliases.iter().any(|a| a == native_name)
            {
                let slug = slugify(native_name);
                if slug != existing_id
                    && !existing.aliases.iter().any(|a| a == &slug)
                    && self.catalog.add_alias(&existing_id, &slug).is_ok()
                {
                    Some(slug)
                } else {
                    None
                }
            } else {
                None
            };

            // Record provenance
            self.state.record_import(
                provider,
                scope,
                ProviderStateEntry {
                    catalog_id: existing_id.clone(),
                    native_name: native_name.into(),
                    source: config_path.to_string_lossy().into(),
                    origin: McpOrigin::Imported,
                    last_seen: Utc::now(),
                },
            );

            report.merged.push(MergedEntry {
                catalog_id: existing_id,
                provider,
                native_name: native_name.into(),
                alias_added,
            });
            return;
        }

        // Generate slug ID
        let mut slug = slugify(native_name);

        // Check if slug conflicts with existing ID that has different fingerprint
        if self.catalog.get_server(&slug).is_some() {
            // Disambiguate with provider suffix
            let disambiguated = format!("{}-{}", slug, provider.as_slug());
            report.conflicts.push(ConflictEntry {
                name: native_name.into(),
                provider,
                new_catalog_id: disambiguated.clone(),
            });
            slug = disambiguated;
        }

        // Create new catalog entry
        server.id = slug.clone();
        server.metadata.created_from = Some(format!(
            "{}:{}",
            provider.as_slug(),
            match scope {
                Scope::User => "user",
                Scope::Repo(_) => "repo",
            }
        ));

        self.catalog.add_server(server);

        // Record provenance
        self.state.record_import(
            provider,
            scope,
            ProviderStateEntry {
                catalog_id: slug.clone(),
                native_name: native_name.into(),
                source: config_path.to_string_lossy().into(),
                origin: McpOrigin::Imported,
                last_seen: Utc::now(),
            },
        );

        report.imported.push(ImportedEntry {
            catalog_id: slug,
            provider,
            native_name: native_name.into(),
        });
    }
}

// ---------------------------------------------------------------------------
// Provider config discovery (per-provider helpers used by McpBehavior)
// ---------------------------------------------------------------------------

pub(crate) fn discover_claude_configs(repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut configs = Vec::new();

    let user_config = home.join(".claude.json");
    if user_config.exists() {
        configs.push((user_config, Scope::User));
    }
    for plugin_config in discover_claude_plugin_configs(&home.join(".claude").join("plugins")) {
        configs.push((plugin_config, Scope::User));
    }
    if let Some(root) = repo_root {
        let repo_config = root.join(".mcp.json");
        if repo_config.exists() {
            configs.push((repo_config, Scope::Repo(root.to_path_buf())));
        }
        let local_config = root.join(".claude").join("settings.local.json");
        if local_config.exists() {
            configs.push((local_config, Scope::Repo(root.to_path_buf())));
        }
        for plugin_config in discover_claude_plugin_configs(&root.join(".claude").join("plugins")) {
            configs.push((plugin_config, Scope::Repo(root.to_path_buf())));
        }
    }

    configs
}

pub(crate) fn discover_codex_configs(repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut configs = Vec::new();
    let user_config = home.join(".codex").join("config.toml");
    if user_config.exists() {
        configs.push((user_config, Scope::User));
    }
    if let Some(root) = repo_root {
        let repo_config = root.join(".codex").join("config.toml");
        if repo_config.exists() {
            configs.push((repo_config, Scope::Repo(root.to_path_buf())));
        }
    }
    configs
}

pub(crate) fn discover_gemini_configs(repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut configs = Vec::new();
    let user_config = home.join(".gemini").join("settings.json");
    if user_config.exists() {
        configs.push((user_config, Scope::User));
    }
    if let Some(root) = repo_root {
        let repo_config = root.join(".gemini").join("settings.json");
        if repo_config.exists() {
            configs.push((repo_config, Scope::Repo(root.to_path_buf())));
        }
    }
    configs
}

pub(crate) fn discover_opencode_configs(repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut configs = Vec::new();
    let user_config = home.join(".config").join("opencode").join("opencode.json");
    if user_config.exists() {
        configs.push((user_config, Scope::User));
    }
    if let Some(root) = repo_root {
        let repo_config = root.join("opencode.json");
        if repo_config.exists() {
            configs.push((repo_config, Scope::Repo(root.to_path_buf())));
        }
    }
    configs
}

fn discover_claude_plugin_configs(plugin_root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(plugin_root) else {
        return Vec::new();
    };

    let mut configs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        for file_name in [".mcp.json", "plugin.json"] {
            let candidate = path.join(file_name);
            if candidate.exists() {
                configs.push(candidate);
            }
        }
    }

    configs
}

// ---------------------------------------------------------------------------
// Per-provider parsers
// ---------------------------------------------------------------------------

/// Parse Claude's MCP config (`~/.claude.json` or `.mcp.json`).
///
/// Both use `mcpServers` as a top-level object with server-name keys.
pub(crate) fn parse_claude_mcp(config_path: &Path) -> Result<Vec<(String, McpServer)>> {
    let content = fs::read_to_string(config_path)?;
    let doc: Value = serde_json::from_str(&content)?;

    let Some(servers) = doc.get("mcpServers").and_then(|v| v.as_object()) else {
        return Ok(Vec::new());
    };

    let now = Utc::now();
    let mut result = Vec::new();

    for (name, def) in servers {
        let transport = match def.get("type").and_then(|v| v.as_str()) {
            Some("sse") => McpTransport::Sse,
            Some("http") => McpTransport::Http,
            _ => McpTransport::Stdio,
        };

        let command = def
            .get("command")
            .and_then(|v| v.as_str())
            .map(String::from);
        let args = parse_string_array(def.get("args"));
        let env = parse_string_map(def.get("env"));
        let cwd = def.get("cwd").and_then(|v| v.as_str()).map(PathBuf::from);
        let url = def.get("url").and_then(|v| v.as_str()).map(String::from);

        let mut server = McpServer {
            id: slugify(name),
            aliases: Vec::new(),
            transport,
            command,
            args,
            cwd,
            env,
            url,
            headers: HashMap::new(),
            enabled_tools: Vec::new(),
            disabled_tools: Vec::new(),
            required: false,
            metadata: McpServerMetadata {
                description: None,
                created_from: None,
                fingerprint: String::new(),
                created_at: now,
                updated_at: now,
            },
            provider_overrides: HashMap::new(),
        };

        if config_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "settings.local.json")
        {
            server.set_provider_override("claude", "scope", json!("local"));
        } else if config_path
            .components()
            .any(|component| component.as_os_str() == "plugins")
        {
            server.set_provider_override("claude", "scope", json!("plugin"));
        }

        result.push((name.clone(), server));
    }

    Ok(result)
}

/// Parse Codex's MCP config (`~/.codex/config.toml`).
///
/// Format: `[mcp_servers.<name>]` tables with `command`, `args`, `env` etc.
pub(crate) fn parse_codex_mcp(config_path: &Path) -> Result<Vec<(String, McpServer)>> {
    let content = fs::read_to_string(config_path)?;
    let doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e: toml_edit::TomlError| crate::error::ClaudineError::TomlParse(e))?;

    let Some(mcp_servers) = doc.get("mcp_servers").and_then(|v| v.as_table()) else {
        return Ok(Vec::new());
    };

    let now = Utc::now();
    let mut result = Vec::new();

    for (name, def) in mcp_servers {
        let Some(table) = def.as_table() else {
            continue;
        };

        let command = table
            .get("command")
            .and_then(|v| v.as_str())
            .map(String::from);
        let url = table.get("url").and_then(|v| v.as_str()).map(String::from);

        let transport = if url.is_some() {
            McpTransport::Http
        } else {
            McpTransport::Stdio
        };

        let args = table
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let env = table
            .get("env")
            .and_then(|v| v.as_table())
            .map(|t| {
                t.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.to_string(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let cwd = table.get("cwd").and_then(|v| v.as_str()).map(PathBuf::from);

        let mut headers = HashMap::new();
        if let Some(h) = table.get("http_headers").and_then(|v| v.as_table()) {
            for (k, v) in h {
                if let Some(s) = v.as_str() {
                    headers.insert(k.to_string(), s.to_string());
                }
            }
        }

        let mut server = McpServer {
            id: slugify(name),
            aliases: Vec::new(),
            transport,
            command,
            args,
            cwd,
            env,
            url,
            headers,
            enabled_tools: Vec::new(),
            disabled_tools: Vec::new(),
            required: false,
            metadata: McpServerMetadata {
                description: None,
                created_from: None,
                fingerprint: String::new(),
                created_at: now,
                updated_at: now,
            },
            provider_overrides: HashMap::new(),
        };

        if let Some(env_vars) = table.get("env_vars").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        }) && !env_vars.is_empty()
        {
            server.set_provider_override("codex", "env_vars", json!(env_vars));
        }
        if let Some(value) = table.get("bearer_token_env_var").and_then(|v| v.as_str()) {
            server.set_provider_override("codex", "bearer_token_env_var", json!(value));
        }
        if let Some(value) = table.get("env_http_headers").and_then(|v| v.as_table()) {
            let headers = value
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|v| (k.to_string(), v.to_string())))
                .collect::<HashMap<_, _>>();
            if !headers.is_empty() {
                server.set_provider_override("codex", "env_http_headers", json!(headers));
            }
        }
        if let Some(value) = table.get("enabled").and_then(|v| v.as_bool()) {
            server.set_provider_override("codex", "enabled", json!(value));
        }
        if let Some(value) = table
            .get("startup_timeout_sec")
            .and_then(|v| v.as_integer())
        {
            server.set_provider_override("codex", "startup_timeout_sec", json!(value));
        }
        if let Some(value) = table.get("tool_timeout_sec").and_then(|v| v.as_integer()) {
            server.set_provider_override("codex", "tool_timeout_sec", json!(value));
        }

        server.required = table
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        server.enabled_tools = table
            .get("enabled_tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        server.disabled_tools = table
            .get("disabled_tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        result.push((name.to_string(), server));
    }

    Ok(result)
}

/// Parse Gemini's MCP config (`~/.gemini/settings.json`).
pub(crate) fn parse_gemini_mcp(config_path: &Path) -> Result<Vec<(String, McpServer)>> {
    let content = fs::read_to_string(config_path)?;
    let doc: Value = serde_json::from_str(&content)?;

    let Some(servers) = doc.get("mcpServers").and_then(|v| v.as_object()) else {
        return Ok(Vec::new());
    };

    let now = Utc::now();
    let mut result = Vec::new();

    for (name, def) in servers {
        let transport = match def.get("transport").and_then(|v| v.as_str()) {
            Some("sse") => McpTransport::Sse,
            Some("http") => McpTransport::Http,
            _ => McpTransport::Stdio,
        };

        let command = def
            .get("command")
            .and_then(|v| v.as_str())
            .map(String::from);
        let args = parse_string_array(def.get("args"));
        let env = parse_string_map(def.get("env"));
        let url = def.get("url").and_then(|v| v.as_str()).map(String::from);

        let enabled_tools = parse_string_array(def.get("include-tools"));
        let disabled_tools = parse_string_array(def.get("exclude-tools"));

        let mut server = McpServer {
            id: slugify(name),
            aliases: Vec::new(),
            transport,
            command,
            args,
            cwd: None,
            env,
            url,
            headers: HashMap::new(),
            enabled_tools,
            disabled_tools,
            required: false,
            metadata: McpServerMetadata {
                description: None,
                created_from: None,
                fingerprint: String::new(),
                created_at: now,
                updated_at: now,
            },
            provider_overrides: HashMap::new(),
        };

        for field in ["description", "timeout", "trust"] {
            if let Some(value) = def.get(field).cloned() {
                server.set_provider_override("gemini", field, value);
            }
        }

        result.push((name.clone(), server));
    }

    Ok(result)
}

/// Parse OpenCode's MCP config.
pub(crate) fn parse_opencode_mcp(config_path: &Path) -> Result<Vec<(String, McpServer)>> {
    let content = fs::read_to_string(config_path)?;
    let doc: Value = serde_json::from_str(&content)?;

    let Some(mcp) = doc.get("mcp").and_then(|v| v.as_object()) else {
        return Ok(Vec::new());
    };

    let now = Utc::now();
    let mut result = Vec::new();

    for (name, def) in mcp {
        let server_type = def.get("type").and_then(|v| v.as_str()).unwrap_or("local");

        let transport = if server_type == "remote" {
            McpTransport::Http
        } else {
            McpTransport::Stdio
        };

        let (command, args) = parse_command_and_args(def.get("command"));
        let url = def.get("url").and_then(|v| v.as_str()).map(String::from);
        let env = parse_string_map(def.get("environment"));
        let headers = parse_string_map(def.get("headers"));

        let mut server = McpServer {
            id: slugify(name),
            aliases: Vec::new(),
            transport,
            command,
            args,
            cwd: None,
            env,
            url,
            headers,
            enabled_tools: Vec::new(),
            disabled_tools: Vec::new(),
            required: false,
            metadata: McpServerMetadata {
                description: None,
                created_from: None,
                fingerprint: String::new(),
                created_at: now,
                updated_at: now,
            },
            provider_overrides: HashMap::new(),
        };

        for field in ["enabled", "oauth", "timeout"] {
            if let Some(value) = def.get(field).cloned() {
                server.set_provider_override("opencode", field, value);
            }
        }

        result.push((name.clone(), server));
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

fn parse_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_string_map(value: Option<&Value>) -> HashMap<String, String> {
    value
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_command_and_args(value: Option<&Value>) -> (Option<String>, Vec<String>) {
    match value {
        Some(Value::String(command)) => (Some(command.clone()), Vec::new()),
        Some(Value::Array(parts)) => {
            let mut parts = parts
                .iter()
                .filter_map(|part| part.as_str().map(str::to_string));
            let command = parts.next();
            let args = parts.collect();
            (command, args)
        }
        _ => (None, Vec::new()),
    }
}

#[cfg(test)]
mod tests;
