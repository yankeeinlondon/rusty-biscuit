//! Daily-summary metrics report — the dual-target proof component.
//!
//! [`MetricsReport`] absorbs the terminal rendering that previously lived in
//! the CLI's `logs today` command (usage line, derived-metrics line, provider
//! split, and top-tools table) and adds the mandated `BrowserRenderable`
//! implementation for report-class components (design ruling 4).
//!
//! The terminal target keeps the original string-building so `claudine logs
//! today` stays byte-identical; the browser target composes `biscuit-terminal`
//! dual-target components (block tags + a `Table` fragment) into a real HTML
//! fragment carrying the same figures.

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{
    BrowserRenderable, RenderableTerminalContent, TerminalRenderable,
};
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Alignment, Edges, Layout, Length};
use renderable::browser::fragment::{BrowserFragment, ComposableNode, Ready};
use renderable::html::tag::BlockTag;

use crate::provider::Provider;
use crate::reporting::{DailySummary, DerivedMetrics, LabeledCount, ProviderSplit, UsageTotals};

/// A rendered daily-summary report.
///
/// Owns the [`DailySummary`] (the [`TerminalRenderable`] contract requires
/// `'static`) plus the error-hint window label and the inline terminal used
/// for colored inline spans. The inline terminal is separate from the display
/// terminal passed to [`render`](TerminalRenderable::render): provider links
/// and error counts render with full color even when the display target is
/// piped, matching the original CLI behavior.
#[derive(Debug)]
pub struct MetricsReport {
    summary: DailySummary,
    error_window: String,
    inline: Terminal,
    layout: Layout,
}

impl MetricsReport {
    /// Build a report for one day's summary.
    ///
    /// The inline terminal defaults to a full-capability optimistic terminal.
    /// Callers that honor plain/`NO_COLOR` mode should override it with
    /// [`with_inline_terminal`](Self::with_inline_terminal).
    pub fn new(summary: DailySummary) -> Self {
        Self {
            summary,
            error_window: "today".to_string(),
            inline: Terminal::new_optimistic(80),
            layout: Layout::default(),
        }
    }

    /// Set the window label used in the "list the errors" hint (e.g. `today`,
    /// `week`).
    pub fn with_error_window(mut self, window: impl Into<String>) -> Self {
        self.error_window = window.into();
        self
    }

    /// Override the terminal used for inline colored spans (provider links,
    /// error counts). Pass the CLI's plain-mode-aware optimistic terminal to
    /// preserve byte-identical output across plain and colored modes.
    pub fn with_inline_terminal(mut self, inline: Terminal) -> Self {
        self.inline = inline;
        self
    }
}

impl TerminalRenderable for MetricsReport {
    fn render(&self, term: &Terminal) -> String {
        let summary = &self.summary;
        let mut lines: Vec<String> = Vec::new();

        lines.push(String::new());
        lines.push(
            Prose::new(format!(
                "<blue><bold>Claudine Logs</bold></blue> <dim>▸</dim> <bold>{}</bold>",
                summary.date
            ))
            .render(term),
        );
        lines.push(format!(
            "Events {}  Sessions {}  Turns {}  Tools {}  Tool errors {}  Turn errors {}",
            summary.total_events,
            summary.session_count,
            summary.total_turns,
            summary.total_tool_calls,
            summary.total_tool_errors,
            summary.total_turn_errors
        ));
        lines.push(format!(
            "Subagents {}  Compactions {}  Permissions {}  Human-in-loop {}  Providers {}  Repos {}",
            summary.total_subagents,
            summary.total_compactions,
            summary.total_permission_requests,
            summary.total_human_in_loop,
            summary.provider_count,
            summary.repo_count
        ));
        lines.push(format!(
            "Provider split: {}",
            provider_split(&summary.providers, &self.inline)
        ));

        if !summary.permission_modes.is_empty() {
            lines.push(format!(
                "Permission modes: {}",
                labeled_counts(&summary.permission_modes)
            ));
        }

        if !summary.models.is_empty() {
            lines.push(format!("Models: {}", labeled_counts(&summary.models)));
        }

        if let Some(usage) = usage_line(&summary.usage) {
            lines.push(usage);
        }
        if let Some(metrics) = metrics_line(&summary.metrics) {
            lines.push(metrics);
        }

        if !summary.top_tools.is_empty() {
            lines.push(String::new());
            lines.push(Prose::new("<bold>Top Tools</bold>").render(term));
            let mut table = base_table(vec![
                TableColumn::new("Tool"),
                TableColumn::new("Calls").with_alignment(Alignment::Right),
                TableColumn::new("Errors").with_alignment(Alignment::Right),
                TableColumn::new("Class"),
            ]);
            for tool in &summary.top_tools {
                table.add_row(vec![
                    tool.tool_name.clone().into(),
                    tool.call_count.to_string().into(),
                    format_errors(tool.error_count, &self.inline).into(),
                    format!("{:?}", tool.classification).to_lowercase().into(),
                ]);
            }
            lines.push(table.render(term));
        }

        if summary.total_tool_errors + summary.total_turn_errors > 0 {
            if !summary.top_tools.is_empty() {
                lines.push(String::new());
            }
            let list = UnorderedList::from(vec![RenderableTerminalContent::from(Prose::new(
                error_hint_markup(&self.error_window),
            ))])
            .with_bullet("  ");
            lines.push(list.render(term));
        }

        lines.join("\n")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

impl BrowserRenderable for MetricsReport {
    /// Composes the daily summary into an HTML `<section>` fragment.
    ///
    /// Carries the same figures as the terminal target — heading, count
    /// paragraphs, provider split, usage and derived-metrics lines, and the
    /// top-tools table (via `Table`'s own browser fragment) — as plain,
    /// structurally-classed HTML with no ANSI.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let summary = &self.summary;
        let base = "claudine-metrics-report";
        let mut section =
            BrowserFragment::new().define_as_block_tag(BlockTag::Section, base);

        section = section.add_component(block_text(
            BlockTag::H2,
            format!("{base}__title"),
            format!("Claudine Logs — {}", summary.date),
        ));
        section = section.add_component(block_text(
            BlockTag::P,
            format!("{base}__counts"),
            format!(
                "Events {} · Sessions {} · Turns {} · Tools {} · Tool errors {} · Turn errors {}",
                summary.total_events,
                summary.session_count,
                summary.total_turns,
                summary.total_tool_calls,
                summary.total_tool_errors,
                summary.total_turn_errors
            ),
        ));
        section = section.add_component(block_text(
            BlockTag::P,
            format!("{base}__totals"),
            format!(
                "Subagents {} · Compactions {} · Permissions {} · Human-in-loop {} · Providers {} · Repos {}",
                summary.total_subagents,
                summary.total_compactions,
                summary.total_permission_requests,
                summary.total_human_in_loop,
                summary.provider_count,
                summary.repo_count
            ),
        ));
        section = section.add_component(block_text(
            BlockTag::P,
            format!("{base}__providers"),
            format!("Provider split: {}", provider_split_plain(&summary.providers)),
        ));

        if !summary.permission_modes.is_empty() {
            section = section.add_component(block_text(
                BlockTag::P,
                format!("{base}__permission-modes"),
                format!("Permission modes: {}", labeled_counts(&summary.permission_modes)),
            ));
        }
        if !summary.models.is_empty() {
            section = section.add_component(block_text(
                BlockTag::P,
                format!("{base}__models"),
                format!("Models: {}", labeled_counts(&summary.models)),
            ));
        }
        if let Some(usage) = usage_line(&summary.usage) {
            section = section.add_component(block_text(
                BlockTag::P,
                format!("{base}__usage"),
                usage,
            ));
        }
        if let Some(metrics) = metrics_line(&summary.metrics) {
            section = section.add_component(block_text(
                BlockTag::P,
                format!("{base}__metrics"),
                metrics,
            ));
        }

        if !summary.top_tools.is_empty() {
            section = section.add_component(block_text(
                BlockTag::H3,
                format!("{base}__top-tools-title"),
                "Top Tools".to_string(),
            ));
            let mut table = Table::new().with_columns(vec![
                TableColumn::new("Tool"),
                TableColumn::new("Calls").with_alignment(Alignment::Right),
                TableColumn::new("Errors").with_alignment(Alignment::Right),
                TableColumn::new("Class"),
            ]);
            for tool in &summary.top_tools {
                table.add_row(vec![
                    tool.tool_name.clone().into(),
                    tool.call_count.to_string().into(),
                    tool.error_count.to_string().into(),
                    format!("{:?}", tool.classification).to_lowercase().into(),
                ]);
            }
            section = section.add_component(BrowserRenderable::render_html_fragment(&table));
        }

        section.finalize()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Build a block-level tag carrying a single escaped text child.
fn block_text(tag: BlockTag, class: String, text: String) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_block_tag(tag, class)
        .add_child(ComposableNode::TextFragment(text))
        .finalize()
}

/// Table with the x-margin the CLI's `base_table` applies, so terminal output
/// matches the original daily-summary table exactly.
fn base_table(columns: Vec<TableColumn>) -> Table {
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().margin = Edges::x(Length::ch(1));
    table
}

/// Colored, OSC8-linked provider split for the terminal target.
fn provider_split(items: &[ProviderSplit], inline: &Terminal) -> String {
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
            Some(provider_link(&item.provider, item.error_count > 0, inline))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Plain provider split for the browser target (no ANSI/OSC8).
fn provider_split_plain(items: &[ProviderSplit]) -> String {
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
            Some(item.provider.to_string())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn provider_link(provider: &Provider, had_error: bool, inline: &Terminal) -> String {
    match provider.usage_dashboard_url() {
        Some(url) => {
            let markup = if had_error {
                format!("<red><a href=\"{url}\">{provider}</a></red>")
            } else {
                format!("<a href=\"{url}\">{provider}</a>")
            };
            Prose::new(markup).render(inline)
        }
        None if had_error => Prose::new(format!("<red>{provider}</red>")).render(inline),
        None => provider.to_string(),
    }
}

fn format_errors(count: u64, inline: &Terminal) -> String {
    let text = count.to_string();
    if count == 0 {
        Prose::new(format!("<dim>{text}</dim>")).render(inline)
    } else {
        Prose::new(format!("<red>{text}</red>")).render(inline)
    }
}

fn labeled_counts(items: &[LabeledCount]) -> String {
    if items.is_empty() {
        return "—".to_string();
    }

    items
        .iter()
        .map(|item| format!("{} {}", item.label, item.count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn usage_line(usage: &UsageTotals) -> Option<String> {
    if usage.total_tokens == 0 && usage.total_cost_usd == 0.0 {
        return None;
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

    Some(format!("Usage: {}", parts.join("  ")))
}

fn metrics_line(metrics: &DerivedMetrics) -> Option<String> {
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

    if items.is_empty() {
        None
    } else {
        Some(format!("Metrics: {}", items.join("  ")))
    }
}

fn format_tokens(count: u64) -> String {
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

fn format_cost(cost: f64) -> String {
    if cost == 0.0 {
        return "—".to_string();
    }
    if cost < 0.01 {
        format!("${cost:.4}")
    } else {
        format!("${cost:.2}")
    }
}

fn error_hint_markup(window: &str) -> String {
    format!(
        "<i><dim>use the <red>claudine logs {window} errors</red> command to list the errors</dim></i>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_summary() -> DailySummary {
        DailySummary {
            date: NaiveDate::from_ymd_opt(2026, 7, 7).unwrap(),
            total_events: 120,
            session_count: 4,
            total_turns: 30,
            total_tool_calls: 88,
            total_tool_errors: 2,
            total_turn_errors: 1,
            total_subagents: 3,
            total_compactions: 1,
            total_permission_requests: 5,
            total_human_in_loop: 2,
            provider_count: 2,
            repo_count: 1,
            providers: vec![ProviderSplit {
                provider: Provider::Claude,
                count: 10,
                turns: 20,
                error_count: 1,
            }],
            top_tools: vec![crate::reporting::DailyToolStat {
                tool_name: "Bash".to_string(),
                call_count: 42,
                error_count: 2,
                classification: crate::reporting::ToolActionClass::Action,
            }],
            permission_modes: vec![LabeledCount {
                label: "acceptEdits".to_string(),
                count: 3,
            }],
            models: vec![LabeledCount {
                label: "sonnet".to_string(),
                count: 4,
            }],
            usage: UsageTotals {
                total_input_tokens: 1_500,
                total_output_tokens: 800,
                total_tokens: 2_300,
                total_cache_read_tokens: 500,
                total_cost_usd: 0.42,
            },
            metrics: DerivedMetrics {
                autonomy_ratio: Some(0.75),
                research_vs_action_ratio: Some(1.20),
                error_recovery_rate: Some(0.5),
                delegation_ratio: Some(0.10),
                session_efficiency: Some(7.5),
                context_pressure_index: Some(0.25),
            },
        }
    }

    fn empty_summary() -> DailySummary {
        DailySummary {
            date: NaiveDate::from_ymd_opt(2026, 7, 7).unwrap(),
            total_events: 0,
            session_count: 0,
            total_turns: 0,
            total_tool_calls: 0,
            total_tool_errors: 0,
            total_turn_errors: 0,
            total_subagents: 0,
            total_compactions: 0,
            total_permission_requests: 0,
            total_human_in_loop: 0,
            provider_count: 0,
            repo_count: 0,
            providers: Vec::new(),
            top_tools: Vec::new(),
            permission_modes: Vec::new(),
            models: Vec::new(),
            usage: UsageTotals::default(),
            metrics: DerivedMetrics::default(),
        }
    }

    #[test]
    fn terminal_render_shows_usage_and_metrics() {
        let term = Terminal::new_optimistic(120);
        let rendered = MetricsReport::new(sample_summary()).render(&term);
        assert!(rendered.contains("Usage:"));
        assert!(rendered.contains("tokens 2.3k"));
        assert!(rendered.contains("cost $0.42"));
        assert!(rendered.contains("Metrics:"));
        assert!(rendered.contains("autonomy 0.75"));
    }

    #[test]
    fn terminal_render_shows_top_tools_table() {
        let term = Terminal::new_optimistic(120);
        let rendered = MetricsReport::new(sample_summary()).render(&term);
        assert!(rendered.contains("Top Tools"));
        assert!(rendered.contains("Bash"));
        assert!(rendered.contains("42"));
    }

    #[test]
    fn terminal_render_empty_day_has_no_usage_or_tools() {
        let term = Terminal::new_optimistic(120);
        let rendered = MetricsReport::new(empty_summary()).render(&term);
        assert!(rendered.contains("Claudine Logs"));
        assert!(rendered.contains("Events 0"));
        assert!(!rendered.contains("Usage:"));
        assert!(!rendered.contains("Metrics:"));
        assert!(!rendered.contains("Top Tools"));
    }

    #[test]
    fn render_browser_fragment_carries_same_figures() {
        let html = BrowserRenderable::render_html_fragment(&MetricsReport::new(sample_summary()))
            .render();
        assert!(html.contains("Claudine Logs"));
        assert!(html.contains("<section"));
        assert!(html.contains("Events 120"));
        assert!(html.contains("tokens 2.3k"));
        assert!(html.contains("autonomy 0.75"));
        assert!(html.contains("Top Tools"));
        assert!(html.contains("Bash"));
        assert!(html.contains("<table"));
    }

    #[test]
    fn render_browser_fragment_empty_day_omits_optional_sections() {
        let html = BrowserRenderable::render_html_fragment(&MetricsReport::new(empty_summary()))
            .render();
        assert!(html.contains("Events 0"));
        assert!(!html.contains("Usage:"));
        assert!(!html.contains("Top Tools"));
    }
}
