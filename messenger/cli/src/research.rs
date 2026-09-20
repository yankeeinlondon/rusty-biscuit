//! `messenger research` — offline maintenance of the provider research
//! contract: validation, deterministic generation and drift checking,
//! reporting, recovery of an interrupted publication, and the refresh and
//! review lifecycle (`research_lifecycle`).
//!
//! Human output uses `TerminalRenderable` components; `--json` prints exactly
//! one JSON document on stdout with no presentation escapes. Nothing here
//! reaches the network, starts an agent, or changes delivery behavior.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use clap::{Args, Subcommand};
use messenger::research::generate::{self, Baseline, Drift, GenerateError, load_fleet};
use messenger::research::model::{Date, PlatformId};
use messenger::research::publish::{self, Options, PublishError};
use messenger::research::report::{self, Enforceability, Filter, Report, coverage_name};
use messenger::research::{Context, Diagnostic, Loader, ResearchError, Scope, Workspace, validate_document};
use serde::Serialize;
use serde_json::json;

use super::research_lifecycle::{self as lifecycle, CheckRunArgs, CleanupArgs, PrepareArgs, PromoteArgs, RejectArgs, RunsArgs};

/// Success.
pub const EXIT_OK: i32 = 0;
/// Findings, drift, or a refused generation.
pub const EXIT_FINDINGS: i32 = 1;
/// The command could not run: no published snapshot, a pending recovery, a
/// held lock, a snapshot that fails verification, or an unreadable input.
pub const EXIT_UNAVAILABLE: i32 = 3;

const RESEARCH_HELP: &str = "\
EXIT STATUS
  0  success
  1  validation findings, drift, a refused generation, or a refused
     lifecycle step (wrong run status, not eligible for promotion)
  2  invalid arguments, including missing run limits, or a research root
     below its Git top level (prepare)
  3  cannot run: no published snapshot, recovery required, lock held,
     a published artifact failed verification, or an input is unreadable";

#[derive(Debug, Args)]
#[command(after_help = RESEARCH_HELP)]
pub struct ResearchArgs {
    /// Repository root (default: the Git repository containing the current directory).
    #[arg(long, global = true, value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Evaluate freshness and override expiry as of this date (YYYY-MM-DD; default: today, UTC).
    #[arg(long, global = true, value_name = "DATE", value_parser = parse_date)]
    pub today: Option<Date>,

    #[command(subcommand)]
    pub command: ResearchCommand,
}

#[derive(Debug, Subcommand)]
pub enum ResearchCommand {
    /// Validate the roster, schemas, platform documents, overrides, mappings, and coverage.
    ///
    /// Without DOCUMENT arguments validates the whole fleet at its accepted
    /// paths. With arguments validates each file as a platform document.
    Validate {
        /// Platform documents to validate instead of the fleet.
        #[arg(value_name = "DOCUMENT")]
        documents: Vec<PathBuf>,
        /// Completeness required of DOCUMENT arguments.
        #[arg(long, value_enum, default_value_t = ScopeArg::Accepted, requires = "documents")]
        scope: ScopeArg,
        /// Emit JSON instead of styled terminal output.
        #[arg(long)]
        json: bool,
    },
    /// Generate the catalog and summary tables and publish the snapshot.
    Generate {
        /// Compare with the published snapshot without writing; fail on drift.
        #[arg(long)]
        check: bool,
        /// Emit JSON instead of styled terminal output.
        #[arg(long)]
        json: bool,
    },
    /// Report constraints, bindings, errors, freshness, coverage, gaps, and handoffs.
    Report {
        /// Only this platform.
        #[arg(long, value_parser = parse_platform)]
        platform: Option<PlatformId>,
        /// Only this interface ID.
        #[arg(long)]
        interface: Option<String>,
        /// Only this operation.
        #[arg(long)]
        operation: Option<String>,
        /// Emit JSON instead of styled terminal output.
        #[arg(long)]
        json: bool,
    },
    /// Complete or undo an interrupted publication.
    Recover {
        /// Emit JSON instead of styled terminal output.
        #[arg(long)]
        json: bool,
    },
    /// Select due platforms and prepare budgeted refresh runs (requires limits).
    Prepare(PrepareArgs),
    /// Judge a run's stage outputs, validate its candidate, and compute its delta.
    CheckRun(CheckRunArgs),
    /// List local refresh runs, their stages, and their budget ledgers.
    Runs(RunsArgs),
    /// Publish a reviewed run: human approval, or a verified unchanged renewal.
    Promote(PromoteArgs),
    /// Reject a run; it stays local and never reaches the CHANGELOG.
    Reject(RejectArgs),
    /// Preview (or, with --apply, remove) old local run and renewal records.
    Cleanup(CleanupArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ScopeArg {
    /// Judged as accepted research: every roster interface, investigated gaps only.
    Accepted,
    /// Judged as a fragment: every present record must be valid.
    Fragment,
}

fn parse_date(text: &str) -> Result<Date, String> {
    Date::parse(text).ok_or_else(|| format!("`{text}` is not a real YYYY-MM-DD calendar date"))
}

pub(crate) fn parse_platform(text: &str) -> Result<PlatformId, String> {
    PlatformId::ALL
        .iter()
        .copied()
        .find(|platform| platform.as_str() == text)
        .ok_or_else(|| {
            let known: Vec<&str> = PlatformId::ALL.iter().map(|p| p.as_str()).collect();
            format!("unknown platform `{text}`; expected one of {}", known.join(", "))
        })
}

/// Runs a research command and returns the process exit status.
pub fn run(args: ResearchArgs) -> i32 {
    let json = match &args.command {
        ResearchCommand::Validate { json, .. }
        | ResearchCommand::Generate { json, .. }
        | ResearchCommand::Report { json, .. }
        | ResearchCommand::Recover { json } => *json,
        ResearchCommand::Prepare(args) => args.json,
        ResearchCommand::CheckRun(args) => args.json,
        ResearchCommand::Runs(args) => args.json,
        ResearchCommand::Promote(args) => args.json,
        ResearchCommand::Reject(args) => args.json,
        ResearchCommand::Cleanup(args) => args.json,
    };
    match execute(args) {
        Ok(code) => code,
        Err(message) => {
            if json {
                println!("{}", pretty(&json!({ "error": message })));
            } else {
                eprint!("{}", Prose::new(format!("<b>error:</b> {}", Prose::escape_text(&message))).render(&terminal()));
                eprintln!();
            }
            EXIT_UNAVAILABLE
        }
    }
}

fn execute(args: ResearchArgs) -> Result<i32, String> {
    let root = resolve_root(args.root.as_deref())?;
    let workspace = Workspace::new(root).map_err(|error| error.to_string())?;
    let loader = Loader::new(workspace);
    let today = args.today.unwrap_or_else(utc_today);
    match args.command {
        ResearchCommand::Validate { documents, scope, json } => validate(&loader, &documents, scope, &today, json),
        ResearchCommand::Generate { check: true, json } => check(&loader, &today, json),
        ResearchCommand::Generate { check: false, json } => generate(&loader, &today, json),
        ResearchCommand::Report { platform, interface, operation, json } => {
            report(&loader, &Filter { platform, interface, operation }, &today, json)
        }
        ResearchCommand::Recover { json } => recover(&loader, json),
        ResearchCommand::Prepare(args) => lifecycle::prepare(&loader, args, &today),
        ResearchCommand::CheckRun(args) => lifecycle::check(&loader, args, &today),
        ResearchCommand::Runs(args) => lifecycle::runs(&loader, args),
        ResearchCommand::Promote(args) => lifecycle::promote(&loader, args, &today),
        ResearchCommand::Reject(args) => lifecycle::reject(&loader, args, &today),
        ResearchCommand::Cleanup(args) => lifecycle::cleanup(&loader, args, &today),
    }
}

fn resolve_root(root: Option<&Path>) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|error| format!("cannot read the current directory: {error}"))?;
    if let Some(root) = root {
        return Ok(std::path::absolute(cwd.join(root)).unwrap_or_else(|_| cwd.join(root)));
    }
    sniff::filesystem::git::api::repo_root(&cwd)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "not inside a Git repository; pass --root".to_string())
}

fn utc_today() -> Date {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    Date::from_unix_days(i64::try_from(seconds / 86_400).unwrap_or(0))
}

fn terminal() -> Terminal {
    Terminal::default()
}

fn pretty<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).expect("research records always serialize")
}

fn research_error(error: ResearchError) -> String {
    error.to_string()
}

/// A word-wrapped bullet list of plain text.
fn list(items: Vec<String>, term: &Terminal) -> String {
    format!("{}\n", UnorderedList::new(items).render(term))
}

// ---- validate ----------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ValidationOutput {
    valid: bool,
    diagnostics: Vec<Diagnostic>,
    missing: Vec<PlatformId>,
}

fn validate(loader: &Loader, documents: &[PathBuf], scope: ScopeArg, today: &Date, json: bool) -> Result<i32, String> {
    let output = if documents.is_empty() {
        let fleet = load_fleet(loader, Baseline::FixedPaths, &BTreeMap::new()).map_err(research_error)?;
        let validation = fleet.validate(today);
        ValidationOutput {
            valid: validation.is_clean(),
            diagnostics: validation.diagnostics,
            missing: validation.missing,
        }
    } else {
        let workspace = loader.workspace();
        let roster = loader.load_roster(&workspace.roster()).map_err(research_error)?;
        let mut diagnostics = messenger::research::validate_roster(&roster, messenger::research::Scope::Fragment);
        let scope = match scope {
            ScopeArg::Accepted => Scope::Accepted,
            ScopeArg::Fragment => Scope::Fragment,
        };
        let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
        for document in documents {
            let path = if document.is_absolute() { document.clone() } else { cwd.join(document) };
            let loaded = loader.load_document(&path).map_err(research_error)?;
            let context = Context { roster: roster.record.as_ref(), scope };
            diagnostics.extend(validate_document(&loaded, &context).diagnostics);
        }
        messenger::research::sort_diagnostics(&mut diagnostics);
        ValidationOutput { valid: diagnostics.is_empty(), diagnostics, missing: Vec::new() }
    };
    if json {
        println!("{}", pretty(&output));
    } else {
        print!("{}", render_validation(&output, &terminal()));
    }
    Ok(if output.valid { EXIT_OK } else { EXIT_FINDINGS })
}

fn render_validation(output: &ValidationOutput, term: &Terminal) -> String {
    let mut out = String::new();
    if output.valid {
        out.push_str(&Prose::new("<b>Research inputs are valid.</b>").render(term));
        out.push('\n');
        return out;
    }
    out.push_str(
        &Prose::new(format!(
            "<b>{} finding(s)</b>, {} platform(s) without a document",
            output.diagnostics.len(),
            output.missing.len()
        ))
        .render(term),
    );
    out.push('\n');
    if !output.missing.is_empty() {
        let items: Vec<String> = output.missing.iter().map(|p| format!("{p}: no accepted document")).collect();
        out.push_str(&list(items, term));
    }
    if !output.diagnostics.is_empty() {
        out.push_str(&diagnostics_list(&output.diagnostics, term));
    }
    out
}

fn diagnostics_list(diagnostics: &[Diagnostic], term: &Terminal) -> String {
    list(diagnostics.iter().map(ToString::to_string).collect(), term)
}

// ---- generate ----------------------------------------------------------------

fn generate(loader: &Loader, today: &Date, json: bool) -> Result<i32, String> {
    match generate::generate(loader, &BTreeMap::new(), today, Options::default()) {
        Ok(generated) => {
            if json {
                println!("{}", pretty(&generated));
            } else {
                let term = terminal();
                let line = if generated.unchanged {
                    format!("<b>Snapshot already current</b> ({})", generated.snapshot_id)
                } else {
                    format!(
                        "<b>Published snapshot</b> {}: {} artifact(s) replaced, {} removed",
                        generated.snapshot_id, generated.replaced, generated.removed
                    )
                };
                println!("{}", Prose::new(line).render(&term));
            }
            Ok(EXIT_OK)
        }
        Err(GenerateError::Refused { diagnostics, missing }) => {
            let output = ValidationOutput { valid: false, diagnostics, missing };
            if json {
                println!("{}", pretty(&json!({ "refused": true, "diagnostics": output.diagnostics, "missing": output.missing })));
            } else {
                let term = terminal();
                print!("{}", Prose::new("<b>Generation refused;</b> the published snapshot is unchanged.").render(&term));
                println!();
                print!("{}", render_validation(&output, &term));
            }
            Ok(EXIT_FINDINGS)
        }
        Err(error) => Err(error.to_string()),
    }
}

fn check(loader: &Loader, today: &Date, json: bool) -> Result<i32, String> {
    let drift = match generate::check(loader, today) {
        Ok(drift) => drift,
        Err(GenerateError::Refused { .. }) => unreachable!("check reports invalid inputs as drift"),
        Err(error) => return Err(error.to_string()),
    };
    if json {
        println!("{}", pretty(&json!({ "drift": drift })));
    } else {
        let term = terminal();
        match &drift {
            None => println!("{}", Prose::new("<b>No drift:</b> the published snapshot matches a fresh generation.").render(&term)),
            Some(Drift::Changed { paths }) => {
                println!("{}", Prose::new("<b>Drift:</b> these published artifacts differ from a fresh generation").render(&term));
                print!("{}", list(paths.clone(), &term));
            }
            Some(Drift::Invalid { diagnostics, missing }) => {
                println!("{}", Prose::new("<b>Drift:</b> the inputs no longer validate").render(&term));
                let output = ValidationOutput { valid: false, diagnostics: diagnostics.clone(), missing: missing.clone() };
                print!("{}", render_validation(&output, &term));
            }
        }
    }
    Ok(if drift.is_none() { EXIT_OK } else { EXIT_FINDINGS })
}

// ---- report ------------------------------------------------------------------

fn report(loader: &Loader, filter: &Filter, today: &Date, json: bool) -> Result<i32, String> {
    let catalog = generate::published_catalog(loader.workspace()).map_err(|error| error.to_string())?;
    let report = report::build(&catalog, filter, today);
    if json {
        println!("{}", pretty(&report));
    } else {
        print!("{}", render_report(&report, &terminal()));
    }
    Ok(EXIT_OK)
}

fn heading(text: &str, term: &Terminal) -> String {
    format!("\n{}\n", Prose::new(format!("<b>{}</b>", Prose::escape_text(text))).render(term))
}

fn empty(text: &str, term: &Terminal) -> String {
    format!("{}\n", Prose::new(format!("<dim>{text}</dim>")).render(term))
}

fn cells(values: Vec<String>) -> Vec<TableCellContent> {
    values.into_iter().map(TableCellContent::from).collect()
}

/// The human report: freshness, coverage, each fact section, implementation
/// gaps, and the per-adapter handoffs. Everything but the small freshness
/// table is a list, so it word-wraps at any terminal width.
pub fn render_report(report: &Report, term: &Terminal) -> String {
    let mut out = String::new();
    out.push_str(&heading("Freshness", term));
    let mut freshness = Table::new().with_columns(vec![
        TableColumn::new("Platform"),
        TableColumn::new("Last updated"),
        TableColumn::new("Refresh due"),
        TableColumn::new("Status"),
    ]);
    for row in &report.freshness {
        freshness.add_row(cells(vec![
            row.platform_id.to_string(),
            row.last_updated.to_string(),
            row.refresh_due.to_string(),
            (if row.stale { "stale" } else { "current" }).to_string(),
        ]));
    }
    out.push_str(&freshness.render(term));

    out.push_str(&heading("Coverage", term));
    let mut coverage: BTreeMap<(String, String), BTreeMap<&str, usize>> = BTreeMap::new();
    for row in &report.coverage {
        *coverage
            .entry((row.platform_id.to_string(), row.interface.clone()))
            .or_default()
            .entry(coverage_name(row.state))
            .or_default() += 1;
    }
    let items: Vec<String> = coverage
        .into_iter()
        .map(|((platform, interface), counts)| {
            let states: Vec<String> = counts.iter().map(|(state, count)| format!("{} {count}", state.replace('_', " "))).collect();
            format!("{platform} {interface}: {}", states.join(", "))
        })
        .collect();
    out.push_str(&list(items, term));

    for section in &report.sections {
        out.push_str(&heading(section.title, term));
        if section.rows.is_empty() {
            out.push_str(&empty("No researched records.", term));
            continue;
        }
        let items: Vec<String> = section
            .rows
            .iter()
            .map(|row| {
                let mut facts = vec![row.platform_id.to_string()];
                facts.extend(row.interface.clone());
                facts.extend(row.state.map(|state| state.to_string()));
                match (row.executable, row.ineligible_reasons.is_empty()) {
                    (Some(true), _) => facts.push("executable".to_string()),
                    (Some(false), false) => facts.push(format!("not executable: {}", row.ineligible_reasons.join(", "))),
                    (Some(false), true) => facts.push("not executable".to_string()),
                    (None, _) => {}
                }
                format!("{} ({}): {}", row.id, facts.join(", "), row.summary)
            })
            .collect();
        out.push_str(&list(items, term));
    }

    out.push_str(&heading("Implementation gaps", term));
    if report.implementation_gaps.is_empty() {
        out.push_str(&empty("None recorded.", term));
    } else {
        let items: Vec<String> = report
            .implementation_gaps
            .iter()
            .map(|gap| format!("{} ({}): {}", gap.source, gap.platform_id, gap.reason))
            .collect();
        out.push_str(&list(items, term));
    }

    out.push_str(&heading("Truncation handoff", term));
    let mut items = Vec::new();
    for handoff in &report.truncation {
        let interface = handoff.interface.as_deref().unwrap_or("unmapped");
        if handoff.surfaces.is_empty() {
            items.push(format!("{} ({interface}): no mapped interface", handoff.adapter));
        }
        for surface in &handoff.surfaces {
            let status = match &surface.enforceability {
                Enforceability::Enforceable => "enforceable".to_string(),
                Enforceability::NotEnforceable { reasons } => format!("not enforceable ({})", reasons.join("; ")),
                Enforceability::NoResearchedBound { gap: Some(gap), .. } => format!("no researched bound (gap {gap})"),
                Enforceability::NoResearchedBound { gap: None, .. } => "no researched bound".to_string(),
            };
            let ids: Vec<&str> = surface.constraints.iter().map(|c| c.id.as_str()).collect();
            let constraints = if ids.is_empty() { String::new() } else { format!("; constraints {}", ids.join(", ")) };
            items.push(format!("{} {} ({interface}): {status}{constraints}", handoff.adapter, surface.surface));
        }
        for loss in &handoff.service_side_loss {
            items.push(format!(
                "{} {}: the service may {} content despite success ({})",
                handoff.adapter, loss.surface, loss.overflow_behavior, loss.id
            ));
        }
    }
    out.push_str(&list(items, term));

    out.push_str(&heading("Diagnostic handoff", term));
    let items: Vec<String> = report
        .diagnostics
        .iter()
        .map(|d| {
            format!(
                "{} ({}): {} envelope(s), {} error(s), {} executable signature(s)",
                d.adapter,
                d.interface.as_deref().unwrap_or("unmapped"),
                d.envelopes.len(),
                d.errors.len(),
                d.errors.iter().filter(|e| e.executable_signature).count()
            )
        })
        .collect();
    out.push_str(&list(items, term));

    let questions: Vec<String> = report
        .truncation
        .iter()
        .flat_map(|h| h.unresolved.iter().map(move |q| format!("{}: {q}", h.adapter)))
        .chain(report.diagnostics.iter().flat_map(|d| d.unresolved.iter().map(move |q| format!("{}: {q}", d.adapter))))
        .collect();
    if !questions.is_empty() {
        out.push_str(&heading("Unresolved questions", term));
        out.push_str(&list(questions, term));
    }
    out
}

// ---- recover -----------------------------------------------------------------

fn recover(loader: &Loader, json: bool) -> Result<i32, String> {
    let outcome = publish::recover(loader.workspace(), Options::default()).map_err(|error: PublishError| error.to_string())?;
    if json {
        println!("{}", pretty(&json!({ "recovery": outcome })));
    } else {
        let text = match outcome {
            publish::Recovery::Clean => "<b>Nothing to recover:</b> no publication was pending.",
            publish::Recovery::RolledBack => "<b>Rolled back:</b> the previous snapshot is selected.",
            publish::Recovery::RolledForward => "<b>Rolled forward:</b> the interrupted publication is complete.",
        };
        println!("{}", Prose::new(text).render(&terminal()));
    }
    Ok(EXIT_OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_platforms_parse_strictly() {
        assert_eq!(parse_date("2026-09-17").unwrap().as_str(), "2026-09-17");
        assert!(parse_date("2026-9-17").is_err());
        assert_eq!(parse_platform("whatsapp").unwrap(), PlatformId::WhatsApp);
        assert!(parse_platform("email").unwrap_err().contains("discord"));
    }

    #[test]
    fn utc_today_is_a_valid_date() {
        assert!(Date::parse(utc_today().as_str()).is_some());
    }
}
