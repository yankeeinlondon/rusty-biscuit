use chrono::{Days, Local, NaiveDate};
use clap::{Args, Subcommand};
use color_eyre::eyre::{Result, eyre};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::components::table::types::ColumnType;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Alignment, WordWrap};
use claudine::events::Provider;
use claudine::reporting::{
    DailySummary, DateRange, ErrorsReport, LabeledCount, ProviderSplit, ReportingFilters,
    ReportingStore, ReposReport, SessionDetailReport, SessionsReport, SyncRequest, SyncSummary,
    ToolsReport, TrendsReport, UsageTotals,
};

use crate::cli_utils::parse_naive_date;
use crate::log;
use crate::provider_values::provider_value_parser;
use crate::table_utils::base_table;

/// Query Claudine's SQLite-backed log reports.
#[derive(Args)]
pub struct LogsArgs {
    #[command(subcommand)]
    pub command: Option<LogsCommand>,

    /// One local date in YYYY-MM-DD format.
    #[arg(long, value_parser = parse_naive_date)]
    pub date: Option<NaiveDate>,

    /// Inclusive range start in YYYY-MM-DD format.
    #[arg(long, value_parser = parse_naive_date)]
    pub from: Option<NaiveDate>,

    /// Inclusive range end in YYYY-MM-DD format.
    #[arg(long, value_parser = parse_naive_date)]
    pub to: Option<NaiveDate>,

    /// Filter by provider.
    #[arg(long, value_parser = provider_value_parser())]
    pub provider: Option<Provider>,

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

#[derive(Args, Debug, Clone, Default)]
pub struct WindowArgs {
    #[command(subcommand)]
    pub command: Option<WindowCommand>,
}

#[derive(Subcommand, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCommand {
    /// List errors for this time window.
    Errors,
}

#[derive(Subcommand, Debug, Clone)]
pub enum LogsCommand {
    /// Daily summary for today.
    Today(WindowArgs),
    /// Daily summary for yesterday.
    Yesterday(WindowArgs),
    /// Seven-day trend report.
    Week(WindowArgs),
    /// Thirty-day trend report.
    Month(WindowArgs),
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
    /// Full detail for a single session.
    Session {
        /// Session key or session ID.
        id: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Run the `claudine logs` command family.
pub async fn run(args: LogsArgs) -> Result<()> {
    let mut store = ReportingStore::open_default()?;
    let filters = parse_filters(&args)?;
    let command = args
        .command
        .clone()
        .unwrap_or(LogsCommand::Today(WindowArgs::default()));

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
        LogsCommand::Today(window) => run_day_window(
            &mut store,
            &args,
            &filters,
            window,
            "today",
            Local::now().date_naive(),
        )?,
        LogsCommand::Yesterday(window) => {
            let fallback = Local::now()
                .date_naive()
                .checked_sub_days(Days::new(1))
                .unwrap_or_else(|| Local::now().date_naive());
            run_day_window(&mut store, &args, &filters, window, "yesterday", fallback)?
        }
        LogsCommand::Week(window) => run_range_window(
            &mut store,
            &args,
            &filters,
            window,
            "week",
            default_window(7),
        )?,
        LogsCommand::Month(window) => run_range_window(
            &mut store,
            &args,
            &filters,
            window,
            "month",
            default_window(30),
        )?,
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
            output_json_or(args.json, &report, || render_trends_report(&report, None))?;
        }
        LogsCommand::Session { id, json } => {
            best_effort_sync(&mut store, SyncRequest::All);
            let report = store.session_detail(&id)?;
            output_json_or(args.json || json, &report, || {
                render_session_detail(&report)
            })?;
        }
    }

    Ok(())
}

fn run_day_window(
    store: &mut ReportingStore,
    args: &LogsArgs,
    filters: &ReportingFilters,
    window: WindowArgs,
    label: &'static str,
    fallback_date: NaiveDate,
) -> Result<()> {
    let date = args.date.unwrap_or(fallback_date);
    let range = DateRange::single(date);
    best_effort_sync(store, SyncRequest::Date(date));

    match window.command {
        Some(WindowCommand::Errors) => {
            let report = store.errors(range, filters, args.top)?;
            output_json_or(args.json, &report, || render_errors_report(&report))?;
        }
        None => {
            let summary = store.daily_summary(date, filters)?;
            output_json_or(args.json, &summary, || {
                render_daily_summary(&summary, Some(label))
            })?;
        }
    }

    Ok(())
}

fn run_range_window(
    store: &mut ReportingStore,
    args: &LogsArgs,
    filters: &ReportingFilters,
    window: WindowArgs,
    label: &'static str,
    range: DateRange,
) -> Result<()> {
    best_effort_sync(store, SyncRequest::Range(range));

    match window.command {
        Some(WindowCommand::Errors) => {
            let report = store.errors(range, filters, args.top)?;
            output_json_or(args.json, &report, || render_errors_report(&report))?;
        }
        None => {
            let report = store.trends(range, filters, args.top)?;
            output_json_or(args.json, &report, || {
                render_trends_report(&report, Some(label))
            })?;
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
    Ok(ReportingFilters {
        provider: args.provider,
        repo: args.repo.clone(),
        package_area: args.package_area.clone(),
        package: args.package.clone(),
    })
}

fn sync_request_from_args(args: &LogsArgs) -> Result<SyncRequest> {
    match (args.date, args.from, args.to) {
        (Some(date), None, None) => Ok(SyncRequest::Date(date)),
        (None, Some(from), Some(to)) => Ok(SyncRequest::Range(DateRange { from, to })),
        (None, None, None) => Ok(SyncRequest::All),
        _ => Err(eyre!(
            "use either `--date YYYY-MM-DD` or `--from YYYY-MM-DD --to YYYY-MM-DD`"
        )),
    }
}

fn range_from_args(args: &LogsArgs, default_date: NaiveDate) -> Result<DateRange> {
    match (args.date, args.from, args.to) {
        (Some(date), None, None) => Ok(DateRange::single(date)),
        (None, Some(from), Some(to)) => Ok(DateRange { from, to }),
        (None, None, None) => Ok(DateRange::single(default_date)),
        _ => Err(eyre!(
            "use either `--date YYYY-MM-DD` or `--from YYYY-MM-DD --to YYYY-MM-DD`"
        )),
    }
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

fn render_daily_summary(summary: &DailySummary, error_hint: Option<&str>) {
    let term = crate::log::terminal();

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

    if !summary.top_tools.is_empty() {
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

    if summary.total_tool_errors + summary.total_turn_errors > 0 {
        if !summary.top_tools.is_empty() {
            log::data("");
        }
        render_error_hint(&term, error_hint.unwrap_or("today"));
    }
}

fn render_sessions_report(report: &SessionsReport) {
    let term = crate::log::terminal();
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
        TableColumn::new("Session ID"),
        TableColumn::new("Provider"),
        TableColumn::new("Repo"),
        TableColumn::new("Duration").with_alignment(Alignment::Right),
        TableColumn::new("Turns").with_alignment(Alignment::Right),
        TableColumn::new("Tools").with_alignment(Alignment::Right),
        TableColumn::new("Errors").with_alignment(Alignment::Right),
        TableColumn::new("Model"),
    ]);

    for session in &report.sessions {
        let repo = if term.width() > 140 {
            repo_label(session.repo_org.as_deref(), session.repo_name.as_deref())
        } else {
            repo_label(None, session.repo_name.as_deref())
        };

        let errors = session.tool_error_count + session.turn_error_count;
        let session_id = session.session_id.as_deref().unwrap_or("—").to_string();
        table.add_row(vec![
            session
                .started_at
                .format("%Y-%m-%d %H:%M")
                .to_string()
                .into(),
            session_id.into(),
            session.provider.to_string().into(),
            repo.into(),
            format_duration(session.duration_seconds).into(),
            session.turn_count.to_string().into(),
            session.tool_call_count.to_string().into(),
            format_errors(errors).into(),
            session
                .model
                .clone()
                .unwrap_or_else(|| "—".to_string())
                .into(),
        ]);
    }

    log::data(&table.render(&term));
}

fn render_session_detail(report: &SessionDetailReport) {
    use claudine::events::AgenticEvent;

    let term = crate::log::terminal();
    let session = &report.session;
    let p = |markup: &str| Prose::new(markup).render(&term);

    // ── Title ──
    log::data("");
    let display_id = session
        .session_id
        .as_deref()
        .unwrap_or(&session.session_key);
    log::data(&p(&format!(
        "<blue><bold>Session</bold></blue> <dim>▸</dim> <bold>{display_id}</bold>"
    )));

    // ── Identity card ──
    let provider_markup = render_provider_link(&session.provider, session.turn_error_count > 0);
    let model = session.model.as_deref().unwrap_or("—");
    let perm = session.permission_mode.as_deref().unwrap_or("—");
    log::data(&p(&format!(
        "  <dim>Provider</dim>  {provider_markup}    <dim>Model</dim>  {model}    <dim>Permission</dim>  {perm}"
    )));

    // Time
    let started = session.started_at.format("%Y-%m-%d %H:%M:%S");
    let duration = format_duration(session.duration_seconds);
    // Determine if session has an explicit end event
    let has_end = report
        .events
        .last()
        .is_some_and(|e| e.event == AgenticEvent::SessionEnd);
    let end_label = if has_end {
        session.ended_at.format("%H:%M:%S").to_string()
    } else {
        format!(
            "{} <dim>(last event, no session_end)</dim>",
            session.ended_at.format("%H:%M:%S")
        )
    };
    log::data(&p(&format!(
        "  <dim>Started</dim>   {started}    <dim>Ended</dim>  {end_label}    <dim>Duration</dim>  {duration}"
    )));

    // Location
    let repo = repo_label(session.repo_org.as_deref(), session.repo_name.as_deref());
    let branch = session.branch.as_deref().unwrap_or("—");
    log::data(&p(&format!(
        "  <dim>Repo</dim>      {repo}    <dim>Branch</dim>  {branch}"
    )));
    if let Some(cwd) = &session.cwd {
        log::data(&p(&format!("  <dim>CWD</dim>       {cwd}")));
    }
    if session.package_area.is_some() || session.package.is_some() {
        let area = session.package_area.as_deref().unwrap_or("—");
        let pkg = session.package.as_deref().unwrap_or("—");
        log::data(&p(&format!(
            "  <dim>Package</dim>   {area} <dim>/</dim> {pkg}"
        )));
    }

    // ── Activity summary ──
    log::data("");
    let turns = session.turn_count;
    let tools = session.tool_call_count;
    let tool_errs = session.tool_error_count;
    let turn_errs = session.turn_error_count;
    let subagents = session.subagent_count;
    let events = session.event_count;

    let mut activity_parts: Vec<String> = Vec::new();
    activity_parts.push(format!("{turns} turns"));
    if tools > 0 {
        activity_parts.push(format!("{tools} tool calls"));
    }
    if subagents > 0 {
        activity_parts.push(format!("{subagents} subagents"));
    }
    activity_parts.push(format!("{events} events"));

    let mut error_parts: Vec<String> = Vec::new();
    if tool_errs > 0 {
        error_parts.push(format!("<red>{tool_errs} tool errors</red>"));
    }
    if turn_errs > 0 {
        error_parts.push(format!("<red>{turn_errs} turn errors</red>"));
    }

    let activity_line = if error_parts.is_empty() {
        activity_parts.join(", ")
    } else {
        format!("{}, {}", activity_parts.join(", "), error_parts.join(", "))
    };
    log::data(&p(&format!("  {activity_line}")));

    // Usage
    let usage = claudine::reporting::UsageTotals {
        total_input_tokens: session.total_input_tokens,
        total_output_tokens: session.total_output_tokens,
        total_tokens: session.total_tokens,
        total_cache_read_tokens: session.total_cache_read_tokens,
        total_cost_usd: session.total_cost_usd,
    };
    if usage.total_tokens > 0 || usage.total_cost_usd > 0.0 {
        let mut parts = Vec::new();
        if usage.total_tokens > 0 {
            parts.push(format!(
                "<dim>tokens</dim> {}",
                format_tokens(usage.total_tokens)
            ));
        }
        if usage.total_input_tokens > 0 {
            parts.push(format!(
                "<dim>in</dim> {}",
                format_tokens(usage.total_input_tokens)
            ));
        }
        if usage.total_output_tokens > 0 {
            parts.push(format!(
                "<dim>out</dim> {}",
                format_tokens(usage.total_output_tokens)
            ));
        }
        if usage.total_cache_read_tokens > 0 {
            parts.push(format!(
                "<dim>cached</dim> {}",
                format_tokens(usage.total_cache_read_tokens)
            ));
        }
        if usage.total_cost_usd > 0.0 {
            parts.push(format!(
                "<dim>cost</dim> {}",
                format_cost(usage.total_cost_usd)
            ));
        }
        log::data(&p(&format!("  {}", parts.join("  "))));
    }

    // ── Tools breakdown ──
    if !report.tools.is_empty() {
        log::data("");
        log::data(&p("<bold>Tools</bold>"));
        let mut table = base_table(vec![
            TableColumn::new("Tool"),
            TableColumn::new("Calls").with_alignment(Alignment::Right),
            TableColumn::new("Errors").with_alignment(Alignment::Right),
            TableColumn::new("Class"),
        ]);
        for tool in &report.tools {
            table.add_row(vec![
                tool.tool_name.clone().into(),
                tool.call_count.to_string().into(),
                format_errors(tool.error_count).into(),
                format!("{:?}", tool.classification).to_lowercase().into(),
            ]);
        }
        log::data(&table.render(&term));
    }

    // ── Event timeline ──
    if !report.events.is_empty() {
        log::data("");
        log::data(&p("<bold>Timeline</bold>"));

        let mut table = base_table(vec![
            TableColumn::new("").with_fixed_width(3),
            TableColumn::new("Time").with_fixed_width(8),
            TableColumn::new("Event"),
            TableColumn::new("Detail").with_word_wrap(WordWrap::WrapProse(None, None)),
        ]);

        for event in &report.events {
            let icon = event.event.abbrev();
            let time = event.timestamp.format("%H:%M:%S").to_string();
            let event_label = format_event_label(&event.event);

            let mut detail_parts: Vec<String> = Vec::new();

            // Tool or agent name
            if let Some(tool) = &event.tool_name {
                detail_parts.push(tool.clone());
            }
            if let Some(agent) = &event.agent_type {
                detail_parts.push(format!("agent:{agent}"));
            }

            // Token/cost info (only when nonzero)
            if event.total_tokens > 0 || event.cost_usd > 0.0 {
                let mut token_parts = Vec::new();
                if event.total_tokens > 0 {
                    token_parts.push(format_tokens(event.total_tokens));
                }
                if event.cost_usd > 0.0 {
                    token_parts.push(format_cost(event.cost_usd));
                }
                detail_parts.push(token_parts.join(" "));
            }

            // Error, prompt, or notification
            if let Some(error) = &event.error {
                detail_parts.push(
                    Prose::new(format!("<red>{}</red>", truncate_str(error, 80)))
                        .render(&crate::log::optimistic_terminal(None)),
                );
            } else if let Some(prompt) = &event.prompt {
                detail_parts.push(
                    Prose::new(format!("<dim>\"{}\"</dim>", truncate_str(prompt, 80)))
                        .render(&crate::log::optimistic_terminal(None)),
                );
            } else if let Some(msg) = &event.notification_message {
                detail_parts.push(
                    Prose::new(format!("<dim>{}</dim>", truncate_str(msg, 80)))
                        .render(&crate::log::optimistic_terminal(None)),
                );
            }

            table.add_row(vec![
                icon.to_string().into(),
                time.into(),
                event_label.into(),
                detail_parts.join("  ").into(),
            ]);
        }

        log::data(&table.render(&term));
    }

    // ── Errors detail ──
    if !report.errors.is_empty() {
        log::data("");
        log::data(&p("<bold>Errors</bold>"));
        for (index, item) in report.errors.iter().enumerate() {
            let mut lines = vec![format!(
                "<dim>─── {} ───</dim> {} <dim>{}</dim>",
                index + 1,
                item.timestamp.format("%H:%M:%S"),
                item.event.as_slug(),
            )];
            if let Some(tool) = &item.tool_name {
                lines.push(format!("  <dim>Tool:</dim>   {tool}"));
            }
            if let Some(model) = &item.model {
                lines.push(format!("  <dim>Model:</dim>  {model}"));
            }
            lines.push(format!(
                "  <red>{}</red>",
                if item.error.is_empty() {
                    "(no details)"
                } else {
                    &item.error
                }
            ));
            if let Some(prompt) = &item.prompt {
                lines.push(format!(
                    "  <dim>Prompt:</dim> {}",
                    truncate_str(prompt, 200)
                ));
            }
            log::data(&p(&lines.join("\n")));
        }
    }
}

/// Format an event type with color coding by category.
fn format_event_label(event: &claudine::events::AgenticEvent) -> String {
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

fn render_tools_report(report: &ToolsReport) {
    let term = crate::log::terminal();
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
    let term = crate::log::terminal();
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
        TableColumn::new("Session"),
        TableColumn::new("Error"),
    ]);

    for item in &report.errors {
        let session_display = item.session_id.as_deref().unwrap_or("—").to_string();
        let error_display = if item.error.is_empty() {
            "(no details)".to_string()
        } else {
            truncate_str(&item.error, 120)
        };
        table.add_row(vec![
            item.timestamp.format("%Y-%m-%d %H:%M").to_string().into(),
            item.provider.to_string().into(),
            session_display.into(),
            error_display.into(),
        ]);
    }

    log::data(&table.render(&term));

    // Show additional detail per error when there's info beyond what the table shows.
    for (index, item) in report.errors.iter().enumerate() {
        let has_detail = item.prompt.is_some() || item.tool_name.is_some() || item.model.is_some();
        if !has_detail {
            continue;
        }

        let mut lines = vec![format!("<dim>─── Error {} ───</dim>", index + 1)];
        if let Some(model) = &item.model {
            lines.push(format!("  <dim>Model:</dim>  {model}"));
        }
        if let Some(tool) = &item.tool_name {
            lines.push(format!("  <dim>Tool:</dim>   {tool}"));
        }
        if let Some(prompt) = &item.prompt {
            let display = truncate_str(prompt, 200);
            lines.push(format!("  <dim>Prompt:</dim> {display}"));
        }

        log::data(&Prose::new(lines.join("\n")).render(&term));
    }
}

fn render_repos_report(report: &ReposReport) {
    let term = crate::log::terminal();
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

fn render_trends_report(report: &TrendsReport, error_hint: Option<&str>) {
    let term = crate::log::terminal();
    let compact_repos = term.width() < 150;
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Trends</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );
    log::data("");
    // if !report.top_tools.is_empty() {
    //     let top_tools = report
    //         .top_tools
    //         .iter()
    //         .map(|tool| format!("{} {}", tool.tool_name, tool.call_count))
    //         .collect::<Vec<_>>()
    //         .join(", ");
    //     log::data(&format!("Top tools: {top_tools}"));
    // }

    let _tool_calls = if term.width() > 120 {
        Some(TableColumn::new("Tool\nCalls").with_type(ColumnType::Integer))
    } else {
        None
    };

    let mut table = base_table(vec![
        TableColumn::new("Date").with_fixed_width(10),
        TableColumn::new("Wrap")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center),
        TableColumn::new("Unwrap")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center),
        TableColumn::new("Non-\nInt")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center),
        TableColumn::new("Yolo")
            .with_alignment(Alignment::Center)
            .with_fixed_width(4),
        TableColumn::new("Tool\nCalls")
            .with_type(ColumnType::Integer)
            .with_word_wrap(WordWrap::None),
        // TableColumn::new("Tool\nErrs")
        //     .with_type(ColumnType::Integer)
        //     .with_alignment(Alignment::Center),
        TableColumn::new("Turn\nErrs")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center)
            .with_fixed_width(4),
        TableColumn::new("Repos").with_word_wrap(WordWrap::WrapProse(None, None)),
        TableColumn::new("Providers").with_word_wrap(WordWrap::WrapProse(None, None)),
    ])
    .alternate_background_color();

    for point in &report.points {
        table.add_row(vec![
            point.date.to_string().into(),
            format_dim_zero(point.wrapped).into(),
            format_dim_zero(point.unwrapped).into(),
            format_dim_zero(point.non_interactive).into(),
            format_percent(point.yolo_percent).into(),
            point.tool_calls.to_string().into(),
            // format_errors(point.tool_errors).into(),
            format_errors(point.turn_errors).into(),
            render_repos(&point.repos, compact_repos).into(),
            render_provider_split(&point.providers).into(),
        ]);
    }

    log::data(&table.render(&term));

    // Definitions footer
    let mut definitions = vec![
        RenderableContent::from(Prose::new(
            "<b>Wrapped:</b> <i><dim>interactive sessions where <blue>claudine</blue> is wrapping the execution of an Agent (e.g. `claudine claude`, `claudine codex`, etc.)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Unwrapped:</b> <i><dim>sessions detected via hooks, execution was not wrapped by <blue>claudine</blue></dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Non Interactive:</b> <i><dim>non-interactive sessions wrapped by <blue>claudine</blue></dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Yolo:</b> <i><dim>percentage of daily sessions that ran with YOLO enabled</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Tools:</b> <i><dim>total tool invocations across all sessions (Read, Edit, Bash, etc.)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Tool Err:</b> <i><dim>failed tool calls (file not found, permission denied, command failed)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Turn Err:</b> <i><dim>failed response cycles (API errors, rate limits, context overflow)</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Repos:</b> <i><dim>repositories worked on that day</dim></i>",
        )),
        RenderableContent::from(Prose::new(
            "<b>Providers:</b> <i><dim>active agentic providers with usage dashboard links; providers with errors are shown in red</dim></i>",
        )),
    ];
    if report
        .points
        .iter()
        .any(|point| point.tool_errors + point.turn_errors > 0)
    {
        definitions.push(RenderableContent::from(Prose::new(error_hint_markup(
            error_hint.unwrap_or("week"),
        ))));
    }
    let definitions = UnorderedList::from(definitions).with_bullet("  ");
    log::data(&definitions.render(&term));
}

fn render_provider_split(items: &[ProviderSplit]) -> String {
    if items.is_empty() {
        return "—".to_string();
    }

    // Sort by turns descending, then by provider name.
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
fn render_provider_link(provider: &Provider, had_error: bool) -> String {
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

fn format_errors(count: u64) -> String {
    let text = count.to_string();
    if count == 0 {
        Prose::new(format!("<dim>{text}</dim>")).render(&crate::log::optimistic_terminal(None))
    } else {
        Prose::new(format!("<red>{text}</red>")).render(&crate::log::optimistic_terminal(None))
    }
}

fn format_dim_zero(count: u64) -> String {
    let text = count.to_string();
    if count == 0 {
        Prose::new(format!("<dim>{text}</dim>")).render(&crate::log::optimistic_terminal(None))
    } else {
        text
    }
}

fn format_percent(value: f64) -> String {
    let text = format!("{value:.0}%");
    if value <= 0.0 {
        Prose::new(format!("<dim>{text}</dim>")).render(&crate::log::optimistic_terminal(None))
    } else {
        text
    }
}

fn render_repos(repos: &[String], compact: bool) -> String {
    if repos.is_empty() {
        return "—".to_string();
    }

    repos
        .iter()
        .map(|repo| render_repo_entry(repo, compact))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_repo_entry(repo: &str, compact: bool) -> String {
    match repo.split_once('/') {
        Some((_org, name)) if compact => name.to_string(),
        Some((org, name)) => Prose::new(format!("<dim>{org}/</dim>{name}"))
            .render(&crate::log::optimistic_terminal(None)),
        None => repo.to_string(),
    }
}

fn render_error_hint(term: &Terminal, window: &str) {
    let list = UnorderedList::from(vec![RenderableContent::from(Prose::new(
        error_hint_markup(window),
    ))])
    .with_bullet("  ");
    log::data(&list.render(term));
}

fn error_hint_markup(window: &str) -> String {
    format!(
        "<i><dim>use the <red>claudine logs {window} errors</red> command to list the errors</dim></i>"
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestLogsCli {
        #[command(flatten)]
        logs: LogsArgs,
    }

    #[test]
    fn parses_week_errors_as_nested_window_subcommand() {
        let cli = TestLogsCli::parse_from(["claudine", "week", "errors"]);

        match cli.logs.command {
            Some(LogsCommand::Week(window)) => {
                assert_eq!(window.command, Some(WindowCommand::Errors));
            }
            other => panic!("expected week errors, got {other:?}"),
        }
    }

    #[test]
    fn render_provider_split_keeps_error_only_provider_visible() {
        let rendered = render_provider_split(&[ProviderSplit {
            provider: Provider::Claude,
            count: 2,
            turns: 0,
            error_count: 1,
        }]);

        assert!(rendered.contains("Claude"));
    }

    #[test]
    fn render_repo_entry_drops_org_in_compact_mode() {
        assert_eq!(
            render_repo_entry("yankeeinlondon/rusty-biscuit", true),
            "rusty-biscuit"
        );
    }
}
