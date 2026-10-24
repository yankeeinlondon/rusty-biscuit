//! Converts stored JSON Schema components to OpenAPI 3.0.3 schemas.
//!
//! A registered component mixes two dialects: schemars applies its OpenAPI
//! 3.0 transforms (`nullable`, `{"nullable": true, "enum": [null]}` members)
//! only to a registration's root, while the definitions it nests keep JSON
//! Schema 2020-12 forms (`type` arrays, `const`, `{"type": "null"}` members,
//! numeric `exclusiveMinimum`). Both convert to the 3.0.3 schema object, as
//! JSON, and the result deserializes into [`openapiv3::Schema`], whose
//! deserializer keeps mixed keywords by falling back to its `Any` kind.
//!
//! Every keyword either converts or fails the export with its schema path:
//! nothing silently becomes unconstrained.

use openapiv3::Schema;
use serde_json::{Map, Value, json};

use super::super::OpenApiError;

/// Keywords that describe a schema without constraining its values. They stay
/// on the outermost converted schema when its constraints move into a
/// composition.
const ANNOTATIONS: &[&str] =
    &["title", "description", "default", "deprecated", "readOnly", "writeOnly", "example", "externalDocs", "xml", "discriminator"];

/// Keywords dropped without effect on the accepted values.
const IGNORED: &[&str] = &["$schema", "$id", "$comment", "examples"];

/// Constraint keywords copied unchanged once their subschemas, if any, are
/// converted.
const CONSTRAINTS: &[&str] = &[
    "type", "enum", "format", "pattern", "minLength", "maxLength", "minimum", "maximum", "exclusiveMinimum",
    "exclusiveMaximum", "multipleOf", "properties", "required", "additionalProperties", "minProperties",
    "maxProperties", "items", "minItems", "maxItems", "uniqueItems", "oneOf", "anyOf", "allOf", "not", "$ref",
];

/// The keywords a member of a multi-type `anyOf` takes for its `type`.
fn applies_to(kind: &str, keyword: &str) -> bool {
    match keyword {
        "format" => matches!(kind, "string" | "integer" | "number"),
        "pattern" | "minLength" | "maxLength" => kind == "string",
        "minimum" | "maximum" | "exclusiveMinimum" | "exclusiveMaximum" | "multipleOf" => matches!(kind, "integer" | "number"),
        "properties" | "required" | "additionalProperties" | "minProperties" | "maxProperties" => kind == "object",
        "items" | "minItems" | "maxItems" | "uniqueItems" => kind == "array",
        _ => false,
    }
}

/// The OpenAPI 3.0.3 schema that admits only `null`: `nullable` needs a
/// `type`, and the `enum` excludes every non-null object.
fn null_branch() -> Value {
    json!({"type": "object", "nullable": true, "enum": [null]})
}

/// Converts one `components.schemas` entry named `name`.
///
/// ## Errors
///
/// [`OpenApiError::UnsupportedFeature`] naming the first keyword OpenAPI 3.0.3
/// cannot express, with its JSON-pointer schema path as context; or
/// [`OpenApiError::Validation`] at the component's pointer when the converted
/// JSON does not deserialize as an [`openapiv3::Schema`].
pub(super) fn convert_component_schema(name: &str, value: &Value) -> Result<Schema, OpenApiError> {
    convert_schema_at(value, &format!("#/components/schemas/{}", escape(name)))
}

/// Converts a JSON Schema `value` found at the JSON pointer `pointer` (an
/// inline parameter, form-field, or body schema) exactly as
/// [`convert_component_schema`] converts a component.
///
/// ## Errors
///
/// As [`convert_component_schema`], located at `pointer`.
pub(super) fn convert_schema_at(value: &Value, pointer: &str) -> Result<Schema, OpenApiError> {
    let pointer = pointer.to_owned();
    let mut converted = convert(value, &pointer)?;
    restore_exact_numbers(&mut converted);
    // `SchemaData::default` is an `Option`, so a source `"default": null`
    // (schemars writes one for every `#[serde(default)]` optional field) reads
    // as no default. Writing it back would make the document differ from its
    // own `openapiv3` parse, which the importer relies on; the field's
    // `nullable` and absence from `required` say the same thing.
    serde_json::from_value(converted).map_err(|error| OpenApiError::Validation { path: pointer, message: error.to_string() })
}

/// Converts a JSON Schema value at `pointer` to OpenAPI 3.0.3 JSON.
fn convert(value: &Value, pointer: &str) -> Result<Value, OpenApiError> {
    let object = match value {
        Value::Bool(true) => return Ok(json!({})),
        Value::Bool(false) => return Ok(json!({"not": {}})),
        Value::Object(object) => object,
        _ => return Err(unsupported("a schema that is neither an object nor a boolean", pointer)),
    };
    let object = &carry_exact_numbers(object);
    if let Some(keyword) = object.keys().find(|key| {
        !(key.starts_with("x-") || ANNOTATIONS.contains(&key.as_str()) || IGNORED.contains(&key.as_str())
            || CONSTRAINTS.contains(&key.as_str()) || matches!(key.as_str(), "const" | "nullable"))
    }) {
        return Err(unsupported(&format!("JSON Schema keyword `{keyword}`"), pointer));
    }

    let mut annotations = Map::new();
    for (key, value) in object {
        if ANNOTATIONS.contains(&key.as_str()) || key.starts_with("x-") {
            annotations.insert(key.clone(), value.clone());
        }
    }
    if let (false, Some(first)) = (annotations.contains_key("example"), object.get("examples").and_then(Value::as_array).and_then(|e| e.first())) {
        annotations.insert("example".into(), first.clone());
    }

    let mut nullable = object.get("nullable") == Some(&Value::Bool(true));
    // Null admitted by an `anyOf`/`oneOf` member rather than by a type: only
    // then does a merged member's `enum` need `null` added.
    let mut null_alternative = false;
    // A sibling `type`, `enum`, or `const` that rejects null keeps rejecting
    // it whatever an `anyOf`/`oneOf` null member admits.
    let siblings_reject_null = rejects_null(object);
    let mut body = Map::new();
    for key in CONSTRAINTS {
        let Some(value) = object.get(*key) else { continue };
        let at = format!("{pointer}/{key}");
        let converted = match *key {
            "type" if !(value.is_string() || value.as_array().is_some_and(|kinds| kinds.iter().all(Value::is_string))) => {
                return Err(unsupported("a `type` that is not a string or an array of strings", &at));
            }
            "enum" if !value.is_array() => return Err(unsupported("an `enum` that is not an array", &at)),
            "type" => match value {
                Value::Array(kinds) => {
                    nullable |= kinds.iter().any(|kind| kind == "null");
                    Value::Array(kinds.iter().filter(|kind| *kind != "null").cloned().collect())
                }
                Value::String(kind) if kind == "null" => {
                    nullable = true;
                    Value::Array(vec![])
                }
                other => other.clone(),
            },
            "$ref" => match value.as_str() {
                Some(reference) => Value::String(reference.replace("#/$defs/", "#/components/schemas/").replace("#/definitions/", "#/components/schemas/")),
                None => return Err(unsupported("a `$ref` that is not a string", &at)),
            },
            "properties" => {
                let properties = value.as_object().ok_or_else(|| unsupported("`properties` that is not an object", &at))?;
                let mut converted = Map::new();
                for (name, schema) in properties {
                    converted.insert(name.clone(), convert(schema, &format!("{at}/{}", escape(name)))?);
                }
                Value::Object(converted)
            }
            "additionalProperties" | "not" => match value {
                Value::Bool(admits) if *key == "additionalProperties" => Value::Bool(*admits),
                schema => convert(schema, &at)?,
            },
            "items" => match value {
                Value::Array(_) => return Err(unsupported("tuple-form `items`", &at)),
                schema => convert(schema, &at)?,
            },
            "oneOf" | "anyOf" | "allOf" => {
                let members = value.as_array().ok_or_else(|| unsupported(&format!("`{key}` that is not an array"), &at))?;
                let mut converted = Vec::new();
                for (index, member) in members.iter().enumerate() {
                    if *key != "allOf" && admits_only_null(member) {
                        nullable |= !siblings_reject_null;
                        null_alternative |= !siblings_reject_null;
                    } else {
                        converted.push(convert(member, &format!("{at}/{index}"))?);
                    }
                }
                Value::Array(converted)
            }
            "exclusiveMinimum" | "exclusiveMaximum" if value.is_number() || exact_text(value).is_some() => {
                // 2020-12 numeric bound: OpenAPI 3.0 spells it as the bound plus
                // a boolean flag.
                let bound = if *key == "exclusiveMinimum" { "minimum" } else { "maximum" };
                if object.contains_key(bound) {
                    return Err(unsupported(&format!("numeric `{key}` alongside `{bound}`"), &at));
                }
                body.insert(bound.into(), value.clone());
                Value::Bool(true)
            }
            _ => value.clone(),
        };
        body.insert((*key).into(), converted);
    }

    if let Some(constant) = object.get("const") {
        if body.contains_key("enum") {
            return Err(unsupported("`const` alongside `enum`", &format!("{pointer}/const")));
        }
        body.insert("enum".into(), Value::Array(vec![constant.clone()]));
    }
    if let Some(Value::Array(values)) = body.get("enum") {
        nullable |= values.iter().any(Value::is_null);
        if values.iter().all(Value::is_null) && !values.is_empty() {
            // `{"nullable": true, "enum": [null]}` and `const: null` admit null
            // alone; the null branch spells that with a type.
            body.remove("enum");
            body.insert("type".into(), Value::Array(vec![]));
        } else if !body.contains_key("type") && let Some(kind) = enum_type(values) {
            body.insert("type".into(), Value::String(kind.into()));
        }
    }
    if !body.contains_key("type") && body.contains_key("properties") {
        body.insert("type".into(), Value::String("object".into()));
    }

    // A single-member `anyOf`/`oneOf` left after removing null members is
    // that member.
    for keyword in ["anyOf", "oneOf"] {
        if let Some(Value::Array(members)) = body.get(keyword) {
            match members.len() {
                0 if nullable => { body.remove(keyword); body.entry("type").or_insert(Value::Array(vec![])); }
                1 if object.get(keyword).and_then(Value::as_array).is_some_and(|source| source.len() > 1) => {
                    let member = members[0].clone();
                    body.remove(keyword);
                    body = merge_member(body, member, null_alternative);
                    if null_alternative
                        && !object.contains_key("enum")
                        && !object.contains_key("const")
                        && let Some(Value::Array(values)) = body.get_mut("enum")
                        && !values.iter().any(Value::is_null)
                    {
                        values.push(Value::Null);
                    }
                }
                _ => {}
            }
        }
    }

    let body = match body.remove("type") {
        Some(Value::Array(kinds)) => split_types(body, &kinds, nullable),
        Some(kind) => {
            body.insert("type".into(), kind);
            nullable_body(body, nullable)
        }
        None => nullable_body(body, nullable),
    };

    let mut converted = match body {
        // OpenAPI 3.0 ignores the siblings of `$ref`, so annotations need a
        // wrapper.
        mut body if body.contains_key("$ref") && (body.len() > 1 || !annotations.is_empty()) => {
            let reference = body.remove("$ref").expect("checked");
            let mut members = vec![json!({"$ref": reference})];
            if let Some(Value::Array(existing)) = body.remove("allOf") {
                members.extend(existing);
            }
            body.insert("allOf".into(), Value::Array(members));
            body
        }
        body => body,
    };
    converted.extend(annotations);
    Ok(Value::Object(converted))
}

/// Marks an exact-text stand-in for a numeric keyword while the schema is
/// converted; never written out.
const EXACT_MARK: &str = "\u{0}exact:";

/// `object` with each numeric keyword's value replaced by its exact text
/// (from [`crate::constraints::EXACT_EXTENSION`] or the number itself), so
/// the text follows the keyword wherever conversion moves it. The extension
/// itself is dropped; [`restore_exact_numbers`] rebuilds it.
fn carry_exact_numbers(object: &Map<String, Value>) -> Map<String, Value> {
    let mut carried = object.clone();
    carried.remove(crate::constraints::EXACT_EXTENSION);
    for keyword in crate::constraints::NUMERIC_KEYWORDS {
        if let Some(number) = crate::constraints::exact_number(object, keyword) {
            carried.insert(keyword.to_owned(), Value::String(format!("{EXACT_MARK}{number}")));
        }
    }
    carried
}

/// The exact text a [`carry_exact_numbers`] value holds.
fn exact_text(value: &Value) -> Option<&str> {
    value.as_str()?.strip_prefix(EXACT_MARK)
}

/// Turns every carried exact text in `value` back into a number, adding a
/// [`crate::constraints::EXACT_EXTENSION`] entry wherever `openapiv3`'s
/// `i64`/`f64` would not hold it exactly.
fn restore_exact_numbers(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for keyword in crate::constraints::NUMERIC_KEYWORDS {
                if let Some(text) = object.get(keyword).and_then(exact_text).map(str::to_owned) {
                    crate::constraints::set_number(object, keyword, &crate::constraints::ConstraintNumber::from(text));
                }
            }
            crate::constraints::mark_inexact_numbers(object);
            for (key, child) in object.iter_mut() {
                if !crate::constraints::is_data_keyword(key) {
                    restore_exact_numbers(child);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(restore_exact_numbers),
        _ => {}
    }
}

/// Folds the sole remaining member of an `anyOf`/`oneOf` into `body`: its own
/// constraints join the body's when they do not overlap, or it becomes an
/// `allOf` member when they do.
///
/// When the fold removed a null alternative (`null_alternative`), an `allOf`
/// member keeps admitting null beside its own values; otherwise its `type`
/// or `enum` would reject the null the source admits.
fn merge_member(mut body: Map<String, Value>, member: Value, null_alternative: bool) -> Map<String, Value> {
    let Value::Object(member) = member else { return body };
    let overlaps = member.keys().any(|key| body.contains_key(key) && CONSTRAINTS.contains(&key.as_str()));
    if overlaps {
        let member = if null_alternative { json!({"anyOf": [member, null_branch()]}) } else { Value::Object(member) };
        body.entry("allOf").or_insert(Value::Array(vec![])).as_array_mut().expect("allOf is an array").push(member);
    } else {
        for (key, value) in member {
            body.entry(key).or_insert(value);
        }
    }
    body
}

/// A body whose `type` was an array of non-null kinds: one kind is that type;
/// several become an `anyOf` of single-type members, each taking the
/// keywords that apply to it (`integer` overlaps `number`, so `oneOf` would
/// reject valid integers); none admits only null.
fn split_types(mut body: Map<String, Value>, kinds: &[Value], nullable: bool) -> Map<String, Value> {
    match kinds {
        [] => {
            let Value::Object(null) = null_branch() else { unreachable!() };
            null
        }
        [kind] => {
            body.insert("type".into(), kind.clone());
            nullable_body(body, nullable)
        }
        kinds => {
            let mut members = Vec::new();
            for kind in kinds {
                let name = kind.as_str().expect("`type` entries are checked to be strings");
                let mut member = Map::new();
                member.insert("type".into(), kind.clone());
                for (key, value) in &body {
                    if applies_to(name, key) {
                        member.insert(key.clone(), value.clone());
                    }
                }
                members.push(Value::Object(nullable_body(member, false)));
            }
            body.retain(|key, _| !kinds.iter().filter_map(Value::as_str).any(|kind| applies_to(kind, key)));
            if nullable {
                members.push(null_branch());
            }
            body.insert("anyOf".into(), Value::Array(members));
            body
        }
    }
}

/// `body` admitting null as well when `nullable`: a typed schema takes
/// `nullable`; an unconstrained one already admits null; anything else joins
/// the null branch in an `anyOf`.
///
/// An `enum` keeps the members it has. OpenAPI 3.0.3's `nullable` does not
/// add null to an `enum`, and neither does a nullable `type`, so an `enum`
/// without null rejects null in both dialects.
fn nullable_body(mut body: Map<String, Value>, nullable: bool) -> Map<String, Value> {
    if !nullable || body.is_empty() {
        return body;
    }
    if body.contains_key("type") {
        body.insert("nullable".into(), Value::Bool(true));
        return body;
    }
    if body.len() == 1 && let Some(Value::Array(members)) = body.get_mut("anyOf") {
        members.push(null_branch());
        return body;
    }
    let mut wrapper = Map::new();
    wrapper.insert("anyOf".into(), Value::Array(vec![Value::Object(body), null_branch()]));
    wrapper
}

/// Whether `member` admits only `null`: `{"type": "null"}` (2020-12),
/// `{"nullable": true, "enum": [null]}` (the schemars OpenAPI 3.0
/// transform), `const: null`, or the [`null_branch`] this module writes,
/// ignoring annotations.
fn admits_only_null(member: &Value) -> bool {
    let Some(member) = member.as_object() else { return false };
    let only_null = |value: &Value| value.as_array().is_some_and(|values| !values.is_empty() && values.iter().all(Value::is_null));
    let restricted = member.get("enum").is_some_and(only_null) || member.get("const") == Some(&Value::Null);
    let null_type = member.get("type").is_some_and(|kind| kind == "null");
    (restricted || null_type)
        && member.keys().all(|key| {
            ANNOTATIONS.contains(&key.as_str()) || IGNORED.contains(&key.as_str()) || matches!(key.as_str(), "type" | "nullable" | "enum" | "const")
        })
}

/// Whether `object`'s own `type`, `enum`, or `const` rejects null: a `type`
/// naming no `null` (unless `nullable`), or an `enum`/`const` without it.
fn rejects_null(object: &Map<String, Value>) -> bool {
    let typed = match object.get("type") {
        Some(Value::String(kind)) => kind != "null",
        Some(Value::Array(kinds)) => !kinds.iter().any(|kind| kind == "null"),
        _ => false,
    };
    (typed && object.get("nullable") != Some(&Value::Bool(true)))
        || object.get("enum").and_then(Value::as_array).is_some_and(|values| !values.iter().any(Value::is_null))
        || object.get("const").is_some_and(|constant| !constant.is_null())
}

/// The single JSON type of the non-null `values`, if they share one.
fn enum_type(values: &[Value]) -> Option<&'static str> {
    let mut kinds = values.iter().filter(|value| !value.is_null()).map(|value| match value {
        Value::String(_) => "string",
        Value::Bool(_) => "boolean",
        Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Null => unreachable!(),
    });
    let first = kinds.next()?;
    kinds.all(|kind| kind == first).then_some(first)
}

/// `name` as a JSON-pointer reference token.
fn escape(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

/// `name` as a JSON-pointer reference token, for callers that build the
/// pointer [`convert_schema_at`] reports errors at.
pub(super) fn pointer_token(name: &str) -> String {
    escape(name)
}

fn unsupported(feature: &str, pointer: &str) -> OpenApiError {
    OpenApiError::UnsupportedFeature { feature: format!("{feature} in an exported schema"), context: Some(pointer.into()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export(schema: Value) -> Value {
        serde_json::to_value(convert_component_schema("Fixture", &schema).unwrap()).unwrap()
    }

    fn null() -> Value {
        null_branch()
    }

    #[test]
    fn a_type_array_with_null_exports_typed_and_nullable() {
        assert_eq!(export(json!({"type": ["string", "null"]})), json!({"type": "string", "nullable": true}));
        assert_eq!(
            export(json!({"type": ["integer", "null"], "format": "uint32", "minimum": 0})),
            json!({"type": "integer", "nullable": true, "format": "uint32", "minimum": 0})
        );
        assert_eq!(
            export(json!({"type": ["array", "null"], "items": {"$ref": "#/components/schemas/Tag"}})),
            json!({"type": "array", "nullable": true, "items": {"$ref": "#/components/schemas/Tag"}})
        );
        assert_eq!(
            export(json!({"type": ["object", "null"], "additionalProperties": {"type": "string"}})),
            json!({"type": "object", "nullable": true, "additionalProperties": {"type": "string"}})
        );
    }

    #[test]
    fn several_non_null_types_export_as_any_of_with_their_own_keywords() {
        let only_null = null();
        // `integer` overlaps `number`, so `oneOf` would reject every integer.
        assert_eq!(
            export(json!({"type": ["integer", "number", "null"], "minimum": 1, "description": "amount"})),
            json!({"description": "amount", "anyOf": [
                {"type": "integer", "minimum": 1},
                {"type": "number", "minimum": 1.0},
                only_null,
            ]})
        );
        assert_eq!(
            export(json!({"type": ["string", "array"], "minLength": 2, "items": {"type": "string"}})),
            json!({"anyOf": [{"type": "string", "minLength": 2}, {"type": "array", "items": {"type": "string"}}]})
        );
    }

    #[test]
    fn a_nullable_reference_joins_the_null_branch_in_either_dialect() {
        let only_null = null();
        let expected = json!({"description": "owner", "anyOf": [{"$ref": "#/components/schemas/User"}, only_null]});
        // 2020-12, as schemars nests it.
        assert_eq!(export(json!({"description": "owner", "anyOf": [{"$ref": "#/components/schemas/User"}, {"type": "null"}]})), expected);
        // OpenAPI 3.0, as schemars transforms a registration's root.
        assert_eq!(
            export(json!({"description": "owner", "anyOf": [{"$ref": "#/components/schemas/User"}, {"nullable": true, "enum": [null]}]})),
            expected
        );
        // The exported form converts to itself.
        assert_eq!(export(expected.clone()), expected);
        // A nullable `$ref` sibling, which OpenAPI 3.0 would ignore.
        assert_eq!(export(json!({"$ref": "#/components/schemas/User", "nullable": true})), json!({"anyOf": [{"$ref": "#/components/schemas/User"}, only_null]}));
    }

    #[test]
    fn a_folded_nullable_member_keeps_its_constraints() {
        assert_eq!(
            export(json!({"anyOf": [{"type": "string", "maxLength": 3}, {"type": "null"}], "description": "code"})),
            json!({"type": "string", "nullable": true, "maxLength": 3, "description": "code"})
        );
    }

    #[test]
    fn enums_and_constants_keep_their_value_domain() {
        assert_eq!(export(json!({"type": "string", "enum": ["a", "b"]})), json!({"type": "string", "enum": ["a", "b"]}));
        assert_eq!(export(json!({"type": "integer", "enum": [1, 2]})), json!({"type": "integer", "enum": [1, 2]}));
        assert_eq!(export(json!({"enum": ["a", "b"]})), json!({"type": "string", "enum": ["a", "b"]}));
        assert_eq!(export(json!({"type": "string", "const": "text"})), json!({"type": "string", "enum": ["text"]}));
        assert_eq!(export(json!({"const": 3})), json!({"type": "integer", "enum": [3]}));
        // A nullable enum keeps `null` among its values, or `nullable` alone
        // would still reject it.
        assert_eq!(
            export(json!({"type": ["string", "null"], "enum": ["a", "b", null]})),
            json!({"type": "string", "nullable": true, "enum": ["a", "b", null]})
        );
        // A nullable type does not add null to an `enum` that omits it; a
        // null alternative beside an enum member does.
        assert_eq!(
            export(json!({"type": "string", "enum": ["a"], "nullable": true})),
            json!({"type": "string", "nullable": true, "enum": ["a"]})
        );
        assert_eq!(
            export(json!({"type": ["string", "null"], "enum": ["a"]})),
            json!({"type": "string", "nullable": true, "enum": ["a"]})
        );
        assert_eq!(
            export(json!({"anyOf": [{"type": "string", "enum": ["a"]}, {"type": "null"}]})),
            json!({"type": "string", "nullable": true, "enum": ["a", null]})
        );
        // A member folded into `allOf` beside an overlapping body keeps
        // admitting the null its alternative admitted.
        assert_eq!(
            export(json!({"maxLength": 5, "anyOf": [{"type": "string", "maxLength": 3, "enum": ["a"]}, {"type": "null"}]})),
            json!({"anyOf": [{"maxLength": 5, "allOf": [{"anyOf": [{"type": "string", "maxLength": 3, "enum": ["a"]}, null_branch()]}]}, null_branch()]})
        );
        // A sibling `type` or `enum` that rejects null still rejects it.
        assert_eq!(
            export(json!({"type": "string", "anyOf": [{"maxLength": 3}, {"type": "null"}]})),
            json!({"type": "string", "maxLength": 3})
        );
        assert_eq!(
            export(json!({"enum": ["a", "b"], "anyOf": [{"type": "string", "maxLength": 1}, {"type": "null"}]})),
            json!({"type": "string", "enum": ["a", "b"], "allOf": [{"type": "string", "maxLength": 1}]})
        );
        // Schemas that admit only null.
        assert_eq!(export(json!({"type": "null"})), null());
        assert_eq!(export(json!({"const": null})), null());
    }

    #[test]
    fn compositions_convert_their_members_and_keep_sibling_constraints() {
        // An internally tagged enum: tagged object members.
        let tagged = json!({"oneOf": [
            {"type": "object", "properties": {"kind": {"type": "string", "const": "circle"}, "radius": {"type": "number", "format": "double"}}, "required": ["kind", "radius"]},
            {"type": "object", "properties": {"kind": {"type": "string", "const": "square"}}, "required": ["kind"]},
        ]});
        assert_eq!(export(tagged), json!({"oneOf": [
            {"type": "object", "properties": {"kind": {"type": "string", "enum": ["circle"]}, "radius": {"type": "number", "format": "double"}}, "required": ["kind", "radius"]},
            {"type": "object", "properties": {"kind": {"type": "string", "enum": ["square"]}}, "required": ["kind"]},
        ]}));
        // A flattened struct beside a flattened enum: the object keeps its
        // properties and its `oneOf`.
        let flattened = json!({"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"],
            "oneOf": [{"$ref": "#/components/schemas/A"}, {"$ref": "#/components/schemas/B"}]});
        assert_eq!(export(flattened.clone()), flattened);
        // Reference members stay references; `allOf` is kept.
        let all_of = json!({"allOf": [{"$ref": "#/components/schemas/Base"}], "description": "x"});
        assert_eq!(export(all_of.clone()), all_of);
        // Constraints beside a `$ref` survive in an `allOf` wrapper.
        assert_eq!(
            export(json!({"$ref": "#/$defs/Base", "description": "d", "required": ["id"]})),
            json!({"allOf": [{"$ref": "#/components/schemas/Base"}], "required": ["id"], "description": "d"})
        );
    }

    #[test]
    fn additional_properties_survive_as_a_schema_or_a_boolean() {
        assert_eq!(
            export(json!({"type": "object", "additionalProperties": {"$ref": "#/components/schemas/Link"}})),
            json!({"type": "object", "additionalProperties": {"$ref": "#/components/schemas/Link"}})
        );
        assert_eq!(export(json!({"type": "object", "additionalProperties": true})), json!({"type": "object", "additionalProperties": true}));
        assert_eq!(export(json!({"type": "object", "additionalProperties": false})), json!({"type": "object", "additionalProperties": false}));
        // `serde_json::Value` values are free-form.
        assert_eq!(export(json!({"type": "object", "additionalProperties": true, "description": "m"})), json!({"type": "object", "additionalProperties": true, "description": "m"}));
    }

    #[test]
    fn bounds_formats_defaults_and_required_survive() {
        let source = json!({"type": "object", "required": ["count"], "properties": {
            "count": {"type": "integer", "format": "uint8", "minimum": 1, "maximum": 10, "default": 5},
            "code": {"type": "string", "minLength": 2, "maxLength": 8, "pattern": "^[a-z]+$"},
            "ratio": {"type": "number", "exclusiveMinimum": 0, "multipleOf": 0.5},
            "tags": {"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 3, "uniqueItems": true},
            "map": {"type": "object", "additionalProperties": {"type": "string"}, "minProperties": 1, "maxProperties": 4},
        }});
        assert_eq!(export(source), json!({"type": "object", "required": ["count"], "properties": {
            "count": {"type": "integer", "format": "uint8", "minimum": 1, "maximum": 10, "default": 5},
            "code": {"type": "string", "minLength": 2, "maxLength": 8, "pattern": "^[a-z]+$"},
            // 2020-12's numeric bound in OpenAPI 3.0's spelling.
            "ratio": {"type": "number", "minimum": 0.0, "exclusiveMinimum": true, "multipleOf": 0.5},
            "tags": {"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 3, "uniqueItems": true},
            "map": {"type": "object", "additionalProperties": {"type": "string"}, "minProperties": 1, "maxProperties": 4},
        }}));
    }

    #[test]
    fn free_form_and_boolean_schemas_export_unconstrained_or_impossible() {
        assert_eq!(export(json!(true)), json!({}));
        assert_eq!(export(json!({"description": "anything"})), json!({"description": "anything"}));
        assert_eq!(export(json!(false)), json!({"not": {}}));
        // Free-form already admits null.
        assert_eq!(export(json!({"anyOf": [{}, {"type": "null"}]})), json!({}));
    }

    #[test]
    fn an_unsupported_keyword_fails_with_its_schema_path() {
        for (schema, feature, context) in [
            (
                json!({"type": "object", "properties": {"a/b": {"type": "object", "patternProperties": {"^x": {}}}}}),
                "JSON Schema keyword `patternProperties` in an exported schema",
                "#/components/schemas/Fixture/properties/a~1b",
            ),
            (json!({"type": "array", "prefixItems": [{"type": "string"}]}), "JSON Schema keyword `prefixItems` in an exported schema", "#/components/schemas/Fixture"),
            (json!({"type": "array", "items": [{"type": "string"}]}), "tuple-form `items` in an exported schema", "#/components/schemas/Fixture/items"),
            (json!({"type": ["string", 5]}), "a `type` that is not a string or an array of strings in an exported schema", "#/components/schemas/Fixture/type"),
            (json!({"type": "string", "enum": "a"}), "an `enum` that is not an array in an exported schema", "#/components/schemas/Fixture/enum"),
            (json!(5), "a schema that is neither an object nor a boolean in an exported schema", "#/components/schemas/Fixture"),
            (
                json!({"anyOf": [{"type": "string"}, {"if": {"type": "string"}}]}),
                "JSON Schema keyword `if` in an exported schema",
                "#/components/schemas/Fixture/anyOf/1",
            ),
        ] {
            assert_eq!(
                convert_component_schema("Fixture", &schema).unwrap_err(),
                OpenApiError::UnsupportedFeature { feature: feature.into(), context: Some(context.into()) },
                "{schema}"
            );
        }
    }
}
