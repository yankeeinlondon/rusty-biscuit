//! Expected-offering classification and artifact join (Phase F,
//! claudine-gen consumption round 1).
//!
//! Builds the catalog-shaped values behind two coercions:
//! `DefaultModelsToExpectedOfferings` (agent-models `default_models[]` →
//! classified, artifact-joined offering records) and
//! `LocalRunnersToOfferingSources` (model-config `local_runners[]` →
//! offering-source namespace records). The join is exact-lookup only —
//! the identity grammar is unchained-ai's parser, never reimplemented
//! here (design/model-catalog-boundary.md).

use claudine_catalog_types::{LocalRunnerIntegration, OfferingClass, family_key};
use serde_json::{Map, Value};
use strum::VariantNames;

use crate::artifact::ArtifactIndex;
use crate::errors::GenError;
use crate::registry::RegistryEntry;

/// Per-provider source prefix for the exact-wire-id join rung: the
/// artifact spells vendor-API offerings `source/model-id`. Providers
/// without a mapped source (goose, qwen, opencode) skip straight to the
/// bare-id rung.
const ARTIFACT_SOURCE_BY_SLUG: &[(&str, &str)] = &[
    ("claude", "anthropic"),
    ("codex", "openai"),
    ("gemini", "gemini"),
    ("kimi", "moonshotai"),
];

/// Curated classification table — reviewable in PRs like code, same
/// philosophy as unchained-ai's curation tables. v1 rulings:
///
/// - kimi `kimi-for-coding` (alias `kimi-code`) is the managed Kimi Code
///   plan endpoint (F4 ruling, 2026-07-06): a subscription surface, not a
///   model-API offering — absent from the unchained catalog by design.
/// - opencode `opencode/`-prefixed ids are Zen gateway offerings fronting
///   other vendors' models → aggregator.
/// - everything else fronts a vendor model API → vendor_api.
pub fn classify(slug: &str, id: &str) -> OfferingClass {
    if slug == "kimi" && id == "kimi-for-coding" {
        return OfferingClass::PlanEndpoint;
    }
    if slug == "opencode" && id.starts_with("opencode/") {
        return OfferingClass::Aggregator;
    }
    OfferingClass::VendorApi
}

/// Curated escape hatch from the `resolves: family_latest` marking rule
/// (`(slug, id)` pairs). Empty in v1: today alias+join always means a
/// rolling pointer. A future joined router id (an alias that dispatches
/// across families rather than tracking one) would be pinned here so it
/// never claims a single family's `latest`.
pub const RESOLVES_EXCEPTIONS: &[(&str, &str)] = &[];

/// The exact-only join ladder:
///
/// (a) exact wire id `mapped_source/<id>` via [`ARTIFACT_SOURCE_BY_SLUG`];
/// (b) bare-id unique match — the id (for opencode, after stripping the
///     `opencode/` gateway prefix) equals the model-id part of artifact
///     offerings AND every match shares exactly one identity key.
///
/// Plan-endpoint ids skip the ladder entirely: they are not in the
/// artifact by design, so a lookup hit would be a false join.
pub fn catalog_id<'i>(
    slug: &str,
    id: &str,
    class: OfferingClass,
    index: &'i ArtifactIndex,
) -> Option<&'i str> {
    if class == OfferingClass::PlanEndpoint {
        return None;
    }
    if let Some((_, source)) = ARTIFACT_SOURCE_BY_SLUG.iter().find(|(s, _)| *s == slug)
        && let Some(key) = index.exact_key(&format!("{source}/{id}"))
    {
        return Some(key);
    }
    let bare = match slug {
        "opencode" => id.strip_prefix("opencode/").unwrap_or(id),
        _ => id,
    };
    index.bare_unique_key(bare)
}

/// Per-provider join coverage, rendered by the `generate`/`check` reports.
#[derive(Debug, Clone, Default)]
pub struct OfferingJoinReport {
    /// Records that joined an identity key.
    pub joined: usize,
    /// All expected-offering records (plan endpoints included).
    pub total: usize,
    /// Non-plan-endpoint ids with no confident join.
    pub unjoined: Vec<String>,
    /// Aliases shared by records whose joins derive DIFFERENT family
    /// keys — the `resolves` mark was dropped from every carrier.
    pub ambiguous_aliases: Vec<String>,
}

/// Source-side half of `DefaultModelsToExpectedOfferings`: agent-models
/// `default_models[]` records → the catalog-shaped expected-offering
/// list, lexically sorted by id.
///
/// `resolves` marking rule: a record is marked `family_latest` iff it
/// carries BOTH an alias AND a `catalog_id` join (minus the curated
/// [`RESOLVES_EXCEPTIONS`]). Duplicate aliases within one provider are
/// fine when their joins derive one family key; same-alias records
/// deriving DIFFERENT family keys lose the mark on every carrier and the
/// alias is reported ambiguous.
pub fn expected_offerings_value(
    entry: &RegistryEntry,
    raw: &Value,
    slug: &str,
    index: &ArtifactIndex,
    report: &mut OfferingJoinReport,
) -> Result<Value, GenError> {
    let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: "expected an array of default-model records".into(),
    })?;
    let mut by_id: std::collections::BTreeMap<String, Value> = std::collections::BTreeMap::new();
    let mut families_by_alias: std::collections::BTreeMap<
        String,
        std::collections::BTreeSet<String>,
    > = std::collections::BTreeMap::new();
    for record in records {
        let id = record
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("default-model record has no string `id`: `{record}`"),
            })?;
        let class = classify(slug, id);
        let joined = catalog_id(slug, id, class, index);
        let alias = record.get("alias").and_then(Value::as_str);
        let resolves = match (alias, joined) {
            (Some(alias), Some(key)) if !RESOLVES_EXCEPTIONS.contains(&(slug, id)) => {
                families_by_alias
                    .entry(alias.to_string())
                    .or_default()
                    .insert(family_key(key).to_string());
                true
            }
            _ => false,
        };

        let mut row = Map::new();
        row.insert("id".into(), Value::String(id.to_string()));
        row.insert("alias".into(), record.get("alias").cloned().unwrap_or(Value::Null));
        row.insert(
            "is_default".into(),
            Value::Bool(record.get("is_default").and_then(Value::as_bool).unwrap_or(false)),
        );
        row.insert(
            "context_window".into(),
            record.get("context_window").cloned().unwrap_or(Value::Null),
        );
        row.insert(
            "class".into(),
            Value::String(<&'static str>::from(class).to_string()),
        );
        row.insert(
            "catalog_id".into(),
            joined.map(|key| Value::String(key.to_string())).unwrap_or(Value::Null),
        );
        row.insert(
            "resolves".into(),
            if resolves {
                Value::String("family_latest".into())
            } else {
                Value::Null
            },
        );
        if by_id.insert(id.to_string(), Value::Object(row)).is_some() {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("duplicate default-model id `{id}` — reconcile the research"),
            });
        }

        report.total += 1;
        if joined.is_some() {
            report.joined += 1;
        } else if class != OfferingClass::PlanEndpoint {
            report.unjoined.push(id.to_string());
        }
    }

    // Ambiguity post-pass: an alias whose marked carriers span more than
    // one family cannot name a single family's `latest` — drop the mark
    // from every carrier, loudly.
    for (alias, keys) in &families_by_alias {
        if keys.len() <= 1 {
            continue;
        }
        report.ambiguous_aliases.push(alias.clone());
        for row in by_id.values_mut() {
            let row = row.as_object_mut().expect("rows are objects");
            if row.get("alias").and_then(Value::as_str) == Some(alias) {
                row.insert("resolves".into(), Value::Null);
            }
        }
    }
    Ok(Value::Array(by_id.into_values().collect()))
}

/// Source-side half of `LocalRunnersToOfferingSources`: model-config
/// `local_runners[]` records → the catalog-shaped offering-source list,
/// sorted by prefix. A missing/null source key (optional entry) emits an
/// empty list.
pub fn offering_sources_value(entry: &RegistryEntry, raw: &Value) -> Result<Value, GenError> {
    let records = match raw {
        Value::Null => return Ok(Value::Array(Vec::new())),
        Value::Array(records) => records,
        other => {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected an array of local-runner records, got `{other}`"),
            });
        }
    };
    let mut by_prefix: std::collections::BTreeMap<String, Value> =
        std::collections::BTreeMap::new();
    for record in records {
        let runner = record
            .get("runner")
            .and_then(Value::as_str)
            .ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("local-runner record has no string `runner`: `{record}`"),
            })?;
        let integration = record
            .get("integration")
            .and_then(Value::as_str)
            .ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("local-runner record has no string `integration`: `{record}`"),
            })?;
        if !LocalRunnerIntegration::VARIANTS.contains(&integration) {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("`{integration}` is not a LocalRunnerIntegration member"),
            });
        }
        let mut row = Map::new();
        row.insert("prefix".into(), Value::String(runner.to_string()));
        row.insert(
            "class".into(),
            Value::String(<&'static str>::from(OfferingClass::LocalRunner).to_string()),
        );
        row.insert(
            "api_standard".into(),
            record.get("standard").cloned().unwrap_or(Value::Null),
        );
        row.insert(
            "integration".into(),
            Value::String(integration.to_string()),
        );
        if by_prefix.insert(runner.to_string(), Value::Object(row)).is_some() {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("duplicate local-runner `{runner}` — reconcile the research"),
            });
        }
    }
    Ok(Value::Array(by_prefix.into_values().collect()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::registry::entry_for;

    fn index() -> ArtifactIndex {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models-catalog.json");
        std::fs::write(
            &path,
            json!({
                "schema_version": crate::artifact::EXPECTED_SCHEMA_VERSION,
                "generated_at": "2026-07-01T00:00:00Z",
                "offerings": [
                    { "id": "moonshotai/kimi-k2.7-code", "identity_key": "moonshotai/kimi-k-code@2.7" },
                    { "id": "anthropic/claude-opus-4-7", "identity_key": "anthropic/claude-opus@4.7" },
                    { "id": "anthropic/claude-opus-4-8", "identity_key": "anthropic/claude-opus@4.8" },
                    { "id": "zenmux/anthropic/claude-opus-4-8", "identity_key": "anthropic/claude-opus@4.8" },
                    { "id": "anthropic/claude-sonnet-5", "identity_key": "anthropic/claude-sonnet@5" },
                    { "id": "opencode/kimi-k2.6", "identity_key": "moonshotai/kimi-k@2.6" },
                    { "id": "vendor-a/shared", "identity_key": "vendor-a/shared" },
                    { "id": "vendor-b/shared", "identity_key": "vendor-b/shared" }
                ]
            })
            .to_string(),
        )
        .unwrap();
        crate::artifact::load_from(
            &path,
            chrono::DateTime::parse_from_rfc3339("2026-07-06T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        )
        .unwrap()
    }

    #[test]
    fn classification_table_applies_the_v1_rulings() {
        assert_eq!(classify("kimi", "kimi-for-coding"), OfferingClass::PlanEndpoint);
        assert_eq!(classify("kimi", "kimi-k2.7-code"), OfferingClass::VendorApi);
        assert_eq!(
            classify("opencode", "opencode/claude-opus-4-8"),
            OfferingClass::Aggregator
        );
        assert_eq!(classify("claude", "claude-opus-4-8"), OfferingClass::VendorApi);
        // The kimi plan-endpoint ruling is id-scoped, not fleet-wide.
        assert_eq!(classify("claude", "kimi-for-coding"), OfferingClass::VendorApi);
    }

    #[test]
    fn join_ladder_exact_hit_uses_the_mapped_source() {
        let index = index();
        assert_eq!(
            catalog_id("kimi", "kimi-k2.7-code", OfferingClass::VendorApi, &index),
            Some("moonshotai/kimi-k-code@2.7")
        );
        assert_eq!(
            catalog_id("claude", "claude-opus-4-8", OfferingClass::VendorApi, &index),
            Some("anthropic/claude-opus@4.8")
        );
    }

    /// Providers without a mapped source join through the bare-id rung;
    /// opencode additionally strips its gateway prefix first.
    #[test]
    fn join_ladder_bare_unique_hit() {
        let index = index();
        assert_eq!(
            catalog_id("goose", "claude-opus-4-8", OfferingClass::VendorApi, &index),
            Some("anthropic/claude-opus@4.8")
        );
        assert_eq!(
            catalog_id(
                "opencode",
                "opencode/kimi-k2.6",
                OfferingClass::Aggregator,
                &index
            ),
            Some("moonshotai/kimi-k@2.6")
        );
    }

    #[test]
    fn join_ladder_ambiguous_bare_and_zero_matches_yield_none() {
        let index = index();
        assert_eq!(
            catalog_id("goose", "shared", OfferingClass::VendorApi, &index),
            None,
            "two identity keys under one spelling is ambiguous"
        );
        assert_eq!(
            catalog_id("qwen", "qwen3.9-ultra", OfferingClass::VendorApi, &index),
            None
        );
    }

    /// A plan-endpoint id skips the ladder even when a lookup would hit.
    #[test]
    fn join_ladder_plan_endpoint_skips_lookup() {
        let index = index();
        assert_eq!(
            catalog_id(
                "kimi",
                "kimi-k2.7-code",
                OfferingClass::PlanEndpoint,
                &index
            ),
            None
        );
    }

    #[test]
    fn expected_offerings_value_classifies_joins_and_reports() {
        let index = index();
        let entry = entry_for("expected_offerings").expect("registered");
        let raw = json!([
            { "id": "kimi-for-coding", "alias": "kimi-code", "is_default": true, "context_window": 262144 },
            { "id": "kimi-k2.7-code", "context_window": 262144 },
            { "id": "kimi-unheard-of" },
        ]);
        let mut report = OfferingJoinReport::default();
        let value = expected_offerings_value(entry, &raw, "kimi", &index, &mut report).unwrap();
        assert_eq!(
            value,
            json!([
                {
                    "id": "kimi-for-coding",
                    "alias": "kimi-code",
                    "is_default": true,
                    "context_window": 262144,
                    "class": "plan_endpoint",
                    "catalog_id": null,
                    "resolves": null,
                },
                {
                    "id": "kimi-k2.7-code",
                    "alias": null,
                    "is_default": false,
                    "context_window": 262144,
                    "class": "vendor_api",
                    "catalog_id": "moonshotai/kimi-k-code@2.7",
                    "resolves": null,
                },
                {
                    "id": "kimi-unheard-of",
                    "alias": null,
                    "is_default": false,
                    "context_window": null,
                    "class": "vendor_api",
                    "catalog_id": null,
                    "resolves": null,
                },
            ])
        );
        assert_eq!((report.joined, report.total), (1, 3));
        // The plan endpoint is unjoined by design and stays off the list.
        assert_eq!(report.unjoined, ["kimi-unheard-of"]);
        assert!(report.ambiguous_aliases.is_empty());
    }

    /// The `resolves` mark requires BOTH an alias and a join: an
    /// alias-only record (the gemini `auto` router shape) and a
    /// join-only record both stay unmarked.
    #[test]
    fn resolves_marks_alias_plus_join_records_only() {
        let index = index();
        let entry = entry_for("expected_offerings").expect("registered");
        let raw = json!([
            { "id": "kimi-k2.7-code", "alias": "kcode" },
            { "id": "kimi-unheard-of", "alias": "ghost" },
            { "id": "kimi-k2.6" },
        ]);
        let mut report = OfferingJoinReport::default();
        let value = expected_offerings_value(entry, &raw, "kimi", &index, &mut report).unwrap();
        let resolves: Vec<(&str, bool)> = value
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["id"].as_str().unwrap(),
                    row["resolves"] == json!("family_latest"),
                )
            })
            .collect();
        assert_eq!(
            resolves,
            [
                ("kimi-k2.6", false),
                ("kimi-k2.7-code", true),
                ("kimi-unheard-of", false),
            ]
        );
        assert!(report.ambiguous_aliases.is_empty());
    }

    /// Duplicate aliases are fine when every carrier's join derives the
    /// same family key (the gemini `flash` shape): all stay marked.
    #[test]
    fn duplicate_aliases_in_one_family_keep_their_marks() {
        let index = index();
        let entry = entry_for("expected_offerings").expect("registered");
        let raw = json!([
            { "id": "claude-opus-4-7", "alias": "opus" },
            { "id": "claude-opus-4-8", "alias": "opus" },
        ]);
        let mut report = OfferingJoinReport::default();
        let value = expected_offerings_value(entry, &raw, "claude", &index, &mut report).unwrap();
        for row in value.as_array().unwrap() {
            assert_eq!(row["resolves"], json!("family_latest"), "{row}");
        }
        assert!(report.ambiguous_aliases.is_empty());
    }

    /// Same-alias records deriving DIFFERENT family keys are ambiguous:
    /// the mark drops from every carrier and the alias is reported.
    #[test]
    fn cross_family_duplicate_alias_drops_both_marks_and_reports() {
        let index = index();
        let entry = entry_for("expected_offerings").expect("registered");
        let raw = json!([
            { "id": "claude-opus-4-8", "alias": "best" },
            { "id": "claude-sonnet-5", "alias": "best" },
            { "id": "claude-opus-4-7", "alias": "opus" },
        ]);
        let mut report = OfferingJoinReport::default();
        let value = expected_offerings_value(entry, &raw, "claude", &index, &mut report).unwrap();
        let resolves: Vec<(&str, bool)> = value
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["id"].as_str().unwrap(),
                    row["resolves"] == json!("family_latest"),
                )
            })
            .collect();
        assert_eq!(
            resolves,
            [
                ("claude-opus-4-7", true),
                ("claude-opus-4-8", false),
                ("claude-sonnet-5", false),
            ]
        );
        assert_eq!(report.ambiguous_aliases, ["best"]);
    }

    #[test]
    fn offering_sources_value_maps_and_sorts_runners() {
        let entry = entry_for("offering_sources").expect("registered");
        let raw = json!([
            { "runner": "ollama", "integration": "first_class", "standard": "openai_compatible" },
            { "runner": "llamacpp", "integration": "base_url_override", "standard": "openai_compatible" },
        ]);
        let value = offering_sources_value(entry, &raw).unwrap();
        assert_eq!(
            value,
            json!([
                {
                    "prefix": "llamacpp",
                    "class": "local_runner",
                    "api_standard": "openai_compatible",
                    "integration": "base_url_override",
                },
                {
                    "prefix": "ollama",
                    "class": "local_runner",
                    "api_standard": "openai_compatible",
                    "integration": "first_class",
                },
            ])
        );
        // Optional source key absent → empty list, not an error.
        assert_eq!(
            offering_sources_value(entry, &Value::Null).unwrap(),
            json!([])
        );
        // Unknown integration member fails the vocabulary loudly.
        let err = offering_sources_value(
            entry,
            &json!([{ "runner": "ollama", "integration": "telepathic" }]),
        )
        .unwrap_err();
        assert!(matches!(err, GenError::UnmappableValue { .. }));
    }
}
