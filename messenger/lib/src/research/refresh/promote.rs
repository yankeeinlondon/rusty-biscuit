//! Promotion and rejection: the only operations that decide a run.
//!
//! Promotion re-evaluates the run, applies the approval policy, and publishes
//! the candidate through [`generate_with`] with every other platform carried
//! from the selected snapshot. A human-approved change adds its review record
//! to the same publication, so documents, catalog, summary, review records,
//! and CHANGELOG are selected together. A verified unchanged renewal adds no
//! review record and no CHANGELOG entry; its maintenance record stays local.
//! Promotion never edits the roster or the reviewed mappings.

use std::collections::BTreeMap;

use serde::Serialize;

use super::RefreshError;
use super::approval::{Decision as Policy, decide};
use super::check::{Evaluation, evaluate};
use super::records::CheckOutcome;
use super::review::{
    AcceptedHashes, Approval, RENEWAL_FORMAT, REVIEW_FORMAT, RenewalRecord, ReviewRecord, review_path, summarize,
};
use super::select::{published, researched_under};
use super::state::{Decision, RunRecord, RunStatus, StateArea, apply_ledger, write_json};
use crate::research::canonical::text_fingerprint;
use crate::research::generate::{Generated, generate_with};
use crate::research::load::Loader;
use crate::research::model::Date;
use crate::research::paths::document_path;
use crate::research::publish::Options;

/// How the operator asks to promote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// A named maintainer approves; a run that qualifies as a verified
    /// unchanged renewal is still recorded as a renewal.
    Human { by: String },
    /// Automatic acceptance; refused unless the run is a verified unchanged renewal.
    Renewal,
}

#[derive(Debug, Clone, Serialize)]
pub struct Promoted {
    pub run_id: String,
    pub platform_id: crate::research::model::PlatformId,
    /// `human` or `renewal`.
    pub kind: &'static str,
    /// The committed review record (human-approved changes only).
    pub review: Option<String>,
    /// The local renewal record (renewals only).
    pub renewal: Option<String>,
    pub generated: Option<Generated>,
}

/// One run's promotion plan: its evaluation and how it will be accepted.
struct Planned {
    record: RunRecord,
    eval: Evaluation,
    renewal: bool,
    accepted: AcceptedHashes,
    text: String,
}

/// Promotes awaiting-review runs together in one publication.
///
/// Each run is judged on its own; the candidates, and a review record for
/// each human-approved change, are then published as one snapshot with every
/// other platform carried from the selected snapshot. Several runs are needed
/// when no snapshot exists yet, because an initial publication requires every
/// active platform. Nothing is accepted unless the publication succeeds.
///
/// ## Errors
///
/// [`RefreshError::WrongStatus`] unless every run awaits review;
/// [`RefreshError::NotEligible`] for incomplete stages, a stale baseline, two
/// runs for one platform, or a renewal request that does not qualify;
/// publication errors (the previous snapshot stays selected and the runs keep
/// awaiting review).
pub fn promote(loader: &Loader, run_ids: &[String], request: &Request, today: &Date, options: Options) -> Result<Vec<Promoted>, RefreshError> {
    let workspace = loader.workspace();
    let state = StateArea::new(workspace);
    let verified = published(workspace)?;
    let under = researched_under(&state, verified.as_ref());
    let mut done = Vec::new();
    let mut planned: Vec<Planned> = Vec::new();
    for run_id in run_ids {
        let mut record = state.load(run_id)?;
        let eval = evaluate(loader, &record, today)?;
        if apply_ledger(&mut record, eval.ledger.as_ref()) {
            state.save(&record)?;
        }
        if record.status != RunStatus::AwaitingReview {
            return Err(RefreshError::WrongStatus {
                run_id: record.run_id.to_string(),
                status: record.status,
                action: "promotion",
                needs: "a run awaiting review (all stages checked by `check-run`)",
            });
        }
        if let Some(ledger) = &eval.ledger
            && matches!(ledger.state.as_str(), "active" | "exhausted")
        {
            return Err(RefreshError::Ledger {
                run_id: record.run_id.to_string(),
                state: ledger.state.clone(),
                guidance: "an active or exhausted run cannot be promoted",
            });
        }
        if planned.iter().any(|p| p.record.platform_id == record.platform_id) {
            return Err(RefreshError::NotEligible {
                run_id: record.run_id.to_string(),
                reasons: vec![format!("another run for {} is in this promotion", record.platform_id)],
            });
        }
        let text = eval.candidate_text.clone().unwrap_or_default();
        let published_text = verified.as_ref().and_then(|v| v.files.get(&document_path(record.platform_id)));
        // A publication that completed before the run was marked accepted.
        if published_text.map(Vec::as_slice) == Some(text.as_bytes())
            && record.baseline.as_ref().map(|b| &b.xxh64) != Some(&text_fingerprint(&text))
        {
            done.push(finish_existing(&state, &mut record, request, &eval, today, verified.as_ref())?);
            continue;
        }
        let previous = under.get(&record.platform_id).map(|(_, under)| under);
        let renewal = match (decide(&eval, &record.researched_under, previous), request) {
            (Policy::Ineligible { reasons }, _) => {
                return Err(RefreshError::NotEligible { run_id: record.run_id.to_string(), reasons });
            }
            (Policy::HumanRequired { mut reasons }, Request::Renewal) => {
                reasons.insert(0, "not a verified unchanged renewal; a named maintainer must approve".to_string());
                return Err(RefreshError::NotEligible { run_id: record.run_id.to_string(), reasons });
            }
            (Policy::Renewal, _) => true,
            (Policy::HumanRequired { .. }, Request::Human { .. }) => false,
        };
        let candidate = eval.candidate.as_ref().expect("an eligible run has a candidate");
        let accepted = AcceptedHashes {
            path: document_path(record.platform_id),
            frontmatter_hash: candidate.frontmatter_hash.clone(),
            body_hash: candidate.body_hash.clone(),
        };
        planned.push(Planned { record, eval, renewal, accepted, text });
    }
    if planned.is_empty() {
        return Ok(done);
    }

    let by = match request {
        Request::Human { by } => Some(by.clone()),
        Request::Renewal => None,
    };
    let updates: BTreeMap<_, _> = planned.iter().map(|p| (p.record.platform_id, p.text.clone())).collect();
    let mut reviews = BTreeMap::new();
    for plan in planned.iter().filter(|p| !p.renewal) {
        let by = by.as_deref().expect("only a human request reaches a human-approved change");
        let review = review_record(&plan.record, &plan.eval, plan.accepted.clone(), by, today);
        reviews.insert(review_path(today, plan.record.platform_id, &plan.record.run_id), review.to_bytes());
    }
    let generated = generate_with(loader, &updates, &reviews, today, options)?;
    for mut plan in planned {
        let (kind, review, renewal) = if plan.renewal {
            ("renewal", None, Some(write_renewal(&state, &plan.record, &plan.eval, plan.accepted.clone(), today)?))
        } else {
            ("human", Some(review_path(today, plan.record.platform_id, &plan.record.run_id)), None)
        };
        plan.record.status = RunStatus::Accepted;
        plan.record.decision = Some(Decision { kind: kind.to_string(), by: by.clone(), on: today.clone(), reason: None });
        state.save(&plan.record)?;
        done.push(Promoted {
            run_id: plan.record.run_id.to_string(),
            platform_id: plan.record.platform_id,
            kind,
            review,
            renewal,
            generated: Some(generated.clone()),
        });
    }
    Ok(done)
}

fn review_record(record: &RunRecord, eval: &Evaluation, accepted: AcceptedHashes, by: &str, today: &Date) -> ReviewRecord {
    let delta = eval.delta.as_ref().expect("an eligible run has a delta");
    let review = eval.review.clone().expect("an eligible run has an evidence review");
    let proposal = eval.proposal.clone().expect("an eligible run has a proposal");
    let mut checks = eval.checks.clone().expect("an eligible run has source checks").checks;
    checks.sort_by(|a, b| (&a.url, &a.checked_on).cmp(&(&b.url, &b.checked_on)));
    ReviewRecord {
        format: REVIEW_FORMAT.to_string(),
        run_id: record.run_id.clone(),
        platform_id: record.platform_id,
        approval: Approval { by: by.to_string(), on: today.clone() },
        researched_under: record.researched_under.clone(),
        accepted,
        summary: summarize(delta, &review, &proposal),
        delta: serde_json::to_value(delta).expect("a delta serializes"),
        evidence_review: review,
        source_checks: checks,
        curated_proposal: proposal,
    }
}

fn write_renewal(state: &StateArea, record: &RunRecord, eval: &Evaluation, accepted: AcceptedHashes, today: &Date) -> Result<String, RefreshError> {
    let mut rechecked: Vec<_> = eval
        .checks
        .as_ref()
        .map(|checks| checks.checks.iter().filter(|c| c.outcome == CheckOutcome::Checked).cloned().collect())
        .unwrap_or_default();
    rechecked.sort_by(|a: &super::records::SourceCheck, b| (&a.url, &a.checked_on).cmp(&(&b.url, &b.checked_on)));
    let renewal = RenewalRecord {
        format: RENEWAL_FORMAT.to_string(),
        run_id: record.run_id.clone(),
        platform_id: record.platform_id,
        renewed_on: today.clone(),
        researched_under: record.researched_under.clone(),
        accepted,
        rechecked,
    };
    let name = format!("{today}-{}-{}.json", record.platform_id, record.run_id);
    write_json(&state.renewals_dir().join(&name), &renewal, record.run_id.as_str())?;
    Ok(format!("{}/{name}", super::state::RENEWALS))
}

/// Marks a run whose candidate is already the published document (its
/// publication completed, or was rolled forward by `recover`) as accepted.
fn finish_existing(
    state: &StateArea,
    record: &mut RunRecord,
    request: &Request,
    eval: &Evaluation,
    today: &Date,
    verified: Option<&crate::research::publish::Verified>,
) -> Result<Promoted, RefreshError> {
    let review = verified.and_then(|v| {
        v.files.iter().find_map(|(path, bytes)| {
            ReviewRecord::parse(bytes).ok().filter(|r| r.run_id == record.run_id && super::review::is_review_path(path)).map(|r| (path.clone(), r))
        })
    });
    let by = match request {
        Request::Human { by } => Some(by.clone()),
        Request::Renewal => None,
    };
    let (kind, review_path, renewal_path, approver) = match review {
        Some((path, review)) => ("human", Some(path), None, Some(review.approval.by)),
        None => {
            let candidate = eval.candidate.as_ref().expect("a published candidate validates");
            let accepted = AcceptedHashes {
                path: document_path(record.platform_id),
                frontmatter_hash: candidate.frontmatter_hash.clone(),
                body_hash: candidate.body_hash.clone(),
            };
            ("renewal", None, Some(write_renewal(state, record, eval, accepted, today)?), by)
        }
    };
    record.status = RunStatus::Accepted;
    record.decision = Some(Decision {
        kind: kind.to_string(),
        by: approver,
        on: today.clone(),
        reason: Some("publication completed before the run was marked accepted".to_string()),
    });
    state.save(record)?;
    Ok(Promoted {
        run_id: record.run_id.to_string(),
        platform_id: record.platform_id,
        kind,
        review: review_path,
        renewal: renewal_path,
        generated: None,
    })
}

/// Rejects an undecided, inactive run; it stays local and never reaches the
/// CHANGELOG.
///
/// ## Errors
///
/// [`RefreshError::WrongStatus`] for an active or decided run.
pub fn reject(loader: &Loader, run_id: &str, by: &str, reason: &str, today: &Date) -> Result<RunRecord, RefreshError> {
    let state = StateArea::new(loader.workspace());
    let mut record = state.load(run_id)?;
    let ledger = state.ledger(&record)?;
    apply_ledger(&mut record, ledger.as_ref());
    if record.status.is_final() || record.status == RunStatus::Active {
        return Err(RefreshError::WrongStatus {
            run_id: record.run_id.to_string(),
            status: record.status,
            action: "rejection",
            needs: "an inactive, undecided run",
        });
    }
    record.status = RunStatus::Rejected;
    record.decision = Some(Decision { kind: "rejected".to_string(), by: Some(by.to_string()), on: today.clone(), reason: Some(reason.to_string()) });
    state.save(&record)?;
    Ok(record)
}
