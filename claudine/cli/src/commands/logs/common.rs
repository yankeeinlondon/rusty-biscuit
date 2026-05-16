use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{TerminalRenderable, RenderableTerminalContent};
use biscuit_terminal::terminal::Terminal;
use claudine::provider::Provider;
use claudine::reporting::{LabeledCount, ProviderSplit, UsageTotals};

use crate::log;

pub(super) fn render_provider_split(items: &[ProviderSplit]) -> String {
    if items.is_empty() {
        return "—".to_string();
    }

    let mut sorted: Vec<&ProviderSplit> = items.iter().collect();
    sorted.sort_by(|a, b| {
        b.turns
            .cmp(&a.turns)
            .then_with(|| a.provider.to_string().cmp(&b.provider.to_string()))
    });

    sorted
        .iter()
        .filter_map(|item| {
            if item.count == 0 {
                return None;
            }
            Some(render_provider_link(&item.provider, item.error_count > 0))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render a provider name as an OSC8 hyperlink to its usage dashboard.
pub(super) fn render_provider_link(provider: &Provider, had_error: bool) -> String {
    match provider.usage_dashboard_url() {
        Some(url) => {
            let markup = if had_error {
                format!("<red><a href=\"{url}\">{provider}</a></red>")
            } else {
                format!("<a href=\"{url}\">{provider}</a>")
            };
            Prose::new(markup).render(&crate::log::optimistic_terminal(None))
        }
        None if had_error => Prose::new(format!("<red>{provider}</red>"))
            .render(&crate::log::optimistic_terminal(None)),
        None => provider.to_string(),
    }
}

pub(super) fn format_errors(count: u64) -> String {
    let text = count.to_string();
    if count == 0 {
        Prose::new(format!("<dim>{text}</dim>")).render(&crate::log::optimistic_terminal(None))
    } else {
        Prose::new(format!("<red>{text}</red>")).render(&crate::log::optimistic_terminal(None))
    }
}

pub(super) fn format_dim_zero(count: u64) -> String {
    let text = count.to_string();
    if count == 0 {
        Prose::new(format!("<dim>{text}</dim>")).render(&crate::log::optimistic_terminal(None))
    } else {
        text
    }
}

pub(super) fn format_percent(value: f64) -> String {
    let text = format!("{value:.0}%");
    if value <= 0.0 {
        Prose::new(format!("<dim>{text}</dim>")).render(&crate::log::optimistic_terminal(None))
    } else {
        text
    }
}

pub(super) fn render_repos(repos: &[String], compact: bool) -> String {
    if repos.is_empty() {
        return "—".to_string();
    }

    repos
        .iter()
        .map(|repo| render_repo_entry(repo, compact))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn render_repo_entry(repo: &str, compact: bool) -> String {
    match repo.split_once('/') {
        Some((_org, name)) if compact => name.to_string(),
        Some((org, name)) => Prose::new(format!("<dim>{org}/</dim>{name}"))
            .render(&crate::log::optimistic_terminal(None)),
        None => repo.to_string(),
    }
}

pub(super) fn render_error_hint(term: &Terminal, window: &str) {
    let list = UnorderedList::from(vec![RenderableTerminalContent::from(Prose::new(
        error_hint_markup(window),
    ))])
    .with_bullet("  ");
    log::data(&list.render(term));
}

pub(super) fn error_hint_markup(window: &str) -> String {
    format!(
        "<i><dim>use the <red>claudine logs {window} errors</red> command to list the errors</dim></i>"
    )
}

pub(super) fn render_labeled_counts(items: &[LabeledCount]) -> String {
    if items.is_empty() {
        return "—".to_string();
    }

    items
        .iter()
        .map(|item| format!("{} {}", item.label, item.count))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn repo_label(repo_org: Option<&str>, repo_name: Option<&str>) -> String {
    match (repo_org, repo_name) {
        (Some(org), Some(name)) => format!("{org}/{name}"),
        (None, Some(name)) => name.to_string(),
        _ => "—".to_string(),
    }
}

pub(super) fn truncate_str(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    value
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>()
        + "…"
}

pub(super) fn format_duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

pub(super) fn render_usage_line(usage: &UsageTotals) {
    if usage.total_tokens == 0 && usage.total_cost_usd == 0.0 {
        return;
    }

    let mut parts = Vec::new();
    if usage.total_tokens > 0 {
        parts.push(format!("tokens {}", format_tokens(usage.total_tokens)));
    }
    if usage.total_input_tokens > 0 {
        parts.push(format!("in {}", format_tokens(usage.total_input_tokens)));
    }
    if usage.total_output_tokens > 0 {
        parts.push(format!("out {}", format_tokens(usage.total_output_tokens)));
    }
    if usage.total_cache_read_tokens > 0 {
        parts.push(format!(
            "cached {}",
            format_tokens(usage.total_cache_read_tokens)
        ));
    }
    if usage.total_cost_usd > 0.0 {
        parts.push(format!("cost {}", format_cost(usage.total_cost_usd)));
    }

    log::data(&format!("Usage: {}", parts.join("  ")));
}

pub(super) fn format_tokens(count: u64) -> String {
    if count == 0 {
        return "—".to_string();
    }
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

pub(super) fn format_cost(cost: f64) -> String {
    if cost == 0.0 {
        return "—".to_string();
    }
    if cost < 0.01 {
        format!("${cost:.4}")
    } else {
        format!("${cost:.2}")
    }
}

pub(super) fn render_metrics_line(metrics: &claudine::reporting::DerivedMetrics) {
    let items = [
        metrics
            .autonomy_ratio
            .map(|value| format!("autonomy {value:.2}")),
        metrics
            .research_vs_action_ratio
            .map(|value| format!("research/action {value:.2}")),
        metrics
            .delegation_ratio
            .map(|value| format!("delegation {value:.2}")),
        metrics
            .error_recovery_rate
            .map(|value| format!("recovery {:.0}%", value * 100.0)),
        metrics
            .session_efficiency
            .map(|value| format!("turns/hr {value:.1}")),
        metrics
            .context_pressure_index
            .map(|value| format!("compactions/session {value:.2}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if !items.is_empty() {
        log::data(&format!("Metrics: {}", items.join("  ")));
    }
}

/// Format an event type with color coding by category.
pub(super) fn format_event_label(event: &claudine::events::AgenticEvent) -> String {
    use claudine::events::AgenticEvent;
    let label = event.as_pascal_case();
    let markup = match event {
        AgenticEvent::SessionStart | AgenticEvent::SessionEnd => {
            format!("<bold>{label}</bold>")
        }
        AgenticEvent::ToolError | AgenticEvent::TurnError => {
            format!("<red>{label}</red>")
        }
        AgenticEvent::BeforeTool | AgenticEvent::AfterTool => label.to_string(),
        AgenticEvent::SubagentStart | AgenticEvent::SubagentStop => {
            format!("<blue>{label}</blue>")
        }
        AgenticEvent::BeforeCompact => format!("<yellow>{label}</yellow>"),
        _ => format!("<dim>{label}</dim>"),
    };
    Prose::new(markup).render(&crate::log::optimistic_terminal(None))
}
