//! CLI binary for updating ProviderModel enum with latest models from provider APIs.
//!
//! This utility fetches the latest model lists from all configured LLM providers
//! and updates the `ProviderModel` enum in `shared/src/providers/types.rs` with
//! new static variants.
//!
//! ## Usage
//!
//! ```bash
//! # Update all providers
//! cargo run --bin update-provider-models
//!
//! # Dry run (show what would be updated)
//! cargo run --bin update-provider-models -- --dry-run
//!
//! # Update specific provider only
//! cargo run --bin update-provider-models -- --provider anthropic
//!
//! # Custom output path
//! cargo run --bin update-provider-models -- --output /path/to/types.rs
//! ```
//!
//! ## Environment Variables
//!
//! The CLI respects all provider API keys from environment:
//! - `ANTHROPIC_API_KEY`
//! - `OPENAI_API_KEY`
//! - `DEEPSEEK_API_KEY`
//! - `GEMINI_API_KEY` or `GOOGLE_API_KEY`
//! - `MOONSHOT_API_KEY`
//! - `OPEN_ROUTER_API_KEY` or `OPENROUTER_API_KEY`
//! - `ZAI_API_KEY`
//! - `ZENMUX_API_KEY`

use clap::{Parser, ValueEnum};
use color_eyre::eyre::{Context, Result};
use shared::providers::ProviderModel;
use shared::providers::base::Provider;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Update ProviderModel enum with latest models from provider APIs
#[derive(Parser, Debug)]
#[command(name = "update-provider-models")]
#[command(about = "Update ProviderModel enum from live provider APIs", long_about = None)]
struct Args {
    /// Show what would be updated without modifying files
    #[arg(long)]
    dry_run: bool,

    /// Only update specific provider (all providers if not specified)
    #[arg(long, value_enum)]
    provider: Option<ProviderArg>,

    /// Custom output path for types.rs (default: shared/src/providers/types.rs)
    #[arg(long)]
    output: Option<String>,

    /// Verbosity level (-v = INFO, -vv = DEBUG, -vvv = TRACE)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

/// Provider argument for CLI (matches Provider enum)
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProviderArg {
    Anthropic,
    Deepseek,
    Gemini,
    MoonshotAi,
    Ollama,
    OpenAi,
    OpenRouter,
    Zai,
    ZenMux,
}

impl From<ProviderArg> for Provider {
    fn from(arg: ProviderArg) -> Self {
        match arg {
            ProviderArg::Anthropic => Provider::Anthropic,
            ProviderArg::Deepseek => Provider::Deepseek,
            ProviderArg::Gemini => Provider::Gemini,
            ProviderArg::MoonshotAi => Provider::MoonshotAi,
            ProviderArg::Ollama => Provider::Ollama,
            ProviderArg::OpenAi => Provider::OpenAi,
            ProviderArg::OpenRouter => Provider::OpenRouter,
            ProviderArg::Zai => Provider::Zai,
            ProviderArg::ZenMux => Provider::ZenMux,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install color-eyre for better error reporting
    color_eyre::install()?;

    let args = Args::parse();

    // Setup tracing subscriber based on verbosity
    let log_level = match args.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive(log_level.into())
                .add_directive("shared=trace".parse()?), // Always trace shared lib
        )
        .init();

    info!("Starting ProviderModel update");

    if args.dry_run {
        info!("DRY RUN MODE - no files will be modified");
    }

    if let Some(provider) = args.provider {
        info!("Filtering to single provider: {:?}", provider);
        // TODO: Add provider filtering to ProviderModel::update()
        // For now, we'll fetch all and filter in the summary
    }

    if let Some(ref output) = args.output {
        info!("Using custom output path: {}", output);
        // TODO: Add output path parameter to ProviderModel::update()
        eprintln!("Warning: Custom output path not yet implemented");
    }

    // Call ProviderModel::update()
    let summary = ProviderModel::update(args.dry_run)
        .await
        .context("Failed to update ProviderModel enum")?;

    // Display results
    println!("\n=== ProviderModel Update Summary ===\n");

    println!("Providers checked: {}", summary.providers_checked.len());
    for provider in &summary.providers_checked {
        println!("  - {:?}", provider);
    }

    println!("\nTotal new models added: {}", summary.total_added());

    if !summary.models_added.is_empty() {
        println!("\nModels added by provider:");
        for (provider, count) in &summary.models_added {
            if *count > 0 {
                println!("  {:?}: {} new models", provider, count);
            }
        }
    }

    if summary.aggregator_hints_applied > 0 {
        println!(
            "\nAggregator hints applied: {}",
            summary.aggregator_hints_applied
        );
    }

    if summary.total_added() == 0 {
        println!("\nNo new models found - enum is up to date!");
    } else if args.dry_run {
        println!("\nDRY RUN: No files were modified");
    } else {
        println!("\nEnum updated successfully!");
        println!("File: shared/src/providers/types.rs");
    }

    Ok(())
}
