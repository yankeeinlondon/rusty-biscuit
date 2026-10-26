//! Command dispatch logic for the schematic-gen CLI.
//!
//! Contains the implementation of import commands, separated from CLI argument
//! parsing to keep `main.rs` focused on clap struct definitions and dispatch.

use clap::ValueEnum;
use colored::Colorize;

use crate::asyncapi_import::{self, AsyncImportOptions, WsRole as AsyncWsRole};
use crate::errors::GeneratorError;
use crate::import_pipeline::{self, ImportOptions};

/// WebSocket role mapping mode for AsyncAPI import.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum WsRoleArg {
    #[default]
    Client,
    Server,
    Both,
}

impl From<WsRoleArg> for AsyncWsRole {
    fn from(value: WsRoleArg) -> Self {
        match value {
            WsRoleArg::Client => AsyncWsRole::Client,
            WsRoleArg::Server => AsyncWsRole::Server,
            WsRoleArg::Both => AsyncWsRole::Both,
        }
    }
}

/// Runs the OpenAPI import command.
pub fn run_import_command(options: ImportOptions, verbose: u8) -> Result<(), GeneratorError> {
    if verbose > 0 {
        eprintln!("Importing OpenAPI spec: {}", options.input);
        eprintln!(
            "Output directory: {}",
            options.definitions_out.as_deref().unwrap_or(&options.output)
        );
        if options.dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }

    println!("{}", "Importing OpenAPI specification...".dimmed());

    let dry_run = options.dry_run;
    let result = import_pipeline::run_import(&options)?;

    let warn_count = result
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(
                d.severity,
                schematic_define::openapi::import::DiagnosticSeverity::Warn
            )
        })
        .count();
    let error_count = result
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(
                d.severity,
                schematic_define::openapi::import::DiagnosticSeverity::Error
            )
        })
        .count();

    if !dry_run {
        println!(
            "{} Imported '{}': {} endpoints, {} models",
            "[OK]".green().bold(),
            result.api_name,
            result.endpoint_count,
            result.model_count,
        );
    } else {
        println!(
            "{} Dry run complete for '{}': {} endpoints, {} models",
            "[OK]".green().bold(),
            result.api_name,
            result.endpoint_count,
            result.model_count,
        );
    }

    if warn_count > 0 || error_count > 0 {
        println!(
            "     {} warnings, {} errors in diagnostics",
            warn_count.to_string().yellow(),
            error_count.to_string().red(),
        );
    }

    Ok(())
}

/// Runs the AsyncAPI export command: one document per registered WebSocket
/// API, or only `api` when given. A full export deletes documents no API
/// produces; a selected one deletes nothing.
///
/// ## Errors
///
/// [`GeneratorError::ConfigError`] for an unknown API name, and the errors of
/// [`prepare_asyncapi_output`](crate::export::asyncapi::prepare_asyncapi_output)
/// and publication.
pub fn run_export_asyncapi_command(
    output: &str,
    api: Option<&str>,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    use crate::export::asyncapi::{asyncapi_documents, prepare_asyncapi_output};

    let mut documents = asyncapi_documents(schematic_definitions::registry::registrations());
    let complete = api.is_none();
    if let Some(name) = api {
        let known: Vec<String> = documents.iter().map(|document| document.api.name.clone()).collect();
        documents.retain(|document| document.api.name.eq_ignore_ascii_case(name));
        if documents.is_empty() {
            return Err(GeneratorError::ConfigError(format!(
                "unknown WebSocket API `{name}`; available: {}",
                known.join(", ")
            )));
        }
    }
    let (plan, warnings) = prepare_asyncapi_output(std::path::Path::new(output), &documents, complete)?;
    for warning in &warnings {
        eprintln!("{} {}", "[WARN]".yellow().bold(), warning);
    }
    if dry_run {
        for mutation in plan.mutations() {
            println!("{} Would {}", "[OK]".green().bold(), mutation);
        }
        return Ok(());
    }
    let published = plan.publish()?;
    if verbose > 0 {
        for mutation in &published.applied {
            println!("  {mutation}");
        }
    }
    println!(
        "{} Published {} AsyncAPI document(s) to {} ({} written, {} deleted)",
        "[OK]".green().bold(),
        documents.len(),
        plan.root().display(),
        published.written(),
        published.deleted(),
    );
    Ok(())
}

/// Runs the AsyncAPI import command.
#[allow(clippy::too_many_arguments)]
pub fn run_import_asyncapi_command(
    input: &str,
    api_name: Option<&str>,
    module_path: Option<&str>,
    ws_role: WsRoleArg,
    output: &str,
    dry_run: bool,
    strict: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    if verbose > 0 {
        eprintln!("Importing AsyncAPI spec: {}", input);
        eprintln!("Output directory: {}", output);
        if dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }

    println!("{}", "Importing AsyncAPI specification...".dimmed());

    let options = AsyncImportOptions {
        input: input.to_string(),
        api_name: api_name.map(String::from),
        module_path: module_path.map(String::from),
        ws_role: ws_role.into(),
        output: output.to_string(),
        dry_run,
        strict,
        verbose: verbose > 0,
    };

    let result = asyncapi_import::run_import_asyncapi(&options)?;
    let warn_count = result.diagnostics.iter().filter(|d| d.is_warn()).count();

    if !dry_run {
        for path in &result.outputs {
            println!("  wrote {}", path.display());
        }
        println!(
            "{} Imported '{}' (AsyncAPI): {} endpoints, {} messages, {} models",
            "[OK]".green().bold(),
            result.api_name,
            result.endpoint_count,
            result.message_count,
            result.model_count,
        );
    } else {
        println!(
            "{} Dry run complete for '{}' (AsyncAPI): {} endpoints, {} messages, {} models",
            "[OK]".green().bold(),
            result.api_name,
            result.endpoint_count,
            result.message_count,
            result.model_count,
        );
    }

    if warn_count > 0 {
        println!("     {} warnings in diagnostics", warn_count.to_string().yellow());
    }

    Ok(())
}
