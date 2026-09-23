//! Passive, schema-aware resolution of authored frontmatter properties.
//!
//! This is the one authority for "which [`PropertyDef`] governs the value at
//! this frontmatter key path" and "which frontmatter values are
//! Expression-typed". DMLS uses it for completion, hover, and `dm.expression.*`
//! diagnostics; the dasherized-identifier corpus gate
//! (`lib/tests/dasherized_identifier_corpus.rs`) uses it to discover the
//! schema-typed expressions it must parse. Both therefore agree on the
//! classification by construction.
//!
//! Everything here is passive: it reads already-resolved schema shapes and
//! already-parsed frontmatter, performs no I/O, and never executes an
//! expression.
//!
//! Only literal properties are consulted. Pattern-dictionary keys and trigger
//! payloads (which layer into the compiled JSON Schema, not into a
//! [`SimplifiedSchema`]) contribute no property definitions here.

use std::borrow::Cow;

use serde_json::Value;

use super::{
    PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr,
    darkmatter_base_schema_ref, select_literal_discriminant_arm,
};

/// The effective top-level property shape for one document: the Darkmatter
/// base properties, overlaid with each extension baseline shape in order, then
/// the document's own `$schema` (document > extension > base, the compose
/// precedence).
///
/// A root `$schema` union overlays the arm its shared literal discriminant
/// selects in `frontmatter`; when no arm is unambiguously selected, every
/// inline arm's properties merge, and a key contributed by more than one arm
/// resolves to the union of those arms' atoms. A file-reference arm
/// contributes no properties.
///
/// `frontmatter` is the authored frontmatter mapping as JSON; pass
/// [`Value::Null`] when none is available, which disables discriminant
/// narrowing.
pub fn effective_property_shape(
    extension_shapes: &[SchemaShape],
    document: Option<&SimplifiedSchema>,
    frontmatter: &Value,
) -> SchemaShape {
    let mut shape = match darkmatter_base_schema_ref() {
        SimplifiedSchema::Single(shape) => shape.clone(),
        SimplifiedSchema::Union(_) => SchemaShape::default(),
    };
    for extension in extension_shapes {
        for (name, def) in &extension.properties {
            shape.properties.insert(name.clone(), def.clone());
        }
    }
    match document {
        Some(SimplifiedSchema::Single(document)) => {
            for (name, def) in &document.properties {
                shape.properties.insert(name.clone(), def.clone());
            }
        }
        Some(SimplifiedSchema::Union(arms)) => overlay_root_union(&mut shape, arms, frontmatter),
        None => {}
    }
    shape
}

/// The [`PropertyDef`] at a full key `path` (ancestor segments, then the leaf
/// key), resolved against `root` and the authored `frontmatter`.
///
/// Each ancestor that is a discriminated inline-object union descends into the
/// arm its authored literal discriminant selects; when narrowing is
/// unavailable, the ancestor's inline-object arms merge, so a leaf present in
/// several arms with divergent types resolves to the union of their atoms.
/// The result borrows for a top-level leaf and is owned for a nested one,
/// because a merged leaf must be owned.
pub fn property_def_at_path<'a>(
    root: &'a SchemaShape,
    frontmatter: &Value,
    path: &[&str],
) -> Option<Cow<'a, PropertyDef>> {
    let (leaf, ancestors) = path.split_last()?;
    if ancestors.is_empty() {
        return root.properties.get(*leaf).map(Cow::Borrowed);
    }
    let shape = nested_property_shape(root, frontmatter, ancestors)?;
    shape.properties.get(*leaf).cloned().map(Cow::Owned)
}

/// The nested [`SchemaShape`] reached by walking `ancestors` from `root`, with
/// the same arm selection as [`property_def_at_path`]. An empty path yields
/// `root`; `None` when a segment is absent or has no inline-object arm.
pub fn nested_property_shape(
    root: &SchemaShape,
    frontmatter: &Value,
    ancestors: &[&str],
) -> Option<SchemaShape> {
    let mut shape = root.clone();
    for depth in 0..ancestors.len() {
        let def = shape.properties.get(ancestors[depth])?;
        let path = &ancestors[..=depth];
        let next = match discriminated_arm_shape(def, frontmatter, path) {
            Some(selected) => selected.clone(),
            None => merged_inline_object_shape(def)?,
        };
        shape = next;
    }
    Some(shape)
}

/// The first `expression`-typed arm of a property, so a union whose
/// expression arm is not first is still recognized.
pub fn expression_atom(def: &PropertyDef) -> Option<&PropertyAtom> {
    atoms_of(def)
        .iter()
        .find(|atom| matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::Expression)))
}

/// One Expression-typed scalar frontmatter value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterExpressionValue {
    /// The decoded mapping-key path from the frontmatter root to the value.
    pub path: Vec<String>,
    /// The expression text compose's schema pass sees: a string verbatim, a
    /// native boolean or number in its canonical JSON spelling (the coercion
    /// the `expression` format applies).
    pub expression: String,
}

/// Every Expression-typed **scalar** value in `frontmatter`, in mapping order,
/// classified through [`property_def_at_path`] and [`expression_atom`].
///
/// Only mapping keys form paths, so a value inside a sequence is never
/// reported, and a mapping or sequence value on an expression property stays a
/// schema type mismatch rather than an expression. A `null` value is absence,
/// not an expression. A root `$schema` key is the schema control key, not
/// document data, and is skipped.
///
/// ## Examples
///
/// ```rust
/// use darkmatter::markdown::schemas::{
///     effective_property_shape, frontmatter_expression_values, parse_yaml_schema,
/// };
///
/// let schema: serde_yaml_ng::Value =
///     serde_yaml_ng::from_str("gate: expression\nlabel: string").unwrap();
/// let schema = parse_yaml_schema(&schema).unwrap();
/// let frontmatter = serde_json::json!({ "gate": "phase-2 > 0", "label": "x" });
/// let shape = effective_property_shape(&[], Some(&schema), &frontmatter);
///
/// let values = frontmatter_expression_values(&shape, &frontmatter);
/// assert_eq!(values.len(), 1);
/// assert_eq!(values[0].path, ["gate"]);
/// assert_eq!(values[0].expression, "phase-2 > 0");
/// ```
pub fn frontmatter_expression_values(
    shape: &SchemaShape,
    frontmatter: &Value,
) -> Vec<FrontmatterExpressionValue> {
    let mut out = Vec::new();
    let Some(root) = frontmatter.as_object() else {
        return out;
    };
    let mut path = Vec::new();
    for (key, value) in root {
        if key == "$schema" {
            continue;
        }
        path.push(key.as_str());
        collect_expression_values(shape, frontmatter, &mut path, value, &mut out);
        path.pop();
    }
    out
}

fn collect_expression_values<'v>(
    shape: &SchemaShape,
    frontmatter: &Value,
    path: &mut Vec<&'v str>,
    value: &'v Value,
    out: &mut Vec<FrontmatterExpressionValue>,
) {
    let expression = match value {
        Value::Object(map) => {
            for (key, child) in map {
                path.push(key.as_str());
                collect_expression_values(shape, frontmatter, path, child, out);
                path.pop();
            }
            return;
        }
        Value::Null | Value::Array(_) => return,
        Value::String(text) => text.clone(),
        Value::Bool(_) | Value::Number(_) => value.to_string(),
    };
    let is_expression = property_def_at_path(shape, frontmatter, path)
        .is_some_and(|def| expression_atom(&def).is_some());
    if is_expression {
        out.push(FrontmatterExpressionValue {
            path: path.iter().map(|segment| (*segment).to_string()).collect(),
            expression,
        });
    }
}

fn atoms_of(def: &PropertyDef) -> &[PropertyAtom] {
    match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    }
}

fn overlay_root_union(shape: &mut SchemaShape, arms: &[SchemaArm], frontmatter: &Value) {
    let arm_json: Vec<Value> = arms.iter().map(root_arm_discriminant_json).collect();
    match select_literal_discriminant_arm(&arm_json, frontmatter) {
        Some(index) => {
            if let SchemaArm::Inline(arm_shape) = &arms[index] {
                for (name, def) in &arm_shape.properties {
                    shape.properties.insert(name.clone(), def.clone());
                }
            }
        }
        // The merge is across arms only; the merged document shape still takes
        // precedence over the base/extension baseline it overlays.
        None => {
            for (name, def) in merged_root_arm_shape(arms).properties {
                shape.properties.insert(name, def);
            }
        }
    }
}

fn merged_root_arm_shape(arms: &[SchemaArm]) -> SchemaShape {
    let mut merged = SchemaShape::default();
    for arm in arms {
        let SchemaArm::Inline(arm_shape) = arm else {
            continue;
        };
        merge_properties_into(&mut merged, arm_shape);
    }
    merged
}

/// Aligned by index with the arm slice so the selector's returned index maps
/// back; a file-reference arm still occupies its slot.
fn root_arm_discriminant_json(arm: &SchemaArm) -> Value {
    match arm {
        SchemaArm::Inline(shape) => shape_discriminant_json(shape),
        SchemaArm::FileRef(_) => serde_json::json!({}),
    }
}

/// Projects a property-union arm to the minimal JSON the discriminant selector
/// reads; a non-inline-object arm keeps its slot so indexes stay aligned with
/// [`atoms_of`].
fn arm_discriminant_json(atom: &PropertyAtom) -> Value {
    match &atom.ty {
        TypeExpr::InlineObject(shape) => shape_discriminant_json(shape),
        _ => serde_json::json!({}),
    }
}

/// `{ "properties": { key: { "const": <value> } } }` for each of the shape's
/// `literal(...)`-typed properties.
fn shape_discriminant_json(shape: &SchemaShape) -> Value {
    let mut props = serde_json::Map::new();
    for (key, def) in &shape.properties {
        if let PropertyDef::Single(child) = def
            && let Some(value) = child.literal_value()
        {
            props.insert(key.clone(), serde_json::json!({ "const": value }));
        }
    }
    serde_json::json!({ "properties": props })
}

/// The instance is the already-parsed, correctly-typed frontmatter mapping at
/// `path`, so a string `'2'` never matches a numeric `literal(2)`.
fn discriminated_arm_shape<'a>(
    def: &'a PropertyDef,
    frontmatter: &Value,
    path: &[&str],
) -> Option<&'a SchemaShape> {
    let atoms = atoms_of(def);
    if atoms.len() < 2 {
        return None;
    }
    let arms: Vec<Value> = atoms.iter().map(arm_discriminant_json).collect();
    let instance = navigate_json(frontmatter, path)?;
    let index = select_literal_discriminant_arm(&arms, instance)?;
    match &atoms[index].ty {
        TypeExpr::InlineObject(shape) => Some(shape),
        _ => None,
    }
}

fn navigate_json<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = current.as_object()?.get(*segment)?;
    }
    Some(current)
}

/// `None` when the property has no inline-object arm.
fn merged_inline_object_shape(def: &PropertyDef) -> Option<SchemaShape> {
    let mut merged: Option<SchemaShape> = None;
    for atom in atoms_of(def) {
        let TypeExpr::InlineObject(inner) = &atom.ty else {
            continue;
        };
        merge_properties_into(merged.get_or_insert_with(SchemaShape::default), inner);
    }
    merged
}

/// Keys keep arm-declaration then property-declaration order; a key already
/// present becomes the union of both definitions' atoms.
fn merge_properties_into(merged: &mut SchemaShape, arm: &SchemaShape) {
    for (name, child) in &arm.properties {
        let combined = match merged.properties.get(name) {
            Some(existing) => merge_defs(existing, child),
            None => child.clone(),
        };
        merged.properties.insert(name.clone(), combined);
    }
}

/// `existing`'s atoms followed by any of `incoming`'s not already present,
/// collapsing to [`PropertyDef::Single`] when exactly one survives.
fn merge_defs(existing: &PropertyDef, incoming: &PropertyDef) -> PropertyDef {
    let mut atoms: Vec<PropertyAtom> = atoms_of(existing).to_vec();
    for atom in atoms_of(incoming) {
        if !atoms.contains(atom) {
            atoms.push(atom.clone());
        }
    }
    match atoms.len() {
        1 => PropertyDef::Single(atoms.into_iter().next().expect("one atom")),
        _ => PropertyDef::Union(atoms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(yaml: &str) -> SimplifiedSchema {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("fixture YAML");
        super::super::parse_yaml_schema(&value).expect("fixture schema parses")
    }

    fn paths(values: &[FrontmatterExpressionValue]) -> Vec<String> {
        values.iter().map(|value| value.path.join(".")).collect()
    }

    #[test]
    fn nested_union_and_native_expression_values_are_discovered() {
        let document = schema(
            "gate: expression\nmode:\n  - string\n  - expression\nnested:\n  inner:\n    check: expression\n    note: string\n",
        );
        let frontmatter = serde_json::json!({
            "gate": true,
            "mode": "a && b",
            "nested": { "inner": { "check": "phase-2 > 0", "note": "phase-2 > 0" } },
            "absent": null,
        });
        let shape = effective_property_shape(&[], Some(&document), &frontmatter);

        let values = frontmatter_expression_values(&shape, &frontmatter);

        assert_eq!(paths(&values), ["gate", "mode", "nested.inner.check"]);
        assert_eq!(values[0].expression, "true");
    }

    #[test]
    fn sequences_nulls_mappings_and_the_schema_key_are_not_expressions() {
        let document = schema("gate: expression\nlist: expression[]\n");
        let frontmatter = serde_json::json!({
            "$schema": { "gate": "expression" },
            "gate": null,
            "list": ["a", "b"],
        });
        let shape = effective_property_shape(&[], Some(&document), &frontmatter);

        assert!(frontmatter_expression_values(&shape, &frontmatter).is_empty());
    }

    #[test]
    fn a_selected_discriminated_arm_governs_the_nested_key() {
        let document = schema(
            "step:\n  - \"{ kind: literal(check), run: expression }\"\n  - \"{ kind: literal(note), run: string }\"\n",
        );
        let check = serde_json::json!({ "step": { "kind": "check", "run": "x" } });
        let note = serde_json::json!({ "step": { "kind": "note", "run": "x" } });
        let shape = effective_property_shape(&[], Some(&document), &check);

        assert_eq!(paths(&frontmatter_expression_values(&shape, &check)), ["step.run"]);
        assert!(frontmatter_expression_values(&shape, &note).is_empty());
    }
}
