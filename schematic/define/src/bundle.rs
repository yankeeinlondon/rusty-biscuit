//! Self-contained definition bundles.
//!
//! A [`DefinitionBundle`] is one [`RestApi`] together with every schema
//! component its definition context registered ([`RestApi::schemas`], each
//! pairing a component name with its Rust binding) and, optionally, the
//! context's structural [`ModelCatalog`]. Its canonical JSON loads without the
//! Rust type registry, so a definition can be compared and exported from a
//! file alone. A loaded bundle carries no executable Rust types and cannot
//! compile a client by itself.
//!
//! Bundles preserve literal header values, examples, defaults, and
//! environment variable *names* as authored; they never hold resolved
//! environment values. Review the literals of a definition before publishing
//! its bundle. See `schematic/docs/io/definition-bundles.md`.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{DefinitionError, ModelCatalog, ModelDef, RestApi, TypeRef};

/// The only bundle format version this build writes and reads.
pub const BUNDLE_FORMAT_VERSION: u64 = 1;

const COMPONENTS: &str = "#/components/schemas/";

/// A [`RestApi`] with its complete schema closure, loadable in isolation.
///
/// ## Examples
///
/// ```
/// use schematic_define::{DefinitionBundle, DefinitionContext, Endpoint, RestApi, definition_type};
/// definition_type!(#[derive(schemars::JsonSchema)] pub struct Item { pub id: u64 });
///
/// let mut ctx = DefinitionContext::types_in("::models");
/// let get = Endpoint::get("GetItem", "/item").describes("Get an item")
///     .returns_json::<Item>(&mut ctx).build()?;
/// let api = RestApi::builder(&ctx, "Items", "https://api.example.com")
///     .describes("Items").no_auth().endpoints([get]).build()?;
///
/// let json = DefinitionBundle::new(api.clone())?.to_json();
/// let loaded = DefinitionBundle::from_json(&json)?;
/// assert_eq!(loaded.api(), &api);
/// assert_eq!(loaded.to_json(), json);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionBundle {
    api: RestApi,
    models: Option<ModelCatalog>,
}

/// Why a bundle could not be built or loaded. Loading reports the first
/// problem and returns no partial bundle.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BundleError {
    /// The text is not exactly one JSON document of the bundle's shape:
    /// invalid syntax, a duplicate object key, trailing content, or a field
    /// of the wrong type.
    #[error("bundle JSON line {line} column {column}: {message}")]
    Json { line: usize, column: usize, message: String },
    /// A member the bundle shape does not accept, at a JSON Pointer.
    #[error("bundle field {pointer}: {message}")]
    Field { pointer: String, message: String },
    /// `format_version` is a number other than [`BUNDLE_FORMAT_VERSION`].
    #[error("unsupported bundle format_version {found}; this build reads version {BUNDLE_FORMAT_VERSION}")]
    UnsupportedVersion { found: String },
    /// A `$ref` inside a component schema that is not a reference to a
    /// component of this bundle. Bundles never fetch files or URLs.
    #[error("{api} schema component \"{component}\": reference \"{reference}\" {problem}")]
    SchemaReference { api: String, component: String, reference: String, problem: &'static str },
    /// The structural catalog disagrees with the bundle's components.
    #[error("{api} model catalog: {message}")]
    Catalog { api: String, message: String },
    /// The definition itself is invalid, including an endpoint reference to
    /// a missing component ([`DefinitionError::UnresolvedReference`]).
    #[error(transparent)]
    Definition(#[from] DefinitionError),
}

#[derive(Serialize)]
struct Written<'a> {
    format_version: u64,
    api: &'a RestApi,
    #[serde(skip_serializing_if = "Option::is_none")]
    models: Option<&'a ModelCatalog>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Loaded {
    #[allow(dead_code)] // checked before typed deserialization
    format_version: u64,
    api: RestApi,
    #[serde(default, deserialize_with = "crate::serde_presence::non_null")]
    models: Option<ModelCatalog>,
}

impl DefinitionBundle {
    /// Bundle `api` with its components and no catalog.
    ///
    /// Component references are normalized to the bundle namespace
    /// (`#/components/schemas/{name}`): `#/$defs/{name}` and
    /// `#/definitions/{name}` are renamed, and a registered component's
    /// `"$ref": "#"` names itself.
    ///
    /// ## Errors
    ///
    /// [`BundleError::SchemaReference`] for any other reference, or one
    /// naming no component; [`BundleError::Definition`] when `api` fails the
    /// validation its builder applies.
    pub fn new(mut api: RestApi) -> Result<Self, BundleError> {
        for component in &mut api.schemas {
            let registered = component.rust_path.is_some();
            normalize_refs(&mut component.schema, &component.name, registered)
                .map_err(|(reference, problem)| BundleError::SchemaReference {
                    api: api.name.clone(), component: component.name.clone(), reference, problem })?;
        }
        validate(&api, None)?;
        Ok(Self { api, models: None })
    }

    /// Attach the definition context's structural catalog
    /// ([`crate::DefinitionContext::models`]). An empty catalog is kept as
    /// explicitly empty.
    ///
    /// ## Errors
    ///
    /// [`BundleError::Catalog`] when a model is not a registered component of
    /// the bundle, is listed twice, names an unregistered member, or a
    /// composition disagrees with its component's schema.
    pub fn with_models(self, models: ModelCatalog) -> Result<Self, BundleError> {
        validate_catalog(&self.api, &models)?;
        Ok(Self { models: Some(models), ..self })
    }

    /// The bundled definition.
    pub fn api(&self) -> &RestApi { &self.api }

    /// The structural catalog, when one was bundled.
    pub fn models(&self) -> Option<&ModelCatalog> { self.models.as_ref() }

    /// The bundled definition, by value.
    pub fn into_api(self) -> RestApi { self.api }

    /// Canonical JSON: object keys sorted at every depth, array order kept,
    /// absent options omitted rather than `null`, two-space indentation and a
    /// final newline. Equal bundles give identical bytes.
    ///
    /// ## Panics
    ///
    /// Never for bundle contents: every definition type serializes to JSON
    /// with string map keys.
    pub fn to_json(&self) -> String {
        let written = Written { format_version: BUNDLE_FORMAT_VERSION, api: &self.api, models: self.models.as_ref() };
        let mut value = serde_json::to_value(written).expect("definition types serialize to JSON");
        omit_nulls(&mut value, &mut String::new());
        let mut text = serde_json::to_string_pretty(&canonical(value)).expect("a JSON value serializes");
        text.push('\n');
        text
    }

    /// Load a bundle from its JSON, without the Rust registry.
    ///
    /// The text must be exactly one JSON document: duplicate keys at any
    /// depth, trailing content, comments, and trailing commas fail. Outside
    /// component schemas, which are data, an explicit `null` fails (absent
    /// options are omitted), except a grant-inapplicable OAuth2 field, which
    /// may be null. Members the shape does not define fail rather than being
    /// ignored. The definition then passes its builder's validation, every
    /// component `$ref` names a bundled component, and the catalog agrees
    /// with the components.
    ///
    /// ## Errors
    ///
    /// The first problem found, as a [`BundleError`] locating it by line and
    /// column, JSON Pointer, or API, endpoint, component, and name.
    pub fn from_json(text: &str) -> Result<Self, BundleError> {
        let value = strict_parse(text)?;
        check_version(&value)?;
        reject_nulls(&value, &mut String::new(), false)?;
        let loaded: Loaded = serde_json::from_str(text).map_err(json_error)?;
        let bundle = Self { api: loaded.api, models: loaded.models };
        let mut written = serde_json::to_value(Written {
            format_version: BUNDLE_FORMAT_VERSION, api: &bundle.api, models: bundle.models.as_ref() })
            .map_err(json_error)?;
        omit_nulls(&mut written, &mut String::new());
        check_members(&value, &written, &mut String::new())?;
        validate(&bundle.api, bundle.models.as_ref())?;
        Ok(bundle)
    }
}

/// What construction and loading both guarantee: the builder's rules, a
/// closed component namespace, and a consistent catalog.
fn validate(api: &RestApi, models: Option<&ModelCatalog>) -> Result<(), BundleError> {
    crate::builders::validate_api(api)?;
    for component in &api.schemas {
        check_refs(&component.schema, api).map_err(|(reference, problem)| BundleError::SchemaReference {
            api: api.name.clone(), component: component.name.clone(), reference, problem })?;
    }
    if let Some(models) = models { validate_catalog(api, models)?; }
    Ok(())
}

fn json_error(error: serde_json::Error) -> BundleError {
    BundleError::Json { line: error.line(), column: error.column(), message: error.to_string() }
}

fn field(pointer: &str, message: impl Into<String>) -> BundleError {
    BundleError::Field { pointer: if pointer.is_empty() { "/".into() } else { pointer.into() }, message: message.into() }
}

fn push_token(pointer: &mut String, token: &str) {
    pointer.push('/');
    pointer.push_str(&token.replace('~', "~0").replace('/', "~1"));
}


/// Parse one JSON document into a value, rejecting duplicate object keys at
/// every depth (`serde_json::Value` keeps the last) and trailing content.
fn strict_parse(text: &str) -> Result<Value, BundleError> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let value = StrictValue.deserialize(&mut deserializer).map_err(json_error)?;
    deserializer.end().map_err(json_error)?;
    Ok(value)
}

struct StrictValue;

impl<'de> DeserializeSeed<'de> for StrictValue {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for StrictValue {
    type Value = Value;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result { formatter.write_str("a JSON value") }
    fn visit_bool<E>(self, value: bool) -> Result<Value, E> { Ok(Value::Bool(value)) }
    fn visit_i64<E>(self, value: i64) -> Result<Value, E> { Ok(value.into()) }
    fn visit_u64<E>(self, value: u64) -> Result<Value, E> { Ok(value.into()) }
    fn visit_f64<E>(self, value: f64) -> Result<Value, E> { Ok(serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)) }
    fn visit_str<E>(self, value: &str) -> Result<Value, E> { Ok(Value::String(value.into())) }
    fn visit_string<E>(self, value: String) -> Result<Value, E> { Ok(Value::String(value)) }
    fn visit_unit<E>(self) -> Result<Value, E> { Ok(Value::Null) }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element_seed(StrictValue)? { values.push(value); }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut map = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if map.contains_key(&key) { return Err(de::Error::custom(format_args!("duplicate key \"{key}\""))); }
            let value = access.next_value_seed(StrictValue)?;
            map.insert(key, value);
        }
        Ok(Value::Object(map))
    }
}

/// Checked first, so a bundle of another version reports its version rather
/// than a shape that version may legitimately have.
fn check_version(value: &Value) -> Result<(), BundleError> {
    let Some(root) = value.as_object() else { return Err(field("", "a bundle is a JSON object")) };
    match root.get("format_version") {
        None => Err(field("/format_version", "missing")),
        Some(Value::Number(number)) if number.as_u64() == Some(BUNDLE_FORMAT_VERSION) => Ok(()),
        Some(Value::Number(number)) => Err(BundleError::UnsupportedVersion { found: number.to_string() }),
        Some(other) => Err(field("/format_version", format!("expected an integer, found {}", kind(other)))),
    }
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// OAuth2 fields a grant does not use; PD9 lets them be absent or null.
const GRANT_FIELDS: [&str; 3] = ["authorization_url", "device_authorization_url", "pkce"];

/// Whether the member at `pointer` holds opaque JSON Schema data, where
/// `null` and any key are legitimate values.
fn is_schema_payload(pointer: &str) -> bool {
    pointer.strip_prefix("/api/schemas/")
        .and_then(|rest| rest.split_once('/'))
        .is_some_and(|(index, member)| index.bytes().all(|b| b.is_ascii_digit()) && member == "schema")
}

fn reject_nulls(value: &Value, pointer: &mut String, grant: bool) -> Result<(), BundleError> {
    match value {
        Value::Null if grant => Ok(()),
        Value::Null => Err(field(pointer, "explicit null; omit an absent option")),
        Value::Array(values) => values.iter().enumerate().try_for_each(|(index, value)| {
            let length = pointer.len();
            push_token(pointer, &index.to_string());
            let result = reject_nulls(value, pointer, false);
            pointer.truncate(length);
            result
        }),
        Value::Object(object) => object.iter().try_for_each(|(key, value)| {
            let length = pointer.len();
            push_token(pointer, key);
            let grant = object.contains_key("grant_type") && GRANT_FIELDS.contains(&key.as_str());
            let result = if is_schema_payload(pointer) { Ok(()) } else { reject_nulls(value, pointer, grant) };
            pointer.truncate(length);
            result
        }),
        _ => Ok(()),
    }
}

/// Members serialized only when they differ from their default, so an
/// explicit default is legitimate input the writer omits. Coupled to the
/// `skip_serializing_if` attributes on `Endpoint`, `EndpointParams`,
/// `ParamDef`, and `AuthPolicy`.
const OMITTED_DEFAULTS: [&str; 5] = ["deprecated", "secret", "pager_managed", "path", "explicit"];

/// Every member of `input` must survive typed deserialization: `written`,
/// the re-serialized bundle, holds it with the same value. This rejects keys
/// that a type without `deny_unknown_fields` would silently drop.
fn check_members(input: &Value, written: &Value, pointer: &mut String) -> Result<(), BundleError> {
    match (input, written) {
        (Value::Object(input), Value::Object(written)) => input.iter().try_for_each(|(key, value)| {
            let length = pointer.len();
            push_token(pointer, key);
            let result = match written.get(key) {
                Some(kept) => check_members(value, kept, pointer),
                None if value.is_null() => Ok(()),
                None if OMITTED_DEFAULTS.contains(&key.as_str()) && (value == &Value::Bool(false) || value == &Value::Array(vec![])) => Ok(()),
                None => Err(field(pointer, "unknown member")),
            };
            pointer.truncate(length);
            result
        }),
        (Value::Array(input), Value::Array(written)) if input.len() == written.len() => {
            input.iter().zip(written).enumerate().try_for_each(|(index, (value, kept))| {
                let length = pointer.len();
                push_token(pointer, &index.to_string());
                let result = check_members(value, kept, pointer);
                pointer.truncate(length);
                result
            })
        }
        (input, written) if input == written => Ok(()),
        _ => Err(field(pointer, "value is not preserved by the bundle shape")),
    }
}


/// Keywords whose values are instance data, not schemas, so a `$ref` inside
/// them is not a reference.
const DATA_KEYWORDS: [&str; 5] = ["const", "default", "enum", "example", "examples"];

fn decode(token: &str) -> String { token.replace("~1", "/").replace("~0", "~") }

/// Rewrite local references into the component namespace. `Err` carries the
/// offending reference and why.
fn normalize_refs(value: &mut Value, own: &str, registered: bool) -> Result<(), (String, &'static str)> {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref") {
                let renamed = if reference == "#" && registered {
                    Some(own.to_owned())
                } else {
                    reference.strip_prefix("#/$defs/").or_else(|| reference.strip_prefix("#/definitions/")).map(str::to_owned)
                };
                if let Some(name) = renamed {
                    *reference = format!("{COMPONENTS}{name}");
                } else if !reference.starts_with(COMPONENTS) {
                    return Err((reference.clone(), "is not a local component reference; bundles never fetch external schemas"));
                }
            }
            for (key, value) in object.iter_mut() {
                if !DATA_KEYWORDS.contains(&key.as_str()) { normalize_refs(value, own, registered)?; }
            }
            Ok(())
        }
        Value::Array(values) => values.iter_mut().try_for_each(|value| normalize_refs(value, own, registered)),
        _ => Ok(()),
    }
}

fn check_refs(value: &Value, api: &RestApi) -> Result<(), (String, &'static str)> {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get("$ref") {
                let Some(name) = reference.strip_prefix(COMPONENTS) else {
                    return Err((reference.clone(), "is not a local component reference; bundles never fetch external schemas"));
                };
                if api.component(&decode(name)).is_none() {
                    return Err((reference.clone(), "names no component in the bundle"));
                }
            }
            object.iter()
                .filter(|(key, _)| !DATA_KEYWORDS.contains(&key.as_str()))
                .try_for_each(|(_, value)| check_refs(value, api))
        }
        Value::Array(values) => values.iter().try_for_each(|value| check_refs(value, api)),
        _ => Ok(()),
    }
}


fn validate_catalog(api: &RestApi, models: &ModelCatalog) -> Result<(), BundleError> {
    let fail = |message: String| BundleError::Catalog { api: api.name.clone(), message };
    if models.module_path.as_deref().is_some_and(|path| path.trim().is_empty()) {
        return Err(fail("module_path is empty".into()));
    }
    for (index, model) in models.types.iter().enumerate() {
        let name = model_name(model);
        if models.types[..index].iter().any(|other| model_name(other) == name) {
            return Err(fail(format!("model \"{name}\" is listed more than once")));
        }
        let Some(component) = api.component(name).filter(|c| c.rust_path.is_some()) else {
            return Err(fail(format!("model \"{name}\" is not a registered component of the bundle")));
        };
        let mut references = Vec::new();
        match model {
            ModelDef::Alias(alias) => references.push(&alias.target),
            ModelDef::Struct(model) => {
                references.extend(model.fields.iter().map(|f| &f.field_type));
                references.extend(model.additional_properties.as_ref());
            }
            ModelDef::Enum(_) => {}
        }
        for reference in references {
            if let Some(member) = reference.names().into_iter().find(|n| api.component(n).is_none_or(|c| c.rust_path.is_none())) {
                return Err(fail(format!("model \"{name}\" names \"{member}\", which is not a registered component")));
            }
        }
        if let ModelDef::Alias(alias) = model
            && let TypeRef::OneOf(members) | TypeRef::AnyOf(members) | TypeRef::AllOf(members) = &alias.target {
            if members.is_empty() || members.iter().any(|m| !matches!(m, TypeRef::Named(_))) {
                return Err(fail(format!("composition \"{name}\" needs one or more component members")));
            }
            if component.schema != alias.target.wire_schema() {
                return Err(fail(format!("composition \"{name}\" disagrees with its component schema")));
            }
        }
    }
    Ok(())
}

fn model_name(model: &ModelDef) -> &str {
    match model {
        ModelDef::Struct(model) => &model.name,
        ModelDef::Enum(model) => &model.name,
        ModelDef::Alias(model) => &model.name,
    }
}


/// Drop `null` members outside component schemas: the definition types
/// serialize some absent options as `null`, and a bundle omits them.
fn omit_nulls(value: &mut Value, pointer: &mut String) {
    match value {
        Value::Object(object) => {
            object.retain(|key, value| {
                let length = pointer.len();
                push_token(pointer, key);
                let keep = is_schema_payload(pointer) || !value.is_null();
                pointer.truncate(length);
                keep
            });
            for (key, value) in object.iter_mut() {
                let length = pointer.len();
                push_token(pointer, key);
                if !is_schema_payload(pointer) { omit_nulls(value, pointer); }
                pointer.truncate(length);
            }
        }
        Value::Array(values) => for (index, value) in values.iter_mut().enumerate() {
            let length = pointer.len();
            push_token(pointer, &index.to_string());
            omit_nulls(value, pointer);
            pointer.truncate(length);
        },
        _ => {}
    }
}

/// Sort object keys at every depth. `serde_json::Map` is ordered by key unless
/// a crate in the build enables `preserve_order`, so sort explicitly.
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries: Vec<(String, Value)> = object.into_iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            Value::Object(entries.into_iter().map(|(key, value)| (key, canonical(value))).collect())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        other => other,
    }
}
