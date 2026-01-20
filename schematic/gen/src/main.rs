//! Schematic Code Generator
//!
//! Generates strongly-typed Rust client code from REST API definitions.

use std::path::Path;

use clap::Parser;
use schematic_definitions::elevenlabs::define_elevenlabs_rest_api;
use schematic_definitions::huggingface::define_huggingface_hub_api;
use schematic_definitions::openai::define_openai_api;
use schematic_gen::cargo_gen::write_cargo_toml;
use schematic_gen::errors::GeneratorError;
use schematic_gen::output::generate_and_write;

/// Schematic code generator - transforms API definitions into typed Rust clients
#[derive(Parser, Debug)]
#[command(name = "schematic-gen")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// API definition to generate code for (e.g., "openai")
    #[arg(short, long)]
    api: String,

    /// Output directory for generated code
    #[arg(short, long, default_value = "schematic/schema/src")]
    output: String,

    /// Print generated code without writing files
    #[arg(long)]
    dry_run: bool,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() -> Result<(), GeneratorError> {
    let cli = Cli::parse();

    if cli.verbose > 0 {
        eprintln!("Generating code for API: {}", cli.api);
        eprintln!("Output directory: {}", cli.output);
        if cli.dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }

    let api = match cli.api.as_str() {
        "openai" => define_openai_api(),
        "elevenlabs" => define_elevenlabs_rest_api(),
        "huggingface" => define_huggingface_hub_api(),
        other => {
            return Err(GeneratorError::ConfigError(format!(
                "Unknown API: '{}'. Available APIs: openai, elevenlabs, huggingface",
                other
            )));
        }
    };

    if cli.verbose > 1 {
        eprintln!("API: {} ({} endpoints)", api.name, api.endpoints.len());
        for endpoint in &api.endpoints {
            eprintln!("  - {} {} {}", endpoint.id, endpoint.method, endpoint.path);
        }
    }

    let output_dir = Path::new(&cli.output);
    generate_and_write(&api, output_dir, cli.dry_run)?;

    // Generate Cargo.toml in the parent directory of src/
    // The output_dir points to src/, so we need to get its parent for Cargo.toml
    let schema_dir = output_dir.parent().unwrap_or(Path::new("schematic/schema"));
    write_cargo_toml(schema_dir, cli.dry_run)?;

    if !cli.dry_run && cli.verbose > 0 {
        eprintln!("Successfully generated code to {}/lib.rs", cli.output);
        eprintln!("Successfully generated {}/Cargo.toml", schema_dir.display());
    }

    Ok(())
}
