use std::collections::HashMap;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Margin;
use clap::Args;
use color_eyre::eyre::Result;

use claudine::config::{RegistrationResult, SkipReason, detect_agents, get_configurator};
use claudine::events::{PROVIDERS_DISPLAY_ORDER, Provider};

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
        hook
    ))
}

/// Format a prose for a removed stale hook.
fn prose_removed_stale(hook: &str) -> Prose {
    Prose::new(format!(
        "<i>removed</i> the <b>stale</b> hook <inverse> {} </inverse>",
        hook
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

/// Format a prose for dry-run would add.
fn prose_would_add(hook: &str) -> Prose {
    Prose::new(format!(
        "<dim>would add hook</dim> <inverse> {} </inverse>",
        hook
    ))
}

/// Format a prose for dry-run would remove stale.
fn prose_would_remove_stale(hook: &str) -> Prose {
    Prose::new(format!(
        "<dim>would remove stale hook</dim> <inverse> {} </inverse>",
        hook
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

/// Re-sync hook registrations with detected agents.
pub async fn run(args: SyncArgs) -> Result<()> {
    let term = Terminal::new();

    // Load current config from user/repo locations
    // If config is missing, treat as "remove all hooks" operation
    let config = match claudine::dispatch::loader::load_config(None, None) {
        Ok(cfg) => Some(cfg),
        Err(claudine::error::ClaudineError::ConfigNotFound(_)) => None,
        Err(e) => return Err(e.into()),
    };

    let filter_provider = args.provider.as_deref().map(parse_provider).transpose()?;

    // Get providers to sync:
    // - If config exists, sync all providers configured in it
    // - If config is None, deregister from all detected agents
    let providers_to_sync: Vec<Provider> = match &config {
        Some(cfg) => cfg.providers.keys().copied().collect(),
        None => detect_agents().into_iter().map(|(p, _)| p).collect(),
    };

    // Collect all actions by provider
    let mut provider_actions: HashMap<Provider, Vec<SyncAction>> = HashMap::new();

    for provider in providers_to_sync {
        if let Some(ref filter) = filter_provider
            && provider != *filter
        {
            continue;
        }

        let configurator = get_configurator(provider);
        let actions = provider_actions.entry(provider).or_default();

        // When config is None, deregister (remove all claudine hooks)
        // When config is Some, register/sync hooks
        match &config {
            None => {
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
            Some(cfg) => {
                if args.dry_run {
                    // For dry run, show what would happen
                    let registered_events =
                        configurator.registered_events(None).unwrap_or_default();
                    let expected_events: Vec<String> = cfg
                        .providers
                        .get(&provider)
                        .map(|p| {
                            p.events
                                .iter()
                                .filter(|(e, b)| b.enabled && provider.supports_event_via_hook(e))
                                .map(|(e, _)| e.to_string())
                                .collect()
                        })
                        .unwrap_or_default();

                    // Find events to add
                    for event in &expected_events {
                        if !registered_events.contains(event) {
                            actions.push(SyncAction::WouldAdd(event.clone()));
                        }
                    }

                    // Find stale events to remove
                    for event in &registered_events {
                        if !expected_events.contains(event) {
                            actions.push(SyncAction::WouldRemoveStale(event.clone()));
                        }
                    }

                    if actions.is_empty() {
                        actions.push(SyncAction::UpToDate);
                    }
                } else {
                    // Get current state before sync
                    let before_events = configurator.registered_events(None).unwrap_or_default();

                    match configurator.register(cfg, None) {
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

                            // If no changes detected but event_count > 0, show up-to-date
                            if actions.is_empty() {
                                actions.push(SyncAction::UpToDate);
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
        }
    }

    // Render the output
    if args.dry_run {
        log::data("");
        let header = Prose::new("<b><yellow>Dry run</yellow></b> - no changes will be made");
        log::data(&header.fallback_render(&term));
    }

    for (provider, actions) in provider_actions {
        let section = build_provider_section(provider, actions);
        log::data(&section.fallback_render(&term));
    }

    log::data("");

    // Check for unsupported events in config
    if let Some(cfg) = &config {
        let mut unsupported_warnings: Vec<(Provider, Vec<String>)> = Vec::new();

        for (provider, provider_config) in &cfg.providers {
            let unsupported: Vec<String> = provider_config
                .events
                .iter()
                .filter(|(event, binding)| {
                    binding.enabled && !provider.supports_event_via_hook(event)
                })
                .map(|(event, _)| event.to_string())
                .collect();

            if !unsupported.is_empty() {
                unsupported_warnings.push((*provider, unsupported));
            }
        }

        if !unsupported_warnings.is_empty() {
            if args.fix && !args.dry_run {
                // Fix: Remove unsupported events and save config
                let mut fixed_config = cfg.clone();
                let removed =
                    claudine::dispatch::loader::remove_unsupported_events(&mut fixed_config);

                // Save the fixed config
                match claudine::dispatch::loader::save_config(&fixed_config) {
                    Ok(path) => {
                        let header = Prose::new(
                            "<green><b>Fixed:</b></green> Removed unsupported events from config:",
                        );
                        log::data(&format!("✓ {}", header.fallback_render(&term)));
                        log::data("");

                        for (provider, events) in &removed {
                            let events_str = events.join(", ");
                            let line = Prose::new(format!(
                                "  <b>{}</b>: <dim><strikethrough>{}</strikethrough></dim>",
                                provider, events_str
                            ));
                            log::data(&line.fallback_render(&term));
                        }

                        log::data("");
                        let saved_msg =
                            Prose::new(format!("<dim>Config saved to {}</dim>", path.display()));
                        log::data(&saved_msg.fallback_render(&term));
                        log::data("");
                    }
                    Err(e) => {
                        let error_msg = Prose::new(format!(
                            "<red><b>Error:</b></red> Failed to save config: {}",
                            e
                        ));
                        log::data(&error_msg.fallback_render(&term));
                        log::data("");
                    }
                }
            } else if args.fix && args.dry_run {
                // Dry run: Show what would be removed
                let header = Prose::new(
                    "<yellow><b>Would fix:</b></yellow> These unsupported events would be removed:",
                );
                log::data(&format!("⚠ {}", header.fallback_render(&term)));
                log::data("");

                for (provider, events) in &unsupported_warnings {
                    let events_str = events.join(", ");
                    let line = Prose::new(format!(
                        "  <b>{}</b>: <dim><strikethrough>{}</strikethrough></dim>",
                        provider, events_str
                    ));
                    log::data(&line.fallback_render(&term));
                }

                log::data("");
            } else {
                // No fix: Show warning
                let warning_header = Prose::new(
                    "<yellow><b>Warning:</b></yellow> Some configured events are not supported by their providers:",
                );
                log::data(&format!("⚠ {}", warning_header.fallback_render(&term)));
                log::data("");

                for (provider, events) in &unsupported_warnings {
                    let events_str = events.join(", ");
                    let line = Prose::new(format!(
                        "  <b>{}</b>: <red><strikethrough>{}</strikethrough></red>",
                        provider, events_str
                    ));
                    log::data(&line.fallback_render(&term));
                }

                log::data("");
                let hint = Prose::new(
                    "<dim>These events won't fire. Use --fix to remove them, or edit ~/.hooker manually.</dim>",
                );
                log::data(&hint.fallback_render(&term));
                log::data("");
            }
        }
    }

    Ok(())
}

fn parse_provider(name: &str) -> color_eyre::eyre::Result<Provider> {
    if let Some(provider) = Provider::parse_cli_name(name) {
        return Ok(provider);
    }

    let supported = PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(Provider::as_slug)
        .collect::<Vec<_>>()
        .join(", ");
    color_eyre::eyre::bail!("Unknown provider: {name}. Supported: {supported}")
}
