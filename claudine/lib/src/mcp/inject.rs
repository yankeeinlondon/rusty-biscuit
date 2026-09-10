use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::types::{McpServer, McpTransport};
use crate::config::atomic::atomic_write;
use crate::error::{ClaudineError, Result};
use crate::provider::Provider;

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
                if let Some(value) = provider_override_value(server, "opencode", field) {
                    entry.insert(field.to_string(), value.clone());
                }
            }
            mcp_config.insert(server.id.clone(), serde_json::Value::Object(entry));
        }

        // Merge `{"mcp": …}` into any existing inline config rather than
        // overwriting it: the system-prompt and YOLO producers target the same
        // `OPENCODE_CONFIG_CONTENT` value, so a blind overwrite would clobber
        // their `instructions`/`permission` keys.
        let overlay = json!({ "mcp": mcp_config });
        // This injector's env map is `String`-valued, so any existing value is
        // already valid UTF-8; wrap it as an `OsStr` for the shared helper, which
        // owns the UTF-8 validity decision for the raw-`OsStr` env call sites.
        let existing = env
            .get("OPENCODE_CONFIG_CONTENT")
            .map(std::ffi::OsStr::new);
        let config_str = crate::opencode_config::merge_overlay(existing, overlay)?;
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
        let home = shadow_home.ok_or_else(|| {
            ClaudineError::ConfigValidation(
                "Codex runtime MCP injection requires a shadow HOME".into(),
            )
        })?;
        let config_dir = home.join(".codex");
        let config_path = config_dir.join("config.toml");

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
        let home = shadow_home.ok_or_else(|| {
            ClaudineError::ConfigValidation(
                "Gemini runtime MCP injection requires a shadow HOME".into(),
            )
        })?;
        let config_dir = home.join(".gemini");
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
/// - Qwen, Goose, Kimi (blocked on research)
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
    fn opencode_merge_preserves_prior_config_keys() {
        // The injector is handed a map that already carries an
        // OPENCODE_CONFIG_CONTENT object; the merge must keep its keys.
        let injector = OpenCodeInjector;
        let servers = vec![make_server("calendar")];
        let mut env = HashMap::new();
        env.insert(
            "OPENCODE_CONFIG_CONTENT".to_string(),
            json!({ "theme": "dark" }).to_string(),
        );

        injector.inject(&servers, &mut env, None).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(env.get("OPENCODE_CONFIG_CONTENT").unwrap()).unwrap();
        assert_eq!(content["theme"], "dark");
        assert!(content["mcp"]["calendar"].is_object());
    }

    #[test]
    fn opencode_merge_coexists_with_instructions() {
        // Regression for the MCP ↔ system-prompt clobber: a pre-existing
        // `instructions` key and the injected `mcp` key must both survive.
        let injector = OpenCodeInjector;
        let servers = vec![make_server("calendar")];
        let mut env = HashMap::new();
        env.insert(
            "OPENCODE_CONFIG_CONTENT".to_string(),
            json!({ "instructions": ["do the thing"] }).to_string(),
        );

        injector.inject(&servers, &mut env, None).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(env.get("OPENCODE_CONFIG_CONTENT").unwrap()).unwrap();
        assert_eq!(content["instructions"], json!(["do the thing"]));
        assert!(content["mcp"]["calendar"].is_object());
    }

    #[test]
    fn opencode_merge_preserves_user_supplied_config() {
        // A user-supplied non-empty config object is preserved, not replaced,
        // when MCP is injected.
        let injector = OpenCodeInjector;
        let servers = vec![make_server("calendar")];
        let mut env = HashMap::new();
        env.insert(
            "OPENCODE_CONFIG_CONTENT".to_string(),
            json!({ "provider": { "anthropic": { "model": "claude" } } }).to_string(),
        );

        injector.inject(&servers, &mut env, None).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(env.get("OPENCODE_CONFIG_CONTENT").unwrap()).unwrap();
        assert_eq!(content["provider"]["anthropic"]["model"], "claude");
        assert!(content["mcp"]["calendar"].is_object());
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
    fn codex_preserves_existing_non_mcp_settings() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".codex");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("config.toml"), "model = \"gpt-5\"\n").unwrap();

        let injector = CodexInjector;
        let servers = vec![make_server("slack")];
        let mut env = HashMap::new();
        injector
            .inject(&servers, &mut env, Some(tmp.path()))
            .unwrap();

        let content = fs::read_to_string(config_dir.join("config.toml")).unwrap();
        let doc: toml_edit::DocumentMut = content.parse().unwrap();
        assert_eq!(doc["model"].as_str(), Some("gpt-5"));
        assert!(doc["mcp_servers"]["slack"].is_table());
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
        assert!(
            result
                .extra_args
                .contains(&"--allowed-mcp-server-names".to_string())
        );
    }

    #[test]
    fn gemini_preserves_existing_settings_and_sidecars() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".gemini");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("settings.json"),
            r#"{"theme":"dark","mcpServers":{"old":{"command":"noop"}}}"#,
        )
        .unwrap();
        fs::write(
            config_dir.join("mcp-server-enablement.json"),
            r#"{"old":false}"#,
        )
        .unwrap();

        let injector = GeminiInjector;
        let servers = vec![make_server("linear")];
        let mut env = HashMap::new();
        injector
            .inject(&servers, &mut env, Some(tmp.path()))
            .unwrap();

        let settings: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(settings["theme"], "dark");
        assert!(settings["mcpServers"]["linear"].is_object());

        let enablement: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(config_dir.join("mcp-server-enablement.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(enablement["old"], false);
    }

    #[test]
    fn file_backed_injectors_require_shadow_home() {
        let mut env = HashMap::new();
        let servers = vec![make_server("linear")];

        assert!(CodexInjector.inject(&servers, &mut env, None).is_err());
        assert!(GeminiInjector.inject(&servers, &mut env, None).is_err());
    }

    #[test]
    fn unsupported_providers_return_none() {
        assert!(injector_for_provider(Provider::Claude).is_none());
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
