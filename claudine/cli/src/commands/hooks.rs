use std::collections::HashSet;

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Margin;
use claudine::config::{detect_agents, AgentConfigurator};
use claudine::dispatch::loader::load_config;
use claudine::events::{AgenticEvent, HookerConfig, Provider};
use sniff::programs::{enums::AiCli, InstalledAiClients};

use crate::log;

/// Arguments for the hooks command.
#[derive(Args)]
pub struct HooksArgs {}

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
/// Returns events that are enabled for this specific provider.
fn expected_events_for_provider(config: &HookerConfig, provider: Provider) -> HashSet<String> {
    config
        .providers
        .get(&provider)
        .map(|p| {
            p.events
                .iter()
                .filter(|(_, binding)| binding.enabled)
                .map(|(event, _)| event.to_string())
                .collect()
        })
        .unwrap_or_default()
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

/// Get action count indicator with circled number.
fn action_count_indicator(count: usize) -> &'static str {
    match count {
        0 => "⓪",
        1 => "❶",
        2 => "❷",
        3 => "❸",
        4 => "❹",
        5 => "❺",
        6 => "❻",
        7 => "❼",
        8 => "❽",
        9 => "❾",
        _ => "❿",
    }
}

/// Show registered hooks for all providers.
pub fn run(_args: HooksArgs, verbose: bool) -> Result<()> {
    let agents = detect_agents();
    let clients = InstalledAiClients::new();

    // Try to load claudine config for sync checking
    let config = load_config(None, None).ok();

    if verbose {
        run_verbose(&agents, &clients, config.as_ref())
    } else {
        run_simple(&agents, &clients, config.as_ref())
    }
}

/// Simple table view showing provider, installed status, and subscribed hooks list.
fn run_simple(
    agents: &[(Provider, Box<dyn AgentConfigurator>)],
    clients: &InstalledAiClients,
    config: Option<&HookerConfig>,
) -> Result<()> {
    let mut table = Table::new().with_columns(vec![
        TableColumn::new("Provider"),
        TableColumn::new("Installed"),
        TableColumn::new("Subscribed Hooks"),
    ]);
    table.layout_mut().left_margin = Margin::Chars(1);

    let mut has_sync_issues = false;

    for provider in ALL_PROVIDERS {
        let installed = clients.is_installed(provider_to_ai_cli(provider));
        let configurator = find_configurator(agents, provider);

        let hooks_cell: TableCellContent = if !installed {
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
                        let is_stale = registered.contains(event) && !expected.contains(event);
                        let is_missing = expected.contains(event) && !registered.contains(event);
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
        };

        // Create OSC8 hyperlink for provider name
        let provider_link = format!(r#"<a href="{}">{}</a>"#, provider.docs_url(), provider);
        let provider_cell: TableCellContent = Prose::new(provider_link).render(None).into();

        table.add_row(vec![provider_cell, bool_indicator(installed), hooks_cell]);
    }

    let term = Terminal::new();
    let rendered = if term.is_tty {
        table.render_with_cursor_alignment(100)
    } else {
        table.render(Some(100))
    };
    log::data(&format!("\n{}", rendered));

    // Show color legend if there are sync issues
    if has_sync_issues {
        log::data("");
        let legend = Prose::new(
            "{{dim}}- Legend: {{red}}red{{reset}}{{dim}} = stale (remove with sync), {{yellow}}orange{{reset}}{{dim}} = missing (add with sync){{reset}}"
        );
        log::data(&format!(" {}", legend.render(Some(100))));
    }

    // Show hint about verbose mode
    log::data("");
    let hint = Prose::new(
        "{{dim}}- Use <blue><bold>-v</bold></blue>{{dim}} flag for detailed event matrix{{reset}}",
    );
    log::data(&format!(" {}", hint.render(Some(100))));

    Ok(())
}

const NOT_INSTALLED: &'static str = "-";
const NOT_ALLOWED: &'static str = "⚠️";

/// Verbose table view showing per-event action counts in a matrix.
fn run_verbose(
    agents: &[(Provider, Box<dyn AgentConfigurator>)],
    clients: &InstalledAiClients,
    config: Option<&HookerConfig>,
) -> Result<()> {
    // Build columns: Provider, ∃ (exists/installed), then one per event
    let mut columns = vec![
        TableColumn::new("Provider"),
        TableColumn::new("∃"), // existence symbol for installed
    ];

    // Add a column for each event using abbreviations
    for event in AgenticEvent::ALL {
        columns.push(TableColumn::new(event.abbrev()));
    }

    let mut table = Table::new().with_columns(columns);
    table.layout_mut().left_margin = Margin::Chars(1);

    for provider in ALL_PROVIDERS {
        let installed = clients.is_installed(provider_to_ai_cli(provider));
        let _configurator = find_configurator(agents, provider);

        // Create OSC8 hyperlink for provider name
        let provider_link = format!(r#"<a href="{}">{}</a>"#, provider.docs_url(), provider);
        let provider_cell: TableCellContent = Prose::new(provider_link).render(None).into();

        let mut row: Vec<TableCellContent> = vec![provider_cell, bool_indicator(installed)];

        // Add cell for each event
        for event in AgenticEvent::ALL {
            let cell = if !installed {
                NOT_INSTALLED.into()
            } else if !provider.supports_event(&event) {
                // Event not supported by this provider
                NOT_ALLOWED.into()
            } else {
                // Check if event is configured and get action count
                let binding = config.and_then(|c| c.get_binding(provider, &event));
                match binding {
                    None => NOT_INSTALLED.into(),
                    Some(b) if !b.enabled => NOT_INSTALLED.into(),
                    Some(b) => action_count_indicator(b.actions.len()).into(),
                }
            };
            row.push(cell);
        }

        table.add_row(row);
    }

    let term = Terminal::new();
    let rendered = if term.is_tty {
        table.render_with_cursor_alignment(160)
    } else {
        table.render(Some(160))
    };
    log::data(&format!("\n{}", rendered));

    // Show legend
    log::data("");
    let legend = Prose::new(
        "{{dim}}- Legend: {{reset}}⚠️{{dim}} = not supported, {{reset}}-{{dim}} = not configured, {{reset}}⓪{{dim}} = 0 actions, {{reset}}❶{{dim}} = 1 action, etc.{{reset}}"
    );
    log::data(&format!(" {}", legend.render(Some(160))));

    // Show hint about provider-specific details
    let hint = Prose::new(
        "{{dim}}- Add a provider name for greater details on a given provider: <blue><bold>claudine</bold> hooks <italic>provider</italic></blue>"
    );
    log::data(&format!(" {}", hint.render(Some(160))));

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
