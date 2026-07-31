//! Schematic Code Generator
//!
//! Generates strongly-typed Rust client code from REST API definitions.

use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;

use schematic_gen::commands::WsRoleArg;
use schematic_gen::pipeline::{self, GenerateOpts, OpenApiFormat as PipelineOpenApiFormat};

/// OpenAPI output format for CLI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OpenApiFormat {
    #[default]
    Json,
    Yaml,
}

impl From<OpenApiFormat> for PipelineOpenApiFormat {
    fn from(format: OpenApiFormat) -> Self {
        match format {
            OpenApiFormat::Json => PipelineOpenApiFormat::Json,
            OpenApiFormat::Yaml => PipelineOpenApiFormat::Yaml,
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
        #[arg(short, long)]
        api: String,

        #[arg(short, long, default_value = "schematic/schema/src")]
        output: String,

        #[arg(long)]
        dry_run: bool,

        #[arg(long, value_name = "DIR")]
        openapi_out: Option<String>,

        #[arg(long, value_enum, default_value = "json")]
        openapi_format: OpenApiFormat,

        #[arg(long, value_name = "VERSION")]
        openapi_version: Option<String>,

        #[arg(long, value_name = "DIR")]
        postman_out: Option<String>,

        #[arg(long)]
        no_openapi: bool,

        #[arg(long)]
        no_postman: bool,
    },

    /// Validate an API definition without generating code
    Validate {
        #[arg(short, long)]
        api: String,
    },

    /// Import an OpenAPI specification and generate Rust client code
    Import {
        #[arg(short, long)]
        input: String,

        #[arg(long, value_name = "NAME")]
        api_name: Option<String>,

        #[arg(long, value_name = "PATH")]
        module_path: Option<String>,

        #[arg(short, long, default_value = "")]
        output: String,

        /// Emit a `schematic-definitions` module here instead of a standalone client
        #[arg(long, value_name = "DIR")]
        definitions_out: Option<String>,

        /// Skip endpoints whose path starts with this prefix (repeatable)
        #[arg(long, value_name = "PREFIX")]
        exclude_path: Vec<String>,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        strict: bool,
    },

    /// Import an AsyncAPI specification and emit websocket import artifacts
    ImportAsyncapi {
        #[arg(short, long)]
        input: String,

        #[arg(long, value_name = "NAME")]
        api_name: Option<String>,

        #[arg(long, value_name = "PATH")]
        module_path: Option<String>,

        #[arg(long, value_enum, default_value = "client")]
        ws_role: WsRoleArg,

        #[arg(short, long)]
        output: String,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        strict: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Generate {
            api,
            output,
            dry_run,
            openapi_out,
            openapi_format,
            openapi_version,
            postman_out,
            no_openapi,
            no_postman,
        }) => pipeline::run_generate(
            &api,
            &GenerateOpts {
                output: &output,
                dry_run,
                verbose: cli.verbose,
                openapi_out: openapi_out.as_deref(),
                openapi_format: openapi_format.into(),
                openapi_version: openapi_version.as_deref(),
                postman_out: postman_out.as_deref(),
                no_openapi,
                no_postman,
            },
        ),
        Some(Commands::Validate { api }) => pipeline::run_validate(&api, cli.verbose),
        Some(Commands::Import {
            input,
            api_name,
            module_path,
            output,
            definitions_out,
            exclude_path,
            dry_run,
            strict,
        }) => schematic_gen::commands::run_import_command(
            schematic_gen::import_pipeline::ImportOptions {
                input,
                api_name,
                module_path,
                output,
                definitions_out,
                exclude_paths: exclude_path,
                dry_run,
                strict,
                verbose: cli.verbose > 0,
            },
            cli.verbose,
        ),
        Some(Commands::ImportAsyncapi {
            input,
            api_name,
            module_path,
            ws_role,
            output,
            dry_run,
            strict,
        }) => schematic_gen::commands::run_import_asyncapi_command(
            &input,
            api_name.as_deref(),
            module_path.as_deref(),
            ws_role,
            &output,
            dry_run,
            strict,
            cli.verbose,
        ),
        None => {
            if let Some(api_name) = cli.api {
                pipeline::run_generate(
                    &api_name,
                    &GenerateOpts {
                        output: &cli.output,
                        dry_run: cli.dry_run,
                        verbose: cli.verbose,
                        openapi_out: None,
                        openapi_format: PipelineOpenApiFormat::default(),
                        openapi_version: None,
                        postman_out: None,
                        no_openapi: false,
                        no_postman: false,
                    },
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
                eprintln!("Available APIs: {}", pipeline::available_apis());
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
