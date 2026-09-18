//! Judging a run from what it wrote, never from an agent's exit code.
//!
//! [`evaluate`] reads every stage's outputs, validates the candidate in
//! `Accepted` scope as if it sat at the accepted path, applies the refresh
//! integrity rules against the published baseline, and computes the delta.
//! [`check_run`] records the result in the run and writes `validation.json`
//! and `delta.json`; it runs inside the budgeted sequence as a `shell:` step.
//!
//! Integrity rules beyond validation: the candidate keeps the platform,
//! `created`, and every chronology entry; removed facts, gaps, and sources are
//! recorded in `changes`; `last_updated` moves only with a successful source
//! check; and a curated source's observation date changes only with a
//! successful check on that date.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::RefreshError;
use super::records::{EvidenceReview, SourceChecks, SourceProposal, SubjectKind, Suggestions};
use super::state::{LedgerView, RunRecord, RunStatus, Stage, StageResult, StageStatus, StateArea, apply_ledger, write_json};
use crate::research::canonical::text_fingerprint;
use crate::research::delta::{self, Delta};
use crate::research::diagnostics::Diagnostic;
use crate::research::load::Loader;
use crate::research::model::{Date, PlatformDocument, Roster};
use crate::research::paths::document_path;
use crate::research::project::AcceptedDocument;
use crate::research::validate::{Context, Scope, validate_document};

/// Everything a run's outputs establish.
#[derive(Debug, Clone)]
pub struct Evaluation {
    pub stages: Vec<StageResult>,
    /// Candidate validation findings (schema and semantic rules).
    pub diagnostics: Vec<Diagnostic>,
    pub candidate_text: Option<String>,
    pub candidate: Option<AcceptedDocument>,
    /// The published document the candidate replaces.
    pub baseline: Option<AcceptedDocument>,
    pub delta: Option<Delta>,
    pub suggestions: Option<Suggestions>,
    pub checks: Option<SourceChecks>,
    pub proposal: Option<SourceProposal>,
    pub review: Option<EvidenceReview>,
    pub ledger: Option<LedgerView>,
}

impl Evaluation {
    pub fn stage(&self, stage: Stage) -> &StageResult {
        self.stages.iter().find(|result| result.stage == stage).expect("every stage is evaluated")
    }

    /// Every stage through `through` is complete.
    pub fn complete_through(&self, through: Stage) -> bool {
        self.stages.iter().filter(|r| r.stage <= through).all(|r| r.status == StageStatus::Complete)
    }
}

enum Read<T> {
    Missing,
    Invalid(String),
    Parsed(T),
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Read<T> {
    match fs::read(path) {
        Err(_) => Read::Missing,
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(value) => Read::Parsed(value),
            Err(error) => Read::Invalid(error.to_string()),
        },
    }
}

fn result(stage: Stage, findings: Vec<String>, pending: bool) -> StageResult {
    let status = if pending {
        StageStatus::Pending
    } else if findings.is_empty() {
        StageStatus::Complete
    } else {
        StageStatus::Failed
    };
    StageResult { stage, status, findings }
}

/// Judges every stage of `record` against its outputs.
///
/// ## Errors
///
/// [`RefreshError`] when the roster, the published snapshot, or a present
/// candidate cannot be read at all. Contract problems are findings.
pub fn evaluate(loader: &Loader, record: &RunRecord, today: &Date) -> Result<Evaluation, RefreshError> {
    let workspace = loader.workspace();
    let state = StateArea::new(workspace);
    let dir = state.run_dir(record.platform_id, &record.run_id);
    let roster: Roster = loader.load_roster(&workspace.roster())?.record.ok_or(RefreshError::InvalidRoster)?;
    let platform = roster
        .active_platforms()
        .find(|p| p.platform_id == record.platform_id)
        .ok_or_else(|| RefreshError::NotInRoster { platform: record.platform_id.to_string() })?;
    let mut eval = Evaluation {
        stages: Vec::new(),
        diagnostics: Vec::new(),
        candidate_text: None,
        candidate: None,
        baseline: None,
        delta: None,
        suggestions: None,
        checks: None,
        proposal: None,
        review: None,
        ledger: state.ledger(record)?,
    };

    // Pass 1.
    let report = fs::read_to_string(dir.join("outputs").join("discovery.md")).unwrap_or_default();
    let (findings, pending) = match read_json::<Suggestions>(&dir.join("outputs").join("suggested-sources.json")) {
        _ if report.trim().is_empty() => (vec!["the discovery report is missing or empty".to_string()], true),
        Read::Missing => (vec!["suggested-sources.json is missing".to_string()], true),
        Read::Invalid(error) => (vec![format!("suggested-sources.json: {error}")], false),
        Read::Parsed(suggestions) => {
            let findings = suggestions.check();
            eval.suggestions = Some(suggestions);
            (findings, false)
        }
    };
    eval.stages.push(result(Stage::Discovery, findings, pending));

    // Pass 2.
    let candidate_path = dir.join("candidate").join(format!("{}.md", record.platform_id));
    eval.candidate_text = fs::read_to_string(&candidate_path).ok();
    let (findings, pending) = match (&eval.candidate_text, read_json::<SourceChecks>(&dir.join("source-checks.json"))) {
        (None, _) => (vec![format!("the candidate {}.md is missing", record.platform_id)], true),
        (_, Read::Missing) => (vec!["source-checks.json is missing".to_string()], true),
        (_, Read::Invalid(error)) => (vec![format!("source-checks.json: {error}")], false),
        (Some(_), Read::Parsed(checks)) => {
            let findings = checks.check(platform, today);
            eval.checks = Some(checks);
            (findings, false)
        }
    };
    eval.stages.push(result(Stage::Reconcile, findings, pending));

    // Pass 3.
    let (findings, pending) = match read_json::<SourceProposal>(&dir.join("outputs").join("source-proposal.json")) {
        Read::Missing => (vec!["source-proposal.json is missing".to_string()], true),
        Read::Invalid(error) => (vec![format!("source-proposal.json: {error}")], false),
        Read::Parsed(proposal) => {
            let findings = proposal.check(platform, roster.curated_source_cap);
            eval.proposal = Some(proposal);
            (findings, false)
        }
    };
    eval.stages.push(result(Stage::Sources, findings, pending));

    // Validation and delta.
    let verified = super::select::published(workspace)?;
    let published = verified
        .as_ref()
        .and_then(|v| v.files.get(&document_path(record.platform_id)))
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned());
    let context = Context { roster: Some(&roster), scope: Scope::Accepted };
    let mut baseline_record: Option<PlatformDocument> = None;
    if let Some(text) = &published {
        let loaded = loader.load_document_text(&workspace.document(record.platform_id), text.clone())?;
        baseline_record = loaded.record.clone();
        let lenient = Context { roster: Some(&roster), scope: Scope::Fragment };
        eval.baseline = validate_document(&loaded, &lenient).validated.and_then(|v| AcceptedDocument::new(v, text).ok());
    }
    let (findings, pending) = if !eval.complete_through(Stage::Reconcile) {
        (vec!["waits for a complete reconciliation pass".to_string()], true)
    } else {
        let text = eval.candidate_text.clone().expect("reconciliation is complete");
        match loader.load_document_text(&workspace.document(record.platform_id), text.clone()) {
            Err(error) => (vec![format!("the candidate cannot be read: {error}")], false),
            Ok(loaded) => candidate_findings(loader, &mut eval, record, &loaded, &text, baseline_record.as_ref(), platform, &context, published.as_deref(), today)?,
        }
    };
    eval.stages.push(result(Stage::Validation, findings, pending));

    // Independent evidence review.
    let (findings, pending) = match (&eval.delta, &eval.candidate) {
        (Some(delta), Some(candidate)) if eval.complete_through(Stage::Validation) => {
            match read_json::<EvidenceReview>(&dir.join("outputs").join("evidence-review.json")) {
                Read::Missing => (vec!["evidence-review.json is missing".to_string()], true),
                Read::Invalid(error) => (vec![format!("evidence-review.json: {error}")], false),
                Read::Parsed(review) => {
                    let findings = review_findings(&review, delta, candidate);
                    eval.review = Some(review);
                    (findings, false)
                }
            }
        }
        _ => (vec!["waits for a valid candidate and its delta".to_string()], true),
    };
    eval.stages.push(result(Stage::Review, findings, pending));
    Ok(eval)
}

#[allow(clippy::too_many_arguments)]
fn candidate_findings(
    loader: &Loader,
    eval: &mut Evaluation,
    record: &RunRecord,
    loaded: &crate::research::load::Loaded<PlatformDocument>,
    text: &str,
    baseline_record: Option<&PlatformDocument>,
    platform: &crate::research::model::RosterPlatform,
    context: &Context<'_>,
    published: Option<&str>,
    today: &Date,
) -> Result<(Vec<String>, bool), RefreshError> {
    let workspace = loader.workspace();
    let validation = validate_document(loaded, context);
    eval.diagnostics = validation.diagnostics.clone();
    let mut findings: Vec<String> = validation.diagnostics.iter().map(ToString::to_string).collect();
    if published.map(text_fingerprint) != record.baseline.as_ref().map(|b| b.xxh64.clone()) {
        findings.push("the accepted document changed since this run was prepared; start a new run".to_string());
    }
    if let Some(candidate) = &loaded.record {
        let checks = eval.checks.as_ref().expect("reconciliation is complete");
        findings.extend(integrity(record, candidate, baseline_record, checks, platform, today));
    }
    if let Some(validated) = validation.validated {
        match AcceptedDocument::new(validated, text) {
            // The integrity findings already name a platform mismatch; a
            // delta across platforms is meaningless.
            Ok(candidate) if candidate.validated.platform_id() != record.platform_id => {}
            Ok(candidate) => {
                let mappings =
                    if workspace.mappings().exists() { loader.load_mappings(&workspace.mappings())?.record } else { None };
                eval.delta = Some(delta::compare(eval.baseline.as_ref(), &candidate, mappings.as_ref()));
                eval.candidate = Some(candidate);
            }
            Err(message) => findings.push(message),
        }
    }
    Ok((findings, false))
}

fn integrity(
    record: &RunRecord,
    candidate: &PlatformDocument,
    baseline: Option<&PlatformDocument>,
    checks: &SourceChecks,
    platform: &crate::research::model::RosterPlatform,
    today: &Date,
) -> Vec<String> {
    let mut findings = Vec::new();
    if candidate.platform_id != record.platform_id {
        findings.push(format!("the candidate describes {}, not {}", candidate.platform_id, record.platform_id));
    }
    if &candidate.last_updated > today {
        findings.push(format!("last_updated {} is after {today}", candidate.last_updated));
    }
    let any_checked = checks.checks.iter().any(|c| c.outcome == super::records::CheckOutcome::Checked);
    if let Some(baseline) = baseline {
        if candidate.created != baseline.created {
            findings.push(format!("created changed from {} to {}; it is preserved", baseline.created, candidate.created));
        }
        if candidate.last_updated < baseline.last_updated {
            findings.push(format!("last_updated moved back from {} to {}", baseline.last_updated, candidate.last_updated));
        }
        if candidate.last_updated != baseline.last_updated && !any_checked {
            findings.push("last_updated changed but no source was successfully checked (a timestamp bump is not a refresh)".to_string());
        }
        let kept: BTreeSet<&str> = candidate.chronology.iter().map(|entry| entry.id.as_str()).collect();
        for entry in &baseline.chronology {
            if !kept.contains(entry.id.as_str()) {
                findings.push(format!("chronology entry {} was dropped; earlier entries are preserved", entry.id));
            }
        }
        let recorded: BTreeSet<&str> =
            candidate.changes.iter().flat_map(|c| c.facts.iter().chain(&c.evidence)).map(String::as_str).collect();
        let ids = |document: &PlatformDocument| -> BTreeSet<String> {
            let value = serde_json::to_value(document).unwrap_or(Value::Null);
            crate::research::validate::fact_ids(document)
                .into_iter()
                .map(|(_, id)| id.to_string())
                .chain(["gaps", "sources"].iter().flat_map(|key| {
                    value.get(*key).and_then(Value::as_array).into_iter().flatten().filter_map(|r| r.get("id")?.as_str().map(str::to_string))
                }))
                .collect()
        };
        let now = ids(candidate);
        for id in ids(baseline).difference(&now) {
            if !recorded.contains(id.as_str()) {
                findings.push(format!("{id} was removed without a changes entry"));
            }
        }
    } else if !any_checked {
        findings.push("initial research recorded no successful source check".to_string());
    }
    let curated: BTreeSet<&str> = platform.curated_sources.iter().map(|s| s.url.as_str()).collect();
    let before: BTreeMap<&str, Option<&Date>> =
        baseline.map(|b| b.sources.iter().map(|s| (s.id.as_str(), s.retrieved.as_ref())).collect()).unwrap_or_default();
    for source in &candidate.sources {
        let (Some(url), Some(retrieved)) = (&source.url, &source.retrieved) else { continue };
        if retrieved > today {
            findings.push(format!("source {} is dated {retrieved}, after {today}", source.id));
        }
        let refreshed = before.get(source.id.as_str()).is_none_or(|old| *old != Some(retrieved));
        if refreshed && curated.contains(url.as_str()) && !checks.checked(url).iter().any(|c| &c.checked_on == retrieved) {
            findings.push(format!(
                "source {} claims a check of {url} on {retrieved}, but no successful check on that date is recorded",
                source.id
            ));
        }
    }
    findings
}

fn review_findings(review: &EvidenceReview, delta: &Delta, candidate: &AcceptedDocument) -> Vec<String> {
    let source_ids: BTreeSet<String> = candidate.validated.document().sources.iter().map(|s| s.id.clone()).collect();
    let mut findings = review.check(&source_ids);
    let mut need: Vec<(SubjectKind, String)> = delta.facts.iter().map(|f| (SubjectKind::Fact, f.id.clone())).collect();
    need.extend(delta.sources.iter().map(|s| (SubjectKind::Source, s.id.clone())));
    need.extend(delta.gaps.iter().map(|g| (SubjectKind::Gap, g.id.clone())));
    if delta.prose.changed {
        need.push((SubjectKind::Prose, "body".to_string()));
    }
    for (kind, subject) in need {
        if !review.covers(kind, &subject) {
            findings.push(format!("the evidence review has no conclusion for changed {kind} {subject}"));
        }
    }
    findings
}

/// What `check-run` reports.
#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    pub run_id: String,
    pub platform_id: crate::research::model::PlatformId,
    pub through: Stage,
    pub passed: bool,
    pub status: RunStatus,
    pub stages: Vec<StageResult>,
}

/// Evaluates a run, records the stage results, and writes `validation.json`
/// and `delta.json` in its directory. A run passes when every stage through
/// `through` is complete; a failed or missing output fails the run, and a
/// passing review stage leaves it awaiting human review.
///
/// ## Errors
///
/// [`RefreshError::WrongStatus`] for a decided run, or a read/write error.
pub fn check_run(loader: &Loader, run_id: &str, through: Stage, today: &Date) -> Result<CheckReport, RefreshError> {
    let state = StateArea::new(loader.workspace());
    let mut record = state.load(run_id)?;
    if record.status.is_final() {
        return Err(RefreshError::WrongStatus {
            run_id: record.run_id.to_string(),
            status: record.status,
            action: "checking",
            needs: "an undecided run",
        });
    }
    let eval = evaluate(loader, &record, today)?;
    apply_ledger(&mut record, eval.ledger.as_ref());
    let dir = state.run_dir(record.platform_id, &record.run_id);
    write_json(&dir.join("validation.json"), &serde_json::json!({ "diagnostics": eval.diagnostics, "stages": eval.stages }), run_id)?;
    if let Some(delta) = &eval.delta {
        write_json(&dir.join("delta.json"), delta, run_id)?;
    }
    record.stages = eval.stages.clone();
    let passed = eval.complete_through(through);
    if matches!(record.status, RunStatus::Active | RunStatus::Failed | RunStatus::AwaitingReview) {
        if passed {
            record.status = if through == Stage::Review { RunStatus::AwaitingReview } else { RunStatus::Active };
            record.stop_reason = None;
        } else {
            let failed = eval.stages.iter().find(|r| r.stage <= through && r.status != StageStatus::Complete).expect("a stage is incomplete");
            record.status = RunStatus::Failed;
            record.stop_reason = Some(format!("{}: {}", failed.stage, failed.findings.first().cloned().unwrap_or_default()));
        }
    }
    state.save(&record)?;
    Ok(CheckReport {
        run_id: record.run_id.to_string(),
        platform_id: record.platform_id,
        through,
        passed: passed && !matches!(record.status, RunStatus::Exhausted | RunStatus::Interrupted),
        status: record.status,
        stages: record.stages,
    })
}
