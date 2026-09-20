use std::collections::HashMap;

use chrono::Utc;
use tempfile::TempDir;

use super::*;
use crate::diagnostics::Diagnostic;
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

/// Pins the researched Kilo shape (`docs/research/mcp/kilo.md` →
/// `server_shape`) and that per-server overrides come from the `kilo` slug
/// only.
#[test]
fn kilo_injects_the_researched_inline_mcp_shape() {
    let mut local = make_server("calendar");
    local.env.insert("API_KEY".into(), "secret".into());
    local.set_provider_override("kilo", "timeout", json!(30000));
    local.set_provider_override("opencode", "enabled", json!(false));
    let mut remote = make_server("linear");
    remote.transport = McpTransport::Http;
    remote.command = None;
    remote.args.clear();
    remote.url = Some("https://mcp.linear.app/mcp".into());
    remote.headers.insert("Authorization".into(), "Bearer token".into());
    let mut env = HashMap::from([(
        "KILO_CONFIG_CONTENT".to_string(),
        json!({ "theme": "dark" }).to_string(),
    )]);

    let result = KiloInjector.inject(&[local, remote], &mut env, None).unwrap();

    assert!(!env.contains_key("OPENCODE_CONFIG_CONTENT"));
    let content: serde_json::Value =
        serde_json::from_str(env.get("KILO_CONFIG_CONTENT").unwrap()).unwrap();
    assert_eq!(
        content,
        json!({
            "theme": "dark",
            "mcp": {
                "calendar": {
                    "type": "local",
                    "command": ["npx", "-y", "@test/calendar"],
                    "environment": { "API_KEY": "secret" },
                    "timeout": 30000
                },
                "linear": {
                    "type": "remote",
                    "url": "https://mcp.linear.app/mcp",
                    "headers": { "Authorization": "Bearer token" }
                }
            }
        })
    );
    assert_eq!(result.provider, Provider::Kilo);
    assert_eq!(result.env_vars_set, vec!["KILO_CONFIG_CONTENT"]);
    assert!(result.temp_files.is_empty() && result.extra_args.is_empty());
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
fn codex_writes_config_toml_directly_in_the_config_root() {
    let tmp = TempDir::new().unwrap();
    let config_root = tmp.path().join(".claudine").join(".codex");
    let injector = CodexInjector;
    let servers = vec![make_server("slack")];
    let mut env = HashMap::new();

    let result = injector
        .inject(&servers, &mut env, Some(&config_root))
        .unwrap();

    // `$CODEX_HOME/config.toml`, not a `.codex` joined on again (audit F1).
    let config_path = config_root.join("config.toml");
    assert!(config_path.exists());
    assert!(!config_root.join(".codex").exists());

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
        .inject(&servers, &mut env, Some(&config_dir))
        .unwrap();

    let content = fs::read_to_string(config_dir.join("config.toml")).unwrap();
    let doc: toml_edit::DocumentMut = content.parse().unwrap();
    assert_eq!(doc["model"].as_str(), Some("gpt-5"));
    assert!(doc["mcp_servers"]["slack"].is_table());
}

#[test]
fn gemini_writes_settings_json_directly_in_the_config_root() {
    let tmp = TempDir::new().unwrap();
    // `GEMINI_CLI_HOME` names the parent; the injector receives `.gemini`.
    let config_root = tmp.path().join(".claudine").join(".gemini");
    let injector = GeminiInjector;
    let servers = vec![make_server("linear")];
    let mut env = HashMap::new();

    let result = injector
        .inject(&servers, &mut env, Some(&config_root))
        .unwrap();

    let config_path = config_root.join("settings.json");
    assert!(config_path.exists());
    assert!(!config_root.join(".gemini").exists());

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
        .inject(&servers, &mut env, Some(&config_dir))
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
fn file_backed_injectors_refuse_a_missing_config_root_with_a_typed_overlay_failure() {
    let injectors = [(&CodexInjector as &dyn McpInjector, Provider::Codex), (&GeminiInjector, Provider::Gemini)];
    for (injector, expected) in injectors {
        let mut env = HashMap::new();
        let error = injector.inject(&[make_server("linear")], &mut env, None).expect_err("needs a config root");

        assert_eq!(error.code(), "provider.overlay_failed", "{expected}");
        let detail = error.detail();
        assert_eq!(detail["provider"], json!(expected.to_string()));
        assert_eq!(detail["reason"], json!("mcp"));
        assert_eq!(detail["stage"], json!("mcp_injection"));
        let message = error.to_string();
        assert!(!message.contains("shadow HOME"), "{message}");
        assert!(!message.to_lowercase().contains("credential"), "{message}");
        assert!(env.is_empty(), "{expected} wrote env before refusing");
    }
}

#[test]
fn gemini_sidecars_become_private_copies_in_the_config_root() {
    let tmp = TempDir::new().unwrap();
    let (user_root, config_root) = (tmp.path().join("user"), tmp.path().join("overlay"));
    let sidecars = [("mcp-server-enablement.json", r#"{"old":false}"#), ("mcp-oauth-tokens.json", "[]")];
    fs::create_dir_all(&user_root).unwrap();
    fs::create_dir_all(&config_root).unwrap();
    for (name, content) in sidecars {
        fs::write(user_root.join(name), content).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(user_root.join(name), config_root.join(name)).unwrap();
        #[cfg(not(unix))]
        fs::copy(user_root.join(name), config_root.join(name)).unwrap();
    }

    GeminiInjector
        .inject(&[make_server("linear")], &mut HashMap::new(), Some(&config_root))
        .unwrap();

    for (name, content) in sidecars {
        let copy = config_root.join(name);
        assert!(fs::symlink_metadata(&copy).unwrap().is_file(), "{name} is still a link");
        let copied: serde_json::Value = serde_json::from_str(&fs::read_to_string(&copy).unwrap()).unwrap();
        assert_eq!(copied, serde_json::from_str::<serde_json::Value>(content).unwrap());
        assert_eq!(fs::read_to_string(user_root.join(name)).unwrap(), content);
    }
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
    assert!(injector_for_provider(Provider::Kilo).is_some());
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
