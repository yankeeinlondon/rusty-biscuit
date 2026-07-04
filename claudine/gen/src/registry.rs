//! The mapping registry: catalog field → declared source → coercion.
//!
//! This is the walking-skeleton (Phase A1) subset — six entries spanning
//! every source kind in the field-source matrix
//! (`design/catalog-generation.md`). Phase B extends it to every
//! `ProviderInfo` field. Entry order is emission order and must follow the
//! field order of the hand-written `CLAUDE_INFO` constant so byte
//! comparison stays line-oriented.

use claudine_catalog_types::ModelCatalogSource;
use serde_json::{Value, json};
use strum::VariantNames;

/// Which input file owns a field's value (the field-source matrix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredSource {
    /// Identity fact from `docs/providers.yaml`.
    Roster { key: &'static str },
    /// Topic-less fact from `docs/providers/facts/<slug>.yaml`.
    Facts { key: &'static str },
    /// Research frontmatter from `docs/research/<topic>/<slug>.md`.
    Research {
        topic: &'static str,
        /// Dot-separated path into the frontmatter / sidecar shape.
        path: &'static str,
    },
}

impl DeclaredSource {
    /// Short source-kind label used in collision errors and `--mapping`.
    pub fn kind(&self) -> &'static str {
        match self {
            DeclaredSource::Roster { .. } => "roster",
            DeclaredSource::Facts { .. } => "facts",
            DeclaredSource::Research { .. } => "research",
        }
    }
}

/// Acceptable sidecar/value shapes for a field, checked BEFORE any value
/// mapping. A field lists alternatives (OR); a shape that matches an
/// alternative's kind but violates its constraint (enum subset) is a loud
/// error, never a silent fall-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaExpectation {
    /// A plain string scalar.
    String,
    /// A boolean (sidecar `boolean` or `boolish`).
    Boolean,
    /// An `enum(...)` feeding a Rust enum: the sidecar members must be a
    /// subset of the strum-introspected variant names.
    EnumSubsetOf {
        rust_enum: &'static str,
        variants: &'static [&'static str],
    },
    /// An array of inline-object records carrying at least these fields.
    RecordArray {
        required_fields: &'static [&'static str],
    },
}

impl SchemaExpectation {
    /// Label used in error messages and `--mapping` output.
    pub fn label(&self) -> String {
        match self {
            SchemaExpectation::String => "string".into(),
            SchemaExpectation::Boolean => "boolean".into(),
            SchemaExpectation::EnumSubsetOf { rust_enum, .. } => {
                format!("enum(subset of {rust_enum})")
            }
            SchemaExpectation::RecordArray { required_fields } => {
                format!("record_array(requires {})", required_fields.join(", "))
            }
        }
    }
}

/// How a source value becomes a Rust field expression.
///
/// Each coercion has two halves: a research/source-side *extraction* that
/// produces a catalog-shaped intermediate JSON value, and a shared
/// *expression* half that turns a catalog-shaped value into Rust text.
/// Overrides supply catalog-shaped values directly, so they flow through
/// the expression half only — override and source values are compared in
/// the same shape for the staleness lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coercion {
    /// String scalar → `&'static str` literal.
    StringLiteral,
    /// Boolean → `bool` literal.
    BoolLiteral,
    /// agent-models `dynamic_listing.available` → [`ModelCatalogSource`]
    /// path expression. `false` maps to `static`; `true` is unmappable in
    /// the skeleton (selecting a dynamic variant needs Phase B data) and
    /// demands an override.
    DynamicListingToModelCatalogSource,
    /// agent-models `model_selection[]` records → the list of bare env-var
    /// identifiers (`method == "env_var"` and `site` is a single
    /// `[A-Z][A-Z0-9_]*` token; compound "A / B" sites are excluded),
    /// document order, deduplicated.
    EnvVarSitesToStringSlice,
}

impl Coercion {
    /// snake_case label for `--mapping` output.
    pub fn label(&self) -> &'static str {
        match self {
            Coercion::StringLiteral => "string_literal",
            Coercion::BoolLiteral => "bool_literal",
            Coercion::DynamicListingToModelCatalogSource => {
                "dynamic_listing_to_model_catalog_source"
            }
            Coercion::EnvVarSitesToStringSlice => "env_var_sites_to_string_slice",
        }
    }
}

/// One mapping-registry row: a `ProviderInfo` field and where its value
/// comes from.
#[derive(Debug, Clone, Copy)]
pub struct RegistryEntry {
    /// `ProviderInfo` field name (also the key overrides and facts use).
    pub field: &'static str,
    pub source: DeclaredSource,
    pub expected: &'static [SchemaExpectation],
    pub coercion: Coercion,
}

/// The A1 mapping registry, in `CLAUDE_INFO` field order.
pub const REGISTRY: &[RegistryEntry] = &[
    RegistryEntry {
        field: "slug",
        source: DeclaredSource::Roster { key: "slug" },
        expected: &[SchemaExpectation::String],
        coercion: Coercion::StringLiteral,
    },
    RegistryEntry {
        field: "binary",
        source: DeclaredSource::Roster { key: "binary" },
        expected: &[SchemaExpectation::String],
        coercion: Coercion::StringLiteral,
    },
    RegistryEntry {
        field: "agent_offset",
        source: DeclaredSource::Roster { key: "repo_dir" },
        expected: &[SchemaExpectation::String],
        coercion: Coercion::StringLiteral,
    },
    RegistryEntry {
        field: "supports_skills",
        source: DeclaredSource::Facts {
            key: "supports_skills",
        },
        expected: &[SchemaExpectation::Boolean],
        coercion: Coercion::BoolLiteral,
    },
    RegistryEntry {
        field: "dynamic_source",
        source: DeclaredSource::Research {
            topic: "agent-models",
            path: "dynamic_listing.available",
        },
        // Boolean today; the enum arm admits a future sidecar that reports
        // the catalog source directly — its members must then be a subset
        // of the Rust variants (the schema<->catalog compatibility gate).
        expected: &[
            SchemaExpectation::Boolean,
            SchemaExpectation::EnumSubsetOf {
                rust_enum: "ModelCatalogSource",
                variants: ModelCatalogSource::VARIANTS,
            },
        ],
        coercion: Coercion::DynamicListingToModelCatalogSource,
    },
    RegistryEntry {
        field: "model_env_vars",
        source: DeclaredSource::Research {
            topic: "agent-models",
            path: "model_selection",
        },
        expected: &[SchemaExpectation::RecordArray {
            required_fields: &["method", "site"],
        }],
        coercion: Coercion::EnvVarSitesToStringSlice,
    },
];

/// Looks up a registry entry by field name.
pub fn entry_for(field: &str) -> Option<&'static RegistryEntry> {
    REGISTRY.iter().find(|entry| entry.field == field)
}

/// Serializes the registry as the `--mapping` JSON document.
pub fn mapping_json() -> Value {
    let fields: Vec<Value> = REGISTRY
        .iter()
        .map(|entry| {
            let source = match entry.source {
                DeclaredSource::Roster { key } => json!({ "kind": "roster", "key": key }),
                DeclaredSource::Facts { key } => json!({ "kind": "facts", "key": key }),
                DeclaredSource::Research { topic, path } => {
                    json!({ "kind": "research", "topic": topic, "path": path })
                }
            };
            json!({
                "field": entry.field,
                "source": source,
                "expected": entry
                    .expected
                    .iter()
                    .map(SchemaExpectation::label)
                    .collect::<Vec<_>>(),
                "coercion": entry.coercion.label(),
            })
        })
        .collect();
    json!({
        "provider_scope": "claude",
        "fields": fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_fields_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for entry in REGISTRY {
            assert!(seen.insert(entry.field), "duplicate field {}", entry.field);
        }
    }

    #[test]
    fn mapping_json_covers_every_entry() {
        let value = mapping_json();
        assert_eq!(value["fields"].as_array().unwrap().len(), REGISTRY.len());
        assert_eq!(value["provider_scope"], "claude");
    }
}
