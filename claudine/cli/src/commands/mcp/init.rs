use std::collections::HashMap;
use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::defaults::{load_user_defaults, save_repo_defaults, save_user_defaults};
use claudine::mcp::import::{ImportReport, McpImporter};
use claudine::mcp::state::McpProviderStateStore;
use claudine::mcp::types::{McpDefaults, defaults_path, repo_defaults_path};
use color_eyre::eyre::Result;
use inquire::MultiSelect;
use serde_json::json;

use crate::log;

use super::{ListArgs, current_repo_root, list};

pub(super) fn run_init(json_output: bool) -> Result<()> {
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

pub(super) fn bootstrap_mcp_state(repo_root: Option<&Path>) -> Result<ImportReport> {
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
        .map(|server| super::server_label(server))
        .collect();
    let lookup: HashMap<String, String> = catalog
        .list_servers()
        .iter()
        .map(|server| (super::server_label(server), server.id.clone()))
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

fn render_init_summary(
    catalog: &McpCatalogStore,
    repo_defaults: &[String],
    user_defaults: &[String],
    _repo_root: &Path,
) {
    log::data("MCP initialization complete.");
    let _ = list::run_list(ListArgs::default(), false);
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
    let user_label = biscuit_file::to_portable_string(&user_path);
    let user_link = crate::cli_utils::file_url(&user_path).map_or_else(
        || user_label.clone(),
        |href| format!(r#"<a href="{href}">{user_label}</a>"#),
    );
    log::data("MCP mode is already initialized.");
    log::data(&Prose::new(format!("User defaults: {user_link}")).render(&crate::log::terminal()));
    if let Some(repo_path) = repo_path {
        let repo_label = biscuit_file::to_portable_string(&repo_path);
        let repo_link = crate::cli_utils::file_url(&repo_path).map_or_else(
            || repo_label.clone(),
            |href| format!(r#"<a href="{href}">{repo_label}</a>"#),
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
