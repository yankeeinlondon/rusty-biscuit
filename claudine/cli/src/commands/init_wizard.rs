use std::collections::HashMap;

use clap::Args;
use color_eyre::eyre::Result;

use claudine::actions::HookAction;
use claudine::config::claudine_config::{
    ClaudineConfig, ClaudineMessengerConfig, DefaultSounds, MessengerProviderConfig, TtsValue,
};
use claudine::config::{
    ProviderHookPlan, RegistrationResult, SkipReason, discover_agents_full, get_configurator,
};
use claudine::events::{AgenticEvent, Provider, recommended_sound};

use crate::log;

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long)]
    pub quick: bool,
}

pub async fn run_initialization() -> Result<()> {
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        return run_headless_initialization().await;
    }

    run_interactive_initialization().await
}

async fn run_headless_initialization() -> Result<()> {
    let config = build_headless_config();
    let path = claudine::dispatch::loader::user_config_path();
    claudine::dispatch::loader::save_claudine_config(&config, &path)?;
    Ok(())
}

async fn run_interactive_initialization() -> Result<()> {
    log::message("");
    log::message("  Welcome to Claudine!");
    log::message("");
    log::message("  Let's get you set up. This will only take a moment.");
    log::message("");

    let tts = configure_tts()?;
    let preferred_agent = configure_preferred_agent()?;

    log::message("");
    log::message("  Services");
    log::message("  Claudine provides two services that run automatically:");
    log::message("    - Logging - records all hook events");
    log::message("    - Protect - blocks dangerous commands");
    log::message("  Both are enabled by default.");
    log::message("");

    let _ = inquire::Confirm::new("  Press Enter to continue")
        .with_default(true)
        .prompt()?;

    let messenger = configure_messenger()?;

    log::message("");
    log::message("  Actions");
    log::message("  By default, a sound plays when human attention is needed.");
    log::message("");

    let _ = inquire::Confirm::new("  Press Enter to complete initialization")
        .with_default(true)
        .prompt()?;

    let config = build_config(tts, preferred_agent, messenger);
    let path = claudine::dispatch::loader::user_config_path();
    claudine::dispatch::loader::save_claudine_config(&config, &path)?;

    register_hooks_all_providers().await?;

    log::message("");
    log::message(&format!("  Config: {}", path.display()));
    log::message("  Edit with: claudine config");
    log::message("");
    Ok(())
}

fn configure_tts() -> Result<TtsValue> {
    log::message("  Text-to-Speech");
    log::message("");

    let has_tts = which::which("say").is_ok()
        || which::which("espeak-ng").is_ok()
        || which::which("espeak").is_ok();

    if has_tts {
        let provider_name = if which::which("say").is_ok() {
            "say (macOS)"
        } else {
            "espeak-ng"
        };
        log::message(&format!("  Found TTS provider: {provider_name}"));
        log::message("  TTS will be enabled.");
        log::message("");
        Ok(TtsValue::Boolean(true))
    } else {
        log::message("  No TTS provider found on this system.");
        log::message("");
        let install = inquire::Confirm::new("  Would you like to install a TTS provider?")
            .with_default(false)
            .prompt()?;
        if install {
            // Attempt to install espeak-ng as a reasonable cross-platform default.
            log::message("");
            log::message("  Attempting to install espeak-ng...");
            let install_result = std::process::Command::new("brew")
                .args(["install", "espeak-ng"])
                .output();
            match install_result {
                Ok(output) if output.status.success() => {
                    log::message("  espeak-ng installed successfully. TTS enabled.");
                    return Ok(TtsValue::Boolean(true));
                }
                _ => {
                    log::message("  Could not install espeak-ng automatically.");
                    log::message(
                        "  You can install a TTS provider later and enable TTS via `claudine config`.",
                    );
                }
            }
        }
        log::message("  TTS will be disabled for now.");
        log::message("");
        Ok(TtsValue::Boolean(false))
    }
}

fn configure_preferred_agent() -> Result<Provider> {
    log::message("");
    log::message("  Preferred Agent");
    log::message("");
    log::message("  When using compose without specifying a provider,");
    log::message("  which agent should be the default?");
    log::message("");

    let agents = discover_agents_full();
    let installed: Vec<Provider> = agents
        .iter()
        .filter(|a| a.on_path)
        .map(|a| a.provider)
        .collect();

    if installed.is_empty() {
        log::message("  No agents detected. Defaulting to Claude.");
        return Ok(Provider::Claude);
    }

    let names: Vec<String> = installed.iter().map(|p| p.to_string()).collect();
    let selection = inquire::Select::new("  Select your preferred agent:", names).prompt()?;

    let index = installed
        .iter()
        .position(|p| p.to_string() == selection)
        .unwrap_or(0);

    Ok(installed[index])
}

fn configure_messenger() -> Result<Option<ClaudineMessengerConfig>> {
    log::message("");
    log::message("  Messenger");
    log::message("  Claudine can send notifications via Discord, Slack, Signal, or WhatsApp.");
    log::message("");

    let setup_now = inquire::Confirm::new("  Would you like to configure a messenger now?")
        .with_default(false)
        .prompt()?;

    if !setup_now {
        return Ok(None);
    }

    let providers = ["Discord", "Slack", "Signal", "WhatsApp"];
    let selection =
        inquire::Select::new("  Select messenger provider:", providers.to_vec()).prompt()?;

    let (name, config) = match selection {
        "Discord" => (
            "discord".to_string(),
            MessengerProviderConfig::Discord {
                channel_id: inquire::Text::new("  Discord channel ID:").prompt()?,
                bot_token_env: inquire::Text::new("  Bot token env var:")
                    .with_default("DISCORD_BOT_TOKEN")
                    .prompt()?,
            },
        ),
        "Slack" => (
            "slack".to_string(),
            MessengerProviderConfig::Slack {
                channel_id: inquire::Text::new("  Slack channel ID:").prompt()?,
                bot_token_env: inquire::Text::new("  Bot token env var:")
                    .with_default("SLACK_BOT_TOKEN")
                    .prompt()?,
            },
        ),
        "Signal" => (
            "signal".to_string(),
            MessengerProviderConfig::Signal {
                recipient: inquire::Text::new("  Signal recipient:").prompt()?,
                rpc_url_env: inquire::Text::new("  RPC URL env var:")
                    .with_default("SIGNAL_RPC_URL")
                    .prompt()?,
                account_env: inquire::Text::new("  Account env var:")
                    .with_default("SIGNAL_ACCOUNT")
                    .prompt()?,
            },
        ),
        "WhatsApp" => (
            "whatsapp".to_string(),
            MessengerProviderConfig::Whatsapp {
                recipient: inquire::Text::new("  WhatsApp recipient:").prompt()?,
                access_token_env: inquire::Text::new("  Access token env var:")
                    .with_default("WHATSAPP_ACCESS_TOKEN")
                    .prompt()?,
                phone_number_id_env: inquire::Text::new("  Phone number ID env var:")
                    .with_default("WHATSAPP_PHONE_NUMBER_ID")
                    .prompt()?,
            },
        ),
        _ => return Ok(None),
    };

    let mut configurations = std::collections::HashMap::new();
    configurations.insert(name.clone(), config);

    Ok(Some(ClaudineMessengerConfig {
        active_config: Some(name),
        configurations,
    }))
}

fn build_config(
    tts: TtsValue,
    preferred_agent: Provider,
    messenger: Option<ClaudineMessengerConfig>,
) -> ClaudineConfig {
    let mut actions = HashMap::new();
    actions.insert(
        AgenticEvent::HumanInTheLoop,
        vec![HookAction::SoundEffect {
            effect: recommended_sound(&AgenticEvent::HumanInTheLoop).to_string(),
            volume: 1.0,
            speed: 1.0,
        }],
    );

    ClaudineConfig {
        tts,
        messenger,
        logging: true,
        protect: claudine::services::protect::config::ProtectConfig::default(),
        actions,
        preferred_agent,
        canonical_provider: None,
        default_sounds: DefaultSounds {
            success: Some("confirmation".to_string()),
            attention: Some("doorbell".to_string()),
            error: Some("error-1".to_string()),
        },
    }
}

/// CI-safe headless defaults: TTS off, Logging on, Protect default.
fn build_headless_config() -> ClaudineConfig {
    let agents = discover_agents_full();
    let preferred_agent = agents
        .iter()
        .find(|a| a.on_path)
        .map(|a| a.provider)
        .unwrap_or(Provider::Claude);

    build_config(TtsValue::Boolean(false), preferred_agent, None)
}

async fn register_hooks_all_providers() -> Result<()> {
    let agents = discover_agents_full();

    log::message("");
    log::message("  Registering with detected agents:");
    for agent in &agents {
        if !agent.on_path {
            continue;
        }
        let provider = agent.provider;
        let plan = ProviderHookPlan {
            events: provider_hook_events(provider),
            canonical_for: None,
        };
        let configurator = get_configurator(provider);
        match configurator.register(&plan, None) {
            Ok(RegistrationResult::Registered { event_count }) => {
                log::message(&format!(
                    "    {provider}: registered ({event_count} events)"
                ));
            }
            Ok(RegistrationResult::Skipped(SkipReason::AlreadyRegistered)) => {
                log::message(&format!("    {provider}: already registered"));
            }
            Ok(RegistrationResult::Skipped(SkipReason::WrapperOnly { guidance })) => {
                log::message(&format!("    {provider}: skipped (wrapper-only)"));
                log::message(&format!("      {guidance}"));
            }
            Ok(RegistrationResult::Skipped(SkipReason::NotDetected)) => {
                log::message(&format!("    {provider}: not detected"));
            }
            Ok(RegistrationResult::Skipped(SkipReason::NoHookSupport)) => {
                log::message(&format!("    {provider}: no hook support"));
            }
            Err(e) => {
                log::message(&format!("    {provider}: {e}"));
            }
        }
    }
    Ok(())
}

fn provider_hook_events(provider: Provider) -> Vec<AgenticEvent> {
    AgenticEvent::ALL
        .into_iter()
        .filter(|event| provider.supports_event_via_hook(event))
        .collect()
}
