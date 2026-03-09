use clap::{Args, Subcommand};
use claudine::events::Provider;
use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::defaults;
use claudine::mcp::export::McpExporter;
use claudine::mcp::import::McpImporter;
use claudine::mcp::state::{McpProviderStateStore, Scope};
use color_eyre::eyre::{Result, eyre};

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
        Some(McpCommand::Default(default_args)) => run_default(default_args),
        Some(McpCommand::Alias(alias_args)) => run_alias(alias_args),
        Some(McpCommand::Remove(remove_args)) => run_remove(&remove_args.id),
        Some(McpCommand::Sync(sync_args)) => run_sync(sync_args),
    }
}

/// `claudine mcp` — list all catalog entries.
fn run_list(json_output: bool) -> Result<()> {
    let catalog = McpCatalogStore::load()?;
    let servers = catalog.list_servers();

    if json_output {
        let entries: Vec<serde_json::Value> = servers
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "aliases": s.aliases,
                    "transport": s.transport,
                    "command": s.command,
                    "url": s.url,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if servers.is_empty() {
        println!("No MCP servers in catalog.");
        println!("Run `claudine mcp init` to import from native provider configs.");
        return Ok(());
    }

    println!("{:<25} {:<10} {:<20} ALIASES", "ID", "TRANSPORT", "COMMAND/URL");
    println!("{}", "-".repeat(75));
    for server in servers {
        let transport = format!("{:?}", server.transport).to_lowercase();
        let target = server
            .command
            .as_deref()
            .or(server.url.as_deref())
            .unwrap_or("-");
        let aliases = if server.aliases.is_empty() {
            String::new()
        } else {
            server.aliases.join(", ")
        };
        println!("{:<25} {:<10} {:<20} {}", server.id, transport, target, aliases);
    }

    Ok(())
}

/// `claudine mcp init` — scan and import.
fn run_init(json_output: bool) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;
    let mut state = McpProviderStateStore::load()?;

    let cwd = std::env::current_dir().ok();
    let mut importer = McpImporter::new(&mut catalog, &mut state);
    let report = importer.import_all(cwd.as_deref());

    catalog.save()?;
    state.save()?;

    if json_output {
        let summary = serde_json::json!({
            "imported": report.imported.len(),
            "merged": report.merged.len(),
            "conflicts": report.conflicts.len(),
            "skipped": report.skipped.len(),
            "errors": report.errors.len(),
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    println!("MCP import complete:");
    if !report.imported.is_empty() {
        println!("  Imported: {}", report.imported.len());
        for entry in &report.imported {
            println!("    + {} (from {:?}:{})", entry.catalog_id, entry.provider, entry.native_name);
        }
    }
    if !report.merged.is_empty() {
        println!("  Merged:   {}", report.merged.len());
        for entry in &report.merged {
            let alias_note = entry
                .alias_added
                .as_ref()
                .map(|a| format!(" (alias added: {a})"))
                .unwrap_or_default();
            println!("    ~ {} (from {:?}){}", entry.catalog_id, entry.provider, alias_note);
        }
    }
    if !report.conflicts.is_empty() {
        println!("  Conflicts: {}", report.conflicts.len());
        for entry in &report.conflicts {
            println!("    ! {} → {} (from {:?})", entry.name, entry.new_catalog_id, entry.provider);
        }
    }
    if !report.errors.is_empty() {
        println!("  Errors: {}", report.errors.len());
        for entry in &report.errors {
            println!("    ✗ {:?}:{} — {}", entry.provider, entry.native_name, entry.reason);
        }
    }
    if report.imported.is_empty() && report.merged.is_empty() && report.conflicts.is_empty() {
        println!("  No MCP servers found in native provider configs.");
    }

    Ok(())
}

/// `claudine mcp show <id>` — display server details.
fn run_show(id: &str, json_output: bool) -> Result<()> {
    let catalog = McpCatalogStore::load()?;
    let server = catalog.resolve(id).map_err(|e| eyre!("{e}"))?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(server)?);
        return Ok(());
    }

    println!("ID:        {}", server.id);
    if !server.aliases.is_empty() {
        println!("Aliases:   {}", server.aliases.join(", "));
    }
    println!("Transport: {:?}", server.transport);
    if let Some(ref cmd) = server.command {
        println!("Command:   {}", cmd);
    }
    if !server.args.is_empty() {
        println!("Args:      {}", server.args.join(" "));
    }
    if let Some(ref url) = server.url {
        println!("URL:       {}", url);
    }
    if !server.env.is_empty() {
        println!("Env:");
        for k in server.env.keys() {
            println!("  {}=<redacted>", k);
        }
    }
    if !server.headers.is_empty() {
        println!("Headers:");
        for k in server.headers.keys() {
            println!("  {}: <redacted>", k);
        }
    }
    if let Some(ref desc) = server.metadata.description {
        println!("Description: {}", desc);
    }
    if let Some(ref from) = server.metadata.created_from {
        println!("Created from: {}", from);
    }
    println!("Fingerprint: {}", server.metadata.fingerprint);

    Ok(())
}

/// `claudine mcp default [ids...]`
fn run_default(args: DefaultArgs) -> Result<()> {
    if args.repo {
        let cwd = std::env::current_dir()?;
        defaults::set_repo_defaults(&cwd, args.ids.clone())?;
        println!("Repo defaults set: {}", args.ids.join(", "));
    } else {
        defaults::set_user_defaults(args.ids.clone())?;
        println!("User defaults set: {}", args.ids.join(", "));
    }
    Ok(())
}

/// `claudine mcp alias add|remove`
fn run_alias(args: AliasArgs) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;

    match args.command {
        AliasCommand::Add(add) => {
            catalog.add_alias(&add.id, &add.alias)?;
            catalog.save()?;
            println!("Alias '{}' added to server '{}'", add.alias, add.id);
        }
        AliasCommand::Remove(remove) => {
            catalog.remove_alias(&remove.alias)?;
            catalog.save()?;
            println!("Alias '{}' removed", remove.alias);
        }
    }
    Ok(())
}

/// `claudine mcp remove <id>`
fn run_remove(id: &str) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;
    let server = catalog.remove_server(id)?;
    catalog.save()?;
    println!("Removed server '{}' from catalog", server.id);
    Ok(())
}

/// `claudine mcp sync <provider>`
fn run_sync(args: SyncExportArgs) -> Result<()> {
    let provider = Provider::fuzzy_match_cli_name(&args.provider)
        .ok_or_else(|| eyre!("unknown provider: {}", args.provider))?;

    let catalog = McpCatalogStore::load()?;
    let state = McpProviderStateStore::load()?;

    let scope = match args.scope.as_str() {
        "repo" => {
            let cwd = std::env::current_dir()?;
            Scope::Repo(cwd)
        }
        _ => Scope::User,
    };

    // Get servers from effective defaults
    let default_ids = defaults::effective_defaults(
        match &scope {
            Scope::Repo(p) => Some(p.as_path()),
            Scope::User => None,
        },
        &catalog,
    )?;

    let servers: Vec<_> = default_ids
        .iter()
        .filter_map(|id| catalog.resolve(id).ok().cloned())
        .collect();

    let exporter = McpExporter::new(&catalog, &state);
    let report = exporter.sync_provider(provider, &scope, &servers, args.apply)?;

    if args.apply {
        println!("Sync applied to {}:", provider.as_slug());
    } else {
        println!("Sync dry run for {} (use --apply to write):", provider.as_slug());
    }
    if !report.written.is_empty() {
        println!("  Write: {}", report.written.join(", "));
    }
    if !report.removed.is_empty() {
        println!("  Remove: {}", report.removed.join(", "));
    }
    if !report.preserved.is_empty() {
        println!("  Preserve: {}", report.preserved.join(", "));
    }

    Ok(())
}
