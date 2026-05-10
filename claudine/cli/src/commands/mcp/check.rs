use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::state::McpProviderStateStore;
use claudine::mcp::validation::{ValidationSeverity, validate_state};
use color_eyre::eyre::Result;

use crate::log;

use super::current_repo_root;

pub(super) fn run_check(json_output: bool) -> Result<()> {
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
