//! `md validate` subcommand implementation.

use crate::args::{Cli, GraphFormat, ValidateTarget};
use crate::io::open_argument;
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::request::MdRequest;
use biscuit_terminal::output::WidthRequest;
use color_eyre::eyre::{Context, Result};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::Diagnostic;
use darkmatter::markdown::compose::report_diagnostics::reference_issue_diagnostic;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ComposeSource};
use darkmatter::markdown::output::command::{CommandValue, GraphNotation, GraphNotationData, code_document};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::{
    ReferenceReportVisibility, ReferenceSeverity, ReferenceValidationOptions, ReferenceValidationReport, ValidationReportView,
};
use tracing::{info, instrument};

/// The error issues of a failed `report`, as run diagnostics.
pub(crate) fn report_failure_diagnostics(report: &ReferenceValidationReport) -> Vec<Diagnostic> {
    report
        .issues
        .iter()
        .filter(|issue| issue.severity == ReferenceSeverity::Error)
        .map(reference_issue_diagnostic)
        .collect()
}

#[instrument(skip_all, fields(command = "validate"))]
pub(crate) fn run_validate(target: ValidateTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    info!("starting reference validation");

    let ValidateTarget::Refs { input, remote, fragments, timeout, fail_fast, output, show_all, graph } = target;
    let mut run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Object)?;

    let prepared = (|| {
        let opened = open_argument(&input, request)?;
        let md = Markdown::try_from(opened.path()).wrap_err_with(|| format!("Failed to load {}", input.display()))?;
        let context = opened.document_context(request)?;
        // Every document the graph prepares shares one data memo and read set.
        let options = ComposeOptions::for_document(request.launch_dir(), &md).with_shared_data_runtime();
        let validation_request = ComposeRequest::with_context(options, context)
            .wrap_err("Failed to prepare the validation request")?;
        let validation_request = crate::commands::compose::with_reference_check_slot_session(validation_request);
        Ok::<_, color_eyre::Report>((opened, md, validation_request))
    })();
    if let Ok((opened, ..)) = &prepared {
        run.set_document(opened.path());
    }

    // `--graph` prints the diagram alone, with no validation.
    if let Some(format) = graph {
        let result = prepared.and_then(|(_, md, validation_request)| graph_value(&md, &validation_request, format));
        return run.finish(result);
    }

    let result = prepared.and_then(|(opened, md, validation_request)| {
        let options = ReferenceValidationOptions {
            graph: ReferenceGraphOptions::with_compose(&validation_request),
            validate_remote: remote,
            remote_timeout: std::time::Duration::from_secs(timeout),
            validate_fragments: fragments,
            fail_fast,
        };
        let report = md.validate_references(options).wrap_err("Reference validation failed")?;
        let failure = (!report.is_valid()).then(|| report_failure_diagnostics(&report));
        let mut view = ValidationReportView::new(report)
            .with_root_source(ComposeSource::File(opened.path().to_path_buf()))
            .with_launch_dir(request.launch_dir())
            .with_visibility(ReferenceReportVisibility::All);
        if show_all {
            view = view.with_reference_listing();
        }
        Ok(match failure {
            Some(diagnostics) => Completed::failure(view).with_diagnostics(diagnostics),
            None => Completed::success(view),
        })
    });
    run.finish(result)
}

/// The diagram `--graph` selects: printed as its source in a terminal, and a
/// code block in Markdown and HTML.
fn graph_value(
    md: &Markdown,
    validation_request: &ComposeRequest,
    format: GraphFormat,
) -> Result<Completed<CommandValue<GraphNotationData>>> {
    let graph = md
        .reference_graph(ReferenceGraphOptions::with_compose(validation_request))
        .wrap_err("Failed to build reference graph")?;
    let source = match format {
        GraphFormat::Mermaid => graph.to_mermaid(),
        GraphFormat::Dot => graph.to_dot(),
    };
    let prose = code_document(Some(format.as_str()), &source);
    let terminal = source.clone();
    let data = GraphNotationData { graph: GraphNotation { format: format.as_str(), source } };
    Ok(Completed::success(CommandValue::new(data, prose).with_terminal(move |_| terminal.clone())))
}
