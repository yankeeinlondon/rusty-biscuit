use chrono::{Days, Local, NaiveDate};
use clap::{Args, Subcommand};
use color_eyre::eyre::{Result, eyre};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Alignment, Margin};
use claudine::events::Provider;
use claudine::reporting::{
    DailySummary, DateRange, ErrorsReport, LabeledCount, ProviderSplit, ReportingFilters,
    ReportingStore, ReposReport, SessionsReport, SyncRequest, SyncSummary, ToolsReport,
    TrendsReport, UsageTotals,
};

use crate::log;
use crate::provider_values::provider_value_parser;

/// Query Claudine's SQLite-backed log reports.
#[derive(Args)]
pub struct LogsArgs {
    #[command(subcommand)]
    pub command: Option<LogsCommand>,

    /// One local date in YYYY-MM-DD format.
    #[arg(long)]
    pub date: Option<String>,

    /// Inclusive range start in YYYY-MM-DD format.
    #[arg(long)]
    pub from: Option<String>,

    /// Inclusive range end in YYYY-MM-DD format.
    #[arg(long)]
    pub to: Option<String>,

    /// Filter by provider.
    #[arg(long, value_parser = provider_value_parser())]
    pub provider: Option<String>,

    /// Filter by repo name or org/name.
    #[arg(long)]
    pub repo: Option<String>,

    /// Filter by monorepo package area.
    #[arg(long = "package-area")]
    pub package_area: Option<String>,

    /// Filter by monorepo package.
    #[arg(long)]
    pub package: Option<String>,

    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,

    /// Limit the number of returned rows for list-style reports.
    #[arg(long, default_value_t = 10)]
    pub top: usize,
}

#[derive(Subcommand, Clone, Copy)]
pub enum LogsCommand {
    /// Daily summary for today.
    Today,
    /// Daily summary for yesterday.
    Yesterday,
    /// Seven-day trend report.
    Week,
    /// Thirty-day trend report.
    Month,
    /// Explicitly sync JSONL logs into SQLite.
    Sync,
    /// Session report for the selected date or range.
    Sessions,
    /// Tool report for the selected date or range.
    Tools,
    /// Error report for the selected date or range.
    Errors,
    /// Repository activity for the selected date or range.
    Repos,
    /// Trend report for the selected date or range.
    Trends,
}

/// Run the `claudine logs` command family.
pub async fn run(args: LogsArgs) -> Result<()> {
    let mut store = ReportingStore::open_default()?;
    let filters = parse_filters(&args)?;
    let command = args.command.unwrap_or(LogsCommand::Today);

    match command {
        LogsCommand::Sync => {
            let request = sync_request_from_args(&args)?;
            let summary = store.sync(request)?;
            if args.json {
                log::data(&serde_json::to_string_pretty(&summary)?);
            } else {
                render_sync_summary(&summary);
            }
        }
        LogsCommand::Today => {
            let date = parse_single_date(args.date.as_deref())?
                .unwrap_or_else(|| Local::now().date_naive());
            best_effort_sync(&mut store, SyncRequest::Date(date));
            let summary = store.daily_summary(date, &filters)?;
            output_json_or(args.json, &summary, || render_daily_summary(&summary))?;
        }
        LogsCommand::Yesterday => {
            let fallback = Local::now()
                .date_naive()
                .checked_sub_days(Days::new(1))
                .unwrap_or_else(|| Local::now().date_naive());
            let date = parse_single_date(args.date.as_deref())?.unwrap_or(fallback);
            best_effort_sync(&mut store, SyncRequest::Date(date));
            let summary = store.daily_summary(date, &filters)?;
            output_json_or(args.json, &summary, || render_daily_summary(&summary))?;
        }
        LogsCommand::Week => {
            let range = default_window(7);
            best_effort_sync(&mut store, SyncRequest::Range(range));
            let report = store.trends(range, &filters, args.top)?;
            output_json_or(args.json, &report, || render_trends_report(&report))?;
        }
        LogsCommand::Month => {
            let range = default_window(30);
            best_effort_sync(&mut store, SyncRequest::Range(range));
            let report = store.trends(range, &filters, args.top)?;
            output_json_or(args.json, &report, || render_trends_report(&report))?;
        }
        LogsCommand::Sessions => {
            let range = range_from_args(&args, Local::now().date_naive())?;
            best_effort_sync(&mut store, SyncRequest::Range(range));
            let report = store.sessions(range, &filters)?;
            output_json_or(args.json, &report, || render_sessions_report(&report))?;
        }
        LogsCommand::Tools => {
            let range = range_from_args(&args, Local::now().date_naive())?;
            best_effort_sync(&mut store, SyncRequest::Range(range));
            let report = store.tools(range, &filters, args.top)?;
            output_json_or(args.json, &report, || render_tools_report(&report))?;
        }
        LogsCommand::Errors => {
            let range = range_from_args(&args, Local::now().date_naive())?;
            best_effort_sync(&mut store, SyncRequest::Range(range));
            let report = store.errors(range, &filters, args.top)?;
            output_json_or(args.json, &report, || render_errors_report(&report))?;
        }
        LogsCommand::Repos => {
            let range = range_from_args(&args, Local::now().date_naive())?;
            best_effort_sync(&mut store, SyncRequest::Range(range));
            let report = store.repos(range, &filters)?;
            output_json_or(args.json, &report, || render_repos_report(&report))?;
        }
        LogsCommand::Trends => {
            let range = range_from_args(&args, default_window(7).to)?;
            best_effort_sync(&mut store, SyncRequest::Range(range));
            let report = store.trends(range, &filters, args.top)?;
            output_json_or(args.json, &report, || render_trends_report(&report))?;
        }
    }

    Ok(())
}

fn output_json_or<T: serde::Serialize>(json: bool, value: &T, render: impl FnOnce()) -> Result<()> {
    if json {
        log::data(&serde_json::to_string_pretty(value)?);
    } else {
        render();
    }

    Ok(())
}

fn parse_filters(args: &LogsArgs) -> Result<ReportingFilters> {
    let provider = args
        .provider
        .as_deref()
        .map(|input| {
            Provider::fuzzy_match_cli_name(input).ok_or_else(|| eyre!("unknown provider `{input}`"))
        })
        .transpose()?;

    Ok(ReportingFilters {
        provider,
        repo: args.repo.clone(),
        package_area: args.package_area.clone(),
        package: args.package.clone(),
    })
}

fn sync_request_from_args(args: &LogsArgs) -> Result<SyncRequest> {
    let date = parse_single_date(args.date.as_deref())?;
    let from = parse_single_date(args.from.as_deref())?;
    let to = parse_single_date(args.to.as_deref())?;

    match (date, from, to) {
        (Some(date), None, None) => Ok(SyncRequest::Date(date)),
        (None, Some(from), Some(to)) => Ok(SyncRequest::Range(DateRange { from, to })),
        (None, None, None) => Ok(SyncRequest::All),
        _ => Err(eyre!(
            "use either `--date YYYY-MM-DD` or `--from YYYY-MM-DD --to YYYY-MM-DD`"
        )),
    }
}

fn range_from_args(args: &LogsArgs, default_date: NaiveDate) -> Result<DateRange> {
    let date = parse_single_date(args.date.as_deref())?;
    let from = parse_single_date(args.from.as_deref())?;
    let to = parse_single_date(args.to.as_deref())?;

    match (date, from, to) {
        (Some(date), None, None) => Ok(DateRange::single(date)),
        (None, Some(from), Some(to)) => Ok(DateRange { from, to }),
        (None, None, None) => Ok(DateRange::single(default_date)),
        _ => Err(eyre!(
            "use either `--date YYYY-MM-DD` or `--from YYYY-MM-DD --to YYYY-MM-DD`"
        )),
    }
}

fn parse_single_date(value: Option<&str>) -> Result<Option<NaiveDate>> {
    value
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(Into::into))
        .transpose()
}

fn default_window(days: u64) -> DateRange {
    let to = Local::now().date_naive();
    let from = to
        .checked_sub_days(Days::new(days.saturating_sub(1)))
        .unwrap_or(to);
    DateRange { from, to }
}

fn best_effort_sync(store: &mut ReportingStore, request: SyncRequest) {
    match store.sync(request) {
        Ok(summary) => {
            if summary.parse_failures > 0 {
                log::warn(&format!(
                    "best-effort sync completed with {} file failures",
                    summary.parse_failures
                ));
            }
        }
        Err(error) => {
            log::warn(&format!("best-effort sync skipped: {error}"));
        }
    }
}

fn render_sync_summary(summary: &SyncSummary) {
    log::data("Sync Summary");
    log::data(&format!(
        "Files scanned:               {}",
        summary.files_scanned
    ));
    log::data(&format!(
        "Files rebuilt:               {}",
        summary.files_rebuilt
    ));
    log::data(&format!(
        "Events inserted:             {}",
        summary.events_inserted
    ));
    log::data(&format!(
        "Events skipped:              {}",
        summary.events_skipped
    ));
    log::data(&format!(
        "Anonymous session fallbacks: {}",
        summary.anonymous_session_fallbacks
    ));
    log::data(&format!(
        "Parse failures:              {}",
        summary.parse_failures
    ));

    if !summary.failures.is_empty() {
        log::data("");
        log::data("Failures");
        for failure in &summary.failures {
            log::data(&format!(
                "- {}:{} {}",
                failure.source_file, failure.line_number, failure.message
            ));
        }
    }
}

fn render_daily_summary(summary: &DailySummary) {
    let term = Terminal::new();

    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Claudine Logs</bold></blue> <dim>▸</dim> <bold>{}</bold>",
            summary.date
        ))
        .render(&term),
    );
    log::data(&format!(
        "Events {}  Sessions {}  Turns {}  Tools {}  Tool errors {}  Turn errors {}",
        summary.total_events,
        summary.session_count,
        summary.total_turns,
        summary.total_tool_calls,
        summary.total_tool_errors,
        summary.total_turn_errors
    ));
    log::data(&format!(
        "Subagents {}  Compactions {}  Permissions {}  Human-in-loop {}  Providers {}  Repos {}",
        summary.total_subagents,
        summary.total_compactions,
        summary.total_permission_requests,
        summary.total_human_in_loop,
        summary.provider_count,
        summary.repo_count
    ));
    log::data(&format!(
        "Provider split: {}",
        render_provider_split(&summary.providers)
    ));

    if !summary.permission_modes.is_empty() {
        log::data(&format!(
            "Permission modes: {}",
            render_labeled_counts(&summary.permission_modes)
        ));
    }

    if !summary.models.is_empty() {
        log::data(&format!(
            "Models: {}",
            render_labeled_counts(&summary.models)
        ));
    }

    render_usage_line(&summary.usage);
    render_metrics_line(&summary.metrics);

    if summary.top_tools.is_empty() {
        return;
    }

    log::data("");
    log::data(&Prose::new("<bold>Top Tools</bold>").render(&term));
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
            format_errors(tool.error_count).into(),
            format!("{:?}", tool.classification).to_lowercase().into(),
        ]);
    }
    log::data(&table.render(&term));
}

fn render_sessions_report(report: &SessionsReport) {
    let term = Terminal::new();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Sessions</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );
    render_metrics_line(&report.metrics);

    let mut table = base_table(vec![
        TableColumn::new("Started"),
        TableColumn::new("Provider"),
        TableColumn::new("Repo"),
        TableColumn::new("Duration").with_alignment(Alignment::Right),
        TableColumn::new("Turns").with_alignment(Alignment::Right),
        TableColumn::new("Tools").with_alignment(Alignment::Right),
        TableColumn::new("Errors").with_alignment(Alignment::Right),
        TableColumn::new("Tokens").with_alignment(Alignment::Right),
        TableColumn::new("Cost").with_alignment(Alignment::Right),
        TableColumn::new("Model"),
    ]);

    for session in &report.sessions {
        let repo = repo_label(session.repo_org.as_deref(), session.repo_name.as_deref());
        let errors = session.tool_error_count + session.turn_error_count;
        table.add_row(vec![
            session
                .started_at
                .format("%Y-%m-%d %H:%M")
                .to_string()
                .into(),
            session.provider.to_string().into(),
            repo.into(),
            format_duration(session.duration_seconds).into(),
            session.turn_count.to_string().into(),
            session.tool_call_count.to_string().into(),
            format_errors(errors).into(),
            format_tokens(session.total_tokens).into(),
            format_cost(session.total_cost_usd).into(),
            session
                .model
                .clone()
                .unwrap_or_else(|| "—".to_string())
                .into(),
        ]);
    }

    log::data(&table.render(&term));
}

fn render_tools_report(report: &ToolsReport) {
    let term = Terminal::new();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Tools</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );
    render_metrics_line(&report.metrics);

    let mut table = base_table(vec![
        TableColumn::new("Tool"),
        TableColumn::new("Calls").with_alignment(Alignment::Right),
        TableColumn::new("Errors").with_alignment(Alignment::Right),
        TableColumn::new("Error %").with_alignment(Alignment::Right),
        TableColumn::new("Class"),
    ]);

    for tool in &report.tools {
        let error_rate = if tool.call_count == 0 {
            0.0
        } else {
            (tool.error_count as f64 / tool.call_count as f64) * 100.0
        };
        table.add_row(vec![
            tool.tool_name.clone().into(),
            tool.call_count.to_string().into(),
            format_errors(tool.error_count).into(),
            format!("{error_rate:.1}").into(),
            format!("{:?}", tool.classification).to_lowercase().into(),
        ]);
    }

    log::data(&table.render(&term));
}

fn render_errors_report(report: &ErrorsReport) {
    let term = Terminal::new();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Errors</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );

    let mut table = base_table(vec![
        TableColumn::new("Time"),
        TableColumn::new("Provider"),
        TableColumn::new("Repo"),
        TableColumn::new("Tool"),
        TableColumn::new("Error"),
        TableColumn::new("Context"),
    ]);

    for item in &report.errors {
        let context = item
            .prompt
            .as_deref()
            .or(item.tool_input_json.as_deref())
            .or(item.notification_message.as_deref())
            .map(|value| truncate_str(value, 120))
            .unwrap_or_else(|| "—".to_string());
        table.add_row(vec![
            item.timestamp.format("%Y-%m-%d %H:%M").to_string().into(),
            item.provider.to_string().into(),
            item.repo_name
                .clone()
                .unwrap_or_else(|| "—".to_string())
                .into(),
            item.tool_name
                .clone()
                .unwrap_or_else(|| "—".to_string())
                .into(),
            truncate_str(&item.error, 120).into(),
            context.into(),
        ]);
    }

    log::data(&table.render(&term));
}

fn render_repos_report(report: &ReposReport) {
    let term = Terminal::new();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Repos</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );

    let mut table = base_table(vec![
        TableColumn::new("Repo"),
        TableColumn::new("Events").with_alignment(Alignment::Right),
        TableColumn::new("Sessions").with_alignment(Alignment::Right),
        TableColumn::new("Branches"),
        TableColumn::new("SHAs").with_alignment(Alignment::Right),
        TableColumn::new("Dirty flips").with_alignment(Alignment::Right),
    ]);

    for repo in &report.repos {
        let label = repo_label(repo.repo_org.as_deref(), Some(repo.repo_name.as_str()));
        table.add_row(vec![
            label.into(),
            repo.event_count.to_string().into(),
            repo.session_count.to_string().into(),
            render_labeled_counts(&repo.branches).into(),
            repo.head_sha_count.to_string().into(),
            repo.dirty_transitions.to_string().into(),
        ]);
    }

    log::data(&table.render(&term));
}

fn render_trends_report(report: &TrendsReport) {
    let term = Terminal::new();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Trends</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );
    if !report.top_tools.is_empty() {
        let top_tools = report
            .top_tools
            .iter()
            .map(|tool| format!("{} {}", tool.tool_name, tool.call_count))
            .collect::<Vec<_>>()
            .join(", ");
        log::data(&format!("Top tools: {top_tools}"));
    }

    let mut table = base_table(vec![
        TableColumn::new("Date").with_min_width(10),
        TableColumn::new("Sessions").with_alignment(Alignment::Right),
        TableColumn::new("Turns").with_alignment(Alignment::Right),
        TableColumn::new("Tools").with_alignment(Alignment::Right),
        TableColumn::new("Tool Err").with_alignment(Alignment::Right),
        TableColumn::new("Turn Err").with_alignment(Alignment::Right),
        TableColumn::new("Providers"),
    ]);

    for point in &report.points {
        table.add_row(vec![
            point.date.to_string().into(),
            point.sessions.to_string().into(),
            point.turns.to_string().into(),
            point.tool_calls.to_string().into(),
            format_errors(point.tool_errors).into(),
            format_errors(point.turn_errors).into(),
            render_provider_split(&point.providers).into(),
        ]);
    }

    log::data(&table.render(&term));

    // Definitions footer
    let definitions = UnorderedList::from(vec![
        RenderableContent::from(Prose::new(
            "<b>Session:</b> <i><dim>one continuous interaction with a provider (start → exit)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Turn:</b> <i><dim>one prompt → response cycle within a session</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Tools:</b> <i><dim>total tool invocations across all turns (Read, Edit, Bash, etc.)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Tool Err:</b> <i><dim>failed tool invocations (e.g. file not found, permission denied, command failed)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Turn Err:</b> <i><dim>failed response cycles (e.g. API errors, rate limits, context overflow)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Providers:</b> <i><dim>active providers, linked to usage dashboards (only providers with turns shown)</dim></i>",
        )),
    ]).with_bullet("  ");
    log::data(&definitions.render(&term));
}

fn base_table(columns: Vec<TableColumn>) -> Table {
    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);
    table
}

fn render_provider_split(items: &[ProviderSplit]) -> String {
    if items.is_empty() {
        return "—".to_string();
    }

    // Sort by turns descending, then by provider name
    let mut sorted: Vec<&ProviderSplit> = items.iter().collect();
    sorted.sort_by(|a, b| {
        b.turns
            .cmp(&a.turns)
            .then_with(|| a.provider.to_string().cmp(&b.provider.to_string()))
    });

    sorted
        .iter()
        .filter_map(|item| {
            if item.turns == 0 {
                return None;
            }
            Some(render_provider_link(&item.provider))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render a provider name as an OSC8 hyperlink to its usage dashboard.
fn render_provider_link(provider: &Provider) -> String {
    match provider.usage_dashboard_url() {
        Some(url) => {
            let markup = format!("<a href=\"{url}\">{provider}</a>");
            Prose::new(markup).render_optimistic(None)
        }
        None => provider.to_string(),
    }
}

fn format_errors(count: u64) -> String {
    let text = count.to_string();
    if count == 0 {
        Prose::new(format!("<dim>{text}</dim>")).render_optimistic(None)
    } else {
        Prose::new(format!("<red>{text}</red>")).render_optimistic(None)
    }
}

fn render_labeled_counts(items: &[LabeledCount]) -> String {
    if items.is_empty() {
        return "—".to_string();
    }

    items
        .iter()
        .map(|item| format!("{} {}", item.label, item.count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn repo_label(repo_org: Option<&str>, repo_name: Option<&str>) -> String {
    match (repo_org, repo_name) {
        (Some(org), Some(name)) => format!("{org}/{name}"),
        (None, Some(name)) => name.to_string(),
        _ => "—".to_string(),
    }
}

fn truncate_str(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    value
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>()
        + "…"
}

fn format_duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn render_usage_line(usage: &UsageTotals) {
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

fn render_metrics_line(metrics: &claudine::reporting::DerivedMetrics) {
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
