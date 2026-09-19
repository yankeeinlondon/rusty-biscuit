//! Reviewed research corrections (`docs/research/platforms/_overrides.yaml`).

use serde::{Deserialize, Serialize};

use super::common::{Date, PlatformId};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Overrides {
    pub schema_version: u32,
    pub overrides: Vec<Override>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<serde_json::Value>,
}

/// A correction applied explicitly after validation. Reports show both the
/// researched and the effective value.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Override {
    pub id: String,
    pub platform_id: PlatformId,
    pub fact: String,
    pub field: String,
    pub effective_value: String,
    /// xxh64 of the targeted fact's canonical JSON when it was reviewed.
    pub target_hash: String,
    /// The schema fingerprint the override was reviewed against.
    pub schema_hash: String,
    pub evidence: Vec<String>,
    pub reason: String,
    pub author: String,
    pub review_by: Date,
}
