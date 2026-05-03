use std::collections::HashSet;

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Alignment, Margin, WordWrap};
use claudine::actions::{HookAction, ReportFormat};
use claudine::config::claudine_config::ClaudineConfig;
use claudine::config::{AgentConfigurator, detect_agents};
use claudine::dispatch::loader::load_claudine_config;
use claudine::dispatch::template::{TemplateVariable, VariableCategory};
use claudine::events::{
    AgenticEvent, EventMeta, NativeEventName, detect_environment, event_native_mapping_matrix,
    event_support_matrix,
};
use claudine::provider::{EventSupportLevel, PROVIDERS_DISPLAY_ORDER, Provider};
use playa::SoundEffect;
use sniff::programs::InstalledAiClients;

use crate::cli_utils::{bool_indicator, event_name_pascal};
use crate::log;
use crate::provider_values::provider_value_parser;

/// Arguments for the hooks command.
#[derive(Args)]
pub struct HooksArgs {
    /// Optional provider name for detailed view (fuzzy matching supported)
    #[arg(value_name = "PROVIDER", value_parser = provider_value_parser())]
    pub provider: Option<Provider>,

    /// Show provider event support matrix (✅ hook / ⛔️ non-hook / ❌ none)
    #[arg(long)]
    pub support: bool,

    /// Show native event name mappings for each provider
    #[arg(long)]
    pub mapping: bool,

    /// Show event descriptions and schemas
    #[arg(long)]
    pub describe: bool,

    /// Show available template variables for speak/report actions
    #[arg(long)]
    pub variables: bool,

    /// Show per-event capture method (hook / non-hook / acp / -) for each provider
    #[arg(long = "capture-method")]
    pub capture_method: bool,
}

/// All supported providers in display order.
const ALL_PROVIDERS: [Provider; 8] = PROVIDERS_DISPLAY_ORDER;
/// Keep provider names from wrapping into multi-line labels in narrow terminals.
const PROVIDER_COLUMN_MIN_WIDTH: usize = 11;

/// Wrap a header label in bold styling via Prose.
fn bold(label: &str) -> String {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable;
    Prose::new(format!("<bold>{label}</bold>")).render(&crate::log::optimistic_terminal(None))
}

fn provider_column() -> TableColumn {
    TableColumn::new(bold("Provider"))
        .with_min_width(PROVIDER_COLUMN_MIN_WIDTH)
        .with_word_wrap(WordWrap::None)
}

/// Get the expected events for a provider from the ClaudineConfig.
///
/// Returns events that are in the config's actions map AND can be
/// registered via the provider's config file.
///
/// Takes an optional configurator to check for provider-specific registerable events.
fn expected_events_for_provider(
    config: &ClaudineConfig,
    provider: Provider,
    configurator: Option<&dyn AgentConfigurator>,
) -> HashSet<String> {
    // If configurator exists and doesn't support config registration, return empty
    if let Some(cfg) = configurator
        && !cfg.supports_config_registration()
    {
        return HashSet::new();
    }

    config
        .actions
        .keys()
        .filter(|event| {
            // Check if configurator has specific registerable events
            if let Some(cfg) = configurator
                && let Some(registerable) = cfg.registerable_events()
            {
                return registerable.contains(event);
            }

            // Default: filter by provider's hook-based event support only
            provider.supports_event_via_hook(event)
        })
        .map(|event| event.as_slug().to_string())
        .collect()
}

/// Get ALL events from the ClaudineConfig's actions map.
///
/// Since ClaudineConfig events are provider-agnostic, all configured events
/// are considered "enabled" for every provider.
fn all_enabled_events(config: &ClaudineConfig) -> HashSet<String> {
    config
        .actions
        .keys()
        .map(|event| event.as_slug().to_string())
        .collect()
}

/// Check if an event is supported by a provider (via hook).
fn is_event_supported(provider: Provider, event_name: &str) -> bool {
    AgenticEvent::from_slug(event_name)
        .map(|event| provider.supports_event_via_hook(&event))
        .unwrap_or(false)
}

/// Format an event with color based on sync status and support.
///
/// - RED + strikethrough: configured but not supported by provider (invalid)
/// - RED: registered but not in config (stale)
/// - ORANGE: in config but not registered (missing)
/// - Normal: in sync
fn format_event_with_color(
    event: &str,
    is_stale: bool,
    is_missing: bool,
    is_unsupported: bool,
) -> String {
    let display = event_name_pascal(event);
    if is_unsupported {
        format!("<red><strikethrough>{display}</strikethrough></red>")
    } else if is_stale {
        format!("<red>{display}</red>")
    } else if is_missing {
        format!("<yellow>{display}</yellow>")
    } else {
        display
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

/// An invalid sound effect with its suggested replacement.
struct InvalidEffect {
    invalid_name: String,
    suggestion: Option<&'static str>,
}

/// Find all invalid sound effect names in the config.
///
/// Returns a list of invalid effect names with their suggested replacements.
fn find_invalid_sound_effects(config: &ClaudineConfig) -> Vec<InvalidEffect> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut invalid_effects: Vec<InvalidEffect> = Vec::new();

    for actions in config.actions.values() {
        for action in actions {
            if let HookAction::SoundEffect { effect, .. } = action
                && SoundEffect::from_name(effect).is_none()
                && !seen.contains(effect)
            {
                seen.insert(effect.clone());
                invalid_effects.push(InvalidEffect {
                    invalid_name: effect.clone(),
                    suggestion: find_similar_effect(effect),
                });
            }
        }
    }

    invalid_effects
}

/// Validate sound effect names in the config and display warnings.
///
/// Returns the list of invalid effects for potential fixing.
fn validate_sound_effects(config: &ClaudineConfig) -> Vec<InvalidEffect> {
    let invalid_effects = find_invalid_sound_effects(config);

    if invalid_effects.is_empty() {
        return invalid_effects;
    }

    log::data("");
    let header = Prose::new("{{yellow}}{{bold}}⚠ Invalid sound effects:{{reset}}");
    log::data(&format!(
        " {}",
        header.render(&crate::log::optimistic_terminal(Some(100)))
    ));

    let mut has_fixable = false;
    for effect in &invalid_effects {
        let msg = match effect.suggestion {
            Some(similar) => {
                has_fixable = true;
                format!(
                    "  {{{{dim}}}}-{{{{reset}}}} {{{{red}}}}{}{{{{reset}}}} {{{{dim}}}}(did you mean {{{{green}}}}{}{{{{reset}}}}{{{{dim}}}}){{{{reset}}}}",
                    effect.invalid_name, similar
                )
            }
            None => format!(
                "  {{{{dim}}}}-{{{{reset}}}} {{{{red}}}}{}{{{{reset}}}} {{{{dim}}}}(no similar effect found){{{{reset}}}}",
                effect.invalid_name
            ),
        };
        log::data(&format!(
            " {}",
            Prose::new(msg).render(&crate::log::optimistic_terminal(Some(100)))
        ));
    }

    log::data("");
    if has_fixable {
        let hint = Prose::new(
            "{{dim}}Edit {{blue}}~/.claudine/config.json{{reset}}{{dim}} to apply suggested fixes{{reset}}",
        );
        log::data(&format!(
            " {}",
            hint.render(&crate::log::optimistic_terminal(Some(100)))
        ));
    }
    let hint = Prose::new(
        "{{dim}}Run {{blue}}playa list-effects{{reset}}{{dim}} to see available effects{{reset}}",
    );
    log::data(&format!(
        " {}",
        hint.render(&crate::log::optimistic_terminal(Some(100)))
    ));

    invalid_effects
}

/// Find a similar valid effect name using simple heuristics.
fn find_similar_effect(invalid: &str) -> Option<&'static str> {
    let all_effects = SoundEffect::all();

    // First try: normalize dashes and numbers (handles "fx-01" vs "fx1")
    let normalized = normalize_effect_name(invalid);
    for effect in &all_effects {
        let effect_name = effect.name();
        if normalize_effect_name(effect_name) == normalized {
            return Some(effect_name);
        }
    }

    // Second try: find effects that contain the invalid name or vice versa
    let invalid_lower = invalid.to_lowercase();
    for effect in &all_effects {
        let effect_name = effect.name();
        let effect_lower = effect_name.to_lowercase();
        if effect_lower.contains(&invalid_lower) || invalid_lower.contains(&effect_lower) {
            return Some(effect_name);
        }
    }

    // Third try: check for shared suffix (handles "swoosh" matching "air-woosh")
    // Look for effects where the last N chars match
    if invalid.len() >= 4 {
        let suffix = &invalid_lower[1..]; // Skip first char to handle s/w differences
        for effect in &all_effects {
            let effect_name = effect.name();
            let effect_lower = effect_name.to_lowercase();
            if effect_lower.ends_with(suffix) || effect_lower.contains(suffix) {
                return Some(effect_name);
            }
        }
    }

    // Fourth try: find effects that start with the same prefix
    let prefix = invalid.split('-').next().unwrap_or(invalid);
    let mut candidates: Vec<&str> = all_effects
        .iter()
        .filter(|e| e.name().starts_with(prefix))
        .map(|e| e.name())
        .collect();

    if !candidates.is_empty() {
        candidates.sort_by_key(|c| (c.len() as i32 - invalid.len() as i32).abs());
        return Some(candidates[0]);
    }

    // Fifth try: check if any effect contains the prefix
    for effect in &all_effects {
        let effect_name = effect.name();
        if effect_name.contains(prefix) {
            return Some(effect_name);
        }
    }

    None
}

/// Normalize an effect name by removing separators and lowercasing.
fn normalize_effect_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Dim+italic opener for action parameters (inside parentheses, not the parens themselves).
const DI: &str = "{{dim}}{{italic}}";
/// Undo dim+italic only (preserves background for table striping).
const DI_R: &str = "{{normal-font-weight}}{{not-italic}}";

/// Format an action for display in the detailed provider view.
fn format_action(action: &HookAction) -> String {
    match action {
        HookAction::Speak { message, .. } => {
            format!(
                "<cyan>Speak</cyan>({DI}\"{}\"{DI_R})",
                truncate_string(message, 40)
            )
        }
        HookAction::SoundEffect {
            effect,
            volume,
            speed,
            ..
        } => {
            let mut params = Vec::new();
            if *volume != 1.0 {
                params.push(format!("vol={:.1}", volume));
            }
            if *speed != 1.0 {
                params.push(format!("speed={:.1}", speed));
            }
            let params_str = if params.is_empty() {
                String::new()
            } else {
                format!(", {}", params.join(", "))
            };
            format!(
                "<magenta>SoundEffect</magenta>({DI}{}{}{DI_R})",
                effect, params_str
            )
        }
        HookAction::Report { handler, .. } => {
            let format_str = handler
                .as_ref()
                .map(|h| match h.format {
                    ReportFormat::Text => "text",
                    ReportFormat::Json => "json",
                    ReportFormat::Compact => "compact",
                    _ => "text",
                })
                .unwrap_or("text");

            let has_template = handler.as_ref().and_then(|h| h.template.as_ref()).is_some();
            if has_template {
                let template = handler
                    .as_ref()
                    .and_then(|h| h.template.as_ref())
                    .map(|t| truncate_string(t, 30))
                    .unwrap_or_default();
                format!(
                    "<yellow>Report</yellow>({DI}format={}, template=\"{}\"{DI_R})",
                    format_str, template
                )
            } else {
                format!("<yellow>Report</yellow>({DI}format={}{DI_R})", format_str)
            }
        }
        HookAction::Bash {
            command, params, ..
        } => {
            if params.is_empty() {
                format!("<green>Bash</green>({DI}\"{}\"{DI_R})", command)
            } else {
                format!(
                    "<green>Bash</green>({DI}\"{} {}\"{DI_R})",
                    command,
                    truncate_string(params, 30)
                )
            }
        }
        HookAction::Call {
            command,
            args,
            timeout_ms,
            ..
        } => {
            let mut params = Vec::new();
            if let Some(a) = args
                && !a.is_empty()
            {
                params.push(a.join(" "));
            }
            if let Some(t) = timeout_ms {
                params.push(format!("timeout={}ms", t));
            }
            let params_str = if params.is_empty() {
                String::new()
            } else {
                format!(", {}", params.join(", "))
            };
            format!(
                "<green>Call</green>({DI}\"{}{}\"{DI_R})",
                truncate_string(command, 30),
                params_str
            )
        }
        _ => format!("<dim>{action:?}</dim>"),
    }
}

/// Truncate a string with ellipsis if it exceeds max length.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

/// Show detailed event/action configuration for a specific provider.
fn run_provider_detail(provider: Provider, config: Option<&ClaudineConfig>) -> Result<()> {
    let term = crate::log::terminal();
    let clients = InstalledAiClients::new();
    let installed = clients.is_installed(provider.sniff_ai_cli());

    // Header
    let status_icon = if installed { "✅" } else { "❌" };
    let header = Prose::new(format!(
        "{{{{bold}}}}{}{{{{reset}}}} {} {{{{dim}}}}({}installed){{{{reset}}}}",
        provider,
        status_icon,
        if installed { "" } else { "not " }
    ));
    log::data(&format!("\n {}", header.render(&term)));

    // Show docs URL
    let docs = Prose::new(format!("{{{{dim}}}}{}{{{{reset}}}}", provider.docs_url()));
    log::data(&format!(" {}", docs.render(&term)));
    log::data("");

    // Build a sorted list of all events with their status
    let mut event_rows: Vec<(AgenticEvent, Option<&Vec<HookAction>>)> = Vec::new();
    for event in AgenticEvent::ALL {
        let actions = config.and_then(|c| c.actions.get(&event));
        event_rows.push((event, actions));
    }

    // Build table
    let columns = vec![
        TableColumn::new(bold("Event")),
        TableColumn::new(bold("Support")),
        TableColumn::new(bold("Actions")),
    ];
    let mut table = Table::new()
        .with_columns(columns)
        .prefer_cursor_alignment()
        .alternate_background_color();
    table.layout_mut().left_margin = Margin::Chars(1);

    for (event, actions) in &event_rows {
        let support_level = provider.event_support_level(event);
        let support_cell: TableCellContent = match support_level {
            EventSupportLevel::Hook { .. } => "hook".into(),
            EventSupportLevel::StreamParse { .. }
            | EventSupportLevel::WireProxy { .. }
            | EventSupportLevel::Wrapper { .. } => {
                Prose::new("{{dim}}non-hook{{reset}}").render(&term).into()
            }
            EventSupportLevel::Acp { .. } => {
                Prose::new("{{cyan}}acp{{reset}}").render(&term).into()
            }
            EventSupportLevel::NotSupported => Prose::new("{{dim}}-{{reset}}").render(&term).into(),
        };

        let actions_cell: TableCellContent = match actions {
            None => "-".into(),
            Some(a) if a.is_empty() => Prose::new("{{dim}}(no actions){{reset}}")
                .render(&term)
                .into(),
            Some(a) => {
                let text = a.iter().map(format_action).collect::<Vec<_>>().join("\n");
                Prose::new(text).render(&term).into()
            }
        };

        table.add_row(vec![
            event.as_pascal_case().into(),
            support_cell,
            actions_cell,
        ]);
    }

    let rendered = table.render(&term);
    log::data(&rendered);

    // Summary stats
    let total_unified_events = AgenticEvent::ALL.len();
    let configured_count = event_rows
        .iter()
        .filter(|row| row.1.is_some_and(|a| !a.is_empty()))
        .count();

    log::data("");
    let summary = Prose::new(format!(
        "{{{{bold}}}}{}{{{{reset}}}} supports {{{{yellow}}}}{}{{{{reset}}}} of the {{{{bold}}}}{{{{yellow}}}}{}{{{{reset}}}} unified events",
        provider, configured_count, total_unified_events
    ));
    log::data(&format!(" {}", summary.render(&term)));

    // Show enabled events table with descriptions
    let enabled_events: Vec<&AgenticEvent> = event_rows
        .iter()
        .filter(|(_, actions)| actions.is_some_and(|a| !a.is_empty()))
        .map(|(event, _)| event)
        .collect();

    // Count unsupported events
    let unsupported_count = enabled_events
        .iter()
        .filter(|e| !provider.supports_event_via_hook(e))
        .count();

    if !enabled_events.is_empty() {
        log::data("");
        let enabled_header = if unsupported_count > 0 {
            Prose::new(format!(
                "{{{{bold}}}}Event Descriptions{{{{reset}}}} {{{{red}}}}(⚠ {} unsupported){{{{reset}}}}",
                unsupported_count
            ))
        } else {
            Prose::new("{{bold}}Event Descriptions{{reset}}")
        };
        log::data(&format!(" {}", enabled_header.render(&term)));

        let desc_columns = vec![
            TableColumn::new(bold("Event")),
            TableColumn::new(bold("Description")),
        ];
        let mut desc_table = Table::new()
            .with_columns(desc_columns)
            .prefer_cursor_alignment()
            .alternate_background_color();
        desc_table.layout_mut().left_margin = Margin::Chars(1);

        for event in enabled_events {
            let is_unsupported = !provider.supports_event_via_hook(event);
            let event_cell: TableCellContent = if is_unsupported {
                Prose::new(format!(
                    "{{{{red}}}}{{{{strikethrough}}}}{}{{{{reset}}}}",
                    event.as_pascal_case()
                ))
                .render(&term)
                .into()
            } else {
                event.as_pascal_case().into()
            };
            let desc_cell: TableCellContent = if is_unsupported {
                Prose::new(format!("{{{{dim}}}}{}{{{{reset}}}}", event.description()))
                    .render(&term)
                    .into()
            } else {
                event.description().into()
            };
            desc_table.add_row(vec![event_cell, desc_cell]);
        }

        let desc_rendered = desc_table.render(&term);
        log::data(&desc_rendered);
    }

    Ok(())
}

/// Show registered hooks for all providers.
pub fn run(args: HooksArgs, verbose: bool) -> Result<()> {
    // Handle --support, --mapping, --describe, and --variables flags first
    if args.support {
        return run_support();
    }
    if args.mapping {
        return run_mapping();
    }
    if args.describe {
        return run_describe();
    }
    if args.variables {
        return run_variables();
    }
    if args.capture_method {
        return run_capture_method();
    }

    // Try to load claudine config for sync checking
    let config = load_claudine_config(None, None).ok();
    render_protect_visibility(config.as_ref());

    // If a provider is specified, show detailed view for that provider
    if let Some(provider) = args.provider {
        let result = run_provider_detail(provider, config.as_ref());

        // Validate sound effects
        if let Some(cfg) = config.as_ref() {
            validate_sound_effects(cfg);
        }

        return result;
    }

    let agents = detect_agents();
    let clients = InstalledAiClients::new();

    let result = if verbose {
        run_verbose(&agents, &clients, config.as_ref())
    } else {
        run_simple(&agents, &clients, config.as_ref())
    };

    // Validate sound effects in the config
    if let Some(cfg) = config.as_ref() {
        validate_sound_effects(cfg);
    }

    result
}

fn render_protect_visibility(config: Option<&ClaudineConfig>) {
    let Some(config) = config else { return };
    log::data("");
    if config.protect.enabled {
        log::data("Protect: enabled");
    } else {
        log::data("Protect: disabled");
    }
}

/// Simple table view showing provider, installed status, and subscribed hooks list.
fn run_simple(
    agents: &[(Provider, Box<dyn AgentConfigurator>)],
    clients: &InstalledAiClients,
    config: Option<&ClaudineConfig>,
) -> Result<()> {
    let mut table = Table::new().with_columns(vec![
        provider_column(),
        TableColumn::new(bold("Installed")),
        TableColumn::new(bold("Subscribed Hooks")),
    ]);
    table.layout_mut().left_margin = Margin::Chars(1);

    let mut has_sync_issues = false;
    let mut has_unsupported_issues = false;

    for provider in ALL_PROVIDERS {
        let installed = clients.is_installed(provider.sniff_ai_cli());
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

            // Get expected events from claudine config for this provider (supported only)
            let expected: HashSet<String> = config
                .map(|c| expected_events_for_provider(c, provider, configurator))
                .unwrap_or_default();

            // Get ALL enabled events to detect config errors
            let all_enabled: HashSet<String> = config.map(all_enabled_events).unwrap_or_default();

            // Find unsupported events (enabled but not in expected because provider doesn't support them)
            let unsupported: HashSet<&String> = all_enabled
                .iter()
                .filter(|e| !is_event_supported(provider, e))
                .collect();

            if registered.is_empty() && expected.is_empty() && unsupported.is_empty() {
                "-".into()
            } else {
                // Combine all events: registered + expected + unsupported
                let mut all_events: HashSet<&String> = registered.union(&expected).collect();
                all_events.extend(&unsupported);
                let mut all_events: Vec<&String> = all_events.into_iter().collect();
                all_events.sort();

                let formatted: Vec<String> = all_events
                    .into_iter()
                    .map(|event| {
                        let is_unsupported = unsupported.contains(event);
                        let is_stale = registered.contains(event) && !expected.contains(event);
                        let is_missing = expected.contains(event) && !registered.contains(event);
                        if is_stale || is_missing {
                            has_sync_issues = true;
                        }
                        if is_unsupported {
                            has_unsupported_issues = true;
                        }
                        format_event_with_color(event, is_stale, is_missing, is_unsupported)
                    })
                    .collect();

                // Use Prose to render the colored output
                let text = formatted.join(", ");
                Prose::new(text)
                    .render(&crate::log::optimistic_terminal(None))
                    .into()
            }
        };

        // Create OSC8 hyperlink for provider name
        let provider_link = format!(r#"<a href="{}">{}</a>"#, provider.docs_url(), provider);
        let provider_cell: TableCellContent = Prose::new(provider_link)
            .render(&crate::log::optimistic_terminal(None))
            .into();

        table.add_row(vec![provider_cell, bool_indicator(installed), hooks_cell]);
    }

    let term = crate::log::terminal();
    let table = table.prefer_cursor_alignment();

    let rendered = table.render(&term);
    log::data(&format!("\n{}", rendered));

    // Show color legend if there are issues
    if has_sync_issues || has_unsupported_issues {
        log::data("");
        let mut legend_parts = Vec::new();
        if has_unsupported_issues {
            legend_parts.push(
                "{{red}}{{strikethrough}}strikethrough{{reset}}{{dim}} = unsupported (won't fire)",
            );
        }
        if has_sync_issues {
            legend_parts.push("{{red}}red{{reset}}{{dim}} = stale (remove with sync)");
            legend_parts.push("{{yellow}}orange{{reset}}{{dim}} = missing (add with sync)");
        }
        let legend = Prose::new(format!(
            "{{{{dim}}}}- Legend: {}{{{{reset}}}}",
            legend_parts.join(", ")
        ));
        log::data(&format!(
            " {}",
            legend.render(&crate::log::optimistic_terminal(Some(120)))
        ));
    }

    // Show hints about available flags
    log::data("");
    let hints = [
        "{{dim}}- Use <blue><bold>-v</bold></blue>{{dim}} for detailed event matrix{{reset}}",
        "{{dim}}- Use <blue><bold>--support</bold></blue>{{dim}} to see which events each provider supports{{reset}}",
        "{{dim}}- Use <blue><bold>--mapping</bold></blue>{{dim}} to see native event name mappings{{reset}}",
        "{{dim}}- Use <blue><bold>--describe</bold></blue>{{dim}} to see event descriptions and schemas{{reset}}",
        "{{dim}}- Use <blue><bold>--variables</bold></blue>{{dim}} to see template variables for speak/report{{reset}}",
        "{{dim}}- Use <blue><bold>--capture-method</bold></blue>{{dim}} to see how each event is captured (hook / non-hook / acp){{reset}}",
    ];
    for hint in hints {
        log::data(&format!(
            " {}",
            Prose::new(hint).render(&crate::log::optimistic_terminal(Some(100)))
        ));
    }

    Ok(())
}

const NOT_INSTALLED: &str = "-";
const NOT_ALLOWED: &str = "⚠️";

/// Verbose table view showing per-event action counts in a matrix.
fn run_verbose(
    agents: &[(Provider, Box<dyn AgentConfigurator>)],
    clients: &InstalledAiClients,
    config: Option<&ClaudineConfig>,
) -> Result<()> {
    // Build columns: Provider, ∃ (exists/installed), then one per event
    let mut columns = vec![provider_column(), TableColumn::new(bold("∃"))]; // existence symbol for installed

    // Add a column for each event using abbreviations
    for event in AgenticEvent::ALL {
        let mut column = TableColumn::new(bold(event.abbrev()));
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
        let installed = clients.is_installed(provider.sniff_ai_cli());
        let _configurator = find_configurator(agents, provider);

        // Create OSC8 hyperlink for provider name
        let provider_link = format!(r#"<a href="{}">{}</a>"#, provider.docs_url(), provider);
        let provider_cell: TableCellContent = Prose::new(provider_link)
            .render(&crate::log::optimistic_terminal(None))
            .into();

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
                let actions = config.and_then(|c| c.actions.get(&event));
                match actions {
                    None => NOT_INSTALLED.into(),
                    Some(a) if a.is_empty() => NOT_INSTALLED.into(),
                    Some(a) => action_count_indicator(a.len()).into(),
                }
            };
            row.push(cell);
        }

        table.add_row(row);
    }

    let term = crate::log::terminal();
    let rendered = table.render(&term);
    log::data(&format!("\n{}", rendered));

    // Show legend
    log::data("");
    let legend = Prose::new(
        "{{dim}}Legend: {{reset}}⚠️{{dim}} = not supported, {{reset}}-{{dim}} = not configured, {{reset}}⓪{{dim}} = 0 actions, {{reset}}❶{{dim}} = 1 action, etc.{{reset}}",
    ).with_left_margin(Margin::Chars(8));
    log::data(&format!(" {}\n", legend.render(&term)));

    // Show hints about available flags
    let hints = [
        "{{dim}}- Use <blue><bold>--support</bold></blue>{{dim}} to see which events each provider supports{{reset}}",
        "{{dim}}- Use <blue><bold>--mapping</bold></blue>{{dim}} to see native event name mappings{{reset}}",
        "{{dim}}- Use <blue><bold>--describe</bold></blue>{{dim}} to see event descriptions and schemas{{reset}}",
        "{{dim}}- Use <blue><bold>--variables</bold></blue>{{dim}} to see template variables for speak/report{{reset}}",
    ];
    for hint in hints {
        log::data(&format!(" {}", Prose::new(hint).render(&term)));
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
/// Indicator for ACP-based support (Goose request_permission, Kimi ApprovalRequest).
const ACP_SUPPORT: &str = "🅐";
/// Indicator for no support (using ❌ U+274C which has width 2 like other emoji).
const NO_SUPPORT: &str = "❌";

/// Show provider event support matrix with ✅/⛔️/❌ indicators.
///
/// - ✅ Hook: Event can be registered via config file
/// - ⛔️ NonHook: Event requires wrapper/proxy (not yet implemented)
/// - ❌ NotSupported: Event is not available from this provider
fn run_support() -> Result<()> {
    let term = crate::log::terminal();
    let matrix = event_support_matrix(&ALL_PROVIDERS);

    // Build columns: Event name (left-aligned), then one per provider (centered)
    let mut columns = vec![TableColumn::new(bold("Event"))];
    for provider in ALL_PROVIDERS {
        columns
            .push(TableColumn::new(bold(&provider.to_string())).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);

    // Add a row for each event
    for matrix_row in matrix {
        let mut row: Vec<TableCellContent> = vec![matrix_row.event.as_pascal_case().into()];

        for cell in matrix_row.cells {
            let rendered = match cell.level {
                EventSupportLevel::Hook { .. } => HOOK_SUPPORT.into(),
                EventSupportLevel::StreamParse { .. }
                | EventSupportLevel::WireProxy { .. }
                | EventSupportLevel::Wrapper { .. } => NON_HOOK_SUPPORT.into(),
                EventSupportLevel::Acp { .. } => ACP_SUPPORT.into(),
                EventSupportLevel::NotSupported => NO_SUPPORT.into(),
            };
            row.push(rendered);
        }

        table.add_row(row);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{}", rendered));

    // Show legend
    log::data("");
    let legend = Prose::new(
        "{{dim}}Legend: {{reset}}✅{{dim}} = hook support (config file), {{reset}}⛔️{{dim}} = non-hook (wrapper/proxy required), {{reset}}{{NO_SUPPORT}}{{dim}} = not supported{{reset}}",
    );
    log::data(&format!(" {}\n", legend.render(&term)));

    Ok(())
}

/// Show the capture method (hook / non-hook / acp / -) for each event x
/// provider pair.
///
/// Phase 7 of the centralized providers refactor surfaces ACP-based capture
/// as a first-class metadata column. Goose `request_permission` and Kimi
/// `ApprovalRequest` are tagged via [`EventSupportLevel::Acp`] in their
/// per-provider `EventMappingTable`, and a non-`NotSupported`
/// [`AcpSupport`](claudine::provider::AcpSupport) is published from the
/// matching `ProviderInfo`. This view renders that pairing per provider.
fn run_capture_method() -> Result<()> {
    use claudine::provider::provider_info;

    let term = crate::log::terminal();
    let matrix = event_support_matrix(&ALL_PROVIDERS);

    // Per-provider header: also include ACP server-mode badge so the
    // capture method matrix and provider ACP posture are co-visible.
    let mut columns = vec![TableColumn::new(bold("Event"))];
    for provider in ALL_PROVIDERS {
        let info = provider_info(provider);
        let header = if info.acp.is_supported() {
            format!("{} {}", provider, ACP_SUPPORT)
        } else {
            provider.to_string()
        };
        columns.push(TableColumn::new(bold(&header)).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);

    for matrix_row in matrix {
        let mut row: Vec<TableCellContent> = vec![matrix_row.event.as_pascal_case().into()];

        for cell in matrix_row.cells {
            let rendered: TableCellContent = match cell.level {
                EventSupportLevel::Hook { .. } => "hook".into(),
                EventSupportLevel::StreamParse { protocol, .. } => {
                    format!("stream-parse ({protocol:?})").into()
                }
                EventSupportLevel::WireProxy { mode, .. } => {
                    format!("wire-proxy ({mode:?})").into()
                }
                EventSupportLevel::Wrapper { .. } => "wrapper".into(),
                EventSupportLevel::Acp { .. } => {
                    Prose::new("{{cyan}}acp{{reset}}").render(&term).into()
                }
                EventSupportLevel::NotSupported => {
                    Prose::new("{{dim}}-{{reset}}").render(&term).into()
                }
            };
            row.push(rendered);
        }

        table.add_row(row);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{}", rendered));

    log::data("");
    let legend = Prose::new(
        "{{dim}}Legend: {{reset}}hook{{dim}} = config-file hook, {{reset}}stream-parse{{dim}} = stream parsing, {{reset}}wire-proxy{{dim}} = wire-mode proxy, {{reset}}wrapper{{dim}} = wrapper script, {{reset}}{{cyan}}acp{{reset}}{{dim}} = Agent Client Protocol, {{reset}}-{{dim}} = not supported{{reset}}",
    );
    log::data(&format!(" {}", legend.render(&term)));

    let acp_note = Prose::new(
        "{{dim}}- 🅐 next to a provider name marks an ACP-supported provider (server or wire-proxy){{reset}}",
    );
    log::data(&format!(" {}", acp_note.render(&term)));

    Ok(())
}

/// Show native event name mappings for each provider.
///
/// Splits providers into two tables to fit width-constrained terminals.
fn run_mapping() -> Result<()> {
    let term = crate::log::terminal();

    for provider_group in ALL_PROVIDERS.chunks(4) {
        let table = build_mapping_table(provider_group).prefer_cursor_alignment();
        let rendered = table.render(&term);
        log::data(&format!("\n{}", rendered));
    }

    // Show legend
    log::data("");
    let legend =
        Prose::new("{{dim}}- Legend: (blank) = not supported or no specific native name{{reset}}");
    log::data(&format!(
        " {}",
        legend.render(&crate::log::optimistic_terminal(Some(100)))
    ));

    Ok(())
}

/// Build a mapping table for a subset of providers.
fn build_mapping_table(providers: &[Provider]) -> Table {
    // Build columns: Event name (left-aligned), then one per provider (center-aligned)
    let mut columns = vec![TableColumn::new(bold("Event"))];
    for provider in providers {
        columns
            .push(TableColumn::new(bold(&provider.to_string())).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns);
    table.layout_mut().left_margin = Margin::Chars(1);

    // Add a row for each event
    for matrix_row in event_native_mapping_matrix(providers) {
        let mut row: Vec<TableCellContent> = vec![matrix_row.event.as_pascal_case().into()];

        for cell in matrix_row.cells {
            let cell: TableCellContent = match cell.native {
                NativeEventName::Unsupported => "".into(),
                NativeEventName::NoSpecificName => "".into(),
                NativeEventName::Named(name) => name.into(),
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
        TableColumn::new(bold("Event")),
        TableColumn::new(bold("Response Schema")),
        TableColumn::new(bold("Return Schema")),
        TableColumn::new(bold("Description")),
    ];

    let term = crate::log::terminal();
    let mut table = Table::new().with_columns(columns).alternate_text_color();
    table.layout_mut().left_margin = Margin::Chars(1);

    // Add a row for each event
    for event in AgenticEvent::ALL {
        let row: Vec<TableCellContent> = vec![
            event.as_pascal_case().into(),
            event.response_schema().into(),
            event.return_schema().into(),
            event.description().into(),
        ];
        table.add_row(row);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{}", rendered));

    // Show legend
    log::data("");
    let legend =
        Prose::new("{{dim}}- Response Schema: fields available in the event payload{{reset}}");
    log::data(&format!(" {}", legend.render(&term)));
    let legend2 = Prose::new(
        "{{dim}}- Return Schema: what hooks can return to influence agent behavior (blocking hooks only){{reset}}",
    );
    log::data(&format!(" {}", legend2.render(&term)));

    Ok(())
}

/// Show available template variables for speak/report actions.
///
/// Uses `TemplateVariable::all()` as the single source of truth - adding new
/// variables to the enum automatically updates this display.
fn run_variables() -> Result<()> {
    let term = crate::log::terminal();

    let header = Prose::new("{{bold}}Template Variables{{reset}}");
    log::data(&format!("\n {}", header.render(&term)));
    log::data("");
    let intro = Prose::new(
        "{{dim}}Use these in speak messages and report templates: {{reset}}{{cyan}}\"Tool {{tool_name}} failed: {{error}}\"{{reset}}",
    );
    log::data(&format!(" {}", intro.render(&term)));
    log::data("");

    // Event fields table
    let columns = vec![
        TableColumn::new(bold("Variable")),
        TableColumn::new(bold("Description")),
        TableColumn::new(bold("Available For")),
    ];
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().left_margin = Margin::Chars(1);

    for var in TemplateVariable::event_variables() {
        table.add_row(vec![
            Prose::new(format!("{{{{cyan}}}}{}{{{{reset}}}}", var.placeholder()))
                .render(&term)
                .into(),
            var.description().into(),
            Prose::new(format!("{{{{dim}}}}{}{{{{reset}}}}", var.availability()))
                .render(&term)
                .into(),
        ]);
    }

    let rendered = table.render(&term);
    log::data(&rendered);

    // Context variables - grouped by category with current values
    log::data("");
    let ctx_header =
        Prose::new("{{bold}}Context Variables{{reset}} {{dim}}(auto-detected at runtime){{reset}}");
    log::data(&format!(" {}", ctx_header.render(&term)));

    // Detect current environment to show live values
    let cwd = std::env::current_dir().unwrap_or_default();
    let env_context = detect_environment(&cwd);
    let dummy_meta = EventMeta::dummy_with_env(env_context);

    let ctx_columns = vec![
        TableColumn::new(bold("Variable")),
        TableColumn::new(bold("Description")),
        TableColumn::new(bold("Current Value")),
    ];
    let mut ctx_table = Table::new().with_columns(ctx_columns);
    ctx_table.layout_mut().left_margin = Margin::Chars(1);

    // Track current category for section headers
    let mut current_category: Option<VariableCategory> = None;

    for var in TemplateVariable::context_variables() {
        let cat = var.category();

        // Add category header when it changes
        if current_category != Some(cat) {
            current_category = Some(cat);
            ctx_table.add_row(vec![
                Prose::new(format!("{{{{dim}}}}# {}{{{{reset}}}}", cat.label()))
                    .render(&term)
                    .into(),
                "".into(),
                "".into(),
            ]);
        }

        // Resolve current value
        let current_value = var.resolve(&dummy_meta);
        let value_display = if current_value.is_empty() {
            Prose::new("{{dim}}-{{reset}}").render(&term)
        } else {
            // Truncate long values
            let truncated = if current_value.len() > 40 {
                format!("{}…", &current_value[..39])
            } else {
                current_value.to_string()
            };
            Prose::new(format!("{{{{green}}}}{}{{{{reset}}}}", truncated)).render(&term)
        };

        ctx_table.add_row(vec![
            Prose::new(format!("{{{{cyan}}}}{}{{{{reset}}}}", var.placeholder()))
                .render(&term)
                .into(),
            var.description().into(),
            value_display.into(),
        ]);
    }

    let ctx_rendered = ctx_table.render(&term);
    log::data(&ctx_rendered);

    // Example
    log::data("");
    let example_header = Prose::new("{{bold}}Example{{reset}}");
    log::data(&format!(" {}", example_header.render(&term)));
    log::data("");
    let example = Prose::new(
        r#"{{dim}}  {
    "type": "speak",
    "message": "Tool {{tool_name}} failed on {{git.branch}}: {{error}}"
  }{{reset}}"#,
    );
    log::data(&example.render(&term));
    log::data("");

    Ok(())
}
