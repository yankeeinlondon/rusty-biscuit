//! Model Citizen CLI - Local LLM model management.
//!
//! Manage local LLM models across Ollama, LM Studio, and Llama.cpp.

mod commands;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;
use clap_complete::engine::ArgValueCandidates;
use color_eyre::eyre::Result;
use commands::run::{RunOptions, RunnerFilter};

const COMPLETIONS_HELP: &str = r#"
SHELL COMPLETIONS

Enable dynamic shell completions for model.

Examples:
  # Bash - add to ~/.bashrc or ~/.bash_profile
  echo 'source <(COMPLETE=bash model)' >> ~/.bashrc

  # Zsh - add to ~/.zshrc
  echo 'source <(COMPLETE=zsh model)' >> ~/.zshrc

  # Fish - add to config
  echo 'COMPLETE=fish model | source' >> ~/.config/fish/config.fish

  # Disable completions
  COMPLETE=0
"#;

/// Model Citizen - Local LLM model management across multiple runners.
#[derive(Parser)]
#[command(name = "model")]
#[command(author, version, about, long_about = None, disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output JSON instead of terminal tables
    #[arg(long, global = true)]
    json: bool,
}

/// Sort order for HuggingFace search results.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum SortOrder {
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

#[derive(Subcommand)]
enum Commands {
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
        output: Option<std::path::PathBuf>,

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
        #[arg(add = ArgValueCandidates::new(commands::run::run_model_candidates))]
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
        llama_server_bin: Option<std::path::PathBuf>,

        /// Do not open browser automatically
        #[arg(long)]
        no_browser: bool,

        /// Print resolved command without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Show shell completions setup instructions
    #[command(after_help = COMPLETIONS_HELP)]
    Completions,
}

#[tokio::main]
async fn main() -> Result<()> {
    CompleteEnv::with_factory(Cli::command).complete();

    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::List {
            filter,
            runner,
            verbose,
            app,
            size,
        } => {
            commands::list::run(filter, runner, cli.json, verbose, app, size).await?;
        }
        Commands::Info { model } => {
            commands::info::run(&model, cli.json).await?;
        }
        Commands::Search {
            query,
            limit,
            sort,
            verbose,
        } => {
            let query = if query.is_empty() {
                None
            } else {
                Some(query.join(" "))
            };
            commands::search::run(query.as_deref(), limit, sort.into(), cli.json, verbose).await?;
        }
        Commands::Download {
            query,
            limit,
            sort,
            verbose,
            output,
            remove_partial,
        } => {
            let query = if query.is_empty() {
                None
            } else {
                Some(query.join(" "))
            };
            commands::download::run(
                query.as_deref(),
                limit,
                sort.into(),
                verbose,
                output.as_deref(),
                remove_partial,
            )
            .await?;
        }
        Commands::Remove {
            model,
            runner,
            force,
        } => {
            commands::remove::run(&model, runner.as_deref(), force).await?;
        }
        Commands::Run {
            model,
            runner,
            host,
            port,
            ctx_size,
            threads,
            n_gpu_layers,
            api_key,
            llama_server_bin,
            no_browser,
            dry_run,
        } => {
            commands::run::run(RunOptions {
                model,
                runner,
                host,
                port,
                ctx_size,
                threads,
                n_gpu_layers,
                api_key,
                llama_server_bin,
                no_browser,
                dry_run,
            })
            .await?;
        }
        Commands::Completions => {
            print!("{}", COMPLETIONS_HELP.trim_start());
        }
    }

    Ok(())
}
