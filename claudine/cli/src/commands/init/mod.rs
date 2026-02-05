//! Interactive init wizard for claudine configuration.

mod defaults;
mod prompts;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::Result;

use claudine::config::{detect_agents, discover_agents_full, RegistrationResult, SkipReason};
use claudine::events::{AgenticEvent, EventBinding, GlobalSettings, HookerConfig};

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

    run_interactive(args.repo).await
}

/// Run the full interactive wizard.
async fn run_interactive(repo_scope: bool) -> Result<()> {
    log::message("Claudine Interactive Setup");
    log::message("===========================");
    log::message("");

    // Phase 0: Check for existing global config in repo mode
    if repo_scope {
        let global_hooker = dirs::home_dir()
            .map(|h| h.join(".hooker"))
            .unwrap_or_else(|| PathBuf::from("~/.hooker"));

        if !global_hooker.exists() {
            log::warn("No global ~/.hooker config found.");
            log::message("Consider running `claudine init` first to set up global defaults.");
            log::message("");
        }
    }

    // Phase 1: Agent Discovery
    log::message("Phase 1: Agent Discovery");
    log::message("-------------------------");
    let all_agents = discover_agents_full();
    let selected_agents = prompts::prompt_agent_selection(&all_agents)?;

    if selected_agents.is_empty() {
        log::error("No agents selected. Exiting.");
        return Ok(());
    }

    log::message("");
    log::message(&format!("Selected {} agent(s)", selected_agents.len()));
    log::message("");

    // Phase 2: Event Selection
    log::message("Phase 2: Event Selection");
    log::message("-------------------------");
    let selected_events = prompts::prompt_event_selection()?;

    if selected_events.is_empty() {
        log::warn("No events selected. Configuration will be minimal.");
    } else {
        log::message("");
        log::message(&format!("Selected {} event(s)", selected_events.len()));
    }
    log::message("");

    // Phase 3: Per-Event Action Configuration
    log::message("Phase 3: Action Configuration");
    log::message("------------------------------");
    let mut event_bindings = HashMap::new();

    for event in &selected_events {
        log::message("");
        log::message(&format!("Configuring: {}", event));
        let actions = prompts::configure_event_actions(event)?;

        if !actions.is_empty() {
            event_bindings.insert(
                event.clone(),
                EventBinding {
                    enabled: true,
                    actions,
                    matcher: None,
                    overrides: HashMap::new(),
                },
            );
        }
    }

    log::message("");
    log::message(&format!(
        "Configured {} event binding(s)",
        event_bindings.len()
    ));
    log::message("");

    // Phase 4: Global Settings
    log::message("Phase 4: Global Settings");
    log::message("-------------------------");
    let settings = prompts::prompt_global_settings()?;

    // Build final config
    let config = HookerConfig {
        version: "1.0".to_string(),
        settings,
        events: event_bindings,
    };

    // Phase 5: Write and Register
    log::message("");
    log::message("Phase 5: Write Configuration");
    log::message("-----------------------------");

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
    log::message(&format!("Wrote config to {}", config_path.display()));

    // Handle .gitignore in repo mode
    if repo_scope {
        handle_repo_gitignore()?;
    }

    // Register with agents
    log::message("");
    log::message("Registering with detected agents:");

    let agents = detect_agents();
    for (provider, configurator) in &agents {
        // Only register with selected agents
        let is_selected = selected_agents.iter().any(|a| a.provider == *provider);
        if !is_selected {
            continue;
        }

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

/// Handle .gitignore prompt in repo mode.
fn handle_repo_gitignore() -> Result<()> {
    let choice = prompts::prompt_gitignore_choice()?;

    match choice {
        prompts::GitignoreChoice::AddToGitignore => {
            let gitignore_path = std::env::current_dir()?.join(".gitignore");
            let content = if gitignore_path.exists() {
                std::fs::read_to_string(&gitignore_path)?
            } else {
                String::new()
            };

            if !content.lines().any(|l| l.trim() == ".hooker") {
                let new_content = if content.is_empty() || content.ends_with('\n') {
                    format!("{}.hooker\n", content)
                } else {
                    format!("{}\n.hooker\n", content)
                };
                std::fs::write(&gitignore_path, new_content)?;
                log::message("Added .hooker to .gitignore");
            } else {
                log::message(".hooker already in .gitignore");
            }
        }
        prompts::GitignoreChoice::CommitIt => {
            log::message(".hooker will be committed to the repository");
        }
        prompts::GitignoreChoice::DoNothing => {
            log::message("No .gitignore changes made");
        }
    }

    Ok(())
}

/// Run quick mode with sensible defaults.
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
    use claudine::events::EventAction;

    let mut events = HashMap::new();

    // SessionStart -> SFX power-up
    events.insert(
        AgenticEvent::SessionStart,
        EventBinding {
            enabled: true,
            actions: vec![EventAction::SoundEffect {
                name: defaults::recommended_sound(&AgenticEvent::SessionStart).to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
            overrides: HashMap::new(),
        },
    );

    // TurnComplete -> SFX success
    events.insert(
        AgenticEvent::TurnComplete,
        EventBinding {
            enabled: true,
            actions: vec![EventAction::SoundEffect {
                name: defaults::recommended_sound(&AgenticEvent::TurnComplete).to_string(),
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
                name: defaults::recommended_sound(&AgenticEvent::ToolError).to_string(),
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
                name: defaults::recommended_sound(&AgenticEvent::PermissionRequest).to_string(),
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
