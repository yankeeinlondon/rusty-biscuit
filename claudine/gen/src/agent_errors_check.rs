//! Deterministic validate-and-resume gate for the `agent-errors` research
//! topic (spec `2026-07-11-provider-errors-as-data`, D10).
//!
//! This is the mechanical half of the fleet lifecycle: it reads one provider's
//! `agent-errors/<slug>.md` research frontmatter and its immutable Phase-A seed,
//! runs the deterministic checks the fleet cannot self-attest, and writes an
//! explicit Markdown outcome report. The fleet `success` stack branches on
//! that report's `status` instead of inferring success from file absence.
//!
//! ## Outcome-report contract
//!
//! Every completed check writes exactly one typed state: `clean`, `findings`,
//! or `gate_error`. Replacement uses a synced sibling temporary file and an
//! atomic persist, so the last valid failure report remains visible until its
//! replacement is durably ready. If the replacement itself fails, the command
//! returns an error and the fleet lifecycle stops before evaluating a stale
//! report.
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
use std::io::Write;
use std::path::{Path, PathBuf};

use biscuit_file::FileReference;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::GenError;
use crate::inputs;
use crate::vocabulary::ErrorVocabulary;

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
    #[serde(default)]
    pub empirical: Option<EmpiricalEvidence>,
}

/// Capture details required when a row uses `evidence: empirical`.
#[derive(Debug, Clone, Deserialize)]
pub struct EmpiricalEvidence {
    pub fixture: String,
    pub capture_notes: String,
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
    #[serde(default)]
    pub empirical: Option<EmpiricalEvidence>,
}

/// An unresearchable-area acknowledgement.
#[derive(Debug, Clone, Deserialize)]
pub struct Gap {
    pub area: String,
    pub notes: String,
}

/// Which structured branch a needle lives in — carried on findings so a
/// resume message can point the model at the exact list to fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// A seeded needle/code is absent from its research branch.
    SeedRemoval,
    /// A seeded needle/code moved to a different semantic kind.
    SeedRekind,
    /// A seeded needle/code changed bucket or item position.
    SeedReorder,
    /// A research needle violates the lowercase or leading/trailing-whitespace
    /// input hygiene required by the runtime matcher.
    NeedleHygiene,
    /// A non-`seed` row lacks its required source or empirical capture data.
    ProvenanceCoherence,
    /// A needle/code claims `evidence: seed` but does not match its immutable
    /// Phase-A seed row.
    InventedSeed,
    /// No overload/capacity vocabulary in any bucket AND no `gaps` entry
    /// acknowledging it.
    MotivatingClass,
}

/// One deterministic finding.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Finding {
    pub check: Check,
    pub branch: Option<Branch>,
    pub detail: String,
}

/// The explicit result of one deterministic gate execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Clean,
    Findings,
    GateError,
}

/// Authority boundary for a deterministic gate error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GateErrorScope {
    /// The provider-authored research document can be corrected by resuming
    /// the same research session.
    ResearchDocument,
    /// An immutable seed or other authoritative checker input needs
    /// maintainer intervention.
    GateInput,
}

/// The durable outcome report for one provider's research document.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FindingsReport {
    pub status: GateStatus,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<Finding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_scope: Option<GateErrorScope>,
}

impl FindingsReport {
    /// A clean report is explicit and never inferred from file absence.
    pub fn is_clean(&self) -> bool {
        self.status == GateStatus::Clean
    }

    fn gate_error(provider: &str, scope: GateErrorScope, error: impl Into<String>) -> Self {
        Self {
            status: GateStatus::GateError,
            provider: provider.to_string(),
            findings: Vec::new(),
            error: Some(error.into()),
            error_scope: Some(scope),
        }
    }
}

/// Runs every deterministic check on one provider's research vocabulary
/// against its immutable Phase-A seed. Pure and order-stable: findings are grouped by
/// check in a fixed sequence so the same inputs always produce the same file.
///
/// `seed` is `None` for a parser-less provider (no Phase-A seed to preserve);
/// seed-preservation and invented-seed then degenerate to "any `evidence:
/// seed` needle is invented", which is the correct behavior.
pub fn evaluate(
    provider: &str,
    seed: Option<&ErrorVocabulary>,
    research: &ResearchVocabulary,
) -> FindingsReport {
    evaluate_with_fixture_base(provider, seed, research, None)
}

fn evaluate_with_fixture_base(
    provider: &str,
    seed: Option<&ErrorVocabulary>,
    research: &ResearchVocabulary,
    fixture_base: Option<&Path>,
) -> FindingsReport {
    let mut findings = Vec::new();

    check_seed_preservation(seed, research, &mut findings);
    check_needle_hygiene(research, &mut findings);
    check_provenance(seed, research, fixture_base, &mut findings);
    check_motivating_class(research, &mut findings);

    FindingsReport {
        status: if findings.is_empty() {
            GateStatus::Clean
        } else {
            GateStatus::Findings
        },
        provider: provider.to_string(),
        findings,
        error: None,
        error_scope: None,
    }
}

/// Every seeded needle/code must retain its branch, bucket position, semantic
/// kind, item position, and value (R1/R3/R4 mechanical half). A mismatch is
/// classified as removal, re-kind, or reorder so adjudication can apply the
/// corresponding rule deliberately.
fn check_seed_preservation(
    seed: Option<&ErrorVocabulary>,
    research: &ResearchVocabulary,
    findings: &mut Vec<Finding>,
) {
    let Some(seed) = seed else { return };

    let seed_rows = seed_rows(seed);
    let research_rows = research_rows(research);

    for seeded in &seed_rows {
        if research_rows.iter().any(|candidate| candidate.same_identity(seeded)) {
            continue;
        }

        let same_value: Vec<&SeedRow> = research_rows
            .iter()
            .filter(|candidate| candidate.branch == seeded.branch && candidate.value == seeded.value)
            .collect();
        let (check, detail) = if same_value.is_empty() {
            (
                Check::SeedRemoval,
                format!(
                    "seeded {} {} `{}` at bucket {}, item {} is missing from that research \
                     branch — seeds are sticky (R1)",
                    seeded.branch.wire(),
                    seeded.value.label(),
                    seeded.value,
                    seeded.bucket_index,
                    seeded.item_index
                ),
            )
        } else if same_value.iter().all(|candidate| candidate.kind != seeded.kind) {
            let candidate = same_value[0];
            (
                Check::SeedRekind,
                format!(
                    "seeded {} {} `{}` changed semantic kind from `{}` to `{}` — re-kinds \
                     require explicit R4 adjudication",
                    seeded.branch.wire(),
                    seeded.value.label(),
                    seeded.value,
                    seeded.kind,
                    candidate.kind
                ),
            )
        } else {
            let candidate = same_value
                .iter()
                .find(|candidate| candidate.kind == seeded.kind)
                .expect("same-kind candidate established above");
            (
                Check::SeedReorder,
                format!(
                    "seeded {} {} `{}` moved from bucket {}, item {} to bucket {}, item {} — \
                     cascade-order changes require explicit R3 adjudication",
                    seeded.branch.wire(),
                    seeded.value.label(),
                    seeded.value,
                    seeded.bucket_index,
                    seeded.item_index,
                    candidate.bucket_index,
                    candidate.item_index
                ),
            )
        };
        findings.push(Finding {
            check,
            branch: Some(seeded.branch),
            detail,
        });
    }
}

/// Enforces the normalized authored-input form required by the runtime's
/// ASCII-lowercased substring matcher.
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
/// citation, and every `evidence: seed` claim must match a complete seed-row
/// identity (no invented or behavior-changing provenance).
fn check_provenance(
    seed: Option<&ErrorVocabulary>,
    research: &ResearchVocabulary,
    fixture_base: Option<&Path>,
    findings: &mut Vec<Finding>,
) {
    let seed_rows = seed.map(seed_rows).unwrap_or_default();
    for row in research_rows_with_provenance(research) {
        let in_seed = seed_rows.iter().any(|seeded| row.row.same_identity(seeded));
        provenance_for(&row, in_seed, fixture_base, findings);
    }
}

/// The per-needle/per-code provenance rule, shared across branches.
fn provenance_for(
    row: &ProvenanceRow<'_>,
    in_seed: bool,
    fixture_base: Option<&Path>,
    findings: &mut Vec<Finding>,
) {
    let branch = row.row.branch;
    let label = row.row.value.to_string();
    let evidence = row.evidence;
    if evidence == "seed" {
        if !in_seed {
            findings.push(Finding {
                check: Check::InventedSeed,
                branch: Some(branch),
                detail: format!(
                    "{}: `{label}` claims `evidence: seed` but does not match the seed's branch, \
                     bucket, semantic kind, and item position — restore the seed row or cite \
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
    if row.source.map(str::trim).unwrap_or("").is_empty() {
        findings.push(Finding {
            check: Check::ProvenanceCoherence,
            branch: Some(branch),
            detail: format!(
                "{}: `{label}` has `evidence: {evidence}` but no `source` citation",
                branch.wire()
            ),
        });
    }
    if evidence == "empirical" {
        check_empirical_provenance(
            branch,
            &label,
            row.empirical,
            fixture_base,
            findings,
        );
    }
}

fn check_empirical_provenance(
    branch: Branch,
    label: &str,
    empirical: Option<&EmpiricalEvidence>,
    fixture_base: Option<&Path>,
    findings: &mut Vec<Finding>,
) {
    let Some(empirical) = empirical else {
        findings.push(Finding {
            check: Check::ProvenanceCoherence,
            branch: Some(branch),
            detail: format!(
                "{}: empirical `{label}` has no `empirical.fixture` and \
                 `empirical.capture_notes`",
                branch.wire()
            ),
        });
        return;
    };

    if empirical.capture_notes.trim().is_empty() {
        findings.push(Finding {
            check: Check::ProvenanceCoherence,
            branch: Some(branch),
            detail: format!(
                "{}: empirical `{label}` has empty `empirical.capture_notes`",
                branch.wire()
            ),
        });
    }

    let fixture = empirical.fixture.as_str();
    if !is_scoped_fixture_reference(fixture) {
        findings.push(Finding {
            check: Check::ProvenanceCoherence,
            branch: Some(branch),
            detail: format!(
                "{}: empirical `{label}` fixture `{fixture}` must be a portable, traversal-free \
                 `./_fixtures/...` file reference",
                branch.wire()
            ),
        });
        return;
    }

    let Some(fixture_base) = fixture_base else {
        return;
    };
    let resolved = FileReference::new(fixture)
        .and_then(|reference| reference.resolve_from(fixture_base));
    match resolved {
        Ok(Some(_)) => {}
        Ok(None) => findings.push(Finding {
            check: Check::ProvenanceCoherence,
            branch: Some(branch),
            detail: format!(
                "{}: empirical `{label}` fixture `{fixture}` does not resolve to an existing file",
                branch.wire()
            ),
        }),
        Err(error) => findings.push(Finding {
            check: Check::ProvenanceCoherence,
            branch: Some(branch),
            detail: format!(
                "{}: empirical `{label}` fixture `{fixture}` is invalid: {error}",
                branch.wire()
            ),
        }),
    }
}

fn is_scoped_fixture_reference(fixture: &str) -> bool {
    if fixture != fixture.trim()
        || !fixture.starts_with("./_fixtures/")
        || fixture.contains('\\')
    {
        return false;
    }
    let relative = &fixture["./_fixtures/".len()..];
    !relative.is_empty()
        && relative.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && !component.contains("{{")
                && !component.contains("}}")
                && !component.chars().any(|character| {
                    character.is_control()
                        || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
                })
                && !component.ends_with(' ')
                && !component.ends_with('.')
        })
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
            haystacks.push(code.code.to_string());
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum SeedValue {
    Needle(String),
    Code(i64),
}

impl SeedValue {
    fn label(&self) -> &'static str {
        match self {
            Self::Needle(_) => "needle",
            Self::Code(_) => "code",
        }
    }
}

impl std::fmt::Display for SeedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Needle(value) => value.fmt(f),
            Self::Code(value) => value.fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SeedRow {
    branch: Branch,
    bucket_index: usize,
    kind: String,
    item_index: usize,
    value: SeedValue,
}

impl SeedRow {
    fn same_identity(&self, other: &Self) -> bool {
        self == other
    }
}

struct ProvenanceRow<'a> {
    row: SeedRow,
    evidence: &'a str,
    source: Option<&'a str>,
    empirical: Option<&'a EmpiricalEvidence>,
}

fn keyword_rows(
    branch: Branch,
    buckets: &[crate::vocabulary::KeywordBucket],
) -> Vec<SeedRow> {
    buckets
        .iter()
        .enumerate()
        .flat_map(|(bucket_index, bucket)| {
            bucket.needles.iter().enumerate().map(move |(item_index, needle)| SeedRow {
                branch,
                bucket_index,
                kind: bucket.kind.clone(),
                item_index,
                value: SeedValue::Needle(needle.clone()),
            })
        })
        .collect()
}

fn seed_rows(seed: &ErrorVocabulary) -> Vec<SeedRow> {
    let mut rows = keyword_rows(Branch::Kind, &seed.kind_buckets);
    rows.extend(keyword_rows(Branch::Msg, &seed.msg_buckets));
    rows.extend(seed.code_buckets.iter().enumerate().map(|(bucket_index, bucket)| SeedRow {
        branch: Branch::Code,
        bucket_index,
        kind: bucket.kind.clone(),
        item_index: 0,
        value: SeedValue::Code(bucket.code),
    }));
    rows
}

fn research_rows(research: &ResearchVocabulary) -> Vec<SeedRow> {
    research_rows_with_provenance(research)
        .into_iter()
        .map(|row| row.row)
        .collect()
}

fn research_rows_with_provenance(research: &ResearchVocabulary) -> Vec<ProvenanceRow<'_>> {
    let mut rows = Vec::new();
    for (branch, buckets) in [
        (Branch::Kind, &research.kind_buckets),
        (Branch::Msg, &research.msg_buckets),
    ] {
        for (bucket_index, bucket) in buckets.iter().enumerate() {
            for (item_index, needle) in bucket.needles.iter().enumerate() {
                rows.push(ProvenanceRow {
                    row: SeedRow {
                        branch,
                        bucket_index,
                        kind: bucket.kind.clone(),
                        item_index,
                        value: SeedValue::Needle(needle.text.clone()),
                    },
                    evidence: &needle.evidence,
                    source: needle.source.as_deref(),
                    empirical: needle.empirical.as_ref(),
                });
            }
        }
    }
    for (bucket_index, bucket) in research.code_buckets.iter().enumerate() {
        for (item_index, code) in bucket.codes.iter().enumerate() {
            rows.push(ProvenanceRow {
                row: SeedRow {
                    branch: Branch::Code,
                    bucket_index,
                    kind: bucket.kind.clone(),
                    item_index,
                    value: SeedValue::Code(code.code),
                },
                evidence: &code.evidence,
                source: code.source.as_deref(),
                empirical: code.empirical.as_ref(),
            });
        }
    }
    rows
}

// ---------------------------------------------------------------------------
// IO: read inputs, evaluate, write/remove the findings file
// ---------------------------------------------------------------------------

/// Path to one provider's research document.
fn research_doc_path(area: &Path, slug: &str) -> PathBuf {
    area.join(format!("docs/research/{TOPIC}/{slug}.md"))
}

/// The default transient outcome-report path (never committed). Callers may
/// override it via `--findings`.
pub fn default_findings_path(area: &Path, slug: &str) -> PathBuf {
    area.join(format!("docs/research/{TOPIC}/.findings/{slug}.md"))
}

/// Reads a provider's immutable Phase-A seed baseline, or `None` when the
/// provider never had runtime vocabulary (currently Goose).
pub fn read_seed(area: &Path, slug: &str) -> Result<Option<ErrorVocabulary>, GenError> {
    let path = area.join(format!("docs/research/{TOPIC}/_seeds/{slug}.yaml"));
    if !path.is_file() {
        return Ok(None);
    }
    let value = inputs::read_yaml(&path)?;
    let vocab = serde_json::from_value(value).map_err(|err| GenError::VocabularyInvalid {
        slug: slug.to_string(),
        message: err.to_string(),
    })?;
    Ok(Some(vocab))
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

/// The full gate: read the seed + research doc, evaluate, and atomically replace
/// the explicit outcome report.
///
/// ## Returns
///
/// The [`FindingsReport`] after it has been durably persisted. Research
/// document failures become repairable `gate_error` reports; seed and other
/// authoritative input failures become terminal `gate_error` reports. An
/// inability to persist the report is returned as an error so lifecycle
/// processing cannot inspect stale state.
pub fn check_provider(
    area: &Path,
    slug: &str,
    findings_path: &Path,
) -> Result<FindingsReport, GenError> {
    let fixture_base = area.join(format!("docs/research/{TOPIC}"));
    let report = match read_seed(area, slug) {
        Err(error) => {
            FindingsReport::gate_error(slug, GateErrorScope::GateInput, error.to_string())
        }
        Ok(seed) => match read_research(area, slug) {
            Ok(research) => {
                evaluate_with_fixture_base(slug, seed.as_ref(), &research, Some(&fixture_base))
            }
            Err(error) => FindingsReport::gate_error(
                slug,
                GateErrorScope::ResearchDocument,
                error.to_string(),
            ),
        },
    };
    write_outcome_report(findings_path, &report)?;
    Ok(report)
}

/// Atomically replaces the prior outcome without deleting it first.
fn write_outcome_report(path: &Path, report: &FindingsReport) -> Result<(), GenError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| GenError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let yaml = serde_yaml_ng::to_string(report).map_err(|err| GenError::Json {
        message: err.to_string(),
    })?;
    let contents = format!("---\n{yaml}---\n\n# Agent Errors Gate Outcome\n");
    atomic_write(path, contents.as_bytes())
}

/// Writes and syncs a unique sibling temporary file before atomically replacing
/// the destination. `persist` overwrites on all supported platforms.
fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), GenError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|source| GenError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    tmp.as_file_mut()
        .write_all(contents)
        .and_then(|()| tmp.as_file_mut().sync_all())
        .map_err(|source| GenError::Io {
            path: tmp.path().to_path_buf(),
            source,
        })?;
    tmp.persist(path).map_err(|error| GenError::Io {
        path: path.to_path_buf(),
        source: error.error,
    })?;
    #[cfg(unix)]
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| GenError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod review6_tests;
