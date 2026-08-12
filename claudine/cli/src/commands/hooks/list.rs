use std::collections::HashSet;

use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Edges, Length, TargetValue, WordWrap};
use claudine::actions::{HookAction, ReportFormat};
use claudine::config::AgentConfigurator;
use claudine::config::claudine_config::ClaudineConfig;
use claudine::events::AgenticEvent;
use claudine::provider::{EventSupportLevel, Provider};
use playa::SoundEffect;
use sniff::programs::InstalledAiClients;

use crate::cli_utils::{bool_indicator, event_name_pascal};
use crate::log;

use super::{ALL_PROVIDERS, bold, find_configurator, provider_column};

/// Get the expected events for a provider from the ClaudineConfig.
fn expected_events_for_provider(
    config: &ClaudineConfig,
    provider: Provider,
    configurator: Option<&dyn AgentConfigurator>,
) -> HashSet<String> {
    if let Some(cfg) = configurator
        && !cfg.supports_config_registration()
    {
        return HashSet::new();
    }

    config
        .actions
        .keys()
        .filter(|event| {
            if let Some(cfg) = configurator
                && let Some(registerable) = cfg.registerable_events()
            {
                return registerable.contains(event);
            }

            provider.supports_event_via_hook(event)
        })
        .map(|event| event.as_slug().to_string())
        .collect()
}

/// Get ALL events from the ClaudineConfig's actions map.
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

pub(super) fn validate_sound_effects(config: &ClaudineConfig) {
    for line in render_invalid_sound_effects(config) {
        log::data(&line);
    }
}

fn render_invalid_sound_effects(config: &ClaudineConfig) -> Vec<String> {
    let invalid_effects = find_invalid_sound_effects(config);

    if invalid_effects.is_empty() {
        return Vec::new();
    }

    let mut lines = vec![String::new()];
    let header = Prose::new("<yellow><bold>⚠ Invalid sound effects:</bold></yellow>");
    lines.push(format!(
        " {}",
        header.render(&crate::log::optimistic_terminal(Some(100)))
    ));

    let mut has_fixable = false;
    for effect in &invalid_effects {
        let msg = match effect.suggestion {
            Some(similar) => {
                has_fixable = true;
                format!(
                    "  <dim>-</dim> <red>{}</red> <dim>(did you mean <green>{}</green>)</dim>",
                    Prose::escape_text(&effect.invalid_name),
                    Prose::escape_text(similar)
                )
            }
            None => format!(
                "  <dim>-</dim> <red>{}</red> <dim>(no similar effect found)</dim>",
                Prose::escape_text(&effect.invalid_name)
            ),
        };
        lines.push(format!(
            " {}",
            Prose::new(msg).render(&crate::log::optimistic_terminal(Some(100)))
        ));
    }

    lines.push(String::new());
    if has_fixable {
        let hint = Prose::new(
            "<dim>Edit <blue>~/.claudine/config.json</blue> to apply suggested fixes</dim>",
        );
        lines.push(format!(
            " {}",
            hint.render(&crate::log::optimistic_terminal(Some(100)))
        ));
    }
    let hint =
        Prose::new("<dim>Run <blue>playa list-effects</blue> to see available effects</dim>");
    lines.push(format!(
        " {}",
        hint.render(&crate::log::optimistic_terminal(Some(100)))
    ));
    lines
}

fn find_similar_effect(invalid: &str) -> Option<&'static str> {
    let all_effects = SoundEffect::all();

    let normalized = normalize_effect_name(invalid);
    for effect in &all_effects {
        let effect_name = effect.name();
        if normalize_effect_name(effect_name) == normalized {
            return Some(effect_name);
        }
    }

    let invalid_lower = invalid.to_lowercase();
    for effect in &all_effects {
        let effect_name = effect.name();
        let effect_lower = effect_name.to_lowercase();
        if effect_lower.contains(&invalid_lower) || invalid_lower.contains(&effect_lower) {
            return Some(effect_name);
        }
    }

    if invalid.len() >= 4 {
        let suffix = &invalid_lower[1..];
        for effect in &all_effects {
            let effect_name = effect.name();
            let effect_lower = effect_name.to_lowercase();
            if effect_lower.ends_with(suffix) || effect_lower.contains(suffix) {
                return Some(effect_name);
            }
        }
    }

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

    for effect in &all_effects {
        let effect_name = effect.name();
        if effect_name.contains(prefix) {
            return Some(effect_name);
        }
    }

    None
}

fn normalize_effect_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Dim+italic opener for action parameters (inside parentheses, not the parens themselves).
const DI: &str = "<dim><italic>";
/// Undo dim+italic only (preserves background for table striping).
const DI_R: &str = "</italic></dim>";

fn format_action(action: &HookAction) -> String {
    match action {
        HookAction::Speak { message, .. } => {
            format!(
                "<cyan>Speak</cyan>({DI}\"{}\"{DI_R})",
                Prose::escape_text(&truncate_string(message, 40))
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
                Prose::escape_text(effect),
                params_str
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
                    format_str,
                    Prose::escape_text(&template)
                )
            } else {
                format!("<yellow>Report</yellow>({DI}format={}{DI_R})", format_str)
            }
        }
        HookAction::Bash {
            command, params, ..
        } => {
            if params.is_empty() {
                format!(
                    "<green>Bash</green>({DI}\"{}\"{DI_R})",
                    Prose::escape_text(command)
                )
            } else {
                format!(
                    "<green>Bash</green>({DI}\"{} {}\"{DI_R})",
                    Prose::escape_text(command),
                    Prose::escape_text(&truncate_string(params, 30))
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
                params.push(
                    a.iter()
                        .map(|arg| Prose::escape_text(arg))
                        .collect::<Vec<_>>()
                        .join(" "),
                );
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
                Prose::escape_text(&truncate_string(command, 30)),
                params_str
            )
        }
        _ => format!("<dim>{action:?}</dim>"),
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

/// Show detailed event/action configuration for a specific provider.
pub(super) fn run_provider_detail(
    provider: Provider,
    config: Option<&ClaudineConfig>,
) -> Result<()> {
    let term = crate::log::terminal();
    let clients = InstalledAiClients::new();
    let installed = clients.is_installed(provider.sniff_ai_cli());

    let status_icon = if installed { "✅" } else { "❌" };
    let header = Prose::new(format!(
        "<bold>{}</bold> {} <dim>({}installed)</dim>",
        provider,
        status_icon,
        if installed { "" } else { "not " }
    ));
    log::data(&format!("\n {}", header.render(&term)));

    let docs = Prose::new(format!("<dim>{}</dim>", provider.docs_url()));
    log::data(&format!(" {}", docs.render(&term)));
    log::data("");

    let mut event_rows: Vec<(AgenticEvent, Option<&Vec<HookAction>>)> = Vec::new();
    for event in AgenticEvent::ALL {
        let actions = config.and_then(|c| c.actions.get(&event));
        event_rows.push((event, actions));
    }

    let columns = vec![
        TableColumn::new(bold("Event")),
        TableColumn::new(bold("Capture")),
        TableColumn::new(bold("Actions")),
    ];
    let mut table = Table::new()
        .with_columns(columns)
        .prefer_cursor_alignment()
        .alternate_background_color();
    table.layout_mut().margin = Edges::x(Length::ch(1));

    for (event, actions) in &event_rows {
        let support_level = provider.event_support_level(event);
        let support_cell: TableCellContent = match support_level {
            EventSupportLevel::Hook { .. } => "hook".into(),
            EventSupportLevel::StreamParse { protocol, .. } => {
                format!("stream-parse ({protocol:?})").into()
            }
            EventSupportLevel::WireProxy { mode, .. } => format!("wire-proxy ({mode:?})").into(),
            EventSupportLevel::Wrapper { .. } => "wrapper".into(),
            EventSupportLevel::Acp { .. } => Prose::new("<cyan>acp</cyan>").render(&term).into(),
            EventSupportLevel::NotSupported => Prose::new("<dim>-</dim>").render(&term).into(),
        };

        let actions_cell: TableCellContent = match actions {
            None => "-".into(),
            Some(a) if a.is_empty() => Prose::new("<dim>(no actions)</dim>").render(&term).into(),
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

    let total_unified_events = AgenticEvent::ALL.len();
    let configured_count = event_rows
        .iter()
        .filter(|row| row.1.is_some_and(|a| !a.is_empty()))
        .count();

    log::data("");
    let summary = Prose::new(format!(
        "<bold>{}</bold> supports <yellow>{}</yellow> of the <bold><yellow>{}</yellow></bold> unified events",
        provider, configured_count, total_unified_events
    ));
    log::data(&format!(" {}", summary.render(&term)));

    let enabled_events: Vec<&AgenticEvent> = event_rows
        .iter()
        .filter(|(_, actions)| actions.is_some_and(|a| !a.is_empty()))
        .map(|(event, _)| event)
        .collect();

    let unsupported_count = enabled_events
        .iter()
        .filter(|e| !provider.supports_event_via_hook(e))
        .count();

    if !enabled_events.is_empty() {
        log::data("");
        let enabled_header = if unsupported_count > 0 {
            Prose::new(format!(
                "<bold>Event Descriptions</bold> <red>(⚠ {} unsupported)</red>",
                unsupported_count
            ))
        } else {
            Prose::new("<bold>Event Descriptions</bold>")
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
        desc_table.layout_mut().margin = Edges::x(Length::ch(1));

        for event in enabled_events {
            let is_unsupported = !provider.supports_event_via_hook(event);
            let event_cell: TableCellContent = if is_unsupported {
                Prose::new(format!(
                    "<red><strikethrough>{}</strikethrough></red>",
                    event.as_pascal_case()
                ))
                .render(&term)
                .into()
            } else {
                event.as_pascal_case().into()
            };
            let desc_cell: TableCellContent = if is_unsupported {
                Prose::new(format!("<dim>{}</dim>", event.description()))
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

    let unmapped = claudine::provider::provider_info(provider).unmapped_native_events;
    if !unmapped.is_empty() {
        log::data("");
        let header = Prose::new("<bold>Not mappable — configure natively:</bold>");
        log::data(&format!(" {}", header.render(&term)));
        for event in unmapped {
            let line = Prose::new(format!(
                "  <dim>-</dim> {}",
                super::unmapped_event_markup(event)
            ))
            .with_word_wrap(WordWrap::WrapProse(Some(8), Some(4)));
            log::data(&format!(" {}", line.render(&term)));
        }
    }

    super::render_protect_visibility(config);
    if let Some(cfg) = config {
        validate_sound_effects(cfg);
    }

    Ok(())
}

/// Simple table view showing provider, installed status, and subscribed hooks list.
pub(super) fn run_simple(
    agents: &[(Provider, Box<dyn AgentConfigurator>)],
    clients: &InstalledAiClients,
    config: Option<&ClaudineConfig>,
) -> Result<()> {
    let mut table = Table::new().with_columns(vec![
        provider_column(),
        TableColumn::new(bold("Installed")),
        TableColumn::new(bold("Subscribed Hooks")),
    ]);
    table.layout_mut().margin = Edges::x(Length::ch(1));

    let mut has_sync_issues = false;
    let mut has_unsupported_issues = false;

    for provider in ALL_PROVIDERS {
        let installed = clients.is_installed(provider.sniff_ai_cli());
        let configurator = find_configurator(agents, provider);

        let hooks_cell: TableCellContent = if !installed {
            "-".into()
        } else {
            let registered: HashSet<String> = configurator
                .and_then(|cfg| cfg.registered_events(None).ok())
                .unwrap_or_default()
                .into_iter()
                .collect();

            let expected: HashSet<String> = config
                .map(|c| expected_events_for_provider(c, provider, configurator))
                .unwrap_or_default();

            let all_enabled: HashSet<String> = config.map(all_enabled_events).unwrap_or_default();

            let unsupported: HashSet<&String> = all_enabled
                .iter()
                .filter(|e| !is_event_supported(provider, e))
                .collect();

            if registered.is_empty() && expected.is_empty() && unsupported.is_empty() {
                "-".into()
            } else {
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

                let text = formatted.join(", ");
                Prose::new(text)
                    .render(&crate::log::optimistic_terminal(None))
                    .into()
            }
        };

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

    // Unconditional legend: the color coding must be documented even when
    // nothing in the current table is colored.
    log::data("");
    let legend = Prose::new(
        "<dim>- Legend: <yellow>yellow</yellow> = missing (not yet registered), <red>red</red> = stale (registered but no longer configured), <red><strikethrough>strikethrough</strikethrough></red> = unsupported (won't fire)</dim>",
    )
    .with_word_wrap(WordWrap::WrapProse(Some(8), Some(4)));
    log::data(&format!(" {}", legend.render(&term)));

    super::render_protect_visibility(config);

    if let Some(cfg) = config {
        validate_sound_effects(cfg);
    }

    if has_sync_issues || has_unsupported_issues {
        log::data("");
        let hint = Prose::new(
            "<dim>Registration drift detected — run </dim><blue><bold>claudine sync --fix</bold></blue><dim> to reconcile</dim>",
        )
        .with_word_wrap(WordWrap::WrapProse(Some(8), Some(4)));
        log::data(&format!(" {}", hint.render(&term)));
    }

    log::data("");
    let hints = [
        "<dim>- Use <blue><bold>-v</bold></blue> for detailed event matrix</dim>",
        "<dim>- Use <blue><bold>--support</bold></blue> to see which events each provider supports</dim>",
        "<dim>- Use <blue><bold>--mapping</bold></blue> to see native event name mappings</dim>",
        "<dim>- Use <blue><bold>--describe</bold></blue> to see event descriptions and schemas</dim>",
        "<dim>- Use <blue><bold>--variables</bold></blue> to see template variables for speak/report</dim>",
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
pub(super) fn run_verbose(
    agents: &[(Provider, Box<dyn AgentConfigurator>)],
    clients: &InstalledAiClients,
    config: Option<&ClaudineConfig>,
) -> Result<()> {
    let mut columns = vec![provider_column(), TableColumn::new(bold("∃"))];

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
    table.layout_mut().margin = Edges::x(Length::ch(1));

    for provider in ALL_PROVIDERS {
        let installed = clients.is_installed(provider.sniff_ai_cli());
        let _configurator = find_configurator(agents, provider);

        let provider_link = format!(r#"<a href="{}">{}</a>"#, provider.docs_url(), provider);
        let provider_cell: TableCellContent = Prose::new(provider_link)
            .render(&crate::log::optimistic_terminal(None))
            .into();

        let mut row: Vec<TableCellContent> = vec![provider_cell, bool_indicator(installed)];

        for event in AgenticEvent::ALL {
            let cell = if !installed {
                NOT_INSTALLED.into()
            } else if !provider.supports_event(&event) {
                NOT_ALLOWED.into()
            } else {
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

    log::data("");
    let legend = Prose::new(
        "<dim>Legend: </dim>⚠️<dim> = not supported, </dim>-<dim> = not configured, </dim>⓪<dim> = 0 actions, </dim>❶<dim> = 1 action, etc.</dim>",
    ).with_left_margin(TargetValue::universal(Length::ch(8)));
    log::data(&format!(" {}", legend.render(&term)));

    super::render_protect_visibility(config);

    if let Some(cfg) = config {
        validate_sound_effects(cfg);
    }

    log::data("");
    let hints = [
        "<dim>- Use <blue><bold>--support</bold></blue> to see which events each provider supports</dim>",
        "<dim>- Use <blue><bold>--mapping</bold></blue> to see native event name mappings</dim>",
        "<dim>- Use <blue><bold>--describe</bold></blue> to see event descriptions and schemas</dim>",
        "<dim>- Use <blue><bold>--variables</bold></blue> to see template variables for speak/report</dim>",
    ];
    for hint in hints {
        log::data(&format!(" {}", Prose::new(hint).render(&term)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_sound_effect_warning_renders_escaped_name() {
        let config: ClaudineConfig = serde_json::from_str(
            r#"{
  "actions": {
    "human_in_the_loop": [
      { "type": "sound_effect", "effect": "bell<x>" }
    ]
  }
}"#,
        )
        .unwrap();

        let rendered = render_invalid_sound_effects(&config).join("\n");
        assert!(rendered.contains("Invalid sound effects:"));
        assert!(rendered.contains("bell<x>"));
        assert!(!rendered.contains(r"bell\<x\>"));
        assert!(!rendered.contains("{{"));
        assert!(rendered.contains("playa list-effects"));
    }
}
