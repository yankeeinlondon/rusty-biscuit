//! Deterministic validate-and-resume gate for the `agent-errors` research
//! topic (spec `2026-07-11-provider-errors-as-data`, D10).
//!
//! This is the mechanical half of the fleet lifecycle: it reads one provider's
//! `agent-errors/<slug>.md` research frontmatter and its Phase-A facts seed,
//! runs the deterministic checks the fleet cannot self-attest, and writes a
//! JSON findings file. The fleet `success` stack runs this check with
//! `no_error: true`, then — when the file exists — `resume`s the same research
//! session with the findings so the model corrects its own output.
//!
//! ## Findings-file contract
//!
//! The check **removes any stale findings first**, then writes the file **only
//! when at least one finding survives**. So an absent file means "clean" and a
//! present file means "needs a resume" — the two states the `when:` guard and
//! the resume message branch on. The write is atomic (temp file + rename) so a
//! concurrent reader never observes a half-written report.
//!
//! ## Why here and not in the mapping registry
//!
//! Like the vocabulary emitter itself ([`crate::vocabulary`]), this gate is
//! deliberately outside the general `ProviderInfo` mapping registry: the
//! research shape (per-needle provenance objects) is not a catalog field, and
//! the checks are research-hygiene rules, not generation rules. Provenance
//! lives only in the research layer and is dropped when Phase C graduates the
//! vocabulary into the runtime tables.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::GenError;
use crate::inputs;
use crate::vocabulary::{ErrorVocabulary, FACTS_KEY};

/// The research topic directory under `docs/research/`.
const TOPIC: &str = "agent-errors";

/// Capacity/overload vocabulary the fleet must either propose as needles or
/// acknowledge as a `gaps` entry (spec D10 "motivating-class coverage"). These
/// are lowercase substrings matched case-insensitively against needle text and
/// gap prose — the same match discipline the runtime classifier uses.
const CAPACITY_TERMS: &[&str] = &[
    "overload",
    "overloaded",
    "capacity",
    "at capacity",
    "resource_exhausted",
    "resource exhausted",
    "429",
    "503",
];

/// One provider's research vocabulary, as the `agent-errors/<slug>.md`
/// frontmatter carries it. Unknown keys (`$schema`, `last_updated`, `agent`,
/// `docs`, `changes`, …) are ignored — this reads only the vocabulary shape.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResearchVocabulary {
    #[serde(default)]
    pub kind_buckets: Vec<ResearchBucket>,
    #[serde(default)]
    pub msg_buckets: Vec<ResearchBucket>,
    #[serde(default)]
    pub code_buckets: Vec<ResearchCodeBucket>,
    #[serde(default)]
    pub gaps: Vec<Gap>,
}

/// One ordered research bucket: a semantic kind plus its provenance-bearing
/// needle objects.
#[derive(Debug, Clone, Deserialize)]
pub struct ResearchBucket {
    pub kind: String,
    #[serde(default)]
    pub needles: Vec<ResearchNeedle>,
}

/// One research needle with its per-needle provenance.
#[derive(Debug, Clone, Deserialize)]
pub struct ResearchNeedle {
    pub text: String,
    pub evidence: String,
    #[serde(default)]
    pub source: Option<String>,
}

/// One ordered research code bucket (Kimi JSON-RPC codes).
#[derive(Debug, Clone, Deserialize)]
pub struct ResearchCodeBucket {
    pub kind: String,
    #[serde(default)]
    pub codes: Vec<ResearchCode>,
}

/// One research numeric wire code with its provenance.
#[derive(Debug, Clone, Deserialize)]
pub struct ResearchCode {
    pub code: i64,
    #[serde(default)]
    pub name: Option<String>,
    pub evidence: String,
    #[serde(default)]
    pub source: Option<String>,
}

/// An unresearchable-area acknowledgement.
#[derive(Debug, Clone, Deserialize)]
pub struct Gap {
    pub area: String,
    pub notes: String,
}

/// Which structured branch a needle lives in — carried on findings so a
/// resume message can point the model at the exact list to fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Branch {
    Kind,
    Msg,
    Code,
}

impl Branch {
    fn wire(self) -> &'static str {
        match self {
            Branch::Kind => "kind_buckets",
            Branch::Msg => "msg_buckets",
            Branch::Code => "code_buckets",
        }
    }
}

/// The deterministic check a finding came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// A seeded needle/code is missing from the research frontmatter (the
    /// mechanical half of R1 — seeds are sticky).
    SeedPreservation,
    /// A research needle is not lowercase or carries leading/trailing/interior
    /// whitespace the substring matcher would never see.
    NeedleHygiene,
    /// A non-`seed` needle/code lacks the required `source` citation.
    ProvenanceCoherence,
    /// A needle/code claims `evidence: seed` but is absent from the facts seed.
    InventedSeed,
    /// No overload/capacity vocabulary in any bucket AND no `gaps` entry
    /// acknowledging it.
    MotivatingClass,
}

/// One deterministic finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: Check,
    pub branch: Option<Branch>,
    pub detail: String,
}

/// The full findings report for one provider's research document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FindingsReport {
    pub provider: String,
    pub findings: Vec<Finding>,
}

impl FindingsReport {
    /// A clean report has no surviving findings — the fleet leaves no file.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Runs every deterministic check on one provider's research vocabulary
/// against its facts seed. Pure and order-stable: findings are grouped by
/// check in a fixed sequence so the same inputs always produce the same file.
///
/// `seed` is `None` for a parser-less provider (no facts seed to preserve);
/// seed-preservation and invented-seed then degenerate to "any `evidence:
/// seed` needle is invented", which is the correct behavior.
pub fn evaluate(
    provider: &str,
    seed: Option<&ErrorVocabulary>,
    research: &ResearchVocabulary,
) -> FindingsReport {
    let mut findings = Vec::new();

    check_seed_preservation(seed, research, &mut findings);
    check_needle_hygiene(research, &mut findings);
    check_provenance(seed, research, &mut findings);
    check_motivating_class(research, &mut findings);

    FindingsReport {
        provider: provider.to_string(),
        findings,
    }
}

/// Every seeded needle/code must reappear in the research frontmatter of the
/// same branch (R1 mechanical half). Text is compared verbatim — seeds are
/// already lowercase, and hygiene separately catches a research needle that
/// mangled one.
fn check_seed_preservation(
    seed: Option<&ErrorVocabulary>,
    research: &ResearchVocabulary,
    findings: &mut Vec<Finding>,
) {
    let Some(seed) = seed else { return };

    let kind_texts = research_texts(&research.kind_buckets);
    let msg_texts = research_texts(&research.msg_buckets);
    let codes = research_codes(research);

    for bucket in &seed.kind_buckets {
        for needle in &bucket.needles {
            if !kind_texts.contains(needle.as_str()) {
                findings.push(Finding {
                    check: Check::SeedPreservation,
                    branch: Some(Branch::Kind),
                    detail: format!(
                        "seeded kind_buckets needle `{needle}` is missing from the research \
                         frontmatter — seeds are sticky (R1); re-add it with `evidence: seed`"
                    ),
                });
            }
        }
    }
    for bucket in &seed.msg_buckets {
        for needle in &bucket.needles {
            if !msg_texts.contains(needle.as_str()) {
                findings.push(Finding {
                    check: Check::SeedPreservation,
                    branch: Some(Branch::Msg),
                    detail: format!(
                        "seeded msg_buckets needle `{needle}` is missing from the research \
                         frontmatter — seeds are sticky (R1); re-add it with `evidence: seed`"
                    ),
                });
            }
        }
    }
    for bucket in &seed.code_buckets {
        if !codes.contains(&bucket.code) {
            findings.push(Finding {
                check: Check::SeedPreservation,
                branch: Some(Branch::Code),
                detail: format!(
                    "seeded code_buckets code `{}` is missing from the research frontmatter — \
                     seeds are sticky (R1); re-add it with `evidence: seed`",
                    bucket.code
                ),
            });
        }
    }
}

/// Needles are matched against ASCII-lowercased input, so a research needle
/// that is not already lowercase or carries stray whitespace could never fire.
fn check_needle_hygiene(research: &ResearchVocabulary, findings: &mut Vec<Finding>) {
    for (branch, buckets) in [
        (Branch::Kind, &research.kind_buckets),
        (Branch::Msg, &research.msg_buckets),
    ] {
        for bucket in buckets {
            for needle in &bucket.needles {
                let text = &needle.text;
                if text.trim().is_empty() {
                    findings.push(Finding {
                        check: Check::NeedleHygiene,
                        branch: Some(branch),
                        detail: format!(
                            "{}: empty or whitespace-only needle text",
                            branch.wire()
                        ),
                    });
                    continue;
                }
                if text != text.trim() {
                    findings.push(Finding {
                        check: Check::NeedleHygiene,
                        branch: Some(branch),
                        detail: format!(
                            "{}: needle `{text}` has leading/trailing whitespace the matcher \
                             never sees",
                            branch.wire()
                        ),
                    });
                }
                if *text != text.to_ascii_lowercase() {
                    findings.push(Finding {
                        check: Check::NeedleHygiene,
                        branch: Some(branch),
                        detail: format!(
                            "{}: needle `{text}` is not lowercase (input is matched \
                             ASCII-lowercased)",
                            branch.wire()
                        ),
                    });
                }
            }
        }
    }
}

/// Provenance coherence: every non-`seed` needle/code needs a `source`
/// citation, and every `evidence: seed` claim must correspond to a real seed
/// needle/code (no invented provenance).
fn check_provenance(
    seed: Option<&ErrorVocabulary>,
    research: &ResearchVocabulary,
    findings: &mut Vec<Finding>,
) {
    let seed_kind = seed.map(|s| flatten_seed(&s.kind_buckets)).unwrap_or_default();
    let seed_msg = seed.map(|s| flatten_seed(&s.msg_buckets)).unwrap_or_default();
    let seed_codes: std::collections::BTreeSet<i64> = seed
        .map(|s| s.code_buckets.iter().map(|c| c.code).collect())
        .unwrap_or_default();

    for (branch, buckets, seed_set) in [
        (Branch::Kind, &research.kind_buckets, &seed_kind),
        (Branch::Msg, &research.msg_buckets, &seed_msg),
    ] {
        for bucket in buckets {
            for needle in &bucket.needles {
                provenance_for(
                    branch,
                    &needle.text,
                    &needle.evidence,
                    needle.source.as_deref(),
                    seed_set.contains(needle.text.as_str()),
                    findings,
                );
            }
        }
    }
    for bucket in &research.code_buckets {
        for code in &bucket.codes {
            provenance_for(
                Branch::Code,
                &code.code.to_string(),
                &code.evidence,
                code.source.as_deref(),
                seed_codes.contains(&code.code),
                findings,
            );
        }
    }
}

/// The per-needle/per-code provenance rule, shared across branches.
fn provenance_for(
    branch: Branch,
    label: &str,
    evidence: &str,
    source: Option<&str>,
    in_seed: bool,
    findings: &mut Vec<Finding>,
) {
    if evidence == "seed" {
        if !in_seed {
            findings.push(Finding {
                check: Check::InventedSeed,
                branch: Some(branch),
                detail: format!(
                    "{}: `{label}` claims `evidence: seed` but is not in the facts seed — cite \
                     real evidence (documented/source_code/issue_tracker/empirical) instead",
                    branch.wire()
                ),
            });
        }
        return;
    }
    // Every non-seed evidence class (documented/source_code/issue_tracker/
    // empirical) requires a stable `source` citation — the sidecar cannot
    // express that conditional, so it is enforced here.
    if source.map(str::trim).unwrap_or("").is_empty() {
        findings.push(Finding {
            check: Check::ProvenanceCoherence,
            branch: Some(branch),
            detail: format!(
                "{}: `{label}` has `evidence: {evidence}` but no `source` citation",
                branch.wire()
            ),
        });
    }
}

/// The motivating incident (Codex "Selected model is at capacity") must be
/// either proposed as vocabulary or acknowledged as an explicit gap.
fn check_motivating_class(research: &ResearchVocabulary, findings: &mut Vec<Finding>) {
    let mut haystacks: Vec<String> = Vec::new();
    for bucket in research.kind_buckets.iter().chain(&research.msg_buckets) {
        for needle in &bucket.needles {
            haystacks.push(needle.text.to_ascii_lowercase());
        }
    }
    for bucket in &research.code_buckets {
        for code in &bucket.codes {
            if let Some(name) = &code.name {
                haystacks.push(name.to_ascii_lowercase());
            }
        }
    }
    for gap in &research.gaps {
        haystacks.push(format!("{} {}", gap.area, gap.notes).to_ascii_lowercase());
    }

    let covered = haystacks
        .iter()
        .any(|h| CAPACITY_TERMS.iter().any(|term| h.contains(term)));
    if !covered {
        findings.push(Finding {
            check: Check::MotivatingClass,
            branch: None,
            detail: "no overload/capacity vocabulary in any bucket and no `gaps` entry \
                     acknowledging it — research the capacity/overload error surfaces \
                     (overloaded, at capacity, resource_exhausted, 429/503) or record the gap"
                .to_string(),
        });
    }
}

/// Flattens research needle text into a lookup set for one branch.
fn research_texts(buckets: &[ResearchBucket]) -> std::collections::BTreeSet<&str> {
    buckets
        .iter()
        .flat_map(|b| b.needles.iter())
        .map(|n| n.text.as_str())
        .collect()
}

/// Collects every research code value across code buckets.
fn research_codes(research: &ResearchVocabulary) -> std::collections::BTreeSet<i64> {
    research
        .code_buckets
        .iter()
        .flat_map(|b| b.codes.iter())
        .map(|c| c.code)
        .collect()
}

/// Flattens a seed branch's needle strings into a lookup set.
fn flatten_seed(
    buckets: &[crate::vocabulary::KeywordBucket],
) -> std::collections::BTreeSet<String> {
    buckets
        .iter()
        .flat_map(|b| b.needles.iter())
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// IO: read inputs, evaluate, write/remove the findings file
// ---------------------------------------------------------------------------

/// Path to one provider's research document.
fn research_doc_path(area: &Path, slug: &str) -> PathBuf {
    area.join(format!("docs/research/{TOPIC}/{slug}.md"))
}

/// The default transient findings-file path (removed on a clean run; never
/// committed). Callers may override it via `--findings`.
pub fn default_findings_path(area: &Path, slug: &str) -> PathBuf {
    area.join(format!("docs/research/{TOPIC}/.findings/{slug}.json"))
}

/// Reads a provider's facts seed vocabulary, or `None` when the provider has
/// no facts seed (parser-less provider, or the seed already graduated).
pub fn read_seed(area: &Path, slug: &str) -> Result<Option<ErrorVocabulary>, GenError> {
    let path = area.join(format!("docs/providers/facts/{slug}.yaml"));
    if !path.is_file() {
        return Ok(None);
    }
    let facts = inputs::read_yaml(&path)?;
    match facts.get(FACTS_KEY) {
        None => Ok(None),
        Some(value) => {
            let vocab: ErrorVocabulary =
                serde_json::from_value(value.clone()).map_err(|err| GenError::VocabularyInvalid {
                    slug: slug.to_string(),
                    message: err.to_string(),
                })?;
            Ok(Some(vocab))
        }
    }
}

/// Reads a provider's research vocabulary from its schema-validated
/// frontmatter (so a malformed document fails the same way generation would).
pub fn read_research(area: &Path, slug: &str) -> Result<ResearchVocabulary, GenError> {
    let path = research_doc_path(area, slug);
    let frontmatter = inputs::load_validated_frontmatter(&path)?;
    parse_research(slug, &frontmatter)
}

/// Deserializes a validated frontmatter object into the research vocabulary.
fn parse_research(slug: &str, frontmatter: &Value) -> Result<ResearchVocabulary, GenError> {
    serde_json::from_value(frontmatter.clone()).map_err(|err| GenError::VocabularyInvalid {
        slug: slug.to_string(),
        message: err.to_string(),
    })
}

/// The full gate: read the seed + research doc, evaluate, and reconcile the
/// findings file (stale removed first, written only when findings survive).
///
/// ## Returns
///
/// The [`FindingsReport`], whether or not it is clean, so the caller can print
/// a summary and exit zero (findings are communicated via the file, per D10).
pub fn check_provider(
    area: &Path,
    slug: &str,
    findings_path: &Path,
) -> Result<FindingsReport, GenError> {
    let seed = read_seed(area, slug)?;
    let research = read_research(area, slug)?;
    let report = evaluate(slug, seed.as_ref(), &research);
    reconcile_findings_file(findings_path, &report)?;
    Ok(report)
}

/// Removes any stale findings file, then writes the report atomically only
/// when it carries at least one finding.
fn reconcile_findings_file(path: &Path, report: &FindingsReport) -> Result<(), GenError> {
    // Stale-first: a prior run's file must never survive a now-clean run.
    if path.exists() {
        fs::remove_file(path).map_err(|source| GenError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    if report.is_clean() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| GenError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(report).map_err(|err| GenError::Json {
        message: err.to_string(),
    })?;
    atomic_write(path, &json)
}

/// Writes `contents` to `path` via a sibling temp file + rename so a reader
/// never sees a partial JSON document. The destination was already removed by
/// [`reconcile_findings_file`], so the rename lands cleanly on every OS.
fn atomic_write(path: &Path, contents: &str) -> Result<(), GenError> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, contents).map_err(|source| GenError::Io {
        path: tmp.clone(),
        source,
    })?;
    fs::rename(&tmp, path).map_err(|source| GenError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::{CodeBucket, KeywordBucket};

    fn seed() -> ErrorVocabulary {
        ErrorVocabulary {
            kind_buckets: vec![KeywordBucket {
                kind: "api_remote".into(),
                needles: vec!["rate".into(), "quota".into()],
            }],
            msg_buckets: vec![KeywordBucket {
                kind: "configuration".into(),
                needles: vec!["api key".into()],
            }],
            code_buckets: vec![CodeBucket {
                code: -32001,
                kind: "configuration".into(),
            }],
        }
    }

    fn needle(text: &str, evidence: &str, source: Option<&str>) -> ResearchNeedle {
        ResearchNeedle {
            text: text.into(),
            evidence: evidence.into(),
            source: source.map(str::to_string),
        }
    }

    /// A research doc that preserves every seed, cites non-seed evidence, and
    /// covers the capacity class — no findings.
    fn clean_research() -> ResearchVocabulary {
        ResearchVocabulary {
            kind_buckets: vec![ResearchBucket {
                kind: "api_remote".into(),
                needles: vec![
                    needle("rate", "seed", None),
                    needle("quota", "seed", None),
                    needle("overloaded", "documented", Some("https://example/docs")),
                ],
            }],
            msg_buckets: vec![ResearchBucket {
                kind: "configuration".into(),
                needles: vec![needle("api key", "seed", None)],
            }],
            code_buckets: vec![ResearchCodeBucket {
                kind: "configuration".into(),
                codes: vec![ResearchCode {
                    code: -32001,
                    name: Some("AUTH_EXPIRED".into()),
                    evidence: "seed".into(),
                    source: None,
                }],
            }],
            gaps: vec![],
        }
    }

    #[test]
    fn clean_research_yields_no_findings() {
        let report = evaluate("codex", Some(&seed()), &clean_research());
        assert!(report.is_clean(), "unexpected findings: {:?}", report.findings);
    }

    #[test]
    fn dropped_seed_needle_is_flagged() {
        let mut research = clean_research();
        research.kind_buckets[0].needles.retain(|n| n.text != "quota");
        let report = evaluate("codex", Some(&seed()), &research);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == Check::SeedPreservation && f.detail.contains("quota")),
            "expected a seed-preservation finding, got {:?}",
            report.findings
        );
    }

    #[test]
    fn dropped_seed_code_is_flagged() {
        let mut research = clean_research();
        research.code_buckets.clear();
        let report = evaluate("kimi", Some(&seed()), &research);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == Check::SeedPreservation && f.branch == Some(Branch::Code)),
            "expected a code seed-preservation finding, got {:?}",
            report.findings
        );
    }

    #[test]
    fn uppercase_and_whitespace_needles_are_flagged() {
        let mut research = clean_research();
        research.msg_buckets[0]
            .needles
            .push(needle("API Error", "documented", Some("https://x")));
        research.msg_buckets[0]
            .needles
            .push(needle(" padded ", "documented", Some("https://x")));
        let report = evaluate("codex", Some(&seed()), &research);
        let hygiene = report
            .findings
            .iter()
            .filter(|f| f.check == Check::NeedleHygiene)
            .count();
        assert!(hygiene >= 2, "expected hygiene findings, got {:?}", report.findings);
    }

    #[test]
    fn non_seed_needle_without_source_is_flagged() {
        let mut research = clean_research();
        research.kind_buckets[0]
            .needles
            .push(needle("upstream", "source_code", None));
        let report = evaluate("codex", Some(&seed()), &research);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == Check::ProvenanceCoherence && f.detail.contains("upstream")),
            "expected a provenance finding, got {:?}",
            report.findings
        );
    }

    #[test]
    fn invented_seed_evidence_is_flagged() {
        let mut research = clean_research();
        // `billing` is NOT in the seed but claims to be a seed.
        research.kind_buckets[0]
            .needles
            .push(needle("billing", "seed", None));
        let report = evaluate("codex", Some(&seed()), &research);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == Check::InventedSeed && f.detail.contains("billing")),
            "expected an invented-seed finding, got {:?}",
            report.findings
        );
    }

    #[test]
    fn missing_capacity_class_is_flagged_but_a_gap_satisfies_it() {
        let mut research = clean_research();
        research.kind_buckets[0].needles.retain(|n| n.text != "overloaded");
        let report = evaluate("codex", Some(&seed()), &research);
        assert!(
            report.findings.iter().any(|f| f.check == Check::MotivatingClass),
            "expected a motivating-class finding, got {:?}",
            report.findings
        );

        // Acknowledging the gap clears it, even with no capacity needle.
        research.gaps.push(Gap {
            area: "capacity".into(),
            notes: "could not confirm the 503 overload phrasing in the CLI source".into(),
        });
        let report = evaluate("codex", Some(&seed()), &research);
        assert!(
            !report.findings.iter().any(|f| f.check == Check::MotivatingClass),
            "gap should satisfy motivating-class, got {:?}",
            report.findings
        );
    }

    #[test]
    fn reconcile_writes_on_findings_and_removes_stale_on_clean() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("codex.json");

        let dirty = FindingsReport {
            provider: "codex".into(),
            findings: vec![Finding {
                check: Check::MotivatingClass,
                branch: None,
                detail: "x".into(),
            }],
        };
        reconcile_findings_file(&path, &dirty).expect("write findings");
        assert!(path.exists(), "findings file should be written");

        // A subsequent clean run removes the stale file.
        let clean = FindingsReport {
            provider: "codex".into(),
            findings: vec![],
        };
        reconcile_findings_file(&path, &clean).expect("clean reconcile");
        assert!(!path.exists(), "clean run must remove the stale findings file");
    }
}
