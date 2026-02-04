use clap::Args;
use color_eyre::eyre::Result;

use claudine_lib::config::{detect_agents, RegistrationResult, SkipReason};
use claudine_lib::events::Provider;

/// Arguments for the sync subcommand.
#[derive(Args)]
pub struct SyncArgs {
    /// Show what would change without writing.
    #[arg(long)]
    pub dry_run: bool,
    /// Sync only a specific provider.
    #[arg(long)]
    pub provider: Option<String>,
}

/// Re-sync hook registrations with detected agents.
pub async fn run(args: SyncArgs) -> Result<()> {
    // Load current config from user/repo locations
    let config = claudine_lib::dispatch::loader::load_config(None, None)?;

    let agents = detect_agents();
    let filter_provider = args
        .provider
        .as_deref()
        .map(parse_provider)
        .transpose()?;

    for (provider, configurator) in &agents {
        if let Some(ref filter) = filter_provider
            && provider != filter
        {
            continue;
        }

        if args.dry_run {
            let registered = configurator.is_registered(None).unwrap_or(false);
            if registered {
                println!("{provider}: already registered (no changes)");
            } else {
                println!("{provider}: would register");
            }
        } else {
            match configurator.register(&config, None) {
                Ok(RegistrationResult::Registered { event_count }) => {
                    println!("{provider}: synced ({event_count} events)");
                }
                Ok(RegistrationResult::Skipped(reason)) => match reason {
                    SkipReason::AlreadyRegistered => {
                        println!("{provider}: already up-to-date");
                    }
                    SkipReason::WrapperOnly { guidance } => {
                        println!("{provider}: wrapper-only - {guidance}");
                    }
                    SkipReason::NotDetected => {
                        println!("{provider}: not detected");
                    }
                },
                Err(e) => {
                    eprintln!("{provider}: error - {e}");
                }
            }
        }
    }

    Ok(())
}

fn parse_provider(name: &str) -> color_eyre::eyre::Result<Provider> {
    match name.to_lowercase().as_str() {
        "claude" => Ok(Provider::Claude),
        "codex" => Ok(Provider::Codex),
        "gemini" => Ok(Provider::Gemini),
        "opencode" | "open_code" => Ok(Provider::OpenCode),
        "roo" | "roo_code" | "roocode" => Ok(Provider::RooCode),
        other => color_eyre::eyre::bail!("Unknown provider: {other}"),
    }
}
