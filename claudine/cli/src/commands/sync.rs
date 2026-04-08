use std::collections::HashMap;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::utils::layout::Margin;
use clap::Args;
use color_eyre::eyre::Result;

use claudine::config::claudine_config::ClaudineConfig;
use claudine::config::{
    ProviderHookPlan, RegistrationResult, SkipReason, detect_agents, get_configurator,
};
use claudine::dispatch::loader::load_claudine_config;
use claudine::events::Provider;

use crate::cli_utils::event_name_pascal;
use crate::log;
use crate::provider_values::provider_value_parser;

/// Arguments for the sync subcommand.
#[derive(Args)]
pub struct SyncArgs {
    /// Show what would change without writing.
    #[arg(long)]
    pub dry_run: bool,
    /// Sync only a specific provider.
    #[arg(long, value_parser = provider_value_parser())]
    pub provider: Option<Provider>,
    /// Remove unsupported events from config.
    ///
    /// When enabled, removes event bindings from providers that don't support
    /// those events via hooks. This cleans up the config file to eliminate
    /// "These events won't fire" warnings.
    #[arg(long)]
    pub fix: bool,
}

/// Action taken for a hook during sync.
#[derive(Debug)]
enum SyncAction {
    /// Hook was added to the provider config.
    Added(String),
    /// Stale hook was removed from provider config.
    RemovedStale(String),
    /// Provider has no hook support.
    NoSupport,
    /// Provider requires wrapper/proxy approach.
    WrapperOnly(String),
    /// Provider not detected on system.
    NotDetected,
    /// Already up-to-date, no changes.
    UpToDate,
    /// Registration was repaired (same events, but config/wrapper fixed).
    Repaired,
    /// Would add hook (dry run).
    WouldAdd(String),
    /// Would remove stale hook (dry run).
    WouldRemoveStale(String),
    /// Would deregister (dry run).
    WouldDeregister,
    /// Deregistered all hooks.
    Deregistered,
    /// Error occurred.
    Error(String),
}

/// Format a prose for an added hook.
fn prose_added(hook: &str) -> Prose {
    Prose::new(format!(
        "<i>added</i> the hook <inverse> {} </inverse>",
        event_name_pascal(hook)
    ))
}

/// Format a prose for a removed stale hook.
fn prose_removed_stale(hook: &str) -> Prose {
    Prose::new(format!(
        "<i>removed</i> the <b>stale</b> hook <inverse> {} </inverse>",
        event_name_pascal(hook)
    ))
}

/// Format a prose for no hook support.
fn prose_no_support() -> Prose {
    Prose::new("<dim>currently no support for hooks</dim>")
}

/// Format a prose for wrapper-only providers.
fn prose_wrapper_only(guidance: &str) -> Prose {
    Prose::new(format!("<dim>requires wrapper: <i>{}</i></dim>", guidance))
}

/// Format a prose for not detected providers.
fn prose_not_detected() -> Prose {
    Prose::new("<dim>not detected on system</dim>")
}

/// Format a prose for already up-to-date.
fn prose_up_to_date() -> Prose {
    Prose::new("<dim>already up-to-date</dim>")
}

/// Format a prose for repaired registration.
fn prose_repaired() -> Prose {
    Prose::new("<i>repaired</i> hook registration")
}

/// Format a prose for dry-run would add.
fn prose_would_add(hook: &str) -> Prose {
    Prose::new(format!(
        "<dim>would add hook</dim> <inverse> {} </inverse>",
        event_name_pascal(hook)
    ))
}

/// Format a prose for dry-run would remove stale.
fn prose_would_remove_stale(hook: &str) -> Prose {
    Prose::new(format!(
        "<dim>would remove stale hook</dim> <inverse> {} </inverse>",
        event_name_pascal(hook)
    ))
}

/// Format a prose for dry-run would deregister.
fn prose_would_deregister() -> Prose {
    Prose::new("<dim>would deregister all hooks</dim>")
}

/// Format a prose for deregistered.
fn prose_deregistered() -> Prose {
    Prose::new("<i>deregistered</i> all hooks")
}

/// Format a prose for errors.
fn prose_error(msg: &str) -> Prose {
    Prose::new(format!("<red><b>error:</b> {}</red>", msg))
}

/// Build a provider section with its actions.
fn build_provider_section(provider: Provider, actions: Vec<SyncAction>) -> UnorderedList {
    let mut items: Vec<RenderableContent> = vec![];

    // Provider header
    let header = Prose::new(format!("<b><blue>{}</blue></b>", provider));
    items.push(RenderableContent::from(header));

    // Build action list
    let action_proses: Vec<RenderableContent> = actions
        .into_iter()
        .map(|action| {
            let prose = match action {
                SyncAction::Added(hook) => prose_added(&hook),
                SyncAction::RemovedStale(hook) => prose_removed_stale(&hook),
                SyncAction::NoSupport => prose_no_support(),
                SyncAction::WrapperOnly(guidance) => prose_wrapper_only(&guidance),
                SyncAction::NotDetected => prose_not_detected(),
                SyncAction::UpToDate => prose_up_to_date(),
                SyncAction::Repaired => prose_repaired(),
                SyncAction::WouldAdd(hook) => prose_would_add(&hook),
                SyncAction::WouldRemoveStale(hook) => prose_would_remove_stale(&hook),
                SyncAction::WouldDeregister => prose_would_deregister(),
                SyncAction::Deregistered => prose_deregistered(),
                SyncAction::Error(msg) => prose_error(&msg),
            };
            RenderableContent::from(prose)
        })
        .collect();

    if !action_proses.is_empty() {
        let action_list = UnorderedList::from(action_proses).with_bullet("  ◦ ");
        items.push(RenderableContent::from(action_list));
    }

    let mut list = UnorderedList::from(items).with_bullet("• ");
    list.layout_mut().top_margin = Margin::Chars(1);
    list
}

/// Get expected events for a provider from ClaudineConfig.
///
/// Returns events that are in the config's actions map AND supported
/// by this provider via hook registration.
fn expected_events(config: &ClaudineConfig, provider: Provider) -> Vec<String> {
    config
        .actions
        .keys()
        .filter(|event| provider.supports_event_via_hook(event))
        .map(|event| event.as_slug().to_string())
        .collect()
}

/// Get expected events as typed `AgenticEvent` values for building a `ProviderHookPlan`.
fn expected_events_typed(
    config: &ClaudineConfig,
    provider: Provider,
) -> Vec<claudine::events::AgenticEvent> {
    config
        .actions
        .keys()
        .filter(|event| provider.supports_event_via_hook(event))
        .copied()
        .collect()
}

/// Re-sync hook registrations with detected agents.
pub async fn run(args: SyncArgs) -> Result<()> {
    let term = crate::log::terminal();

    // Load current config from user/repo locations
    // If config is missing, treat as "remove all hooks" operation
    let config = match load_claudine_config(None, None) {
        Ok(cfg) => Some(cfg),
        Err(claudine::error::ClaudineError::ConfigNotFound(_)) => None,
        Err(e) => return Err(e.into()),
    };

    let filter_provider = args.provider;

    // Get detected agents for registration
    let detected = detect_agents();
    let detected_providers: Vec<Provider> = detected.iter().map(|(p, _)| *p).collect();

    // Collect all actions by provider
    let mut provider_actions: HashMap<Provider, Vec<SyncAction>> = HashMap::new();

    for &provider in &detected_providers {
        if let Some(ref filter) = filter_provider
            && provider != *filter
        {
            continue;
        }

        let configurator = get_configurator(provider);
        let actions = provider_actions.entry(provider).or_default();

        // Build a ProviderHookPlan for this provider from the config
        let plan = config.as_ref().map(|cfg| ProviderHookPlan {
            events: expected_events_typed(cfg, provider),
            canonical_for: None,
        });

        // When config is None, deregister (remove all claudine hooks)
        // When config is Some, register/sync hooks
        match (&config, &plan) {
            (None, _) => {
                // Config removed - deregister from all providers
                if args.dry_run {
                    let registered = configurator.is_registered(None).unwrap_or(false);
                    if registered {
                        actions.push(SyncAction::WouldDeregister);
                    } else {
                        actions.push(SyncAction::UpToDate);
                    }
                } else {
                    match configurator.deregister(None) {
                        Ok(()) => {
                            actions.push(SyncAction::Deregistered);
                        }
                        Err(e) => {
                            actions.push(SyncAction::Error(e.to_string()));
                        }
                    }
                }
            }
            (Some(cfg), Some(hook_plan)) => {
                if args.dry_run {
                    // For dry run, show what would happen
                    let registered_events =
                        configurator.registered_events(None).unwrap_or_default();
                    let expected = expected_events(cfg, provider);

                    // Find events to add
                    for event in &expected {
                        if !registered_events.contains(event) {
                            actions.push(SyncAction::WouldAdd(event.clone()));
                        }
                    }

                    // Find stale events to remove
                    for event in &registered_events {
                        if !expected.contains(event) {
                            actions.push(SyncAction::WouldRemoveStale(event.clone()));
                        }
                    }

                    if actions.is_empty() {
                        actions.push(SyncAction::UpToDate);
                    }
                } else {
                    // Get current state before sync
                    let was_registered = configurator.is_registered(None).unwrap_or(false);
                    let before_events = configurator.registered_events(None).unwrap_or_default();

                    match configurator.register(hook_plan, None) {
                        Ok(RegistrationResult::Registered { event_count: _ }) => {
                            // Get events after sync
                            let after_events =
                                configurator.registered_events(None).unwrap_or_default();

                            // Determine what was added
                            for event in &after_events {
                                if !before_events.contains(event) {
                                    actions.push(SyncAction::Added(event.clone()));
                                }
                            }

                            // Determine what was removed (stale)
                            for event in &before_events {
                                if !after_events.contains(event) {
                                    actions.push(SyncAction::RemovedStale(event.clone()));
                                }
                            }

                            // If no event changes but register() ran (didn't skip),
                            // it was a repair (e.g. wrapper script recreated).
                            if actions.is_empty() {
                                if was_registered {
                                    actions.push(SyncAction::Repaired);
                                } else {
                                    // Fresh registration with same events shouldn't happen,
                                    // but handle gracefully
                                    for event in &after_events {
                                        actions.push(SyncAction::Added(event.clone()));
                                    }
                                    if actions.is_empty() {
                                        actions.push(SyncAction::UpToDate);
                                    }
                                }
                            }
                        }
                        Ok(RegistrationResult::Skipped(reason)) => match reason {
                            SkipReason::AlreadyRegistered => {
                                actions.push(SyncAction::UpToDate);
                            }
                            SkipReason::WrapperOnly { guidance } => {
                                actions.push(SyncAction::WrapperOnly(guidance));
                            }
                            SkipReason::NotDetected => {
                                actions.push(SyncAction::NotDetected);
                            }
                            SkipReason::NoHookSupport => {
                                actions.push(SyncAction::NoSupport);
                            }
                        },
                        Err(e) => {
                            actions.push(SyncAction::Error(e.to_string()));
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    // Render the output
    if args.dry_run {
        log::data("");
        let header = Prose::new("<b><yellow>Dry run</yellow></b> - no changes will be made");
        log::data(&header.render(&term));
    }

    for (provider, actions) in provider_actions {
        let section = build_provider_section(provider, actions);
        log::data(&section.render(&term));
    }

    log::data("");

    // With ClaudineConfig, events are provider-agnostic — every event applies
    // to all providers. Events that a provider doesn't support simply won't
    // fire for that provider. This is expected, not a configuration error.
    // The --fix flag is preserved for CLI compatibility but is not actionable.

    Ok(())
}
