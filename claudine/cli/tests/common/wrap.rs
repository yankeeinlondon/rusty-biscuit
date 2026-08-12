//! Shared fixtures for the `wrap_*` integration test binaries.
//!
//! The `cli/tests/wrap_commands.rs` god file was split by responsibility
//! into themed `wrap_*.rs` binaries (basics, provider flags, structured
//! stream, opencode, compose, inline-compose, watchdog/timeout, sigint,
//! and the opencode-models dry-run guard). Every helper here was lifted
//! **verbatim** from the original `wrap_commands.rs` so the split stays a
//! pure relocation. Helpers used by only one binary still live here; the
//! module-level `#![allow(dead_code)]` keeps per-binary unused warnings
//! quiet.

#![allow(dead_code)]

use super::{init_git_repo, write, write_json};
use chrono::Local;
use claudine::mcp::types::{
    McpCatalog, McpDefaults, McpProviderState, McpServer, McpServerMetadata, McpTransport,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn create_claudine_monorepo(workspace: &Path) -> Option<(PathBuf, PathBuf, PathBuf)> {
    let repo_root = workspace.join("repo");
    let launch_dir = repo_root.join("claudine/cli");
    let lib_dir = repo_root.join("claudine/lib");
    let bin_dir = repo_root.join("bin");

    fs::create_dir_all(launch_dir.join("src")).unwrap();
    fs::create_dir_all(lib_dir.join("src")).unwrap();
    fs::create_dir_all(&bin_dir).unwrap();

    write(
        &repo_root.join("Cargo.toml"),
        r#"[workspace]
resolver = "2"
members = ["claudine/lib", "claudine/cli"]
"#,
    );
    write(
        &lib_dir.join("Cargo.toml"),
        r#"[package]
name = "claudine"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(&lib_dir.join("src/lib.rs"), "");
    write(
        &launch_dir.join("Cargo.toml"),
        r#"[package]
name = "claudine-cli"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(&launch_dir.join("src/main.rs"), "fn main() {}\n");

    if !init_git_repo(&repo_root) {
        return None;
    }

    Some((repo_root, launch_dir, bin_dir))
}

pub(crate) fn redact_session_id(input: &str) -> String {
    let result = redact_temp_home(input);
    const PREFIX: &str = "CLAUDINE_SESSION_ID=";
    let Some(start) = result.find(PREFIX) else {
        return result;
    };
    let value_start = start + PREFIX.len();
    let value_end = (value_start + 36).min(result.len());
    format!(
        "{}{}<redacted>{}",
        &result[..start],
        PREFIX,
        &result[value_end..]
    )
}

pub(crate) fn redact_claudine_pid(input: &str) -> String {
    const PREFIX: &str = "CLAUDINE_PID=";
    let mut result = input.to_string();
    let mut search_from = 0;
    while let Some(start) = result[search_from..].find(PREFIX) {
        let start = search_from + start;
        let value_start = start + PREFIX.len();
        let value_end = result[value_start..]
            .find(|c: char| !c.is_ascii_digit())
            .map(|i| value_start + i)
            .unwrap_or(result.len());
        result.replace_range(value_start..value_end, "<redacted>");
        search_from = start + PREFIX.len() + "<redacted>".len();
    }
    result
}

pub(crate) fn redact_temp_home(input: &str) -> String {
    const MARKER: &str = "HOME=/var/folders/";
    let Some(start) = input.find(MARKER) else {
        return input.to_string();
    };
    let value_start = start + 5;
    let after = &input[value_start..];
    let end = after.find('\n').unwrap_or(after.len());
    format!("{}HOME=<redacted>{}", &input[..start], &after[end..])
}

/// Replace every occurrence of the tempdir root (and its canonicalized form)
/// with `<workspace>` so snapshots are stable across machines and runs.
///
/// macOS canonicalizes `/var/folders/...` → `/private/var/folders/...`, so
/// the canonical (longer) form must be replaced first to avoid leaving a
/// stray `/private` prefix.
pub(crate) fn redact_workspace_paths(workspace: &Path, input: &str) -> String {
    let mut out = input.to_string();
    if let Ok(canon) = workspace.canonicalize() {
        let canon = canon.display().to_string();
        if !canon.is_empty() {
            out = out.replace(&canon, "<workspace>");
        }
    }
    let raw = workspace.display().to_string();
    if !raw.is_empty() {
        out = out.replace(&raw, "<workspace>");
    }
    out
}

pub(crate) fn today_log_path(home: &Path) -> std::path::PathBuf {
    home.join(".claudine")
        .join("logs")
        .join(format!("{}.jsonl", Local::now().format("%Y-%m-%d")))
}

pub(crate) fn make_server(id: &str) -> McpServer {
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

pub(crate) fn seed_catalog(home: &Path, servers: &[McpServer]) {
    write_json(
        &home.join(".claudine/mcp/catalog.json"),
        &McpCatalog {
            version: 1,
            servers: servers
                .iter()
                .cloned()
                .map(|server| (server.id.clone(), server))
                .collect(),
        },
    );
}

pub(crate) fn seed_defaults(home: &Path, ids: &[&str]) {
    write_json(
        &home.join(".claudine/mcp/defaults.json"),
        &McpDefaults {
            version: 1,
            defaults: ids.iter().map(|id| (*id).to_string()).collect(),
        },
    );
}

pub(crate) fn seed_empty_provider_state(home: &Path) {
    write_json(
        &home.join(".claudine/mcp/provider-state.json"),
        &McpProviderState {
            version: 1,
            providers: HashMap::new(),
            repos: HashMap::new(),
        },
    );
}

pub(crate) fn seed_minimal_config(home: &Path) {
    let dir = home.join(".claudine");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("config.json"), "{}").unwrap();
}
