//! `md graph` subcommand implementation.

use crate::args::Cli;
use crate::commands::validate::report_failure_diagnostics;
use crate::io::open_argument;
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::request::MdRequest;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::output::{OutputTarget, WidthRequest, render_output};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ComposeSource};
use darkmatter::markdown::output::command::{CommandValue, code_document};
use darkmatter::markdown::reference::file_tree::FileTree;
use darkmatter::markdown::reference::validate::ValidationReportView;
use renderable::{CommandOutput as _, OutputOptions, TerminalProfile, Width};
use serde_json::Value;
use std::path::Path;
use tracing::instrument;

#[instrument(skip_all)]
pub(crate) fn run_graph(input: &Path, follow: bool, validate: bool, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let mut run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Object)?;
    let result = graph_value(&mut run, input, follow, validate, request);
    run.finish(result)
}

fn graph_value(
    run: &mut CommandRun,
    input: &Path,
    follow: bool,
    validate: bool,
    request: &MdRequest,
) -> Result<Completed<CommandValue<Value>>> {
    let opened = open_argument(input, request)?;
    run.set_document(opened.path());
    // The graph resolves through the document's own context: one derived from
    // the launch context, or built at the document for a document in another
    // repository (a launch-anchored request rejects such a document).
    let context = opened.document_context(request)?;
    let resolved = opened.path();
    let md = Markdown::try_from(resolved)
        .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
    let graph_request = ComposeRequest::with_context(
        // Every document the graph prepares shares one data memo and read set.
        ComposeOptions::for_document(request.launch_dir(), &md).with_shared_data_runtime(),
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

    let report = validate.then(|| tree.validation_report().cloned()).flatten();
    let mut data = match tree.graph() {
        Some(graph) => serde_json::to_value(graph.view(follow))?,
        None => Value::Null,
    };
    if let Some(report) = &report {
        data["validation"] = serde_json::to_value(report)?;
    }

    let view = report.as_ref().map(|report| {
        ValidationReportView::new(report.clone())
            .with_root_source(ComposeSource::File(resolved.to_path_buf()))
            .with_launch_dir(request.launch_dir())
    });
    let failure = report.as_ref().filter(|report| !report.is_valid()).map(report_failure_diagnostics);

    let plain = OutputOptions::new(Width::Unlimited, TerminalProfile::plain());
    let plain_tree = tree.render(&Terminal::from_output_options(&plain));
    let mut prose = code_document(None, plain_tree.trim_end());
    if let Some(view) = &view
        && let Some(children) = prose.root.children_mut()
    {
        children.extend(view.as_prose().root.children().iter().cloned());
    }
    let value = CommandValue::new(data, prose).with_terminal(move |options| {
        let mut text = tree.render(&Terminal::from_output_options(options));
        if let Some(view) = &view {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push('\n');
            match render_output(view, OutputTarget::Terminal, options) {
                Ok(rendered) => text.push_str(&rendered.text),
                Err(error) => text.push_str(&error.to_string()),
            }
        }
        text
    });
    Ok(match failure {
        Some(diagnostics) => Completed::failure(value).with_diagnostics(diagnostics),
        None => Completed::success(value),
    })
}
