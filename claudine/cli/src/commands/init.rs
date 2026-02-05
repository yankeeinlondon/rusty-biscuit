use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::Result;

use claudine::config::{detect_agents, RegistrationResult, SkipReason};
use claudine::events::*;

use crate::log;

/// Arguments for the init wizard.
#[derive(Args)]
pub struct InitArgs {
    /// Use default configuration (skip interactive prompts).
    #[arg(long)]
    pub quick: bool,
    /// Configure for repository scope instead of user scope.
    #[arg(long)]
    pub repo: bool,
}

/// Run the interactive init wizard.
pub async fn run(args: InitArgs) -> Result<()> {
    if args.quick {
        return run_quick(args.repo).await;
    }

    // Full interactive mode -- for now, delegate to quick defaults.
    // TODO: Phase 9 enhancement -- use inquire for interactive prompts
    log::warn("Interactive mode not yet implemented. Using --quick defaults.");
    run_quick(args.repo).await
}

async fn run_quick(repo_scope: bool) -> Result<()> {
    log::message("Claudine quick setup");
    log::message("");

    // Build default config
    let config = default_config();

    // Determine config path
    let config_path = if repo_scope {
        let cwd = std::env::current_dir()?;
        cwd.join(".hooker")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".hooker")
    };

    // Write config file
    let json = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, &json)?;
    log::message(&format!("  Wrote config to {}", config_path.display()));

    // Detect and register with agents
    let agents = detect_agents();
    log::message("");
    log::message("Registering with detected agents:");

    for (provider, configurator) in &agents {
        match configurator.register(&config, None) {
            Ok(RegistrationResult::Registered { event_count }) => {
                log::message(&format!("  {provider}: registered ({event_count} events)"));
            }
            Ok(RegistrationResult::Skipped(SkipReason::WrapperOnly { guidance })) => {
                log::message(&format!("  {provider}: skipped (wrapper-only)"));
                log::message(&format!("    {guidance}"));
            }
            Ok(RegistrationResult::Skipped(SkipReason::AlreadyRegistered)) => {
                log::message(&format!("  {provider}: already registered"));
            }
            Ok(RegistrationResult::Skipped(SkipReason::NotDetected)) => {
                log::message(&format!("  {provider}: not detected"));
            }
            Err(e) => {
                log::error(&format!("  {provider}: {e}"));
            }
        }
    }

    log::message("");
    log::message("Done! Run `claudine status` to verify.");
    Ok(())
}

fn default_config() -> HookerConfig {
    let mut events = HashMap::new();

    // TurnComplete -> SFX success
    events.insert(
        AgenticEvent::TurnComplete,
        EventBinding {
            enabled: true,
            actions: vec![EventAction::SoundEffect {
                name: "success".to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
            overrides: HashMap::new(),
        },
    );

    // ToolError -> SFX error
    events.insert(
        AgenticEvent::ToolError,
        EventBinding {
            enabled: true,
            actions: vec![EventAction::SoundEffect {
                name: "error".to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
            overrides: HashMap::new(),
        },
    );

    // PermissionRequest -> SFX notification
    events.insert(
        AgenticEvent::PermissionRequest,
        EventBinding {
            enabled: true,
            actions: vec![EventAction::SoundEffect {
                name: "notification".to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
            overrides: HashMap::new(),
        },
    );

    // SessionStart -> SFX power-up
    events.insert(
        AgenticEvent::SessionStart,
        EventBinding {
            enabled: true,
            actions: vec![EventAction::SoundEffect {
                name: "power-up".to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
            overrides: HashMap::new(),
        },
    );

    HookerConfig {
        version: "1.0".to_string(),
        settings: GlobalSettings::default(),
        events,
    }
}
