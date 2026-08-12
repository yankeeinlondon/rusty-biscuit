use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{CompleteEnv, Shell};
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

        /// Print one canonical provider/model identifier per line
        #[arg(long)]
        flat: bool,
    },
    /// Show detailed metadata for a specific model
    Model {
        /// Model identifier in provider/model-id format (e.g. openai/o3)
        #[arg(value_parser = model_value_parser())]
        model: String,
    },
    /// Print shell completion setup for the specified shell
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell, elvish)
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Dynamic shell completions: when invoked by a generated completion script
    // (COMPLETE=<shell> set in the env) this emits candidates and exits before
    // any real work happens.
    CompleteEnv::with_factory(Cli::command).complete();

    // Set up tracing
    let filter = match std::env::var("RUST_LOG") {
        Ok(_) => tracing_subscriber::EnvFilter::from_default_env(),
        Err(_) => tracing_subscriber::EnvFilter::new("warn"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Limits { platform }) => {
            commands::limits::run(platform, cli.json).await?;
        }
        Some(Commands::Models { provider, flat }) => {
            commands::models::run(provider, cli.json, cli.verbose > 0, flat).await?;
        }
        Some(Commands::Model { model }) => {
            commands::model::run(model, cli.json).await?;
        }
        Some(Commands::Completions { shell }) => {
            print_completions(shell);
        }
        None => {
            Cli::command().print_help()?;
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Prints shell completion setup instructions.
///
/// Completions are dynamic: the emitted line registers a callback that shells
/// out to `unchained` on every `<TAB>`, so candidates always reflect the
/// installed binary — no regeneration after upgrades. Sourcing this output
/// (`source <(unchained completions zsh)`) activates completions directly; the
/// leading comment is inert when sourced.
fn print_completions(shell: Shell) {
    let (setup_cmd, config_file) = match shell {
        Shell::Bash => ("source <(COMPLETE=bash unchained)", "~/.bashrc"),
        Shell::Zsh => ("source <(COMPLETE=zsh unchained)", "~/.zshrc"),
        Shell::Fish => (
            "COMPLETE=fish unchained | source",
            "~/.config/fish/config.fish",
        ),
        Shell::PowerShell => (
            r#"$env:COMPLETE = "powershell"; unchained | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE"#,
            "$PROFILE",
        ),
        Shell::Elvish => ("eval (E:COMPLETE=elvish unchained | slurp)", "~/.elvish/rc.elv"),
        _ => {
            eprintln!("Shell {shell:?} is not supported for dynamic completions");
            return;
        }
    };

    println!("# Add this line to {config_file}:");
    println!("{setup_cmd}");
}
