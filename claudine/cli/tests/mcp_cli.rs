use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;
use claudine::mcp::types::{
    McpCatalog, McpDefaults, McpOrigin, McpProviderState, McpServer, McpServerMetadata,
    McpTransport, ProviderScopeEntries, ProviderStateEntry, RepoProviderState,
};
use predicates::str::contains;

struct TestWorkspace {
    root: PathBuf,
}

static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

impl TestWorkspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "claudine-mcp-it-{}-{nonce}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    write(path, &serde_json::to_string_pretty(value).unwrap());
}

fn init_git_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("init")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

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

fn seed_defaults(home: &Path, ids: &[&str]) {
    write_json(
        &home.join(".claudine/mcp/defaults.json"),
        &McpDefaults {
            version: 1,
            defaults: ids.iter().map(|id| (*id).to_string()).collect(),
        },
    );
}

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

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;
    write(path, content);
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[test]
fn mcp_show_json_includes_provenance() {
    let workspace = TestWorkspace::new();
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

    let assert = cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "show", "calendar", "--json"])
        .assert()
        .success();

    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(value["server"]["id"], "calendar");
    assert_eq!(value["provenance"][0]["provider"], "codex");
    assert_eq!(value["provenance"][0]["native_name"], "calendar-native");
}

#[test]
fn mcp_default_repo_uses_repo_root_from_nested_directory() {
    let workspace = TestWorkspace::new();
    let home = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    let nested = repo_root.join("claudine/cli");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&nested).unwrap();

    if !init_git_repo(&repo_root) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    cargo_bin_cmd!("claudine")
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

#[test]
fn mcp_sync_reports_unresolved_defaults_and_uses_native_name() {
    let workspace = TestWorkspace::new();
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

    let assert = cargo_bin_cmd!("claudine")
        .current_dir(&nested)
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .args(["mcp", "sync", "codex", "--scope", "repo", "--apply", "--json"])
        .assert()
        .success();

    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(value["written"][0], "calendar");
    assert_eq!(value["unresolved"][0], "missing-server");

    let content = fs::read_to_string(&codex_config).unwrap();
    assert!(content.contains("[mcp_servers.calendar]"));
    assert!(!content.contains("[mcp_servers.google-calendar]"));
}

#[cfg(unix)]
#[test]
fn codex_wrapper_mcp_dry_run_shows_cleaned_prompt_and_shadow_file() {
    let workspace = TestWorkspace::new();
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(home.join(".codex")).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    seed_catalog(&home, &[make_server("calendar")]);
    seed_defaults(&home, &["calendar"]);

    let assert = cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "codex",
            "--mcp",
            "--dry-run",
            "--non-interactive",
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
    let workspace = TestWorkspace::new();
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(home.join(".gemini")).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("gemini"), "#!/bin/sh\nexit 0\n");
    write_executable(&path_dir.join("opencode"), "#!/bin/sh\nexit 0\n");

    seed_catalog(&home, &[make_server("linear"), make_server("github")]);
    seed_defaults(&home, &[]);

    let gemini = cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "gemini",
            "--mcp",
            "--use",
            "linear",
            "--dry-run",
            "--non-interactive",
            "--",
            "--prompt",
            "fix #linear auth",
        ])
        .assert()
        .success();
    let gemini_stderr = String::from_utf8_lossy(&gemini.get_output().stderr);
    assert!(gemini_stderr.contains("--allowed-mcp-server-names linear"));
    assert!(gemini_stderr.contains(".gemini/settings.json"));

    let opencode = cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "opencode",
            "--mcp",
            "--use",
            "github",
            "--dry-run",
            "--non-interactive",
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
    let workspace = TestWorkspace::new();
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(home.join(".claude")).unwrap();
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    seed_catalog(&home, &[make_server("calendar")]);
    seed_defaults(&home, &["calendar"]);

    cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .args([
            "claude",
            "--mcp",
            "--dry-run",
            "--non-interactive",
            "--",
            "--print",
            "do work",
        ])
        .assert()
        .failure()
        .stderr(contains("Use `claudine mcp sync claude`"));
}
