//! Who else is using a checkout: one [`query_path_usage`] of it, read into
//! the [`Holder`]s the removal question names ([`holders`]) and the
//! [`CoverageSummary`] that says how much of the host the query could see.
//!
//! Nothing here decides anything. A holder is a warning in the question
//! ([`super::facts`]), and a holder of a file deletion is waiting on is named
//! while it waits ([`super::delete`]).

use std::path::Path;

use chrono::{DateTime, Utc};
use sniff::filesystem::query::{
    Ancestor, CoverageStatus, Evidence, EvidenceKind, FileIdentity, Mechanism, NativeString, Outcome,
    PathUsageError, PathUsageOptions, PathUsageReport, query_path_usage,
};

/// Where usage reports come from. [`SniffUsage`] queries the host; tests
/// inject reports instead, so they never depend on what the host is running.
pub trait UsageSource: Sync {
    /// Queries `path`, walking the tree as it is now.
    fn query(&self, path: &Path) -> Result<PathUsageReport, PathUsageError>;
}

/// [`UsageSource`] through Sniff with its default options (one shared
/// two-second budget).
#[derive(Debug, Clone, Copy, Default)]
pub struct SniffUsage;

impl UsageSource for SniffUsage {
    fn query(&self, path: &Path) -> Result<PathUsageReport, PathUsageError> {
        query_path_usage(path, &PathUsageOptions::default())
    }
}

/// Every process lifetime in `report` with evidence in the checkout, in
/// report order (by PID, then creation token).
pub fn holders(report: &PathUsageReport) -> Vec<Holder> {
    report
        .processes
        .iter()
        .filter(|record| !record.evidence.is_empty())
        .map(|record| Holder {
            pid: record.pid,
            creation_token: record.creation_token,
            identity_uncertain: record.identity_uncertain,
            name: record.name.clone(),
            executable: record.executable.clone(),
            start_time: record.start_time,
            app_name: record.app_name.clone(),
            ancestors: record.ancestors.clone(),
            evidence: record
                .evidence
                .iter()
                .map(|item| HolderEvidence {
                    kind: item.kind,
                    location: location(item),
                    writing: item.access.is_some_and(|access| access.write),
                })
                .collect(),
        })
        .collect()
}

/// One process lifetime using the checkout.
#[derive(Debug, Clone, PartialEq)]
pub struct Holder {
    pub pid: u32,
    pub creation_token: Option<u64>,
    /// Sniff could not tie this record to one process lifetime. Its name, if
    /// any, came with the evidence itself; none is looked up afterward, so a
    /// reused PID never lends its name.
    pub identity_uncertain: bool,
    pub name: Option<NativeString>,
    pub executable: Option<NativeString>,
    /// Presentation only, never an identity key.
    pub start_time: Option<DateTime<Utc>>,
    /// Sniff's recognizable name: the enclosing app on macOS, the program's
    /// description on Windows.
    pub app_name: Option<NativeString>,
    /// Verified parents, nearest first, bounded by Sniff.
    pub ancestors: Vec<Ancestor>,
    pub evidence: Vec<HolderEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HolderEvidence {
    pub kind: EvidenceKind,
    pub location: EvidenceLocation,
    /// The handle was observed open for writing. `false` when Sniff could
    /// not tell, so it never proves the handle is read-only.
    pub writing: bool,
}

/// Which object in the checkout the evidence is about.
#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceLocation {
    /// Exactly one path in the checkout names it.
    Path(NativeString),
    /// Several paths in the checkout name it (hard links); none is the
    /// preferred one.
    Aliases(Vec<NativeString>),
    /// No path in the checkout names it any longer: it is known only by its
    /// identity, and by the spelling the process used when Sniff had one.
    Unnamed { identity: Option<FileIdentity>, observed: Option<NativeString> },
}

/// How much of the host the query could see.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverageSummary {
    pub outcome: Outcome,
    pub budget_exhausted: bool,
    /// Every mechanism and prerequisite that did not complete, in report
    /// order.
    pub gaps: Vec<CoverageGap>,
    /// Evidence records Sniff counted but did not retain.
    pub evidence_omitted: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageGap {
    pub mechanism: Mechanism,
    pub status: CoverageStatus,
    pub reason: Option<String>,
}

impl CoverageSummary {
    pub fn of(report: &PathUsageReport) -> Self {
        Self {
            outcome: report.outcome,
            budget_exhausted: report.budget_exhausted,
            gaps: report
                .coverage
                .iter()
                .filter(|coverage| coverage.status != CoverageStatus::Complete)
                .map(|coverage| CoverageGap {
                    mechanism: coverage.mechanism,
                    status: coverage.status,
                    reason: coverage.reason.clone(),
                })
                .collect(),
            evidence_omitted: report.coverage.iter().filter_map(|c| c.truncation.as_ref()).map(|t| t.omitted).sum(),
        }
    }

    /// Every mechanism completed and nothing was omitted.
    pub fn is_complete(&self) -> bool {
        self.outcome == Outcome::Usable && !self.budget_exhausted && self.gaps.is_empty() && self.evidence_omitted == 0
    }
}

fn location(evidence: &Evidence) -> EvidenceLocation {
    match evidence.matched_paths.as_slice() {
        [] => EvidenceLocation::Unnamed { identity: evidence.identity, observed: evidence.observed_path.clone() },
        [path] => EvidenceLocation::Path(path.clone()),
        paths => EvidenceLocation::Aliases(paths.to_vec()),
    }
}

#[cfg(test)]
mod tests;
