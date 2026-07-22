use tempfile::TempDir;

use super::*;

#[test]
fn parse_claude_mcp_basic() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join("claude.json");
    fs::write(
        &config,
        r#"{
            "mcpServers": {
                "sequential-thinking": {
                    "command": "npx",
                    "args": ["-y", "@anthropic/sequential-thinking"]
                },
                "slack": {
                    "command": "npx",
                    "args": ["-y", "@anthropic/slack-mcp"],
                    "env": { "SLACK_TOKEN": "xoxb-1234" }
                }
            }
        }"#,
    )
    .unwrap();

    let servers = parse_claude_mcp(&config).unwrap();
    assert_eq!(servers.len(), 2);

    let (name, server) = servers.iter().find(|(n, _)| n == "slack").unwrap();
    assert_eq!(name, "slack");
    assert_eq!(server.command.as_deref(), Some("npx"));
    assert_eq!(server.env.get("SLACK_TOKEN").unwrap(), "xoxb-1234");
}

#[test]
fn parse_codex_mcp_basic() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join("config.toml");
    fs::write(
        &config,
        r#"
[mcp_servers.calendar]
command = "npx"
args = ["-y", "@google/calendar-mcp"]

[mcp_servers.calendar.env]
GOOGLE_TOKEN = "secret-123"
"#,
    )
    .unwrap();

    let servers = parse_codex_mcp(&config).unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].0, "calendar");
    assert_eq!(servers[0].1.command.as_deref(), Some("npx"));
    assert_eq!(servers[0].1.env.get("GOOGLE_TOKEN").unwrap(), "secret-123");
}

#[test]
fn parse_codex_mcp_preserves_known_provider_fields() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join("config.toml");
    fs::write(
        &config,
        r#"
[mcp_servers.calendar]
command = "npx"
args = ["-y", "@google/calendar-mcp"]
env_vars = ["GOOGLE_TOKEN"]
bearer_token_env_var = "CALENDAR_BEARER"
enabled = false
required = true
startup_timeout_sec = 15
tool_timeout_sec = 45
enabled_tools = ["list_events"]
disabled_tools = ["delete_event"]

[mcp_servers.calendar.env_http_headers]
Authorization = "AUTH_TOKEN"
"#,
    )
    .unwrap();

    let servers = parse_codex_mcp(&config).unwrap();
    let server = &servers[0].1;
    assert!(server.required);
    assert_eq!(server.enabled_tools, vec!["list_events"]);
    assert_eq!(server.disabled_tools, vec!["delete_event"]);
    assert_eq!(
        server
            .provider_override_object("codex")
            .unwrap()
            .get("env_vars")
            .unwrap(),
        &json!(["GOOGLE_TOKEN"])
    );
    assert_eq!(
        server
            .provider_override_object("codex")
            .unwrap()
            .get("bearer_token_env_var")
            .unwrap(),
        &json!("CALENDAR_BEARER")
    );
    assert_eq!(
        server
            .provider_override_object("codex")
            .unwrap()
            .get("enabled")
            .unwrap(),
        &json!(false)
    );
}

#[test]
fn parse_gemini_mcp_basic() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join("settings.json");
    fs::write(
        &config,
        r#"{
            "mcpServers": {
                "linear": {
                    "command": "npx",
                    "args": ["-y", "@linear/mcp"],
                    "include-tools": ["create_issue"]
                }
            }
        }"#,
    )
    .unwrap();

    let servers = parse_gemini_mcp(&config).unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].1.enabled_tools, vec!["create_issue"]);
}

#[test]
fn parse_opencode_mcp_basic() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join("opencode.json");
    fs::write(
        &config,
        r#"{
            "mcp": {
                "github": {
                    "type": "local",
                    "command": "gh-mcp"
                }
            }
        }"#,
    )
    .unwrap();

    let servers = parse_opencode_mcp(&config).unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].1.transport, McpTransport::Stdio);
}

#[test]
fn parse_opencode_mcp_command_array_round_trips() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join("opencode.json");
    fs::write(
        &config,
        r#"{
            "mcp": {
                "github": {
                    "type": "local",
                    "command": ["npx", "-y", "@company/github-mcp"],
                    "enabled": true,
                    "timeout": 5000,
                    "oauth": {}
                }
            }
        }"#,
    )
    .unwrap();

    let servers = parse_opencode_mcp(&config).unwrap();
    let server = &servers[0].1;
    assert_eq!(server.command.as_deref(), Some("npx"));
    assert_eq!(server.args, vec!["-y", "@company/github-mcp"]);
    assert_eq!(
        server
            .provider_override_object("opencode")
            .unwrap()
            .get("timeout")
            .unwrap(),
        &json!(5000)
    );
}

#[test]
fn import_idempotent() {
    let tmp = TempDir::new().unwrap();
    let catalog_path = tmp.path().join("catalog.json");
    let state_path = tmp.path().join("state.json");

    let config = tmp.path().join("claude.json");
    fs::write(
        &config,
        r#"{
            "mcpServers": {
                "test-server": {
                    "command": "npx",
                    "args": ["-y", "@test/server"]
                }
            }
        }"#,
    )
    .unwrap();

    // First import
    let mut catalog = McpCatalogStore::load_from(&catalog_path).unwrap();
    let mut state = McpProviderStateStore::load_from(&state_path).unwrap();
    {
        let mut importer = McpImporter::new(&mut catalog, &mut state);
        let servers = parse_claude_mcp(&config).unwrap();
        let mut report = ImportReport::default();
        for (native_name, server) in servers {
            importer.process_import(
                Provider::Claude,
                &Scope::User,
                &config,
                &native_name,
                server,
                &mut report,
            );
        }
        assert_eq!(report.imported.len(), 1);
        assert!(report.merged.is_empty());
    }

    // Second import — should merge, not import
    {
        let mut importer = McpImporter::new(&mut catalog, &mut state);
        let servers = parse_claude_mcp(&config).unwrap();
        let mut report = ImportReport::default();
        for (native_name, server) in servers {
            importer.process_import(
                Provider::Claude,
                &Scope::User,
                &config,
                &native_name,
                server,
                &mut report,
            );
        }
        assert!(report.imported.is_empty());
        assert_eq!(report.merged.len(), 1);
    }

    assert_eq!(catalog.list_servers().len(), 1);
}

#[test]
fn import_same_fingerprint_different_name_adds_alias() {
    let tmp = TempDir::new().unwrap();
    let catalog_path = tmp.path().join("catalog.json");
    let state_path = tmp.path().join("state.json");

    // Claude config has "my-server"
    let claude_config = tmp.path().join("claude.json");
    fs::write(
        &claude_config,
        r#"{ "mcpServers": { "my-server": { "command": "npx", "args": ["-y", "@test/server"] } } }"#,
    )
    .unwrap();

    // Codex config has same definition named "server"
    let codex_config = tmp.path().join("config.toml");
    fs::write(
        &codex_config,
        "[mcp_servers.server]\ncommand = \"npx\"\nargs = [\"-y\", \"@test/server\"]\n",
    )
    .unwrap();

    let mut catalog = McpCatalogStore::load_from(&catalog_path).unwrap();
    let mut state = McpProviderStateStore::load_from(&state_path).unwrap();

    // Import from Claude first
    {
        let mut importer = McpImporter::new(&mut catalog, &mut state);
        let servers = parse_claude_mcp(&claude_config).unwrap();
        let mut report = ImportReport::default();
        for (native_name, server) in servers {
            importer.process_import(
                Provider::Claude,
                &Scope::User,
                &claude_config,
                &native_name,
                server,
                &mut report,
            );
        }
        assert_eq!(report.imported.len(), 1);
    }

    // Import from Codex — same fingerprint, different name
    {
        let mut importer = McpImporter::new(&mut catalog, &mut state);
        let servers = parse_codex_mcp(&codex_config).unwrap();
        let mut report = ImportReport::default();
        for (native_name, server) in servers {
            importer.process_import(
                Provider::Codex,
                &Scope::User,
                &codex_config,
                &native_name,
                server,
                &mut report,
            );
        }
        assert_eq!(report.merged.len(), 1);
        assert!(report.merged[0].alias_added.is_some());
    }

    // Should still have only 1 server
    assert_eq!(catalog.list_servers().len(), 1);
    let server = catalog.get_server("my-server").unwrap();
    assert!(server.aliases.contains(&"server".to_string()));
}

/// L1 / D9: a per-file parse failure must project a catalog-shaped
/// `DiagnosticSnapshot` beside its prose `reason`, not merely stringify.
///
/// `import_provider` also discovers the ambient `$HOME` configs, so the
/// assertions key on the planted path rather than on `errors` as a whole.
#[test]
fn import_provider_projects_a_diagnostic_snapshot_for_a_parse_failure() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    let planted = repo_root.join(".mcp.json");
    fs::write(&planted, "{ this is not json").unwrap();

    let catalog_path = repo_root.join("catalog.json");
    let state_path = repo_root.join("state.json");
    let mut catalog = McpCatalogStore::load_from(&catalog_path).unwrap();
    let mut state = McpProviderStateStore::load_from(&state_path).unwrap();
    let mut importer = McpImporter::new(&mut catalog, &mut state);

    let report = importer.import_provider(Provider::Claude, Some(repo_root));

    let planted_str = planted.to_string_lossy().to_string();
    let error = report
        .errors
        .iter()
        .find(|e| e.native_name == planted_str)
        .expect("the planted malformed config must be reported as an error");

    // The prose field is unchanged — the snapshot is additive beside it.
    assert!(!error.reason.is_empty());

    // D7: a registered code projects a catalog-shaped detail object, never a
    // top-level null.
    let snapshot = &error.diagnostic;
    assert!(!snapshot.code.is_empty());
    assert_eq!(snapshot.category, snapshot.code.split('.').next().unwrap());
    assert!(
        !snapshot.detail.is_null(),
        "a registered code must not project a top-level null detail"
    );
    assert!(!snapshot.message.is_empty());
    assert_eq!(
        snapshot.schema_version,
        crate::diagnostics::DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION
    );
}

/// The snapshot must survive the report's own serialization boundary, since
/// `ImportReport` is what the CLI emits as machine output.
#[test]
fn import_error_snapshot_round_trips_through_the_report_serialization() {
    let error = ImportError {
        provider: Provider::Claude,
        native_name: "/tmp/.mcp.json".to_string(),
        reason: "boom".to_string(),
        diagnostic: DiagnosticSnapshot::from_diagnostic(&crate::error::ClaudineError::JsonParse(
            serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
        )),
    };

    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(json["reason"], "boom");
    assert!(
        json["diagnostic"]["detail"].is_object(),
        "detail must serialize as an object: {json}"
    );
    assert!(json["diagnostic"]["code"].as_str().is_some());
}
