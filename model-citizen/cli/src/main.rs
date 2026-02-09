//! Model Citizen CLI - Local LLM model management.
//!
//! Manage local LLM models across Ollama, LM Studio, and Llama.cpp.

mod commands;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;
use color_eyre::eyre::Result;

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
            runner,
            verbose,
            app,
            size,
        } => {
            commands::list::run(runner, cli.format == OutputFormat::Json, verbose, app, size)
                .await?;
        }
        Commands::Info { model } => {
            commands::info::run(&model, cli.format == OutputFormat::Json).await?;
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
            commands::search::run(
                query.as_deref(),
                limit,
                sort.into(),
                cli.format == OutputFormat::Json,
                verbose,
            )
            .await?;
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
        Commands::Completions => {
            print!("{}", COMPLETIONS_HELP.trim_start());
        }
    }

    Ok(())
}
