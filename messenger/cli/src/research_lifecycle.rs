//! `messenger research prepare|check-run|runs|promote|reject|cleanup`: the
//! refresh and review lifecycle over `messenger::research::refresh`.
//!
//! These commands write only the gitignored state area, except `promote`,
//! which publishes accepted research through the snapshot protocol. None of
//! them starts Claudine or an agent: `prepare` prints the commands to run.

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use clap::Args;
use messenger::research::Loader;
use messenger::research::generate::GenerateError;
use messenger::research::model::{Date, PlatformId};
use messenger::research::publish::Options;
use messenger::research::refresh::check::{CheckReport, check_run};
use messenger::research::refresh::cleanup::{self, CleanupPlan, DEFAULT_THRESHOLD_DAYS};
use messenger::research::refresh::prepare::{self, PreparedRun};
use messenger::research::refresh::promote::{self, Promoted, Request};
use messenger::research::refresh::select::{self, Reason, Skip};
use messenger::research::refresh::state::{LedgerView, StageStatus};
use messenger::research::refresh::{RefreshError, RunLimits, Stage, StateArea};
use serde::Serialize;
use serde_json::json;

use super::research::{EXIT_FINDINGS, EXIT_OK, parse_platform};

/// Invalid arguments, including missing run limits.
pub const EXIT_USAGE: i32 = 2;

#[derive(Debug, Args)]
pub struct PrepareArgs {
    /// Platforms to consider (default: every active roster platform).
    #[arg(value_name = "PLATFORM", value_parser = parse_platform)]
    pub platforms: Vec<PlatformId>,
    /// Refresh the named platforms (or all) even when they are current.
    #[arg(long)]
    pub force: bool,
    /// Elapsed-time limit for each platform run, in seconds. Required; no default.
    #[arg(long, value_name = "SECONDS")]
    pub max_seconds: Option<u64>,
    /// Agent-invocation limit for each platform run. Required; no default.
    #[arg(long, value_name = "COUNT")]
    pub max_invocations: Option<u64>,
    /// A currently observed version (repeatable); a version absent from the
    /// accepted findings makes the platform due.
    #[arg(long = "observed-version", value_name = "INTERFACE=VERSION", value_parser = parse_observed)]
    pub observed: Vec<(String, String)>,
    /// Show the selection without creating runs (limits not required).
    #[arg(long, conflicts_with = "resume")]
    pub dry_run: bool,
    /// Re-prepare a failed, interrupted, or exhausted run from its first
    /// incomplete stage, within its existing budget (at most two times).
    #[arg(long, value_name = "RUN_ID", conflicts_with_all = ["platforms", "force", "max_seconds", "max_invocations", "observed"])]
    pub resume: Option<String>,
    /// Emit JSON instead of styled terminal output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum StageArg {
    Validation,
    Review,
}

#[derive(Debug, Args)]
pub struct CheckRunArgs {
    pub run_id: String,
    /// The last stage that must be complete.
    #[arg(long, value_enum, default_value_t = StageArg::Review)]
    pub through: StageArg,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RunsArgs {
    /// Only this platform.
    #[arg(long, value_parser = parse_platform)]
    pub platform: Option<PlatformId>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct PromoteArgs {
    /// Runs to publish together; an initial publication needs one for every
    /// active platform.
    #[arg(value_name = "RUN_ID", required = true)]
    pub run_ids: Vec<String>,
    /// The maintainer approving this change.
    #[arg(long, value_name = "NAME", required_unless_present = "renewal", conflicts_with = "renewal")]
    pub approved_by: Option<String>,
    /// Accept automatically; refused unless the run is a verified unchanged renewal.
    #[arg(long)]
    pub renewal: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RejectArgs {
    pub run_id: String,
    #[arg(long, value_name = "NAME")]
    pub by: String,
    #[arg(long)]
    pub reason: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CleanupArgs {
    /// Minimum age in days of the records to remove.
    #[arg(long, value_name = "DAYS", default_value_t = DEFAULT_THRESHOLD_DAYS)]
    pub older_than: u32,
    /// Remove the previewed records. Without it nothing is deleted.
    #[arg(long)]
    pub apply: bool,
    #[arg(long)]
    pub json: bool,
}

fn parse_observed(text: &str) -> Result<(String, String), String> {
    match text.split_once('=') {
        Some((interface, version)) if !interface.is_empty() && !version.is_empty() => Ok((interface.to_string(), version.to_string())),
        _ => Err(format!("`{text}` is not INTERFACE=VERSION")),
    }
}

fn pretty<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).expect("research records always serialize")
}

fn prose(text: impl Into<String>, term: &Terminal) -> String {
    format!("{}\n", Prose::new(text.into()).render(term))
}

fn list(items: Vec<String>, term: &Terminal) -> String {
    if items.is_empty() { String::new() } else { format!("{}\n", UnorderedList::new(items).render(term)) }
}

/// Prints a refusal and returns its exit status; environment failures go
/// back to the caller as exit 3.
fn refusal(error: RefreshError, json: bool) -> Result<i32, String> {
    let code = match &error {
        RefreshError::Config(_) => EXIT_USAGE,
        RefreshError::WrongStatus { .. }
        | RefreshError::NotEligible { .. }
        | RefreshError::RecoveryLimit { .. }
        | RefreshError::Ledger { .. }
        | RefreshError::NotInRoster { .. } => EXIT_FINDINGS,
        RefreshError::Generate(inner) if matches!(**inner, GenerateError::Refused { .. }) => EXIT_FINDINGS,
        _ => return Err(error.to_string()),
    };
    let reasons = match &error {
        RefreshError::NotEligible { reasons, .. } => reasons.clone(),
        RefreshError::Generate(inner) => match &**inner {
            GenerateError::Refused { diagnostics, missing } => missing
                .iter()
                .map(|platform| format!("{platform}: no accepted document; promote its run in the same command"))
                .chain(diagnostics.iter().map(ToString::to_string))
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    if json {
        println!("{}", pretty(&json!({ "refused": error.to_string(), "reasons": reasons })));
    } else {
        let term = Terminal::default();
        eprint!("{}", prose(format!("<b>refused:</b> {}", Prose::escape_text(&error.to_string())), &term));
        eprint!("{}", list(reasons, &term));
    }
    Ok(code)
}

// ---- prepare ------------------------------------------------------------------

pub fn prepare(loader: &Loader, args: PrepareArgs, today: &Date) -> Result<i32, String> {
    if let Some(run_id) = &args.resume {
        return match prepare::resume(loader, run_id) {
            Ok(run) => {
                emit_prepared(&[], std::slice::from_ref(&run), args.json);
                Ok(EXIT_OK)
            }
            Err(error) => refusal(error, args.json),
        };
    }
    let request = select::Request {
        forced: if args.force { args.platforms.iter().copied().collect() } else { Default::default() },
        force_all: args.force && args.platforms.is_empty(),
        observed_versions: args.observed.iter().cloned().collect(),
    };
    if args.dry_run {
        let mut selections = select::select(loader, today, &request).map_err(|e| e.to_string())?;
        if !args.platforms.is_empty() {
            selections.retain(|s| args.platforms.contains(&s.platform_id));
        }
        if args.json {
            println!("{}", pretty(&json!({ "selections": selections, "runs": [] })));
        } else {
            print!("{}", render_selections(&selections, &Terminal::default()));
        }
        return Ok(EXIT_OK);
    }
    // Limits are validated before anything is read or written.
    let limits = match RunLimits::new(args.max_seconds, args.max_invocations) {
        Ok(limits) => limits,
        Err(error) => return refusal(error.into(), args.json),
    };
    match prepare::prepare(loader, &args.platforms, limits, today, &request) {
        Ok(prepared) => {
            emit_prepared(&prepared.selections, &prepared.runs, args.json);
            Ok(EXIT_OK)
        }
        Err(error) => refusal(error, args.json),
    }
}

fn reason_text(reason: &Reason) -> String {
    match reason {
        Reason::Missing => "no accepted document".to_string(),
        Reason::Expired { refresh_due } => format!("expired (due {refresh_due})"),
        Reason::Forced => "forced".to_string(),
        Reason::SchemaInvalid { findings } => format!("the accepted document fails validation ({findings} finding(s))"),
        Reason::PromptChanged => "the shared research prompt changed".to_string(),
        Reason::SchemaChanged => "the schema changed".to_string(),
        Reason::NoReviewRecord => "no review or renewal record".to_string(),
        Reason::VersionChanged { interface, observed } => format!("{interface} observed at {observed}"),
    }
}

fn render_selections(selections: &[select::Selection], term: &Terminal) -> String {
    let items = selections
        .iter()
        .map(|selection| match &selection.skip {
            Some(Skip::Current { last_updated, refresh_due }) => {
                format!("{}: skipped, current (last updated {last_updated}, due {refresh_due})", selection.platform_id)
            }
            Some(Skip::OpenRun { run_id, status }) => format!("{}: skipped, run {run_id} is {status}", selection.platform_id),
            None => format!(
                "{}: due ({})",
                selection.platform_id,
                selection.due.iter().map(reason_text).collect::<Vec<_>>().join("; ")
            ),
        })
        .collect();
    format!("{}{}", prose("<b>Refresh selection</b>", term), list(items, term))
}

fn emit_prepared(selections: &[select::Selection], runs: &[PreparedRun], json: bool) {
    if json {
        println!("{}", pretty(&json!({ "selections": selections, "runs": runs })));
        return;
    }
    let term = Terminal::default();
    if !selections.is_empty() {
        print!("{}", render_selections(selections, &term));
    }
    for run in runs {
        let stages: Vec<&str> = run.stages.iter().map(|stage| stage.as_str()).collect();
        print!("{}", prose(format!("<b>Prepared run {}</b> for {}: {}", run.run_id, run.platform_id, stages.join(", ")), &term));
        print!("{}", prose(format!("Run these from the repository root ({}):", Prose::escape_text(&run.run_dir)), &term));
        print!("{}", list(run.commands.iter().map(|command| command.join(" ")).collect(), &term));
    }
    if runs.is_empty() {
        print!("{}", prose("Nothing to prepare.", &term));
    }
}

// ---- check-run -----------------------------------------------------------------

pub fn check(loader: &Loader, args: CheckRunArgs, today: &Date) -> Result<i32, String> {
    let through = match args.through {
        StageArg::Validation => Stage::Validation,
        StageArg::Review => Stage::Review,
    };
    let report = match check_run(loader, &args.run_id, through, today) {
        Ok(report) => report,
        Err(error) => return refusal(error, args.json),
    };
    if args.json {
        println!("{}", pretty(&report));
    } else {
        print!("{}", render_check(&report, &Terminal::default()));
    }
    Ok(if report.passed { EXIT_OK } else { EXIT_FINDINGS })
}

fn render_check(report: &CheckReport, term: &Terminal) -> String {
    let headline = if report.passed {
        format!("<b>Run {} passed through {}</b>; it is {}.", report.run_id, report.through, report.status)
    } else {
        format!("<b>Run {} did not pass through {}</b>; it is {}.", report.run_id, report.through, report.status)
    };
    let mut items = Vec::new();
    for stage in &report.stages {
        items.push(format!("{}: {}", stage.stage, stage.status));
        if stage.status != StageStatus::Complete {
            items.extend(stage.findings.iter().map(|finding| format!("{}: {finding}", stage.stage)));
        }
    }
    format!("{}{}", prose(headline, term), list(items, term))
}

// ---- runs ----------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct RunRow {
    path: String,
    run_id: Option<String>,
    platform_id: Option<PlatformId>,
    status: Option<String>,
    created: Option<Date>,
    next_stage: Option<Stage>,
    recovery_attempts: Option<u32>,
    stop_reason: Option<String>,
    ledger: Option<LedgerView>,
    error: Option<String>,
}

pub fn runs(loader: &Loader, args: RunsArgs) -> Result<i32, String> {
    let state = StateArea::new(loader.workspace());
    let mut rows = Vec::new();
    for (path, record) in state.list() {
        let row = match record {
            Err(error) => RunRow {
                path,
                run_id: None,
                platform_id: None,
                status: None,
                created: None,
                next_stage: None,
                recovery_attempts: None,
                stop_reason: None,
                ledger: None,
                error: Some(error.to_string()),
            },
            Ok(mut record) => {
                let ledger = state.ledger(&record).ok().flatten();
                messenger::research::refresh::state::apply_ledger(&mut record, ledger.as_ref());
                RunRow {
                    path,
                    run_id: Some(record.run_id.to_string()),
                    platform_id: Some(record.platform_id),
                    status: Some(record.status.to_string()),
                    created: Some(record.created.clone()),
                    next_stage: record.next_stage(),
                    recovery_attempts: Some(record.recovery_attempts),
                    stop_reason: record.stop_reason.clone(),
                    ledger,
                    error: None,
                }
            }
        };
        if args.platform.is_none_or(|platform| row.platform_id == Some(platform)) {
            rows.push(row);
        }
    }
    let state_dir = messenger::research::paths::STATE_DIR;
    if args.json {
        println!("{}", pretty(&json!({ "state_dir": state_dir, "runs": rows })));
        return Ok(EXIT_OK);
    }
    let term = Terminal::default();
    print!("{}", prose(format!("<b>Research runs</b> in {state_dir}"), &term));
    let items: Vec<String> = rows
        .iter()
        .map(|row| match (&row.run_id, &row.error) {
            (_, Some(error)) => format!("{}: {error}", row.path),
            (Some(run_id), None) => {
                let mut facts = vec![row.status.clone().unwrap_or_default()];
                if let Some(stage) = row.next_stage {
                    facts.push(format!("next stage {stage}"));
                }
                if let Some(ledger) = &row.ledger {
                    facts.push(format!(
                        "ledger {} ({}/{} invocations, {}/{} s)",
                        ledger.state,
                        ledger.used.invocations,
                        ledger.limits.invocations,
                        ledger.used.active_ms / 1000,
                        ledger.limits.active_ms / 1000
                    ));
                }
                if let Some(reason) = &row.stop_reason {
                    facts.push(reason.clone());
                }
                format!("{} {run_id}: {}", row.platform_id.map(|p| p.to_string()).unwrap_or_default(), facts.join("; "))
            }
            (None, None) => row.path.clone(),
        })
        .collect();
    if items.is_empty() {
        print!("{}", prose("No runs.", &term));
    }
    print!("{}", list(items, &term));
    Ok(EXIT_OK)
}

// ---- promote / reject ------------------------------------------------------------

pub fn promote(loader: &Loader, args: PromoteArgs, today: &Date) -> Result<i32, String> {
    let request = match args.approved_by {
        Some(by) => Request::Human { by },
        None => Request::Renewal,
    };
    let promoted: Vec<Promoted> = match promote::promote(loader, &args.run_ids, &request, today, Options::default()) {
        Ok(promoted) => promoted,
        Err(error) => return refusal(error, args.json),
    };
    if args.json {
        println!("{}", pretty(&json!({ "promoted": promoted })));
        return Ok(EXIT_OK);
    }
    let term = Terminal::default();
    for promoted in &promoted {
        let what = if promoted.kind == "renewal" { "as a verified unchanged renewal" } else { "with human approval" };
        print!("{}", prose(format!("<b>Promoted run {}</b> for {} {what}.", promoted.run_id, promoted.platform_id), &term));
        let mut items = Vec::new();
        if let Some(path) = &promoted.review {
            items.push(format!("review record {path} (commit it with the snapshot)"));
        }
        if let Some(path) = &promoted.renewal {
            items.push(format!("local renewal record {path}"));
        }
        if let Some(generated) = &promoted.generated {
            items.push(format!("snapshot {}: {} artifact(s) replaced", generated.snapshot_id, generated.replaced));
        }
        print!("{}", list(items, &term));
    }
    Ok(EXIT_OK)
}

pub fn reject(loader: &Loader, args: RejectArgs, today: &Date) -> Result<i32, String> {
    match promote::reject(loader, &args.run_id, &args.by, &args.reason, today) {
        Ok(record) => {
            if args.json {
                println!("{}", pretty(&json!({ "run_id": record.run_id, "status": record.status })));
            } else {
                print!("{}", prose(format!("<b>Rejected run {}.</b> It stays local and never reaches the CHANGELOG.", record.run_id), &Terminal::default()));
            }
            Ok(EXIT_OK)
        }
        Err(error) => refusal(error, args.json),
    }
}

// ---- cleanup -------------------------------------------------------------------

pub fn cleanup(loader: &Loader, args: CleanupArgs, today: &Date) -> Result<i32, String> {
    let state = StateArea::new(loader.workspace());
    let plan = cleanup::plan(&state, today, args.older_than);
    let removed = if args.apply { Some(cleanup::apply(&state, &plan).map_err(|e| e.to_string())?) } else { None };
    if args.json {
        println!("{}", pretty(&json!({ "applied": args.apply, "plan": plan, "removed": removed })));
    } else {
        print!("{}", render_cleanup(&plan, removed.as_deref(), &Terminal::default()));
    }
    Ok(EXIT_OK)
}

fn render_cleanup(plan: &CleanupPlan, removed: Option<&[String]>, term: &Terminal) -> String {
    let mut out = match removed {
        None => prose(
            format!(
                "<b>Cleanup preview</b> of {} (records at least {} days old). Nothing was deleted; rerun with --apply.",
                plan.state_dir, plan.threshold_days
            ),
            term,
        ),
        Some(removed) => prose(format!("<b>Removed {} record(s)</b> from {}.", removed.len(), plan.state_dir), term),
    };
    let items: Vec<String> = plan
        .removals
        .iter()
        .map(|removal| {
            let status = removal.status.map(|s| format!(", {s}")).unwrap_or_default();
            let lost = if removal.loses_resumability { "; the run can no longer be resumed" } else { "" };
            format!("{} ({}{status}, {} days){lost}", removal.path, removal.kind, removal.age_days)
        })
        .collect();
    if items.is_empty() {
        out.push_str(&prose("No records are old enough to remove.", term));
    }
    out.push_str(&list(items, term));
    if !plan.protected.is_empty() {
        out.push_str(&prose("<b>Protected</b>", term));
        out.push_str(&list(plan.protected.iter().map(|p| format!("{}: {}", p.path, p.reason)).collect(), term));
    }
    out
}
