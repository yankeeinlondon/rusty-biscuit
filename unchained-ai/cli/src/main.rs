use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use color_eyre::eyre::Result;

mod commands;

/// AI pipeline tools and agent status monitoring
#[derive(Parser)]
#[command(
    name = "unchained",
    version,
    about,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Set up tracing
    let filter = match std::env::var("RUST_LOG") {
        Ok(_) => tracing_subscriber::EnvFilter::from_default_env(),
        Err(_) => tracing_subscriber::EnvFilter::new("warn"),
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

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
        None => {
            Cli::command().print_help()?;
            std::process::exit(1);
        }
    }

    Ok(())
}
