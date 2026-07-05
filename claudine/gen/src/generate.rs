//! The generation pipeline: gates → value resolution → data.rs emission.
//!
//! `generate` and `check` share one code path (`--check` IS the drift
//! test): both call [`generate_for_area`], and `check` additionally
//! byte-compares the emitted file against the committed
//! `lib/src/provider/<slug>/data.rs`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use claudine_catalog_types::ModelCatalogSource;
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::emit::{self, FieldValue};
use crate::errors::GenError;
use crate::inputs::{self, ProviderInputs};
use crate::registry::{Coercion, DeclaredSource, REGISTRY, RegistryEntry, entry_for};
use crate::schema_compat;

/// Where a resolved field's value came from.
#[derive(Debug, Clone)]
pub enum Provenance {
    /// The field's declared source supplied the value.
    Declared { kind: &'static str },
    /// An override won.
    Override {
        reason: String,
        /// The catalog-shaped value the declared source produced, when it
        /// produced one — the value the override is suppressing.
        suppressed: Option<Value>,
        /// `true` when the suppressed value equals the override (the
        /// staleness lint: the override should be deleted).
        stale: bool,
    },
}

/// One resolved field: the catalog-shaped value plus its provenance.
#[derive(Debug, Clone)]
pub struct ResolvedField {
    pub field: &'static str,
    /// Catalog-shaped value (the shape overrides are authored in).
    pub value: Value,
    pub provenance: Provenance,
}

/// Input records a coercion was pointed at but could not use.
///
/// A silent drop here is the one gap in the fail-loudly posture: the field
/// still generates, so nothing errors, but data quietly never enters the
/// catalog (Checkpoint A ruling, 2026-07-04). Skips are collected during
/// value mapping — including for override-pinned fields, so the drop is
/// visible before the override is ever deleted — and rendered by both the
/// `generate` and `check` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoercionSkip {
    pub field: &'static str,
    /// Why the records were unusable (one reason per skip entry).
    pub reason: &'static str,
    /// Display form of each skipped record.
    pub records: Vec<String>,
}

/// A completed generation for one provider.
#[derive(Debug)]
pub struct Generation {
    pub slug: String,
    pub fields: Vec<ResolvedField>,
    /// The emitted `data.rs` file text (deterministic, byte-stable).
    pub data_rs: String,
    /// Coercion skips gathered during value mapping (never empty silently —
    /// the report prints every entry).
    pub skips: Vec<CoercionSkip>,
    /// Topic → schema-validated, coerced frontmatter — the superset the
    /// committed `catalog.json` projects (mapped and unmapped fields).
    pub research: std::collections::BTreeMap<String, Value>,
}

/// Every generated provider slug, `PROVIDERS_DISPLAY_ORDER` order. The
/// catalog document requires the full scope, so this is the single list
/// the CLI, the drift test, and [`generate_all`] share.
pub const PROVIDER_SLUGS: &[&str] = &[
    "claude", "codex", "gemini", "goose", "kimi", "opencode", "qwen",
];

/// Generates every provider in [`PROVIDER_SLUGS`] order.
pub fn generate_all(area: &Path) -> Result<Vec<Generation>, GenError> {
    PROVIDER_SLUGS
        .iter()
        .map(|slug| generate_for_area(area, slug))
        .collect()
}

/// Outcome of comparing a generation against the committed file.
#[derive(Debug)]
pub enum CheckOutcome {
    Clean,
    Drift { details: Vec<String> },
    MissingCommitted { path: PathBuf },
}

/// The committed generated-data path for a provider under an area root.
pub fn committed_data_path(area: &Path, slug: &str) -> PathBuf {
    area.join(format!("lib/src/provider/{slug}/data.rs"))
}

/// Walks upward from `from` to find the claudine package area (the
/// directory containing `docs/providers.yaml`, either directly or as a
/// `claudine/` child — so running from the repo root works).
pub fn find_area(from: &Path) -> Result<PathBuf, GenError> {
    let mut dir = Some(from);
    while let Some(current) = dir {
        if current.join("docs/providers.yaml").is_file() {
            return Ok(current.to_path_buf());
        }
        let child = current.join("claudine");
        if child.join("docs/providers.yaml").is_file() {
            return Ok(child);
        }
        dir = current.parent();
    }
    Err(GenError::AreaNotFound {
        from: from.to_path_buf(),
    })
}

/// Runs the full pipeline for one provider under `area`.
///
/// Order matters: schema↔catalog compatibility and source-collision gates
/// run BEFORE any value mapping.
pub fn generate_for_area(area: &Path, slug: &str) -> Result<Generation, GenError> {
    let topics: BTreeSet<&str> = REGISTRY
        .iter()
        .filter_map(|entry| match entry.source {
            DeclaredSource::Research { topic, .. } => Some(topic),
            _ => None,
        })
        .collect();
    let topics: Vec<&str> = topics.into_iter().collect();
    let inputs = inputs::load(area, slug, &topics)?;

    // Gate 1: schema<->catalog enum-subset / shape compatibility.
    let mut sidecars = std::collections::BTreeMap::new();
    for (topic, path) in &inputs.sidecars {
        sidecars.insert(topic.clone(), schema_compat::load_sidecar_schema(path)?);
    }
    schema_compat::check_entries(REGISTRY, |topic| sidecars.get(topic))?;

    // Gate 2: source collisions (a value arriving from a source other than
    // the field's declared one; overrides are the sanctioned exception).
    // A facts file still carrying a research-graduated key (e.g.
    // `supports_skills`) fails here — delete-on-graduate is enforced.
    check_collisions(&inputs)?;

    // Value mapping + override application, then whole-file emission.
    let (fields, skips) = resolve_fields(&inputs)?;
    let values: Vec<FieldValue<'_>> = fields
        .iter()
        .map(|f| FieldValue {
            field: f.field,
            value: &f.value,
        })
        .collect();
    let display_name = fields
        .iter()
        .find(|f| f.field == "display_name")
        .and_then(|f| f.value.as_str())
        .ok_or_else(|| GenError::MissingValue {
            field: "display_name",
            message: "display_name did not resolve to a string".into(),
        })?
        .to_string();
    let data_rs = emit::emit_data_file(slug, &display_name, &values)?;

    Ok(Generation {
        slug: slug.to_string(),
        fields,
        data_rs,
        skips,
        research: inputs.research,
    })
}

/// Generates and byte-compares against the committed data.rs. This is the
/// single code path behind both the CLI `check` subcommand and the nextest
/// drift test.
pub fn check_area(area: &Path, slug: &str) -> Result<(Generation, CheckOutcome), GenError> {
    let generation = generate_for_area(area, slug)?;
    let committed_path = committed_data_path(area, slug);
    if !committed_path.is_file() {
        return Ok((
            generation,
            CheckOutcome::MissingCommitted {
                path: committed_path,
            },
        ));
    }
    let committed = std::fs::read_to_string(&committed_path).map_err(|source| GenError::Io {
        path: committed_path.clone(),
        source,
    })?;
    if committed == generation.data_rs {
        return Ok((generation, CheckOutcome::Clean));
    }
    let details = diff_lines(&committed, &generation.data_rs);
    Ok((generation, CheckOutcome::Drift { details }))
}

/// Line-oriented drift summary (committed vs regenerated).
pub fn diff_lines(committed: &str, generated: &str) -> Vec<String> {
    let committed: Vec<&str> = committed.lines().collect();
    let generated: Vec<&str> = generated.lines().collect();
    let mut details = Vec::new();
    for i in 0..committed.len().max(generated.len()) {
        let old = committed.get(i).copied();
        let new = generated.get(i).copied();
        if old != new {
            details.push(format!(
                "line {}: committed `{}` vs generated `{}`",
                i + 1,
                old.unwrap_or("<absent>"),
                new.unwrap_or("<absent>"),
            ));
        }
    }
    details
}

/// Source-collision gate over the human-owned input files.
fn check_collisions(inputs: &ProviderInputs) -> Result<(), GenError> {
    for key in inputs.facts.keys() {
        match entry_for(key) {
            None => return Err(GenError::UnknownFactsKey { key: key.clone() }),
            Some(entry) if !matches!(entry.source, DeclaredSource::Facts { .. }) => {
                return Err(GenError::SourceCollision {
                    field: key.clone(),
                    declared: entry.source.kind().to_string(),
                    offending: "facts".to_string(),
                });
            }
            Some(_) => {}
        }
    }
    if let Some(roster) = inputs.roster.as_object() {
        for key in roster.keys() {
            if let Some(entry) = entry_for(key)
                && !matches!(entry.source, DeclaredSource::Roster { .. })
            {
                return Err(GenError::SourceCollision {
                    field: key.clone(),
                    declared: entry.source.kind().to_string(),
                    offending: "roster".to_string(),
                });
            }
        }
    }
    for field in inputs.overrides.keys() {
        if entry_for(field).is_none() {
            return Err(GenError::UnknownOverrideField {
                field: field.clone(),
            });
        }
    }
    Ok(())
}

/// Resolves every registry field: declared source → catalog-shaped value,
/// then override application (whole-value replacement, staleness lint).
fn resolve_fields(
    inputs: &ProviderInputs,
) -> Result<(Vec<ResolvedField>, Vec<CoercionSkip>), GenError> {
    let mut fields = Vec::with_capacity(REGISTRY.len());
    let mut skips = Vec::new();
    for entry in REGISTRY {
        let source_value = extract_catalog_value(entry, inputs, &mut skips);
        let (value, provenance) = match inputs.overrides.get(entry.field) {
            Some(over) => {
                let suppressed = source_value.ok();
                let stale = suppressed.as_ref() == Some(&over.value);
                (
                    over.value.clone(),
                    Provenance::Override {
                        reason: over.reason.clone(),
                        suppressed,
                        stale,
                    },
                )
            }
            None => (
                source_value?,
                Provenance::Declared {
                    kind: entry.source.kind(),
                },
            ),
        };
        fields.push(ResolvedField {
            field: entry.field,
            value,
            provenance,
        });
    }
    Ok((fields, skips))
}

/// Extracts the declared source's raw value and coerces it to catalog
/// shape (the same shape overrides are authored in).
fn extract_catalog_value(
    entry: &RegistryEntry,
    inputs: &ProviderInputs,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    let raw = match entry.source {
        DeclaredSource::Roster { key } => match inputs.roster.get(key) {
            Some(value) => value.clone(),
            None if entry.optional => Value::Null,
            None => {
                return Err(GenError::RosterKeyMissing {
                    slug: inputs.slug.clone(),
                    key,
                });
            }
        },
        DeclaredSource::Facts { key } => match inputs.facts.get(key) {
            Some(value) => value.clone(),
            None if entry.optional => Value::Null,
            None => {
                return Err(GenError::MissingValue {
                    field: entry.field,
                    message: format!("facts file has no `{key}` key"),
                });
            }
        },
        DeclaredSource::Research { topic, path } => {
            let frontmatter =
                inputs
                    .research
                    .get(topic)
                    .ok_or_else(|| GenError::MissingValue {
                        field: entry.field,
                        message: format!("no research loaded for topic `{topic}`"),
                    })?;
            match walk_path(frontmatter, path) {
                Some(value) => value,
                None if entry.optional => Value::Null,
                None => {
                    return Err(GenError::MissingValue {
                        field: entry.field,
                        message: format!(
                            "frontmatter path `{path}` not found in topic `{topic}`"
                        ),
                    });
                }
            }
        }
    };
    coerce_to_catalog_shape(entry, &raw, skips)
}

/// Walks a dot-separated path into a JSON object.
fn walk_path(value: &Value, path: &str) -> Option<Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current.clone())
}

/// A single bare env-var identifier: `[A-Z][A-Z0-9_]*`. Compound sites
/// ("A / B") and annotated sites are excluded by construction.
fn is_env_var_ident(site: &str) -> bool {
    let mut chars = site.chars();
    matches!(chars.next(), Some('A'..='Z'))
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// A bare, usable config path: rejects prose annotations ("a; b",
/// "a, b", "a or b"), `<placeholder>` grammar, and env-var references
/// (`$VAR`) that no `PathContext` can resolve. Rejected records are
/// skipped loudly and, where they hollow out the projection, pinned back
/// via field-keyed overrides.
fn is_bare_config_path(path: &str) -> bool {
    !(path.contains(';')
        || path.contains(", ")
        || path.contains(" or ")
        || path.contains('<')
        || path.contains('$'))
}

/// Shared shape check: value must be an array of strings.
fn expect_string_array(entry: &RegistryEntry, raw: &Value) -> Result<Value, GenError> {
    let items = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: format!("expected a string array, got `{raw}`"),
    })?;
    for item in items {
        if !item.is_string() {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected string elements, got `{item}`"),
            });
        }
    }
    Ok(raw.clone())
}

/// Source-side half of a coercion: raw source value → catalog-shaped value.
///
/// Record-shaped facts coercions extract as identity — the facts file
/// already carries the catalog shape, and the expression half in [`emit`]
/// performs the loud shape validation.
fn coerce_to_catalog_shape(
    entry: &RegistryEntry,
    raw: &Value,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    match entry.coercion {
        Coercion::StringLiteral
        | Coercion::ProviderVariantFromSlug
        | Coercion::SniffBindingVariant => match raw {
            Value::String(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a string, got `{other}`"),
            }),
        },
        Coercion::OptionalStringLiteral | Coercion::StreamProtocolWire => match raw {
            Value::Null | Value::String(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a string or null, got `{other}`"),
            }),
        },
        Coercion::BoolLiteral => match raw {
            Value::Bool(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a boolean, got `{other}`"),
            }),
        },
        Coercion::StringSlice => expect_string_array(entry, raw),
        Coercion::SkillSupportToBool => {
            let member = raw.as_str().ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a support enum member, got `{raw}`"),
            })?;
            // Open question 3 ruling: first_class/partial -> true;
            // convention_only/none/unknown -> false.
            match member {
                "first_class" | "partial" => Ok(Value::Bool(true)),
                "convention_only" | "none" | "unknown" => Ok(Value::Bool(false)),
                other => Err(GenError::UnmappableValue {
                    field: entry.field,
                    message: format!("`{other}` is not a known skills `support` member"),
                }),
            }
        }
        Coercion::DefaultModelsToStaticModels => {
            let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: "expected an array of default-model records".into(),
            })?;
            let mut ids = BTreeSet::new();
            for record in records {
                let id = record.get("id").and_then(Value::as_str).ok_or_else(|| {
                    GenError::UnmappableValue {
                        field: entry.field,
                        message: format!("default-model record has no string `id`: `{record}`"),
                    }
                })?;
                ids.insert(id.to_string());
            }
            Ok(Value::Array(ids.into_iter().map(Value::String).collect()))
        }
        Coercion::DynamicListingToModelCatalogSource => match raw {
            Value::Bool(false) => Ok(Value::String("static".into())),
            Value::Bool(true) => Err(GenError::UnmappableValue {
                field: entry.field,
                message: "dynamic listing is available but the boolean cannot select a \
                          catalog mechanism — pin today's value with a field-keyed \
                          override until agent-models grows a typed source key"
                    .into(),
            }),
            // A future enum-typed sidecar reports the member directly.
            Value::String(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a boolean or enum member, got `{other}`"),
            }),
        },
        Coercion::EnvVarSitesToStringSlice => {
            let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: "expected an array of selection records".into(),
            })?;
            let mut seen = BTreeSet::new();
            let mut vars = Vec::new();
            let mut dropped = Vec::new();
            for record in records {
                if record.get("method").and_then(Value::as_str) != Some("env_var") {
                    continue;
                }
                let Some(site) = record.get("site").and_then(Value::as_str) else {
                    continue;
                };
                if !is_env_var_ident(site) {
                    dropped.push(site.to_string());
                } else if seen.insert(site.to_string()) {
                    vars.push(Value::String(site.to_string()));
                }
            }
            if !dropped.is_empty() {
                skips.push(CoercionSkip {
                    field: entry.field,
                    reason: "site is not a single env-var identifier",
                    records: dropped,
                });
            }
            Ok(Value::Array(vars))
        }
        Coercion::SurfacesToSessionLogPaths => {
            let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: "expected an array of logging-surface records".into(),
            })?;
            let mut paths = Vec::new();
            let mut dropped = Vec::new();
            for record in records {
                if record.get("role").and_then(Value::as_str) != Some("session_transcript") {
                    continue;
                }
                let Some(path) = record.get("path_macos").and_then(Value::as_str) else {
                    continue;
                };
                // " (" marks an annotated pseudo-path (e.g. a SQLite table
                // note), not a filesystem template.
                if path.contains(" (") {
                    dropped.push(path.to_string());
                } else {
                    paths.push(Value::String(path.to_string()));
                }
            }
            if !dropped.is_empty() {
                skips.push(CoercionSkip {
                    field: entry.field,
                    reason: "path_macos is an annotated pseudo-path, not a path template",
                    records: dropped,
                });
            }
            Ok(Value::Array(paths))
        }
        Coercion::ConfigPathRecordsToConfigPaths => {
            let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: "expected an array of config-path records".into(),
            })?;
            let mut paths = Vec::new();
            let mut dropped = Vec::new();
            for record in records {
                if record.get("os").and_then(Value::as_str) != Some("macos") {
                    continue;
                }
                if !matches!(
                    record.get("scope").and_then(Value::as_str),
                    Some("user") | Some("repo")
                ) {
                    continue;
                }
                let Some(path) = record.get("path").and_then(Value::as_str) else {
                    continue;
                };
                if !is_bare_config_path(path) {
                    dropped.push(path.to_string());
                } else {
                    paths.push(Value::String(path.to_string()));
                }
            }
            if !dropped.is_empty() {
                skips.push(CoercionSkip {
                    field: entry.field,
                    reason: "path is not a bare filesystem template",
                    records: dropped,
                });
            }
            Ok(Value::Array(paths))
        }
        // Facts-shaped records pass through; emit.rs validates loudly.
        Coercion::PathTemplateList
        | Coercion::EventMappingRecords
        | Coercion::ResourceSupportRecord
        | Coercion::OutputFormatRecords
        | Coercion::EntrypointRecords
        | Coercion::SystemPromptSpecRecord
        | Coercion::YoloRecordToYoloSupport
        | Coercion::ReasoningRecord
        | Coercion::KnownGapRecords
        | Coercion::AcpRecord
        | Coercion::PromptArgRecord
        | Coercion::AxesRecord => Ok(raw.clone()),
    }
}

/// snake_case member → unit `ModelCatalogSource` variant name. An unknown
/// member is the "new variant needed" moment and fails loudly;
/// `shell_command` carries data and must be authored as the externally
/// tagged object, never a bare member string.
pub(crate) fn model_catalog_source_variant(
    field: &'static str,
    member: &str,
) -> Result<String, GenError> {
    if member == "shell_command" {
        return Err(GenError::UnmappableValue {
            field,
            message: "`shell_command` carries data — author the object form \
                      `{shell_command: {program, args}}`, not a bare member string"
                .into(),
        });
    }
    ModelCatalogSource::iter()
        .find(|variant| <&'static str>::from(*variant) == member)
        .map(|variant| match variant {
            ModelCatalogSource::None => "None".to_string(),
            ModelCatalogSource::Static => "Static".to_string(),
            // Unreachable: the guard above rejected the member string.
            ModelCatalogSource::ShellCommand { .. } => unreachable!(),
        })
        .ok_or_else(|| GenError::UnmappableValue {
            field,
            message: format!("`{member}` is not a ModelCatalogSource variant"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_ident_accepts_bare_identifiers_only() {
        assert!(is_env_var_ident("ANTHROPIC_MODEL"));
        assert!(is_env_var_ident("CLAUDE_CODE_SUBAGENT_MODEL"));
        assert!(!is_env_var_ident("A / B"));
        assert!(!is_env_var_ident("lowercase"));
        assert!(!is_env_var_ident(""));
        assert!(!is_env_var_ident("1ABC"));
    }

    #[test]
    fn bare_config_path_rejects_annotations_and_placeholders() {
        assert!(is_bare_config_path("~/.claude/settings.json"));
        assert!(is_bare_config_path(".claude/settings.local.json"));
        assert!(is_bare_config_path("~/.qwen/debug/*.txt"));
        assert!(!is_bare_config_path(
            "$CODEX_HOME/config.toml; default /Users/<user>/.codex/config.toml"
        ));
        assert!(!is_bare_config_path("/Users/<name>/.kimi-code/config.toml"));
        assert!(!is_bare_config_path("AGENTS.override.md, AGENTS.md"));
        assert!(!is_bare_config_path("a.md or b.md"));
        assert!(!is_bare_config_path("$CODEX_HOME/auth.json"));
    }

    #[test]
    fn model_catalog_source_variant_maps_snake_members() {
        assert_eq!(
            model_catalog_source_variant("model_catalog_source", "static").unwrap(),
            "Static"
        );
        assert_eq!(
            model_catalog_source_variant("model_catalog_source", "none").unwrap(),
            "None"
        );
        assert!(matches!(
            model_catalog_source_variant("model_catalog_source", "telepathic"),
            Err(GenError::UnmappableValue { .. })
        ));
    }

    /// `shell_command` as a bare member string must fail loudly: the
    /// variant carries `program`/`args` and only the object form can
    /// supply them.
    #[test]
    fn model_catalog_source_variant_rejects_bare_shell_command() {
        assert!(matches!(
            model_catalog_source_variant("model_catalog_source", "shell_command"),
            Err(GenError::UnmappableValue { .. })
        ));
    }
}
