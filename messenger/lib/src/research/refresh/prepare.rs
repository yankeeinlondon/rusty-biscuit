//! Preparing a run: limits, selection, separately prepared pass inputs, and
//! the Claudine sequence document.
//!
//! Each pass reads only its own `inputs/<pass>/` directory. Discovery gets the
//! platform identification, the shared instructions, and the schema: never the
//! previous document or the curated sources. Reconciliation gets both, plus
//! the discovery outputs. Messenger writes these files and returns the
//! commands to run; it never starts Claudine or an agent itself.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::RefreshError;
use super::config::RunLimits;
use super::select::{self, Reason, Request, Selection};
use super::state::{
    BaselineRef, MAX_RECOVERY_ATTEMPTS, RUN_FORMAT, RunId, RunRecord, RunStatus, Stage, StageResult, StageStatus, StateArea, io_err,
};
use crate::research::canonical::text_fingerprint;
use crate::research::load::Loader;
use crate::research::model::{Date, PlatformId, Roster, RosterPlatform};
use crate::research::paths::document_path;

/// A run ready for Claudine.
#[derive(Debug, Clone, Serialize)]
pub struct PreparedRun {
    pub run_id: RunId,
    pub platform_id: PlatformId,
    /// Repository-relative run directory.
    pub run_dir: String,
    /// Stages the sequence document runs, in order.
    pub stages: Vec<Stage>,
    /// Commands to run from the repository root, as argument vectors. A
    /// resumption omits `budget init`: it keeps the ledger and its consumption.
    pub commands: Vec<Vec<String>>,
}

/// The outcome of preparing the due platforms.
#[derive(Debug, Clone, Serialize)]
pub struct Prepared {
    pub selections: Vec<Selection>,
    pub runs: Vec<PreparedRun>,
}

/// Selects due platforms among `platforms` (every active platform when
/// empty) and prepares a run for each. `limits` were validated by
/// [`RunLimits::new`] before anything is read.
///
/// ## Errors
///
/// [`RefreshError`] when selection or writing fails; runs prepared before a
/// failure remain and are listed by `runs`.
pub fn prepare(
    loader: &Loader,
    platforms: &[PlatformId],
    limits: RunLimits,
    today: &Date,
    request: &Request,
) -> Result<Prepared, RefreshError> {
    let mut selections = select::select(loader, today, request)?;
    if !platforms.is_empty() {
        for platform in platforms {
            if selections.iter().all(|s| s.platform_id != *platform) {
                return Err(RefreshError::NotInRoster { platform: platform.to_string() });
            }
        }
        selections.retain(|s| platforms.contains(&s.platform_id));
    }
    let roster = loader.load_roster(&loader.workspace().roster())?.record.ok_or(RefreshError::InvalidRoster)?;
    let mut runs = Vec::new();
    for selection in selections.iter().filter(|s| s.is_due()) {
        runs.push(create_run(loader, &roster, selection.platform_id, &selection.due, limits, today)?);
    }
    Ok(Prepared { selections, runs })
}

fn create_run(
    loader: &Loader,
    roster: &Roster,
    platform_id: PlatformId,
    due: &[Reason],
    limits: RunLimits,
    today: &Date,
) -> Result<PreparedRun, RefreshError> {
    let workspace = loader.workspace();
    let state = StateArea::new(workspace);
    let platform = roster.platform(platform_id).ok_or_else(|| RefreshError::NotInRoster { platform: platform_id.to_string() })?;
    let verified = select::published(workspace)?;
    let published = verified.as_ref().and_then(|v| v.files.get(&document_path(platform_id)));
    // Initial migration: legacy prose at the fixed path is reconciliation input,
    // never a baseline.
    let previous = match published {
        Some(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        None => fs::read_to_string(workspace.document(platform_id)).ok(),
    };
    let record = RunRecord {
        format: RUN_FORMAT.to_string(),
        run_id: RunId::generate(today, platform_id),
        platform_id,
        created: today.clone(),
        status: RunStatus::Active,
        selected_because: due.iter().map(|reason| reason.code().to_string()).collect(),
        limits,
        baseline: published.map(|bytes| BaselineRef { xxh64: text_fingerprint(&String::from_utf8_lossy(bytes)) }),
        researched_under: select::current_contract(workspace)?,
        recovery_attempts: 0,
        ledger_runs: 0,
        stages: Stage::ALL.iter().map(|stage| pending(*stage)).collect(),
        stop_reason: None,
        decision: None,
    };
    let dir = state.create(&record)?;
    write_inputs(loader, &dir, platform, roster.curated_source_cap, previous.as_deref())?;
    let stages = Stage::ALL.to_vec();
    write_sequence(loader, &dir, &record, &stages)?;
    Ok(prepared(&record, stages, true))
}

/// Re-prepares a failed, interrupted, or exhausted run from its first
/// incomplete stage (rerunning reconciliation when validation failed). The
/// same ledger keeps charging; a resumption never adds budget, and at most
/// [`MAX_RECOVERY_ATTEMPTS`] are allowed.
///
/// ## Errors
///
/// [`RefreshError::WrongStatus`], [`RefreshError::RecoveryLimit`],
/// [`RefreshError::Ledger`] for an exhausted ledger without a recorded grant.
pub fn resume(loader: &Loader, run_id: &str) -> Result<PreparedRun, RefreshError> {
    let state = StateArea::new(loader.workspace());
    let mut record = state.load(run_id)?;
    let ledger = state.ledger(&record)?;
    super::state::apply_ledger(&mut record, ledger.as_ref());
    if !matches!(record.status, RunStatus::Failed | RunStatus::Interrupted | RunStatus::Exhausted) {
        return Err(RefreshError::WrongStatus {
            run_id: record.run_id.to_string(),
            status: record.status,
            action: "resuming",
            needs: "a failed, interrupted, or exhausted run",
        });
    }
    if record.recovery_attempts >= MAX_RECOVERY_ATTEMPTS {
        return Err(RefreshError::RecoveryLimit { run_id: record.run_id.to_string() });
    }
    if let Some(ledger) = &ledger {
        let guidance = match ledger.state.as_str() {
            "exhausted" => Some("an operator must record a grant with `claudine budget grant` first"),
            "interrupted" | "suspended" => Some("an operator must run `claudine budget resume` first"),
            "active" => Some("a runner still holds it"),
            _ => None,
        };
        if let Some(guidance) = guidance {
            state.save(&record)?;
            return Err(RefreshError::Ledger { run_id: record.run_id.to_string(), state: ledger.state.clone(), guidance });
        }
    }
    let first = record.next_stage().unwrap_or(Stage::Review);
    let first = if first == Stage::Validation { Stage::Reconcile } else { first };
    let stages: Vec<Stage> = Stage::ALL.iter().copied().filter(|stage| *stage >= first).collect();
    for result in &mut record.stages {
        if stages.contains(&result.stage) {
            *result = pending(result.stage);
        }
    }
    record.recovery_attempts += 1;
    record.ledger_runs = ledger.as_ref().map_or(0, |ledger| ledger.runs);
    record.status = RunStatus::Active;
    record.stop_reason = None;
    state.save(&record)?;
    let dir = state.run_dir(record.platform_id, &record.run_id);
    write_sequence(loader, &dir, &record, &stages)?;
    Ok(prepared(&record, stages, false))
}

fn pending(stage: Stage) -> StageResult {
    StageResult { stage, status: StageStatus::Pending, findings: Vec::new() }
}

fn prepared(record: &RunRecord, stages: Vec<Stage>, init: bool) -> PreparedRun {
    let run_dir = format!("{}/{}/{}", super::state::RUNS, record.platform_id, record.run_id);
    let ledger = format!("{run_dir}/budget.json");
    let mut commands = Vec::new();
    if init {
        commands.push(
            [
                "claudine", "budget", "init", &ledger, "--run-id", record.run_id.as_str(), "--platform", record.platform_id.as_str(),
                "--max-seconds", &record.limits.max_seconds.to_string(), "--max-invocations", &record.limits.max_invocations.to_string(),
                "--exclusive-lock", "../../fleet.lock",
            ]
            .map(str::to_string)
            .to_vec(),
        );
    }
    // `--yolo` approves the `shell:` check steps without a terminal.
    commands.push(
        ["claudine", "sequence", "--yolo", "--budget-ledger", &ledger, &format!("{run_dir}/run.md")].map(str::to_string).to_vec(),
    );
    PreparedRun { run_id: record.run_id.clone(), platform_id: record.platform_id, run_dir, stages, commands }
}

const CONTRACT_FILES: &[&str] = &["_fleet.md", "_schema.yaml", "_types.yaml", "_rules.md"];

fn copy_contract(loader: &Loader, into: &Path, files: &[&str]) -> Result<(), RefreshError> {
    let source = loader.workspace().documents_dir();
    for name in files {
        let bytes = fs::read(source.join(name)).map_err(io_err(format!("read {name}")))?;
        fs::write(into.join(name), bytes).map_err(io_err(format!("copy {name}")))?;
    }
    Ok(())
}

fn mkdir(path: &Path) -> Result<PathBuf, RefreshError> {
    fs::create_dir_all(path).map_err(io_err(format!("create {}", path.display())))?;
    Ok(path.to_path_buf())
}

fn write(path: &Path, text: &str) -> Result<(), RefreshError> {
    fs::write(path, text).map_err(io_err(format!("write {}", path.display())))?;
    Ok(())
}

/// Writes the per-pass input directories and the candidate's schema copies.
fn write_inputs(loader: &Loader, dir: &Path, platform: &RosterPlatform, cap: u32, previous: Option<&str>) -> Result<(), RefreshError> {
    let id = platform.platform_id;
    let outputs = mkdir(&dir.join("outputs"))?;
    let candidate_dir = mkdir(&dir.join("candidate"))?;
    copy_contract(loader, &candidate_dir, &["_schema.yaml", "_types.yaml"])?;
    let candidate = candidate_dir.join(format!("{id}.md"));
    let show = |path: &Path| path.display().to_string();

    let discovery = mkdir(&dir.join("inputs").join("discovery"))?;
    copy_contract(loader, &discovery, CONTRACT_FILES)?;
    write(&discovery.join("identification.md"), &identification(platform))?;
    write(
        &discovery.join("prompt.md"),
        &format!(
            "# Pass 1: independent discovery for {name}\n\n\
             Follow Pass 1 in `{fleet}`. Your inputs are this directory only: `{ident}` (the platform and its \
             interfaces), the instructions, and the schema (`{schema}`). You receive no previous research and no \
             curated sources; do not look for them.\n\n\
             Write:\n\n\
             - the discovery report to `{report}`;\n\
             - `{suggested}` as JSON: `{{\"suggestions\": [{{\"url\": \"…\", \"questions\": [\"…\"], \"contribution\": \"…\"}}]}}`. \
             Every suggestion names the research questions it answered and what it contributed that other sources did not.\n",
            name = platform.name,
            fleet = show(&discovery.join("_fleet.md")),
            ident = show(&discovery.join("identification.md")),
            schema = show(&discovery.join("_schema.yaml")),
            report = show(&outputs.join("discovery.md")),
            suggested = show(&outputs.join("suggested-sources.json")),
        ),
    )?;

    let reconcile = mkdir(&dir.join("inputs").join("reconcile"))?;
    copy_contract(loader, &reconcile, CONTRACT_FILES)?;
    write(&reconcile.join("curated-sources.md"), &curated(platform, cap))?;
    let previous_line = match previous {
        Some(text) => {
            write(&reconcile.join("previous.md"), text)?;
            format!("the previous document `{}`", show(&reconcile.join("previous.md")))
        }
        None => "no previous document (initial research)".to_string(),
    };
    write(
        &reconcile.join("prompt.md"),
        &format!(
            "# Pass 2: curated reconciliation for {name}\n\n\
             Follow Pass 2 in `{fleet}`. Inputs: the discovery report `{report}` and suggestions `{suggested}`, the \
             curated sources `{curated}`, and {previous_line}.\n\n\
             Write:\n\n\
             - the candidate document to `{candidate}`, keeping `$schema: ./_schema.yaml`, `created`, stable IDs, and every \
             chronology entry; validate it with `md schema validate {candidate}`;\n\
             - `{checks}` as JSON: `{{\"checks\": [{{\"url\": \"…\", \"checked_on\": \"YYYY-MM-DD\", \"outcome\": \"checked\", \"finding\": \"…\"}}]}}`. \
             Record one attempt per curated source; an inaccessible source has `\"outcome\": \"inaccessible\"` and a \
             `\"failure\"` instead of a finding. Never refresh the `retrieved` date of a source you did not successfully check.\n",
            name = platform.name,
            fleet = show(&reconcile.join("_fleet.md")),
            report = show(&outputs.join("discovery.md")),
            suggested = show(&outputs.join("suggested-sources.json")),
            curated = show(&reconcile.join("curated-sources.md")),
            candidate = show(&candidate),
            checks = show(&dir.join("source-checks.json")),
        ),
    )?;

    let sources = mkdir(&dir.join("inputs").join("sources"))?;
    copy_contract(loader, &sources, &["_fleet.md"])?;
    write(
        &sources.join("prompt.md"),
        &format!(
            "# Pass 3: source-list maintenance for {name}\n\n\
             Follow Pass 3 in `{fleet}`. Inputs: the suggestions `{suggested}`, the curated sources `{curated}`, and the \
             source checks `{checks}`. The cap is {cap} sources shared across the platform's interfaces.\n\n\
             Write `{proposal}` as JSON: `{{\"retain\": [{{\"url\": \"…\", \"interfaces\": [\"…\"], \"contribution\": \"…\"}}], \
             \"add\": [{{\"url\": \"…\", \"interfaces\": [\"…\"], \"contribution\": \"…\", \"replaces\": \"…\", \"coverage_gained\": \"…\", \
             \"coverage_lost\": \"…\"}}], \"remove\": [{{\"url\": \"…\", \"reason\": \"…\"}}]}}`. List every curated source under \
             `retain` or `remove`. `replaces` and the coverage fields are required only at capacity. Never edit the roster.\n",
            name = platform.name,
            fleet = show(&sources.join("_fleet.md")),
            suggested = show(&outputs.join("suggested-sources.json")),
            curated = show(&reconcile.join("curated-sources.md")),
            checks = show(&dir.join("source-checks.json")),
            proposal = show(&outputs.join("source-proposal.json")),
        ),
    )?;

    let review = mkdir(&dir.join("inputs").join("review"))?;
    let baseline_line = if previous.is_some() {
        format!("the previous document `{}`", show(&reconcile.join("previous.md")))
    } else {
        "no accepted baseline (initial research: review every fact)".to_string()
    };
    write(
        &review.join("prompt.md"),
        &format!(
            "# Independent evidence review for {name}\n\n\
             You did not write this research. Check each changed claim in the candidate `{candidate}` against its cited \
             evidence, using the mechanical comparison `{delta}` and {baseline_line}. Review changed evidence even at an \
             unchanged URL, and meaningful prose changes even when typed values match. A new API release alone never \
             validates a changed fact. Leave anything you cannot confirm `unresolved`; never invent certainty. Your \
             conclusions are evidence for a human reviewer, not approval.\n\n\
             Write `{output}` as JSON: `{{\"reviewer\": \"…\", \"conclusions\": [{{\"kind\": \"fact\", \"subject\": \"<id>\", \
             \"verdict\": \"supported\", \"evidence\": [\"<source id>\"], \"finding\": \"…\"}}]}}`. `kind` is `fact`, `source`, \
             `gap`, or `prose` (subject `body`); `verdict` is `supported`, `unresolved`, or `unsupported`. Cover every \
             changed fact, source, and gap in the comparison, and the prose when it changed. Store concise findings and \
             links only: no transcripts, thread copies, credentials, or message content.\n",
            name = platform.name,
            candidate = show(&candidate),
            delta = show(&dir.join("delta.json")),
            output = show(&outputs.join("evidence-review.json")),
        ),
    )?;
    Ok(())
}

fn identification(platform: &RosterPlatform) -> String {
    let mut out = format!("# {} ({})\n\n- Website: {}\n", platform.name, platform.platform_id, platform.website);
    if let Some(api) = &platform.api_url {
        let _ = writeln!(out, "- API: {api}");
    }
    out.push_str("\n## Interfaces\n\n");
    for interface in &platform.interfaces {
        let _ = writeln!(
            out,
            "- `{}` ({}): {}; identification {}",
            interface.interface_id, interface.role, interface.api_identity, interface.identification_url
        );
        for relationship in &interface.relationships {
            let _ = writeln!(out, "  - {} `{}`", relationship.kind, relationship.target);
        }
    }
    out
}

fn curated(platform: &RosterPlatform, cap: u32) -> String {
    let mut out = format!("# Curated sources for {} (cap {cap})\n\n", platform.name);
    for source in &platform.curated_sources {
        let _ = writeln!(out, "- {} ({}): {}", source.url, source.interfaces.join(", "), source.contribution);
    }
    out
}

/// A YAML single-quoted scalar: no escapes, so Windows paths stay literal.
fn yaml_single(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The sequence document for `stages`. Validation and the review check are
/// `shell:` steps so their time is charged to the run's ledger.
fn write_sequence(loader: &Loader, dir: &Path, record: &RunRecord, stages: &[Stage]) -> Result<(), RefreshError> {
    // Forward slashes: Windows accepts them, and a backslash inside the
    // quoted argument could otherwise be read as an escape by the tokenizer.
    let root = loader.workspace().repo_root().display().to_string().replace('\\', "/");
    let check = |through: Stage| {
        yaml_single(&format!("messenger research check-run {} --through {through} --root \"{root}\"", record.run_id))
    };
    let mut out = String::from("---\nsequence:\n");
    for stage in stages {
        let (name, step) = match stage {
            Stage::Discovery => ("discovery", "prompt: './inputs/discovery/prompt.md'".to_string()),
            Stage::Reconcile => ("reconcile", "prompt: './inputs/reconcile/prompt.md'".to_string()),
            Stage::Sources => ("sources", "prompt: './inputs/sources/prompt.md'".to_string()),
            Stage::Validation => ("validation", format!("shell: {}", check(Stage::Validation))),
            Stage::Review => ("review", "prompt: './inputs/review/prompt.md'".to_string()),
        };
        let _ = write!(out, "  - name: {name}\n    {step}\n");
        if *stage == Stage::Review {
            let _ = write!(out, "  - name: review-check\n    shell: {}\n", check(Stage::Review));
        }
    }
    out.push_str("---\n");
    write(&dir.join("run.md"), &out)
}
