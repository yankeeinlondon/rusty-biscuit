//! Source edit-set validation and application.
//!
//! One shared utility applies candidate repairs to an authored source:
//! spans are validated against UTF-8 boundaries and source extent, rejected
//! on overlap, and applied from the end of the source toward the beginning
//! so earlier spans stay valid. Every candidate is accounted for in an
//! [`EditAudit`] — applied or rejected with a reason — in stable source
//! order.

use serde::{Deserialize, Serialize};

use super::diagnostic::YamlRepair;

/// Why a candidate edit was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditRejection {
    /// Span is inverted or extends past the end of the source.
    OutOfRange,
    /// Span boundary does not fall on a UTF-8 character boundary.
    NonUtf8Boundary,
    /// Span intersects another accepted candidate's span.
    Overlap,
}

/// A candidate edit that was not applied, with the reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedEdit {
    /// The rejected candidate.
    pub repair: YamlRepair,
    /// Why it was rejected.
    pub reason: EditRejection,
}

/// Audit record of an edit-set application.
///
/// Both lists are in stable source order (ascending by span).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditAudit {
    /// Candidates that were applied.
    pub applied: Vec<YamlRepair>,
    /// Candidates that were rejected, with reasons.
    pub rejected: Vec<RejectedEdit>,
}

/// Result of applying an edit set to a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditSetOutcome {
    /// The patched source. Byte-identical to the input when no candidate
    /// applied; every byte outside an applied span is preserved.
    pub source: String,
    /// Which candidates were applied and which were rejected (and why).
    pub audit: EditAudit,
}

impl EditSetOutcome {
    /// Returns `true` when at least one candidate was applied.
    #[must_use]
    pub fn changed(&self) -> bool {
        !self.audit.applied.is_empty()
    }
}

/// Validates and applies a set of candidate repairs to `source`.
///
/// A candidate is rejected when its span is inverted or out of range, when a
/// span boundary splits a UTF-8 character, or when it overlaps an already
/// accepted candidate (the earlier candidate in argument order wins).
/// Accepted candidates apply from the end of the source toward the
/// beginning; two insertions at the same offset both apply, with the later
/// candidate's text landing first.
#[must_use]
pub fn apply_edit_set(source: &str, repairs: &[YamlRepair]) -> EditSetOutcome {
    let mut accepted: Vec<&YamlRepair> = Vec::with_capacity(repairs.len());
    let mut rejected: Vec<RejectedEdit> = Vec::new();

    'candidates: for repair in repairs {
        let span = &repair.span;
        if span.start > span.end || span.end > source.len() {
            rejected.push(RejectedEdit {
                repair: repair.clone(),
                reason: EditRejection::OutOfRange,
            });
            continue;
        }
        if !source.is_char_boundary(span.start) || !source.is_char_boundary(span.end) {
            rejected.push(RejectedEdit {
                repair: repair.clone(),
                reason: EditRejection::NonUtf8Boundary,
            });
            continue;
        }
        for other in &accepted {
            if span.start < other.span.end && other.span.start < span.end {
                rejected.push(RejectedEdit {
                    repair: repair.clone(),
                    reason: EditRejection::Overlap,
                });
                continue 'candidates;
            }
        }
        accepted.push(repair);
    }

    // Apply end-of-source toward beginning so earlier spans stay valid. The
    // sort is stable, so same-offset insertions keep argument order here and
    // land in reverse (later candidate's text first).
    let mut ordered = accepted.clone();
    ordered.sort_by_key(|repair| std::cmp::Reverse((repair.span.start, repair.span.end)));
    let mut patched = source.to_string();
    for repair in ordered {
        patched.replace_range(repair.span.clone(), &repair.replacement);
    }

    let mut applied: Vec<YamlRepair> = accepted.into_iter().cloned().collect();
    applied.sort_by_key(|repair| (repair.span.start, repair.span.end));
    rejected.sort_by_key(|edit| (edit.repair.span.start, edit.repair.span.end));

    EditSetOutcome {
        source: patched,
        audit: EditAudit { applied, rejected },
    }
}
