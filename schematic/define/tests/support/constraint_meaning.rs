//! The value rules of a `RestApi`, keyed by where they sit and normalized
//! by meaning, so two routes that spell a rule differently compare equal.
//!
//! Shared by `schematic-define`'s direct import/export acceptance test and
//! by `schematic-gen`'s definitions-out acceptance test, which pastes this
//! file into a freshly generated crate; it uses only `schematic_define` and
//! `serde_json`.
//!
//! Normalization: numbers compare by exact decimal value (`1`, `1.0`, and
//! `1e0` agree), a numeric exclusive bound and an OpenAPI 3.0 Boolean flag
//! with its bound agree, `const: c` equals `enum: [c]`, a component used
//! only as a primitive alias contributes its rules where it is referenced,
//! a nullable wrapper contributes the rules of its non-null member, each
//! member of any other `anyOf`, `oneOf`, multi-member `allOf`, or `not` is
//! its own node (`{at}/anyOf/1`, `{at}/not`), and
//! component nodes compare without the `format` values `int64` and
//! `double`, the annotations Schemars writes for a model field's `i64` and
//! `f64` where the source wrote none.

use std::collections::BTreeMap;

use schematic_define::constraints::{ConstraintNumber, Constraints};
use schematic_define::{ApiRequest, RestApi};
use serde_json::{Map, Value};

/// One location's rules: keyword → normalized value.
pub type Rules = BTreeMap<&'static str, String>;

/// Every non-empty rule set of `api`: parameters (`param:{endpoint}:{in}.{name}`),
/// form fields (`form:{endpoint}:{name}`), body roots (`body:{endpoint}`), and
/// each component node (`schema:{component}{path}`, through `properties`,
/// `items`, `additionalProperties`, and the members of a generic
/// composition) for the components named in `components`.
pub fn rule_map(api: &RestApi, components: &[&str]) -> BTreeMap<String, Rules> {
    let mut map = BTreeMap::new();
    for endpoint in &api.endpoints {
        if let Some(params) = &endpoint.params {
            for (location, params) in [("path", &params.path), ("query", &params.query), ("header", &params.header), ("cookie", &params.cookie)] {
                for param in params {
                    insert(&mut map, format!("param:{}:{location}.{}", endpoint.id, param.name.to_ascii_lowercase()), param.constraints.as_deref());
                }
            }
        }
        if let Some(ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields }) = &endpoint.request {
            for field in fields {
                insert(&mut map, format!("form:{}:{}", endpoint.id, field.name), field.constraints.as_deref());
            }
        }
        insert(&mut map, format!("body:{}", endpoint.id), endpoint.request_constraints.as_deref());
    }
    for component in api.schemas.iter().filter(|component| components.contains(&component.name.as_str())) {
        walk(api, &component.schema, &format!("schema:{}", component.name), &mut map);
    }
    map
}

fn insert(map: &mut BTreeMap<String, Rules>, key: String, rules: Option<&Constraints>) {
    if let Some(rules) = rules {
        let rules = normalize(rules);
        if !rules.is_empty() {
            map.insert(key, rules);
        }
    }
}

/// Collects the rules at `schema` and below.
fn walk(api: &RestApi, schema: &Value, at: &str, map: &mut BTreeMap<String, Rules>) {
    let Some(object) = schema.as_object() else { return };
    let mut rules = node_rules(api, object);
    for wrapper in ["anyOf", "oneOf"] {
        if let Some(members) = nullable_member(object, wrapper) {
            rules.extend(node_rules(api, members));
            walk_children(api, members, at, map);
        }
    }
    if !rules.is_empty() {
        map.insert(at.to_owned(), rules);
    }
    walk_children(api, object, at, map);
}

fn walk_children(api: &RestApi, object: &Map<String, Value>, at: &str, map: &mut BTreeMap<String, Rules>) {
    if let Some(Value::Object(properties)) = object.get("properties") {
        for (name, property) in properties {
            walk(api, property, &format!("{at}/properties/{name}"), map);
        }
    }
    for keyword in ["items", "additionalProperties", "not"] {
        if let Some(child) = object.get(keyword) {
            walk(api, child, &format!("{at}/{keyword}"), map);
        }
    }
    for keyword in ["anyOf", "oneOf", "allOf"] {
        let Some(members) = object.get(keyword).and_then(Value::as_array) else { continue };
        let wrapper = match keyword {
            "allOf" => members.len() == 1,
            _ => nullable_member(object, keyword).is_some(),
        };
        if !wrapper {
            for (index, member) in members.iter().enumerate() {
                walk(api, member, &format!("{at}/{keyword}/{index}"), map);
            }
        }
    }
}

/// The non-null member of a two-member `anyOf`/`oneOf` whose other member
/// admits only `null`.
fn nullable_member<'a>(object: &'a Map<String, Value>, keyword: &str) -> Option<&'a Map<String, Value>> {
    let members = object.get(keyword)?.as_array()?;
    let only_null = |member: &Value| {
        member.get("type") == Some(&Value::from("null"))
            || member.get("enum").and_then(Value::as_array).is_some_and(|values| values.iter().all(Value::is_null))
    };
    match members.as_slice() {
        [first, second] if only_null(second) => first.as_object(),
        [first, second] if only_null(first) => second.as_object(),
        _ => None,
    }
}

/// The rules written on `object`, plus those of a primitive alias component
/// it references (directly or as the only `allOf` member).
fn node_rules(api: &RestApi, object: &Map<String, Value>) -> Rules {
    let mut rules = normalize(&schema_rules(object));
    drop_type_format(&mut rules);
    let reference = object.get("$ref").or_else(|| object.get("allOf")?.as_array()?.first()?.get("$ref"));
    if let Some(name) = reference.and_then(Value::as_str).and_then(|reference| reference.rsplit('/').next())
        && let Some(target) = api.schemas.iter().find(|component| component.name == name)
        && let Some(target) = target.schema.as_object()
        && !target.contains_key("properties")
        && matches!(target.get("type").and_then(Value::as_str), Some("integer" | "number" | "string" | "boolean"))
    {
        rules.extend(normalize(&schema_rules(target)));
        drop_type_format(&mut rules);
    }
    rules
}

/// Removes a `format` Schemars writes for the Rust type of a model field
/// (`i64`, `f64`) where the source wrote none; a source `format` is rebuilt
/// and compared.
fn drop_type_format(rules: &mut Rules) {
    if rules.get("format").is_some_and(|format| format == "int64" || format == "double") {
        rules.remove("format");
    }
}

/// `Constraints::from_schema`, except that a schema's `items` are compared
/// as their own node and never folded in here.
fn schema_rules(object: &Map<String, Value>) -> Constraints {
    let mut object = object.clone();
    object.remove("items");
    let mut rules = Constraints::from_schema(&object).expect("compared schemas carry well-formed rules");
    // A nullable enum lists `null`; whether the value is nullable is the
    // type's business, not the rule's.
    if let Some(values) = &mut rules.allowed_values {
        values.retain(|value| !value.is_null());
    }
    rules
}

/// One rule set as keyword → canonical text.
pub fn normalize(rules: &Constraints) -> Rules {
    let mut out = Rules::new();
    let mut put = |keyword: &'static str, value: Option<String>| {
        if let Some(value) = value {
            out.insert(keyword, value);
        }
    };
    put("minLength", rules.min_length.map(|count| count.to_string()));
    put("maxLength", rules.max_length.map(|count| count.to_string()));
    put("pattern", rules.pattern.clone());
    put("format", rules.format.clone());
    put("minimum", rules.minimum.as_ref().map(canonical));
    put("maximum", rules.maximum.as_ref().map(canonical));
    put("exclusiveMinimum", rules.exclusive_minimum.as_ref().map(canonical));
    put("exclusiveMaximum", rules.exclusive_maximum.as_ref().map(canonical));
    put("multipleOf", rules.multiple_of.as_ref().map(canonical));
    put("minItems", rules.min_items.map(|count| count.to_string()));
    put("maxItems", rules.max_items.map(|count| count.to_string()));
    put("uniqueItems", rules.unique_items.then(|| "true".to_owned()));
    let values = match (&rules.allowed_values, &rules.constant) {
        (Some(values), _) => Some(values.clone()),
        (None, Some(constant)) => Some(vec![constant.clone()]),
        (None, None) => None,
    };
    put("enum", values.map(|values| values.iter().map(canonical_value).collect::<Vec<_>>().join(",")));
    put("items", rules.items.as_deref().map(|items| format!("{:?}", normalize(items))));
    out
}

/// A JSON value with its numbers in canonical form.
fn canonical_value(value: &Value) -> String {
    match value {
        Value::Number(number) => canonical(&ConstraintNumber::from(number.to_string())),
        other => other.to_string(),
    }
}

/// Exact decimal value as `[-]digits e exponent`, with no leading or
/// trailing zero digits; zero is `0`.
pub fn canonical(number: &ConstraintNumber) -> String {
    let text = number.as_str();
    let (negative, unsigned) = match text.strip_prefix('-') { Some(rest) => (true, rest), None => (false, text) };
    let (mantissa, written) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.trim_start_matches('+').parse::<i64>().expect("exponent")),
        None => (unsigned, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{integer}{fraction}");
    let trimmed = digits.trim_start_matches('0');
    let significant = trimmed.trim_end_matches('0');
    if significant.is_empty() {
        return "0".to_owned();
    }
    let exponent = written - fraction.len() as i64 + (trimmed.len() - significant.len()) as i64;
    format!("{}{significant}e{exponent}", if negative { "-" } else { "" })
}
