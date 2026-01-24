//! Schematic Code Generator
//!
//! Generates strongly-typed Rust client code from REST API definitions.

use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use colored::Colorize;
use schematic_definitions::elevenlabs::define_elevenlabs_rest_api;
use schematic_definitions::huggingface::define_huggingface_hub_api;
use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};
use schematic_definitions::openai::define_openai_api;
use schematic_gen::cargo_gen::write_cargo_toml;
use schematic_gen::errors::GeneratorError;
use schematic_gen::output::{generate_and_write, generate_and_write_all};
use schematic_gen::validate_api;

/// List of available API names for error messages.
const AVAILABLE_APIS: &str = "openai, elevenlabs, huggingface, ollama-native, ollama-openai, all";

/// Schematic code generator - transforms API definitions into typed Rust clients
#[derive(Parser, Debug)]
#[command(name = "schematic-gen")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    // Legacy flat arguments for backwards compatibility
    /// API definition to generate code for (e.g., "openai")
    #[arg(short, long, global = true)]
    api: Option<String>,

    /// Output directory for generated code
    #[arg(short, long, default_value = "schematic/schema/src")]
    output: String,

    /// Print generated code without writing files
    #[arg(long)]
    dry_run: bool,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate Rust client code from an API definition
    Generate {
        /// API definition to generate code for (e.g., "openai")
        #[arg(short, long)]
        api: String,

        /// Output directory for generated code
        #[arg(short, long, default_value = "schematic/schema/src")]
        output: String,

        /// Print generated code without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate an API definition without generating code
    Validate {
        /// API definition to validate (e.g., "openai")
        #[arg(short, long)]
        api: String,
    },
}

/// Resolves an API name to its definition.
fn resolve_api(name: &str) -> Result<schematic_define::RestApi, GeneratorError> {
    match name {
        "openai" => Ok(define_openai_api()),
        "elevenlabs" => Ok(define_elevenlabs_rest_api()),
        "huggingface" => Ok(define_huggingface_hub_api()),
        "ollama-native" => Ok(define_ollama_native_api()),
        "ollama-openai" => Ok(define_ollama_openai_api()),
        "all" => Err(GeneratorError::ConfigError(
            "Use resolve_all_apis() for 'all'".to_string(),
        )),
        other => Err(GeneratorError::ConfigError(format!(
            "Unknown API: '{}'. Available APIs: {}",
            other, AVAILABLE_APIS
        ))),
    }
}

/// Returns all available API definitions for batch generation.
///
/// Note: Ollama APIs are excluded from "all" because they share a definitions
/// module, which requires different handling. Generate them individually.
fn resolve_all_apis() -> Vec<schematic_define::RestApi> {
    vec![
        define_openai_api(),
        define_elevenlabs_rest_api(),
        define_huggingface_hub_api(),
        // Note: Ollama APIs excluded from "all" - generate individually
        // define_ollama_native_api(),
        // define_ollama_openai_api(),
    ]
}

/// Runs validation on an API and prints colored results.
///
/// ## Returns
///
/// `true` if validation passed, `false` if it failed.
fn run_validation(api: &schematic_define::RestApi, verbose: u8) -> bool {
    if verbose > 0 {
        eprintln!(
            "{} Validating API: {} ({} endpoints)",
            "...".dimmed(),
            api.name,
            api.endpoints.len()
        );
    }

    match validate_api(api) {
        Ok(()) => {
            println!("{} Request suffix format", "  [PASS]".green().bold());
            println!(
                "{} No naming collisions detected",
                "  [PASS]".green().bold()
            );
            println!();
            println!(
                "{} All validation checks passed for '{}'",
                "[OK]".green().bold(),
                api.name
            );
            true
        }
        Err(err) => {
            match &err {
                GeneratorError::InvalidRequestSuffix { suffix, reason } => {
                    println!(
                        "{} Request suffix '{}': {}",
                        "  [FAIL]".red().bold(),
                        suffix,
                        reason
                    );
                }
                GeneratorError::NamingCollision {
                    endpoint_id,
                    body_type,
                    suggestion,
                } => {
                    println!("{} Request suffix format", "  [PASS]".green().bold());
                    println!(
                        "{} Naming collision in endpoint '{}'",
                        "  [FAIL]".red().bold(),
                        endpoint_id
                    );
                    println!(
                        "         Body type '{}' conflicts with generated request struct",
                        body_type.yellow()
                    );
                    println!(
                        "         {} Rename to '{}'",
                        "Suggestion:".cyan(),
                        suggestion.green()
                    );
                }
                _ => {
                    println!("{} {}", "  [FAIL]".red().bold(), err);
                }
            }
            println!();
            println!(
                "{} Validation failed for '{}'",
                "[ERROR]".red().bold(),
                api.name
            );
            false
        }
    }
}

/// Runs the generate command.
fn run_generate(
    api_name: &str,
    output: &str,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    if api_name == "all" {
        return run_generate_all(output, dry_run, verbose);
    }

    let api = resolve_api(api_name)?;

    if verbose > 0 {
        eprintln!("Generating code for API: {}", api_name);
        eprintln!("Output directory: {}", output);
        if dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }

    // Run validation first
    println!("{}", "Validating API definition...".dimmed());
    if !run_validation(&api, verbose) {
        return Err(GeneratorError::ConfigError(
            "Validation failed. Fix the issues above before generating code.".to_string(),
        ));
    }
    println!();

    if verbose > 1 {
        eprintln!("API: {} ({} endpoints)", api.name, api.endpoints.len());
        for endpoint in &api.endpoints {
            eprintln!("  - {} {} {}", endpoint.id, endpoint.method, endpoint.path);
        }
    }

    println!("{}", "Generating code...".dimmed());
    let output_dir = Path::new(output);
    generate_and_write(&api, output_dir, dry_run)?;

    // Generate Cargo.toml in the parent directory of src/
    // The output_dir points to src/, so we need to get its parent for Cargo.toml
    let schema_dir = output_dir.parent().unwrap_or(Path::new("schematic/schema"));
    write_cargo_toml(schema_dir, dry_run, None)?;

    if !dry_run {
        println!(
            "{} Generated code to {}/lib.rs",
            "[OK]".green().bold(),
            output
        );
        println!(
            "{} Generated {}/Cargo.toml",
            "[OK]".green().bold(),
            schema_dir.display()
        );
    } else {
        println!(
            "{} Dry run complete (no files written)",
            "[OK]".green().bold()
        );
    }

    Ok(())
}

/// Runs the generate command for all APIs at once.
fn run_generate_all(output: &str, dry_run: bool, verbose: u8) -> Result<(), GeneratorError> {
    let apis = resolve_all_apis();

    if verbose > 0 {
        eprintln!("Generating code for all {} APIs", apis.len());
        eprintln!("Output directory: {}", output);
        if dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }

    // Run validation on all APIs first
    println!("{}", "Validating all API definitions...".dimmed());
    let mut all_valid = true;
    for api in &apis {
        if !run_validation(api, verbose) {
            all_valid = false;
        }
        println!();
    }

    if !all_valid {
        return Err(GeneratorError::ConfigError(
            "Validation failed. Fix the issues above before generating code.".to_string(),
        ));
    }

    if verbose > 1 {
        for api in &apis {
            eprintln!("API: {} ({} endpoints)", api.name, api.endpoints.len());
            for endpoint in &api.endpoints {
                eprintln!("  - {} {} {}", endpoint.id, endpoint.method, endpoint.path);
            }
        }
    }

    println!("{}", "Generating code for all APIs...".dimmed());
    let output_dir = Path::new(output);
    let api_refs: Vec<&schematic_define::RestApi> = apis.iter().collect();
    generate_and_write_all(&api_refs, output_dir, dry_run)?;

    // Generate Cargo.toml in the parent directory of src/
    let schema_dir = output_dir.parent().unwrap_or(Path::new("schematic/schema"));
    write_cargo_toml(schema_dir, dry_run, None)?;

    if !dry_run {
        println!(
            "{} Generated code for {} APIs to {}",
            "[OK]".green().bold(),
            apis.len(),
            output
        );
        println!(
            "{} Generated {}/Cargo.toml",
            "[OK]".green().bold(),
            schema_dir.display()
        );
    } else {
        println!(
            "{} Dry run complete (no files written)",
            "[OK]".green().bold()
        );
    }

    Ok(())
}

/// Runs the validate command.
fn run_validate(api_name: &str, verbose: u8) -> Result<(), GeneratorError> {
    let api = resolve_api(api_name)?;

    if run_validation(&api, verbose) {
        Ok(())
    } else {
        Err(GeneratorError::ConfigError("Validation failed".to_string()))
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        // Explicit subcommand: generate
        Some(Commands::Generate {
            api,
            output,
            dry_run,
        }) => run_generate(&api, &output, dry_run, cli.verbose),
        // Explicit subcommand: validate
        Some(Commands::Validate { api }) => run_validate(&api, cli.verbose),
        // No subcommand: backwards-compatible mode (acts like generate)
        None => {
            if let Some(api_name) = cli.api {
                run_generate(&api_name, &cli.output, cli.dry_run, cli.verbose)
            } else {
                eprintln!(
                    "{} Missing required argument: --api <NAME>",
                    "[ERROR]".red().bold()
                );
                eprintln!();
                eprintln!("Usage:");
                eprintln!("  schematic-gen --api <NAME> [OPTIONS]");
                eprintln!("  schematic-gen generate --api <NAME> [OPTIONS]");
                eprintln!("  schematic-gen validate --api <NAME>");
                eprintln!();
                eprintln!("Available APIs: {}", AVAILABLE_APIS);
                return ExitCode::from(2);
            }
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{} {}", "[ERROR]".red().bold(), err);
            ExitCode::FAILURE
        }
    }
}
