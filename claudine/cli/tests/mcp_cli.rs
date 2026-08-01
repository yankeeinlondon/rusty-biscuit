#[cfg(unix)]
use std::collections::HashMap;
use std::fs;
#[cfg(unix)]
use std::path::Path;

#[cfg(unix)]
use claudine::mcp::types::{
    McpCatalog, McpDefaults, McpOrigin, McpProviderState, McpServer, McpServerMetadata,
    McpTransport, ProviderScopeEntries, ProviderStateEntry, RepoProviderState,
};
use predicates::str::contains;
mod common;
use common::{TestWorkspace, init_git_repo};
#[cfg(unix)]
use common::{write, write_executable, write_json};

#[cfg(unix)]
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
            created_from: Some("codex:user".into()),
            fingerprint: format!("fp-{id}"),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        provider_overrides: HashMap::new(),
    }
}

#[cfg(unix)]
fn seed_catalog(home: &Path, servers: &[McpServer]) {
    let catalog = McpCatalog {
        version: 1,
        servers: servers
            .iter()
            .cloned()
            .map(|server| (server.id.clone(), server))
            .collect(),
    };
    write_json(&home.join(".claudine/mcp/catalog.json"), &catalog);
}

#[cfg(unix)]
fn seed_defaults(home: &Path, ids: &[&str]) {
    write_json(
        &home.join(".claudine/mcp/defaults.json"),
        &McpDefaults {
            version: 1,
            defaults: ids.iter().map(|id| (*id).to_string()).collect(),
        },
    );
}

#[cfg(unix)]
fn seed_provider_state(
    home: &Path,
    repo_root: Option<&Path>,
    provider: &str,
    catalog_id: &str,
    native_name: &str,
    origin: McpOrigin,
    source: &Path,
) {
    let mut state = McpProviderState {
        version: 1,
        providers: HashMap::new(),
        repos: HashMap::new(),
    };

    let entry = ProviderStateEntry {
        catalog_id: catalog_id.into(),
        native_name: native_name.into(),
        source: source.display().to_string(),
        origin,
        last_seen: chrono::Utc::now(),
    };

    if let Some(repo_root) = repo_root {
        state.repos.insert(
            repo_root.display().to_string(),
            RepoProviderState {
                providers: HashMap::from([(
                    provider.into(),
                    ProviderScopeEntries {
                        user: Vec::new(),
                        repo: vec![entry],
                    },
                )]),
            },
        );
    } else {
        state.providers.insert(
            provider.into(),
            ProviderScopeEntries {
                user: vec![entry],
                repo: Vec::new(),
            },
        );
    }

    write_json(&home.join(".claudine/mcp/provider-state.json"), &state);
}

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn mcp_show_json_includes_provenance() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let server = make_server("calendar");
    seed_catalog(&home, std::slice::from_ref(&server));
    seed_provider_state(
        &home,
        None,
        "codex",
        "calendar",
        "calendar-native",
        McpOrigin::Imported,
        &home.join(".codex/config.toml"),
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "show", "calendar", "--json"])
        .assert()
        .success();

    let value: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(value["server"]["id"], "calendar");
    assert_eq!(value["provenance"][0]["provider"], "codex");
    assert_eq!(value["provenance"][0]["native_name"], "calendar-native");
}

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn mcp_config_json_uses_new_command_name() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let mut server = make_server("calendar");
    server.aliases.push("gcal".into());
    seed_catalog(&home, std::slice::from_ref(&server));

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "config", "gcal", "--json"])
        .assert()
        .success();

    let value: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(value["server"]["id"], "calendar");
    assert_eq!(value["server"]["aliases"][0], "gcal");
}

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn mcp_check_json_reports_invalid_servers() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let mut invalid = make_server("broken");
    invalid.command = None;
    seed_catalog(&home, &[invalid]);

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "check", "--json"])
        .assert()
        .success();

    let value: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert!(
        value["issues"]
            .as_array()
            .is_some_and(|issues| !issues.is_empty())
    );
    assert_eq!(value["issues"][0]["code"], "stdio-missing-command");
}

#[test]
fn mcp_default_repo_uses_repo_root_from_nested_directory() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    let nested = repo_root.join("claudine/cli");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&nested).unwrap();

    if !init_git_repo(&repo_root) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&nested)
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "default", "--repo", "calendar", "slack"])
        .assert()
        .success();

    let defaults_path = repo_root.join(".claudine/mcp.json");
    assert!(defaults_path.exists());
    assert!(!nested.join(".claudine/mcp.json").exists());
}

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn mcp_export_reports_unresolved_defaults_and_uses_native_name() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    let nested = repo_root.join("claudine/cli");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&nested).unwrap();

    if !init_git_repo(&repo_root) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    let server = make_server("google-calendar");
    seed_catalog(&home, std::slice::from_ref(&server));
    write_json(
        &repo_root.join(".claudine/mcp.json"),
        &McpDefaults {
            version: 1,
            defaults: vec!["google-calendar".into(), "missing-server".into()],
        },
    );

    let codex_config = repo_root.join(".codex/config.toml");
    write(
        &codex_config,
        r#"
[mcp_servers.calendar]
command = "npx"
args = ["-y", "@test/google-calendar"]
"#,
    );
    seed_provider_state(
        &home,
        Some(&repo_root.canonicalize().unwrap_or(repo_root.clone())),
        "codex",
        "google-calendar",
        "calendar",
        McpOrigin::Imported,
        &codex_config,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&nested)
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args([
            "mcp", "export", "codex", "--scope", "repo", "--apply", "--json",
        ])
        .assert()
        .success();

    let value: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(value["written"][0], "calendar");
    assert_eq!(value["unresolved"][0], "missing-server");

    let content = fs::read_to_string(&codex_config).unwrap();
    assert!(content.contains("[mcp_servers.calendar]"));
    assert!(!content.contains("[mcp_servers.google-calendar]"));
}

#[cfg(unix)]
#[test]
fn codex_wrapper_mcp_dry_run_shows_cleaned_prompt_and_shadow_file() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(home.join(".codex")).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    seed_catalog(&home, &[make_server("calendar")]);
    seed_defaults(&home, &["calendar"]);

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "codex",
            "--mcp",
            "--dry-run",
            "--",
            "--json",
            "fix #calendar bugs",
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("cleaned_prompt"));
    assert!(stderr.contains("fix bugs"));
    assert!(stderr.contains("calendar"));
    assert!(stderr.contains(".codex/config.toml"));
}

#[cfg(unix)]
#[test]
fn gemini_and_opencode_wrapper_mcp_dry_run_show_provider_specific_injection() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(home.join(".gemini")).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("gemini"), "#!/bin/sh\nexit 0\n");
    write_executable(&path_dir.join("opencode"), "#!/bin/sh\nexit 0\n");

    seed_catalog(&home, &[make_server("linear"), make_server("github")]);
    seed_defaults(&home, &[]);

    let gemini = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "gemini",
            "--mcp",
            "--use",
            "linear",
            "--dry-run",
            "--",
            "--prompt",
            "fix #linear auth",
        ])
        .assert()
        .success();
    let gemini_stderr = String::from_utf8_lossy(&gemini.get_output().stderr);
    assert!(gemini_stderr.contains("--allowed-mcp-server-names linear"));
    assert!(gemini_stderr.contains(".gemini/settings.json"));

    let opencode = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("OPENCODE_MODEL", "test-model")
        .env("PATH", &path_dir)
        .args([
            "opencode",
            "--mcp",
            "--use",
            "github",
            "--dry-run",
            "--",
            "debug #github sync",
        ])
        .assert()
        .success();
    let opencode_stderr = String::from_utf8_lossy(&opencode.get_output().stderr);
    assert!(opencode_stderr.contains("OPENCODE_CONFIG_CONTENT"));
    assert!(opencode_stderr.contains("cleaned_prompt"));
}

#[cfg(unix)]
#[test]
fn claude_wrapper_mcp_reports_sync_guidance() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(home.join(".claude")).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    seed_catalog(&home, &[make_server("calendar")]);
    seed_defaults(&home, &["calendar"]);

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args(["claude", "--mcp", "--dry-run", "--", "--print", "do work"])
        .assert()
        .failure()
        .stderr(contains("Use `claudine mcp export claude --apply`"));
}

// ---------------------------------------------------------------------------
// Recommendation #1: repo-root detection returns None outside a repo
// ---------------------------------------------------------------------------

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn mcp_list_outside_repo_returns_no_repo_defaults() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let non_repo = workspace.path().join("not-a-repo");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&non_repo).unwrap();

    seed_catalog(&home, &[make_server("calendar")]);
    seed_defaults(&home, &["calendar"]);

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&non_repo)
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "--json"])
        .assert()
        .success();

    let value: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let entry = &value[0];
    assert_eq!(entry["defaults"]["user"], true);
    // Outside a repo, repo defaults should not be set
    assert_eq!(entry["defaults"]["repo"], false);
}

#[test]
fn mcp_default_repo_fails_outside_repo() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let non_repo = workspace.path().join("not-a-repo");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&non_repo).unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&non_repo)
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "default", "--repo", "calendar"])
        .assert()
        .failure()
        .stderr(contains("repo root"));
}

// ---------------------------------------------------------------------------
// Recommendation #2: mcp remove cascades to defaults
// ---------------------------------------------------------------------------

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn mcp_remove_cascades_to_user_defaults() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let non_repo = workspace.path().join("not-a-repo");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&non_repo).unwrap();

    let server = make_server("calendar");
    let other = make_server("slack");
    seed_catalog(&home, &[server, other]);
    seed_defaults(&home, &["calendar", "slack"]);

    // Remove calendar (--json to skip interactive confirmation)
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&non_repo)
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "remove", "calendar", "--json"])
        .assert()
        .success();

    let value: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(value["removed_server"], "calendar");
    assert!(
        value["defaults_cleaned"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == "user"))
    );

    // Verify calendar was removed from defaults
    let defaults: McpDefaults = serde_json::from_str(
        &fs::read_to_string(home.join(".claudine/mcp/defaults.json")).unwrap(),
    )
    .unwrap();
    assert!(!defaults.defaults.contains(&"calendar".to_string()));
    assert!(defaults.defaults.contains(&"slack".to_string()));
}

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn mcp_remove_cascades_to_repo_defaults() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&repo_root).unwrap();

    if !init_git_repo(&repo_root) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    let server = make_server("calendar");
    seed_catalog(&home, &[server]);
    seed_defaults(&home, &["calendar"]);
    write_json(
        &repo_root.join(".claudine/mcp.json"),
        &McpDefaults {
            version: 1,
            defaults: vec!["calendar".into()],
        },
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&repo_root)
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "remove", "calendar", "--json"])
        .assert()
        .success();

    let value: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert!(
        value["defaults_cleaned"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == "repo"))
    );

    // Verify calendar removed from repo defaults
    let repo_defaults: McpDefaults =
        serde_json::from_str(&fs::read_to_string(repo_root.join(".claudine/mcp.json")).unwrap())
            .unwrap();
    assert!(!repo_defaults.defaults.contains(&"calendar".to_string()));
}

// ---------------------------------------------------------------------------
// Recommendation #6: sync no longer accepts <provider> positional arg
// ---------------------------------------------------------------------------

#[test]
fn mcp_sync_rejects_positional_provider() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "sync", "codex"])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Recommendation #7: repo defaults replace user defaults
// ---------------------------------------------------------------------------

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn effective_defaults_repo_replaces_user() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&repo_root).unwrap();

    if !init_git_repo(&repo_root) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    let server_a = make_server("user-only");
    let server_b = make_server("repo-only");
    seed_catalog(&home, &[server_a, server_b]);
    seed_defaults(&home, &["user-only"]);
    write_json(
        &repo_root.join(".claudine/mcp.json"),
        &McpDefaults {
            version: 1,
            defaults: vec!["repo-only".into()],
        },
    );

    // List from repo context — active defaults should be repo-only, not user-only
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&repo_root)
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "--json"])
        .assert()
        .success();

    let entries: Vec<serde_json::Value> =
        serde_json::from_slice(&assert.get_output().stdout).unwrap();
    let repo_only = entries.iter().find(|e| e["id"] == "repo-only").unwrap();
    let user_only = entries.iter().find(|e| e["id"] == "user-only").unwrap();

    assert_eq!(repo_only["defaults"]["active"], true);
    assert_eq!(user_only["defaults"]["active"], false);
}

// ---------------------------------------------------------------------------
// Recommendation #8: --strict behavior tests
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn strict_mode_errors_on_missing_tag() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("opencode"), "#!/bin/sh\nexit 0\n");

    seed_catalog(&home, &[make_server("calendar")]);
    seed_defaults(&home, &["calendar"]);

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("OPENCODE_MODEL", "test-model")
        .env("PATH", &path_dir)
        .args([
            "opencode",
            "--mcp",
            "--strict",
            "--dry-run",
            "--",
            "fix #nonexistent bugs",
        ])
        .assert()
        .failure()
        .stderr(contains("unresolved MCP tag(s)"));
}

#[cfg(unix)]
#[test]
fn strict_mode_errors_on_ambiguous_tag() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("opencode"), "#!/bin/sh\nexit 0\n");

    seed_catalog(
        &home,
        &[make_server("calendar"), make_server("calendar-beta")],
    );
    seed_defaults(&home, &[]);

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("OPENCODE_MODEL", "test-model")
        .env("PATH", &path_dir)
        .args([
            "opencode",
            "--mcp",
            "--strict",
            "--dry-run",
            "--",
            "fix #cal bugs",
        ])
        .assert()
        .failure()
        .stderr(contains("ambiguous MCP tag(s)"));
}

// ---------------------------------------------------------------------------
// Recommendation #8: mcp remove alias reports owner and remaining aliases
// ---------------------------------------------------------------------------

// This subprocess fixture requires Unix HOME isolation for user-global MCP state.
#[cfg(unix)]
#[test]
fn mcp_remove_alias_reports_owner_and_remaining() {
    let workspace = TestWorkspace::named("claudine-mcp-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let mut server = make_server("calendar");
    server.aliases = vec!["gcal".into(), "cal".into()];
    seed_catalog(&home, std::slice::from_ref(&server));

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "remove", "gcal", "--json"])
        .assert()
        .success();

    let value: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(value["removed_alias"], "gcal");
    assert_eq!(value["owner"], "calendar");
    assert!(
        value["remaining_aliases"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == "cal"))
    );
}
