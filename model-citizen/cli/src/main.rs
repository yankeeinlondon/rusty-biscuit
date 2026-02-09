//! Model Citizen CLI - Local LLM model management.
//!
//! Manage local LLM models across Ollama, LM Studio, and Llama.cpp.

mod commands;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

/// Model Citizen - Local LLM model management across multiple runners.
#[derive(Parser)]
#[command(name = "model")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format (table or json)
    #[arg(long, global = true, default_value = "table")]
    format: OutputFormat,
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

#[derive(Subcommand)]
enum Commands {
    /// List all models across all runners
    List {
        /// Filter by runner (ollama, lmstudio, llamacpp)
        #[arg(short, long)]
        runner: Option<String>,

        /// Show additional columns (format)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show detailed information about a model
    Info {
        /// Model name or ID
        model: String,
    },

    /// Search for models on Hugging Face
    Search {
        /// Search query
        query: String,

        /// Maximum results to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Download a model from Hugging Face
    Download {
        /// Repository ID (e.g., TheBloke/Llama-2-7B-GGUF)
        repo: String,

        /// Specific variant to download (optional, interactive if not provided)
        variant: Option<String>,

        /// Destination directory
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Remove a model
    Remove {
        /// Model name or ID
        model: String,

        /// Only remove from specific runner
        #[arg(short, long)]
        runner: Option<String>,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::List { runner, verbose } => {
            commands::list::run(runner, cli.format == OutputFormat::Json, verbose).await?;
        }
        Commands::Info { model } => {
            commands::info::run(&model, cli.format == OutputFormat::Json).await?;
        }
        Commands::Search { query, limit } => {
            commands::search::run(&query, limit, cli.format == OutputFormat::Json).await?;
        }
        Commands::Download {
            repo,
            variant,
            output,
        } => {
            commands::download::run(&repo, variant.as_deref(), output.as_deref()).await?;
        }
        Commands::Remove {
            model,
            runner,
            force,
        } => {
            commands::remove::run(&model, runner.as_deref(), force).await?;
        }
    }

    Ok(())
}
