//! `md validate` subcommand implementation.

use crate::args::{GraphFormat, ValidateOutputFormat, ValidateTarget};
use crate::io::open_argument;
use crate::request::MdRequest;
use biscuit_terminal::output::{OutputTarget, render_output};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ComposeSource};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::{
    ReferenceReportVisibility, ReferenceValidationOptions, ValidationReportView,
};
use renderable::output::{CommandOutput, OutputOptions, Width};
use tracing::{info, instrument};

#[instrument(skip_all, fields(command = "validate"))]
pub(crate) fn run_validate(target: ValidateTarget, verbosity: u8, request: &MdRequest) -> Result<()> {
    info!("starting reference validation");

    match target {
        ValidateTarget::Refs {
            input,
            remote,
            fragments,
            timeout,
            fail_fast,
            format,
            show_all,
            graph,
        } => {
            let opened = open_argument(&input, request)?;
            let md = Markdown::try_from(opened.path())
                .wrap_err_with(|| format!("Failed to load {}", input.display()))?;
            let context = opened.document_context(request)?;
            let validation_request = ComposeRequest::with_context(
                ComposeOptions::for_document(request.launch_dir(), &md),
                context,
            )
            .wrap_err("Failed to prepare the validation request")?;

            // If --graph requested, print graph and exit
            if let Some(graph_format) = graph {
                let graph_options = ReferenceGraphOptions::with_compose(&validation_request);
                let ref_graph = md
                    .reference_graph(graph_options)
                    .wrap_err("Failed to build reference graph")?;

                match graph_format {
                    GraphFormat::Mermaid => println!("{}", ref_graph.to_mermaid()),
                    GraphFormat::Dot => println!("{}", ref_graph.to_dot()),
                }
                return Ok(());
            }

            let options = ReferenceValidationOptions {
                graph: ReferenceGraphOptions::with_compose(&validation_request),
                validate_remote: remote,
                remote_timeout: std::time::Duration::from_secs(timeout),
                validate_fragments: fragments,
                fail_fast,
            };

            let report = md
                .validate_references(options)
                .wrap_err("Reference validation failed")?;

            let is_valid = report.is_valid();
            let error_count = report.error_count();
            let mut view = ValidationReportView::new(report)
                .with_root_source(ComposeSource::File(opened.path().to_path_buf()))
                .with_launch_dir(request.launch_dir())
                .with_visibility(ReferenceReportVisibility::All);
            if show_all {
                view = view.with_reference_listing();
            }

            match format {
                ValidateOutputFormat::Text => {
                    println!("{}", terminal_report(&view, &Terminal::default(), verbosity)?);
                }
                ValidateOutputFormat::Json => {
                    // The data form is the library `ReferenceValidationReport`,
                    // the same serde shape as `md graph --validate --json`'s
                    // `validation` block (plus `references` under `--show-all`),
                    // so the two CLI surfaces cannot drift. The shape is pinned
                    // by the `2026-06-17-cli-atheist` baseline fixtures.
                    println!("{}", serde_json::to_string_pretty(&view.as_data())?);
                }
            }

            if is_valid {
                Ok(())
            } else {
                Err(eyre!("{error_count} error(s) found"))
            }
        }
    }
}

/// `view`'s `terminal` output for `term`, which degrades to `plain` off a
/// color terminal.
///
/// Full `--output`/`--width` selection is not wired to these commands yet.
pub(crate) fn terminal_report(view: &ValidationReportView, term: &Terminal, verbosity: u8) -> Result<String> {
    let width = term.detected_columns().and_then(Width::columns).unwrap_or(Width::Unlimited);
    let options = OutputOptions::new(width, term.output_profile()).with_verbosity(verbosity);
    let rendered = render_output(view, OutputTarget::Terminal, &options)
        .wrap_err("Failed to render the reference report")?;
    Ok(rendered.text)
}
