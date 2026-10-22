//! Schema components carried by a [`RestApi`], and the references that name them.
//!
//! Endpoint JSON bodies store a [`TypeRef`]: component names plus wrapper
//! shape. Each component's Rust path and wire schema is stored once, in
//! [`RestApi::schemas`], so a reference cannot disagree with it; it can only
//! dangle, which [`RestApi::validate_references`] rejects.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ApiRequest, ApiResponse, DefinitionError, Endpoint, ErrorEntry, FormFieldKind, ResponseShape, RestApi, TypeRef};

/// One named schema component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaComponent {
    /// Component name, the target of `#/components/schemas/{name}`.
    pub name: String,
    /// Fully qualified Rust path of a registered type. `None` for a schema
    /// that only appears nested inside a registered one; references may not
    /// name it directly.
    pub rust_path: Option<String>,
    /// JSON Schema (OpenAPI 3.0 settings) without its nested definitions,
    /// which are components of their own.
    pub schema: Value,
}

/// Flatten registrations, given as
/// `(name, rust_path, diagnostic, schema, composition)`, into components. A
/// registration takes over a same-named nested entry in place; otherwise
/// first position wins. This ordering is the OpenAPI `components.schemas`
/// order.
///
/// A `composition` replaces the registration's own schema as the component,
/// and the own schema's nested definitions are not walked: the composition
/// references only registered components. A nested copy of the registered
/// type elsewhere still agrees when it matches the own schema (schemars emits
/// `anyOf` for an untagged enum bound to `oneOf`).
///
/// ## Errors
///
/// [`DefinitionError::Collisions`] holding one
/// [`DefinitionError::ComponentConflict`] per name that two sources define
/// with different wire schemas. A registration and its own nested copy
/// elsewhere compare equal: see [`wire_shape`].
pub(crate) fn flatten<'a>(
    registrations: impl Iterator<Item = (&'a str, &'a str, &'a str, &'a Value, Option<Value>)>,
) -> Result<Vec<SchemaComponent>, DefinitionError> {
    let mut merged = Merged::default();
    for (name, rust_path, diagnostic, schema, composition) in registrations {
        let own = SchemaComponent { name: name.into(), rust_path: Some(rust_path.into()), schema: strip(schema, name) };
        let own_shape = wire_shape(&own.schema, name, false);
        let composed = composition.is_some();
        let (entry, shape, alternate) = match composition {
            Some(composition) => (SchemaComponent { schema: composition.clone(), ..own }, composition, Some(own_shape)),
            None => (own, own_shape, None),
        };
        match merged.components.iter().position(|c| c.name == name) {
            Some(index) if !merged.agrees(index, &shape) && alternate.as_ref().is_none_or(|a| !merged.agrees(index, a)) => merged.conflict(index, diagnostic.into()),
            Some(index) => {
                merged.components[index] = entry;
                merged.shapes[index].0 = shape;
                merged.alternates[index] = alternate;
            }
            None => merged.push(entry, (shape, alternate), diagnostic.into()),
        }
        if !composed { merged.nested(schema, name, diagnostic); }
    }
    if merged.conflicts.is_empty() { Ok(merged.components) } else { Err(DefinitionError::Collisions(merged.conflicts)) }
}

/// Components with the wire shape and source description of each, by index.
#[derive(Default)]
struct Merged {
    components: Vec<SchemaComponent>,
    shapes: Vec<(Value, String)>,
    /// A composed component's own schema shape, which a nested copy may match.
    alternates: Vec<Option<Value>>,
    conflicts: Vec<DefinitionError>,
}

impl Merged {
    fn push(&mut self, component: SchemaComponent, (shape, alternate): (Value, Option<Value>), source: String) {
        self.components.push(component);
        self.shapes.push((shape, source));
        self.alternates.push(alternate);
    }

    fn agrees(&self, index: usize, shape: &Value) -> bool {
        &self.shapes[index].0 == shape || self.alternates[index].as_ref() == Some(shape)
    }

    fn conflict(&mut self, index: usize, second: String) {
        let error = DefinitionError::ComponentConflict {
            schema_name: self.components[index].name.clone(), first: self.shapes[index].1.clone(), second };
        if !self.conflicts.contains(&error) { self.conflicts.push(error); }
    }

    /// `root_name` is the component of the registration being walked, the
    /// target of its definitions' `"$ref": "#"`; `root` describes it.
    fn nested(&mut self, value: &Value, root_name: &str, root: &str) {
        match value {
            Value::Object(object) => {
                let defs = object.get("$defs").and_then(Value::as_object);
                let schemas = object.get("components").and_then(|c| c.get("schemas")).and_then(Value::as_object);
                for (name, schema) in defs.into_iter().chain(schemas).flatten() {
                    let stripped = strip(schema, root_name);
                    let shape = wire_shape(&stripped, root_name, true);
                    match self.components.iter().position(|c| &c.name == name) {
                        Some(index) if !self.agrees(index, &shape) => self.conflict(index, format!("a schema nested in {root}")),
                        Some(_) => {}
                        None => {
                            self.push(SchemaComponent { name: name.clone(), rust_path: None, schema: stripped }, (shape, None), format!("a schema nested in {root}"));
                            self.nested(schema, root_name, root);
                        }
                    }
                }
                for value in object.values() { self.nested(value, root_name, root); }
            }
            Value::Array(values) => for value in values { self.nested(value, root_name, root); },
            _ => {}
        }
    }
}

/// The part of a stripped component schema that two same-named definitions
/// must agree on. schemars adds a top-level `title` and applies the OpenAPI
/// 3.0 transforms (for example `nullable`) only to a root schema, not to the
/// definitions it places under `components/schemas`, so a nested copy is
/// transformed here and `title` is ignored on both sides. A recursive type
/// reaches its registered root as `"$ref": "#"`, which names `root_name`.
fn wire_shape(schema: &Value, root_name: &str, nested: bool) -> Value {
    let mut shape = schema.clone();
    if nested && let Ok(mut transformed) = schemars::Schema::try_from(shape.clone()) {
        for transform in &mut schemars::generate::SchemaSettings::openapi3().transforms {
            transform.transform(&mut transformed);
        }
        shape = transformed.to_value();
    }
    if let Some(object) = shape.as_object_mut() { object.remove("title"); }
    resolve_root_refs(&mut shape, &format!("#/components/schemas/{root_name}"));
    shape
}

fn resolve_root_refs(value: &mut Value, target: &str) {
    match value {
        Value::Object(object) => for (key, value) in object.iter_mut() {
            if key == "$ref" && value == "#" { *value = target.into(); } else { resolve_root_refs(value, target); }
        },
        Value::Array(values) => for value in values { resolve_root_refs(value, target); },
        _ => {}
    }
}

/// `schema` without its nested definitions, with each `"$ref": "#"` naming
/// `root_name`: a nested component no longer knows which registration's
/// document `#` was, so it is resolved while that is still known.
fn strip(schema: &Value, root_name: &str) -> Value {
    let mut schema = schema.clone();
    if let Some(object) = schema.as_object_mut() {
        for key in ["$schema", "$defs", "components"] { object.remove(key); }
    }
    resolve_root_refs(&mut schema, &format!("#/components/schemas/{root_name}"));
    schema
}

impl Endpoint {
    /// The JSON body references of this endpoint: response, request, JSON
    /// form fields, then declared error entries, in that order.
    pub fn json_references(&self) -> Vec<&TypeRef> {
        let mut references = Vec::new();
        if let ApiResponse::Json(reference) = &self.response { references.push(reference); }
        match &self.request {
            Some(ApiRequest::Json(reference)) => references.push(reference),
            Some(ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields }) => references.extend(
                fields.iter().filter_map(|field| match &field.kind { FormFieldKind::Json(reference) => Some(reference), _ => None })),
            _ => {}
        }
        references.extend(error_references(&self.errors));
        references
    }
}

impl RestApi {
    /// The component named `name`.
    pub fn component(&self, name: &str) -> Option<&SchemaComponent> {
        self.schemas.iter().find(|c| c.name == name)
    }

    /// Rust spelling of `reference`, resolving names through [`Self::schemas`].
    /// `None` when a name does not resolve to a registered component, or the
    /// form has no Rust type (compositions and `Unknown`).
    pub fn rust_type(&self, reference: &TypeRef) -> Option<String> {
        reference.rust_path_with(&|name| self.component(name).and_then(|c| c.rust_path.clone()))
    }

    /// Check that components are well formed with unique names, and that
    /// every endpoint reference resolves. API build runs this; run it on a
    /// deserialized definition before use.
    pub fn validate_references(&self) -> Result<(), DefinitionError> {
        let invalid = |message: String| DefinitionError::InvalidDefinition { api: self.name.clone(), endpoint: "<api>".into(), message };
        for (index, component) in self.schemas.iter().enumerate() {
            if component.name.trim().is_empty() {
                return Err(invalid(format!("schema component {index} has an empty name")));
            }
            if component.rust_path.as_deref().is_some_and(|path| path.trim().is_empty()) {
                return Err(invalid(format!("schema component \"{}\" has an empty Rust path", component.name)));
            }
            if !(component.schema.is_object() || component.schema.is_boolean()) {
                return Err(invalid(format!("schema component \"{}\" is not a JSON Schema object or boolean", component.name)));
            }
            if self.schemas[..index].iter().any(|c| c.name == component.name) {
                return Err(invalid(format!("schema component \"{}\" is defined more than once", component.name)));
            }
        }
        let api_errors = error_references(&self.errors).map(|reference| ("<api>", reference));
        let endpoints = self.endpoints.iter().flat_map(|endpoint| endpoint.json_references().into_iter().map(|reference| (endpoint.id.as_str(), reference)));
        for (endpoint, reference) in api_errors.chain(endpoints) {
            if let Some(name) = self.unresolved_name(reference) {
                return Err(DefinitionError::UnresolvedReference { api: self.name.clone(), endpoint: endpoint.into(), name: name.into() });
            }
            if self.rust_type(reference).is_none() || has_empty_key(reference) {
                return Err(DefinitionError::InvalidDefinition { api: self.name.clone(), endpoint: endpoint.into(),
                    message: format!("unsupported schema reference {reference:?}") });
            }
        }
        for operation in self.graphql.iter().flat_map(|surface| &surface.operations) {
            for reference in operation.json_references() {
                let problem = match self.unresolved_name(reference) {
                    Some(name) => format!("unresolved schema reference \"{name}\""),
                    None if self.rust_type(reference).is_none() || has_empty_key(reference) => format!("unsupported schema reference {reference:?}"),
                    None => continue,
                };
                return Err(DefinitionError::InvalidOperation { api: self.name.clone(), operation: operation.id.clone(), message: problem });
            }
        }
        Ok(())
    }

    fn unresolved_name<'a>(&self, reference: &'a TypeRef) -> Option<&'a str> {
        reference.names().into_iter().find(|name| self.component(name).is_none_or(|c| c.rust_path.is_none()))
    }
}

fn error_references(entries: &[ErrorEntry]) -> impl Iterator<Item = &TypeRef> {
    entries.iter().filter_map(|entry| match &entry.shape { ResponseShape::Json(reference) => Some(reference), _ => None })
}

fn has_empty_key(reference: &TypeRef) -> bool {
    match reference {
        TypeRef::KeyedMap { key, value } => key.trim().is_empty() || has_empty_key(value),
        TypeRef::Array(inner) | TypeRef::Optional(inner) | TypeRef::Map(inner) => has_empty_key(inner),
        _ => false,
    }
}
