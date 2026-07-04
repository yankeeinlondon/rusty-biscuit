//! The generation pipeline: gates → value resolution → fragment emission.
//!
//! `generate` and `check` share one code path (`--check` IS the drift
//! test): both call [`generate_for_area`], and `check` additionally
//! byte-compares the emitted fragment against the committed file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use claudine_catalog_types::ModelCatalogSource;
use serde_json::Value;
use strum::IntoEnumIterator;

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

/// One generated field: the Rust expression plus its provenance.
#[derive(Debug, Clone)]
pub struct ResolvedField {
    pub field: &'static str,
    pub expr: String,
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
    /// The emitted `data.rs` fragment (deterministic, byte-stable).
    pub fragment: String,
    /// Coercion skips gathered during value mapping (never empty silently —
    /// the report prints every entry).
    pub skips: Vec<CoercionSkip>,
}

/// Outcome of comparing a generation against the committed fragment.
#[derive(Debug)]
pub enum CheckOutcome {
    Clean,
    Drift { details: Vec<String> },
    MissingCommitted { path: PathBuf },
}

/// The committed fragment path for a provider under an area root.
pub fn committed_fragment_path(area: &Path, slug: &str) -> PathBuf {
    area.join(format!("gen/generated/{slug}.data.rs"))
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
    check_collisions(&inputs)?;

    // Value mapping + override application.
    let (fields, skips) = resolve_fields(&inputs)?;
    let fragment = emit_fragment(slug, &fields);

    Ok(Generation {
        slug: slug.to_string(),
        fields,
        fragment,
        skips,
    })
}

/// Generates and byte-compares against the committed fragment. This is the
/// single code path behind both the CLI `check` subcommand and the nextest
/// drift test.
pub fn check_area(area: &Path, slug: &str) -> Result<(Generation, CheckOutcome), GenError> {
    let generation = generate_for_area(area, slug)?;
    let committed_path = committed_fragment_path(area, slug);
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
    if committed == generation.fragment {
        return Ok((generation, CheckOutcome::Clean));
    }
    let details = diff_lines(&committed, &generation.fragment);
    Ok((generation, CheckOutcome::Drift { details }))
}

/// Line-oriented drift summary (committed vs regenerated).
fn diff_lines(committed: &str, generated: &str) -> Vec<String> {
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
        let (catalog_value, provenance) = match inputs.overrides.get(entry.field) {
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
        let expr = catalog_expr(entry, &catalog_value)?;
        fields.push(ResolvedField {
            field: entry.field,
            expr,
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
        DeclaredSource::Roster { key } => {
            inputs
                .roster
                .get(key)
                .cloned()
                .ok_or(GenError::RosterKeyMissing {
                    slug: inputs.slug.clone(),
                    key,
                })?
        }
        DeclaredSource::Facts { key } => {
            inputs
                .facts
                .get(key)
                .cloned()
                .ok_or_else(|| GenError::MissingValue {
                    field: entry.field,
                    message: format!("facts file has no `{key}` key"),
                })?
        }
        DeclaredSource::Research { topic, path } => {
            let frontmatter =
                inputs
                    .research
                    .get(topic)
                    .ok_or_else(|| GenError::MissingValue {
                        field: entry.field,
                        message: format!("no research loaded for topic `{topic}`"),
                    })?;
            walk_path(frontmatter, path).ok_or_else(|| GenError::MissingValue {
                field: entry.field,
                message: format!("frontmatter path `{path}` not found in topic `{topic}`"),
            })?
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

/// Source-side half of a coercion: raw source value → catalog-shaped value.
fn coerce_to_catalog_shape(
    entry: &RegistryEntry,
    raw: &Value,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    match entry.coercion {
        Coercion::StringLiteral => match raw {
            Value::String(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a string, got `{other}`"),
            }),
        },
        Coercion::BoolLiteral => match raw {
            Value::Bool(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a boolean, got `{other}`"),
            }),
        },
        Coercion::DynamicListingToModelCatalogSource => match raw {
            Value::Bool(false) => Ok(Value::String("static".into())),
            Value::Bool(true) => Err(GenError::UnmappableValue {
                field: entry.field,
                message: "dynamic listing is available but the skeleton cannot select a \
                          dynamic catalog variant yet (Phase B)"
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
    }
}

/// A single bare env-var identifier: `[A-Z][A-Z0-9_]*`. Compound sites
/// ("A / B") and annotated sites are excluded by construction.
fn is_env_var_ident(site: &str) -> bool {
    let mut chars = site.chars();
    matches!(chars.next(), Some('A'..='Z'))
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Expression half of a coercion: catalog-shaped value → Rust field
/// expression text. Overrides flow through here too, so an override is
/// validated against the same shape as the field it replaces.
fn catalog_expr(entry: &RegistryEntry, value: &Value) -> Result<String, GenError> {
    match entry.coercion {
        Coercion::StringLiteral => value
            .as_str()
            .map(|s| format!("{s:?}"))
            .ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a string, got `{value}`"),
            }),
        Coercion::BoolLiteral => value
            .as_bool()
            .map(|b| b.to_string())
            .ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a boolean, got `{value}`"),
            }),
        Coercion::DynamicListingToModelCatalogSource => {
            let member = value.as_str().ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected an enum member string, got `{value}`"),
            })?;
            model_catalog_source_expr(entry, member)
        }
        Coercion::EnvVarSitesToStringSlice => {
            let items = value.as_array().ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a string array, got `{value}`"),
            })?;
            let mut literals = Vec::with_capacity(items.len());
            for item in items {
                let s = item.as_str().ok_or_else(|| GenError::UnmappableValue {
                    field: entry.field,
                    message: format!("expected a string element, got `{item}`"),
                })?;
                literals.push(format!("{s:?}"));
            }
            Ok(format!("&[{}]", literals.join(", ")))
        }
    }
}

/// snake_case member → `ModelCatalogSource::<Variant>` path expression.
/// An unknown member is the "new variant needed" moment and fails loudly.
fn model_catalog_source_expr(entry: &RegistryEntry, member: &str) -> Result<String, GenError> {
    ModelCatalogSource::iter()
        .find(|variant| <&'static str>::from(*variant) == member)
        .map(|variant| format!("ModelCatalogSource::{variant:?}"))
        .ok_or_else(|| GenError::UnmappableValue {
            field: entry.field,
            message: format!("`{member}` is not a ModelCatalogSource variant"),
        })
}

/// Emits the committed fragment text: a stable header plus one
/// `field: expr,` line per registry entry, indented to match the
/// hand-written `CLAUDE_INFO` initializer.
fn emit_fragment(slug: &str, fields: &[ResolvedField]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "// GENERATED by claudine-gen — DO NOT EDIT BY HAND.\n\
         //\n\
         // Walking-skeleton (Phase A1) data fragment for the `{slug}` provider:\n\
         // the mapped subset of the `CLAUDE_INFO` field expressions in\n\
         // `claudine/lib/src/provider/claude.rs`. Inputs: docs/providers.yaml,\n\
         // docs/providers/facts/{slug}.yaml, docs/providers/overrides/{slug}.yaml,\n\
         // and the research frontmatter named by the mapping registry.\n\
         // Regenerate with `claudine-gen generate`; drift-check with\n\
         // `claudine-gen check` (the same code path as the nextest drift test).\n"
    ));
    for field in fields {
        out.push_str(&format!("    {}: {},\n", field.field, field.expr));
    }
    out
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
    fn model_catalog_source_expr_maps_snake_members() {
        let entry = &REGISTRY[4];
        assert_eq!(
            model_catalog_source_expr(entry, "static").unwrap(),
            "ModelCatalogSource::Static"
        );
        assert_eq!(
            model_catalog_source_expr(entry, "opencode_cli").unwrap(),
            "ModelCatalogSource::OpencodeCli"
        );
        assert!(matches!(
            model_catalog_source_expr(entry, "telepathic"),
            Err(GenError::UnmappableValue { .. })
        ));
    }
}
