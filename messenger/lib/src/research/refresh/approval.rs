//! The approval policy.
//!
//! Human approval is required for an initial baseline and for any change to
//! facts, gaps, applicability, supporting evidence, evidence-source
//! membership, schema, shared prompt, curated URLs, or prose. The only
//! preauthorized automatic acceptance is a verified unchanged renewal: every
//! substantive element is identical, and every cited source and every
//! curated source was successfully rechecked on the date the candidate
//! records. Observation dates, `last_updated`, and the researching agent may
//! differ; nothing else may. Validation is necessary, never sufficient.

use serde::Serialize;
use serde_json::Value;

use super::check::Evaluation;
use super::records::CheckOutcome;
use super::state::{ResearchedUnder, Stage};

/// What promotion may do with an evaluated run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum Decision {
    /// Not promotable at all: incomplete or invalid stages.
    Ineligible { reasons: Vec<String> },
    /// A verified unchanged renewal: automatic acceptance is preauthorized.
    Renewal,
    /// Promotable only with a named human approver.
    HumanRequired { reasons: Vec<String> },
}

/// Frontmatter keys a renewal may change: observation work and provenance.
const RENEWAL_MUTABLE: &[&str] = &["last_updated", "agent", "model"];

/// Frontmatter with renewal-mutable fields and source observation dates removed.
fn substantive(frontmatter: &Value) -> Value {
    let mut value = frontmatter.clone();
    if let Some(map) = value.as_object_mut() {
        for key in RENEWAL_MUTABLE {
            map.remove(*key);
        }
        if let Some(sources) = map.get_mut("sources").and_then(Value::as_array_mut) {
            for source in sources {
                if let Some(source) = source.as_object_mut() {
                    source.remove("retrieved");
                }
            }
        }
    }
    value
}

/// Decides how the run may be promoted. `previous` is what the accepted
/// document was last researched under (`None` when no record says).
pub fn decide(eval: &Evaluation, run_under: &ResearchedUnder, previous: Option<&ResearchedUnder>) -> Decision {
    let incomplete: Vec<String> = eval
        .stages
        .iter()
        .filter(|result| result.stage <= Stage::Review && result.status != super::state::StageStatus::Complete)
        .map(|result| format!("{} is {}: {}", result.stage, result.status, result.findings.join("; ")))
        .collect();
    if !incomplete.is_empty() {
        return Decision::Ineligible { reasons: incomplete };
    }
    let candidate = eval.candidate.as_ref().expect("a complete run has a candidate");
    let checks = eval.checks.as_ref().expect("a complete run has source checks");
    let proposal = eval.proposal.as_ref().expect("a complete run has a proposal");

    let mut reasons = Vec::new();
    let Some(baseline) = &eval.baseline else {
        return Decision::HumanRequired { reasons: vec!["initial baseline: there is no accepted document".to_string()] };
    };
    if substantive(baseline.validated.frontmatter()) != substantive(candidate.validated.frontmatter()) {
        let delta = eval.delta.as_ref().expect("a complete run has a delta");
        let facts = delta.facts.len();
        let sources = delta.sources.iter().filter(|s| s.field_changes.iter().any(|f| f.pointer != "/retrieved")).count();
        reasons.push(format!(
            "typed metadata changed ({facts} fact(s), {sources} evidence source(s), {} gap(s), or other records)",
            delta.gaps.len()
        ));
    }
    if baseline.body_hash != candidate.body_hash {
        reasons.push("explanatory prose changed".to_string());
    }
    if proposal.changes_list() {
        reasons.push("a curated-source change is proposed".to_string());
    }
    match previous {
        None => reasons.push("no review or renewal record says what the accepted document was researched under".to_string()),
        Some(previous) => {
            if previous.schema != run_under.schema {
                reasons.push("the schema changed since the accepted research".to_string());
            }
            if previous.fleet_prompt != run_under.fleet_prompt {
                reasons.push("the shared research prompt changed since the accepted research".to_string());
            }
        }
    }
    for check in &checks.checks {
        if check.outcome == CheckOutcome::Inaccessible {
            reasons.push(format!("curated source {} was inaccessible", check.url));
        }
    }
    for source in &candidate.validated.document().sources {
        let Some(url) = &source.url else { continue };
        let rechecked = source.retrieved.as_ref().is_some_and(|date| checks.checked(url).iter().any(|c| &c.checked_on == date));
        if !rechecked {
            reasons.push(format!("source {} was not successfully rechecked on the date it records", source.id));
        }
    }
    if reasons.is_empty() { Decision::Renewal } else { Decision::HumanRequired { reasons } }
}
