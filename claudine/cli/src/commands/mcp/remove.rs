use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::defaults::{self, load_repo_defaults, load_user_defaults};
use color_eyre::eyre::{Result, eyre};
use inquire::Confirm;
use serde_json::json;

use crate::log;

use super::{current_repo_root, prompt_for_server_query, render_json_or_text};

pub(super) fn run_remove(query: Option<&str>, json_output: bool) -> Result<()> {
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
