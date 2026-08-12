//! Loader for the committed unchained-ai models-catalog artifact.
//!
//! The artifact (`unchained-ai/artifacts/models-catalog.json`) is the
//! entire cross-area boundary between unchained-ai and claudine
//! (design/model-catalog-boundary.md): claudine-gen consumes it by
//! workspace-relative path and joins expected offerings to identity keys
//! by lookup only — the identity grammar/parser stays unchained-ai's.
//! Absence or a schema_version mismatch is a loud generation error; a
//! `generated_at` older than the ContentPolicy max age
//! (`{ generated_at, max_age: 30d, on_stale: warn }`) yields a warning.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::errors::GenError;

/// The artifact schema this consumer understands.
pub const EXPECTED_SCHEMA_VERSION: u64 = 2;

/// ContentPolicy max age before `generated_at` triggers the staleness
/// warning.
pub const MAX_AGE_DAYS: i64 = 30;

/// Artifact location relative to the workspace root.
pub const ARTIFACT_WORKSPACE_PATH: &str = "unchained-ai/artifacts/models-catalog.json";

/// Minimal artifact projection — the join ladder's offerings plus the
/// family index the `family_latest` compilation vendors.
#[derive(Debug, Deserialize)]
struct Artifact {
    schema_version: u64,
    generated_at: String,
    offerings: Vec<Offering>,
    #[serde(default)]
    families: BTreeMap<String, FamilyEntry>,
}

/// One `families{}` entry (keyed `vendor/family` in the artifact).
#[derive(Debug, Deserialize)]
pub struct FamilyEntry {
    /// Member offering wire ids, release-ordered ascending.
    #[serde(default)]
    pub members: Vec<String>,
    /// Identity key naming the newest release (Checkpoint F: a release,
    /// not an offering).
    #[serde(default)]
    pub latest: Option<String>,
    /// Wire ids the vendor re-targets over releases.
    #[serde(default)]
    pub rolling_aliases: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Offering {
    /// Wire id `source/model-id`; the model-id part may itself contain
    /// `/` for aggregators (`openrouter/anthropic/claude-opus-4.8`).
    id: String,
    /// Absent for the artifact's identity-less gap offerings.
    #[serde(default)]
    identity_key: Option<String>,
}

/// Lookup index over the artifact's offering ids.
#[derive(Debug)]
pub struct ArtifactIndex {
    /// Exact wire id → identity key.
    exact: BTreeMap<String, String>,
    /// Bare model-id part (after the FIRST `/` of the wire id) → every
    /// identity key seen under that spelling.
    bare: BTreeMap<String, BTreeSet<String>>,
    /// The artifact's family index, keyed `vendor/family`.
    families: BTreeMap<String, FamilyEntry>,
    /// The artifact's RFC3339 `generated_at` stamp, verbatim.
    generated_at: String,
    /// Present when `generated_at` exceeded [`MAX_AGE_DAYS`] at load time.
    pub staleness_warning: Option<String>,
}

impl ArtifactIndex {
    /// Identity key for an exact `source/model-id` wire id.
    pub fn exact_key(&self, wire_id: &str) -> Option<&str> {
        self.exact.get(wire_id).map(String::as_str)
    }

    /// The family entry for a `vendor/family` key.
    pub fn family(&self, key: &str) -> Option<&FamilyEntry> {
        self.families.get(key)
    }

    /// The artifact's RFC3339 `generated_at` stamp, verbatim.
    pub fn generated_at(&self) -> &str {
        &self.generated_at
    }

    /// Identity key for a bare model id, only when every artifact
    /// offering spelled that way shares exactly ONE identity key —
    /// zero matches and key-ambiguous spellings both return `None`.
    pub fn bare_unique_key(&self, model_id: &str) -> Option<&str> {
        let keys = self.bare.get(model_id)?;
        if keys.len() == 1 {
            keys.iter().next().map(String::as_str)
        } else {
            None
        }
    }
}

/// The artifact path for a claudine area root (the workspace root is the
/// area root's parent).
pub fn artifact_path(area: &Path) -> PathBuf {
    area.parent().unwrap_or(area).join(ARTIFACT_WORKSPACE_PATH)
}

/// Loads and indexes the artifact for a claudine area root.
pub fn load(area: &Path) -> Result<ArtifactIndex, GenError> {
    load_from(&artifact_path(area), Utc::now())
}

/// Path- and clock-parameterized loader (the testable core of [`load`]).
pub fn load_from(path: &Path, now: DateTime<Utc>) -> Result<ArtifactIndex, GenError> {
    if !path.is_file() {
        return Err(GenError::ArtifactMissing {
            path: path.to_path_buf(),
        });
    }
    let text = std::fs::read_to_string(path).map_err(|source| GenError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let artifact: Artifact =
        serde_json::from_str(&text).map_err(|err| GenError::ArtifactInvalid {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
    if artifact.schema_version != EXPECTED_SCHEMA_VERSION {
        return Err(GenError::ArtifactSchemaVersion {
            path: path.to_path_buf(),
            found: artifact.schema_version,
            expected: EXPECTED_SCHEMA_VERSION,
        });
    }
    let generated_at = DateTime::parse_from_rfc3339(&artifact.generated_at).map_err(|err| {
        GenError::ArtifactInvalid {
            path: path.to_path_buf(),
            message: format!(
                "`generated_at` `{}` is not RFC3339: {err}",
                artifact.generated_at
            ),
        }
    })?;
    let age_days = (now - generated_at.with_timezone(&Utc)).num_days();
    let staleness_warning = (age_days > MAX_AGE_DAYS).then(|| {
        format!(
            "models-catalog artifact is stale: generated_at {} is {age_days} days old \
             (max {MAX_AGE_DAYS}) — regenerate with `cargo run -p unchained-ai-gen --bin \
             emit-catalog`; catalog_id joins may name outdated releases",
            artifact.generated_at
        )
    });

    let mut exact = BTreeMap::new();
    let mut bare: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for offering in artifact.offerings {
        let Some(key) = offering.identity_key else {
            // Identity-less gap offerings are unjoinable by definition.
            continue;
        };
        if let Some((_source, model_id)) = offering.id.split_once('/') {
            bare.entry(model_id.to_string())
                .or_default()
                .insert(key.clone());
        }
        exact.insert(offering.id, key);
    }
    Ok(ArtifactIndex {
        exact,
        bare,
        families: artifact.families,
        generated_at: artifact.generated_at,
        staleness_warning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_artifact(dir: &Path, content: &str) -> PathBuf {
        let path = dir.join("models-catalog.json");
        std::fs::write(&path, content).unwrap();
        path
    }

    fn fixture_json(schema_version: u64, generated_at: &str) -> String {
        serde_json::json!({
            "schema_version": schema_version,
            "generated_at": generated_at,
            "offerings": [
                { "id": "moonshotai/kimi-k2.7-code", "identity_key": "moonshotai/kimi-k-code@2.7" },
                { "id": "openrouter/moonshotai/kimi-k2.7-code", "identity_key": "moonshotai/kimi-k-code@2.7" },
                { "id": "anthropic/claude-opus-4-8", "identity_key": "anthropic/claude-opus@4.8" },
                { "id": "vendor-a/shared-name", "identity_key": "vendor-a/shared-name" },
                { "id": "vendor-b/shared-name", "identity_key": "vendor-b/shared-name" },
                { "id": "openrouter/auto" }
            ],
            "families": {
                "anthropic/claude-opus": {
                    "members": ["anthropic/claude-opus-4-8"],
                    "latest": "anthropic/claude-opus@4.8",
                    "rolling_aliases": ["openrouter/anthropic/claude-opus-latest"]
                },
                "vendor-a/shared-name": {
                    "members": ["vendor-a/shared-name"]
                }
            }
        })
        .to_string()
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-06T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn missing_file_is_a_loud_error_naming_the_producer() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_from(&dir.path().join("models-catalog.json"), now()).unwrap_err();
        assert!(matches!(err, GenError::ArtifactMissing { .. }));
        assert!(err.to_string().contains("emit-catalog"), "{err}");
    }

    #[test]
    fn schema_version_one_is_rejected_after_v2_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_artifact(dir.path(), &fixture_json(1, "2026-07-01T00:00:00Z"));
        let err = load_from(&path, now()).unwrap_err();
        match err {
            GenError::ArtifactSchemaVersion {
                found, expected, ..
            } => {
                assert_eq!(found, 1);
                assert_eq!(expected, EXPECTED_SCHEMA_VERSION);
            }
            other => panic!("expected ArtifactSchemaVersion, got: {other}"),
        }
    }

    #[test]
    fn stale_generated_at_warns_but_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_artifact(dir.path(), &fixture_json(2, "2026-05-01T00:00:00Z"));
        let index = load_from(&path, now()).unwrap();
        let warning = index.staleness_warning.expect("66-day-old artifact warns");
        assert!(warning.contains("stale"), "{warning}");
        assert!(warning.contains("2026-05-01T00:00:00Z"), "{warning}");
    }

    #[test]
    fn fresh_generated_at_does_not_warn() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_artifact(dir.path(), &fixture_json(2, "2026-07-01T00:00:00Z"));
        let index = load_from(&path, now()).unwrap();
        assert!(index.staleness_warning.is_none());
    }

    /// The bare index splits on the FIRST slash only: an aggregator wire
    /// id `openrouter/moonshotai/kimi-k2.7-code` contributes the bare
    /// spelling `moonshotai/kimi-k2.7-code`, not `kimi-k2.7-code`.
    #[test]
    fn index_splits_wire_ids_on_first_slash_and_skips_gap_offerings() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_artifact(dir.path(), &fixture_json(2, "2026-07-01T00:00:00Z"));
        let index = load_from(&path, now()).unwrap();
        assert_eq!(
            index.exact_key("moonshotai/kimi-k2.7-code"),
            Some("moonshotai/kimi-k-code@2.7")
        );
        assert_eq!(
            index.bare_unique_key("moonshotai/kimi-k2.7-code"),
            Some("moonshotai/kimi-k-code@2.7"),
            "aggregator bare spelling keeps the vendor prefix"
        );
        // Same spelling under two sources with two different identity
        // keys: ambiguous, no confident join.
        assert_eq!(index.bare_unique_key("shared-name"), None);
        // The gap offering has no identity key and never enters the index.
        assert_eq!(index.exact_key("openrouter/auto"), None);
        assert_eq!(index.bare_unique_key("auto"), None);
    }

    /// `latest` and `rolling_aliases` are optional per family entry;
    /// `generated_at` survives verbatim for the compiled staleness anchor.
    #[test]
    fn family_index_and_generated_at_are_projected() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_artifact(dir.path(), &fixture_json(2, "2026-07-01T00:00:00Z"));
        let index = load_from(&path, now()).unwrap();
        assert_eq!(index.generated_at(), "2026-07-01T00:00:00Z");
        let opus = index.family("anthropic/claude-opus").expect("family present");
        assert_eq!(opus.latest.as_deref(), Some("anthropic/claude-opus@4.8"));
        assert_eq!(opus.rolling_aliases, ["openrouter/anthropic/claude-opus-latest"]);
        let bare = index.family("vendor-a/shared-name").expect("family present");
        assert_eq!(bare.latest, None);
        assert!(bare.rolling_aliases.is_empty());
        assert!(index.family("nobody/nothing").is_none());
    }
}
