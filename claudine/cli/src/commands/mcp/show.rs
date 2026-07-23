use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::state::McpProviderStateStore;
use claudine::provider::PROVIDERS_DISPLAY_ORDER;
use color_eyre::eyre::Result;
use serde_json::{Value, json};

use crate::log;

use super::{prompt_for_server_query, redacted_keys, transport_label};

pub(super) fn run_config(query: Option<&str>, json_output: bool) -> Result<()> {
    let catalog = McpCatalogStore::load()?;
    let state = McpProviderStateStore::load()?;
    let query = match query {
        Some(query) => query.to_string(),
        None => prompt_for_server_query(&catalog, "Select an MCP server to inspect:")?,
    };
    let server = catalog.resolve(&query)?;
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
