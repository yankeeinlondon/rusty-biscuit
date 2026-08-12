//! Model-catalog artifact: schema types, builder, and emission helpers.
//!
//! The catalog JSON emitted from here (`unchained-ai/artifacts/
//! models-catalog.json`) is the entire cross-area boundary to claudine —
//! claudine consumes the committed artifact by path and never links this
//! crate. Design: `claudine/features/2026-07-02-provider-metadata/design/
//! model-catalog-boundary.md`.
//!
//! Every collection in the artifact is deterministically ordered (`BTreeMap`
//! keys, sorted `Vec`s) so offline re-emission is byte-stable; the drift test
//! in `tests/catalog_drift.rs` relies on this.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use unchained_ai::models::identity::ModelIdentity;
use unchained_ai::models::model_metadata::ProviderModelMetadata;
use unchained_ai::rigging::providers::models::ProviderModel;

/// Version of the artifact shape. v2 adds serialized metadata `release_date`.
/// Breaking changes to the JSON layout — and curation changes that would move
/// existing identity keys — bump this.
pub const SCHEMA_VERSION: u32 = 2;

/// Serving tags that change what the offering *is*, not how it is delivered.
///
/// Ruling (spike rough-edge #1, promoted here): delivery-tier tags (`free`,
/// `extended`, `nitro`, `floor`, `exacto`) name the same model at a different
/// tier and stay OUT of the identity key; `thinking` switches the offering's
/// reasoning behavior and joins it.
pub const IDENTITY_BEARING_SERVING_TAGS: &[&str] = &["thinking"];

/// The committed model-catalog artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelsCatalog {
    /// See [`SCHEMA_VERSION`]. Consumers must treat a mismatch as a loud
    /// generation error, not a warning.
    pub schema_version: u32,
    /// RFC 3339 generation time of the underlying catalog *data* (see
    /// [`catalog_generated_at`]), not of the artifact file. Consumers apply
    /// their staleness policy against this value.
    pub generated_at: String,
    /// All catalog offerings, sorted by wire id.
    pub offerings: Vec<Offering>,
    /// Family index keyed by `vendor/family`.
    pub families: BTreeMap<String, FamilyEntry>,
    /// Identity key → wire ids, retained only when the group spans more than
    /// one distinct source (the cross-source duplicate-offering groups).
    pub duplicate_groups: BTreeMap<String, Vec<String>>,
    /// Wire ids that parsed to no family — identity-less offerings, carried
    /// through rather than failed on.
    pub gaps: Vec<String>,
}

/// One catalog offering: a wire id plus its parsed identity and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Offering {
    /// Wire id (`provider-slug/model-id`).
    pub id: String,
    /// Offering source: the first wire-id path segment (direct provider or
    /// aggregator).
    pub source: String,
    /// Canonicalized model vendor; equals `source` for direct-provider ids.
    pub vendor: String,
    /// Parsed identity components.
    pub identity: Identity,
    /// Normalized identity key (see [`identity_key`]); `None` for
    /// identity-less offerings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<OfferingMetadata>,
}

/// Artifact mirror of the parsed [`ModelIdentity`] fields (source and vendor
/// live on [`Offering`]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Identity {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub family: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version: Vec<u32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version_raw: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date_pin: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub size: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serving: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub rolling: bool,
}

/// Artifact mirror of [`ProviderModelMetadata`]; owned here so the lib types
/// stay serde-free.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct OfferingMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modalities: Option<OfferingModalities>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing: Option<OfferingPricing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported_parameters: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_parameters: Option<OfferingDefaultParameters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_cutoff: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
}

/// Input/output modalities as lowercase strings (`text`, `image`, `audio`,
/// `video`, `embeddings`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct OfferingModalities {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output: Vec<String>,
}

/// Per-token and per-request pricing in USD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct OfferingPricing {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_per_token: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_per_token: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search_per_request: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_cache_read_per_token: Option<f64>,
}

/// Provider-recommended default generation parameters.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct OfferingDefaultParameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
}

/// One family's index entry: release-ordered members plus rolling aliases.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct FamilyEntry {
    /// Member wire ids, ascending release order (rolling aliases excluded —
    /// they name no release).
    pub members: Vec<String>,
    /// Identity key of the newest non-rolling release. Names the release, not
    /// an offering — consumers pick a concrete offering from the duplicate
    /// group by their own policy (Checkpoint F ruling: no canonical offering
    /// is elected).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    /// Rolling-alias wire ids (`-latest` forms) that re-point over time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolling_aliases: Vec<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Errors from deriving the catalog data's generation time.
#[derive(Debug, Error)]
pub enum CatalogError {
    /// Reading the generated provider-model sources failed.
    #[error("reading generated provider models under {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// A `//! Generated:` header carried an unparseable timestamp.
    #[error("invalid `//! Generated:` timestamp {value:?} in {path}: {source}")]
    Timestamp {
        path: String,
        value: String,
        #[source]
        source: chrono::ParseError,
    },
    /// No `//! Generated:` header was found in any file.
    #[error("no `//! Generated:` headers found under {path}")]
    NoHeaders { path: String },
}

/// Derives the normalized identity key for a parsed wire id.
///
/// Grammar:
///
/// ```text
/// key = vendor "/" family
///       [ "@" (version_raw | date_pin) ]    version wins when both exist
///       ( "+" variant )*                    sorted
///       ( "+" size )*                       sorted
///       ( ":" tag )*                        identity-bearing serving tags only
/// ```
///
/// Delivery-tier serving tags are excluded per
/// [`IDENTITY_BEARING_SERVING_TAGS`]: `claude-x:free` and `claude-x` share a
/// key, while `claude-x:thinking` gets its own.
///
/// ## Returns
///
/// `None` when the family is empty (identity-less offerings such as
/// meta-routers); those ids land in the artifact's gap list.
pub fn identity_key(id: &ModelIdentity) -> Option<String> {
    if id.family.is_empty() {
        return None;
    }
    let mut key = format!("{}/{}", id.vendor, id.family);
    if !id.version_raw.is_empty() {
        key.push('@');
        key.push_str(&id.version_raw);
    } else if !id.date_pin.is_empty() {
        key.push('@');
        key.push_str(&id.date_pin);
    }
    let mut variants = id.variants.clone();
    variants.sort();
    for variant in &variants {
        key.push('+');
        key.push_str(variant);
    }
    let mut sizes = id.size.clone();
    sizes.sort();
    for size in &sizes {
        key.push('+');
        key.push_str(size);
    }
    let mut tags: Vec<&str> = id
        .serving
        .iter()
        .map(String::as_str)
        .filter(|tag| IDENTITY_BEARING_SERVING_TAGS.contains(tag))
        .collect();
    tags.sort_unstable();
    for tag in tags {
        key.push(':');
        key.push_str(tag);
    }
    Some(key)
}

/// Builds the catalog artifact from the generated provider enums.
///
/// Fully offline: enumerates `ProviderModel::all_wire_ids()` and the
/// compiled-in metadata table — no network, no API keys.
pub fn build_catalog(generated_at: String) -> ModelsCatalog {
    let offerings = ProviderModel::all_wire_ids()
        .into_iter()
        .map(|wire_id| {
            let metadata = ProviderModel::parse_wire_id(wire_id)
                .ok()
                .and_then(|model| model.metadata())
                .map(metadata_from);
            offering_for(wire_id, metadata)
        })
        .collect();
    build_from_offerings(generated_at, offerings)
}

/// Derives `generated_at` from the committed generated provider files.
///
/// The artifact's staleness contract tracks when the catalog *data* was
/// generated (the last `gen-models` run against live APIs), not when the
/// artifact file was emitted — so re-emitting offline must not refresh the
/// timestamp. Reading the max `//! Generated:` header from the committed
/// files under `lib/src/rigging/providers/models/` keeps offline re-emission
/// deterministic for the drift test.
pub fn catalog_generated_at() -> Result<String, CatalogError> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("lib")
        .join("src")
        .join("rigging")
        .join("providers")
        .join("models");
    generated_at_from_dir(&dir)
}

fn generated_at_from_dir(dir: &Path) -> Result<String, CatalogError> {
    let io_err = |source: std::io::Error| CatalogError::Io {
        path: dir.display().to_string(),
        source,
    };
    let mut newest: Option<DateTime<FixedOffset>> = None;
    for entry in std::fs::read_dir(dir).map_err(io_err)? {
        let path = entry.map_err(io_err)?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let contents = std::fs::read_to_string(&path).map_err(io_err)?;
        // Not every file carries the header (`ollama.rs` is hand-maintained,
        // `mod.rs` is not generated); absence is fine as long as one file has
        // it.
        let Some(raw) = contents
            .lines()
            .find_map(|line| line.strip_prefix("//! Generated: "))
        else {
            continue;
        };
        let timestamp = DateTime::parse_from_rfc3339(raw.trim()).map_err(|source| {
            CatalogError::Timestamp {
                path: path.display().to_string(),
                value: raw.trim().to_string(),
                source,
            }
        })?;
        newest = Some(newest.map_or(timestamp, |current| current.max(timestamp)));
    }
    let newest = newest.ok_or_else(|| CatalogError::NoHeaders {
        path: dir.display().to_string(),
    })?;
    Ok(newest.to_rfc3339())
}

/// Serializes the catalog to its canonical committed form: pretty JSON with a
/// trailing newline.
pub fn to_canonical_json(catalog: &ModelsCatalog) -> String {
    let mut json = serde_json::to_string_pretty(catalog)
        .expect("ModelsCatalog serialization cannot fail: string keys, no fallible types");
    json.push('\n');
    json
}

/// Serializes the artifact's JSON Schema in the same canonical form.
pub fn schema_json() -> String {
    let schema = schemars::schema_for!(ModelsCatalog);
    let mut json = serde_json::to_string_pretty(&schema)
        .expect("JSON Schema serialization cannot fail");
    json.push('\n');
    json
}

/// A possible new variant-vocabulary token surfaced by the gen report.
///
/// Feeds the human promotion workflow from the boundary design doc: the
/// parser flags unrecognized candidates; a human promotes them to the curated
/// table in `unchained-ai/lib/src/models/identity.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateToken {
    pub token: String,
    /// Number of distinct families whose trailing token this is.
    pub family_count: usize,
}

/// Flags trailing family tokens that look like unpromoted variant vocabulary.
///
/// Considers the trailing token of each multi-token family, keeps pure-alpha
/// tokens of length >= 2, and counts across distinct families. Tokens already
/// in the parser's variant vocabulary are excluded by construction (the
/// parser strips them from families); the probe parse below also guards
/// against future vocab drift. Sorted by count descending, then token.
pub fn candidate_variant_tokens(catalog: &ModelsCatalog) -> Vec<CandidateToken> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for family_key in catalog.families.keys() {
        let family = family_key.split_once('/').map_or(family_key.as_str(), |(_, f)| f);
        let Some((_, token)) = family.rsplit_once('-') else {
            continue;
        };
        if token.len() < 2 || !token.chars().all(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        let probe = ModelIdentity::parse(&format!("vendor/stem-{token}"));
        if probe.variants.iter().any(|v| v == token) {
            continue;
        }
        *counts.entry(token.to_string()).or_default() += 1;
    }
    let mut candidates: Vec<CandidateToken> = counts
        .into_iter()
        .map(|(token, family_count)| CandidateToken {
            token,
            family_count,
        })
        .collect();
    candidates.sort_by(|a, b| b.family_count.cmp(&a.family_count).then_with(|| a.token.cmp(&b.token)));
    candidates
}

/// Assembles one artifact offering; metadata is supplied by the caller so
/// fixture tests can drive the builder without the generated catalog.
fn offering_for(wire_id: &str, metadata: Option<OfferingMetadata>) -> Offering {
    let parsed = ModelIdentity::parse(wire_id);
    Offering {
        id: wire_id.to_string(),
        source: parsed.source.clone(),
        vendor: parsed.vendor.clone(),
        identity_key: identity_key(&parsed),
        identity: Identity {
            family: parsed.family,
            version: parsed.version,
            version_raw: parsed.version_raw,
            date_pin: parsed.date_pin,
            size: parsed.size,
            variants: parsed.variants,
            serving: parsed.serving,
            rolling: parsed.rolling,
        },
        metadata,
    }
}

fn build_from_offerings(generated_at: String, mut offerings: Vec<Offering>) -> ModelsCatalog {
    offerings.sort_by(|a, b| a.id.cmp(&b.id));

    let mut families: BTreeMap<String, FamilyEntry> = BTreeMap::new();
    let mut gaps: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for (index, offering) in offerings.iter().enumerate() {
        if offering.identity.family.is_empty() {
            gaps.push(offering.id.clone());
            continue;
        }
        let family_key = format!("{}/{}", offering.vendor, offering.identity.family);
        let entry = families.entry(family_key).or_default();
        if offering.identity.rolling {
            entry.rolling_aliases.push(offering.id.clone());
        } else {
            entry.members.push(offering.id.clone());
        }
        if let Some(key) = &offering.identity_key {
            groups.entry(key.clone()).or_default().push(index);
        }
    }

    for entry in families.values_mut() {
        // Re-parsing keeps the ordering rule single-sourced in
        // ModelIdentity::release_order; wire id is the deterministic
        // tie-break.
        entry.members.sort_by(|a, b| {
            ModelIdentity::parse(a)
                .release_order(&ModelIdentity::parse(b))
                .then_with(|| a.cmp(b))
        });
        entry.latest = entry
            .members
            .last()
            .and_then(|id| identity_key(&ModelIdentity::parse(id)));
    }

    let duplicate_groups = groups
        .into_iter()
        .filter_map(|(key, indexes)| {
            let sources: BTreeSet<&str> = indexes
                .iter()
                .map(|&i| offerings[i].source.as_str())
                .collect();
            if sources.len() < 2 {
                return None;
            }
            let mut ids: Vec<String> = indexes.iter().map(|&i| offerings[i].id.clone()).collect();
            ids.sort();
            Some((key, ids))
        })
        .collect();

    ModelsCatalog {
        schema_version: SCHEMA_VERSION,
        generated_at,
        offerings,
        families,
        duplicate_groups,
        gaps,
    }
}

fn metadata_from(meta: &ProviderModelMetadata) -> OfferingMetadata {
    OfferingMetadata {
        display_name: meta.display_name.clone(),
        family: meta.family.clone(),
        context_window: meta.context_window,
        max_output_tokens: meta.max_output_tokens,
        modalities: meta.modalities.as_ref().map(|m| OfferingModalities {
            input: m.input.iter().map(ToString::to_string).collect(),
            output: m.output.iter().map(ToString::to_string).collect(),
        }),
        capabilities: meta.capabilities.clone(),
        description: meta.description.clone(),
        pricing: meta.pricing.as_ref().map(|p| OfferingPricing {
            prompt_per_token: p.prompt_per_token,
            completion_per_token: p.completion_per_token,
            web_search_per_request: p.web_search_per_request,
            input_cache_read_per_token: p.input_cache_read_per_token,
        }),
        supported_parameters: meta.supported_parameters.clone(),
        default_parameters: meta.default_parameters.as_ref().map(|d| {
            OfferingDefaultParameters {
                temperature: d.temperature,
                top_p: d.top_p,
                top_k: d.top_k,
                frequency_penalty: d.frequency_penalty,
                presence_penalty: d.presence_penalty,
            }
        }),
        knowledge_cutoff: meta.knowledge_cutoff.clone(),
        created: meta.created,
        release_date: meta.release_date.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_of(wire_id: &str) -> Option<String> {
        identity_key(&ModelIdentity::parse(wire_id))
    }

    #[test]
    fn identity_key_versioned() {
        assert_eq!(
            key_of("openrouter/anthropic/claude-sonnet-4.5"),
            Some("anthropic/claude-sonnet@4.5".to_string())
        );
    }

    #[test]
    fn identity_key_prefers_version_over_date_pin() {
        assert_eq!(
            key_of("anthropic/claude-sonnet-4-5-20250929"),
            Some("anthropic/claude-sonnet@4.5".to_string())
        );
    }

    #[test]
    fn identity_key_date_pinned_when_versionless() {
        assert_eq!(
            key_of("mistral/codestral-2508"),
            Some("mistral/codestral@2508".to_string())
        );
    }

    #[test]
    fn identity_key_versionless() {
        assert_eq!(
            key_of("groq/compound"),
            Some("groq/compound".to_string())
        );
    }

    #[test]
    fn identity_key_variant_and_size_sorted() {
        assert_eq!(
            key_of("openrouter/qwen/qwen3-coder-30b-a3b-instruct"),
            Some("qwen/qwen-coder@3+instruct+30b+a3b".to_string())
        );
    }

    #[test]
    fn identity_key_thinking_tag_joins() {
        assert_eq!(
            key_of("openrouter/anthropic/claude-3.7-sonnet:thinking"),
            Some("anthropic/claude-sonnet@3.7:thinking".to_string())
        );
    }

    #[test]
    fn identity_key_free_tag_does_not_join() {
        assert_eq!(
            key_of("openrouter/qwen/qwen3-coder:free"),
            key_of("openrouter/qwen/qwen3-coder"),
        );
    }

    #[test]
    fn identity_key_empty_family_is_none() {
        assert_eq!(identity_key(&ModelIdentity::default()), None);
    }

    fn fixture_catalog(ids: &[&str]) -> ModelsCatalog {
        let offerings = ids.iter().map(|id| offering_for(id, None)).collect();
        build_from_offerings("2026-01-01T00:00:00+00:00".to_string(), offerings)
    }

    #[test]
    fn family_members_release_ordered_with_latest() {
        let catalog = fixture_catalog(&[
            "openrouter/anthropic/claude-opus-4.7",
            "anthropic/claude-opus-4-20250514",
            "anthropic/claude-opus-4-5-20251101",
        ]);
        let entry = &catalog.families["anthropic/claude-opus"];
        assert_eq!(
            entry.members,
            vec![
                "anthropic/claude-opus-4-20250514",
                "anthropic/claude-opus-4-5-20251101",
                "openrouter/anthropic/claude-opus-4.7",
            ]
        );
        assert_eq!(entry.latest.as_deref(), Some("anthropic/claude-opus@4.7"));
    }

    #[test]
    fn rolling_aliases_excluded_from_members() {
        let catalog = fixture_catalog(&["moonshotai/kimi-latest", "moonshotai/kimi-k2.6"]);
        let entry = &catalog.families["moonshotai/kimi"];
        assert_eq!(entry.rolling_aliases, vec!["moonshotai/kimi-latest"]);
        assert!(entry.members.is_empty());
        assert_eq!(entry.latest, None);

        let k_entry = &catalog.families["moonshotai/kimi-k"];
        assert_eq!(k_entry.members, vec!["moonshotai/kimi-k2.6"]);
    }

    #[test]
    fn duplicate_groups_require_multiple_sources() {
        let catalog = fixture_catalog(&[
            // Same key, two sources: retained.
            "anthropic/claude-sonnet-4.5",
            "openrouter/anthropic/claude-sonnet-4.5",
            // Same key twice from one source: dropped.
            "zenmux/anthropic/claude-opus-4.7",
        ]);
        assert_eq!(
            catalog.duplicate_groups.get("anthropic/claude-sonnet@4.5"),
            Some(&vec![
                "anthropic/claude-sonnet-4.5".to_string(),
                "openrouter/anthropic/claude-sonnet-4.5".to_string(),
            ])
        );
        assert!(!catalog.duplicate_groups.contains_key("anthropic/claude-opus@4.7"));
    }

    #[test]
    fn offerings_sorted_and_deterministic() {
        let forward = fixture_catalog(&["b/model-1", "a/model-1"]);
        let reverse = fixture_catalog(&["a/model-1", "b/model-1"]);
        assert_eq!(forward, reverse);
        assert_eq!(forward.offerings[0].id, "a/model-1");
    }

    #[test]
    fn metadata_serializes_release_date_and_skips_none() {
        let source = ProviderModelMetadata {
            release_date: Some("2026-07-06".to_string()),
            ..ProviderModelMetadata::default()
        };
        let offering = offering_for("anthropic/claude-opus-4-8", Some(metadata_from(&source)));
        let json = serde_json::to_value(&offering).unwrap();
        assert_eq!(json["metadata"]["release_date"], "2026-07-06");
        let round_trip: Offering = serde_json::from_value(json).unwrap();
        assert_eq!(
            round_trip
                .metadata
                .and_then(|metadata| metadata.release_date)
                .as_deref(),
            Some("2026-07-06")
        );

        let without_release =
            serde_json::to_value(offering_for("anthropic/claude-sonnet-4-5", None)).unwrap();
        assert!(
            without_release.get("metadata").is_none(),
            "release_date must be absent when no source date is known"
        );
    }
}
