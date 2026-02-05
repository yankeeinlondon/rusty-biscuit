use clap::Args;
use color_eyre::eyre::Result;

use claudine::config::{RegistrationResult, SkipReason, detect_agents};
use claudine::events::Provider;

use crate::log;

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
    // If config is missing, treat as "remove all hooks" operation
    let config = match claudine::dispatch::loader::load_config(None, None) {
        Ok(cfg) => Some(cfg),
        Err(claudine::error::ClaudineError::ConfigNotFound(_)) => None,
        Err(e) => return Err(e.into()),
    };

    let agents = detect_agents();
    let filter_provider = args.provider.as_deref().map(parse_provider).transpose()?;

    for (provider, configurator) in &agents {
        if let Some(ref filter) = filter_provider
            && provider != filter
        {
            continue;
        }

        // When config is None, deregister (remove all claudine hooks)
        // When config is Some, register/sync hooks
        match &config {
            None => {
                // Config removed - deregister from all providers
                if args.dry_run {
                    let registered = configurator.is_registered(None).unwrap_or(false);
                    if registered {
                        log::data(&format!("{provider}: would deregister"));
                    } else {
                        log::data(&format!("{provider}: not registered (no changes)"));
                    }
                } else {
                    match configurator.deregister(None) {
                        Ok(()) => {
                            log::data(&format!("{provider}: deregistered"));
                        }
                        Err(e) => {
                            log::error(&format!("{provider}: {e}"));
                        }
                    }
                }
            }
            Some(cfg) => {
                if args.dry_run {
                    let registered = configurator.is_registered(None).unwrap_or(false);
                    if registered {
                        log::data(&format!("{provider}: already registered (no changes)"));
                    } else {
                        log::data(&format!("{provider}: would register"));
                    }
                } else {
                    match configurator.register(cfg, None) {
                        Ok(RegistrationResult::Registered { event_count }) => {
                            log::data(&format!("{provider}: synced ({event_count} events)"));
                        }
                        Ok(RegistrationResult::Skipped(reason)) => match reason {
                            SkipReason::AlreadyRegistered => {
                                log::data(&format!("{provider}: already up-to-date"));
                            }
                            SkipReason::WrapperOnly { guidance } => {
                                log::data(&format!("{provider}: wrapper-only - {guidance}"));
                            }
                            SkipReason::NotDetected => {
                                log::data(&format!("{provider}: not detected"));
                            }
                            SkipReason::NoHookSupport => {
                                log::data(&format!("{provider}: no native hook support yet"));
                            }
                        },
                        Err(e) => {
                            log::error(&format!("{provider}: {e}"));
                        }
                    }
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
        other => color_eyre::eyre::bail!("Unknown provider: {other}"),
    }
}
