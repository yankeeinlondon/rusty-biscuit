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
                    print_validation_report_json(&report)?;
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

fn print_validation_report_json(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
) -> Result<()> {
    use darkmatter::markdown::reference::validate::ReferenceSeverity;

    let issues: Vec<serde_json::Value> = report
        .issues
        .iter()
        .map(|i| {
            serde_json::json!({
                "code": format!("{:?}", i.code),
                "message": i.message,
                "severity": match i.severity {
                    ReferenceSeverity::Error => "error",
                    ReferenceSeverity::Warning => "warning",
                    ReferenceSeverity::Info => "info",
                },
                "reference_id": i.reference_id,
                "line": i.origin.line,
            })
        })
        .collect();

    let json = serde_json::json!({
        "references_scanned": report.references_scanned,
        "references_valid": report.references_valid,
        "issues": issues,
        "warnings": report.warnings,
        "is_valid": report.is_valid(),
    });

    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
