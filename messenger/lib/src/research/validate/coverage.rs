//! Coverage rules (SR-COVERAGE, SR-GAP, SR-CHANGE, SR-OVERRIDE) and the
//! per-interface completeness summary.
//!
//! Coverage is proved by the matrix, never by the presence or absence of
//! records: an empty category array never means "no constraints", and an
//! interface without a matrix is reported as missing, not as unrestricted.

use std::collections::BTreeSet;

use serde::Serialize;

use super::{Context, Index, Scope, ValidatedDocument, category_record_count, knowledge_blocks, push};
use crate::research::canonical::record_fingerprint;
use crate::research::diagnostics::{Findings, Rule};
use crate::research::model::{
    Category, ChangeKind, CoverageStatus, Date, GapKind, GapStatus, Overrides, PlatformDocument, PlatformId, State,
};

pub(super) fn check(d: &PlatformDocument, index: &Index<'_>, context: &Context<'_>, findings: &mut Findings) {
    matrices(d, context.scope, findings);
    gaps(d, index, context.scope, findings);
    changes(d, index, findings);
}

// ---- SR-COVERAGE ------------------------------------------------------------

fn matrices(d: &PlatformDocument, scope: Scope, findings: &mut Findings) {
    for interface in &d.interfaces {
        if !d.coverage.iter().any(|c| c.interface == interface.interface_id) {
            push(
                findings,
                Rule::Coverage,
                "/coverage",
                &interface.interface_id,
                "interface has no coverage matrix; an uncovered interface is never unrestricted",
            );
        }
    }
    for (c, coverage) in d.coverage.iter().enumerate() {
        for (category, cell) in coverage.categories.cells() {
            let pointer = format!("/coverage/{c}/categories/{category}");
            match cell.status {
                CoverageStatus::Researched => {
                    if category_record_count(d, &coverage.interface, category) == 0 {
                        push(
                            findings,
                            Rule::Coverage,
                            pointer.clone(),
                            &coverage.interface,
                            format!("{category} is researched but has no record; an empty list is not unlimited"),
                        );
                    }
                    // Accepted research answers every image role and the
                    // author-geolocation question for a researched interface.
                    if scope == Scope::Accepted {
                        let incomplete = match category {
                            Category::Images => !d.role_coverage.iter().any(|r| r.interface == coverage.interface),
                            Category::Location => !d.author_geolocation.iter().any(|g| g.interface == coverage.interface),
                            _ => false,
                        };
                        if incomplete {
                            push(
                                findings,
                                Rule::Coverage,
                                pointer.clone(),
                                &coverage.interface,
                                format!("researched {category} lacks its complete per-interface matrix"),
                            );
                        }
                    }
                }
                CoverageStatus::NotApplicable if cell.explanation.is_none() => push(
                    findings,
                    Rule::Coverage,
                    format!("{pointer}/explanation"),
                    &coverage.interface,
                    format!("not-applicable {category} explains why"),
                ),
                CoverageStatus::Gap if cell.gap.is_none() => push(
                    findings,
                    Rule::Coverage,
                    format!("{pointer}/gap"),
                    &coverage.interface,
                    format!("{category} gap names its gap record"),
                ),
                _ => {}
            }
        }
    }
}

// ---- SR-GAP -----------------------------------------------------------------

fn gaps(d: &PlatformDocument, index: &Index<'_>, scope: Scope, findings: &mut Findings) {
    for (g, gap) in d.gaps.iter().enumerate() {
        let base = format!("/gaps/{g}");
        if gap.status == GapStatus::Investigated {
            let missing: Vec<&str> = [
                ("searches", gap.searches.is_empty()),
                ("inspected_sources", gap.inspected_sources.is_empty()),
                ("unresolved_reason", gap.unresolved_reason.is_none()),
                ("blocked_decision", gap.blocked_decision.is_none()),
            ]
            .into_iter()
            .filter_map(|(field, absent)| absent.then_some(field))
            .collect();
            if !missing.is_empty() {
                push(
                    findings,
                    Rule::Gap,
                    base.clone(),
                    &gap.id,
                    format!("an investigated gap records {}", missing.join(", ")),
                );
            }
        }
        for fact in &gap.facts {
            if !index.facts.contains(fact.as_str()) {
                push(findings, Rule::Gap, format!("{base}/facts"), &gap.id, format!("gap names fact {fact}, which does not exist"));
            }
        }
    }
    if scope != Scope::Accepted {
        return;
    }
    let mut required: Vec<(String, &str, &str)> = Vec::new();
    for (c, coverage) in d.coverage.iter().enumerate() {
        for (category, cell) in coverage.categories.cells() {
            if let Some(gap) = &cell.gap {
                required.push((format!("/coverage/{c}/categories/{category}/gap"), &coverage.interface, gap));
            }
        }
    }
    for (c, coverage) in d.role_coverage.iter().enumerate() {
        for (role, cell) in coverage.roles.iter() {
            if let Some(gap) = &cell.gap {
                required.push((format!("/role_coverage/{c}/roles/{role}/gap"), &coverage.interface, gap));
            }
        }
    }
    for (pointer, id, knowledge) in knowledge_blocks(d) {
        if matches!(knowledge.state, State::Unknown | State::Conflicting)
            && let Some(gap) = &knowledge.gap
        {
            required.push((format!("{pointer}/gap"), id, gap));
        }
    }
    for (pointer, subject, gap) in required {
        if index.gaps.get(gap).is_some_and(|gap| gap.status != GapStatus::Investigated) {
            push(
                findings,
                Rule::Gap,
                pointer,
                subject,
                format!("accepted coverage rests on open gap {gap}; a placeholder unknown is not an investigation"),
            );
        }
    }
}

// ---- SR-CHANGE --------------------------------------------------------------

fn changes(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    if d.requires_messenger_update {
        if d.reason.is_none() {
            push(findings, Rule::Change, "/reason", d.platform_id.as_str(), "requires_messenger_update: true states its reason");
        }
        if !d.gaps.iter().any(|gap| gap.kind == GapKind::RequiresMessengerUpdate) {
            push(
                findings,
                Rule::Change,
                "/gaps",
                d.platform_id.as_str(),
                "requires_messenger_update: true needs a requires_messenger_update gap",
            );
        }
    }
    for (c, change) in d.changes.iter().enumerate() {
        for fact in &change.facts {
            // Changes classify facts and gaps alike.
            let exists = index.facts.contains(fact.as_str()) || index.gaps.contains_key(fact.as_str());
            if change.kind == ChangeKind::Removed && exists {
                push(
                    findings,
                    Rule::Change,
                    format!("/changes/{c}/facts"),
                    &change.id,
                    format!("removed fact {fact} is still present; removed IDs are never reused"),
                );
            } else if change.kind != ChangeKind::Removed && !exists {
                push(
                    findings,
                    Rule::Change,
                    format!("/changes/{c}/facts"),
                    &change.id,
                    format!("change names fact {fact}, which does not exist"),
                );
            }
        }
    }
}

// ---- SR-OVERRIDE ------------------------------------------------------------

pub(super) fn check_overrides(
    overrides: &Overrides,
    documents: &[&ValidatedDocument],
    schema_fingerprint: &str,
    today: &Date,
    findings: &mut Findings,
) {
    let mut ids = BTreeSet::new();
    for (o, entry) in overrides.overrides.iter().enumerate() {
        let base = format!("/overrides/{o}");
        if !ids.insert(entry.id.as_str()) {
            push(findings, Rule::Unique, format!("{base}/id"), &entry.id, "duplicate override ID");
        }
        if entry.review_by < *today {
            push(
                findings,
                Rule::Override,
                format!("{base}/review_by"),
                &entry.id,
                format!("override expired on {}", entry.review_by),
            );
        }
        if entry.schema_hash != schema_fingerprint {
            push(
                findings,
                Rule::Override,
                format!("{base}/schema_hash"),
                &entry.id,
                "override was reviewed against a different schema",
            );
        }
        let document = documents.iter().find(|d| d.platform_id() == entry.platform_id);
        let Some(record) = document.and_then(|d| d.record(&entry.fact)) else {
            push(
                findings,
                Rule::Override,
                format!("{base}/fact"),
                &entry.id,
                format!("override targets {}#{}, which does not exist", entry.platform_id, entry.fact),
            );
            continue;
        };
        if entry.target_hash != record_fingerprint(record) {
            push(
                findings,
                Rule::Override,
                format!("{base}/target_hash"),
                &entry.id,
                format!("{} changed since the override was reviewed", entry.fact),
            );
        }
        if let Some(document) = document {
            let sources: BTreeSet<&str> = document.document().sources.iter().map(|s| s.id.as_str()).collect();
            for evidence in &entry.evidence {
                if !sources.contains(evidence.as_str()) {
                    push(
                        findings,
                        Rule::Override,
                        format!("{base}/evidence"),
                        &entry.id,
                        format!("evidence {evidence} is not a source of {}", entry.platform_id),
                    );
                }
            }
        }
    }
}

// ---- completeness summary ----------------------------------------------------

/// Coverage of one category for one interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageState {
    Researched,
    NotApplicable,
    InvestigatedGap,
    OpenGap,
    /// No matrix covers the interface: never unrestricted support.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CategoryCoverage {
    pub category: Category,
    pub state: CoverageState,
    /// Records of this category scoped to the interface.
    pub records: usize,
    pub gap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterfaceCoverageSummary {
    pub interface: String,
    /// All sixteen categories, in category order.
    pub categories: Vec<CategoryCoverage>,
}

impl InterfaceCoverageSummary {
    /// Researched or explicitly not applicable in every category.
    pub fn is_complete(&self) -> bool {
        self.categories
            .iter()
            .all(|c| matches!(c.state, CoverageState::Researched | CoverageState::NotApplicable | CoverageState::InvestigatedGap))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageSummary {
    pub platform_id: PlatformId,
    /// Interfaces in document order.
    pub interfaces: Vec<InterfaceCoverageSummary>,
}

pub(super) fn summary(d: &PlatformDocument) -> CoverageSummary {
    let index = Index::new(d);
    let interfaces = d
        .interfaces
        .iter()
        .map(|interface| {
            let matrix = d.coverage.iter().find(|c| c.interface == interface.interface_id);
            let categories = Category::ALL
                .iter()
                .map(|category| {
                    let records = category_record_count(d, &interface.interface_id, *category);
                    let Some(matrix) = matrix else {
                        return CategoryCoverage { category: *category, state: CoverageState::Missing, records, gap: None };
                    };
                    let cell = matrix.categories.cell(*category);
                    let state = match cell.status {
                        CoverageStatus::Researched => CoverageState::Researched,
                        CoverageStatus::NotApplicable => CoverageState::NotApplicable,
                        CoverageStatus::Gap => match cell.gap.as_deref().and_then(|gap| index.gaps.get(gap)) {
                            Some(gap) if gap.status == GapStatus::Investigated => CoverageState::InvestigatedGap,
                            _ => CoverageState::OpenGap,
                        },
                    };
                    CategoryCoverage { category: *category, state, records, gap: cell.gap.clone() }
                })
                .collect();
            InterfaceCoverageSummary { interface: interface.interface_id.clone(), categories }
        })
        .collect();
    CoverageSummary { platform_id: d.platform_id, interfaces }
}
