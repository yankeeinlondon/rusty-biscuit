use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::types::{McpServer, McpTransport};
use crate::config::atomic::atomic_write;
use crate::error::{ClaudineError, Result};
use crate::provider::Provider;
use crate::provider_overlay::{OverlayReason, OverlayStage};

// ---------------------------------------------------------------------------
// Injection result
// ---------------------------------------------------------------------------

/// Result of injecting MCP servers into a provider's runtime environment.
#[derive(Debug)]
pub struct InjectionResult {
    pub provider: Provider,
    pub servers_injected: Vec<String>,
    pub temp_files: Vec<PathBuf>,
    pub env_vars_set: Vec<String>,
    pub extra_args: Vec<String>,
}

// ---------------------------------------------------------------------------
// Injector trait
// ---------------------------------------------------------------------------

/// Per-provider strategy for injecting MCP servers at runtime.
pub trait McpInjector {
    fn provider(&self) -> Provider;
    fn supports_runtime(&self) -> bool;

    /// Inject `servers` for one launch.
    ///
    /// `config_root` is the directory that directly contains the provider's
    /// config file — the overlay plan's provider-visible root, never a home
    /// directory the injector joins the agent offset onto. A file-backed
    /// injector refuses `None` with `ProviderOverlayFailed`; an inline one
    /// ignores it.
    fn inject(
        &self,
        servers: &[McpServer],
        env: &mut HashMap<String, String>,
        config_root: Option<&Path>,
    ) -> Result<InjectionResult>;

    fn cleanup(&self, result: &InjectionResult) -> Result<()>;
}

// ---------------------------------------------------------------------------
// OpenCode and Kilo injectors — inline JSON config env var only
// ---------------------------------------------------------------------------

pub struct OpenCodeInjector;

impl McpInjector for OpenCodeInjector {
    fn provider(&self) -> Provider {
        Provider::OpenCode
    }

    fn supports_runtime(&self) -> bool {
        true
    }

    fn inject(
        &self,
        servers: &[McpServer],
        env: &mut HashMap<String, String>,
        _config_root: Option<&Path>,
    ) -> Result<InjectionResult> {
        inject_inline_config(
            Provider::OpenCode,
            crate::opencode_config::OPENCODE_CONFIG_CONTENT,
            servers,
            env,
        )
    }

    fn cleanup(&self, _result: &InjectionResult) -> Result<()> {
        Ok(()) // No temp files
    }
}

/// Kilo is an OpenCode fork whose `KILO_CONFIG_CONTENT` accepts the same
/// `mcp` map (`docs/research/mcp/kilo.md` → `server_shape`,
/// `runtime_injection`).
pub struct KiloInjector;

impl McpInjector for KiloInjector {
    fn provider(&self) -> Provider {
        Provider::Kilo
    }

    fn supports_runtime(&self) -> bool {
        true
    }

    fn inject(
        &self,
        servers: &[McpServer],
        env: &mut HashMap<String, String>,
        _config_root: Option<&Path>,
    ) -> Result<InjectionResult> {
        inject_inline_config(
            Provider::Kilo,
            crate::opencode_config::KILO_CONFIG_CONTENT,
            servers,
            env,
        )
    }

    fn cleanup(&self, _result: &InjectionResult) -> Result<()> {
        Ok(()) // No temp files
    }
}

/// Merge the OpenCode-shaped `{"mcp": …}` map into the inline config `key`.
///
/// Per-server overrides are read from the `provider`'s own slug, so an
/// `opencode` override never leaks into a Kilo launch.
fn inject_inline_config(
    provider: Provider,
    key: &str,
    servers: &[McpServer],
    env: &mut HashMap<String, String>,
) -> Result<InjectionResult> {
    let override_slug = provider.as_slug();
    let mut mcp_config = serde_json::Map::new();

    for server in servers {
        let mut entry = serde_json::Map::new();
        match server.transport {
            McpTransport::Stdio => {
                entry.insert("type".into(), json!("local"));
                if let Some(ref cmd) = server.command {
                    let command = std::iter::once(cmd.clone())
                        .chain(server.args.iter().cloned())
                        .collect::<Vec<_>>();
                    entry.insert("command".into(), json!(command));
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
        for field in ["enabled", "oauth", "timeout"] {
            if let Some(value) = provider_override_value(server, override_slug, field) {
                entry.insert(field.to_string(), value.clone());
            }
        }
        mcp_config.insert(server.id.clone(), serde_json::Value::Object(entry));
    }

    // Merge `{"mcp": …}` into any existing inline config rather than
    // overwriting it: for OpenCode the system-prompt and YOLO producers target
    // the same value, and for either provider the user may have exported one.
    let overlay = json!({ "mcp": mcp_config });
    // This injector's env map is `String`-valued, so any existing value is
    // already valid UTF-8; wrap it as an `OsStr` for the shared helper, which
    // owns the UTF-8 validity decision for the raw-`OsStr` env call sites.
    let existing = env.get(key).map(std::ffi::OsStr::new);
    let config_str = crate::opencode_config::merge_named_overlay(key, existing, overlay)?;
    env.insert(key.into(), config_str);

    Ok(InjectionResult {
        provider,
        servers_injected: servers.iter().map(|s| s.id.clone()).collect(),
        temp_files: Vec::new(),
        env_vars_set: vec![key.into()],
        extra_args: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// Codex injector — overlay `config.toml`
// ---------------------------------------------------------------------------

pub struct CodexInjector;

impl McpInjector for CodexInjector {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn supports_runtime(&self) -> bool {
        true
    }

    fn inject(
        &self,
        servers: &[McpServer],
        _env: &mut HashMap<String, String>,
        config_root: Option<&Path>,
    ) -> Result<InjectionResult> {
        let config_path = require_config_root(Provider::Codex, config_root)?.join("config.toml");

        let mut doc = if config_path.exists() {
            fs::read_to_string(&config_path)?
                .parse()
                .map_err(ClaudineError::TomlParse)?
        } else {
            toml_edit::DocumentMut::new()
        };

        doc["mcp_servers"] = toml_edit::Item::Table(toml_edit::Table::new());

        let mcp_table = doc["mcp_servers"].as_table_mut().unwrap();

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
                let mut sorted: Vec<_> = server.env.iter().collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                for (k, v) in sorted {
                    env_table[k.as_str()] = toml_edit::value(v.as_str());
                }
                table["env"] = toml_edit::Item::Table(env_table);
            }
            if let Some(ref url) = server.url {
                table["url"] = toml_edit::value(url.as_str());
            }
            if let Some(ref cwd) = server.cwd {
                table["cwd"] = toml_edit::value(cwd.to_string_lossy().as_ref());
            }
            if !server.headers.is_empty() {
                let mut header_table = toml_edit::Table::new();
                let mut sorted: Vec<_> = server.headers.iter().collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                for (k, v) in sorted {
                    header_table[k.as_str()] = toml_edit::value(v.as_str());
                }
                table["http_headers"] = toml_edit::Item::Table(header_table);
            }
            if !server.enabled_tools.is_empty() {
                table["enabled_tools"] = toml_edit::value(to_toml_array(&server.enabled_tools));
            }
            if !server.disabled_tools.is_empty() {
                table["disabled_tools"] = toml_edit::value(to_toml_array(&server.disabled_tools));
            }
            if server.required {
                table["required"] = toml_edit::value(true);
            }
            if let Some(enabled) = provider_override_bool(server, "codex", "enabled") {
                table["enabled"] = toml_edit::value(enabled);
            }
            if let Some(env_vars) = provider_override_string_array(server, "codex", "env_vars")
                && !env_vars.is_empty()
            {
                table["env_vars"] = toml_edit::value(to_toml_array(&env_vars));
            }
            if let Some(value) = provider_override_string(server, "codex", "bearer_token_env_var") {
                table["bearer_token_env_var"] = toml_edit::value(value.as_str());
            }
            if let Some(headers) = provider_override_string_map(server, "codex", "env_http_headers")
                && !headers.is_empty()
            {
                let mut header_table = toml_edit::Table::new();
                let mut sorted: Vec<_> = headers.iter().collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                for (k, v) in sorted {
                    header_table[k.as_str()] = toml_edit::value(v.as_str());
                }
                table["env_http_headers"] = toml_edit::Item::Table(header_table);
            }
            if let Some(value) = provider_override_i64(server, "codex", "startup_timeout_sec") {
                table["startup_timeout_sec"] = toml_edit::value(value);
            }
            if let Some(value) = provider_override_i64(server, "codex", "tool_timeout_sec") {
                table["tool_timeout_sec"] = toml_edit::value(value);
            }

            mcp_table[&server.id] = toml_edit::Item::Table(table);
        }

        atomic_write(&config_path, doc.to_string().as_bytes())?;
        crate::provider_overlay::record_claudine_write(&config_path);

        Ok(InjectionResult {
            provider: Provider::Codex,
            servers_injected: servers.iter().map(|s| s.id.clone()).collect(),
            temp_files: vec![config_path],
            env_vars_set: Vec::new(),
            extra_args: Vec::new(),
        })
    }

    fn cleanup(&self, result: &InjectionResult) -> Result<()> {
        for path in &result.temp_files {
            let _ = fs::remove_file(path);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Gemini injector — overlay `settings.json`
// ---------------------------------------------------------------------------

pub struct GeminiInjector;

impl McpInjector for GeminiInjector {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }

    fn supports_runtime(&self) -> bool {
        true
    }

    fn inject(
        &self,
        servers: &[McpServer],
        _env: &mut HashMap<String, String>,
        config_root: Option<&Path>,
    ) -> Result<InjectionResult> {
        let config_dir = require_config_root(Provider::Gemini, config_root)?;
        let config_path = config_dir.join("settings.json");

        materialize_json_copy_if_exists(&config_dir.join("mcp-server-enablement.json"))?;
        materialize_json_copy_if_exists(&config_dir.join("mcp-oauth-tokens.json"))?;

        let mut doc = if config_path.exists() {
            serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&config_path)?)?
        } else {
            json!({})
        };
        let mut mcp_servers = serde_json::Map::new();

        for server in servers {
            let mut entry = serde_json::Map::new();
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
            if !server.enabled_tools.is_empty() {
                entry.insert("include-tools".into(), json!(server.enabled_tools));
            }
            if !server.disabled_tools.is_empty() {
                entry.insert("exclude-tools".into(), json!(server.disabled_tools));
            }
            for field in ["description", "timeout", "trust"] {
                if let Some(value) = provider_override_value(server, "gemini", field) {
                    entry.insert(field.to_string(), value.clone());
                }
            }
            mcp_servers.insert(server.id.clone(), serde_json::Value::Object(entry));
        }

        let root = doc.as_object_mut().expect("gemini settings object");
        root.insert("mcpServers".into(), serde_json::Value::Object(mcp_servers));
        let content = serde_json::to_string_pretty(&doc)?;
        atomic_write(&config_path, content.as_bytes())?;
        crate::provider_overlay::record_claudine_write(&config_path);

        // Build extra args for allowed server names
        let mut extra_args = Vec::new();
        if !servers.is_empty() {
            let names: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
            extra_args.push("--allowed-mcp-server-names".into());
            extra_args.push(names.join(","));
        }

        Ok(InjectionResult {
            provider: Provider::Gemini,
            servers_injected: servers.iter().map(|s| s.id.clone()).collect(),
            temp_files: vec![config_path],
            env_vars_set: Vec::new(),
            extra_args,
        })
    }

    fn cleanup(&self, result: &InjectionResult) -> Result<()> {
        for path in &result.temp_files {
            let _ = fs::remove_file(path);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Get the runtime injector for a provider, if one exists.
///
/// Returns `None` for providers that don't support runtime injection:
/// - Claude (import/sync only in v1)
/// - Qwen, Goose, Kimi, Pi, Antigravity
pub fn injector_for_provider(provider: Provider) -> Option<Box<dyn McpInjector>> {
    crate::provider::provider_info(provider)
        .mcp
        .runtime_injector()
}

fn provider_override_value<'a>(
    server: &'a McpServer,
    provider_slug: &str,
    key: &str,
) -> Option<&'a serde_json::Value> {
    server
        .provider_override_object(provider_slug)
        .and_then(|map| map.get(key))
}

fn provider_override_bool(server: &McpServer, provider_slug: &str, key: &str) -> Option<bool> {
    provider_override_value(server, provider_slug, key).and_then(serde_json::Value::as_bool)
}

fn provider_override_i64(server: &McpServer, provider_slug: &str, key: &str) -> Option<i64> {
    provider_override_value(server, provider_slug, key).and_then(serde_json::Value::as_i64)
}

fn provider_override_string(server: &McpServer, provider_slug: &str, key: &str) -> Option<String> {
    provider_override_value(server, provider_slug, key)
        .and_then(serde_json::Value::as_str)
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
) -> Option<HashMap<String, String>> {
    provider_override_value(server, provider_slug, key).map(|value| {
        value
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
            .collect()
    })
}

/// A file-backed injector writes only into a provider overlay; without one it
/// would have to write the user's own config, so it stops the launch instead.
fn require_config_root(provider: Provider, config_root: Option<&Path>) -> Result<&Path> {
    config_root.ok_or_else(|| ClaudineError::ProviderOverlayFailed {
        provider,
        reason: OverlayReason::Mcp,
        stage: OverlayStage::McpInjection,
        source: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no provider overlay config root was planned for runtime MCP injection",
        ),
    })
}

fn materialize_json_copy_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;
    let content = serde_json::to_string_pretty(&value)?;
    Ok(atomic_write(path, content.as_bytes())?)
}

fn to_toml_array(values: &[String]) -> toml_edit::Array {
    let mut arr = toml_edit::Array::new();
    for value in values {
        arr.push(value.as_str());
    }
    arr
}

#[cfg(test)]
mod tests;
