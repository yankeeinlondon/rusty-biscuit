use std::collections::HashMap;

use crate::queries::provenance::{QueryProvenance, TranslationStatus};
use crate::queries::{QueryKind, query_provenance};
use crate::shared::ProgrammingLanguage;

/// A report of differences between a vendored query and its upstream source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftReport {
    /// Language of the query.
    pub language: ProgrammingLanguage,
    /// Query kind.
    pub kind: QueryKind,
    /// The provenance record for the vendored query.
    pub provenance: QueryProvenance,
    /// Detected drift items.
    pub items: Vec<DriftItem>,
    /// Whether the drift is considered actionable.
    pub actionable: bool,
}

/// A single drift observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftItem {
    /// Upstream revision is newer than what we have recorded.
    NewerUpstreamRevision {
        recorded: String,
        upstream: String,
    },
    /// The vendored file differs from upstream (content hash mismatch).
    ContentMismatch,
    /// The upstream path no longer exists.
    UpstreamPathMissing,
    /// A capture name has diverged from upstream conventions.
    CaptureDivergence {
        capture: String,
        expected: String,
        actual: String,
    },
    /// A predicate is used that is not in the upstream query.
    PredicateAdded {
        predicate: String,
    },
    /// A predicate from upstream is missing in the vendored query.
    PredicateRemoved {
        predicate: String,
    },
}

impl std::fmt::Display for DriftItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewerUpstreamRevision { recorded, upstream } => {
                write!(
                    f,
                    "upstream revision changed from {recorded} to {upstream}"
                )
            }
            Self::ContentMismatch => {
                write!(f, "vendored content does not match upstream")
            }
            Self::UpstreamPathMissing => {
                write!(f, "upstream path no longer exists")
            }
            Self::CaptureDivergence {
                capture,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "capture '{capture}' diverged: expected '{expected}', found '{actual}'"
                )
            }
            Self::PredicateAdded { predicate } => {
                write!(f, "predicate '{predicate}' added locally")
            }
            Self::PredicateRemoved { predicate } => {
                write!(f, "predicate '{predicate}' removed from upstream")
            }
        }
    }
}

/// Checks a single query for upstream drift.
///
/// In a full implementation this would compare file hashes against a cloned
/// upstream repository. The static version records what would be checked and
/// returns drift items based on metadata mismatches.
pub fn check_query_drift(
    language: ProgrammingLanguage,
    kind: QueryKind,
    current_upstream_revision: Option<&str>,
) -> Option<DriftReport> {
    let provenance = query_provenance(language, kind)?;

    // Only vendor queries can drift.
    if provenance.translation_status == TranslationStatus::Original {
        return None;
    }

    let mut items = Vec::new();

    // Check revision drift if an upstream revision was provided.
    if let Some(upstream) = current_upstream_revision
        && upstream != provenance.upstream_revision
    {
        items.push(DriftItem::NewerUpstreamRevision {
            recorded: provenance.upstream_revision.clone(),
            upstream: upstream.to_string(),
        });
    }

    let actionable = !items.is_empty();

    Some(DriftReport {
        language,
        kind,
        provenance,
        items,
        actionable,
    })
}

/// Checks all vendored queries for drift.
pub fn check_all_drift(
    current_upstream_revision: Option<&str>,
) -> Vec<DriftReport> {
    let mut reports = Vec::new();

    for language in ProgrammingLanguage::all() {
        for kind in [QueryKind::Locals] {
            if let Some(report) = check_query_drift(language, kind, current_upstream_revision) {
                reports.push(report);
            }
        }
    }

    reports
}

/// Summarizes drift reports into a human-readable string.
pub fn summarize_drift(reports: &[DriftReport]) -> String {
    if reports.is_empty() {
        return "No drift detected.".to_string();
    }

    let actionable: Vec<_> = reports.iter().filter(|r| r.actionable).collect();

    if actionable.is_empty() {
        return "Drift checked: all queries up to date.".to_string();
    }

    let mut summary = format!(
        "Upstream drift detected in {} query(s):\n",
        actionable.len()
    );

    for report in actionable {
        summary.push_str(&format!(
            "  {} {} ({}):\n",
            report.language,
            report.kind,
            report.provenance.upstream_path
        ));
        for item in &report.items {
            summary.push_str(&format!("    - {item}\n"));
        }
    }

    summary
}

/// Returns a map of expected upstream revision per source project.
pub fn expected_upstream_revisions() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert(
        "nvim-treesitter".to_string(),
        "0.10.4".to_string(),
    );
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_drift_when_revision_matches() {
        let report = check_query_drift(
            ProgrammingLanguage::Rust,
            QueryKind::Locals,
            Some("0.10.4"),
        );
        // When revision matches, there should be no actionable drift
        assert!(
            report.is_some(),
            "report should exist for a vendored query"
        );
        assert!(
            !report.unwrap().actionable,
            "should not be actionable when revision matches"
        );
    }

    #[test]
    fn drift_when_revision_differs() {
        let report = check_query_drift(
            ProgrammingLanguage::Rust,
            QueryKind::Locals,
            Some("0.11.0"),
        );
        assert!(report.is_some());
        let report = report.unwrap();
        assert!(report.actionable);
        assert!(report
            .items
            .iter()
            .any(|i| matches!(i, DriftItem::NewerUpstreamRevision { .. })));
    }

    #[test]
    fn original_queries_have_no_drift() {
        // Lint queries are original, not vendored
        let report = check_query_drift(
            ProgrammingLanguage::Rust,
            QueryKind::Lint,
            Some("0.11.0"),
        );
        assert!(report.is_none());
    }

    #[test]
    fn all_drift_includes_locals() {
        let reports = check_all_drift(Some("0.10.4"));
        // With matching revision, none should be actionable
        assert!(!reports.iter().any(|r| r.actionable));
    }

    #[test]
    fn summarize_empty_reports() {
        let summary = summarize_drift(&[]);
        assert_eq!(summary, "No drift detected.");
    }

    #[test]
    fn summarize_no_actionable() {
        let reports = check_all_drift(Some("0.10.4"));
        let summary = summarize_drift(&reports);
        assert_eq!(summary, "Drift checked: all queries up to date.");
    }

    #[test]
    fn summarize_with_actionable() {
        let reports = check_all_drift(Some("0.11.0"));
        let summary = summarize_drift(&reports);
        assert!(summary.contains("Upstream drift detected"));
    }

    #[test]
    fn expected_revisions_include_nvim() {
        let revisions = expected_upstream_revisions();
        assert!(revisions.contains_key("nvim-treesitter"));
    }

    #[test]
    fn drift_item_display() {
        let item = DriftItem::NewerUpstreamRevision {
            recorded: "0.10.4".to_string(),
            upstream: "0.11.0".to_string(),
        };
        assert_eq!(
            item.to_string(),
            "upstream revision changed from 0.10.4 to 0.11.0"
        );
    }
}
