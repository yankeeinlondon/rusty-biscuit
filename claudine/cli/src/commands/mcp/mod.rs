//! See [`claudine/docs/mcp-support.md`](../../../docs/mcp-support.md) for the
//! full MCP catalog/state model and provider rollout.

use std::io::IsTerminal;
use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use claudine::linking::resolve_repo_root;
use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::types::McpServer;
use claudine::provider::Provider;
use color_eyre::eyre::{Result, eyre};
use inquire::Select;
use serde_json::Value;

use crate::log;
use crate::provider_values::provider_value_parser;

mod add;
mod alias;
mod check;
mod default;
mod export;
mod init;
mod list;
mod remove;
mod show;
mod sync;

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
    #[arg(value_parser = provider_value_parser())]
    pub provider: Provider,

    /// Scope: user or repo.
    #[arg(long, value_enum, default_value_t = ExportScopeArg::User)]
    pub scope: ExportScopeArg,

    /// Actually apply changes (default: dry run).
    #[arg(long)]
    pub apply: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExportScopeArg {
    User,
    Repo,
}

pub fn run(args: McpArgs) -> Result<()> {
    let json_output = args.json;

    match args.command {
        None => list::run_list(ListArgs::default(), json_output),
        Some(McpCommand::Init) => init::run_init(json_output),
        Some(McpCommand::List(list_args)) => list::run_list(list_args, json_output),
        Some(McpCommand::Add(add_args)) => add::run_add(add_args, json_output),
        Some(McpCommand::Config(config_args)) | Some(McpCommand::Show(config_args)) => {
            show::run_config(config_args.query.as_deref(), json_output)
        }
        Some(McpCommand::Default(default_args)) => default::run_default(default_args, json_output),
        Some(McpCommand::Alias(alias_args)) => alias::run_alias(alias_args, json_output),
        Some(McpCommand::Remove(remove_args)) => {
            remove::run_remove(remove_args.query.as_deref(), json_output)
        }
        Some(McpCommand::Check) => check::run_check(json_output),
        Some(McpCommand::Sync(sync_args)) => sync::run_sync(sync_args, json_output),
        Some(McpCommand::Export(export_args)) => export::run_export(export_args, json_output),
    }
}

pub(super) fn render_json_or_text(json_output: bool, value: Value, message: String) -> Result<()> {
    if json_output {
        log::data(&serde_json::to_string_pretty(&value)?);
    } else {
        log::data(&message);
    }
    Ok(())
}

pub(super) fn current_repo_root() -> Result<Option<PathBuf>> {
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

pub(super) fn server_label(server: &McpServer) -> String {
    if server.aliases.is_empty() {
        server.id.clone()
    } else {
        format!("{} ({})", server.id, server.aliases.join(", "))
    }
}

pub(super) fn prompt_for_server_query(catalog: &McpCatalogStore, prompt: &str) -> Result<String> {
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

pub(super) fn redacted_keys(mut keys: Vec<String>) -> String {
    keys.sort();
    keys.into_iter()
        .map(|key| format!("{key}=<redacted>"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn transport_label(server: &McpServer) -> &'static str {
    use claudine::mcp::types::McpTransport;
    match server.transport {
        McpTransport::Stdio => "local",
        McpTransport::Http => "http",
        McpTransport::Sse => "sse",
    }
}

pub(super) fn auth_summary(server: &McpServer) -> String {
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
