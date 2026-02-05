use std::collections::HashSet;

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::Margin;
use claudine::config::{AgentConfigurator, detect_agents};
use claudine::dispatch::loader::load_config;
use claudine::events::{HookerConfig, Provider};
use sniff::programs::{enums::AiCli, InstalledAiClients};

use crate::log;

/// Arguments for the status command.
#[derive(Args)]
pub struct StatusArgs {}

/// All supported providers in display order.
const ALL_PROVIDERS: [Provider; 6] = [
    Provider::Claude,
    Provider::Codex,
    Provider::Gemini,
    Provider::Goose,
    Provider::KimiCode,
    Provider::OpenCode,
];

/// Map a claudine `Provider` to the corresponding sniff `AiCli` variant.
fn provider_to_ai_cli(provider: Provider) -> AiCli {
    match provider {
        Provider::Claude => AiCli::Claude,
        Provider::Codex => AiCli::Codex,
        Provider::Gemini => AiCli::GeminiCli,
        Provider::Goose => AiCli::Goose,
        Provider::KimiCode => AiCli::KimiCli,
        Provider::OpenCode => AiCli::Opencode,
    }
}

fn bool_indicator(value: bool) -> TableCellContent {
    if value {
        "\u{2705}".into()
    } else {
        "\u{274C}".into()
    }
}

/// Get the expected events for a provider from the claudine config.
///
/// Returns events that are enabled for this provider (considering overrides).
fn expected_events_for_provider(config: &HookerConfig, provider: Provider) -> HashSet<String> {
    let mut expected = HashSet::new();

    for (event, binding) in &config.events {
        // Check if this binding is enabled for this provider
        let enabled = if let Some(override_) = binding.overrides.get(&provider) {
            override_.enabled
        } else {
            binding.enabled
        };

        if enabled {
            expected.insert(event.to_string());
        }
    }

    expected
}

/// Format an event with color based on sync status.
///
/// - RED: registered but not in config (stale)
/// - ORANGE: in config but not registered (missing)
/// - Normal: in sync
fn format_event_with_color(event: &str, is_stale: bool, is_missing: bool) -> String {
    if is_stale {
        format!("{{{{red}}}}{event}{{{{reset}}}}")
    } else if is_missing {
        format!("{{{{yellow}}}}{event}{{{{reset}}}}")
    } else {
        event.to_string()
    }
}

/// Show registration status for all providers.
pub fn run(_args: StatusArgs, verbose: bool) -> Result<()> {
    let agents = detect_agents();
    let clients = InstalledAiClients::new();

    // Try to load claudine config for sync checking
    let config = load_config(None, None).ok();

    let hooks_column = if verbose {
        "Subscribed Events"
    } else {
        "Hooks Registered"
    };
    let mut table = Table::new().with_columns(vec![
        TableColumn::new("Provider"),
        TableColumn::new("Installed"),
        TableColumn::new(hooks_column),
    ]);
    table.layout_mut().left_margin = Margin::Chars(1);

    let mut has_sync_issues = false;

    for provider in ALL_PROVIDERS {
        let installed = clients.is_installed(provider_to_ai_cli(provider));
        let configurator = find_configurator(&agents, provider);

        let hooks_cell: TableCellContent = if verbose {
            // Only show sync info for INSTALLED providers
            if !installed {
                "-".into()
            } else {
                // Get registered events from provider config
                let registered: HashSet<String> = configurator
                    .and_then(|cfg| cfg.registered_events(None).ok())
                    .unwrap_or_default()
                    .into_iter()
                    .collect();

                // Get expected events from claudine config for this provider
                let expected: HashSet<String> = config
                    .as_ref()
                    .map(|c| expected_events_for_provider(c, provider))
                    .unwrap_or_default();

                if registered.is_empty() && expected.is_empty() {
                    "-".into()
                } else {
                    // Combine all events and sort
                    let mut all_events: Vec<&String> = registered.union(&expected).collect();
                    all_events.sort();

                    let formatted: Vec<String> = all_events
                        .into_iter()
                        .map(|event| {
                            let is_stale =
                                registered.contains(event) && !expected.contains(event);
                            let is_missing =
                                expected.contains(event) && !registered.contains(event);
                            if is_stale || is_missing {
                                has_sync_issues = true;
                            }
                            format_event_with_color(event, is_stale, is_missing)
                        })
                        .collect();

                    // Use Prose to render the colored output
                    let text = formatted.join(", ");
                    Prose::new(text).render(None).into()
                }
            }
        } else {
            // Show checkmark/X indicator (only if installed)
            if !installed {
                "-".into()
            } else {
                let registered = configurator
                    .and_then(|cfg| cfg.is_registered(None).ok())
                    .unwrap_or(false);
                bool_indicator(registered)
            }
        };

        table.add_row(vec![
            provider.to_string().into(),
            bool_indicator(installed),
            hooks_cell,
        ]);
    }

    log::data(&format!("\n{}", table.render(Some(100))));

    // Show color legend in verbose mode if there are sync issues
    if verbose && has_sync_issues {
        log::data("");
        let legend = Prose::new(
            "{{dim}}Legend: {{red}}red{{reset}}{{dim}} = stale (remove with sync), {{yellow}}orange{{reset}}{{dim}} = missing (add with sync){{reset}}"
        );
        log::data(&format!(" {}", legend.render(Some(100))));
    }

    // Show hint for verbose mode (only when not in verbose mode)
    if !verbose {
        let hint =
            Prose::new("{{dim}}- use {{bold}}{{blue}}-v{{reset}}{{dim}} flag for more information");
        log::data(&format!(" {}", hint.render(Some(100))));
    }

    Ok(())
}

/// Find the configurator for a given provider, if detected.
fn find_configurator(
    agents: &[(Provider, Box<dyn AgentConfigurator>)],
    provider: Provider,
) -> Option<&dyn AgentConfigurator> {
    agents
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, cfg)| cfg.as_ref())
}
