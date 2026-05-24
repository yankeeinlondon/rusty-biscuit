//! CLI arguments parsing

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::engine::ArgValueCandidates;
use std::path::PathBuf;

use crate::commands::run::{RunnerFilter, run_model_candidates};

/// Sort order for HuggingFace search results.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortOrder {
    Downloads,
    Likes,
    Trending,
    Created,
    Modified,
}

impl From<SortOrder> for model_citizen::SortOrder {
    fn from(s: SortOrder) -> Self {
        match s {
            SortOrder::Downloads => Self::Downloads,
            SortOrder::Likes => Self::Likes,
            SortOrder::Trending => Self::Trending,
            SortOrder::Created => Self::Created,
            SortOrder::Modified => Self::Modified,
        }
    }
}

/// Model Citizen - Local LLM model management across multiple runners.
#[derive(Parser)]
#[command(name = "model")]
#[command(author, version, about, long_about = None, disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output JSON instead of terminal tables
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all models across all runners
    List {
        /// Filter models by name (case-insensitive substring match)
        filter: Option<String>,

        /// Filter by runner (ollama, lmstudio, llamacpp)
        #[arg(short, long)]
        runner: Option<String>,

        /// Show additional columns (format)
        #[arg(short, long)]
        verbose: bool,

        /// Sort by source app, then by name
        #[arg(long)]
        app: bool,

        /// Sort by file size (largest first)
        #[arg(long)]
        size: bool,
    },

    /// Show detailed information about a model
    Info {
        /// Model name or ID
        model: String,
    },

    /// Search for models on Hugging Face
    Search {
        /// Search terms (omit to browse by sort order)
        query: Vec<String>,

        /// Maximum results to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Sort results by
        #[arg(short, long, default_value = "downloads", value_enum)]
        sort: SortOrder,

        /// Show additional columns (created date, modified date)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Download a model from Hugging Face
    Download {
        /// Search terms or exact repo ID (e.g., "llama gguf" or "TheBloke/Llama-2-7B-GGUF")
        query: Vec<String>,

        /// Maximum search results to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Sort search results by
        #[arg(short, long, default_value = "downloads", value_enum)]
        sort: SortOrder,

        /// Show additional columns (created date, modified date)
        #[arg(short, long)]
        verbose: bool,

        /// Destination directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Remove any existing partial download (`.tmp`) files before downloading
        #[arg(long)]
        remove_partial: bool,
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

    /// Run a local GGUF model with llama-server
    Run {
        /// Model name or ID (omit for interactive selection)
        #[arg(add = ArgValueCandidates::new(run_model_candidates))]
        model: Option<String>,

        /// Filter models by runner (ollama, lmstudio, llamacpp)
        #[arg(long, value_enum)]
        runner: Option<RunnerFilter>,

        /// Host interface for llama-server
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port for llama-server
        #[arg(long, default_value_t = 8080)]
        port: u16,

        /// Context size in tokens
        #[arg(long)]
        ctx_size: Option<u32>,

        /// Number of CPU threads
        #[arg(long)]
        threads: Option<usize>,

        /// Number of layers to offload to GPU
        #[arg(long)]
        n_gpu_layers: Option<i32>,

        /// API key for llama-server
        #[arg(long)]
        api_key: Option<String>,

        /// Path to llama-server binary
        #[arg(long)]
        llama_server_bin: Option<PathBuf>,

        /// Do not open browser automatically
        #[arg(long)]
        no_browser: bool,

        /// Print resolved command without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Show shell completions setup instructions
    #[command(after_help = crate::COMPLETIONS_HELP)]
    Completions,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_list_command() {
        let cli = Cli::parse_from(["model", "list", "--runner", "ollama"]);
        match cli.command {
            Commands::List { runner, .. } => assert_eq!(runner.as_deref(), Some("ollama")),
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn parse_run_command() {
        let cli = Cli::parse_from(["model", "run", "llama-2-7b", "--runner", "llamacpp"]);
        match cli.command {
            Commands::Run { model, runner, .. } => {
                assert_eq!(model.as_deref(), Some("llama-2-7b"));
                assert_eq!(runner, Some(RunnerFilter::Llamacpp));
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn parse_json_flag() {
        let cli = Cli::parse_from(["model", "--json", "list"]);
        assert!(cli.json);
    }
}
