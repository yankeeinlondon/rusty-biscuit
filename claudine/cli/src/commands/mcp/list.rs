use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::terminal::Terminal;
use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::defaults::{self, load_repo_defaults, load_user_defaults};
use claudine::mcp::state::McpProviderStateStore;
use claudine::mcp::types::McpServer;
use claudine::provider::PROVIDERS_DISPLAY_ORDER;
use color_eyre::eyre::Result;
use serde_json::{Value, json};

use crate::log;
use crate::table_utils::base_table;

use super::{
    ListArgs, auth_summary, current_repo_root, redacted_keys, transport_label,
};

pub(super) fn run_list(args: ListArgs, json_output: bool) -> Result<()> {
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
