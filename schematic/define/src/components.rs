//! Schema components carried by a [`RestApi`], and the references that name them.
//!
//! Endpoint JSON bodies store a [`TypeRef`]: component names plus wrapper
//! shape. Each component's Rust path and wire schema is stored once, in
//! [`RestApi::schemas`], so a reference cannot disagree with it; it can only
//! dangle, which [`RestApi::validate_references`] rejects.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ApiRequest, DefinitionError, Endpoint, FormFieldKind, ResponseShape, RestApi, SchemaReference, StreamFraming, TypeRef};

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

/// The JSON body references of an endpoint, shared by
/// [`Endpoint::json_references`] and [`Endpoint::json_references_mut`] so the
/// locations reference validation reads and the locations a rename rewrites
/// cannot drift apart. Pass `mut` for mutable references.
macro_rules! endpoint_json_references {
    ($endpoint:expr $(, $mutability:tt)?) => {{
        let endpoint = $endpoint;
        let mut references = Vec::new();
        references.extend((&$($mutability)? endpoint.response).into_iter().filter_map(|success| match &$($mutability)? success.shape {
            ResponseShape::Json(reference)
            | ResponseShape::Stream { framing: StreamFraming::Sse(reference) | StreamFraming::Ndjson(reference), .. } => Some(reference),
            _ => None,
        }));
        match &$($mutability)? endpoint.request {
            Some(ApiRequest::Json(reference)) => references.push(reference),
            Some(ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields }) => references.extend(
                fields.into_iter().filter_map(|field| match &$($mutability)? field.kind { FormFieldKind::Json(reference) => Some(reference), _ => None })),
            _ => {}
        }
        references.extend(error_references!(&$($mutability)? endpoint.errors $(, $mutability)?));
        references
    }};
}

/// The JSON references of an error table's entries; `mut` for mutable ones.
macro_rules! error_references {
    ($entries:expr $(, $mutability:tt)?) => {
        ($entries).into_iter().filter_map(|entry| match &$($mutability)? entry.shape { crate::ResponseShape::Json(reference) => Some(reference), _ => None })
    };
}
pub(crate) use error_references;

impl Endpoint {
    /// The JSON body references of this endpoint: success shapes (JSON
    /// bodies and SSE/NDJSON stream items, in declaration order), request,
    /// JSON form fields, then declared error entries, in that order.
    pub fn json_references(&self) -> Vec<&TypeRef> {
        endpoint_json_references!(self)
    }

    /// [`Self::json_references`], mutably.
    // OpenAPI import renames components through it.
    #[cfg_attr(not(feature = "openapi"), allow(dead_code))]
    pub(crate) fn json_references_mut(&mut self) -> Vec<&mut TypeRef> {
        endpoint_json_references!(self, mut)
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
    /// every endpoint reference resolves. API build runs this; run it, and
    /// [`Self::validate_schema_closure`], on a deserialized definition before
    /// use.
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
        // Reads exactly what `Self::json_references_mut` rewrites; keep the two in step.
        let api_errors = error_references!(&self.errors).map(|reference| ("<api>", reference));
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

    /// Check that every `$ref` inside every component schema resolves to a
    /// component of [`Self::schemas`]. Endpoint references name components,
    /// so this closes every schema an endpoint names as well.
    ///
    /// A reference resolves when it is `#/components/schemas/{name}`, with
    /// percent-encoding and JSON-pointer escapes (`~1`, `~0`) decoded; a
    /// `$ref` inside instance data (`default`, `enum`, ...) is not one. Each
    /// component is walked once, without following references, so a
    /// recursive model terminates. Nested definitions extracted from a
    /// registration are components already, so for a built API this is a
    /// backstop. API build, bundle loading, generation, and export run it;
    /// OpenAPI import does not, since an imported document can hold
    /// references its components cannot satisfy.
    ///
    /// ## Errors
    ///
    /// [`DefinitionError::UnresolvedSchemaReferences`] listing every
    /// reference that does not resolve, with its parent component and path.
    pub fn validate_schema_closure(&self) -> Result<(), DefinitionError> {
        let mut references = Vec::new();
        for component in &self.schemas {
            collect_references(&component.schema, &mut String::new(), &mut |path, reference| {
                let missing = match component_reference(reference) {
                    Some(name) if self.component(&name).is_some() => return,
                    other => other,
                };
                references.push(SchemaReference { parent: component.name.clone(), path: path.into(), reference: reference.into(), missing });
            });
        }
        if references.is_empty() { Ok(()) } else { Err(DefinitionError::UnresolvedSchemaReferences { api: self.name.clone(), references }) }
    }

    /// Every JSON reference of the definition, mutably: API-level error
    /// defaults, each endpoint's [`Endpoint::json_references`], then each
    /// GraphQL operation's. These are exactly the references
    /// [`Self::validate_references`] reads, so renaming a component through
    /// them leaves no reference to its old name.
    #[cfg_attr(not(feature = "openapi"), allow(dead_code))]
    pub(crate) fn json_references_mut(&mut self) -> Vec<&mut TypeRef> {
        let mut references: Vec<&mut TypeRef> = error_references!(&mut self.errors, mut).collect();
        references.extend(self.endpoints.iter_mut().flat_map(Endpoint::json_references_mut));
        references.extend(self.graphql.iter_mut().flat_map(|surface| &mut surface.operations).flat_map(crate::GraphQlOperation::json_references_mut));
        references
    }

    fn unresolved_name<'a>(&self, reference: &'a TypeRef) -> Option<&'a str> {
        reference.names().into_iter().find(|name| self.component(name).is_none_or(|c| c.rust_path.is_none()))
    }
}

/// Keywords whose values are instance data, not schemas, so a `$ref` inside
/// them is not a reference.
const DATA_KEYWORDS: [&str; 5] = ["const", "default", "enum", "example", "examples"];

/// Keywords whose values map names to schemas, so a key there is a name,
/// never a keyword.
const SCHEMA_MAPS: [&str; 5] = ["properties", "patternProperties", "dependentSchemas", "$defs", "definitions"];

/// Calls `found(path, reference)` for every `$ref` of the schema
/// `value` and its subschemas, where `path` is the JSON pointer to the
/// schema holding it.
fn collect_references(value: &Value, path: &mut String, found: &mut impl FnMut(&str, &str)) {
    match value {
        Value::Object(object) => {
            // A `$ref` that is not a string is reported in its JSON spelling,
            // never read as no reference.
            match object.get("$ref") {
                Some(Value::String(reference)) => found(if path.is_empty() { "/" } else { path }, reference),
                Some(other) => found(if path.is_empty() { "/" } else { path }, &other.to_string()),
                None => {}
            }
            for (key, child) in object.iter().filter(|(key, _)| !DATA_KEYWORDS.contains(&key.as_str())) {
                let length = push_token(path, key);
                match child.as_object().filter(|_| SCHEMA_MAPS.contains(&key.as_str())) {
                    Some(named) => for (name, schema) in named {
                        let inner = push_token(path, name);
                        collect_references(schema, path, found);
                        path.truncate(inner);
                    },
                    None => collect_references(child, path, found),
                }
                path.truncate(length);
            }
        }
        Value::Array(values) => for (index, child) in values.iter().enumerate() {
            let length = push_token(path, &index.to_string());
            collect_references(child, path, found);
            path.truncate(length);
        },
        _ => {}
    }
}

/// Appends `token`, escaped, to the JSON pointer `path`; returns the length
/// to truncate back to.
fn push_token(path: &mut String, token: &str) -> usize {
    let length = path.len();
    path.push('/');
    path.push_str(&token.replace('~', "~0").replace('/', "~1"));
    length
}

/// The component name a `#/components/schemas/{name}` reference names,
/// decoded; `None` for any other reference.
fn component_reference(reference: &str) -> Option<String> {
    let fragment = percent_decode(reference.strip_prefix('#')?)?;
    let token = fragment.strip_prefix("/components/schemas/")?;
    (!token.is_empty() && !token.contains('/')).then(|| token.replace("~1", "/").replace("~0", "~"))
}

/// `text` with `%XX` escapes decoded; `None` for a malformed escape or
/// non-UTF-8 result.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn has_empty_key(reference: &TypeRef) -> bool {
    match reference {
        TypeRef::KeyedMap { key, value } => key.trim().is_empty() || has_empty_key(value),
        TypeRef::Array(inner) | TypeRef::Optional(inner) | TypeRef::Map(inner) => has_empty_key(inner),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AddressRule, ApiRequest, DefinitionContext, Endpoint, ErrorEntry, FormField, GraphQlOperation, GraphQlVariables, RestApi,
        ResponseShape, StatusMatch, StreamEnd, StreamFraming, SuccessShape, TypeRef,
    };

    /// Every reference [`RestApi::validate_references`] reads, as names.
    fn validated_names(api: &RestApi) -> Vec<String> {
        let api_errors = error_references!(&api.errors);
        let endpoints = api.endpoints.iter().flat_map(Endpoint::json_references);
        let operations = api.graphql.iter().flat_map(|surface| &surface.operations).flat_map(GraphQlOperation::json_references);
        api_errors.chain(endpoints).chain(operations).flat_map(TypeRef::names).map(str::to_owned).collect()
    }

    #[test]
    fn json_references_mut_reaches_every_reference_validation_reads() {
        let named = || TypeRef::Named("Thing".into());
        let wrapped = || TypeRef::Optional(Box::new(TypeRef::Array(Box::new(TypeRef::AnyOf(vec![named()])))));
        let error = |reference| ErrorEntry::new(StatusMatch::exact(400), ResponseShape::Json(reference));
        let stream = |framing| SuccessShape::new(ResponseShape::Stream { framing, end: StreamEnd::ConnectionClose, keep: false });
        let mut ctx = DefinitionContext::types_in("::models");
        let operation = GraphQlOperation::query("Q", "{ a }").describes("Q").no_variables()
            .data::<serde_json::Value>(&mut ctx).build().unwrap();
        let endpoint = Endpoint::get("E", "/e").describes("E").returns_empty().build().unwrap();
        let mut api = RestApi::builder(&ctx, "A", "https://a.example.com").describes("A").no_auth()
            .endpoints([endpoint.clone(), endpoint]).graphql(AddressRule::github()).operations([operation]).build().unwrap();
        api.errors = vec![error(named()), error(wrapped())];
        api.endpoints[0].response = vec![
            SuccessShape::json(wrapped()), stream(StreamFraming::Sse(named())), stream(StreamFraming::Ndjson(wrapped())),
        ];
        api.endpoints[0].request = Some(ApiRequest::form_data(vec![FormField::json("a", named()), FormField::json("b", wrapped())]));
        api.endpoints[0].errors = vec![error(wrapped())];
        api.endpoints[1].request = Some(ApiRequest::json(wrapped()));
        let operation = &mut api.graphql.as_mut().unwrap().operations[0];
        operation.variables = GraphQlVariables::Typed(named());
        operation.data = wrapped();
        operation.errors = vec![error(named())];
        let validated = validated_names(&api).len();

        for name in api.json_references_mut().into_iter().flat_map(TypeRef::names_mut) {
            *name = "Renamed".into();
        }

        assert_eq!(validated, 12);
        assert_eq!(validated_names(&api), vec!["Renamed".to_owned(); validated]);
    }
}
