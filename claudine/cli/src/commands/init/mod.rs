//! Interactive init wizard for claudine configuration.

mod prompts;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::Result;

use claudine::actions::HookAction;
use claudine::config::{
    AgentInfo, RegistrationResult, SkipReason, discover_agents_full, get_configurator,
};
use claudine::events::{
    AgenticEvent, CanonicalProviderSettings, EventBinding, GlobalSettings, HookerConfig,
    LinkingSettings, Provider, ProviderConfig, recommended_sound,
};
use claudine::linking::{
    CanonicalSelection, LinkableResource, ResourceScope, ranked_provider_preferences,
    resolve_repo_root, select_canonical_provider, set_canonical_provider,
};
use claudine::services::{ProtectConfig, ProtectPosture};

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
    let selected_agents: Vec<AgentInfo> = all_agents
        .iter()
        .filter(|agent| agent.is_available())
        .cloned()
        .collect();

    if selected_agents.is_empty() {
        log::error("No agents detected. Exiting.");
        return Ok(());
    }

    log::message("");
    log::message(&format!(
        "Detected {} available agent(s)",
        selected_agents.len()
    ));
    for agent in &selected_agents {
        log::message(&format!("  {}", agent.provider));
    }
    log::message("");

    // Phase 2: Provider Preferences
    log::message("Phase 2: Provider Preferences");
    log::message("------------------------------");
    let installed_providers = installed_provider_list(&all_agents);
    let ranked_preferences = prompts::prompt_provider_preferences(&installed_providers)?;
    let preference = ranked_provider_preferences(&installed_providers, &ranked_preferences);

    // Phase 3: Global Action Interview
    log::message("Phase 3: Action Defaults");
    log::message("-------------------------");
    let action_profile = prompts::prompt_action_profile()?;

    log::message("");
    log::message("Phase 4: Protect Defaults");
    log::message("-------------------------");
    let protect_posture = prompts::prompt_protect_posture()?;
    let protect_defaults = protect_posture
        .map(|posture| ProtectConfig::provider_aware_defaults(&installed_providers, posture));

    // Build provider-specific event bindings:
    // - include every event the provider can register via native hooks
    // - include empty actions as explicit no-op bindings
    let mut provider_event_bindings: HashMap<Provider, HashMap<AgenticEvent, EventBinding>> =
        HashMap::new();
    for agent in &selected_agents {
        provider_event_bindings.insert(
            agent.provider,
            build_provider_event_bindings(agent.provider, &action_profile),
        );
    }

    let total_bindings: usize = provider_event_bindings
        .values()
        .map(std::collections::HashMap::len)
        .sum();

    log::message("");
    log::message(&format!("Prepared {total_bindings} event binding(s):"));
    for agent in &selected_agents {
        let count = provider_event_bindings
            .get(&agent.provider)
            .map(std::collections::HashMap::len)
            .unwrap_or(0);
        log::message(&format!("  {}: {} events", agent.provider, count));
    }
    log::message("");

    // Build global settings (non-interactive)
    let mut settings = GlobalSettings::default();
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
    settings.protect = protect_defaults;

    // Build final config with per-provider configuration
    let mut providers = HashMap::new();
    for agent in &selected_agents {
        let events = provider_event_bindings
            .remove(&agent.provider)
            .unwrap_or_default();
        providers.insert(agent.provider, ProviderConfig { events });
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
    log::message("Registering with available agents:");
    for agent in &selected_agents {
        let provider = agent.provider;
        let configurator = get_configurator(provider);
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

    // Register with providers included in this config
    log::message("");
    log::message("Registering with configured providers:");
    let mut providers: Vec<Provider> = config.providers.keys().copied().collect();
    providers.sort_by_key(|provider| provider.to_string());
    for provider in providers {
        let configurator = get_configurator(provider);
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
        protect: Some(ProtectConfig::provider_aware_defaults(
            &installed_providers,
            ProtectPosture::Balanced,
        )),
        ..GlobalSettings::default()
    };

    // Apply to installed providers that can register at least one event via hooks.
    let quick_providers: Vec<Provider> = installed_providers
        .iter()
        .copied()
        .filter(|provider| !provider_hook_events(*provider).is_empty())
        .collect();

    let mut providers = HashMap::new();
    for provider in quick_providers {
        providers.insert(
            provider,
            ProviderConfig {
                events: create_quick_provider_events(provider),
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

fn provider_hook_events(provider: Provider) -> Vec<AgenticEvent> {
    AgenticEvent::ALL
        .into_iter()
        .filter(|event| provider.supports_event_via_hook(event))
        .collect()
}

fn build_provider_event_bindings(
    provider: Provider,
    action_profile: &prompts::InitActionProfile,
) -> HashMap<AgenticEvent, EventBinding> {
    provider_hook_events(provider)
        .into_iter()
        .map(|event| {
            (
                event,
                EventBinding {
                    enabled: true,
                    actions: actions_for_event(event, action_profile),
                    matcher: None,
                },
            )
        })
        .collect()
}

fn actions_for_event(
    event: AgenticEvent,
    action_profile: &prompts::InitActionProfile,
) -> Vec<HookAction> {
    let mut actions = Vec::new();

    match &action_profile.logging {
        prompts::LoggingProfile::None => {}
        prompts::LoggingProfile::All { target } => {
            actions.push(HookAction::Log {
                target: target.clone(),
            });
        }
        prompts::LoggingProfile::Some { target, events } => {
            if events.contains(&event) {
                actions.push(HookAction::Log {
                    target: target.clone(),
                });
            }
        }
    }

    if matches!(
        event,
        AgenticEvent::PermissionRequest | AgenticEvent::HumanInTheLoop
    ) {
        actions.extend(action_profile.input_required_actions.clone());
    }

    actions
}

fn create_quick_provider_events(provider: Provider) -> HashMap<AgenticEvent, EventBinding> {
    provider_hook_events(provider)
        .into_iter()
        .map(|event| {
            let actions = if matches!(
                event,
                AgenticEvent::SessionStart
                    | AgenticEvent::TurnComplete
                    | AgenticEvent::ToolError
                    | AgenticEvent::PermissionRequest
                    | AgenticEvent::HumanInTheLoop
            ) {
                vec![HookAction::SoundEffect {
                    name: recommended_sound(&event).to_string(),
                    volume: 1.0,
                    speed: 1.0,
                }]
            } else {
                vec![]
            };

            (
                event,
                EventBinding {
                    enabled: true,
                    actions,
                    matcher: None,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::actions::LogTarget;

    fn no_action_profile() -> prompts::InitActionProfile {
        prompts::InitActionProfile {
            logging: prompts::LoggingProfile::None,
            input_required_actions: vec![],
        }
    }

    #[test]
    fn provider_hook_events_only_returns_hook_supported_events() {
        for provider in [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::Goose,
            Provider::KimiCode,
            Provider::OpenCode,
            Provider::QwenCode,
            Provider::RooCode,
        ] {
            let events = provider_hook_events(provider);
            for event in events {
                assert!(provider.supports_event_via_hook(&event));
            }
        }
    }

    #[test]
    fn interactive_bindings_include_noop_entries_for_supported_events() {
        let bindings = build_provider_event_bindings(Provider::Claude, &no_action_profile());
        let expected: Vec<AgenticEvent> = AgenticEvent::ALL
            .into_iter()
            .filter(|event| Provider::Claude.supports_event_via_hook(event))
            .collect();

        assert_eq!(bindings.len(), expected.len());
        for event in expected {
            let binding = bindings
                .get(&event)
                .expect("every hook-supported event should be configured");
            assert!(binding.enabled);
            assert!(
                binding.actions.is_empty(),
                "expected no-op actions for {event}"
            );
        }
    }

    #[test]
    fn interactive_profile_applies_logging_and_input_sound() {
        let profile = prompts::InitActionProfile {
            logging: prompts::LoggingProfile::All {
                target: LogTarget::File {
                    path: None,
                    rotate_daily: true,
                },
            },
            input_required_actions: vec![HookAction::SoundEffect {
                name: recommended_sound(&AgenticEvent::HumanInTheLoop).to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
        };

        let bindings = build_provider_event_bindings(Provider::Claude, &profile);

        // Non-input event gets logging only.
        let session_start = bindings
            .get(&AgenticEvent::SessionStart)
            .expect("session_start should exist for Claude");
        assert_eq!(session_start.actions.len(), 1);
        assert!(matches!(session_start.actions[0], HookAction::Log { .. }));

        // Input-required event gets logging + sound.
        let hitl = bindings
            .get(&AgenticEvent::HumanInTheLoop)
            .expect("human_in_the_loop should exist for Claude");
        assert_eq!(hitl.actions.len(), 2);
        assert!(matches!(hitl.actions[0], HookAction::Log { .. }));
        assert!(matches!(hitl.actions[1], HookAction::SoundEffect { .. }));
    }

    #[test]
    fn interactive_profile_logs_only_selected_events_when_configured() {
        let profile = prompts::InitActionProfile {
            logging: prompts::LoggingProfile::Some {
                target: LogTarget::File {
                    path: None,
                    rotate_daily: true,
                },
                events: [AgenticEvent::TurnComplete].into_iter().collect(),
            },
            input_required_actions: vec![],
        };

        let bindings = build_provider_event_bindings(Provider::Claude, &profile);

        let turn_complete = bindings
            .get(&AgenticEvent::TurnComplete)
            .expect("turn_complete should exist for Claude");
        assert_eq!(turn_complete.actions.len(), 1);
        assert!(matches!(turn_complete.actions[0], HookAction::Log { .. }));

        let session_start = bindings
            .get(&AgenticEvent::SessionStart)
            .expect("session_start should exist for Claude");
        assert!(session_start.actions.is_empty());
    }

    #[test]
    fn quick_mode_keeps_all_hook_events_and_adds_default_sounds() {
        let events = create_quick_provider_events(Provider::Gemini);

        // All hook-supported Gemini events should be present.
        let expected: Vec<AgenticEvent> = AgenticEvent::ALL
            .into_iter()
            .filter(|event| Provider::Gemini.supports_event_via_hook(event))
            .collect();
        assert_eq!(events.len(), expected.len());

        // turn_complete is a default sound event in quick mode.
        let turn_complete = events
            .get(&AgenticEvent::TurnComplete)
            .expect("turn_complete should be configured");
        assert!(
            turn_complete
                .actions
                .iter()
                .any(|action| matches!(action, HookAction::SoundEffect { .. }))
        );

        // before_model should still be present but no-op by default.
        let before_model = events
            .get(&AgenticEvent::BeforeModel)
            .expect("before_model should be configured");
        assert!(before_model.actions.is_empty());
    }

    #[test]
    fn quick_mode_seeds_provider_aware_protect_defaults() {
        let config = default_config(false).expect("default config should build");
        let protect = config.settings.protect.expect("protect defaults missing");

        assert_eq!(protect.posture, ProtectPosture::Balanced);
    }
}
