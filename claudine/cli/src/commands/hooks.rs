use std::collections::HashSet;

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Alignment, Margin};
use claudine::config::{detect_agents, AgentConfigurator};
use claudine::dispatch::loader::load_config;
use claudine::events::{AgenticEvent, EventSupportLevel, HookerConfig, Provider};
use sniff::programs::{enums::AiCli, InstalledAiClients};

use crate::log;

/// Arguments for the hooks command.
#[derive(Args)]
pub struct HooksArgs {
    /// Show provider event support matrix (✅ hook / ⛔️ non-hook / ❌ none)
    #[arg(long)]
    pub support: bool,

    /// Show native event name mappings for each provider
    #[arg(long)]
    pub mapping: bool,

    /// Show event descriptions and schemas
    #[arg(long)]
    pub describe: bool,
}

/// All supported providers in display order.
const ALL_PROVIDERS: [Provider; 7] = [
    Provider::Claude,
    Provider::Codex,
    Provider::Gemini,
    Provider::Goose,
    Provider::KimiCode,
    Provider::OpenCode,
    Provider::QwenCode,
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
        Provider::QwenCode => AiCli::QwenCli,
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
/// Returns events that are enabled for this specific provider AND can be
/// registered via the provider's config file.
///
/// Takes an optional configurator to check for provider-specific registerable events.
fn expected_events_for_provider(
    config: &HookerConfig,
    provider: Provider,
    configurator: Option<&dyn claudine::config::AgentConfigurator>,
) -> HashSet<String> {
    // If configurator exists and doesn't support config registration, return empty
    if let Some(cfg) = configurator {
        if !cfg.supports_config_registration() {
            return HashSet::new();
        }
    }

    config
        .providers
        .get(&provider)
        .map(|p| {
            p.events
                .iter()
                .filter(|(event, binding)| {
                    if !binding.enabled {
                        return false;
                    }

                    // Check if configurator has specific registerable events
                    if let Some(cfg) = configurator {
                        if let Some(registerable) = cfg.registerable_events() {
                            return registerable.contains(event);
                        }
                    }

                    // Default: filter by provider's hook-based event support only
                    provider.supports_event_via_hook(event)
                })
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
pub fn run(args: HooksArgs, verbose: bool) -> Result<()> {
    // Handle --support, --mapping, and --describe flags first
    if args.support {
        return run_support();
    }
    if args.mapping {
        return run_mapping();
    }
    if args.describe {
        return run_describe();
    }

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
                .map(|c| expected_events_for_provider(c, provider, configurator))
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
    let table = table.prefer_cursor_alignment();

    let rendered = table.fallback_render(&term);
    log::data(&format!("\n{}", rendered));

    // Show color legend if there are sync issues
    if has_sync_issues {
        log::data("");
        let legend = Prose::new(
            "{{dim}}- Legend: {{red}}red{{reset}}{{dim}} = stale (remove with sync), {{yellow}}orange{{reset}}{{dim}} = missing (add with sync){{reset}}",
        );
        log::data(&format!(" {}", legend.render(Some(100))));
    }

    // Show hints about available flags
    log::data("");
    let hints = [
        "{{dim}}- Use <blue><bold>-v</bold></blue>{{dim}} for detailed event matrix{{reset}}",
        "{{dim}}- Use <blue><bold>--support</bold></blue>{{dim}} to see which events each provider supports{{reset}}",
        "{{dim}}- Use <blue><bold>--mapping</bold></blue>{{dim}} to see native event name mappings{{reset}}",
        "{{dim}}- Use <blue><bold>--describe</bold></blue>{{dim}} to see event descriptions and schemas{{reset}}",
    ];
    for hint in hints {
        log::data(&format!(" {}", Prose::new(hint).render(Some(100))));
    }

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
        let mut column = TableColumn::new(event.abbrev());
        if matches!(
            event,
            AgenticEvent::SubagentStart | AgenticEvent::SubagentStop
        ) {
            column = column.with_fixed_width(4);
        }
        columns.push(column);
    }

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
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
    let rendered = table.fallback_render(&term);
    log::data(&format!("\n{}", rendered));

    // Show legend
    log::data("");
    let legend = Prose::new(
        "{{dim}}Legend: {{reset}}⚠️{{dim}} = not supported, {{reset}}-{{dim}} = not configured, {{reset}}⓪{{dim}} = 0 actions, {{reset}}❶{{dim}} = 1 action, etc.{{reset}}",
    ).with_left_margin(Margin::Chars(8));
    log::data(&format!(" {}\n", legend.fallback_render(&term)));

    // Show hints about available flags
    let hints = [
        "{{dim}}- Use <blue><bold>--support</bold></blue>{{dim}} to see which events each provider supports{{reset}}",
        "{{dim}}- Use <blue><bold>--mapping</bold></blue>{{dim}} to see native event name mappings{{reset}}",
        "{{dim}}- Use <blue><bold>--describe</bold></blue>{{dim}} to see event descriptions and schemas{{reset}}",
    ];
    for hint in hints {
        log::data(&format!(" {}", Prose::new(hint).fallback_render(&term)));
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

/// Indicator for hook-based support (config file registration).
const HOOK_SUPPORT: &str = "✅";
/// Indicator for non-hook support (wrapper/wire-mode required).
const NON_HOOK_SUPPORT: &str = "⛔️";
/// Indicator for no support (using ❌ U+274C which has width 2 like other emoji).
const NO_SUPPORT: &str = "❌";

/// Show provider event support matrix with ✅/⛔️/❌ indicators.
///
/// - ✅ Hook: Event can be registered via config file
/// - ⛔️ NonHook: Event requires wrapper/proxy (not yet implemented)
/// - ❌ NotSupported: Event is not available from this provider
fn run_support() -> Result<()> {
    let term = Terminal::new();

    // Build columns: Event name (left-aligned), then one per provider (centered)
    let mut columns = vec![TableColumn::new("Event")];
    for provider in ALL_PROVIDERS {
        columns.push(TableColumn::new(provider.to_string()).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);

    // Add a row for each event
    for event in AgenticEvent::ALL {
        let mut row: Vec<TableCellContent> = vec![event.to_string().into()];

        for provider in ALL_PROVIDERS {
            let cell = match provider.event_support_level(&event) {
                EventSupportLevel::Hook => HOOK_SUPPORT.into(),
                EventSupportLevel::NonHook => NON_HOOK_SUPPORT.into(),
                EventSupportLevel::NotSupported => NO_SUPPORT.into(),
            };
            row.push(cell);
        }

        table.add_row(row);
    }

    let rendered = table.fallback_render(&term);
    log::data(&format!("\n{}", rendered));

    // Show legend
    log::data("");
    let legend = Prose::new(
        "{{dim}}Legend: {{reset}}✅{{dim}} = hook support (config file), {{reset}}⛔️{{dim}} = non-hook (wrapper/proxy required), {{reset}}{{NO_SUPPORT}}{{dim}} = not supported{{reset}}",
    );
    log::data(&format!(" {}\n", legend.fallback_render(&term)));

    Ok(())
}

/// First group of providers for mapping table (to fit width constraints).
const MAPPING_GROUP_1: [Provider; 4] = [
    Provider::Claude,
    Provider::Codex,
    Provider::Gemini,
    Provider::OpenCode,
];

/// Second group of providers for mapping table.
const MAPPING_GROUP_2: [Provider; 3] = [Provider::Goose, Provider::KimiCode, Provider::QwenCode];

/// Show native event name mappings for each provider.
///
/// Splits providers into two tables to fit width-constrained terminals.
fn run_mapping() -> Result<()> {
    let term = Terminal::new();

    // Render first table (Claude, Codex, Gemini, Goose)
    let table1 = build_mapping_table(&MAPPING_GROUP_1).prefer_cursor_alignment();
    let rendered1 = table1.fallback_render(&term);
    log::data(&format!("\n{}", rendered1));

    // Render second table (KimiCode, OpenCode, QwenCode)
    let table2 = build_mapping_table(&MAPPING_GROUP_2).prefer_cursor_alignment();
    let rendered2 = table2.fallback_render(&term);
    log::data(&format!("\n{}", rendered2));

    // Show legend
    log::data("");
    let legend =
        Prose::new("{{dim}}- Legend: (blank) = not supported or no specific native name{{reset}}");
    log::data(&format!(" {}", legend.render(Some(100))));

    Ok(())
}

/// Build a mapping table for a subset of providers.
fn build_mapping_table(providers: &[Provider]) -> Table {
    // Build columns: Event name (left-aligned), then one per provider (center-aligned)
    let mut columns = vec![TableColumn::new("Event")];
    for provider in providers {
        columns.push(TableColumn::new(provider.to_string()).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns);
    table.layout_mut().left_margin = Margin::Chars(1);

    // Add a row for each event
    for event in AgenticEvent::ALL {
        let mut row: Vec<TableCellContent> = vec![event.to_string().into()];

        for provider in providers {
            let cell: TableCellContent = match provider.native_event_name(&event) {
                None => "".into(),         // Not supported - blank
                Some("") => "".into(),     // Supported but no specific name
                Some(name) => name.into(), // Native name
            };
            row.push(cell);
        }

        table.add_row(row);
    }

    table
}

/// Show event descriptions and schemas.
fn run_describe() -> Result<()> {
    let columns = vec![
        TableColumn::new("Event"),
        TableColumn::new("Response Schema"),
        TableColumn::new("Return Schema"),
        TableColumn::new("Description"),
    ];

    let term = Terminal::new();
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().left_margin = Margin::Chars(1);

    // Add a row for each event
    for event in AgenticEvent::ALL {
        let row: Vec<TableCellContent> = vec![
            event.to_string().into(),
            event.response_schema().into(),
            event.return_schema().into(),
            event.description().into(),
        ];
        table.add_row(row);
    }

    let rendered = table.fallback_render(&term);
    log::data(&format!("\n{}", rendered));

    // Show legend
    log::data("");
    let legend =
        Prose::new("{{dim}}- Response Schema: fields available in the event payload{{reset}}");
    log::data(&format!(" {}", legend.fallback_render(&term)));
    let legend2 = Prose::new(
        "{{dim}}- Return Schema: what hooks can return to influence agent behavior (blocking hooks only){{reset}}",
    );
    log::data(&format!(" {}", legend2.fallback_render(&term)));

    Ok(())
}
