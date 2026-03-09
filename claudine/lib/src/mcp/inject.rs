use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::config::atomic::atomic_write;
use crate::error::Result;
use crate::events::Provider;

use super::types::{McpServer, McpTransport};

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

    fn inject(
        &self,
        servers: &[McpServer],
        env: &mut HashMap<String, String>,
        shadow_home: Option<&Path>,
    ) -> Result<InjectionResult>;

    fn cleanup(&self, result: &InjectionResult) -> Result<()>;
}

// ---------------------------------------------------------------------------
// OpenCode injector — env var only
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
        _shadow_home: Option<&Path>,
    ) -> Result<InjectionResult> {
        let mut mcp_config = serde_json::Map::new();

        for server in servers {
            let mut entry = serde_json::Map::new();
            match server.transport {
                McpTransport::Stdio => {
                    entry.insert("type".into(), json!("local"));
                    if let Some(ref cmd) = server.command {
                        entry.insert("command".into(), json!(cmd));
                    }
                    if !server.args.is_empty() {
                        entry.insert("args".into(), json!(server.args));
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
            mcp_config.insert(server.id.clone(), serde_json::Value::Object(entry));
        }

        let config = json!({ "mcp": mcp_config });
        let config_str = serde_json::to_string(&config)?;
        env.insert("OPENCODE_CONFIG_CONTENT".into(), config_str);

        Ok(InjectionResult {
            provider: Provider::OpenCode,
            servers_injected: servers.iter().map(|s| s.id.clone()).collect(),
            temp_files: Vec::new(),
            env_vars_set: vec!["OPENCODE_CONFIG_CONTENT".into()],
            extra_args: Vec::new(),
        })
    }

    fn cleanup(&self, _result: &InjectionResult) -> Result<()> {
        Ok(()) // No temp files
    }
}

// ---------------------------------------------------------------------------
// Codex injector — shadow home TOML
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
        shadow_home: Option<&Path>,
    ) -> Result<InjectionResult> {
        let home = shadow_home.unwrap_or_else(|| Path::new("."));
        let config_dir = home.join(".codex");
        let config_path = config_dir.join("config.toml");

        let mut doc = toml_edit::DocumentMut::new();
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
            mcp_table[&server.id] = toml_edit::Item::Table(table);
        }

        atomic_write(&config_path, doc.to_string().as_bytes())?;

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
// Gemini injector — shadow home JSON
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
        shadow_home: Option<&Path>,
    ) -> Result<InjectionResult> {
        let home = shadow_home.unwrap_or_else(|| Path::new("."));
        let config_dir = home.join(".gemini");
        let config_path = config_dir.join("settings.json");

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
            mcp_servers.insert(server.id.clone(), serde_json::Value::Object(entry));
        }

        let doc = json!({ "mcpServers": mcp_servers });
        let content = serde_json::to_string_pretty(&doc)?;
        atomic_write(&config_path, content.as_bytes())?;

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
/// - Roo (VS Code extension, not a wrapper target)
/// - Qwen, Goose, Kimi (blocked on research)
pub fn injector_for_provider(provider: Provider) -> Option<Box<dyn McpInjector>> {
    match provider {
        Provider::OpenCode => Some(Box::new(OpenCodeInjector)),
        Provider::Codex => Some(Box::new(CodexInjector)),
        Provider::Gemini => Some(Box::new(GeminiInjector)),
        _ => None,
    }
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
    fn opencode_injects_env_var() {
        let injector = OpenCodeInjector;
        let servers = vec![make_server("calendar")];
        let mut env = HashMap::new();

        let result = injector.inject(&servers, &mut env, None).unwrap();

        assert!(env.contains_key("OPENCODE_CONFIG_CONTENT"));
        let content: serde_json::Value =
            serde_json::from_str(env.get("OPENCODE_CONFIG_CONTENT").unwrap()).unwrap();
        assert!(content["mcp"]["calendar"].is_object());
        assert!(result.temp_files.is_empty());
        assert_eq!(result.servers_injected, vec!["calendar"]);
    }

    #[test]
    fn codex_creates_toml_in_shadow_home() {
        let tmp = TempDir::new().unwrap();
        let injector = CodexInjector;
        let servers = vec![make_server("slack")];
        let mut env = HashMap::new();

        let result = injector
            .inject(&servers, &mut env, Some(tmp.path()))
            .unwrap();

        let config_path = tmp.path().join(".codex").join("config.toml");
        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        let doc: toml_edit::DocumentMut = content.parse().unwrap();
        assert!(doc["mcp_servers"]["slack"].is_table());

        assert_eq!(result.temp_files, vec![config_path]);
    }

    #[test]
    fn gemini_creates_json_in_shadow_home() {
        let tmp = TempDir::new().unwrap();
        let injector = GeminiInjector;
        let servers = vec![make_server("linear")];
        let mut env = HashMap::new();

        let result = injector
            .inject(&servers, &mut env, Some(tmp.path()))
            .unwrap();

        let config_path = tmp.path().join(".gemini").join("settings.json");
        assert!(config_path.exists());

        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(content["mcpServers"]["linear"].is_object());

        // Should include extra args for allowed server names
        assert!(result.extra_args.contains(&"--allowed-mcp-server-names".to_string()));
    }

    #[test]
    fn unsupported_providers_return_none() {
        assert!(injector_for_provider(Provider::Claude).is_none());
        assert!(injector_for_provider(Provider::RooCode).is_none());
        assert!(injector_for_provider(Provider::QwenCode).is_none());
        assert!(injector_for_provider(Provider::Goose).is_none());
        assert!(injector_for_provider(Provider::KimiCode).is_none());
    }

    #[test]
    fn supported_providers_return_some() {
        assert!(injector_for_provider(Provider::OpenCode).is_some());
        assert!(injector_for_provider(Provider::Codex).is_some());
        assert!(injector_for_provider(Provider::Gemini).is_some());
    }

    #[test]
    fn cleanup_removes_temp_files() {
        let tmp = TempDir::new().unwrap();
        let injector = CodexInjector;
        let servers = vec![make_server("test")];
        let mut env = HashMap::new();

        let result = injector
            .inject(&servers, &mut env, Some(tmp.path()))
            .unwrap();
        assert!(result.temp_files[0].exists());

        injector.cleanup(&result).unwrap();
        assert!(!result.temp_files[0].exists());
    }
}
