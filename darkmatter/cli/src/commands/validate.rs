//! `md validate` subcommand implementation.

use crate::args::{GraphFormat, ValidateOutputFormat, ValidateTarget};
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use tracing::{info, instrument};

#[instrument(skip_all, fields(command = "validate"))]
pub(crate) fn run_validate(target: ValidateTarget) -> Result<()> {
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
            let md = Markdown::try_from(input.as_path())
                .wrap_err_with(|| format!("Failed to load {}", input.display()))?;

            // If --graph requested, print graph and exit
            if let Some(graph_format) = graph {
                let graph_options = ReferenceGraphOptions::default();
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
                graph: ReferenceGraphOptions::default(),
                validate_remote: remote,
                remote_timeout: std::time::Duration::from_secs(timeout),
                validate_fragments: fragments,
                fail_fast,
            };

            let report = md
                .validate_references(options)
                .wrap_err("Reference validation failed")?;

            match format {
                ValidateOutputFormat::Text => {
                    print_validation_report_text(&report, &input, show_all);
                }
                ValidateOutputFormat::Json => {
                    // Serialize the library `ReferenceValidationReport`
                    // directly. This is the same serde shape emitted by
                    // `md graph --validate --json`'s `validation` block,
                    // so the two CLI surfaces share one contract and
                    // cannot drift (review-2 finding #2). The shape is
                    // pinned by the baseline fixtures under
                    // `darkmatter/features/2026-06-17-cli-atheist/baseline/json/`.
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }

            if report.is_valid() {
                Ok(())
            } else {
                Err(eyre!("{} error(s) found", report.error_count()))
            }
        }
    }
}

/// Prints the validation report in the legacy plain-text shape.
///
/// `md validate refs`'s text output is the *primary* per-issue report: it
/// lists every issue (not just errors), it includes the scan/valid/issue
/// counts at the top, and it stays readable in CI logs that strip ANSI.
/// That is a different role from
/// [`ValidationReportView`](darkmatter::markdown::reference::validate::ValidationReportView),
/// which is a styled error-only summary used as a footer by
/// `md graph --validate`. Routing `--format text` through the view would
/// silently drop warnings, info-severity issues, the count header, and
/// the success case (`ValidationReportView` renders empty when there are
/// no errors), so the two surfaces are intentionally separate.
fn print_validation_report_text(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
    input: &std::path::Path,
    show_all: bool,
) {
    use darkmatter::markdown::reference::validate::ReferenceSeverity;

    println!("References scanned: {}", report.references_scanned);
    println!("Valid: {}", report.references_valid);
    println!("Issues: {}", report.issues.len());

    if !report.issues.is_empty() {
        println!();
    }

    for issue in &report.issues {
        let severity = match issue.severity {
            ReferenceSeverity::Error => "ERROR",
            ReferenceSeverity::Warning => "WARN ",
            ReferenceSeverity::Info => "INFO ",
        };

        let source = match &issue.origin.source {
            darkmatter::markdown::compose::ComposeSource::File(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| input.display().to_string()),
            _ => input.display().to_string(),
        };

        println!(
            "{severity}  {source}:{line}  {message}",
            line = issue.origin.line,
            message = issue.message
        );
    }

    if show_all && !report.warnings.is_empty() {
        println!();
        for warning in &report.warnings {
            println!("WARN   {warning}");
        }
    }
}
