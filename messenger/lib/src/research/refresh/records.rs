//! The structured outputs agents write during a run, and their checks.
//!
//! Each pass writes JSON beside its prose so completion is judged from
//! content, not an exit code: discovery's `suggested-sources.json`, the
//! reconciliation pass's `source-checks.json`, source-list maintenance's
//! `source-proposal.json`, and the evidence reviewer's `evidence-review.json`.
//! Every free-text field is bounded and scanned by [`unsafe_content`], because
//! review records built from them are committed.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::research::model::{Date, RosterPlatform};

/// The longest free-text field an agent may store.
pub const MAX_TEXT: usize = 1000;

/// Pass 1: sources discovery found valuable. Leads, never evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suggestions {
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suggestion {
    pub url: String,
    /// Research-question numbers or names the source answered.
    pub questions: Vec<String>,
    /// What it contributed that other sources did not.
    pub contribution: String,
}

crate::research::model::common::string_enum! {
    pub enum CheckOutcome {
        Checked => "checked",
        Inaccessible => "inaccessible",
    }
}

/// Pass 2: one attempt record per curated source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceChecks {
    pub checks: Vec<SourceCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCheck {
    pub url: String,
    pub checked_on: Date,
    pub outcome: CheckOutcome,
    /// What a successful review established.
    #[serde(default)]
    pub finding: Option<String>,
    /// Why an attempt failed (status, redirect, access policy).
    #[serde(default)]
    pub failure: Option<String>,
}

impl SourceChecks {
    /// Successful checks of `url`, latest date first.
    pub fn checked(&self, url: &str) -> Vec<&SourceCheck> {
        let mut checks: Vec<&SourceCheck> =
            self.checks.iter().filter(|c| c.url == url && c.outcome == CheckOutcome::Checked).collect();
        checks.sort_by(|a, b| b.checked_on.cmp(&a.checked_on));
        checks
    }
}

/// Pass 3: a proposal for the curated list. It never edits the roster.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProposal {
    pub retain: Vec<ProposedSource>,
    pub add: Vec<ProposedSource>,
    pub remove: Vec<RemovedSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedSource {
    pub url: String,
    pub interfaces: Vec<String>,
    pub contribution: String,
    /// At capacity: the curated URL this addition replaces.
    #[serde(default)]
    pub replaces: Option<String>,
    #[serde(default)]
    pub coverage_gained: Option<String>,
    #[serde(default)]
    pub coverage_lost: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemovedSource {
    pub url: String,
    pub reason: String,
}

impl SourceProposal {
    /// Whether the proposal changes the curated list at all.
    pub fn changes_list(&self) -> bool {
        !self.add.is_empty() || !self.remove.is_empty()
    }
}

crate::research::model::common::string_enum! {
    pub enum SubjectKind {
        Fact => "fact",
        Source => "source",
        Gap => "gap",
        Prose => "prose",
    }
}

crate::research::model::common::string_enum! {
    pub enum Verdict {
        Supported => "supported",
        Unresolved => "unresolved",
        Unsupported => "unsupported",
    }
}

/// The independent evidence reviewer's conclusions. Evidence for human
/// review, never approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReview {
    pub reviewer: String,
    pub conclusions: Vec<Conclusion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conclusion {
    pub kind: SubjectKind,
    /// The fact, source, or gap ID; `body` for prose.
    pub subject: String,
    pub verdict: Verdict,
    /// Candidate source IDs the reviewer checked.
    pub evidence: Vec<String>,
    pub finding: String,
}

/// Checks a text field is present, bounded, and safe to commit.
pub(crate) fn check_text(findings: &mut Vec<String>, what: &str, text: &str) {
    if text.trim().is_empty() {
        findings.push(format!("{what} is empty"));
    } else if text.chars().count() > MAX_TEXT {
        findings.push(format!("{what} exceeds {MAX_TEXT} characters"));
    } else if let Some(reason) = unsafe_content(text) {
        findings.push(format!("{what} is unsafe to store: {reason}"));
    }
}

impl Suggestions {
    pub fn check(&self) -> Vec<String> {
        let mut findings = Vec::new();
        for (i, suggestion) in self.suggestions.iter().enumerate() {
            check_text(&mut findings, &format!("suggestion {i} url"), &suggestion.url);
            check_text(&mut findings, &format!("suggestion {i} contribution"), &suggestion.contribution);
            if suggestion.questions.is_empty() {
                findings.push(format!("suggestion {i} ({}) names no research question", suggestion.url));
            }
        }
        findings
    }
}

impl SourceChecks {
    /// Every curated source has an attempt, dated no later than `today`,
    /// with a finding when checked and a failure when inaccessible.
    pub fn check(&self, platform: &RosterPlatform, today: &Date) -> Vec<String> {
        let mut findings = Vec::new();
        for curated in &platform.curated_sources {
            if !self.checks.iter().any(|check| check.url == curated.url) {
                findings.push(format!("no attempt recorded for curated source {}", curated.url));
            }
        }
        for (i, check) in self.checks.iter().enumerate() {
            check_text(&mut findings, &format!("check {i} url"), &check.url);
            if &check.checked_on > today {
                findings.push(format!("check {i} ({}) is dated after {today}", check.url));
            }
            match (check.outcome, &check.finding, &check.failure) {
                (CheckOutcome::Checked, Some(finding), None) => check_text(&mut findings, &format!("check {i} finding"), finding),
                (CheckOutcome::Inaccessible, None, Some(failure)) => {
                    check_text(&mut findings, &format!("check {i} failure"), failure);
                }
                (CheckOutcome::Checked, _, _) => findings.push(format!("check {i} ({}) is checked: record a finding and no failure", check.url)),
                (CheckOutcome::Inaccessible, _, _) => {
                    findings.push(format!("check {i} ({}) is inaccessible: record the failure and no finding", check.url));
                }
            }
        }
        findings
    }
}

impl SourceProposal {
    /// Every curated URL is retained or removed; additions are new; every
    /// retained or added URL explains its contribution and serves roster
    /// interfaces; the result fits the cap; and at capacity each addition
    /// names the source it replaces and the coverage gained and lost.
    pub fn check(&self, platform: &RosterPlatform, cap: u32) -> Vec<String> {
        let mut findings = Vec::new();
        let curated: BTreeSet<&str> = platform.curated_sources.iter().map(|s| s.url.as_str()).collect();
        let retained: BTreeSet<&str> = self.retain.iter().map(|s| s.url.as_str()).collect();
        let removed: BTreeSet<&str> = self.remove.iter().map(|s| s.url.as_str()).collect();
        for url in &curated {
            match (retained.contains(url), removed.contains(url)) {
                (false, false) => findings.push(format!("curated source {url} is neither retained nor removed")),
                (true, true) => findings.push(format!("curated source {url} is both retained and removed")),
                _ => {}
            }
        }
        for url in retained.iter().chain(&removed) {
            if !curated.contains(url) {
                findings.push(format!("{url} is not a curated source; propose it under add"));
            }
        }
        for (list, entries) in [("retain", &self.retain), ("add", &self.add)] {
            for entry in entries {
                check_text(&mut findings, &format!("{list} {} contribution", entry.url), &entry.contribution);
                if entry.interfaces.is_empty() {
                    findings.push(format!("{list} {} serves no interface", entry.url));
                }
                for interface in &entry.interfaces {
                    if platform.interface(interface).is_none() {
                        findings.push(format!("{list} {} names unknown interface {interface}", entry.url));
                    }
                }
            }
        }
        for entry in &self.remove {
            check_text(&mut findings, &format!("remove {} reason", entry.url), &entry.reason);
        }
        let at_capacity = curated.len() >= cap as usize;
        for entry in &self.add {
            if curated.contains(entry.url.as_str()) {
                findings.push(format!("add {} is already curated", entry.url));
            }
            if at_capacity {
                match &entry.replaces {
                    Some(replaced) if removed.contains(replaced.as_str()) => {}
                    Some(replaced) => findings.push(format!("add {} replaces {replaced}, which is not removed", entry.url)),
                    None => findings.push(format!("the list is at its cap of {cap}: add {} must name the source it replaces", entry.url)),
                }
                for (field, value) in [("coverage_gained", &entry.coverage_gained), ("coverage_lost", &entry.coverage_lost)] {
                    match value {
                        Some(text) => check_text(&mut findings, &format!("add {} {field}", entry.url), text),
                        None => findings.push(format!("the list is at its cap of {cap}: add {} must explain {field}", entry.url)),
                    }
                }
            }
        }
        let size = self.retain.len() + self.add.len();
        if size > cap as usize {
            findings.push(format!("the proposed list has {size} sources; the cap is {cap}"));
        }
        findings
    }
}

impl EvidenceReview {
    /// Bounded, safe text; evidence IDs must exist in the candidate.
    pub fn check(&self, source_ids: &BTreeSet<String>) -> Vec<String> {
        let mut findings = Vec::new();
        check_text(&mut findings, "reviewer", &self.reviewer);
        for conclusion in &self.conclusions {
            let what = format!("{} {}", conclusion.kind, conclusion.subject);
            check_text(&mut findings, &format!("{what} finding"), &conclusion.finding);
            for id in &conclusion.evidence {
                if !source_ids.contains(id) {
                    findings.push(format!("{what} cites unknown source {id}"));
                }
            }
        }
        findings
    }

    /// Whether a conclusion covers `subject` of `kind`.
    pub fn covers(&self, kind: SubjectKind, subject: &str) -> bool {
        self.conclusions.iter().any(|c| c.kind == kind && c.subject == subject)
    }
}

const SECRET_MARKERS: &[&str] = &[
    "xoxb-",
    "xoxp-",
    "xapp-",
    "xoxe.",
    "Bearer ",
    "-----BEGIN",
    "hooks.slack.com/services/T",
    "api.telegram.org/bot1",
    "api.telegram.org/bot2",
    "api.telegram.org/bot5",
    "api.telegram.org/bot6",
    "api.telegram.org/bot7",
];

const ESCAPE_SPELLINGS: &[&str] = &["\\e[", "\\x1b", "\\x1B", "\\u001b", "\\u001B", "\\033", "\\u009b", "\\u009B"];

/// Why `text` is unsafe to store (control characters, credential shapes,
/// concrete webhook IDs, bot tokens, phone numbers), or `None`. The same
/// rules as the fixture corpus scanner in `tests/research_corpus.rs`.
pub fn unsafe_content(text: &str) -> Option<String> {
    if let Some(c) = text.chars().find(|c| c.is_control() && !matches!(c, '\n' | '\t' | '\r')) {
        return Some(format!("control character U+{:04X}", c as u32));
    }
    if let Some(esc) = ESCAPE_SPELLINGS.iter().find(|e| text.contains(**e)) {
        return Some(format!("escaped control sequence {esc}"));
    }
    if let Some(marker) = SECRET_MARKERS.iter().find(|m| text.contains(**m)) {
        return Some(format!("credential marker {marker:?}"));
    }
    if let Some(at) = text.find("/webhooks/")
        && text[at + "/webhooks/".len()..].starts_with(|c: char| c.is_ascii_digit())
    {
        return Some("webhook URL with a concrete ID".to_string());
    }
    let bytes = text.as_bytes();
    for (i, _) in text.match_indices(':') {
        let digits = bytes[..i].iter().rev().take_while(|b| b.is_ascii_digit()).count();
        let tail = bytes[i + 1..]
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric() || **b == b'_' || **b == b'-')
            .count();
        if (8..=10).contains(&digits) && tail >= 35 {
            return Some("bot-token shape".to_string());
        }
    }
    for (i, _) in text.match_indices('+') {
        if bytes[i + 1..].iter().take_while(|b| b.is_ascii_digit()).count() >= 10 {
            return Some("phone-number shape".to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_text_is_bounded_and_scanned() {
        let mut findings = Vec::new();
        check_text(&mut findings, "a", "fine");
        check_text(&mut findings, "b", "  ");
        check_text(&mut findings, "c", &"x".repeat(MAX_TEXT + 1));
        check_text(&mut findings, "d", "red \u{1b}[31m");
        check_text(&mut findings, "e", "Authorization: Bearer abc");
        assert_eq!(findings.len(), 4, "{findings:?}");
        assert!(findings[0].starts_with("b is empty"));
        assert!(findings[1].contains("exceeds"));
        assert!(findings[2].contains("control character"));
        assert!(findings[3].contains("credential"));
    }
}
