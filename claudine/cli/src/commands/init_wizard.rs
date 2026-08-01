use std::collections::HashMap;

use clap::Args;
use color_eyre::eyre::Result;
use tracing::info_span;

use crate::log;
use biscuit_speaks::detection::get_available_providers as get_available_tts_providers;
use biscuit_speaks::types::{CloudTtsProvider, HostTtsProvider, TtsProvider};
use claudine::actions::HookAction;
use claudine::config::claudine_config::{
    ClaudineConfig, ClaudineMessengerConfig, DefaultSounds, MessengerProviderConfig, TtsValue,
};
use claudine::config::{
    ProviderHookPlan, RegistrationResult, SkipReason, discover_agents_full, get_configurator,
};
use claudine::events::{AgenticEvent, recommended_sound};
use claudine::provider::Provider;

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long)]
    pub quick: bool,
}

pub async fn run_initialization() -> Result<()> {
    let _span = info_span!("init_wizard").entered();
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
    log::message(&format!(
        "  Config: {}",
        biscuit_file::to_portable_string(&path)
    ));
    log::message("  Edit with: claudine config");
    log::message("");
    Ok(())
}

fn configure_tts() -> Result<TtsValue> {
    log::message("  Text-to-Speech");
    log::message("");

    if let Some(provider_name) = detected_tts_provider_name() {
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
            log::message("");
            match tts_install_plan_for_current_host() {
                TtsInstallPlan::AutoInstall { command, args } => {
                    let rendered_command = format!("{command} {}", args.join(" "));
                    log::message(&format!(
                        "  Attempting to install a TTS provider with `{rendered_command}`..."
                    ));
                    let install_result = std::process::Command::new(&command).args(&args).output();
                    match install_result {
                        Ok(output) if output.status.success() => {
                            log::message("  TTS provider installed successfully. TTS enabled.");
                            return Ok(TtsValue::Boolean(true));
                        }
                        _ => {
                            log::message("  Could not install a TTS provider automatically.");
                            print_tts_install_guidance(&TtsInstallPlan::Guidance {
                                lines: fallback_tts_install_guidance(std::env::consts::OS),
                            });
                        }
                    }
                }
                guidance @ TtsInstallPlan::Guidance { .. } => {
                    print_tts_install_guidance(&guidance);
                }
            }
        }
        log::message("  TTS will be disabled for now.");
        log::message("");
        Ok(TtsValue::Boolean(false))
    }
}

fn configure_preferred_agent() -> Result<Option<Provider>> {
    log::message("");
    log::message("  Favorite Agent");
    log::message("");
    log::message("  When using compose without specifying a provider,");
    log::message("  which agent should be the default? (You can skip this and set it later.)");
    log::message("");

    let agents = discover_agents_full();
    let installed: Vec<Provider> = agents
        .iter()
        .filter(|a| a.on_path)
        .map(|a| a.provider)
        .collect();

    if installed.is_empty() {
        log::message("  No agents detected. No favorite agent will be configured.");
        log::message("  Set one later with: claudine config set favorite-agent <provider>");
        return Ok(None);
    }

    const SKIP_LABEL: &str = "(no favorite — decide each run)";
    let mut names: Vec<String> = installed.iter().map(|p| p.to_string()).collect();
    names.push(SKIP_LABEL.to_string());
    let selection = inquire::Select::new("  Select your favorite agent:", names).prompt()?;

    if selection == SKIP_LABEL {
        return Ok(None);
    }

    let index = installed
        .iter()
        .position(|p| p.to_string() == selection)
        .unwrap_or(0);

    Ok(Some(installed[index]))
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
    preferred_agent: Option<Provider>,
    messenger: Option<ClaudineMessengerConfig>,
) -> ClaudineConfig {
    let mut actions = HashMap::new();
    actions.insert(
        AgenticEvent::HumanInTheLoop,
        vec![HookAction::SoundEffect {
            effect: recommended_sound(&AgenticEvent::HumanInTheLoop).to_string(),
            volume: 1.0,
            speed: 1.0,
            when: None,
        }],
    );

    ClaudineConfig {
        tts,
        messenger,
        logging: true,
        protect: claudine::protect::config::ProtectConfig::default(),
        actions,
        matchers: HashMap::new(),
        preferred_agent,
        canonical_provider: None,
        models: HashMap::new(),
        default_sounds: DefaultSounds {
            success: Some("confirmation".to_string()),
            attention: Some("doorbell".to_string()),
            error: Some("error-1".to_string()),
        },
        prompt_for_missing: true,
        harvest_unmatched: false,
        exit_expressions: None,
        guard_settings: Default::default(),
    }
}

/// CI-safe headless defaults: TTS off, Logging on, Protect default.
fn build_headless_config() -> ClaudineConfig {
    let agents = discover_agents_full();
    let preferred_agent = agents.iter().find(|a| a.on_path).map(|a| a.provider);

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum TtsInstallPlan {
    AutoInstall { command: String, args: Vec<String> },
    Guidance { lines: Vec<String> },
}

#[derive(Debug, Clone, Copy, Default)]
struct TtsInstallCapabilities {
    has_brew: bool,
    has_apt: bool,
    has_dnf: bool,
    has_pacman: bool,
    has_zypper: bool,
    has_winget: bool,
    has_choco: bool,
}

fn detected_tts_provider_name() -> Option<&'static str> {
    get_available_tts_providers()
        .first()
        .map(tts_provider_display_name)
}

fn tts_provider_display_name(provider: &TtsProvider) -> &'static str {
    match provider {
        TtsProvider::Host(host) => match host {
            HostTtsProvider::Say => "say (macOS)",
            HostTtsProvider::ESpeak => "espeak-ng",
            HostTtsProvider::Piper => "piper",
            HostTtsProvider::EchoGarden => "echogarden",
            HostTtsProvider::Sherpa => "sherpa-onnx",
            HostTtsProvider::Mimic3 => "mimic3",
            HostTtsProvider::Festival => "festival",
            HostTtsProvider::Gtts => "gtts",
            HostTtsProvider::Sapi => "sapi (Windows)",
            HostTtsProvider::KokoroTts => "kokoro-tts",
            HostTtsProvider::Pico2Wave => "pico2wave",
            HostTtsProvider::SpdSay => "spd-say",
            _ => "host TTS",
        },
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs) => "elevenlabs",
        _ => "cloud TTS",
    }
}

fn tts_install_plan_for_current_host() -> TtsInstallPlan {
    let capabilities = TtsInstallCapabilities {
        has_brew: command_exists("brew"),
        has_apt: command_exists("apt-get"),
        has_dnf: command_exists("dnf"),
        has_pacman: command_exists("pacman"),
        has_zypper: command_exists("zypper"),
        has_winget: command_exists("winget"),
        has_choco: command_exists("choco"),
    };
    tts_install_plan(std::env::consts::OS, capabilities)
}

fn tts_install_plan(os: &str, capabilities: TtsInstallCapabilities) -> TtsInstallPlan {
    match os {
        "macos" if capabilities.has_brew => TtsInstallPlan::AutoInstall {
            command: "brew".to_string(),
            args: vec!["install".to_string(), "espeak-ng".to_string()],
        },
        "linux" if capabilities.has_apt => TtsInstallPlan::Guidance {
            lines: vec![
                "Install eSpeak NG with: sudo apt-get install espeak-ng".to_string(),
                "Then rerun `claudine config` to enable TTS.".to_string(),
            ],
        },
        "linux" if capabilities.has_dnf => TtsInstallPlan::Guidance {
            lines: vec![
                "Install eSpeak NG with: sudo dnf install espeak-ng".to_string(),
                "Then rerun `claudine config` to enable TTS.".to_string(),
            ],
        },
        "linux" if capabilities.has_pacman => TtsInstallPlan::Guidance {
            lines: vec![
                "Install eSpeak NG with: sudo pacman -S espeak-ng".to_string(),
                "Then rerun `claudine config` to enable TTS.".to_string(),
            ],
        },
        "linux" if capabilities.has_zypper => TtsInstallPlan::Guidance {
            lines: vec![
                "Install eSpeak NG with: sudo zypper install espeak-ng".to_string(),
                "Then rerun `claudine config` to enable TTS.".to_string(),
            ],
        },
        "windows" if capabilities.has_winget => TtsInstallPlan::Guidance {
            lines: vec![
                "Install eSpeak NG with: winget install eSpeak-NG.eSpeak-NG".to_string(),
                "Then reopen your terminal and rerun `claudine config`.".to_string(),
            ],
        },
        "windows" if capabilities.has_choco => TtsInstallPlan::Guidance {
            lines: vec![
                "Install eSpeak NG with: choco install espeak".to_string(),
                "Then reopen your terminal and rerun `claudine config`.".to_string(),
            ],
        },
        _ => TtsInstallPlan::Guidance {
            lines: fallback_tts_install_guidance(os),
        },
    }
}

fn fallback_tts_install_guidance(os: &str) -> Vec<String> {
    match os {
        "macos" => vec![
            "Homebrew was not found, so automatic installation is unavailable.".to_string(),
            "Install Homebrew from https://brew.sh and then run: brew install espeak-ng"
                .to_string(),
            "You can enable TTS later from `claudine config`.".to_string(),
        ],
        "linux" => vec![
            "Automatic installation is unavailable on this Linux host.".to_string(),
            "Install `espeak-ng` with your distro package manager, then rerun `claudine config`."
                .to_string(),
        ],
        "windows" => vec![
            "Automatic installation is unavailable on this Windows host.".to_string(),
            "Install a host TTS provider such as eSpeak NG, then rerun `claudine config`."
                .to_string(),
        ],
        _ => vec![
            "Automatic installation is unavailable on this host.".to_string(),
            "Install a supported TTS provider manually, then rerun `claudine config`.".to_string(),
        ],
    }
}

fn print_tts_install_guidance(plan: &TtsInstallPlan) {
    if let TtsInstallPlan::Guidance { lines } = plan {
        for line in lines {
            log::message(&format!("  {line}"));
        }
        log::message("  You can enable TTS later via `claudine config`.");
    }
}

fn command_exists(command: &str) -> bool {
    which::which(command).is_ok()
}

fn provider_hook_events(provider: Provider) -> Vec<AgenticEvent> {
    AgenticEvent::ALL
        .into_iter()
        .filter(|event| provider.supports_event_via_hook(event))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_with_brew_uses_auto_install() {
        assert_eq!(
            tts_install_plan(
                "macos",
                TtsInstallCapabilities {
                    has_brew: true,
                    ..TtsInstallCapabilities::default()
                },
            ),
            TtsInstallPlan::AutoInstall {
                command: "brew".to_string(),
                args: vec!["install".to_string(), "espeak-ng".to_string()],
            }
        );
    }

    #[test]
    fn linux_with_apt_uses_guided_instructions() {
        assert_eq!(
            tts_install_plan(
                "linux",
                TtsInstallCapabilities {
                    has_apt: true,
                    ..TtsInstallCapabilities::default()
                },
            ),
            TtsInstallPlan::Guidance {
                lines: vec![
                    "Install eSpeak NG with: sudo apt-get install espeak-ng".to_string(),
                    "Then rerun `claudine config` to enable TTS.".to_string(),
                ],
            }
        );
    }

    #[test]
    fn windows_without_package_manager_falls_back_to_manual_guidance() {
        assert_eq!(
            tts_install_plan("windows", TtsInstallCapabilities::default()),
            TtsInstallPlan::Guidance {
                lines: fallback_tts_install_guidance("windows"),
            }
        );
    }
}
