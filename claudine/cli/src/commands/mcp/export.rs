use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::defaults;
use claudine::mcp::export::McpExporter;
use claudine::mcp::state::{McpProviderStateStore, Scope};
use color_eyre::eyre::{Result, eyre};
use serde_json::json;

use crate::log;

use super::{ExportArgs, ExportScopeArg, current_repo_root};

pub(super) fn run_export(args: ExportArgs, json_output: bool) -> Result<()> {
    let catalog = McpCatalogStore::load()?;
    let mut state = McpProviderStateStore::load()?;

    let scope = match args.scope {
        ExportScopeArg::Repo => {
            Scope::Repo(current_repo_root()?.ok_or_else(|| eyre!("failed to resolve repo root"))?)
        }
        ExportScopeArg::User => Scope::User,
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
    let report = exporter.sync_provider(args.provider, &scope, &servers, args.apply)?;
    if args.apply {
        state.save()?;
    }

    if json_output {
        log::data(&serde_json::to_string_pretty(&json!({
            "provider": args.provider.as_slug(),
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
        log::data(&format!("Export applied to {}.", args.provider.as_slug()));
    } else {
        log::data(&format!(
            "Export dry run for {} (use --apply to write).",
            args.provider.as_slug()
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

fn scope_name(scope: &Scope) -> &'static str {
    match scope {
        Scope::User => "user",
        Scope::Repo(_) => "repo",
    }
}
