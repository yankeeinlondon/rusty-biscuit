//! `md graph` subcommand implementation.

use crate::io::resolve_file_path;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};
use darkmatter::markdown::reference::file_tree::FileTree;
use std::path::PathBuf;
use tracing::instrument;

#[instrument(skip_all)]
pub(crate) fn run_graph(input: &PathBuf, follow: bool, validate: bool, json: bool) -> Result<()> {
    let resolved = resolve_file_path(input)?;
    let mut tree = FileTree::new(&resolved).map_err(|e| eyre!("{e}"))?;

    if follow {
        tree = tree.follow_transclusions();
    }
    if validate {
        tree = tree.validate();
    }

    tree.ensure_built().map_err(|e| eyre!("{e}"))?;

    // ── JSON output ──────────────────────────────────────────────────
    if json {
        if let Some(graph) = tree.graph() {
            let mut root_json = serde_json::to_value(graph.view(follow))?;

            if validate && let Some(report) = tree.validation_report() {
                root_json["validation"] = serde_json::to_value(report)?;
            }

            println!("{}", serde_json::to_string_pretty(&root_json)?);
        }
        // JSON mode always exits 0
        return Ok(());
    }

    // ── Terminal tree output ─────────────────────────────────────────
    let term = Terminal::default();
    print!("{}", tree.display(&term));

    // Validation summary footer
    if validate && let Some(report) = tree.validation_report() {
        use darkmatter::markdown::reference::validate::ValidationReportView;

        let formatted = ValidationReportView::new(report.clone()).render(&term);
        if !formatted.is_empty() {
            println!("{formatted}");
        } else {
            let summary = format!(
                "{} references scanned, {} valid, 0 issues",
                report.references_scanned, report.references_valid,
            );
            println!("\n{}", Prose::new(summary).render(&term));
        }

        // Exit code 2 for validation errors
        if !report.is_valid() {
            std::process::exit(2);
        }
    }

    Ok(())
}
