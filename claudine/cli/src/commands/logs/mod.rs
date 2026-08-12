use chrono::{Days, Local, NaiveDate};
use clap::{Args, Subcommand};
use color_eyre::eyre::{Result, eyre};

use claudine::provider::Provider;
use claudine::reporting::{DateRange, ReportingFilters, ReportingStore, SyncRequest};

use crate::cli_utils::parse_naive_date;
use crate::log;
use crate::provider_values::provider_value_parser;

mod common;
mod drift;
mod errors;
mod month;
mod repos;
mod sessions;
mod sync;
mod today;
mod tools;
mod trends;
mod week;

use drift::render_drift_report;
use errors::render_errors_report;
use repos::render_repos_report;
use sessions::{render_session_detail, render_sessions_report};
use sync::render_sync_summary;
use today::render_daily_summary;
use tools::render_tools_report;
use trends::render_trends_report;

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
    /// Model-catalog drift signals and alias resolutions for the selected date or range.
    Drift,
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
        LogsCommand::Drift => {
            let range = range_from_args(&args, Local::now().date_naive())?;
            best_effort_sync(&mut store, SyncRequest::Range(range));
            let report = store.drift(range, &filters, args.top)?;
            output_json_or(args.json, &report, || render_drift_report(&report))?;
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

#[cfg(test)]
mod tests {
    use super::common::{render_provider_split, render_repo_entry};
    use super::*;
    use clap::Parser;
    use claudine::reporting::ProviderSplit;

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
    fn parses_drift_subcommand() {
        let cli = TestLogsCli::parse_from(["claudine", "drift"]);
        assert!(matches!(cli.logs.command, Some(LogsCommand::Drift)));
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
