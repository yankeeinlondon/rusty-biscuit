//! Example values built from an API's registered JSON Schemas.
//!
//! [`ExampleSource`] builds one example per [`TypeRef`] from
//! [`RestApi::schemas`], the components registered with the definition, not
//! from their OpenAPI conversion. Every value it returns validates against
//! the schema it was built for; a schema it cannot satisfy yields an
//! [`ExampleError`] naming the schema path, never an invalid value. The same
//! input always gives the same bytes: nothing reads a clock or a random
//! source.
//!
//! A schema's value is the first of these that validates against it: its
//! first `examples` entry, `example`, `default`, `const`, each `enum` value
//! in order, then a value built from its type. A built object holds its
//! required properties plus those declaring an example or default; an array
//! holds `minItems` items; strings, numbers, and integers take a typed
//! placeholder moved into their declared bounds. `allOf` members are merged
//! into one schema, and `oneOf`/`anyOf` branches are tried in declaration
//! order until one validates against the whole schema, so a `oneOf` value
//! matches exactly one branch. A component that would expand inside itself
//! more than three times fails; an optional property or a nullable member
//! absorbs that failure by omission or `null`. There is no regex solver: a
//! `pattern` the placeholder misses is reported, not satisfied.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use jsonschema::{Draft, Registry, Validator};
use schematic_define::{RestApi, TypeRef};
use serde_json::{Map, Value, json};

/// The base URI the API's components are registered under, so a validator
/// can reference any subschema by pointer.
const BASE: &str = "https://schematic.invalid/components";

/// How many times one component may expand inside itself on one path.
const MAX_DEPTH: usize = 3;

/// Keywords whose values are instance data, never schemas.
const DATA_KEYWORDS: [&str; 5] = ["const", "default", "enum", "example", "examples"];

/// Why no example could be built for a schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleError {
    /// Where the failing schema is: `#/components/schemas/{name}/…` inside a
    /// registered component, or `#` for the reference's own wrapper schema.
    pub schema_path: String,
    /// What could not be satisfied.
    pub reason: String,
}

impl fmt::Display for ExampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "schema {}: {}", self.schema_path, self.reason)
    }
}

impl std::error::Error for ExampleError {}

/// Builds validated examples from one API's schema components.
///
/// ## Examples
///
/// ```
/// use schematic_define::{DefinitionContext, Endpoint, RestApi, TypeRef};
/// use schematic_gen::export::examples::ExampleSource;
///
/// let mut ctx = DefinitionContext::types_in("crate::types");
/// let endpoint = Endpoint::get("Count", "/count").describes("Count").returns_json::<u32>(&mut ctx).build().unwrap();
/// let api = RestApi::builder(&ctx, "Counter", "https://counter.test").describes("Counter").no_auth()
///     .endpoints([endpoint]).build().unwrap();
///
/// let source = ExampleSource::new(&api);
/// let schematic_define::ResponseShape::Json(reference) = &api.endpoints[0].response[0].shape else { unreachable!() };
/// assert_eq!(source.example(reference).unwrap(), serde_json::json!(0));
/// ```
pub struct ExampleSource {
    /// `{"components": {"schemas": …}}`, with OpenAPI 3.0 `nullable` read as
    /// a `null` type member.
    document: Value,
    registry: Result<Registry<'static>, String>,
    validators: RefCell<HashMap<String, Result<Rc<Validator>, String>>>,
}

impl ExampleSource {
    /// A source over `api`'s registered components.
    pub fn new(api: &RestApi) -> Self {
        let schemas: Map<String, Value> = api
            .schemas
            .iter()
            .map(|component| (component.name.clone(), component.schema.clone()))
            .collect();
        let mut document = json!({"components": {"schemas": schemas}});
        normalize_nullable(&mut document);
        let registry = Registry::new()
            .draft(Draft::Draft202012)
            .add(BASE, document.clone())
            .and_then(|builder| builder.prepare())
            .map_err(|error| format!("the API's schema components do not form a schema registry: {error}"));
        Self { document, registry, validators: RefCell::new(HashMap::new()) }
    }

    /// A value that validates against `reference`.
    ///
    /// ## Errors
    ///
    /// [`ExampleError`] when no candidate validates: a required property
    /// recursing deeper than three levels, an unsatisfiable `allOf`, no
    /// matching `oneOf`/`anyOf` branch, or a constraint (such as `pattern`)
    /// the placeholders cannot meet.
    pub fn example(&self, reference: &TypeRef) -> Result<Value, ExampleError> {
        let mut schema = reference.wire_schema();
        normalize_nullable(&mut schema);
        let value = Walk { source: self, stack: Vec::new() }.build(&schema, &At::Inline)?;
        self.validate(&At::Inline, &schema, &value)?;
        Ok(value)
    }

    /// `Ok` when `value` validates against `reference`.
    ///
    /// ## Errors
    ///
    /// [`ExampleError`] naming the first violated schema location.
    pub fn check(&self, reference: &TypeRef, value: &Value) -> Result<(), ExampleError> {
        let mut schema = reference.wire_schema();
        normalize_nullable(&mut schema);
        self.validate(&At::Inline, &schema, value)
    }

    /// Validates `value` against the schema `node` found at `at`.
    fn validate(&self, at: &At, node: &Value, value: &Value) -> Result<(), ExampleError> {
        let validator = match at {
            At::Doc(pointer) => {
                let mut cache = self.validators.borrow_mut();
                let entry = cache.entry(pointer.clone()).or_insert_with(|| {
                    self.compile(&json!({"$ref": format!("{BASE}#{}", encode_fragment(pointer))})).map(Rc::new)
                });
                entry.clone()
            }
            At::Inline => {
                let mut absolute = node.clone();
                absolutize(&mut absolute);
                self.compile(&absolute).map(Rc::new)
            }
        }
        .map_err(|reason| at.error(reason))?;
        match validator.iter_errors(value).next() {
            None => Ok(()),
            Some(error) => Err(ExampleError {
                schema_path: at.path(),
                reason: format!(
                    "the built value {} fails `{}` at instance path `{}`: {}",
                    compact(value),
                    error.schema_path(),
                    error.instance_path(),
                    error
                ),
            }),
        }
    }

    fn compile(&self, schema: &Value) -> Result<Validator, String> {
        let registry = self.registry.as_ref().map_err(Clone::clone)?;
        jsonschema::options()
            .with_draft(Draft::Draft202012)
            .with_registry(registry)
            .offline()
            .build(schema)
            .map_err(|error| format!("the schema does not compile: {error}"))
    }

    /// The schema a `$ref` names, and its document pointer.
    fn resolve(&self, reference: &str) -> Option<(String, &Value)> {
        let pointer = percent_decode(reference.strip_prefix('#')?)?;
        let target = self.document.pointer(&pointer)?;
        Some((pointer, target))
    }
}

/// Where a schema node lives: at a pointer into the component document, or
/// inline in a wrapper schema built for one reference.
#[derive(Debug, Clone)]
enum At {
    Doc(String),
    Inline,
}

impl At {
    fn child(&self, tokens: &[&str]) -> Self {
        match self {
            Self::Doc(pointer) => {
                let mut pointer = pointer.clone();
                for token in tokens {
                    pointer.push('/');
                    pointer.push_str(&token.replace('~', "~0").replace('/', "~1"));
                }
                Self::Doc(pointer)
            }
            Self::Inline => Self::Inline,
        }
    }

    fn path(&self) -> String {
        match self {
            Self::Doc(pointer) => format!("#{pointer}"),
            Self::Inline => "#".to_string(),
        }
    }

    fn error(&self, reason: impl Into<String>) -> ExampleError {
        ExampleError { schema_path: self.path(), reason: reason.into() }
    }
}

/// One example build: the components being expanded on the current path.
struct Walk<'a> {
    source: &'a ExampleSource,
    stack: Vec<String>,
}

impl Walk<'_> {
    fn build(&mut self, node: &Value, at: &At) -> Result<Value, ExampleError> {
        let object = match node {
            Value::Bool(true) => return Ok(json!({})),
            Value::Bool(false) => return Err(at.error("the schema accepts no value")),
            Value::Object(object) => object,
            _ => return Err(at.error("the schema is not an object or boolean")),
        };

        for candidate in authored_candidates(object) {
            if self.source.validate(at, node, candidate).is_ok() {
                return Ok(candidate.clone());
            }
        }

        if let Some(reference) = object.get("$ref") {
            return self.follow(reference, at);
        }
        if object.contains_key("allOf") {
            let merged = Value::Object(self.merge_all_of(object, at, 0)?);
            let value = self.build(&merged, &At::Inline)?;
            self.source.validate(at, node, &value)?;
            return Ok(value);
        }
        for keyword in ["oneOf", "anyOf"] {
            if let Some(branches) = object.get(keyword) {
                return self.choose_branch(object, keyword, branches, node, at);
            }
        }
        self.build_typed(object, at)
    }

    fn follow(&mut self, reference: &Value, at: &At) -> Result<Value, ExampleError> {
        let reference = reference.as_str().ok_or_else(|| at.error("`$ref` is not a string"))?;
        let (pointer, target) = self
            .source
            .resolve(reference)
            .ok_or_else(|| at.error(format!("`$ref` {reference} does not name a registered component")))?;
        if self.stack.iter().filter(|seen| **seen == pointer).count() >= MAX_DEPTH {
            return Err(at.error(format!(
                "`{reference}` is required inside itself more than {MAX_DEPTH} levels deep"
            )));
        }
        self.stack.push(pointer.clone());
        let built = self.build(target, &At::Doc(pointer));
        self.stack.pop();
        built
    }

    /// `object` without its `allOf`, with every member's keywords added:
    /// properties and required names unioned, other keywords kept from the
    /// first schema declaring them.
    fn merge_all_of(&self, object: &Map<String, Value>, at: &At, depth: usize) -> Result<Map<String, Value>, ExampleError> {
        if depth > 8 {
            return Err(at.error("`allOf` members nest more than eight levels deep"));
        }
        let mut merged: Map<String, Value> = object.iter().filter(|(key, _)| *key != "allOf").map(|(k, v)| (k.clone(), v.clone())).collect();
        let members = object.get("allOf").and_then(Value::as_array).ok_or_else(|| at.error("`allOf` is not an array"))?;
        for member in members {
            let mut member = member.as_object().cloned().ok_or_else(|| at.error("an `allOf` member is not an object schema"))?;
            if let Some(reference) = member.get("$ref").and_then(Value::as_str) {
                let (_, target) = self
                    .source
                    .resolve(reference)
                    .ok_or_else(|| at.error(format!("`$ref` {reference} does not name a registered component")))?;
                let mut target = target.as_object().cloned().ok_or_else(|| at.error("an `allOf` member is not an object schema"))?;
                // The member's own keywords next to `$ref` still apply.
                for (key, value) in member.into_iter().filter(|(key, _)| key != "$ref") {
                    target.entry(key).or_insert(value);
                }
                member = target;
            }
            if member.contains_key("allOf") {
                member = self.merge_all_of(&member, at, depth + 1)?;
            }
            for (key, value) in member {
                match (key.as_str(), merged.get_mut(&key)) {
                    ("properties", Some(Value::Object(existing))) => {
                        for (name, schema) in value.as_object().into_iter().flatten() {
                            existing.entry(name.clone()).or_insert_with(|| schema.clone());
                        }
                    }
                    ("required", Some(Value::Array(existing))) => {
                        for name in value.as_array().into_iter().flatten() {
                            if !existing.contains(name) {
                                existing.push(name.clone());
                            }
                        }
                    }
                    (_, Some(_)) => {}
                    (_, None) => {
                        merged.insert(key, value);
                    }
                }
            }
        }
        Ok(merged)
    }

    /// The first branch, in declaration order, whose value validates against
    /// the whole schema (`oneOf` also rejects a value two branches accept).
    fn choose_branch(&mut self, object: &Map<String, Value>, keyword: &str, branches: &Value, node: &Value, at: &At) -> Result<Value, ExampleError> {
        let branches = branches.as_array().ok_or_else(|| at.error(format!("`{keyword}` is not an array")))?;
        // Sibling properties apply to every branch's object.
        let siblings: Map<String, Value> = object.iter().filter(|(key, _)| *key != keyword).map(|(k, v)| (k.clone(), v.clone())).collect();
        let shared = if siblings.contains_key("properties") || siblings.contains_key("required") {
            match self.build_typed(&siblings, at)? {
                Value::Object(shared) => Some(shared),
                _ => None,
            }
        } else {
            None
        };
        let mut last = None;
        for (index, branch) in branches.iter().enumerate() {
            let built = match self.build(branch, &at.child(&[keyword, &index.to_string()])) {
                Ok(built) => built,
                Err(error) => {
                    last = Some(error);
                    continue;
                }
            };
            let candidate = match (&shared, built) {
                (Some(shared), Value::Object(own)) => {
                    let mut combined = shared.clone();
                    combined.extend(own);
                    Value::Object(combined)
                }
                (_, built) => built,
            };
            match self.source.validate(at, node, &candidate) {
                Ok(()) => return Ok(candidate),
                Err(error) => last = Some(error),
            }
        }
        Err(match last {
            Some(last) => at.error(format!("no `{keyword}` branch yields a valid value; the last failure: {last}")),
            None => at.error(format!("`{keyword}` has no branches")),
        })
    }

    /// A value built from the schema's type: each declared type in order,
    /// `null` last, so a nullable schema falls back to `null`.
    fn build_typed(&mut self, object: &Map<String, Value>, at: &At) -> Result<Value, ExampleError> {
        let mut types: Vec<&str> = match object.get("type") {
            Some(Value::String(kind)) => vec![kind.as_str()],
            Some(Value::Array(kinds)) => kinds.iter().filter_map(Value::as_str).collect(),
            Some(_) => return Err(at.error("`type` is neither a string nor an array")),
            None => vec![inferred_type(object)],
        };
        types.sort_by_key(|kind| *kind == "null");
        let mut last = None;
        for kind in types {
            let built = match kind {
                "null" => Ok(Value::Null),
                "boolean" => Ok(Value::Bool(false)),
                "integer" => number(object, true, at),
                "number" => number(object, false, at),
                "string" => Ok(Value::String(string(object))),
                "array" => self.array(object, at),
                "object" => self.object(object, at),
                "any" => Ok(json!({})),
                other => Err(at.error(format!("unsupported type `{other}`"))),
            };
            match built {
                Ok(value) => return Ok(value),
                Err(error) => last = Some(error),
            }
        }
        Err(last.unwrap_or_else(|| at.error("`type` lists no types")))
    }

    fn array(&mut self, object: &Map<String, Value>, at: &At) -> Result<Value, ExampleError> {
        let count = object.get("minItems").and_then(Value::as_u64).unwrap_or(0);
        let prefix = object.get("prefixItems").and_then(Value::as_array);
        let mut items = Vec::new();
        for index in 0..count {
            let index_text = index.to_string();
            let item = match prefix.and_then(|prefix| prefix.get(index as usize)) {
                Some(schema) => self.build(schema, &at.child(&["prefixItems", &index_text]))?,
                None => match object.get("items") {
                    Some(schema) => self.build(schema, &at.child(&["items"]))?,
                    None => json!({}),
                },
            };
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn object(&mut self, object: &Map<String, Value>, at: &At) -> Result<Value, ExampleError> {
        let properties = object.get("properties").and_then(Value::as_object);
        let required: Vec<&str> = object.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
        let mut built = Map::new();
        for (name, schema) in properties.into_iter().flatten() {
            let is_required = required.contains(&name.as_str());
            if !is_required && !schema.as_object().is_some_and(declares_example) {
                continue;
            }
            match self.build(schema, &at.child(&["properties", name])) {
                Ok(value) => {
                    built.insert(name.clone(), value);
                }
                Err(_) if !is_required => {}
                Err(error) => return Err(error),
            }
        }
        let additional = object.get("additionalProperties");
        for name in &required {
            if built.contains_key(*name) {
                continue;
            }
            let value = match additional {
                Some(schema @ Value::Object(_)) => self.build(schema, &at.child(&["additionalProperties"]))?,
                _ => json!({}),
            };
            built.insert((*name).to_string(), value);
        }
        let minimum = object.get("minProperties").and_then(Value::as_u64).unwrap_or(0) as usize;
        for (name, schema) in properties.into_iter().flatten() {
            if built.len() >= minimum {
                break;
            }
            if !built.contains_key(name)
                && let Ok(value) = self.build(schema, &at.child(&["properties", name]))
            {
                built.insert(name.clone(), value);
            }
        }
        let mut extra = 1;
        while built.len() < minimum {
            let value = match additional {
                Some(schema @ Value::Object(_)) => self.build(schema, &at.child(&["additionalProperties"]))?,
                _ => json!({}),
            };
            built.insert(format!("key{extra}"), value);
            extra += 1;
        }
        Ok(Value::Object(built))
    }
}

/// The schema's authored values, in priority order: first `examples` entry,
/// `example`, `default`, `const`, then every `enum` value.
fn authored_candidates(object: &Map<String, Value>) -> Vec<&Value> {
    let mut candidates = Vec::new();
    if let Some(first) = object.get("examples").and_then(Value::as_array).and_then(|examples| examples.first()) {
        candidates.push(first);
    }
    candidates.extend(["example", "default", "const"].into_iter().filter_map(|key| object.get(key)));
    candidates.extend(object.get("enum").and_then(Value::as_array).into_iter().flatten());
    candidates
}

/// Whether an optional property names its own example or default, which
/// puts it in the built object.
fn declares_example(object: &Map<String, Value>) -> bool {
    object.get("examples").and_then(Value::as_array).is_some_and(|examples| !examples.is_empty())
        || object.contains_key("example")
        || object.contains_key("default")
}

/// The type a schema without `type` describes, from its keywords.
fn inferred_type(object: &Map<String, Value>) -> &'static str {
    let has = |keys: &[&str]| keys.iter().any(|key| object.contains_key(*key));
    if has(&["properties", "required", "additionalProperties", "minProperties", "patternProperties"]) {
        "object"
    } else if has(&["items", "prefixItems", "minItems"]) {
        "array"
    } else if has(&["minLength", "maxLength", "pattern", "format"]) {
        "string"
    } else if has(&["minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf"]) {
        "number"
    } else {
        "any"
    }
}

/// `0` moved into the schema's bounds and onto its `multipleOf` grid.
fn number(object: &Map<String, Value>, integer: bool, at: &At) -> Result<Value, ExampleError> {
    let bound = |inclusive: &str, exclusive: &str| -> Option<(f64, bool)> {
        match object.get(exclusive) {
            Some(Value::Number(limit)) => limit.as_f64().map(|limit| (limit, true)),
            _ => object.get(inclusive).and_then(Value::as_f64).map(|limit| (limit, false)),
        }
    };
    let low = bound("minimum", "exclusiveMinimum");
    let high = bound("maximum", "exclusiveMaximum");
    let step = object.get("multipleOf").and_then(Value::as_f64).filter(|step| *step > 0.0);
    let unit = if integer { 1.0 } else { step.unwrap_or(1.0) };

    let mut value: f64 = 0.0;
    if let Some((limit, exclusive)) = low
        && (value < limit || (exclusive && value <= limit))
    {
        value = if integer { limit.ceil() } else { limit };
        if exclusive && value <= limit {
            value += unit;
        }
    }
    if let Some(step) = step {
        value = (value / step).ceil() * step;
    }
    if let Some((limit, exclusive)) = high
        && (value > limit || (exclusive && value >= limit))
    {
        value = if integer { limit.floor() } else { limit };
        if exclusive && value >= limit {
            value -= unit;
        }
        if let Some(step) = step {
            value = (value / step).floor() * step;
        }
    }
    if integer || value.fract() == 0.0 && value.abs() < 9.0e15 {
        return Ok(json!(value as i64));
    }
    serde_json::Number::from_f64(value).map(Value::Number).ok_or_else(|| at.error("the numeric bounds leave no finite value"))
}

/// A placeholder for the string's `format`, padded to `minLength` and cut to
/// `maxLength`.
fn string(object: &Map<String, Value>) -> String {
    let mut text = match object.get("format").and_then(Value::as_str) {
        Some("date-time") => "2000-01-01T00:00:00Z",
        Some("date") => "2000-01-01",
        Some("time") => "00:00:00Z",
        Some("duration") => "P1D",
        Some("uuid") => "00000000-0000-0000-0000-000000000000",
        Some("email") => "user@example.com",
        Some("hostname") => "example.com",
        Some("ipv4") => "192.0.2.1",
        Some("ipv6") => "2001:db8::1",
        Some("uri" | "url" | "iri") => "https://example.com",
        Some("byte") => "c3RyaW5n",
        _ => "string",
    }
    .to_string();
    let length = |key: &str| object.get(key).and_then(Value::as_u64).map(|n| n as usize);
    if let Some(minimum) = length("minLength") {
        while text.chars().count() < minimum {
            text.push('x');
        }
    }
    if let Some(maximum) = length("maxLength") {
        text = text.chars().take(maximum).collect();
    }
    text
}

/// Reads OpenAPI 3.0 `nullable: true` as a `null` member of `type`, which is
/// how every JSON Schema draft spells it, outside instance data.
fn normalize_nullable(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if object.get("nullable") == Some(&Value::Bool(true)) {
                object.remove("nullable");
                match object.get_mut("type") {
                    Some(Value::String(kind)) => {
                        let kind = kind.clone();
                        object.insert("type".into(), json!([kind, "null"]));
                    }
                    Some(Value::Array(kinds)) if !kinds.contains(&json!("null")) => kinds.push(json!("null")),
                    _ => {}
                }
            }
            for (key, child) in object.iter_mut() {
                if !DATA_KEYWORDS.contains(&key.as_str()) {
                    normalize_nullable(child);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(normalize_nullable),
        _ => {}
    }
}

/// Rewrites document-relative `$ref`s (`#/components/…`) against [`BASE`],
/// so an inline schema compiled on its own resolves them in the registry.
fn absolutize(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref")
                && let Some(pointer) = reference.strip_prefix('#').and_then(percent_decode)
            {
                *reference = format!("{BASE}#{}", encode_fragment(&pointer));
            }
            for (key, child) in object.iter_mut() {
                if !DATA_KEYWORDS.contains(&key.as_str()) {
                    absolutize(child);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(absolutize),
        _ => {}
    }
}

/// A JSON pointer as a URI fragment: every byte outside the unreserved set
/// and `/` percent-encoded.
fn encode_fragment(pointer: &str) -> String {
    let mut encoded = String::with_capacity(pointer.len());
    for byte in pointer.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// A URI fragment's percent-escapes decoded; `None` when malformed.
fn percent_decode(fragment: &str) -> Option<String> {
    let bytes = fragment.as_bytes();
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

/// `value` as compact JSON, cut to 120 characters for a diagnostic.
fn compact(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 120 { format!("{}…", text.chars().take(120).collect::<String>()) } else { text }
}

#[cfg(test)]
mod tests;
