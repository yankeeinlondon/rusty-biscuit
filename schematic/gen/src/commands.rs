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
    let error_count = result.diagnostics.iter().filter(|d| d.is_error()).count();

    if !dry_run {
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

    if warn_count > 0 || error_count > 0 {
        println!(
            "     {} warnings, {} errors in diagnostics",
            warn_count.to_string().yellow(),
            error_count.to_string().red(),
        );
    }

    Ok(())
}
