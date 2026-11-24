//! Exports: the lowered JSON Schema, and the authored SimplifiedSchema
//! document reassembled from the type serializer.
//!
//! The YAML export writes the declaration as authored (named types by name,
//! never expanded), so a recursive or shared named type is a finite
//! reference graph in the output, as in the input. Nested objects are
//! written as block mappings and property unions as YAML sequences, the two
//! forms that keep every pattern key, `$constraints` entry, and per-arm
//! description. Anything the serializer would have to drop is refused with
//! [`SimplifiedSchemaError::ExportUnsupported`] instead.
//!
//! Composition is written as authored, never as its merged members: a
//! mapping's `$extends` keeps its scalar or sequence form, and an inline
//! object with `...Name` spreads stays one type-expression string.

use std::sync::Arc;

use serde_json::Value;
use serde_yaml_ng::{Mapping, Value as YamlValue};

use super::layer::{Authored, AuthoredSchema};
use super::{SimplifiedSchema, SimplifiedSchemaError};
use crate::simplified::serialize::{composition_operand_text, pattern_key_text, serialize_property_atom};
use crate::simplified::standalone::{visit_atom, visit_def};
use crate::simplified::{
    Constraint, Extends, ObjectComposition, PropertyAtom, PropertyDef, SchemaArm, SchemaCategory, SchemaDeclaration,
    SchemaRoot, SchemaShape, StandaloneSchemaEnvelope, TypeExpr,
};

impl SimplifiedSchema {
    /// The schema lowered to Draft 2020-12 JSON Schema: the shared lowered
    /// form validation uses (an `Arc` clone, not a copy), with its metadata,
    /// vocabulary, and ordering.
    ///
    /// ## Errors
    ///
    /// [`SimplifiedSchemaError::Unprepared`] with the lowerer's located error
    /// when the schema has no JSON Schema form (a function-typed property),
    /// or names a file an in-memory schema cannot read, and
    /// [`SimplifiedSchemaError::Unresolved`] for a template whose `{{ … }}`
    /// slots are not resolved.
    pub fn to_json_schema(&self) -> Result<Arc<Value>, SimplifiedSchemaError> {
        self.lowered().map_err(SimplifiedSchemaError::from_lowering)
    }

    /// The schema as one SimplifiedSchema YAML document, in its authored
    /// envelope (`$schema` with optional `types`, or `kind: schema`), laid
    /// out with [`biscuit_file::Indentation::default`] (four spaces per
    /// level). The string has no destination; a caller that writes it to a
    /// file and wants that file's indentation re-lays it.
    ///
    /// A layered schema exports as the receiver's document with the
    /// properties merged in from the other layers. A template exports as
    /// authored: each `{{ … }}` slot is written back as its expression and
    /// nothing is evaluated.
    ///
    /// ## Errors
    ///
    /// [`SimplifiedSchemaError::ExportUnsupported`] for raw JSON Schema
    /// (there is no reverse conversion), for a layered schema whose layers do
    /// not combine into one document, and for a declaration the serializer
    /// cannot write without losing part of it.
    pub fn to_simplified_yaml(&self) -> Result<String, SimplifiedSchemaError> {
        let document = match self.definition.layers.as_slice() {
            [layer] => match &layer.authored {
                Authored::Simplified(authored) => write_document(authored)?,
                Authored::JsonSchema => return Err(unsupported("raw JSON Schema has no SimplifiedSchema form")),
            },
            layers => write_layered(layers)?,
        };
        biscuit_file::to_yaml_string(&YamlValue::Mapping(document), biscuit_file::Indentation::default())
            .map_err(|error| unsupported(&error.to_string()))
    }
}

fn unsupported(reason: &str) -> SimplifiedSchemaError {
    SimplifiedSchemaError::ExportUnsupported { reason: reason.to_string() }
}

/// The layers merged with whole-property replacement, highest first: the top
/// layer's document, plus each lower layer's properties no higher layer
/// declares. A copied property may not refer to named types, which resolve
/// in the layer that declared them, nor compose an object.
///
/// No layer's root may compose: one flattened root cannot say which members
/// each layer's operands contributed, or in which scope they resolve.
fn write_layered(layers: &[Arc<super::layer::Layer>]) -> Result<Mapping, SimplifiedSchemaError> {
    let mut shapes = Vec::with_capacity(layers.len());
    for layer in layers {
        let Authored::Simplified(authored) = &layer.authored else {
            return Err(unsupported("a layer is raw JSON Schema, which has no SimplifiedSchema form"));
        };
        let Some(SchemaRoot::Single(shape)) = authored.schema() else {
            return Err(unsupported("only object-schema layers combine into one document"));
        };
        if shape.composition.is_some() {
            return Err(unsupported(
                "a layer's root composes another object, which a single merged document cannot express",
            ));
        }
        shapes.push((authored, shape));
    }
    let (top, top_shape) = shapes[0];
    let mut merged = top_shape.clone();
    for (_, shape) in &shapes[1..] {
        for (name, def) in &shape.properties {
            if merged.properties.contains_key(name) {
                continue;
            }
            if refers_to_named_type(def) {
                return Err(unsupported(&format!(
                    "`{name}` comes from a lower layer and refers to that layer's named types"
                )));
            }
            if composes(def) {
                return Err(unsupported(&format!(
                    "`{name}` comes from a lower layer and composes an object from that layer's named types"
                )));
            }
            merged.properties.insert(name.clone(), def.clone());
        }
    }
    let combined = AuthoredSchema {
        declaration: SchemaDeclaration::Schema(SchemaRoot::Single(merged)),
        local_types: top.local_types.clone(),
        envelope: top.envelope,
        categories: top.categories.clone(),
        selected: top.selected.clone(),
    };
    write_document(&combined)
}

fn composes(def: &PropertyDef) -> bool {
    let mut found = false;
    visit_def(def, &mut |atom| {
        found |= matches!(&atom.ty, TypeExpr::InlineObject(shape) if shape.composition.is_some());
    });
    found
}

fn refers_to_named_type(def: &PropertyDef) -> bool {
    let mut found = false;
    visit_def(def, &mut |atom| found |= matches!(atom.ty, TypeExpr::Imported { .. } | TypeExpr::Ref(_)));
    found
}

/// The document for `authored` in its envelope.
fn write_document(authored: &AuthoredSchema) -> Result<Mapping, SimplifiedSchemaError> {
    let mut document = Mapping::new();
    match authored.envelope {
        StandaloneSchemaEnvelope::Pure => {
            if let Some(categories) = &authored.categories {
                document.insert("categories".into(), write_categories(categories));
            }
            document.insert("$schema".into(), write_declaration(authored)?);
            if let Some(types) = &authored.local_types {
                document.insert("types".into(), YamlValue::Mapping(write_shape(types)?));
            }
        }
        StandaloneSchemaEnvelope::YamlSchema => {
            // A `yaml-schema` always exports `$schema`.
            document.insert("kind".into(), "yaml-schema".into());
            if let Some(categories) = &authored.categories {
                document.insert("categories".into(), write_categories(categories));
            }
            if let Some(types) = &authored.local_types {
                document.insert("types".into(), YamlValue::Mapping(write_shape(types)?));
            }
            document.insert("$schema".into(), write_declaration(authored)?);
        }
        // The factories refuse a namespace schema, so none is ever exported.
        StandaloneSchemaEnvelope::NamespaceSchema => {
            return Err(crate::errors::SchemaError::NotASchemaDocument {
                path: std::path::PathBuf::new(),
                reason: crate::errors::NotSchemaReason::NamespaceCatalog,
            }
            .into());
        }
        StandaloneSchemaEnvelope::Tagged => {
            document.insert("kind".into(), "schema".into());
            if let Some(categories) = &authored.categories {
                document.insert("categories".into(), write_categories(categories));
            }
            match &authored.local_types {
                // An exported `$schema` beside its `types` table.
                Some(types) => {
                    document.insert("types".into(), YamlValue::Mapping(write_shape(types)?));
                    document.insert("$schema".into(), write_declaration(authored)?);
                }
                // The `types` table is the payload.
                None => {
                    document.insert("types".into(), write_declaration(authored)?);
                }
            }
        }
    }
    Ok(document)
}

fn write_categories(categories: &[SchemaCategory]) -> YamlValue {
    YamlValue::Sequence(
        categories
            .iter()
            .map(|category| match &category.description {
                Some(description) => format!("{} -> {description}", category.name).into(),
                None => category.name.clone().into(),
            })
            .collect(),
    )
}

fn write_declaration(authored: &AuthoredSchema) -> Result<YamlValue, SimplifiedSchemaError> {
    match &authored.declaration {
        SchemaDeclaration::Reference(reference) => Ok(reference.file_reference().raw().into()),
        // A selected named object composed with `...` spreads has no mapping
        // form; the root extends the selection, whose inline expression is
        // in the `types` table written beside it.
        SchemaDeclaration::Schema(SchemaRoot::Single(shape))
            if matches!(shape.composition.as_deref(), Some(ObjectComposition::Inline(_))) =>
        {
            Ok(YamlValue::Mapping(write_selected_root(authored.selected.as_deref())?))
        }
        SchemaDeclaration::Schema(SchemaRoot::Single(shape)) => Ok(YamlValue::Mapping(write_shape(shape)?)),
        SchemaDeclaration::Schema(SchemaRoot::Union(arms)) => arms
            .iter()
            .map(|arm| match arm {
                SchemaArm::Inline(shape) => write_shape(shape).map(YamlValue::Mapping),
                SchemaArm::FileRef(reference) => Ok(reference.as_str().into()),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(YamlValue::Sequence),
    }
}

/// `{ $extends: "Name@this" }` for the selected type `Name`, refused when
/// `Name` cannot be written as a composition operand.
fn write_selected_root(selected: Option<&str>) -> Result<Mapping, SimplifiedSchemaError> {
    let Some(name) = selected else {
        return Err(unsupported("an inline object with `...` spreads has no mapping form for a schema root"));
    };
    let operand = format!("{name}@this");
    let parsed = crate::simplified::grammar::parse_extends_operand("$extends", &operand);
    if !parsed.is_ok_and(|parsed| parsed.name == name) {
        return Err(unsupported(&format!(
            "the selected type `{name}` composes with `...` spreads, and its name cannot be written as the \
             `$extends` operand of an exported root"
        )));
    }
    let mut mapping = Mapping::new();
    mapping.insert("$extends".into(), operand.into());
    Ok(mapping)
}

/// A block-form object: `$extends`, literal properties, pattern keys, then
/// `$constraints`.
///
/// An inline object with `...` spreads never reaches here (see
/// [`write_atom`]); its spread positions have no mapping form.
fn write_shape(shape: &SchemaShape) -> Result<Mapping, SimplifiedSchemaError> {
    let mut mapping = Mapping::new();
    match shape.composition.as_deref() {
        Some(ObjectComposition::Extends(Extends::Scalar(operand))) => {
            mapping.insert("$extends".into(), composition_operand_text(operand).into());
        }
        Some(ObjectComposition::Extends(Extends::Sequence(operands))) => {
            let operands = operands.iter().map(|operand| composition_operand_text(operand).into()).collect();
            mapping.insert("$extends".into(), YamlValue::Sequence(operands));
        }
        Some(ObjectComposition::Inline(_)) => {
            return Err(unsupported("an inline object with `...` spreads has no block mapping form"));
        }
        None => {}
    }
    for (name, def) in &shape.properties {
        mapping.insert(name.as_str().into(), write_def(def)?);
    }
    for pattern in &shape.pattern_keys {
        mapping.insert(pattern_key_text(&pattern.key).into(), write_def(&pattern.def)?);
    }
    if !shape.constraints.is_empty() {
        mapping.insert("$constraints".into(), YamlValue::Mapping(write_object_constraints(&shape.constraints)?));
    }
    Ok(mapping)
}

/// A block object's `$constraints` mapping; the inverse of the parser's.
fn write_object_constraints(constraints: &[Constraint]) -> Result<Mapping, SimplifiedSchemaError> {
    let mut mapping = Mapping::new();
    for constraint in constraints {
        let (key, value): (&str, YamlValue) = match constraint {
            Constraint::MinKeys(count) => ("min-keys", (*count as u64).into()),
            Constraint::MaxKeys(count) => ("max-keys", (*count as u64).into()),
            Constraint::MinItems(count) => ("min-items", (*count as u64).into()),
            Constraint::MaxItems(count) => ("max-items", (*count as u64).into()),
            Constraint::UncountedKeys(keys) => (
                "uncounted-keys",
                YamlValue::Sequence(keys.iter().map(|key| key.as_str().into()).collect()),
            ),
            Constraint::Required => ("required", true.into()),
            Constraint::NotEmpty => ("not-empty", true.into()),
            Constraint::Unique => ("unique", true.into()),
            Constraint::Generated => ("generated", true.into()),
            Constraint::Integer => ("integer", true.into()),
            Constraint::Closed => ("closed", true.into()),
            other => {
                return Err(unsupported(&format!(
                    "the object constraint `{}` has no `$constraints` spelling",
                    other.to_schema_text(None)
                )));
            }
        };
        mapping.insert(key.into(), value);
    }
    Ok(mapping)
}

/// One property: a nested block object, a sequence of union arms, or a type
/// expression string. A union arm is a string unless its object composes
/// with `$extends`, which only a mapping arm can write.
fn write_def(def: &PropertyDef) -> Result<YamlValue, SimplifiedSchemaError> {
    match def {
        PropertyDef::Single(atom) => write_atom(atom),
        PropertyDef::Union(atoms) => atoms
            .iter()
            .map(|atom| match &atom.ty {
                TypeExpr::InlineObject(shape)
                    if matches!(shape.composition.as_deref(), Some(ObjectComposition::Extends(_))) =>
                {
                    write_atom(atom)
                }
                _ => expression(atom).map(YamlValue::String),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(YamlValue::Sequence),
    }
}

fn write_atom(atom: &PropertyAtom) -> Result<YamlValue, SimplifiedSchemaError> {
    match &atom.ty {
        // The parser moves a block object's presence flags onto its atom, so
        // they return to the block's `$constraints`.
        // An object with `...` spreads is written inline, where each spread
        // keeps its position among the members.
        TypeExpr::InlineObject(shape)
            if !atom.is_array
                && !matches!(shape.composition.as_deref(), Some(ObjectComposition::Inline(_)))
                && atom.description.is_none()
                && atom
                    .constraints
                    .iter()
                    .all(|c| matches!(c, Constraint::Required | Constraint::Generated)) =>
        {
            if atom.constraints.is_empty() {
                return write_shape(shape).map(YamlValue::Mapping);
            }
            let mut block = shape.clone();
            block.constraints.splice(0..0, atom.constraints.iter().cloned());
            write_shape(&block).map(YamlValue::Mapping)
        }
        _ => expression(atom).map(YamlValue::String),
    }
}

/// `atom` as one type-expression string, refused when an inline object
/// inside it carries what the inline form does not write: `$constraints`,
/// a `$extends` directive, or pattern keys in an object without `...`
/// spreads (an object with spreads writes its pattern keys inline).
fn expression(atom: &PropertyAtom) -> Result<String, SimplifiedSchemaError> {
    let mut lossy = None;
    visit_atom(atom, &mut |inner| {
        let TypeExpr::InlineObject(shape) = &inner.ty else { return };
        let spreads = matches!(shape.composition.as_deref(), Some(ObjectComposition::Inline(_)));
        if !shape.constraints.is_empty() || (!spreads && !shape.pattern_keys.is_empty()) {
            lossy.get_or_insert(
                "an inline object inside a type expression has pattern keys or `$constraints`, which only the \
                 block form writes",
            );
        }
        if matches!(shape.composition.as_deref(), Some(ObjectComposition::Extends(_))) {
            lossy.get_or_insert("an object that composes with `$extends` inside a type expression has no inline form");
        }
    });
    match lossy {
        Some(reason) => Err(unsupported(reason)),
        None => Ok(serialize_property_atom(atom)),
    }
}
