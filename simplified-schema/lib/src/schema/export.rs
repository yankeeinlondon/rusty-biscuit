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

use std::sync::Arc;

use serde_json::Value;
use serde_yaml_ng::{Mapping, Value as YamlValue};

use super::layer::{Authored, AuthoredSchema};
use super::{SimplifiedSchema, SimplifiedSchemaError};
use crate::simplified::serialize::serialize_property_atom;
use crate::simplified::standalone::{visit_atom, visit_def};
use crate::simplified::{
    Constraint, PatternKey, PropertyAtom, PropertyDef, SchemaArm, SchemaCategory, SchemaDeclaration, SchemaRoot,
    SchemaShape, StandaloneSchemaEnvelope, TypeExpr,
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
    /// or names a file an in-memory schema cannot read.
    pub fn to_json_schema(&self) -> Result<Arc<Value>, SimplifiedSchemaError> {
        self.lowered().map_err(SimplifiedSchemaError::Unprepared)
    }

    /// The schema as one SimplifiedSchema YAML document, in its authored
    /// envelope (`$schema` with optional `types`, or `kind: schema`).
    ///
    /// A layered schema exports as the receiver's document with the
    /// properties merged in from the other layers.
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
        serde_yaml_ng::to_string(&YamlValue::Mapping(document)).map_err(|error| unsupported(&error.to_string()))
    }
}

fn unsupported(reason: &str) -> SimplifiedSchemaError {
    SimplifiedSchemaError::ExportUnsupported { reason: reason.to_string() }
}

/// The layers merged with whole-property replacement, highest first: the top
/// layer's document, plus each lower layer's properties no higher layer
/// declares. A copied property may not refer to named types, which resolve
/// in the layer that declared them.
fn write_layered(layers: &[Arc<super::layer::Layer>]) -> Result<Mapping, SimplifiedSchemaError> {
    let mut shapes = Vec::with_capacity(layers.len());
    for layer in layers {
        let Authored::Simplified(authored) = &layer.authored else {
            return Err(unsupported("a layer is raw JSON Schema, which has no SimplifiedSchema form"));
        };
        let Some(SchemaRoot::Single(shape)) = authored.schema() else {
            return Err(unsupported("only object-schema layers combine into one document"));
        };
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
            merged.properties.insert(name.clone(), def.clone());
        }
    }
    let combined = AuthoredSchema {
        declaration: SchemaDeclaration::Schema(SchemaRoot::Single(merged)),
        local_types: top.local_types.clone(),
        envelope: top.envelope,
        categories: top.categories.clone(),
    };
    write_document(&combined)
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
            document.insert("$schema".into(), write_declaration(&authored.declaration)?);
            if let Some(types) = &authored.local_types {
                document.insert("types".into(), YamlValue::Mapping(write_shape(types)?));
            }
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
                    document.insert("$schema".into(), write_declaration(&authored.declaration)?);
                }
                // The `types` table is the payload.
                None => {
                    document.insert("types".into(), write_declaration(&authored.declaration)?);
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

fn write_declaration(declaration: &SchemaDeclaration) -> Result<YamlValue, SimplifiedSchemaError> {
    match declaration {
        SchemaDeclaration::Reference(reference) => Ok(reference.file_reference().raw().into()),
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

/// A block-form object: literal properties, pattern keys, then
/// `$constraints`.
fn write_shape(shape: &SchemaShape) -> Result<Mapping, SimplifiedSchemaError> {
    let mut mapping = Mapping::new();
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

fn pattern_key_text(key: &PatternKey) -> String {
    match key {
        PatternKey::CatchAll => "<string>".into(),
        PatternKey::Starting(prefix) => format!("<starting::{prefix}>"),
        PatternKey::Ending(suffix) => format!("<ending::{suffix}>"),
        PatternKey::Pattern(pattern) => format!("<pattern::{pattern}>"),
    }
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
/// expression string.
fn write_def(def: &PropertyDef) -> Result<YamlValue, SimplifiedSchemaError> {
    match def {
        PropertyDef::Single(atom) => write_atom(atom),
        PropertyDef::Union(atoms) => atoms
            .iter()
            .map(|atom| expression(atom).map(YamlValue::String))
            .collect::<Result<Vec<_>, _>>()
            .map(YamlValue::Sequence),
    }
}

fn write_atom(atom: &PropertyAtom) -> Result<YamlValue, SimplifiedSchemaError> {
    match &atom.ty {
        TypeExpr::InlineObject(shape)
            if !atom.is_array && atom.constraints.is_empty() && atom.description.is_none() =>
        {
            write_shape(shape).map(YamlValue::Mapping)
        }
        _ => expression(atom).map(YamlValue::String),
    }
}

/// `atom` as one type-expression string, refused when an inline object
/// inside it carries pattern keys or `$constraints`, which the inline form
/// does not write.
fn expression(atom: &PropertyAtom) -> Result<String, SimplifiedSchemaError> {
    let mut lossy = false;
    visit_atom(atom, &mut |inner| {
        if let TypeExpr::InlineObject(shape) = &inner.ty {
            lossy |= !shape.pattern_keys.is_empty() || !shape.constraints.is_empty();
        }
    });
    if lossy {
        return Err(unsupported(
            "an inline object inside a type expression has pattern keys or `$constraints`, which only the \
             block form writes",
        ));
    }
    Ok(serialize_property_atom(atom))
}
