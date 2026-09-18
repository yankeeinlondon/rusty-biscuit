//! Reviewed implementation mappings (`docs/research/implementation/mappings.yaml`).
//!
//! Mappings describe Messenger's adapter code, not the platforms. They
//! support reporting only and never change `CapabilitySet` or delivery.

use serde::{Deserialize, Serialize};

use super::common::{AdapterId, Category, Date, PlatformId, string_enum};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mappings {
    pub schema_version: u32,
    /// Provenance only; never a reuse key.
    pub inspected_revision: String,
    pub fingerprint_algorithm: String,
    pub assessments: Vec<Assessment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<serde_json::Value>,
}

string_enum! {
    pub enum FingerprintKind {
        CodeFile => "code_file",
        TestFile => "test_file",
        ResearchFact => "research_fact",
    }
}

/// One relevant input. File inputs are repository-relative paths; research
/// facts are `{platform_id}#{fact_id}`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fingerprint {
    pub input: String,
    pub kind: FingerprintKind,
    pub digest: String,
}

string_enum! {
    pub enum ReviewStatus {
        Proposed => "proposed",
        Accepted => "accepted",
        Rejected => "rejected",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssessmentReview {
    pub status: ReviewStatus,
    pub proposed_by: String,
    pub reviewed_by: Option<String>,
    pub reviewed_on: Option<Date>,
}

string_enum! {
    pub enum ImplementationStatus {
        Implemented => "implemented",
        Partial => "partial",
        Missing => "missing",
        Unassessed => "unassessed",
    }
}

string_enum! {
    pub enum StaleReason {
        FingerprintChanged => "fingerprint_changed",
        NeverReviewed => "never_reviewed",
        FactRemoved => "fact_removed",
    }
}

/// One assessment, scoped to an adapter and category.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub id: String,
    pub adapter: AdapterId,
    pub category: Category,
    pub platform_id: PlatformId,
    pub facts: Vec<String>,
    pub status: ImplementationStatus,
    pub stale_reason: Option<StaleReason>,
    pub summary: String,
    pub code_refs: Vec<String>,
    pub test_refs: Vec<String>,
    pub assessed_revision: String,
    pub fingerprints: Vec<Fingerprint>,
    pub review: AssessmentReview,
    pub requires_messenger_update: bool,
    pub follow_up: Option<String>,
}
