//! Types for god-file analysis.

use std::collections::BTreeMap;
use std::path::PathBuf;

use biscuit_file::FileReference;
use serde::{Deserialize, Serialize};

use crate::shared::{ProgrammingLanguage, SymbolKind};

use super::constants::{HIGH_MIN_SLOC, MODERATE_MIN_SLOC};

/// Risk band for a god-file candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskBand {
    Moderate,
    High,
}

impl RiskBand {
    /// Classify effective SLOC into a risk band.
    ///
    /// Files below `MODERATE_MIN_SLOC` are not candidates and do not
    /// produce a `RiskBand`.
    pub fn from_sloc(effective_sloc: usize) -> Option<Self> {
        if effective_sloc >= HIGH_MIN_SLOC {
            Some(Self::High)
        } else if effective_sloc >= MODERATE_MIN_SLOC {
            Some(Self::Moderate)
        } else {
            None
        }
    }
}

/// A symbol block ranked by effective SLOC within its span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolBlock {
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: usize,
    pub end_line: usize,
    pub sloc: usize,
    /// First line of the symbol's doc comment, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_summary: Option<String>,
    /// Present when the container has more than `MANY_MEMBERS_THRESHOLD`
    /// structural members.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub many_members: Option<ContainerCallout>,
}

/// Call-out for a container with many structural members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerCallout {
    pub member_count: usize,
    pub members: Vec<MemberSummary>,
}

/// Summary of a single member inside a container call-out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberSummary {
    pub name: String,
    pub kind: SymbolKind,
    pub sloc: usize,
}

/// Histogram of top-level symbol kinds, sorted deterministically.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KindHistogram(pub BTreeMap<SymbolKind, usize>);

/// A rule-based refactor hint derived from structural signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RefactorHint {
    DominatedBySingleSymbol { name: String, share: f32 },
    ManyUnrelatedTopLevel { count: usize },
    DeeplyNested { depth: usize },
    HighCoupling { import_fan_out: usize },
    LowCodeDensity { comment_density: f32 },
}

/// Structured analysis for a single god-file candidate.
#[derive(Debug, Clone, Serialize)]
pub struct GodAnalysis {
    /// Resolvable reference to the file (drives the OSC8 hyperlink).
    #[serde(skip)]
    pub file: FileReference,
    /// Path relative to the scan root, for display.
    pub relative_path: PathBuf,
    pub language: ProgrammingLanguage,

    /// Raw newline count (screening metric).
    pub physical_lines: usize,
    /// Physical − blank − comment_only (authoritative metric).
    pub effective_sloc: usize,
    /// Risk band decided by `effective_sloc`.
    pub risk: RiskBand,

    /// Largest symbol blocks, pre-sorted descending by SLOC.
    pub blocks: Vec<SymbolBlock>,
    /// Count of blocks omitted by the top-N cap.
    pub blocks_truncated: usize,

    // --- Signal: structural shape ---
    pub top_level_symbol_count: usize,
    pub kind_histogram: KindHistogram,

    // --- Signal: complexity ---
    /// Deepest nesting of control-flow constructs (if/loop/match/try), counted
    /// from a single syntax-tree walk. Function and class scopes do not count.
    pub max_nesting_depth: usize,

    // --- Signal: coupling & debt ---
    pub import_fan_out: usize,
    pub todo_fixme_count: usize,
    /// Comment lines divided by physical lines, clamped to 0.0..=1.0.
    pub comment_density: f32,

    // --- Signal: derived guidance ---
    pub refactor_hints: Vec<RefactorHint>,

    /// Diagnostic note explaining degraded analysis (e.g. a candidate that
    /// could not be parsed and was banded by physical line count alone).
    /// `None` for fully-analyzed files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
