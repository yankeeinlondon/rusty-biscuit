use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use color_eyre::eyre::Result;
use unchained_ai::rigging::providers::models::ProviderModel;

mod commands;

/// AI pipeline tools and agent status monitoring
fn model_value_parser() -> clap::builder::PossibleValuesParser {
    clap::builder::PossibleValuesParser::new(ProviderModel::all_wire_ids())
}

#[derive(Parser)]
#[command(
    name = "unchained",
    version,
    about = "AI pipeline tools and agent status monitoring",
    after_help = "Use 'unchained <command> --help' for more information about a command."
)]
struct Cli {
    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Increase output verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL", hide = true)]
    completions: Option<Shell>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Show usage limits and cap status for agentic platforms
    Limits {
        /// Filter to a specific platform (claude, codex)
        #[arg(short, long)]
        platform: Option<String>,
    },
    /// List models defined by `unchained-ai-gen`, optionally with metadata
    Models {
        /// Filter to a specific provider (e.g. openai, anthropic, gemini)
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Show detailed metadata for a specific model
    Model {
        /// Model identifier in provider/model-id format (e.g. openai/o3)
        #[arg(value_parser = model_value_parser())]
        model: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Set up tracing
    let filter = match std::env::var("RUST_LOG") {
        Ok(_) => tracing_subscriber::EnvFilter::from_default_env(),
        Err(_) => tracing_subscriber::EnvFilter::new("warn"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();

    // Handle shell completions
    if let Some(shell) = cli.completions {
        clap_complete::generate(
            shell,
            &mut Cli::command(),
            "unchained",
            &mut std::io::stdout(),
        );
        return Ok(());
    }

    match cli.command {
        Some(Commands::Limits { platform }) => {
            commands::limits::run(platform, cli.json).await?;
        }
        Some(Commands::Models { provider }) => {
            commands::models::run(provider, cli.json, cli.verbose > 0).await?;
        }
        Some(Commands::Model { model }) => {
            commands::model::run(model, cli.json).await?;
        }
        None => {
            Cli::command().print_help()?;
            std::process::exit(1);
        }
    }

    Ok(())
}
