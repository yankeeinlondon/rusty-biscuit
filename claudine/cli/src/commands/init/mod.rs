//! Interactive init wizard for claudine configuration.
//!
//! This module is compiled only for tests (`#[cfg(test)] pub mod init;`
//! in `commands/mod.rs`). The production wizard lives in
//! `init_wizard`. Helpers below are retained alongside the tests for
//! reference / documentation purposes, so `dead_code` is allowed here.
#![allow(dead_code)]

mod prompts;

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::Result;

use claudine::actions::HookAction;
use claudine::config::claudine_config::{ClaudineConfig, DefaultSounds, TtsValue};
use claudine::config::{
    AgentInfo, ProviderHookPlan, RegistrationResult, SkipReason, discover_agents_full,
    get_configurator,
};
use claudine::dispatch::loader::{load_claudine_config, save_claudine_config};
use claudine::events::{AgenticEvent, CanonicalProviderSettings, EventBinding, recommended_sound};
use claudine::linking::{
    CanonicalSelection, LinkableResource, ResourceScope, ranked_provider_preferences,
    resolve_repo_root, select_canonical_provider, set_canonical_provider,
};
use claudine::protect::config::ProtectConfig;
use claudine::provider::Provider;

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

    let cwd = std::env::current_dir()?;
    let repo_root = if repo_scope {
        resolve_repo_root(&cwd)
    } else {
        cwd.clone()
    };
    let defaults = load_init_defaults(repo_scope, &repo_root);

    // Phase 0: Check for existing global config in repo mode
    if repo_scope {
        let global_config = dirs::home_dir()
            .map(|h| h.join(".claudine").join("config.json"))
            .unwrap_or_else(|| PathBuf::from("~/.claudine/config.json"));

        if !global_config.exists() {
            log::warn("No global ~/.claudine/config.json found.");
            log::message(
                "Run any claudine command to trigger initialization, or run `claudine config`.",
            );
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
        log::warn("No agents detected on this host.");
        log::message("Continuing without a favorite agent — you can configure one later.");
        log::message("");
    } else {
        log::message("");
        log::message(&format!(
            "Detected {} available agent(s)",
            selected_agents.len()
        ));
        for agent in &selected_agents {
            log::message(&format!("  {}", agent.provider));
        }
        log::message("");
    }

    // Phase 2: Provider Preferences
    log::message("Phase 2: Provider Preferences");
    log::message("------------------------------");
    let installed_providers = installed_provider_list(&all_agents);
    let ranked_preferences = prompts::prompt_provider_preferences_with_defaults(
        &installed_providers,
        &defaults.provider_preferences,
    )?;
    let preference = ranked_provider_preferences(&installed_providers, &ranked_preferences);

    // Phase 3: Global Action Interview
    log::message("Phase 3: Action Defaults");
    log::message("-------------------------");
    let action_profile =
        prompts::prompt_action_profile_with_defaults(defaults.action_profile.as_ref())?;

    log::message("");
    log::message("Phase 4: Protect Defaults");
    log::message("-------------------------");
    let protect_enabled = prompts::prompt_protect_enabled(defaults.protect_enabled.or(Some(true)))?;
    let protect = if protect_enabled {
        ProtectConfig::default()
    } else {
        ProtectConfig {
            enabled: false,
            ..ProtectConfig::default()
        }
    };

    let first_agent = selected_agents.first();

    let mut actions = HashMap::new();
    if let Some(agent) = first_agent {
        let hook_events = provider_hook_events(agent.provider);
        for event in &hook_events {
            let event_actions = actions_for_event(*event, &action_profile);
            actions.insert(*event, event_actions);
        }
    }

    let total_bindings = actions.len();

    log::message("");
    log::message(&format!("Prepared {total_bindings} event binding(s):"));
    for agent in &selected_agents {
        let count = provider_hook_events(agent.provider).len();
        log::message(&format!("  {}: {} events", agent.provider, count));
    }
    log::message("");

    let canonical_provider = if repo_scope {
        None
    } else {
        let scope = ResourceScope::User;
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let mut canonical = CanonicalProviderSettings::default();
        for resource in LinkableResource::ALL {
            if let CanonicalSelection::Selected { provider, .. } = select_canonical_provider(
                scope,
                resource,
                &installed_providers,
                &preference,
                &home_dir,
                &repo_root,
            ) {
                set_canonical_provider(&mut canonical, scope, resource, provider);
            }
        }
        Some(canonical)
    };

    let config = ClaudineConfig {
        tts: TtsValue::default(),
        messenger: None,
        logging: true,
        protect,
        actions,
        matchers: HashMap::new(),
        preferred_agent: first_agent.map(|agent| agent.provider),
        canonical_provider: canonical_provider.and_then(|_| preference.first().copied()),
        models: HashMap::new(),
        default_sounds: DefaultSounds::default(),
        prompt_for_missing: true,
        harvest_unmatched: false,
        exit_expressions: None,
        guard_settings: Default::default(),
    };

    // Phase 5: Write and Register
    log::message("");
    log::message("Phase 5: Write Configuration");
    log::message("-----------------------------");

    let config_path = if repo_scope {
        repo_root.join(".claudine").join("config.json")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".claudine")
            .join("config.json")
    };

    save_claudine_config(&config, &config_path)?;
    log::message(&format!(
        "Wrote config to {}",
        biscuit_file::to_portable_string(&config_path)
    ));

    // Handle .gitignore in repo mode
    if repo_scope {
        handle_repo_gitignore(&repo_root)?;
    }

    // Register with agents
    log::message("");
    log::message("Registering with available agents:");
    for agent in &selected_agents {
        let provider = agent.provider;
        let plan = ProviderHookPlan {
            events: config.actions.keys().copied().collect(),
            canonical_for: None,
        };
        let configurator = get_configurator(provider);
        match configurator.register(&plan, None) {
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

#[derive(Debug, Clone, Default)]
struct InitDefaults {
    provider_preferences: Vec<Provider>,
    action_profile: Option<prompts::InitActionProfile>,
    protect_enabled: Option<bool>,
}

fn load_init_defaults(repo_scope: bool, repo_root: &std::path::Path) -> InitDefaults {
    let Some(config) = load_existing_init_config(repo_scope, repo_root) else {
        return InitDefaults::default();
    };

    InitDefaults {
        provider_preferences: vec![],
        action_profile: infer_action_profile(&config),
        protect_enabled: Some(config.protect.enabled),
    }
}

fn load_existing_init_config(
    repo_scope: bool,
    repo_root: &std::path::Path,
) -> Option<ClaudineConfig> {
    let home = dirs::home_dir()?;
    let user_config = home.join(".claudine").join("config.json");
    let repo_config = repo_root.join(".claudine").join("config.json");

    if repo_scope {
        read_config_if_exists(&repo_config).or_else(|| read_config_if_exists(&user_config))
    } else {
        read_config_if_exists(&user_config)
    }
}

fn read_config_if_exists(path: &std::path::Path) -> Option<ClaudineConfig> {
    if !path.is_file() {
        return None;
    }
    load_claudine_config(Some(path), None).ok()
}

fn infer_action_profile(config: &ClaudineConfig) -> Option<prompts::InitActionProfile> {
    let mut configured_events = HashSet::new();
    let mut input_required_actions = Vec::new();

    for (event, actions) in &config.actions {
        if actions.is_empty() {
            continue;
        }

        configured_events.insert(*event);

        if input_required_actions.is_empty()
            && matches!(
                event,
                AgenticEvent::PermissionRequest | AgenticEvent::HumanInTheLoop
            )
        {
            input_required_actions = actions.clone();
        }
    }

    if configured_events.is_empty() && input_required_actions.is_empty() {
        return None;
    }

    Some(prompts::InitActionProfile {
        logging: prompts::LoggingProfile::None,
        input_required_actions,
    })
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

    save_claudine_config(&config, &config_path)?;
    log::message(&format!(
        "  Wrote config to {}",
        biscuit_file::to_portable_string(&config_path)
    ));

    // Register with providers
    log::message("");
    log::message("Registering with configured providers:");
    let all_agents = discover_agents_full();
    let installed_providers = installed_provider_list(&all_agents);
    let mut providers: Vec<Provider> = installed_providers
        .iter()
        .copied()
        .filter(|provider| !provider_hook_events(*provider).is_empty())
        .collect();
    providers.sort_by_key(|provider| provider.to_string());
    for provider in providers {
        let plan = ProviderHookPlan {
            events: config.actions.keys().copied().collect(),
            canonical_for: None,
        };
        let configurator = get_configurator(provider);
        match configurator.register(&plan, None) {
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

fn default_config(repo_scope: bool) -> Result<ClaudineConfig> {
    let all_agents = discover_agents_full();
    let installed_providers = installed_provider_list(&all_agents);

    let quick_providers: Vec<Provider> = installed_providers
        .iter()
        .copied()
        .filter(|provider| !provider_hook_events(*provider).is_empty())
        .collect();

    let preferred_agent = quick_providers.first().copied();

    let mut actions = HashMap::new();
    if let Some(&first_provider) = quick_providers.first() {
        for event in provider_hook_events(first_provider) {
            let event_actions = if matches!(
                event,
                AgenticEvent::SessionStart
                    | AgenticEvent::TurnComplete
                    | AgenticEvent::ToolError
                    | AgenticEvent::PermissionRequest
                    | AgenticEvent::HumanInTheLoop
            ) {
                vec![HookAction::SoundEffect {
                    effect: recommended_sound(&event).to_string(),
                    volume: 1.0,
                    speed: 1.0,
                    when: None,
                }]
            } else {
                vec![]
            };
            actions.insert(event, event_actions);
        }
    }

    let canonical_provider = if repo_scope {
        None
    } else {
        let cwd = std::env::current_dir()?;
        let repo_root = resolve_repo_root(&cwd);
        let scope = ResourceScope::User;
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let preference = ranked_provider_preferences(&installed_providers, &[]);
        let mut canonical = CanonicalProviderSettings::default();
        for resource in LinkableResource::ALL {
            if let CanonicalSelection::Selected { provider, .. } = select_canonical_provider(
                scope,
                resource,
                &installed_providers,
                &preference,
                &home_dir,
                &repo_root,
            ) {
                set_canonical_provider(&mut canonical, scope, resource, provider);
            }
        }
        preference.first().copied()
    };

    Ok(ClaudineConfig {
        tts: TtsValue::default(),
        messenger: None,
        logging: true,
        protect: ProtectConfig::default(),
        actions,
        matchers: HashMap::new(),
        preferred_agent,
        canonical_provider,
        models: HashMap::new(),
        default_sounds: DefaultSounds::default(),
        prompt_for_missing: true,
        harvest_unmatched: false,
        exit_expressions: None,
        guard_settings: Default::default(),
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
                    effect: recommended_sound(&event).to_string(),
                    volume: 1.0,
                    speed: 1.0,
                    when: None,
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

    fn config_with_actions(events: Vec<(AgenticEvent, Vec<HookAction>)>) -> ClaudineConfig {
        let actions = events.into_iter().collect();

        ClaudineConfig {
            tts: TtsValue::default(),
            messenger: None,
            logging: true,
            protect: ProtectConfig::default(),
            actions,
            matchers: HashMap::new(),
            preferred_agent: Some(Provider::Claude),
            canonical_provider: None,
            models: HashMap::new(),
            default_sounds: DefaultSounds::default(),
            prompt_for_missing: true,
            harvest_unmatched: false,
            exit_expressions: None,
            guard_settings: Default::default(),
        }
    }

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
    fn interactive_profile_applies_input_sound() {
        let profile = prompts::InitActionProfile {
            logging: prompts::LoggingProfile::None,
            input_required_actions: vec![HookAction::SoundEffect {
                effect: recommended_sound(&AgenticEvent::HumanInTheLoop).to_string(),
                volume: 1.0,
                speed: 1.0,
                when: None,
            }],
        };

        let bindings = build_provider_event_bindings(Provider::Claude, &profile);

        // Non-input event gets no actions.
        let session_start = bindings
            .get(&AgenticEvent::SessionStart)
            .expect("session_start should exist for Claude");
        assert!(session_start.actions.is_empty());

        // Input-required event gets sound.
        let hitl = bindings
            .get(&AgenticEvent::HumanInTheLoop)
            .expect("human_in_the_loop should exist for Claude");
        assert_eq!(hitl.actions.len(), 1);
        assert!(matches!(hitl.actions[0], HookAction::SoundEffect { .. }));
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

        assert!(config.protect.enabled);
    }

    #[test]
    fn infer_action_profile_restores_input_actions() {
        let sound = HookAction::SoundEffect {
            effect: recommended_sound(&AgenticEvent::HumanInTheLoop).to_string(),
            volume: 1.0,
            speed: 1.0,
            when: None,
        };
        let config = config_with_actions(vec![
            (AgenticEvent::SessionStart, vec![]),
            (AgenticEvent::HumanInTheLoop, vec![sound.clone()]),
        ]);

        let profile = infer_action_profile(&config).expect("profile should be inferred");
        assert!(matches!(profile.logging, prompts::LoggingProfile::None));
        assert_eq!(profile.input_required_actions, vec![sound]);
    }
}
