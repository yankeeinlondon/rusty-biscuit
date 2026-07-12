//! Property-name (key) enumeration and authored-order recovery.
//!
//! Produces the setter-name candidates (before `=`) and the authored
//! property-name ordering the completer sorts them by.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use biscuit_file::YamlValue;

use darkmatter::markdown::schemas::{
    Constraint, EffectiveSchema, PropertyAtom, PropertyDef, SchemaArm, SimplifiedSchema,
};

use crate::completion::fuzzy;
use crate::completion::scopes::ScopeContext;

use super::resolve_prompt_path;

/// Property name candidates for the cursor partial.
///
/// Returns names in priority order: required properties first (in declaration
/// order), then optional properties (in declaration order). `supplied` is the
/// set of property names already passed on the command line — those are
/// filtered out so the user is never offered to re-set the same key.
///
/// `declared_order` is the property-name sequence as authored in the prompt's
/// raw frontmatter YAML (or an empty slice when it could not be recovered).
/// When populated, candidates within each required/optional group are sorted
/// to match that authored order; names absent from the list keep their
/// `IndexMap` iteration position appended at the end of the group. This
/// breaks the underlying [`serde_json::Map`] alphabetisation that
/// Darkmatter's [`EffectiveSchema`] inherits when it stores nested
/// frontmatter values.
///
/// Each candidate is rendered with a trailing `=` so accepting it leaves the
/// cursor positioned to start typing the value.
pub(crate) fn property_names(
    effective: &EffectiveSchema,
    partial: &str,
    supplied: &HashSet<String>,
    declared_order: &[String],
) -> Vec<String> {
    let defs = collect_property_defs(effective);
    if defs.is_empty() {
        return Vec::new();
    }

    let mut required: Vec<(usize, String)> = Vec::new();
    let mut optional: Vec<(usize, String)> = Vec::new();
    for (iter_idx, (name, def)) in defs.into_iter().enumerate() {
        if supplied.contains(name) {
            continue;
        }
        if !name_matches(name, partial) {
            continue;
        }
        let rank = declaration_rank(declared_order, name, iter_idx);
        if is_required(def) {
            required.push((rank, format!("{name}=")));
        } else {
            optional.push((rank, format!("{name}=")));
        }
    }

    required.sort_by_key(|(rank, _)| *rank);
    optional.sort_by_key(|(rank, _)| *rank);

    required
        .into_iter()
        .chain(optional)
        .map(|(_, candidate)| candidate)
        .collect()
}

/// Sort key for a property: its index in the authored `declared_order` when
/// present, otherwise an offset past the end of that list (preserving the
/// underlying [`IndexMap`] iteration position so the relative order of
/// unknown names stays deterministic).
fn declaration_rank(declared_order: &[String], name: &str, iter_idx: usize) -> usize {
    declared_order
        .iter()
        .position(|n| n == name)
        .unwrap_or(declared_order.len() + iter_idx)
}

/// Authored property-name order for `file_arg`'s `$schema` declaration.
///
/// Re-parses the prompt's raw frontmatter YAML with `serde_yaml_ng` (whose
/// `Mapping` preserves insertion order) so the original authored sequence of
/// property names is recoverable. This is necessary because Darkmatter's
/// [`EffectiveSchema`] stores nested frontmatter values as
/// `serde_json::Value`, whose `Map` is alphabetised
/// ([`serde_json::Map`] is a [`BTreeMap`] unless the `preserve_order`
/// feature is enabled) — the declaration order is lost before
/// `parse_yaml_schema` ever sees it.
///
/// Handles three `$schema` shapes:
///
/// - **Inline mapping** → returns the mapping's keys in authored order.
/// - **String file reference** → loads the referenced file (YAML or JSON) and
///   returns the property-names from its `$schema` mapping (or its root
///   `properties` object for raw JSON Schema files).
/// - **Sequence (root union)** → returns an empty `Vec`; a root union has no
///   single authored property order, so callers fall back to the arm-merge
///   order that [`collect_property_defs`] produces (first-seen across arms).
///
/// Returns an empty `Vec` on any failure (file missing, frontmatter not
/// parseable, no `$schema`, unsupported shape). Callers treat the empty
/// case as "no authored ordering available" and fall back to whatever order
/// the [`SchemaShape::properties`] map provides.
///
/// [`BTreeMap`]: std::collections::BTreeMap
pub(crate) fn declared_property_order(file_arg: &str, ctx: &ScopeContext) -> Vec<String> {
    let Some(path) = resolve_prompt_path(file_arg, ctx) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Some(yaml) = extract_frontmatter_yaml(&text) else {
        return Vec::new();
    };
    let Ok(root) = biscuit_file::serde_yaml_ng::from_str::<YamlValue>(yaml) else {
        return Vec::new();
    };
    let Some(schema_value) = root
        .as_mapping()
        .and_then(|m| m.get(YamlValue::String("$schema".into())))
    else {
        return Vec::new();
    };
    let base_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    schema_keys_in_order(schema_value, &base_dir)
}

/// Splits the leading `---`-delimited frontmatter block out of a Markdown
/// document and returns its YAML body. Returns `None` when no frontmatter
/// block is present or the block is unterminated.
fn extract_frontmatter_yaml(text: &str) -> Option<&str> {
    let body = text.strip_prefix("---\n")?;
    let end = body.find("\n---\n").or_else(|| {
        body.strip_suffix("\n---")
            .map(|trimmed| trimmed.len())
    })?;
    Some(&body[..end])
}

/// Returns the property-name sequence for a `$schema` YAML value, following
/// file references as needed. Returns an empty `Vec` for shapes that have
/// no single ordered property set (root unions, unsupported types).
fn schema_keys_in_order(value: &YamlValue, base_dir: &Path) -> Vec<String> {
    match value {
        YamlValue::Mapping(map) => map
            .keys()
            .filter_map(|k| k.as_str().map(str::to_string))
            .collect(),
        YamlValue::String(reference) => referenced_schema_keys(reference, base_dir),
        _ => Vec::new(),
    }
}

/// Loads a `$schema` file reference (YAML or JSON), returning the authored
/// property-name order. Mirrors Darkmatter's disambiguation rule: YAML files
/// whose root holds a `$schema:` mapping are SimplifiedSchema documents
/// (the order comes from that nested mapping); everything else is a raw JSON
/// Schema (the order comes from its top-level `properties` object).
fn referenced_schema_keys(reference: &str, base_dir: &Path) -> Vec<String> {
    let candidate = base_dir.join(reference);
    let Ok(text) = std::fs::read_to_string(&candidate) else {
        return Vec::new();
    };
    let lower = candidate
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    if lower.as_deref() == Some("json") {
        return json_schema_property_keys(&text);
    }
    let Ok(root) = biscuit_file::serde_yaml_ng::from_str::<YamlValue>(&text) else {
        return Vec::new();
    };
    if let YamlValue::Mapping(map) = &root
        && let Some(inner) = map.get(YamlValue::String("$schema".into()))
        && let YamlValue::Mapping(inner_map) = inner
    {
        return inner_map
            .keys()
            .filter_map(|k| k.as_str().map(str::to_string))
            .collect();
    }
    yaml_root_property_keys(&root)
}

/// Property-name order for a raw JSON Schema file (whose `properties` object
/// is the source of truth).
fn json_schema_property_keys(text: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    value
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

/// Property-name order for a YAML JSON-Schema-shaped root (its `properties`
/// mapping, when present).
fn yaml_root_property_keys(value: &YamlValue) -> Vec<String> {
    let YamlValue::Mapping(root) = value else {
        return Vec::new();
    };
    let Some(props) = root.get(YamlValue::String("properties".into())) else {
        return Vec::new();
    };
    let YamlValue::Mapping(map) = props else {
        return Vec::new();
    };
    map.keys()
        .filter_map(|k| k.as_str().map(str::to_string))
        .collect()
}

fn name_matches(name: &str, partial: &str) -> bool {
    if partial.is_empty() {
        return true;
    }
    // Use fuzzy subsequence matching to match the rest of the engine's
    // partial-classification semantics. Short partials behave as starting
    // substrings under `fuzzy_match` (which lowercases both sides).
    fuzzy::fuzzy_match(name, partial)
}

/// Collect every property `(name, def)` a schema exposes for setter-name
/// completion.
///
/// A [`SimplifiedSchema::Single`] yields its shape's properties in declaration
/// order. A [`SimplifiedSchema::Union`] (root-level union) yields the union of
/// every inline arm's properties, deduplicated by name in first-seen (arm)
/// order, so a `spec`-or-`design` root union offers both names. Unresolved
/// [`SchemaArm::FileRef`] arms are skipped.
fn collect_property_defs(effective: &EffectiveSchema) -> Vec<(&String, &PropertyDef)> {
    let Some(simplified) = effective.simplified.as_ref() else {
        return Vec::new();
    };
    match simplified {
        SimplifiedSchema::Single(shape) => shape.properties.iter().collect(),
        SimplifiedSchema::Union(arms) => {
            let mut out: Vec<(&String, &PropertyDef)> = Vec::new();
            let mut seen: HashSet<&str> = HashSet::new();
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    for (name, def) in shape.properties.iter() {
                        if seen.insert(name.as_str()) {
                            out.push((name, def));
                        }
                    }
                }
            }
            out
        }
    }
}

fn is_required(def: &PropertyDef) -> bool {
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(atom) => vec![atom],
        PropertyDef::Union(items) => items.iter().collect(),
    };
    atoms.iter().any(|atom| {
        atom.constraints
            .iter()
            .any(|c| matches!(c, Constraint::Required))
    })
}
