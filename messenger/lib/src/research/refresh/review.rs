//! Accepted change history: durable review records, the generated research
//! CHANGELOG, and local unchanged-renewal records.
//!
//! A human-approved promotion commits one review record,
//! `messenger/docs/research/reviews/{date}-{platform}-{run_id}.json`, holding
//! the full mechanical delta, the structured evidence review, the source-check
//! results, and the curated-list proposal: links, fingerprints, and concise
//! findings only, never transcripts. `docs/research/CHANGELOG.md` is rendered
//! from those records alone, so it cannot list candidate, rejected, or
//! unchanged-renewal outcomes. Renewal records stay in the local state area.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::records::{EvidenceReview, SourceCheck, SourceProposal, Verdict};
use super::state::{ResearchedUnder, RunId};
use crate::research::model::{Date, PlatformId};

/// The review-record format tag.
pub const REVIEW_FORMAT: &str = "messenger-research-review/1";
/// The renewal-record format tag.
pub const RENEWAL_FORMAT: &str = "messenger-research-renewal/1";
/// Durable review records.
pub const REVIEWS_DIR: &str = "messenger/docs/research/reviews";
/// The generated accepted-change summary.
pub const CHANGELOG: &str = "messenger/docs/research/CHANGELOG.md";

/// The repository-relative path of a review record.
pub fn review_path(on: &Date, platform: PlatformId, run_id: &RunId) -> String {
    format!("{REVIEWS_DIR}/{on}-{platform}-{run_id}.json")
}

/// Whether `path` names a review record.
pub fn is_review_path(path: &str) -> bool {
    path.strip_prefix(REVIEWS_DIR)
        .and_then(|rest| rest.strip_prefix('/'))
        .is_some_and(|name| name.ends_with(".json") && !name.contains('/'))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub by: String,
    pub on: Date,
}

/// The accepted document's Darkmatter hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedHashes {
    pub path: String,
    pub frontmatter_hash: String,
    pub body_hash: String,
}

/// The concise, derived summary the CHANGELOG renders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeSummary {
    pub initial_baseline: bool,
    pub facts_added: Vec<String>,
    pub facts_changed: Vec<String>,
    pub facts_removed: Vec<String>,
    pub sources_changed: Vec<String>,
    pub gaps_changed: Vec<String>,
    pub prose_changed: bool,
    /// `kind: fact` for each suspicious-change flag.
    pub flags: Vec<String>,
    /// Subjects the evidence reviewer left unresolved or unsupported.
    pub unresolved: Vec<String>,
    pub supported: usize,
    pub curated_list_change_proposed: bool,
}

/// A committed review record for one human-approved promotion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRecord {
    pub format: String,
    pub run_id: RunId,
    pub platform_id: PlatformId,
    pub approval: Approval,
    pub researched_under: ResearchedUnder,
    pub accepted: AcceptedHashes,
    pub summary: ChangeSummary,
    /// The full mechanical delta (`research::delta::Delta`).
    pub delta: Value,
    pub evidence_review: EvidenceReview,
    pub source_checks: Vec<SourceCheck>,
    /// Applying a proposed curated-list change is a separate human edit of the
    /// roster; promotion never applies it.
    pub curated_proposal: SourceProposal,
}

impl ReviewRecord {
    /// Deterministic bytes: sorted keys, pretty, trailing LF.
    pub fn to_bytes(&self) -> Vec<u8> {
        let value = crate::research::project::sorted(&serde_json::to_value(self).expect("a review record serializes"));
        let mut bytes = serde_json::to_vec_pretty(&value).expect("json");
        bytes.push(b'\n');
        bytes
    }

    /// Parses and checks the format tag.
    ///
    /// ## Errors
    ///
    /// A message naming the problem.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let record: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if record.format != REVIEW_FORMAT {
            return Err(format!("unknown format {}", record.format));
        }
        Ok(record)
    }
}

/// Builds the summary from a delta and the evidence review.
pub fn summarize(delta: &crate::research::delta::Delta, review: &EvidenceReview, proposal: &SourceProposal) -> ChangeSummary {
    use crate::research::delta::{Baseline, ChangeKind};
    let ids = |kind: ChangeKind| -> Vec<String> {
        delta.facts.iter().filter(|fact| fact.change == kind).map(|fact| fact.id.clone()).collect()
    };
    let mut unresolved: Vec<String> = review
        .conclusions
        .iter()
        .filter(|c| c.verdict != Verdict::Supported)
        .map(|c| format!("{} {} ({})", c.kind, c.subject, c.verdict))
        .collect();
    unresolved.sort();
    ChangeSummary {
        initial_baseline: delta.baseline == Baseline::None,
        facts_added: ids(ChangeKind::Added),
        facts_changed: ids(ChangeKind::Changed),
        facts_removed: ids(ChangeKind::Removed),
        sources_changed: delta.sources.iter().map(|s| s.id.clone()).collect(),
        gaps_changed: delta.gaps.iter().map(|g| g.id.clone()).collect(),
        prose_changed: delta.prose.changed,
        flags: delta
            .flags
            .iter()
            .map(|flag| format!("{}: {}", serde_json::to_value(flag.kind).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default(), flag.fact))
            .collect(),
        unresolved,
        supported: review.conclusions.iter().filter(|c| c.verdict == Verdict::Supported).count(),
        curated_list_change_proposed: proposal.changes_list(),
    }
}

/// A local record of a verified unchanged renewal; never a CHANGELOG entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenewalRecord {
    pub format: String,
    pub run_id: RunId,
    pub platform_id: PlatformId,
    pub renewed_on: Date,
    pub researched_under: ResearchedUnder,
    pub accepted: AcceptedHashes,
    /// The sources successfully rechecked, with their dates.
    pub rechecked: Vec<SourceCheck>,
}

const LIST_LIMIT: usize = 12;

fn ids(items: &[String]) -> String {
    let mut shown: Vec<String> = items.iter().take(LIST_LIMIT).map(|id| format!("`{id}`")).collect();
    if items.len() > LIST_LIMIT {
        shown.push(format!("and {} more", items.len() - LIST_LIMIT));
    }
    shown.join(", ")
}

/// Renders the CHANGELOG from review records keyed by repository path,
/// newest approval first. Identical records give identical bytes.
pub fn render_changelog(reviews: &BTreeMap<String, ReviewRecord>) -> String {
    let mut ordered: Vec<(&String, &ReviewRecord)> = reviews.iter().collect();
    ordered.sort_by(|(a_path, a), (b_path, b)| b.approval.on.cmp(&a.approval.on).then_with(|| b_path.cmp(a_path)));
    let mut out = String::from(
        "<!-- Generated by `messenger research generate` from docs/research/reviews/; do not edit. -->\n\
         # Provider research changelog\n\n\
         Accepted research changes, newest first. Each entry links its review record, which holds the full \
         mechanical comparison and the structured evidence review. Candidate, rejected, and unchanged-renewal \
         outcomes never appear here.\n",
    );
    if ordered.is_empty() {
        out.push_str("\nNo accepted changes yet.\n");
    }
    for (path, record) in ordered {
        let s = &record.summary;
        let file = path.rsplit('/').next().unwrap_or(path);
        let _ = write!(out, "\n## {} — {} (run {})\n\n", record.approval.on, record.platform_id, record.run_id);
        let kind = if s.initial_baseline { "initial baseline" } else { "change to the accepted baseline" };
        let _ = writeln!(out, "- {kind}, approved by {}", record.approval.by);
        let mut lines: Vec<String> = Vec::new();
        for (label, list) in [("Facts added", &s.facts_added), ("Facts changed", &s.facts_changed), ("Facts removed", &s.facts_removed)] {
            if !list.is_empty() {
                lines.push(format!("{label} ({}): {}", list.len(), ids(list)));
            }
        }
        if !s.sources_changed.is_empty() {
            lines.push(format!("Evidence sources changed: {}", ids(&s.sources_changed)));
        }
        if !s.gaps_changed.is_empty() {
            lines.push(format!("Gaps changed: {}", ids(&s.gaps_changed)));
        }
        if s.prose_changed {
            lines.push("Explanatory prose changed".to_string());
        }
        if !s.flags.is_empty() {
            lines.push(format!("Suspicious-change flags: {}", ids(&s.flags)));
        }
        if s.unresolved.is_empty() {
            lines.push(format!("Evidence review: {} supported, none unresolved", s.supported));
        } else {
            lines.push(format!("Evidence review: {} supported; remaining uncertainty: {}", s.supported, ids(&s.unresolved)));
        }
        if s.curated_list_change_proposed {
            lines.push("A curated-source change was proposed for separate maintainer approval".to_string());
        }
        for line in lines {
            let _ = writeln!(out, "- {line}");
        }
        let _ = writeln!(out, "- Review record: [{file}](reviews/{file})");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_paths_are_recognized_only_directly_under_the_reviews_directory() {
        let run = RunId::parse("2026-09-17-0123abcd").unwrap();
        let path = review_path(&Date::parse("2026-09-20").unwrap(), PlatformId::Slack, &run);
        assert_eq!(path, "messenger/docs/research/reviews/2026-09-20-slack-2026-09-17-0123abcd.json");
        assert!(is_review_path(&path));
        assert!(!is_review_path("messenger/docs/research/reviews/nested/x.json"));
        assert!(!is_review_path("messenger/docs/research/reviews/x.md"));
        assert!(!is_review_path(CHANGELOG));
    }

    #[test]
    fn an_empty_history_renders_a_stable_changelog() {
        let text = render_changelog(&BTreeMap::new());
        assert!(text.starts_with("<!-- Generated"));
        assert!(text.ends_with("No accepted changes yet.\n"));
        assert_eq!(text, render_changelog(&BTreeMap::new()));
    }
}
