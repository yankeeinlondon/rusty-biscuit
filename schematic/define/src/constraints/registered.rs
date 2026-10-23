//! Rules on registered models that sit where a generated client cannot
//! enforce them.
//!
//! A registered model's rules live only in its stored schema component (see
//! `schematic/docs/typed-registration.md#value-rules-on-registered-models`).
//! [`unenforceable_rules`] walks a JSON body's schema through those
//! components and lists the rules beneath a composition other than an
//! `Option`, or beside a reference that names no component.

use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::{RestApi, SchemaComponent, TypeRef};

/// The validation keywords a rule can use. Annotations (`title`,
/// `description`, `format`) are never rules.
const RULE_KEYWORDS: [&str; 13] = [
    "minLength", "maxLength", "pattern", "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum",
    "multipleOf", "minItems", "maxItems", "uniqueItems", "enum", "const",
];

/// Prefixes of a local reference to a stored component. Schemars writes
/// `#/components/schemas/` under the repository's settings; the other two
/// are its draft 2020-12 and draft-07 spellings of the same name.
const COMPONENT_PREFIXES: [&str; 3] = ["/components/schemas/", "/$defs/", "/definitions/"];

/// One rule of a JSON body's schema that cannot be enforced where it sits.
///
/// ## Examples
///
/// See [`unenforceable_rules`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnenforceableRule {
    /// The schema component holding the rule; `None` for the body's own
    /// wire schema ([`TypeRef::wire_schema`]), which only a container or
    /// primitive body has.
    pub component: Option<String>,
    /// JSON pointer, inside that schema, to the schema object carrying the
    /// keyword; `""` is the schema's root.
    pub pointer: String,
    /// The rule's JSON Schema keyword (`maxLength`, `enum`, ...).
    pub keyword: &'static str,
    /// Why it cannot be enforced, naming the composition or reference.
    pub reason: String,
}

/// Every rule of the JSON body `body` that sits beneath `oneOf`, `allOf`,
/// `not`, or an `anyOf` other than an `Option`, or beside a `$ref` that names
/// no component of `api.schemas`, in walk order.
///
/// The walk starts at `body`'s wire schema and follows component references
/// (`#/components/schemas/{name}`, `#/$defs/{name}`, `#/definitions/{name}`,
/// and a component's `#` to itself), `properties`, `items`, and
/// `additionalProperties`. An `anyOf` is an `Option`, and is walked through,
/// only when it has exactly two members and one admits only `null`. Each
/// component is walked at most once outside a composition and once beneath
/// one, so a recursive model terminates.
///
/// The bounds Schemars writes for a Rust integer width (`minimum: 0` on a
/// `uint32`, `0..=255` on a `uint8`) are the type's, not authored rules, and
/// are never listed. A unit enum's `enum` is listed like any other rule.
///
/// ## Examples
///
/// ```
/// use schematic_define::constraints::unenforceable_rules;
/// use schematic_define::{DefinitionContext, Endpoint, RestApi, TypeRef, definition_type};
/// use schemars::JsonSchema;
///
/// definition_type!(#[derive(JsonSchema)] pub struct Tag {
///     #[schemars(length(min = 1, max = 40))]
///     pub label: String,
/// });
///
/// let mut ctx = DefinitionContext::types_in("::docs::models");
/// let create = Endpoint::post("CreateTag", "/tags").describes("Create a tag")
///     .request_json::<Tag>(&mut ctx).returns_empty().build()?;
/// let mut api = RestApi::builder(&ctx, "Tags", "https://example.test")
///     .describes("Tags").no_auth().endpoints([create]).build()?;
/// let body = TypeRef::Named("Tag".into());
/// assert!(unenforceable_rules(&api, &body).is_empty());
///
/// // The same rule beneath a `oneOf` cannot be enforced.
/// let label = api.schemas[0].schema["properties"]["label"].take();
/// api.schemas[0].schema["properties"]["label"] = serde_json::json!({"oneOf": [label, {"type": "integer"}]});
/// let rules = unenforceable_rules(&api, &body);
/// assert_eq!(rules.iter().map(|rule| (rule.pointer.as_str(), rule.keyword)).collect::<Vec<_>>(),
///     [("/properties/label/oneOf/0", "minLength"), ("/properties/label/oneOf/0", "maxLength")]);
/// # Ok::<(), schematic_define::DefinitionError>(())
/// ```
pub fn unenforceable_rules(api: &RestApi, body: &TypeRef) -> Vec<UnenforceableRule> {
    let mut walk = Walk { api, visited: HashSet::new(), found: Vec::new() };
    walk.schema(&body.wire_schema(), None, &mut String::new(), None);
    walk.found
}

struct Walk<'a> {
    api: &'a RestApi,
    /// `(component, beneath a composition)` pairs already walked.
    visited: HashSet<(&'a str, bool)>,
    found: Vec<UnenforceableRule>,
}

impl<'a> Walk<'a> {
    /// Walks the schema `value` of `component` at `pointer`; `blocked` is
    /// the reason every rule found is unenforceable, once one applies.
    fn schema(&mut self, value: &Value, component: Option<&'a str>, pointer: &mut String, blocked: Option<&str>) {
        let Some(object) = value.as_object() else { return };
        if let Some(reason) = blocked {
            self.report(object, component, pointer, reason);
        }
        if let Some(reference) = object.get("$ref") {
            match self.resolve(reference, component) {
                Some(target) => self.component(target, blocked),
                None if blocked.is_none() => {
                    self.report(object, component, pointer, &format!("beside the unresolved reference {reference}"));
                }
                None => {}
            }
        }
        if let Some(properties) = object.get("properties").and_then(Value::as_object) {
            for (name, child) in properties {
                let length = push_token(pointer, "properties");
                push_token(pointer, name);
                self.schema(child, component, pointer, blocked);
                pointer.truncate(length);
            }
        }
        for key in ["items", "additionalProperties"] {
            let length = push_token(pointer, key);
            match object.get(key) {
                // OpenAPI 3.0 settings rewrite `prefixItems` as an `items` list.
                Some(Value::Array(members)) => self.members(members, component, pointer, blocked),
                Some(child) => self.schema(child, component, pointer, blocked),
                None => {}
            }
            pointer.truncate(length);
        }
        for key in ["anyOf", "oneOf", "allOf", "not"] {
            let Some(child) = object.get(key) else { continue };
            let option = key == "anyOf" && child.as_array().is_some_and(|members| members.len() == 2 && members.iter().any(admits_only_null));
            let reason = format!("beneath the `{key}` at {}", place(component, pointer));
            let inner = if option { blocked } else { Some(blocked.unwrap_or(&reason)) };
            let length = push_token(pointer, key);
            match child {
                Value::Array(members) => self.members(members, component, pointer, inner),
                child => self.schema(child, component, pointer, inner),
            }
            pointer.truncate(length);
        }
    }

    /// Walks each member of a schema list, skipping a member that only
    /// marks nullability.
    fn members(&mut self, members: &[Value], component: Option<&'a str>, pointer: &mut String, blocked: Option<&str>) {
        for (index, member) in members.iter().enumerate().filter(|(_, member)| !admits_only_null(member)) {
            let length = push_token(pointer, &index.to_string());
            self.schema(member, component, pointer, blocked);
            pointer.truncate(length);
        }
    }

    fn component(&mut self, target: &'a SchemaComponent, blocked: Option<&str>) {
        if self.visited.insert((target.name.as_str(), blocked.is_some())) {
            self.schema(&target.schema, Some(&target.name), &mut String::new(), blocked);
        }
    }

    fn resolve(&self, reference: &Value, current: Option<&str>) -> Option<&'a SchemaComponent> {
        let reference = reference.as_str()?;
        let name = if reference == "#" {
            current?.to_owned()
        } else {
            let fragment = percent_decode(reference.strip_prefix('#')?)?;
            let token = COMPONENT_PREFIXES.iter().find_map(|prefix| fragment.strip_prefix(prefix))?;
            if token.is_empty() || token.contains('/') { return None }
            token.replace("~1", "/").replace("~0", "~")
        };
        self.api.component(&name)
    }

    fn report(&mut self, object: &Map<String, Value>, component: Option<&str>, pointer: &str, reason: &str) {
        for keyword in RULE_KEYWORDS {
            let Some(value) = object.get(keyword) else { continue };
            if value == &Value::Bool(false) || implicit_bound(object, keyword, value) { continue }
            self.found.push(UnenforceableRule {
                component: component.map(str::to_owned),
                pointer: pointer.to_owned(),
                keyword,
                reason: reason.to_owned(),
            });
        }
    }
}

fn place(component: Option<&str>, pointer: &str) -> String {
    match component {
        Some(name) => format!("`#{pointer}` of component `{name}`"),
        None => format!("`#{pointer}` of the request body"),
    }
}

/// Whether `member` admits only `null`: `{"type": "null"}` (draft 2020-12)
/// or `{"enum": [null], "nullable": true}` (OpenAPI 3.0), with nothing but
/// annotations beside it.
fn admits_only_null(member: &Value) -> bool {
    let Some(member) = member.as_object() else { return false };
    let only_null = |values: &Value| values.as_array().is_some_and(|values| !values.is_empty() && values.iter().all(Value::is_null));
    let restricted = member.get("enum").is_some_and(only_null) || member.get("const") == Some(&Value::Null);
    let null_type = member.get("type").is_some_and(|kind| kind == "null");
    (restricted || null_type)
        && member.keys().all(|key| matches!(key.as_str(), "type" | "nullable" | "enum" | "const" | "title" | "description" | "default" | "example" | "examples"))
}

/// Whether `keyword: value` is the bound Schemars writes for the Rust
/// integer width named by the object's `format`, rather than an authored
/// rule. A rule equal to that bound adds nothing, so the two need not be
/// told apart.
fn implicit_bound(object: &Map<String, Value>, keyword: &str, value: &Value) -> bool {
    let (minimum, maximum): (i64, Option<i64>) = match object.get("format").and_then(Value::as_str) {
        Some("int8") => (i8::MIN.into(), Some(i8::MAX.into())),
        Some("int16") => (i16::MIN.into(), Some(i16::MAX.into())),
        Some("uint8") => (0, Some(u8::MAX.into())),
        Some("uint16") => (0, Some(u16::MAX.into())),
        Some("uint32" | "uint64" | "uint128" | "uint") => (0, None),
        _ => return false,
    };
    match keyword {
        "minimum" => value.as_i64() == Some(minimum),
        "maximum" => maximum.is_some() && value.as_i64() == maximum,
        _ => false,
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

/// `text` with `%XX` escapes decoded; `None` for a malformed escape or a
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
