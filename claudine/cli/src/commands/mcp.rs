use std::io;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use claudine::events::{PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::linking::resolve_repo_root;
use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::defaults::{self, load_repo_defaults, load_user_defaults};
use claudine::mcp::export::McpExporter;
use claudine::mcp::import::McpImporter;
use claudine::mcp::state::{McpProviderStateStore, Scope};
use color_eyre::eyre::{Result, eyre};
use serde_json::{Value, json};

use crate::log;

/// MCP (Model Context Protocol) server management.
#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: Option<McpCommand>,

    /// Output as JSON.
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Scan and import native provider configs.
    Init,
    /// Display full normalized definition and provenance for a server.
    Show(ShowArgs),
    /// Set user- or repo-scope default active servers.
    Default(DefaultArgs),
    /// Manage aliases for catalog entries.
    Alias(AliasArgs),
    /// Remove a server from the catalog.
    Remove(RemoveArgs),
    /// Export/sync to a native provider config.
    Sync(SyncExportArgs),
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Server ID, alias, or query.
    pub id: String,
}

#[derive(Debug, Args)]
pub struct DefaultArgs {
    /// Use repo scope instead of user scope.
    #[arg(long)]
    pub repo: bool,

    /// Server IDs to set as defaults.
    pub ids: Vec<String>,
}

#[derive(Debug, Args)]
pub struct AliasArgs {
    #[command(subcommand)]
    pub command: AliasCommand,
}

#[derive(Debug, Subcommand)]
pub enum AliasCommand {
    /// Add an alias to a server.
    Add(AliasAddArgs),
    /// Remove an alias.
    Remove(AliasRemoveArgs),
}

#[derive(Debug, Args)]
pub struct AliasAddArgs {
    /// Server ID to add the alias to.
    pub id: String,
    /// Alias to add.
    pub alias: String,
}

#[derive(Debug, Args)]
pub struct AliasRemoveArgs {
    /// Alias to remove.
    pub alias: String,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Server ID to remove.
    pub id: String,
}

#[derive(Debug, Args)]
pub struct SyncExportArgs {
    /// Provider to sync to (e.g. claude, codex, gemini).
    pub provider: String,

    /// Scope: user or repo.
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// Actually apply changes (default: dry run).
    #[arg(long)]
    pub apply: bool,
}

pub fn run(args: McpArgs) -> Result<()> {
    let json_output = args.json;

    match args.command {
        None => run_list(json_output),
        Some(McpCommand::Init) => run_init(json_output),
        Some(McpCommand::Show(show_args)) => run_show(&show_args.id, json_output),
        Some(McpCommand::Default(default_args)) => run_default(default_args, json_output),
        Some(McpCommand::Alias(alias_args)) => run_alias(alias_args, json_output),
        Some(McpCommand::Remove(remove_args)) => run_remove(&remove_args.id, json_output),
        Some(McpCommand::Sync(sync_args)) => run_sync(sync_args, json_output),
    }
}

/// `claudine mcp` — list all catalog entries.
fn run_list(json_output: bool) -> Result<()> {
    let catalog = McpCatalogStore::load()?;
    let state = McpProviderStateStore::load()?;
    let user_defaults = load_user_defaults()?.defaults;
    let repo_root = current_repo_root()?;
    let repo_defaults = repo_root
        .as_deref()
        .map(load_repo_defaults)
        .transpose()?
        .flatten()
        .map(|defaults| defaults.defaults)
        .unwrap_or_default();
    let active_defaults = defaults::effective_defaults(repo_root.as_deref(), &catalog)?;
    let servers = catalog.list_servers();

    if json_output {
        let entries: Vec<Value> = servers
            .iter()
            .map(|server| {
                json!({
                    "id": server.id,
                    "aliases": server.aliases,
                    "transport": server.transport,
                    "defaults": {
                        "user": user_defaults.contains(&server.id),
                        "repo": repo_defaults.contains(&server.id),
                        "active": active_defaults.contains(&server.id),
                    },
                    "providers": collect_provider_presence(&state, &server.id, repo_root.as_deref()),
                })
            })
            .collect();
        log::data(&serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if servers.is_empty() {
        log::data("No MCP servers in catalog.");
        log::data("Run `claudine mcp init` to import from native provider configs.");
        return Ok(());
    }

    log::data(&format!(
        "{:<25} {:<10} {:<12} {:<28} ALIASES",
        "ID", "TRANSPORT", "DEFAULTS", "PROVIDERS"
    ));
    log::data(&"-".repeat(95));
    for server in servers {
        let transport = format!("{:?}", server.transport).to_lowercase();
        let defaults = format_defaults(&server.id, &user_defaults, &repo_defaults, &active_defaults);
        let providers = collect_provider_presence(&state, &server.id, repo_root.as_deref())
            .into_iter()
            .map(|value| {
                let provider = value["provider"].as_str().unwrap_or_default();
                let scope = value["scope"].as_str().unwrap_or_default();
                let native = value["native_name"].as_str().unwrap_or_default();
                if native == server.id {
                    format!("{provider}:{scope}")
                } else {
                    format!("{provider}:{scope}={native}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let aliases = if server.aliases.is_empty() {
            String::new()
        } else {
            server.aliases.join(", ")
        };
        log::data(&format!(
            "{:<25} {:<10} {:<12} {:<28} {}",
            server.id,
            transport,
            defaults,
            truncate_column(&providers, 28),
            aliases
        ));
    }

    Ok(())
}

/// `claudine mcp init` — scan and import.
fn run_init(json_output: bool) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;
    let mut state = McpProviderStateStore::load()?;

    let repo_root = current_repo_root()?;
    let mut importer = McpImporter::new(&mut catalog, &mut state);
    let report = importer.import_all(repo_root.as_deref());

    catalog.save()?;
    state.save()?;

    if json_output {
        let summary = json!({
            "imported": report.imported,
            "merged": report.merged,
            "conflicts": report.conflicts,
            "skipped": report.skipped,
            "errors": report.errors,
        });
        log::data(&serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    log::data("MCP import complete:");
    if !report.imported.is_empty() {
        log::data(&format!("  Imported: {}", report.imported.len()));
        for entry in &report.imported {
            log::data(&format!(
                "    + {} (from {:?}:{})",
                entry.catalog_id, entry.provider, entry.native_name
            ));
        }
    }
    if !report.merged.is_empty() {
        log::data(&format!("  Merged:   {}", report.merged.len()));
        for entry in &report.merged {
            let alias_note = entry
                .alias_added
                .as_ref()
                .map(|alias| format!(" (alias added: {alias})"))
                .unwrap_or_default();
            log::data(&format!(
                "    ~ {} (from {:?}:{}){}",
                entry.catalog_id, entry.provider, entry.native_name, alias_note
            ));
        }
    }
    if !report.conflicts.is_empty() {
        log::data(&format!("  Conflicts: {}", report.conflicts.len()));
        for entry in &report.conflicts {
            log::data(&format!(
                "    ! {} -> {} (from {:?})",
                entry.name, entry.new_catalog_id, entry.provider
            ));
        }
    }
    if !report.skipped.is_empty() {
        log::data(&format!("  Skipped: {}", report.skipped.len()));
    }
    if !report.errors.is_empty() {
        log::data(&format!("  Errors: {}", report.errors.len()));
        for entry in &report.errors {
            log::data(&format!(
                "    x {:?}:{} - {}",
                entry.provider, entry.native_name, entry.reason
            ));
        }
    }
    if report.imported.is_empty()
        && report.merged.is_empty()
        && report.conflicts.is_empty()
        && report.skipped.is_empty()
    {
        log::data("  No MCP servers found in native provider configs.");
    }

    Ok(())
}

/// `claudine mcp show <id>` — display server details.
fn run_show(id: &str, json_output: bool) -> Result<()> {
    let catalog = McpCatalogStore::load()?;
    let state = McpProviderStateStore::load()?;
    let server = catalog.resolve(id).map_err(|e| eyre!("{e}"))?;
    let provenance = collect_provenance(&state, &server.id);

    if json_output {
        log::data(&serde_json::to_string_pretty(&json!({
            "server": server,
            "provenance": provenance,
        }))?);
        return Ok(());
    }

    log::data(&format!("ID:          {}", server.id));
    if !server.aliases.is_empty() {
        log::data(&format!("Aliases:     {}", server.aliases.join(", ")));
    }
    log::data(&format!(
        "Transport:   {}",
        format!("{:?}", server.transport).to_lowercase()
    ));
    if let Some(ref cmd) = server.command {
        log::data(&format!("Command:     {}", cmd));
    }
    if !server.args.is_empty() {
        log::data(&format!("Args:        {}", server.args.join(" ")));
    }
    if let Some(ref url) = server.url {
        log::data(&format!("URL:         {}", url));
    }
    if !server.env.is_empty() {
        log::data(&format!(
            "Env:         {}",
            redacted_keys(server.env.keys().cloned().collect())
        ));
    }
    if !server.headers.is_empty() {
        log::data(&format!(
            "Headers:     {}",
            redacted_keys(server.headers.keys().cloned().collect())
        ));
    }
    if let Some(ref desc) = server.metadata.description {
        log::data(&format!("Description: {}", desc));
    }
    if let Some(ref from) = server.metadata.created_from {
        log::data(&format!("Created from: {}", from));
    }
    log::data(&format!("Fingerprint: {}", server.metadata.fingerprint));

    if provenance.is_empty() {
        log::data("Provenance:  none");
    } else {
        log::data("Provenance:");
        for entry in provenance {
            log::data(&format!(
                "  - {}:{} as {} ({})",
                entry["provider"].as_str().unwrap_or_default(),
                entry["scope"].as_str().unwrap_or_default(),
                entry["native_name"].as_str().unwrap_or_default(),
                entry["origin"].as_str().unwrap_or_default()
            ));
        }
    }

    Ok(())
}

/// `claudine mcp default [ids...]`
fn run_default(args: DefaultArgs, json_output: bool) -> Result<()> {
    if args.repo {
        let repo_root = current_repo_root()?.ok_or_else(|| eyre!("failed to resolve repo root"))?;
        defaults::set_repo_defaults(&repo_root, args.ids.clone())?;
        if json_output {
            log::data(&serde_json::to_string_pretty(&json!({
                "scope": "repo",
                "repo_root": repo_root,
                "defaults": args.ids,
            }))?);
        } else {
            log::data(&format!("Repo defaults set: {}", args.ids.join(", ")));
        }
    } else {
        defaults::set_user_defaults(args.ids.clone())?;
        if json_output {
            log::data(&serde_json::to_string_pretty(&json!({
                "scope": "user",
                "defaults": args.ids,
            }))?);
        } else {
            log::data(&format!("User defaults set: {}", args.ids.join(", ")));
        }
    }
    Ok(())
}

/// `claudine mcp alias add|remove`
fn run_alias(args: AliasArgs, json_output: bool) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;

    match args.command {
        AliasCommand::Add(add) => {
            catalog.add_alias(&add.id, &add.alias)?;
            catalog.save()?;
            if json_output {
                log::data(&serde_json::to_string_pretty(&json!({
                    "action": "add",
                    "id": add.id,
                    "alias": add.alias,
                }))?);
            } else {
                log::data(&format!("Alias '{}' added to server '{}'", add.alias, add.id));
            }
        }
        AliasCommand::Remove(remove) => {
            catalog.remove_alias(&remove.alias)?;
            catalog.save()?;
            if json_output {
                log::data(&serde_json::to_string_pretty(&json!({
                    "action": "remove",
                    "alias": remove.alias,
                }))?);
            } else {
                log::data(&format!("Alias '{}' removed", remove.alias));
            }
        }
    }
    Ok(())
}

/// `claudine mcp remove <id>`
fn run_remove(id: &str, json_output: bool) -> Result<()> {
    confirm_remove(id)?;

    let mut catalog = McpCatalogStore::load()?;
    let server = catalog.remove_server(id)?;
    catalog.save()?;

    if json_output {
        log::data(&serde_json::to_string_pretty(&json!({
            "removed": server.id,
        }))?);
    } else {
        log::data(&format!("Removed server '{}' from catalog", server.id));
    }
    Ok(())
}

/// `claudine mcp sync <provider>`
fn run_sync(args: SyncExportArgs, json_output: bool) -> Result<()> {
    let provider = Provider::fuzzy_match_cli_name(&args.provider)
        .ok_or_else(|| eyre!("unknown provider: {}", args.provider))?;

    let catalog = McpCatalogStore::load()?;
    let mut state = McpProviderStateStore::load()?;

    let scope = match args.scope.as_str() {
        "repo" => Scope::Repo(current_repo_root()?.ok_or_else(|| eyre!("failed to resolve repo root"))?),
        "user" => Scope::User,
        other => return Err(eyre!("unknown scope '{}'; expected user or repo", other)),
    };

    let repo_root = match &scope {
        Scope::User => None,
        Scope::Repo(root) => Some(root.as_path()),
    };

    let default_ids = defaults::effective_defaults(repo_root, &catalog)?;
    let mut unresolved = Vec::new();
    let mut servers = Vec::new();

    for id in &default_ids {
        match catalog.resolve(id) {
            Ok(server) => servers.push(server.clone()),
            Err(_) => unresolved.push(id.clone()),
        }
    }

    let mut exporter = McpExporter::new(&catalog, &mut state);
    let report = exporter.sync_provider(provider, &scope, &servers, args.apply)?;
    if args.apply {
        state.save()?;
    }

    if json_output {
        log::data(&serde_json::to_string_pretty(&json!({
            "provider": provider.as_slug(),
            "scope": scope_name(&scope),
            "applied": args.apply,
            "written": report.written,
            "removed": report.removed,
            "preserved": report.preserved,
            "unresolved": unresolved,
        }))?);
        return Ok(());
    }

    if args.apply {
        log::data(&format!("Sync applied to {}:", provider.as_slug()));
    } else {
        log::data(&format!(
            "Sync dry run for {} (use --apply to write):",
            provider.as_slug()
        ));
    }
    if !report.written.is_empty() {
        log::data(&format!("  Write: {}", report.written.join(", ")));
    }
    if !report.removed.is_empty() {
        log::data(&format!("  Remove: {}", report.removed.join(", ")));
    }
    if !report.preserved.is_empty() {
        log::data(&format!("  Preserve: {}", report.preserved.join(", ")));
    }
    if !unresolved.is_empty() {
        log::data(&format!("  Unresolved defaults: {}", unresolved.join(", ")));
    }

    Ok(())
}

fn current_repo_root() -> Result<Option<PathBuf>> {
    let cwd = std::env::current_dir()?;
    Ok(Some(resolve_repo_root(&cwd)))
}

fn collect_provider_presence(
    state: &McpProviderStateStore,
    catalog_id: &str,
    current_repo_root: Option<&Path>,
) -> Vec<Value> {
    let mut entries = Vec::new();
    let state = state.state();

    for provider in PROVIDERS_DISPLAY_ORDER {
        let slug = provider.as_slug();
        if let Some(provider_entries) = state.providers.get(slug) {
            for entry in &provider_entries.user {
                if entry.catalog_id == catalog_id {
                    entries.push(json!({
                        "provider": slug,
                        "scope": "user",
                        "native_name": entry.native_name,
                        "origin": format!("{:?}", entry.origin).to_lowercase(),
                    }));
                }
            }
        }

        if let Some(repo_root) = current_repo_root
            && let Some(repo_state) = state.repos.get(&repo_root.to_string_lossy().to_string())
            && let Some(provider_entries) = repo_state.providers.get(slug)
        {
            for entry in &provider_entries.repo {
                if entry.catalog_id == catalog_id {
                    entries.push(json!({
                        "provider": slug,
                        "scope": "repo",
                        "native_name": entry.native_name,
                        "origin": format!("{:?}", entry.origin).to_lowercase(),
                    }));
                }
            }
        }
    }

    entries
}

fn collect_provenance(state: &McpProviderStateStore, catalog_id: &str) -> Vec<Value> {
    let mut entries = Vec::new();
    let state = state.state();

    for provider in PROVIDERS_DISPLAY_ORDER {
        let slug = provider.as_slug();
        if let Some(provider_entries) = state.providers.get(slug) {
            for entry in &provider_entries.user {
                if entry.catalog_id == catalog_id {
                    entries.push(json!({
                        "provider": slug,
                        "scope": "user",
                        "native_name": entry.native_name,
                        "origin": format!("{:?}", entry.origin).to_lowercase(),
                        "source": entry.source,
                    }));
                }
            }
        }

        for (repo_path, repo_state) in &state.repos {
            if let Some(provider_entries) = repo_state.providers.get(slug) {
                for entry in &provider_entries.repo {
                    if entry.catalog_id == catalog_id {
                        entries.push(json!({
                            "provider": slug,
                            "scope": "repo",
                            "repo_root": repo_path,
                            "native_name": entry.native_name,
                            "origin": format!("{:?}", entry.origin).to_lowercase(),
                            "source": entry.source,
                        }));
                    }
                }
            }
        }
    }

    entries
}

fn confirm_remove(id: &str) -> Result<()> {
    log::message(&format!("Remove MCP server '{id}'? [y/N]"));
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let answer = line.trim().to_ascii_lowercase();
    if answer == "y" || answer == "yes" {
        return Ok(());
    }

    Err(eyre!("aborted removal of '{id}'"))
}

fn format_defaults(
    id: &str,
    user_defaults: &[String],
    repo_defaults: &[String],
    active_defaults: &[String],
) -> String {
    let mut scopes = Vec::new();
    if user_defaults.contains(&id.to_string()) {
        scopes.push("user");
    }
    if repo_defaults.contains(&id.to_string()) {
        scopes.push("repo");
    }
    if active_defaults.contains(&id.to_string()) {
        scopes.push("active");
    }
    if scopes.is_empty() {
        "-".to_string()
    } else {
        scopes.join(",")
    }
}

fn redacted_keys(mut keys: Vec<String>) -> String {
    keys.sort();
    keys.into_iter()
        .map(|key| format!("{key}=<redacted>"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn scope_name(scope: &Scope) -> &'static str {
    match scope {
        Scope::User => "user",
        Scope::Repo(_) => "repo",
    }
}

fn truncate_column(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }

    value.chars().take(max_len.saturating_sub(3)).collect::<String>() + "..."
}
