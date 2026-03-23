use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Alignment, Margin};
use chrono::Utc;
use clap::{Args, Subcommand};
use claudine::events::{PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::linking::resolve_repo_root;
use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::defaults::{
    self, load_repo_defaults, load_user_defaults, save_repo_defaults, save_user_defaults,
};
use claudine::mcp::export::McpExporter;
use claudine::mcp::import::{ImportReport, McpImporter};
use claudine::mcp::state::{McpProviderStateStore, Scope};
use claudine::mcp::types::{
    McpDefaults, McpServer, McpServerMetadata, McpTransport, defaults_path, derive_server_id,
    repo_defaults_path,
};
use claudine::mcp::validation::{ValidationSeverity, validate_state};
use color_eyre::eyre::{Result, eyre};
use inquire::{Confirm, MultiSelect, Select, Text};
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
    /// Initialize MCP mode, import the catalog, and choose defaults.
    Init,
    /// List catalog entries.
    List(ListArgs),
    /// Add a local or remote MCP server.
    Add(AddArgs),
    /// Display full normalized definition and provenance for a server.
    Config(ConfigArgs),
    /// Deprecated alias for `config`.
    #[command(hide = true)]
    Show(ConfigArgs),
    /// Set user- or repo-scope default active servers.
    Default(DefaultArgs),
    /// Add an alias to a catalog entry.
    Alias(AliasArgs),
    /// Remove a server or alias from the catalog.
    Remove(RemoveArgs),
    /// Validate the current MCP state.
    Check,
    /// Refresh the catalog from provider configs (pull-style only).
    Sync(SyncArgs),
    /// Export effective defaults back into a provider config.
    Export(ExportArgs),
}

#[derive(Debug, Args, Clone, Default)]
pub struct ListArgs {
    /// Optional substring filter on server IDs.
    pub filter: Option<String>,

    /// Filter by alias substring instead of server ID.
    #[arg(long)]
    pub alias: Option<String>,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    #[command(subcommand)]
    pub kind: AddKind,
}

#[derive(Debug, Subcommand)]
pub enum AddKind {
    /// Add a local stdio MCP server interactively.
    Local,
    /// Add a remote HTTP MCP server interactively.
    Remote,
}

#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// Server ID, alias, or query.
    pub query: Option<String>,
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
    /// Existing server name or alias owner.
    pub name: Option<String>,

    /// Alias to add.
    pub alias: Option<String>,

    #[command(subcommand)]
    pub compatibility: Option<AliasCompatibilityCommand>,
}

#[derive(Debug, Subcommand)]
pub enum AliasCompatibilityCommand {
    /// Deprecated compatibility alias for `claudine mcp alias <name> <alias>`.
    Add(AliasAddArgs),
    /// Deprecated compatibility alias for removing only an alias.
    Remove(AliasRemoveArgs),
}

#[derive(Debug, Args)]
pub struct AliasAddArgs {
    pub name: String,
    pub alias: String,
}

#[derive(Debug, Args)]
pub struct AliasRemoveArgs {
    pub alias: String,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Server ID, alias, or fuzzy query.
    pub query: Option<String>,
}

#[derive(Debug, Args)]
pub struct SyncArgs {}

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Provider to export to (e.g. claude, codex, gemini).
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
        None => run_list(ListArgs::default(), json_output),
        Some(McpCommand::Init) => run_init(json_output),
        Some(McpCommand::List(list_args)) => run_list(list_args, json_output),
        Some(McpCommand::Add(add_args)) => run_add(add_args, json_output),
        Some(McpCommand::Config(config_args)) | Some(McpCommand::Show(config_args)) => {
            run_config(config_args.query.as_deref(), json_output)
        }
        Some(McpCommand::Default(default_args)) => run_default(default_args, json_output),
        Some(McpCommand::Alias(alias_args)) => run_alias(alias_args, json_output),
        Some(McpCommand::Remove(remove_args)) => {
            run_remove(remove_args.query.as_deref(), json_output)
        }
        Some(McpCommand::Check) => run_check(json_output),
        Some(McpCommand::Sync(sync_args)) => run_sync(sync_args, json_output),
        Some(McpCommand::Export(export_args)) => run_export(export_args, json_output),
    }
}

fn run_list(args: ListArgs, json_output: bool) -> Result<()> {
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
    let servers = filtered_servers(&catalog, &args);

    if json_output {
        let entries: Vec<Value> = servers
            .iter()
            .map(|server| {
                json!({
                    "id": server.id,
                    "aliases": server.aliases,
                    "transport": server.transport,
                    "auth": auth_summary(server),
                    "env": redacted_keys(server.env.keys().cloned().collect()),
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
        log::data("No MCP servers matched the current filter.");
        log::data("Run `claudine mcp init` or `claudine mcp sync` to populate the catalog.");
        return Ok(());
    }

    let term = crate::log::terminal();
    let mut table = base_table(vec![
        TableColumn::new("MCP server"),
        TableColumn::new("Aliases"),
        TableColumn::new("Type"),
        TableColumn::new("Auth"),
        TableColumn::new("ENV"),
    ]);

    for server in servers {
        let styled_name = styled_server_name(server, &user_defaults, &repo_defaults, &term);
        let aliases = if server.aliases.is_empty() {
            "—".to_string()
        } else {
            server.aliases.join(", ")
        };
        let env_summary = if server.env.is_empty() {
            "—".to_string()
        } else {
            redacted_keys(server.env.keys().cloned().collect())
        };

        table.add_row(vec![
            styled_name.into(),
            aliases.into(),
            transport_label(server).into(),
            auth_summary(server).into(),
            env_summary.into(),
        ]);
    }

    log::data(&table.render(&term));
    Ok(())
}

fn run_init(json_output: bool) -> Result<()> {
    let repo_root = current_repo_root()?;
    let user_defaults_exists = defaults_path().exists();
    let repo_defaults_exists = repo_root
        .as_deref()
        .is_some_and(|root| repo_defaults_path(root).exists());
    let report = bootstrap_mcp_state(repo_root.as_deref())?;

    if json_output {
        log::data(&serde_json::to_string_pretty(&json!({
            "imported": report.imported,
            "merged": report.merged,
            "conflicts": report.conflicts,
            "skipped": report.skipped,
            "errors": report.errors,
        }))?);
        return Ok(());
    }

    if user_defaults_exists
        && !repo_defaults_exists
        && let Some(repo_root) = repo_root
    {
        let catalog = McpCatalogStore::load()?;
        let user_defaults = load_user_defaults()?;

        if !user_defaults.defaults.is_empty() {
            log::data(&format!(
                "Current user defaults: {}",
                user_defaults.defaults.join(", ")
            ));
        }

        let selected = prompt_for_defaults(
            &catalog,
            "Select repo-default MCP servers:",
            &user_defaults.defaults,
        )?;
        save_repo_defaults(
            &repo_root,
            &McpDefaults {
                version: 1,
                defaults: selected.clone(),
            },
        )?;
        render_init_summary(
            &catalog,
            &selected,
            load_user_defaults()?.defaults.as_slice(),
            repo_root.as_path(),
        );
        return Ok(());
    }

    if user_defaults_exists || repo_defaults_exists {
        render_reentry_help(repo_root.as_deref());
        return Ok(());
    }

    let catalog = McpCatalogStore::load()?;
    let user_defaults = prompt_for_defaults(&catalog, "Select user-default MCP servers:", &[])?;
    save_user_defaults(&McpDefaults {
        version: 1,
        defaults: user_defaults.clone(),
    })?;

    let repo_defaults = if let Some(root) = repo_root.as_deref() {
        let selected = prompt_for_defaults(&catalog, "Select repo-default MCP servers:", &[])?;
        save_repo_defaults(
            root,
            &McpDefaults {
                version: 1,
                defaults: selected.clone(),
            },
        )?;
        selected
    } else {
        Vec::new()
    };

    render_init_summary(
        &catalog,
        &repo_defaults,
        &user_defaults,
        repo_root.as_deref().unwrap_or(Path::new(".")),
    );
    Ok(())
}

fn run_add(args: AddArgs, json_output: bool) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;
    let server = match args.kind {
        AddKind::Local => prompt_for_local_server()?,
        AddKind::Remote => prompt_for_remote_server()?,
    };
    let added = insert_manual_server(&mut catalog, server)?;
    catalog.save()?;

    if json_output {
        log::data(&serde_json::to_string_pretty(&json!({ "added": added }))?);
    } else {
        log::data(&format!("Added MCP server `{added}`."));
    }

    Ok(())
}

fn run_config(query: Option<&str>, json_output: bool) -> Result<()> {
    let catalog = McpCatalogStore::load()?;
    let state = McpProviderStateStore::load()?;
    let query = match query {
        Some(query) => query.to_string(),
        None => prompt_for_server_query(&catalog, "Select an MCP server to inspect:")?,
    };
    let server = catalog.resolve(&query).map_err(|e| eyre!("{e}"))?;
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
    log::data(&format!("Transport:   {}", transport_label(server)));
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

fn run_alias(args: AliasArgs, json_output: bool) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;

    match args.compatibility {
        Some(AliasCompatibilityCommand::Add(add)) => {
            catalog.add_alias(&catalog.resolve(&add.name)?.id.clone(), &add.alias)?;
            catalog.save()?;
            return render_json_or_text(
                json_output,
                json!({ "action": "add", "id": add.name, "alias": add.alias }),
                "Alias added.".to_string(),
            );
        }
        Some(AliasCompatibilityCommand::Remove(remove)) => {
            catalog.remove_alias(&remove.alias)?;
            catalog.save()?;
            return render_json_or_text(
                json_output,
                json!({ "action": "remove", "alias": remove.alias }),
                format!("Alias `{}` removed.", remove.alias),
            );
        }
        None => {}
    }

    let name = match args.name {
        Some(name) => name,
        None => prompt_for_server_query(&catalog, "Choose the MCP server to alias:")?,
    };
    let alias = match args.alias {
        Some(alias) => alias,
        None => Text::new("Alias to add:").prompt()?,
    };

    let server_id = catalog.resolve(&name)?.id.clone();
    catalog.add_alias(&server_id, &alias)?;
    catalog.save()?;

    render_json_or_text(
        json_output,
        json!({ "action": "add", "id": server_id, "alias": alias }),
        format!("Alias `{alias}` added to `{server_id}`."),
    )
}

fn run_remove(query: Option<&str>, json_output: bool) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;
    let query = match query {
        Some(query) => query.to_string(),
        None => prompt_for_server_query(&catalog, "Choose an MCP server or alias to remove:")?,
    };

    if catalog.get_server(&query).is_none()
        && let Some(owner) = catalog.find_alias_owner(&query)
    {
        let owner_id = owner.id.clone();
        let remaining_aliases: Vec<String> = owner
            .aliases
            .iter()
            .filter(|a| a.as_str() != query)
            .cloned()
            .collect();
        catalog.remove_alias(&query)?;
        catalog.save()?;

        if !json_output && !remaining_aliases.is_empty() {
            log::data(&format!(
                "Owner `{owner_id}` still has aliases: {}",
                remaining_aliases.join(", ")
            ));
        }

        return render_json_or_text(
            json_output,
            json!({ "removed_alias": query, "owner": owner_id, "remaining_aliases": remaining_aliases }),
            format!("Removed alias `{query}` from `{owner_id}`."),
        );
    }

    let server_id = catalog.resolve(&query)?.id.clone();
    let aliases = catalog
        .get_server(&server_id)
        .map(|server| server.aliases.clone())
        .unwrap_or_default();
    if !json_output {
        confirm_remove_server(&server_id, &aliases)?;
    }
    catalog.remove_server(&server_id)?;
    catalog.save()?;

    // Cascade: remove server from user defaults
    let mut defaults_cleaned = Vec::new();
    let user_defaults = load_user_defaults()?;
    if user_defaults.defaults.contains(&server_id) {
        let cleaned: Vec<String> = user_defaults
            .defaults
            .into_iter()
            .filter(|id| id != &server_id)
            .collect();
        defaults::set_user_defaults(cleaned)?;
        defaults_cleaned.push("user");
    }

    // Cascade: remove server from repo defaults (if in a repo)
    if let Some(repo_root) = current_repo_root()?
        && let Some(repo_defaults) = load_repo_defaults(&repo_root)?
        && repo_defaults.defaults.contains(&server_id)
    {
        let cleaned: Vec<String> = repo_defaults
            .defaults
            .into_iter()
            .filter(|id| id != &server_id)
            .collect();
        defaults::set_repo_defaults(&repo_root, cleaned)?;
        defaults_cleaned.push("repo");
    }

    if !json_output && !defaults_cleaned.is_empty() {
        log::data(&format!(
            "Also removed from {} defaults.",
            defaults_cleaned.join(" and ")
        ));
    }

    render_json_or_text(
        json_output,
        json!({ "removed_server": server_id, "aliases_removed": aliases, "defaults_cleaned": defaults_cleaned }),
        format!("Removed MCP server `{server_id}`."),
    )
}

fn run_check(json_output: bool) -> Result<()> {
    let catalog = McpCatalogStore::load()?;
    let state = McpProviderStateStore::load()?;
    let report = validate_state(&catalog, &state, current_repo_root()?.as_deref())?;

    if json_output {
        log::data(&serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if report.issues.is_empty() {
        log::data("MCP configuration is valid.");
        return Ok(());
    }

    for issue in &report.issues {
        let label = match issue.severity {
            ValidationSeverity::Error => "ERROR",
            ValidationSeverity::Warning => "WARN",
        };
        log::data(&format!("[{label}] {}: {}", issue.code, issue.message));
    }

    Ok(())
}

fn run_sync(_args: SyncArgs, json_output: bool) -> Result<()> {
    let report = bootstrap_mcp_state(current_repo_root()?.as_deref())?;

    if json_output {
        log::data(&serde_json::to_string_pretty(&json!({
            "imported": report.imported,
            "merged": report.merged,
            "conflicts": report.conflicts,
            "skipped": report.skipped,
            "errors": report.errors,
        }))?);
    } else {
        log::data("MCP catalog refreshed from native provider configs.");
        render_import_report(&report);
    }

    Ok(())
}

fn run_export(args: ExportArgs, json_output: bool) -> Result<()> {
    let provider = Provider::fuzzy_match_cli_name(&args.provider)
        .ok_or_else(|| eyre!("unknown provider: {}", args.provider))?;
    let catalog = McpCatalogStore::load()?;
    let mut state = McpProviderStateStore::load()?;

    let scope = match args.scope.as_str() {
        "repo" => {
            Scope::Repo(current_repo_root()?.ok_or_else(|| eyre!("failed to resolve repo root"))?)
        }
        "user" => Scope::User,
        other => return Err(eyre!("unknown scope `{other}`; expected user or repo")),
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
        log::data(&format!("Export applied to {}.", provider.as_slug()));
    } else {
        log::data(&format!(
            "Export dry run for {} (use --apply to write).",
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

fn bootstrap_mcp_state(repo_root: Option<&Path>) -> Result<ImportReport> {
    let mut catalog = McpCatalogStore::load()?;
    let mut state = McpProviderStateStore::load()?;
    let mut importer = McpImporter::new(&mut catalog, &mut state);
    let report = importer.import_all(repo_root);
    catalog.save()?;
    state.save()?;

    if !defaults_path().exists() {
        save_user_defaults(&McpDefaults::default())?;
    }
    if let Some(repo_root) = repo_root
        && !repo_defaults_path(repo_root).exists()
    {
        save_repo_defaults(repo_root, &McpDefaults::default())?;
    }

    Ok(report)
}

fn prompt_for_defaults(
    catalog: &McpCatalogStore,
    prompt: &str,
    current: &[String],
) -> Result<Vec<String>> {
    let options: Vec<String> = catalog
        .list_servers()
        .iter()
        .map(|server| server_label(server))
        .collect();
    let lookup: HashMap<String, String> = catalog
        .list_servers()
        .iter()
        .map(|server| (server_label(server), server.id.clone()))
        .collect();

    let preselected: Vec<usize> = if current.is_empty() {
        Vec::new()
    } else {
        options
            .iter()
            .enumerate()
            .filter_map(|(index, label)| {
                lookup
                    .get(label)
                    .filter(|id| current.contains(id))
                    .map(|_| index)
            })
            .collect()
    };

    let mut multi = MultiSelect::new(prompt, options);
    if !preselected.is_empty() {
        multi = multi.with_default(&preselected);
    }
    let selected = multi.prompt()?;

    Ok(selected
        .iter()
        .filter_map(|label| lookup.get(label).cloned())
        .collect())
}

fn prompt_for_server_query(catalog: &McpCatalogStore, prompt: &str) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        return Err(eyre!("missing MCP server argument"));
    }

    let options: Vec<String> = catalog
        .list_servers()
        .iter()
        .map(|server| server_label(server))
        .collect();
    let selection = Select::new(prompt, options).prompt()?;
    Ok(selection
        .split(' ')
        .next()
        .unwrap_or(selection.as_str())
        .to_string())
}

fn prompt_for_local_server() -> Result<McpServer> {
    let name = Text::new("Name (leave empty to auto-derive):").prompt()?;
    let command = Text::new("Command:").prompt()?;
    let args = Text::new("Arguments (space-separated, optional):").prompt()?;
    let env = prompt_for_env_map()?;

    let mut server = McpServer {
        id: String::new(),
        aliases: Vec::new(),
        transport: McpTransport::Stdio,
        command: Some(command),
        args: split_args(&args),
        cwd: None,
        env,
        url: None,
        headers: HashMap::new(),
        enabled_tools: Vec::new(),
        disabled_tools: Vec::new(),
        required: false,
        metadata: McpServerMetadata {
            description: None,
            created_from: Some("manual:user".into()),
            fingerprint: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        provider_overrides: HashMap::new(),
    };
    server.id = derive_server_id(&server, (!name.trim().is_empty()).then_some(name.trim()));
    server.metadata.fingerprint = server.fingerprint();
    Ok(server)
}

fn prompt_for_remote_server() -> Result<McpServer> {
    let name = Text::new("Name (leave empty to auto-derive):").prompt()?;
    let url = Text::new("URL:").prompt()?;
    let env = prompt_for_env_map()?;

    let mut server = McpServer {
        id: String::new(),
        aliases: Vec::new(),
        transport: McpTransport::Http,
        command: None,
        args: Vec::new(),
        cwd: None,
        env,
        url: Some(url),
        headers: HashMap::new(),
        enabled_tools: Vec::new(),
        disabled_tools: Vec::new(),
        required: false,
        metadata: McpServerMetadata {
            description: None,
            created_from: Some("manual:user".into()),
            fingerprint: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        provider_overrides: HashMap::new(),
    };
    server.id = derive_server_id(&server, (!name.trim().is_empty()).then_some(name.trim()));
    server.metadata.fingerprint = server.fingerprint();
    Ok(server)
}

fn prompt_for_env_map() -> Result<HashMap<String, String>> {
    let add_env = Confirm::new("Add environment variables?")
        .with_default(false)
        .prompt()?;
    if !add_env {
        return Ok(HashMap::new());
    }

    let mut env = HashMap::new();
    loop {
        let entry = Text::new("Environment entry (KEY=VALUE, leave empty to finish):").prompt()?;
        if entry.trim().is_empty() {
            break;
        }
        let Some((key, value)) = entry.split_once('=') else {
            return Err(eyre!("environment entries must look like KEY=VALUE"));
        };
        env.insert(key.trim().to_string(), value.to_string());
    }
    Ok(env)
}

fn insert_manual_server(catalog: &mut McpCatalogStore, mut server: McpServer) -> Result<String> {
    let base_id = server.id.clone();
    let fingerprint = server.metadata.fingerprint.clone();

    if let Some(existing) = catalog.find_by_fingerprint(&fingerprint) {
        return Ok(existing.id.clone());
    }

    let mut attempt = 1usize;
    while let Some(existing) = catalog.get_server(&server.id) {
        if existing.metadata.fingerprint == fingerprint {
            return Ok(existing.id.clone());
        }
        attempt += 1;
        server.id = format!("{base_id}-{attempt}");
    }

    let added = server.id.clone();
    catalog.add_server(server);
    Ok(added)
}

fn render_init_summary(
    catalog: &McpCatalogStore,
    repo_defaults: &[String],
    user_defaults: &[String],
    _repo_root: &Path,
) {
    log::data("MCP initialization complete.");
    let _ = run_list(ListArgs::default(), false);
    log::data("");
    log::data(&format!(
        "User defaults: {}",
        if user_defaults.is_empty() {
            "none".to_string()
        } else {
            user_defaults.join(", ")
        }
    ));
    log::data(&format!(
        "Repo defaults: {}",
        if repo_defaults.is_empty() {
            "none".to_string()
        } else {
            repo_defaults.join(", ")
        }
    ));
    if !catalog.list_servers().is_empty() {
        log::data("Use `#tag` in the initial wrapped prompt to opt into MCP servers.");
        log::data("Use `claudine mcp alias <server> <alias>` to add shorter names.");
        log::data("Use `claudine mcp config <server>` to inspect a catalog entry.");
    }
}

fn render_reentry_help(repo_root: Option<&Path>) {
    let user_path = defaults_path();
    let repo_path = repo_root.map(repo_defaults_path);
    let user_link = format!(
        r#"<a href="file://{}">{}</a>"#,
        user_path.display(),
        user_path.display()
    );
    log::data("MCP mode is already initialized.");
    log::data(&Prose::new(format!("User defaults: {user_link}")).render(&crate::log::terminal()));
    if let Some(repo_path) = repo_path {
        let repo_link = format!(
            r#"<a href="file://{}">{}</a>"#,
            repo_path.display(),
            repo_path.display()
        );
        log::data(
            &Prose::new(format!("Repo defaults: {repo_link}")).render(&crate::log::terminal()),
        );
    }
    log::data("Management commands:");
    log::data("  claudine mcp");
    log::data("  claudine mcp config <server>");
    log::data("  claudine mcp alias <server> <alias>");
    log::data("  claudine mcp remove <server-or-alias>");
    log::data("  claudine mcp export <provider> --apply");
}

fn render_import_report(report: &ImportReport) {
    if !report.imported.is_empty() {
        log::data(&format!("  Imported: {}", report.imported.len()));
    }
    if !report.merged.is_empty() {
        log::data(&format!("  Merged: {}", report.merged.len()));
    }
    if !report.conflicts.is_empty() {
        log::data(&format!("  Conflicts: {}", report.conflicts.len()));
    }
    if !report.skipped.is_empty() {
        log::data(&format!("  Skipped: {}", report.skipped.len()));
    }
    if !report.errors.is_empty() {
        log::data(&format!("  Errors: {}", report.errors.len()));
    }
}

fn render_json_or_text(json_output: bool, value: Value, message: String) -> Result<()> {
    if json_output {
        log::data(&serde_json::to_string_pretty(&value)?);
    } else {
        log::data(&message);
    }
    Ok(())
}

fn confirm_remove_server(id: &str, aliases: &[String]) -> Result<()> {
    let alias_note = if aliases.is_empty() {
        String::new()
    } else {
        format!(" This also removes aliases: {}.", aliases.join(", "))
    };
    if !Confirm::new(&format!("Remove MCP server `{id}`?{alias_note}"))
        .with_default(false)
        .prompt()?
    {
        return Err(eyre!("aborted removal of `{id}`"));
    }
    Ok(())
}

fn filtered_servers<'a>(catalog: &'a McpCatalogStore, args: &ListArgs) -> Vec<&'a McpServer> {
    let filter = args.filter.as_ref().map(|value| value.to_ascii_lowercase());
    let alias_filter = args.alias.as_ref().map(|value| value.to_ascii_lowercase());
    catalog
        .list_servers()
        .into_iter()
        .filter(|server| {
            let id_match = filter
                .as_ref()
                .is_none_or(|needle| server.id.to_ascii_lowercase().contains(needle));
            let alias_match = alias_filter.as_ref().is_none_or(|needle| {
                server
                    .aliases
                    .iter()
                    .any(|alias| alias.to_ascii_lowercase().contains(needle))
            });
            id_match && alias_match
        })
        .collect()
}

fn styled_server_name(
    server: &McpServer,
    user_defaults: &[String],
    repo_defaults: &[String],
    term: &Terminal,
) -> String {
    let mut label = server.id.clone();
    if user_defaults.contains(&server.id) {
        label.push_str(" [user]");
    }
    if repo_defaults.contains(&server.id) {
        label.push_str(" [repo]");
    }

    if user_defaults.contains(&server.id) || repo_defaults.contains(&server.id) {
        let _ = term;
        Prose::new(format!("<bold>{label}</bold>")).render(&crate::log::optimistic_terminal(None))
    } else {
        label
    }
}

fn transport_label(server: &McpServer) -> &'static str {
    match server.transport {
        McpTransport::Stdio => "local",
        McpTransport::Http => "http",
        McpTransport::Sse => "sse",
    }
}

fn auth_summary(server: &McpServer) -> String {
    if server
        .provider_override_object("opencode")
        .and_then(|value| value.get("oauth"))
        .is_some()
    {
        return "oauth".into();
    }
    if !server.headers.is_empty() {
        return "headers".into();
    }
    if server.required {
        return "required".into();
    }
    "—".into()
}

fn server_label(server: &McpServer) -> String {
    if server.aliases.is_empty() {
        server.id.clone()
    } else {
        format!("{} ({})", server.id, server.aliases.join(", "))
    }
}

fn split_args(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_string).collect()
}

fn current_repo_root() -> Result<Option<PathBuf>> {
    let cwd = std::env::current_dir()?;
    if sniff::filesystem::detect_git(&cwd, false, 1)
        .ok()
        .flatten()
        .is_some()
        || sniff::filesystem::detect_repo(&cwd)
            .ok()
            .flatten()
            .is_some()
    {
        Ok(Some(resolve_repo_root(&cwd)))
    } else {
        Ok(None)
    }
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

fn base_table(columns: Vec<TableColumn>) -> Table {
    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);
    let _ = Alignment::Right;
    table
}
