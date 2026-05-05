use claudine::mcp::import::ImportReport;
use color_eyre::eyre::Result;
use serde_json::json;

use crate::log;

use super::{SyncArgs, current_repo_root, init::bootstrap_mcp_state};

pub(super) fn run_sync(_args: SyncArgs, json_output: bool) -> Result<()> {
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
