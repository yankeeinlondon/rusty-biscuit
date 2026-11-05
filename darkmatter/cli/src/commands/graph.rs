//! `md graph` subcommand implementation.

use crate::io::open_argument;
use crate::request::MdRequest;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ComposeSource};
use darkmatter::markdown::reference::file_tree::FileTree;
use std::path::Path;
use tracing::instrument;

#[instrument(skip_all)]
pub(crate) fn run_graph(
    input: &Path,
    follow: bool,
    validate: bool,
    json: bool,
    verbosity: u8,
    request: &MdRequest,
) -> Result<()> {
    let opened = open_argument(input, request)?;
    // The graph resolves through the document's own context: one derived from
    // the launch context, or built at the document for a document in another
    // repository (a launch-anchored request rejects such a document).
    let context = opened.document_context(request)?;
    let resolved = opened.path();
    let md = Markdown::try_from(resolved)
        .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
    let graph_request = ComposeRequest::with_context(
        ComposeOptions::for_document(request.launch_dir(), &md),
        context,
    )
    .wrap_err("Failed to prepare the reference graph request")?;
    let mut tree = FileTree::from_markdown(md, &graph_request);

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

    if validate && let Some(report) = tree.validation_report() {
        use darkmatter::markdown::reference::validate::ValidationReportView;

        let view = ValidationReportView::new(report.clone())
            .with_root_source(ComposeSource::File(resolved.to_path_buf()))
            .with_launch_dir(request.launch_dir());
        println!("\n{}", super::validate::terminal_report(&view, &term, verbosity)?);

        // Exit code 2 for validation errors
        if !report.is_valid() {
            std::process::exit(2);
        }
    }

    Ok(())
}
