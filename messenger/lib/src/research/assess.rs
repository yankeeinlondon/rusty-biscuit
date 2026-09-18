//! Reviewed implementation assessments (SR-MAPPING) and fingerprint reuse.
//!
//! An accepted assessment is an implementation claim only while every
//! recorded fingerprint still matches its input. Any difference makes it
//! `unassessed` and raises a `requires_messenger_update` gap. Nothing here
//! reads or changes `CapabilitySet` or delivery behavior; the checkout
//! revision is provenance, never a reuse key.

use std::collections::BTreeSet;

use serde::Serialize;

use super::canonical::{record_fingerprint, text_fingerprint};
use super::diagnostics::{Findings, Rule};
use super::model::{
    AdapterId, Assessment, Category, FingerprintKind, ImplementationStatus, Mappings, ReviewStatus,
    StaleReason,
};
use super::paths::{RepoPath, Workspace};
use super::validate::ValidatedDocument;

pub(crate) fn check_mappings(mappings: &Mappings, documents: &[&ValidatedDocument], findings: &mut Findings) {
    let mut ids = BTreeSet::new();
    for (a, assessment) in mappings.assessments.iter().enumerate() {
        let base = format!("/assessments/{a}");
        let id = assessment.id.as_str();
        let mut push = |pointer: String, message: String| findings.push(Rule::Mapping, pointer, Some(id), message);
        if !ids.insert(id) {
            push(format!("{base}/id"), "duplicate assessment ID".to_string());
        }
        if assessment.adapter.platform() != assessment.platform_id {
            push(
                format!("{base}/adapter"),
                format!("adapter {} belongs to {}, not {}", assessment.adapter, assessment.adapter.platform(), assessment.platform_id),
            );
        }
        match documents.iter().find(|d| d.platform_id() == assessment.platform_id) {
            Some(document) => {
                for fact in &assessment.facts {
                    if document.record(fact).is_none() {
                        push(format!("{base}/facts"), format!("fact {}#{fact} does not exist", assessment.platform_id));
                    }
                }
            }
            None => push(
                format!("{base}/platform_id"),
                format!("no validated {} document to resolve the assessed facts", assessment.platform_id),
            ),
        }
        let accepted = assessment.review.status == ReviewStatus::Accepted;
        if !accepted && assessment.status != ImplementationStatus::Unassessed {
            push(
                format!("{base}/status"),
                format!("a {} assessment is not an implementation claim; its status is unassessed", assessment.review.status),
            );
        }
        if accepted && (assessment.review.reviewed_by.is_none() || assessment.review.reviewed_on.is_none()) {
            push(format!("{base}/review"), "an accepted assessment records reviewed_by and reviewed_on".to_string());
        }
        match (assessment.status, assessment.stale_reason) {
            (ImplementationStatus::Unassessed, None) => {
                push(format!("{base}/stale_reason"), "an unassessed mapping states its stale_reason".to_string())
            }
            (status, Some(_)) if status != ImplementationStatus::Unassessed => {
                push(format!("{base}/stale_reason"), "only an unassessed mapping has a stale_reason".to_string())
            }
            _ => {}
        }
        if assessment.requires_messenger_update && assessment.follow_up.is_none() {
            push(format!("{base}/follow_up"), "requires_messenger_update: true proposes a follow_up".to_string());
        }
        for (field, refs, kind) in [
            ("code_refs", &assessment.code_refs, FingerprintKind::CodeFile),
            ("test_refs", &assessment.test_refs, FingerprintKind::TestFile),
        ] {
            for reference in refs {
                if !portable(reference) {
                    push(format!("{base}/{field}"), format!("{reference} is not a portable repository-relative path"));
                }
                if !assessment.fingerprints.iter().any(|f| f.kind == kind && f.input == *reference) {
                    push(format!("{base}/fingerprints"), format!("{reference} has no {kind} fingerprint"));
                }
            }
        }
        for (f, fingerprint) in assessment.fingerprints.iter().enumerate() {
            let pointer = format!("{base}/fingerprints/{f}/input");
            match fingerprint.kind {
                FingerprintKind::ResearchFact => match fingerprint.input.split_once('#') {
                    Some((platform, fact)) if platform == assessment.platform_id.as_str() && assessment.facts.iter().any(|f| f == fact) => {}
                    _ => push(pointer, format!("research input {} names no assessed fact of {}", fingerprint.input, assessment.platform_id)),
                },
                _ if !portable(&fingerprint.input) => {
                    push(pointer, format!("{} is not a portable repository-relative path", fingerprint.input))
                }
                _ => {}
            }
        }
    }
}

/// Forward-slash, relative, without `.`/`..` segments.
fn portable(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path.split('/').all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

/// One input whose recorded fingerprint no longer matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedInput {
    pub input: String,
    pub kind: FingerprintKind,
    pub recorded: String,
    /// `None` when the input is gone (missing file or removed fact).
    pub current: Option<String>,
}

/// The reusable state of one assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AssessmentState {
    /// Accepted and every fingerprint matches: an implementation claim.
    Current { status: ImplementationStatus },
    /// Not an implementation claim until reviewed again.
    Unassessed { reason: StaleReason, changed: Vec<ChangedInput> },
}

/// A structured `requires_messenger_update` gap raised by an assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImplementationGap {
    pub assessment: String,
    pub adapter: AdapterId,
    pub category: Category,
    pub facts: Vec<String>,
    pub reason: String,
    pub follow_up: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssessmentOutcome {
    pub id: String,
    pub adapter: AdapterId,
    pub category: Category,
    /// Provenance only.
    pub assessed_revision: String,
    pub state: AssessmentState,
    pub gap: Option<ImplementationGap>,
}

/// Decides reuse for every assessment by recomputing its fingerprints from
/// the workspace files and the validated documents.
pub fn evaluate(mappings: &Mappings, documents: &[&ValidatedDocument], workspace: &Workspace) -> Vec<AssessmentOutcome> {
    mappings
        .assessments
        .iter()
        .map(|assessment| {
            let state = assessment_state(assessment, documents, workspace);
            let gap = match &state {
                AssessmentState::Unassessed { reason, .. } => Some(gap(assessment, format!("assessment is unassessed ({reason})"))),
                AssessmentState::Current { .. } if assessment.requires_messenger_update => {
                    Some(gap(assessment, "accepted assessment requires a Messenger update".to_string()))
                }
                AssessmentState::Current { .. } => None,
            };
            AssessmentOutcome {
                id: assessment.id.clone(),
                adapter: assessment.adapter,
                category: assessment.category,
                assessed_revision: assessment.assessed_revision.clone(),
                state,
                gap,
            }
        })
        .collect()
}

fn gap(assessment: &Assessment, reason: String) -> ImplementationGap {
    ImplementationGap {
        assessment: assessment.id.clone(),
        adapter: assessment.adapter,
        category: assessment.category,
        facts: assessment.facts.clone(),
        reason,
        follow_up: assessment.follow_up.clone(),
    }
}

fn assessment_state(assessment: &Assessment, documents: &[&ValidatedDocument], workspace: &Workspace) -> AssessmentState {
    if assessment.review.status != ReviewStatus::Accepted || assessment.status == ImplementationStatus::Unassessed {
        return AssessmentState::Unassessed {
            reason: assessment.stale_reason.unwrap_or(StaleReason::NeverReviewed),
            changed: Vec::new(),
        };
    }
    let document = documents.iter().find(|d| d.platform_id() == assessment.platform_id);
    let mut changed = Vec::new();
    let mut fact_removed = assessment
        .facts
        .iter()
        .any(|fact| document.and_then(|d| d.record(fact)).is_none());
    for fingerprint in &assessment.fingerprints {
        let current = match fingerprint.kind {
            FingerprintKind::ResearchFact => fingerprint
                .input
                .split_once('#')
                .and_then(|(_, fact)| document.and_then(|d| d.record(fact)))
                .map(record_fingerprint),
            FingerprintKind::CodeFile | FingerprintKind::TestFile => {
                std::fs::read_to_string(workspace.resolve(&RepoPath::from_portable(fingerprint.input.clone())))
                    .ok()
                    .map(|text| text_fingerprint(&text))
            }
        };
        if current.as_deref() != Some(fingerprint.digest.as_str()) {
            if fingerprint.kind == FingerprintKind::ResearchFact && current.is_none() {
                fact_removed = true;
            }
            changed.push(ChangedInput {
                input: fingerprint.input.clone(),
                kind: fingerprint.kind,
                recorded: fingerprint.digest.clone(),
                current,
            });
        }
    }
    match (fact_removed, changed.is_empty()) {
        (true, _) => AssessmentState::Unassessed { reason: StaleReason::FactRemoved, changed },
        (false, false) => AssessmentState::Unassessed { reason: StaleReason::FingerprintChanged, changed },
        (false, true) => AssessmentState::Current { status: assessment.status },
    }
}
