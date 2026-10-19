//! Schema components carried by a [`RestApi`], and the references that name them.
//!
//! Endpoint JSON bodies store a [`TypeRef`]: component names plus wrapper
//! shape. Each component's Rust path and wire schema is stored once, in
//! [`RestApi::schemas`], so a reference cannot disagree with it; it can only
//! dangle, which [`RestApi::validate_references`] rejects.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ApiRequest, ApiResponse, DefinitionError, Endpoint, FormFieldKind, RestApi, TypeRef};

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

/// Flatten registrations into components. Each registration replaces a
/// same-named nested entry in place; a nested entry never replaces an
/// existing one. This ordering is the OpenAPI `components.schemas` order.
pub(crate) fn flatten<'a>(registrations: impl Iterator<Item = (&'a str, &'a str, &'a Value)>) -> Vec<SchemaComponent> {
    let mut components: Vec<SchemaComponent> = Vec::new();
    for (name, rust_path, schema) in registrations {
        let entry = SchemaComponent { name: name.into(), rust_path: Some(rust_path.into()), schema: strip(schema) };
        match components.iter_mut().find(|c| c.name == name) {
            Some(existing) => *existing = entry,
            None => components.push(entry),
        }
        nested(schema, &mut components);
    }
    components
}

fn nested(value: &Value, components: &mut Vec<SchemaComponent>) {
    match value {
        Value::Object(object) => {
            let defs = object.get("$defs").and_then(Value::as_object);
            let schemas = object.get("components").and_then(|c| c.get("schemas")).and_then(Value::as_object);
            for (name, schema) in defs.into_iter().chain(schemas).flatten() {
                if !components.iter().any(|c| &c.name == name) {
                    components.push(SchemaComponent { name: name.clone(), rust_path: None, schema: strip(schema) });
                    nested(schema, components);
                }
            }
            for value in object.values() { nested(value, components); }
        }
        Value::Array(values) => for value in values { nested(value, components); },
        _ => {}
    }
}

fn strip(schema: &Value) -> Value {
    let mut schema = schema.clone();
    if let Some(object) = schema.as_object_mut() {
        for key in ["$schema", "$defs", "components"] { object.remove(key); }
    }
    schema
}

impl Endpoint {
    /// The JSON body references of this endpoint: response, request, and
    /// JSON form fields, in that order.
    pub fn json_references(&self) -> Vec<&TypeRef> {
        let mut references = Vec::new();
        if let ApiResponse::Json(reference) = &self.response { references.push(reference); }
        match &self.request {
            Some(ApiRequest::Json(reference)) => references.push(reference),
            Some(ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields }) => references.extend(
                fields.iter().filter_map(|field| match &field.kind { FormFieldKind::Json(reference) => Some(reference), _ => None })),
            _ => {}
        }
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
        for endpoint in &self.endpoints {
            for reference in endpoint.json_references() {
                for name in reference.names() {
                    if self.component(name).is_none_or(|c| c.rust_path.is_none()) {
                        return Err(DefinitionError::UnresolvedReference { api: self.name.clone(), endpoint: endpoint.id.clone(), name: name.into() });
                    }
                }
                if self.rust_type(reference).is_none() || has_empty_key(reference) {
                    return Err(DefinitionError::InvalidDefinition { api: self.name.clone(), endpoint: endpoint.id.clone(),
                        message: format!("unsupported schema reference {reference:?}") });
                }
            }
        }
        Ok(())
    }
}

fn has_empty_key(reference: &TypeRef) -> bool {
    match reference {
        TypeRef::KeyedMap { key, value } => key.trim().is_empty() || has_empty_key(value),
        TypeRef::Array(inner) | TypeRef::Optional(inner) | TypeRef::Map(inner) => has_empty_key(inner),
        _ => false,
    }
}
