//! Schematic Code Generator
//!
//! Generates strongly-typed Rust client code from REST API definitions.

use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use schematic_define::openapi::{ExportFormat, ExportOptions};
use schematic_definitions::anthropic::define_anthropic_api;
use schematic_definitions::elevenlabs::define_elevenlabs_rest_api;
use schematic_definitions::emqx::{define_emqx_basic_api, define_emqx_bearer_api};
use schematic_definitions::gitea::define_gitea_api;
use schematic_definitions::github::define_github_api;
use schematic_definitions::gitlab::define_gitlab_api;
use schematic_definitions::huggingface::define_huggingface_hub_api;
use schematic_definitions::lmstudio::define_lmstudio_api;
use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};
use schematic_definitions::openai::define_openai_api;
use schematic_definitions::registry::get_registry;
use schematic_gen::cargo_gen::write_cargo_toml;
use schematic_gen::errors::GeneratorError;
use schematic_gen::import_pipeline::{self, ImportOptions};
use schematic_gen::openapi_output::write_openapi;
use schematic_gen::output::{generate_and_write, generate_and_write_all};
use schematic_gen::validate_api;

/// List of available API names for error messages.
const AVAILABLE_APIS: &str = "anthropic, openai, elevenlabs, gitea, github, gitlab, huggingface, lmstudio, ollama-native, ollama-openai, emqx-basic, emqx-bearer, all";

/// OpenAPI output format for CLI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OpenApiFormat {
    /// JSON format (default).
    #[default]
    Json,
    /// YAML format.
    Yaml,
}

impl From<OpenApiFormat> for ExportFormat {
    fn from(format: OpenApiFormat) -> Self {
        match format {
            OpenApiFormat::Json => ExportFormat::Json,
            OpenApiFormat::Yaml => ExportFormat::Yaml,
        }
    }
}

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

        /// Output directory for OpenAPI specification files
        ///
        /// When provided, exports the API definition to an OpenAPI 3.0.3 spec
        /// after generating Rust client code.
        #[arg(long, value_name = "DIR")]
        openapi_out: Option<String>,

        /// Output format for OpenAPI specification
        #[arg(long, value_enum, default_value = "json")]
        openapi_format: OpenApiFormat,
    },

    /// Validate an API definition without generating code
    Validate {
        /// API definition to validate (e.g., "openai")
        #[arg(short, long)]
        api: String,
    },

    /// Import an OpenAPI specification and generate Rust client code
    Import {
        /// Path to OpenAPI spec file (JSON or YAML)
        #[arg(short, long)]
        input: String,

        /// Override API name (derived from spec title by default)
        #[arg(long, value_name = "NAME")]
        api_name: Option<String>,

        /// Override module path for generated code
        #[arg(long, value_name = "PATH")]
        module_path: Option<String>,

        /// Output directory for generated code
        #[arg(short, long)]
        output: String,

        /// Print generated code without writing files
        #[arg(long)]
        dry_run: bool,

        /// Fail on any warning-level diagnostic
        #[arg(long)]
        strict: bool,
    },
}

/// Resolves an API name to its definition.
fn resolve_api(name: &str) -> Result<schematic_define::RestApi, GeneratorError> {
    match name {
        "anthropic" => Ok(define_anthropic_api()),
        "openai" => Ok(define_openai_api()),
        "elevenlabs" => Ok(define_elevenlabs_rest_api()),
        "gitea" => Ok(define_gitea_api()),
        "github" => Ok(define_github_api()),
        "gitlab" => Ok(define_gitlab_api()),
        "huggingface" => Ok(define_huggingface_hub_api()),
        "lmstudio" => Ok(define_lmstudio_api()),
        "ollama-native" => Ok(define_ollama_native_api()),
        "ollama-openai" => Ok(define_ollama_openai_api()),
        "emqx-basic" => Ok(define_emqx_basic_api()),
        "emqx-bearer" => Ok(define_emqx_bearer_api()),
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
fn resolve_all_apis() -> Vec<schematic_define::RestApi> {
    vec![
        define_anthropic_api(),
        define_openai_api(),
        define_elevenlabs_rest_api(),
        define_gitea_api(),
        define_github_api(),
        define_gitlab_api(),
        define_huggingface_hub_api(),
        define_lmstudio_api(),
        define_ollama_native_api(),
        define_ollama_openai_api(),
        define_emqx_basic_api(),
        define_emqx_bearer_api(),
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
    openapi_out: Option<&str>,
    openapi_format: OpenApiFormat,
) -> Result<(), GeneratorError> {
    if api_name == "all" {
        return run_generate_all(output, dry_run, verbose, openapi_out, openapi_format);
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

    // Export OpenAPI spec if requested
    if let Some(openapi_dir) = openapi_out {
        run_openapi_export(
            api_name,
            &api,
            openapi_dir,
            openapi_format,
            dry_run,
            verbose,
        )?;
    }

    Ok(())
}

/// Exports an API definition to OpenAPI format.
fn run_openapi_export(
    api_name: &str,
    api: &schematic_define::RestApi,
    openapi_dir: &str,
    format: OpenApiFormat,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    // Get the schema registry for this API
    let registry = match get_registry(api_name) {
        Some(r) => r,
        None => {
            println!(
                "{} Skipping OpenAPI export for '{}' - schema registry not available",
                "[WARN]".yellow().bold(),
                api_name
            );
            println!(
                "         {} Currently only 'openai' has complete schema registries.",
                "Hint:".cyan()
            );
            return Ok(());
        }
    };

    if verbose > 0 {
        println!("{}", "Exporting OpenAPI specification...".dimmed());
    }

    let openapi_path = Path::new(openapi_dir);

    // Create the directory if it doesn't exist
    if !dry_run && !openapi_path.exists() {
        std::fs::create_dir_all(openapi_path).map_err(|e| GeneratorError::WriteError {
            path: openapi_dir.to_string(),
            source: e,
        })?;
    }

    let options = ExportOptions::new()
        .with_version("1.0.0")
        .with_format(format.into());

    if dry_run {
        println!(
            "{} Would export OpenAPI spec to {}/{}.{}",
            "[OK]".green().bold(),
            openapi_dir,
            api.name.to_lowercase(),
            match format {
                OpenApiFormat::Json => "json",
                OpenApiFormat::Yaml => "yaml",
            }
        );
    } else {
        let path = write_openapi(api, &registry, &options, openapi_path)?;
        println!(
            "{} Exported OpenAPI spec to {}",
            "[OK]".green().bold(),
            path.display()
        );
    }

    Ok(())
}

/// Runs the generate command for all APIs at once.
fn run_generate_all(
    output: &str,
    dry_run: bool,
    verbose: u8,
    openapi_out: Option<&str>,
    openapi_format: OpenApiFormat,
) -> Result<(), GeneratorError> {
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

    // Export OpenAPI specs if requested
    if let Some(openapi_dir) = openapi_out {
        // Map API structs to names for registry lookup
        let api_names = [
            ("Anthropic", "anthropic"),
            ("OpenAI", "openai"),
            ("ElevenLabs", "elevenlabs"),
            ("Gitea", "gitea"),
            ("GitHub", "github"),
            ("GitLab", "gitlab"),
            ("HuggingFaceHub", "huggingface"),
            ("LmStudio", "lmstudio"),
            ("OllamaNative", "ollama-native"),
            ("OllamaOpenAI", "ollama-openai"),
            ("EmqxBasic", "emqx-basic"),
            ("EmqxBearer", "emqx-bearer"),
        ];

        for api in &apis {
            let api_name_lower = api.name.to_lowercase();
            let api_name = api_names
                .iter()
                .find(|(name, _)| *name == api.name)
                .map(|(_, key)| *key)
                .unwrap_or(&api_name_lower);

            run_openapi_export(api_name, api, openapi_dir, openapi_format, dry_run, verbose)?;
        }
    }

    Ok(())
}

/// Runs the import command.
fn run_import_command(
    input: &str,
    api_name: Option<&str>,
    module_path: Option<&str>,
    output: &str,
    dry_run: bool,
    strict: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    if verbose > 0 {
        eprintln!("Importing OpenAPI spec: {}", input);
        eprintln!("Output directory: {}", output);
        if dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }

    println!("{}", "Importing OpenAPI specification...".dimmed());

    let options = ImportOptions {
        input: input.to_string(),
        api_name: api_name.map(String::from),
        module_path: module_path.map(String::from),
        output: output.to_string(),
        dry_run,
        strict,
        verbose: verbose > 0,
    };

    let result = import_pipeline::run_import(&options)?;

    // Print summary
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
            openapi_out,
            openapi_format,
        }) => run_generate(
            &api,
            &output,
            dry_run,
            cli.verbose,
            openapi_out.as_deref(),
            openapi_format,
        ),
        // Explicit subcommand: validate
        Some(Commands::Validate { api }) => run_validate(&api, cli.verbose),
        // Explicit subcommand: import
        Some(Commands::Import {
            input,
            api_name,
            module_path,
            output,
            dry_run,
            strict,
        }) => run_import_command(
            &input,
            api_name.as_deref(),
            module_path.as_deref(),
            &output,
            dry_run,
            strict,
            cli.verbose,
        ),
        // No subcommand: backwards-compatible mode (acts like generate)
        None => {
            if let Some(api_name) = cli.api {
                run_generate(
                    &api_name,
                    &cli.output,
                    cli.dry_run,
                    cli.verbose,
                    None, // Legacy mode doesn't support OpenAPI export
                    OpenApiFormat::default(),
                )
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
