//! Interactive init wizard for claudine configuration.

mod prompts;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::Result;

use claudine::config::{
    AgentInfo, RegistrationResult, SkipReason, detect_agents, discover_agents_full,
};
use claudine::events::{
    AgenticEvent, CanonicalProviderSettings, EventBinding, GlobalSettings, HookerConfig,
    LinkingSettings, Provider, ProviderConfig, quick_start_supported_providers, recommended_sound,
};
use claudine::linking::{
    CanonicalSelection, LinkableResource, ResourceScope, ranked_provider_preferences,
    resolve_repo_root, select_canonical_provider, set_canonical_provider,
};

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
        let global_config = dirs::home_dir()
            .map(|h| h.join(".claudine").join("config.json"))
            .unwrap_or_else(|| PathBuf::from("~/.claudine/config.json"));

        if !global_config.exists() {
            log::warn("No global ~/.claudine/config.json found.");
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

    let installed_providers = installed_provider_list(&all_agents);
    let ranked_preferences = prompts::prompt_provider_preferences(&installed_providers)?;
    let preference = ranked_provider_preferences(&installed_providers, &ranked_preferences);

    // Phase 2: Event Selection
    log::message("Phase 2: Event Selection");
    log::message("-------------------------");
    let selected_providers: Vec<_> = selected_agents.iter().map(|a| a.provider).collect();
    let selected_events = prompts::prompt_event_selection(&selected_providers)?;

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
                *event,
                EventBinding {
                    enabled: true,
                    actions,
                    matcher: None,
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
    let mut settings = prompts::prompt_global_settings()?;
    let scope = if repo_scope {
        ResourceScope::Repo
    } else {
        ResourceScope::User
    };
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let cwd = std::env::current_dir()?;
    let repo_root = if repo_scope {
        resolve_repo_root(&cwd)
    } else {
        cwd
    };
    settings.linking = build_linking_settings(
        scope,
        &installed_providers,
        &preference,
        &home_dir,
        &repo_root,
    );

    // Build final config with per-provider configuration
    let mut providers = HashMap::new();
    for agent in &selected_agents {
        providers.insert(
            agent.provider,
            ProviderConfig {
                events: event_bindings.clone(),
            },
        );
    }

    let config = HookerConfig {
        version: "1.0".to_string(),
        settings,
        providers,
    };

    // Phase 5: Write and Register
    log::message("");
    log::message("Phase 5: Write Configuration");
    log::message("-----------------------------");

    // Determine config path
    let config_path = if repo_scope {
        repo_root.join(".claudine").join("config.json")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".claudine")
            .join("config.json")
    };

    // Write config file
    let json = serde_json::to_string_pretty(&config)?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config_path, &json)?;
    log::message(&format!("Wrote config to {}", config_path.display()));

    // Handle .gitignore in repo mode
    if repo_scope {
        handle_repo_gitignore(&repo_root)?;
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
            Ok(RegistrationResult::Skipped(SkipReason::NoHookSupport)) => {
                log::message(&format!("  {provider}: no native hook support yet"));
            }
            Err(e) => {
                log::error(&format!("  {provider}: {e}"));
            }
        }
    }

    log::message("");
    log::message("Done! Run `claudine hooks` to verify.");
    Ok(())
}

/// Handle .gitignore prompt in repo mode.
fn handle_repo_gitignore(repo_root: &std::path::Path) -> Result<()> {
    let choice = prompts::prompt_gitignore_choice()?;

    match choice {
        prompts::GitignoreChoice::AddToGitignore => {
            let gitignore_path = repo_root.join(".gitignore");
            let content = if gitignore_path.exists() {
                std::fs::read_to_string(&gitignore_path)?
            } else {
                String::new()
            };

            let has_entry = content
                .lines()
                .map(str::trim)
                .any(|line| line == ".claudine/" || line == ".claudine");
            if !has_entry {
                let new_content = if content.is_empty() || content.ends_with('\n') {
                    format!("{}.claudine/\n", content)
                } else {
                    format!("{}\n.claudine/\n", content)
                };
                std::fs::write(&gitignore_path, new_content)?;
                log::message("Added .claudine/ to .gitignore");
            } else {
                log::message(".claudine/ already in .gitignore");
            }
        }
        prompts::GitignoreChoice::CommitIt => {
            log::message(".claudine/config.json will be committed to the repository");
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
    let config = default_config(repo_scope)?;

    // Determine config path
    let config_path = if repo_scope {
        let cwd = std::env::current_dir()?;
        let repo_root = resolve_repo_root(&cwd);
        repo_root.join(".claudine").join("config.json")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".claudine")
            .join("config.json")
    };

    // Write config file
    let json = serde_json::to_string_pretty(&config)?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
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
            Ok(RegistrationResult::Skipped(SkipReason::NoHookSupport)) => {
                log::message(&format!("  {provider}: no native hook support yet"));
            }
            Err(e) => {
                log::error(&format!("  {provider}: {e}"));
            }
        }
    }

    log::message("");
    log::message("Done! Run `claudine hooks` to verify.");
    Ok(())
}

fn default_config(repo_scope: bool) -> Result<HookerConfig> {
    // Create default event bindings
    let default_events = create_default_events();

    let scope = if repo_scope {
        ResourceScope::Repo
    } else {
        ResourceScope::User
    };
    let all_agents = discover_agents_full();
    let installed_providers = installed_provider_list(&all_agents);
    let preference = ranked_provider_preferences(&installed_providers, &[]);
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let cwd = std::env::current_dir()?;
    let repo_root = resolve_repo_root(&cwd);
    let settings = GlobalSettings {
        linking: build_linking_settings(
            scope,
            &installed_providers,
            &preference,
            &home_dir,
            &repo_root,
        ),
        ..GlobalSettings::default()
    };

    // Apply to all supported providers that have hook registration
    let mut providers = HashMap::new();
    for provider in quick_start_supported_providers() {
        providers.insert(
            provider,
            ProviderConfig {
                events: default_events.clone(),
            },
        );
    }

    Ok(HookerConfig {
        version: "1.0".to_string(),
        settings,
        providers,
    })
}

fn installed_provider_list(agents: &[AgentInfo]) -> Vec<Provider> {
    let mut providers: Vec<Provider> = agents
        .iter()
        .filter(|agent| agent.is_available())
        .map(|agent| agent.provider)
        .collect();
    providers.sort_by_key(|provider| provider.to_string());
    providers.dedup();
    providers
}

fn build_linking_settings(
    scope: ResourceScope,
    installed_providers: &[Provider],
    preference: &[Provider],
    home_dir: &std::path::Path,
    repo_root: &std::path::Path,
) -> Option<LinkingSettings> {
    if installed_providers.is_empty() {
        return None;
    }

    let preference = ranked_provider_preferences(installed_providers, preference);
    let mut canonical_provider = CanonicalProviderSettings::default();

    for resource in LinkableResource::ALL {
        if let CanonicalSelection::Selected { provider, .. } = select_canonical_provider(
            scope,
            resource,
            installed_providers,
            &preference,
            home_dir,
            repo_root,
        ) {
            set_canonical_provider(&mut canonical_provider, scope, resource, provider);
        }
    }

    Some(LinkingSettings {
        preference,
        canonical_provider,
    })
}

fn create_default_events() -> HashMap<AgenticEvent, EventBinding> {
    use claudine::events::HookAction;

    let mut events = HashMap::new();

    // SessionStart -> SFX power-up
    events.insert(
        AgenticEvent::SessionStart,
        EventBinding {
            enabled: true,
            actions: vec![HookAction::SoundEffect {
                name: recommended_sound(&AgenticEvent::SessionStart).to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
        },
    );

    // TurnComplete -> SFX success
    events.insert(
        AgenticEvent::TurnComplete,
        EventBinding {
            enabled: true,
            actions: vec![HookAction::SoundEffect {
                name: recommended_sound(&AgenticEvent::TurnComplete).to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
        },
    );

    // ToolError -> SFX error
    events.insert(
        AgenticEvent::ToolError,
        EventBinding {
            enabled: true,
            actions: vec![HookAction::SoundEffect {
                name: recommended_sound(&AgenticEvent::ToolError).to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
        },
    );

    // PermissionRequest -> SFX notification
    events.insert(
        AgenticEvent::PermissionRequest,
        EventBinding {
            enabled: true,
            actions: vec![HookAction::SoundEffect {
                name: recommended_sound(&AgenticEvent::PermissionRequest).to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
            matcher: None,
        },
    );

    events
}
