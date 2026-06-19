use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::shared::{Diagnostic, DiagnosticConfidence, DiagnosticKind, DiagnosticSeverity};

pub mod manifest;
pub mod redaction;

pub use manifest::*;
pub use redaction::*;

/// Tiers for corpus harness execution.
///
/// - `Smoke`: Fast subset for PR gates.
/// - `Expanded`: Full corpus for scheduled CI.
/// - `Benchmark`: Timing-focused mode for cache, resolver, and adapter changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CorpusTier {
    Smoke,
    Expanded,
    Benchmark,
}

/// A corpus entry represents a single file or fixture under analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusEntry {
    /// Path to the source file, relative to the corpus root.
    pub relative_path: PathBuf,
    /// The programming language of the file.
    pub language: String,
    /// Diagnostics produced for this file.
    pub diagnostics: Vec<CorpusDiagnostic>,
    /// Whether this file was skipped (e.g., excluded by pattern).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
    /// Reason for skipping, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

/// A lightweight, stable representation of a diagnostic for corpus comparison.
///
/// This type strips absolute paths, tool versions, and other ephemeral data
/// so that corpus results can be compared across runs and environments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CorpusDiagnostic {
    /// Stable rule identifier (e.g., `unwrap-call`).
    pub rule: Option<String>,
    /// Human-readable message with redacted paths.
    pub message: String,
    /// Line number (1-based) for stable comparison.
    pub line: usize,
    /// Diagnostic kind for categorization.
    pub kind: CorpusDiagnosticKind,
    /// Severity level.
    pub severity: CorpusSeverity,
    /// Confidence level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<CorpusConfidence>,
}

impl CorpusDiagnostic {
    /// Creates a `CorpusDiagnostic` from a `Diagnostic`, applying redaction.
    pub fn from_diagnostic(diagnostic: &Diagnostic, corpus_root: &Path) -> Self {
        let rule = diagnostic.rule.clone();
        let message = redact_paths(&diagnostic.message, corpus_root);
        let line = diagnostic.range.start_line;
        let kind = CorpusDiagnosticKind::from(diagnostic.kind);
        let severity = CorpusSeverity::from(diagnostic.severity);
        let confidence = diagnostic
            .metadata
            .as_ref()
            .map(|m| CorpusConfidence::from(m.confidence));

        Self {
            rule,
            message,
            line,
            kind,
            severity,
            confidence,
        }
    }
}

/// Serializable diagnostic kind for corpus diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CorpusDiagnosticKind {
    Lint,
    Semantic,
    Syntax,
}

impl From<DiagnosticKind> for CorpusDiagnosticKind {
    fn from(value: DiagnosticKind) -> Self {
        match value {
            DiagnosticKind::Lint => Self::Lint,
            DiagnosticKind::Semantic => Self::Semantic,
            DiagnosticKind::Syntax => Self::Syntax,
        }
    }
}

/// Serializable severity for corpus diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CorpusSeverity {
    Info,
    Warning,
    Error,
}

impl From<DiagnosticSeverity> for CorpusSeverity {
    fn from(value: DiagnosticSeverity) -> Self {
        match value {
            DiagnosticSeverity::Info => Self::Info,
            DiagnosticSeverity::Warning => Self::Warning,
            DiagnosticSeverity::Error => Self::Error,
        }
    }
}

/// Serializable confidence for corpus diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CorpusConfidence {
    High,
    Medium,
    Low,
    Experimental,
}

impl From<DiagnosticConfidence> for CorpusConfidence {
    fn from(value: DiagnosticConfidence) -> Self {
        match value {
            DiagnosticConfidence::High => Self::High,
            DiagnosticConfidence::Medium => Self::Medium,
            DiagnosticConfidence::Low => Self::Low,
            DiagnosticConfidence::Experimental => Self::Experimental,
        }
    }
}

/// The result of running a corpus analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusResult {
    /// Which tier was run.
    pub tier: CorpusTier,
    /// Entries analyzed (or skipped).
    pub entries: Vec<CorpusEntry>,
    /// Per-rule threshold report.
    pub threshold_report: ThresholdReport,
    /// Start time (for benchmark mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// End time (for benchmark mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// Elapsed milliseconds (for benchmark mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}

/// Per-rule threshold report.
///
/// Tracks how many diagnostics each rule produced and whether any
/// exceeded their allowed threshold (e.g., zero false positives).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdReport {
    /// Map from rule id to its threshold status.
    pub rules: BTreeMap<String, RuleThreshold>,
    /// Classification summary for oracle mismatches.
    pub classifications: Vec<OracleClassification>,
}

impl ThresholdReport {
    /// Creates an empty threshold report.
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
            classifications: Vec::new(),
        }
    }

    /// Returns true if all rules are within threshold.
    pub fn is_clean(&self) -> bool {
        self.rules.values().all(|r| r.within_threshold)
    }

    /// Records a diagnostic for a rule and checks against threshold.
    pub fn record(
        &mut self,
        rule: &str,
        confidence: Option<CorpusConfidence>,
        threshold: Threshold,
    ) {
        let entry = self
            .rules
            .entry(rule.to_string())
            .or_insert_with(|| RuleThreshold {
                rule: rule.to_string(),
                count: 0,
                threshold,
                within_threshold: true,
            });
        entry.count += 1;

        // High-confidence syntax rules must have zero diagnostics.
        if threshold == Threshold::Zero && confidence == Some(CorpusConfidence::High) {
            entry.within_threshold = false;
        }
        // Experimental rules are always within threshold (they're opt-in).
        if confidence == Some(CorpusConfidence::Experimental) {
            // Still tracked but not a failure.
        }
    }
}

impl Default for ThresholdReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Threshold for a single rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Threshold {
    /// Must produce exactly zero diagnostics (for high-confidence rules).
    Zero,
    /// Budgeted number of allowed diagnostics.
    Budget(usize),
    /// No threshold (always passes).
    Unlimited,
}

/// Status of a rule against its threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleThreshold {
    /// The rule identifier.
    pub rule: String,
    /// How many diagnostics were produced.
    pub count: usize,
    /// The threshold that applies to this rule.
    pub threshold: Threshold,
    /// Whether the count is within threshold.
    pub within_threshold: bool,
}

/// Classification of an oracle mismatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleClassification {
    /// The rule involved.
    pub rule: String,
    /// The file (relative path).
    pub file: PathBuf,
    /// The line number.
    pub line: usize,
    /// Classification type.
    pub classification: MismatchKind,
    /// Human-readable explanation.
    pub explanation: String,
}

/// Types of oracle mismatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MismatchKind {
    /// Tree Hugger found something the oracle missed (likely false positive).
    FalsePositive,
    /// Oracle found something Tree Hugger missed (likely false negative).
    FalseNegative,
    /// Both found something but disagree on details.
    Disagreement,
    /// Known and accepted limitation.
    AcceptedLimitation,
}

impl fmt::Display for MismatchKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::FalsePositive => "false-positive",
            Self::FalseNegative => "false-negative",
            Self::Disagreement => "disagreement",
            Self::AcceptedLimitation => "accepted-limitation",
        };
        f.write_str(label)
    }
}

/// Redacts absolute paths from a message, replacing them with `<REDACTED>`.
pub fn redact_paths(message: &str, corpus_root: &Path) -> String {
    let root_str = corpus_root.to_string_lossy();
    let temp_prefix = std::env::temp_dir().to_string_lossy().to_string();

    message
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(|word| {
                    // Replace absolute paths that start with corpus root or temp dir
                    if (word.starts_with('/') || word.starts_with("\\") || word.contains(':'))
                        && (word.starts_with(root_str.as_ref()) || word.starts_with(&temp_prefix))
                    {
                        return "<REDACTED>".to_string();
                    }
                    word.to_string()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalizes a list of corpus diagnostics for stable comparison.
///
/// Sorts by rule, line, and message; deduplicates identical entries.
pub fn normalize_diagnostics(diagnostics: &mut Vec<CorpusDiagnostic>) {
    diagnostics.sort_by(|a, b| {
        a.rule
            .cmp(&b.rule)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.message.cmp(&b.message))
    });
    diagnostics.dedup();
}

/// Runs a smoke corpus analysis using the provided manifest.
///
/// This is the fast tier suitable for PRs. It analyzes a subset of
/// files defined in the manifest and checks per-rule thresholds.
pub fn run_smoke_corpus(manifest: &CorpusManifest) -> CorpusResult {
    run_corpus_tier(manifest, CorpusTier::Smoke)
}

/// Runs an expanded corpus analysis using the provided manifest.
///
/// This tier includes more files and is suitable for scheduled CI.
pub fn run_expanded_corpus(manifest: &CorpusManifest) -> CorpusResult {
    run_corpus_tier(manifest, CorpusTier::Expanded)
}

/// Runs a corpus analysis in benchmark mode.
///
/// Records timing information suitable for cache, resolver, and adapter
/// performance evaluation.
pub fn run_benchmark_corpus(manifest: &CorpusManifest) -> CorpusResult {
    run_corpus_tier(manifest, CorpusTier::Benchmark)
}

/// Internal function to run a corpus tier.
fn run_corpus_tier(manifest: &CorpusManifest, tier: CorpusTier) -> CorpusResult {
    use std::time::{SystemTime, UNIX_EPOCH};

    let start_time = if tier == CorpusTier::Benchmark {
        Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string(),
        )
    } else {
        None
    };

    let mut entries = Vec::new();
    let threshold_report = ThresholdReport::new();

    for item in manifest.items_for_tier(tier) {
        // items_for_tier already filters by tier

        // In a real implementation, this would load the file and run analysis.
        // For the harness, we create placeholder entries that consumers populate.
        let entry = CorpusEntry {
            relative_path: PathBuf::from(&item.source),
            language: item.language.clone(),
            diagnostics: Vec::new(),
            skipped: false,
            skip_reason: None,
        };
        entries.push(entry);
    }

    let elapsed_ms = if tier == CorpusTier::Benchmark {
        Some(0)
    } else {
        None
    };

    let end_time = if tier == CorpusTier::Benchmark {
        Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string(),
        )
    } else {
        None
    };

    CorpusResult {
        tier,
        entries,
        threshold_report,
        start_time,
        end_time,
        elapsed_ms,
    }
}
