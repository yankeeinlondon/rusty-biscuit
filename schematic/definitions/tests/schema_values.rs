//! Values the exported OpenAPI schemas accept and reject (spec
//! `2026-10-18-schematic-fidelity-polish`, R1 to R4): a registered fixture
//! is exported, and its `components.schemas` validate instances with
//! `jsonschema` (Draft 2020-12, offline) after the standard OpenAPI 3.0
//! rewrite of `nullable: true` to a `type` array, which no draft reads.

use std::collections::HashMap;

use schematic_define::openapi::{ExportOptions, export};
use schematic_define::{DefinitionContext, Endpoint, RestApi, definition_type};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

definition_type! {
    /// A unit enum renamed by serde.
    #[derive(JsonSchema, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(dead_code)]
    enum Tier { FreeTier, ProTier }
}

definition_type! {
    /// A nested definition, so its optional fields keep schemars' 2020-12
    /// forms (`type` arrays, `{"type": "null"}` members).
    #[derive(JsonSchema, Serialize, Deserialize)]
    #[allow(dead_code)]
    struct Child { id: u32, nick: Option<String>, level: Option<Tier>, sizes: Option<Vec<u8>> }
}

definition_type! {
    /// An internally tagged enum.
    #[derive(JsonSchema, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "lowercase")]
    #[allow(dead_code)]
    enum Shape { Circle { radius: f64 }, Square { side: f64 } }
}

definition_type! {
    #[derive(JsonSchema, Serialize, Deserialize)]
    #[allow(dead_code)]
    struct Base { created: String }
}

/// `integer` overlaps `number`: an instance like `3` matches both.
fn integer_or_number(_: &mut SchemaGenerator) -> Schema {
    json_schema!({"type": ["integer", "number", "null"]})
}

definition_type! {
    /// The registration root, whose optional fields take schemars' OpenAPI
    /// 3.0 forms (`nullable`, `{"nullable": true, "enum": [null]}` members).
    #[derive(JsonSchema, Serialize, Deserialize)]
    #[allow(dead_code)]
    struct Fixture {
        name: Option<String>,
        tags: Option<Vec<Child>>,
        child: Option<Child>,
        tier: Option<Tier>,
        #[schemars(schema_with = "integer_or_number")]
        amount: Option<f64>,
        shape: Shape,
        #[serde(flatten)]
        base: Base,
        labels: HashMap<String, Child>,
        extra: HashMap<String, Value>,
        #[schemars(range(min = 1, max = 10))]
        count: u8,
        #[schemars(length(min = 2))]
        code: String,
        #[serde(default)]
        note: Option<String>,
    }
}

/// The exported `components.schemas` of an API returning `Fixture` and
/// `Vec<Child>`, and the whole document.
fn exported() -> (Value, Value) {
    let mut context = DefinitionContext::types_in("::schema_values");
    let endpoints = [
        Endpoint::get("GetFixture", "/fixture").describes("Get").returns_json::<Fixture>(&mut context).build().unwrap(),
        Endpoint::get("ListChildren", "/children").describes("List").returns_json::<Vec<Child>>(&mut context).build().unwrap(),
    ];
    let api = RestApi::builder(&context, "Fixture", "https://fixture.test").describes("Fixture").no_auth().endpoints(endpoints).build().unwrap();
    let document = serde_json::to_value(export(&api, &ExportOptions::new()).unwrap()).unwrap();
    (document["components"]["schemas"].clone(), document)
}

/// `schema` with each `{"type": T, "nullable": true}` as `{"type": [T, "null"]}`.
fn without_nullable(schema: &mut Value) {
    match schema {
        Value::Object(object) => {
            if object.remove("nullable") == Some(Value::Bool(true))
                && let Some(Value::String(kind)) = object.get("type").cloned()
            {
                object.insert("type".into(), json!([kind, "null"]));
            }
            object.values_mut().for_each(without_nullable);
        }
        Value::Array(values) => values.iter_mut().for_each(without_nullable),
        _ => {}
    }
}

/// A validator for the exported component `name`.
fn validator(components: &Value, name: &str) -> jsonschema::Validator {
    let mut root = json!({"components": {"schemas": components}, "$ref": format!("#/components/schemas/{name}")});
    without_nullable(&mut root);
    jsonschema::options().with_draft(jsonschema::Draft::Draft202012).offline().build(&root).expect("exported schemas compile")
}

/// A `Fixture` instance every exported constraint accepts.
fn valid() -> Value {
    json!({
        "name": "n", "tags": [{"id": 1}], "child": {"id": 2, "nick": "c", "level": "proTier", "sizes": [1]},
        "tier": "freeTier", "amount": 3, "shape": {"kind": "circle", "radius": 1.5}, "created": "2026-10-23",
        "labels": {"a": {"id": 1}}, "extra": {"x": [1, {"y": true}]}, "count": 5, "code": "ab",
    })
}

/// `valid()` with the value at `pointer` replaced, or removed for `None`.
fn edited(pointer: &str, value: Option<Value>) -> Value {
    let mut instance = valid();
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let parent = instance.pointer_mut(parent).unwrap().as_object_mut().unwrap();
    match value {
        Some(value) => parent.insert(key.into(), value),
        None => parent.remove(key),
    };
    instance
}

fn check(validator: &jsonschema::Validator, cases: &[(&str, Option<Value>)], accepted: bool) {
    for (pointer, value) in cases {
        let instance = edited(pointer, value.clone());
        assert_eq!(validator.is_valid(&instance), accepted, "{pointer} = {value:?} should be {}", if accepted { "accepted" } else { "rejected" });
    }
}

#[test]
fn optional_fields_accept_null_and_their_type_only() {
    let (components, _) = exported();
    let fixture = validator(&components, "Fixture");
    assert!(fixture.is_valid(&valid()));
    check(&fixture, &[
        // `Option<String>`, `Option<Vec<T>>`, `Option<Struct>`, a nullable
        // enum: at the root (OpenAPI 3.0 forms) and nested (2020-12 forms).
        ("/name", Some(Value::Null)), ("/tags", Some(Value::Null)), ("/child", Some(Value::Null)), ("/tier", Some(Value::Null)),
        ("/child/nick", Some(Value::Null)), ("/child/level", Some(Value::Null)), ("/child/sizes", Some(Value::Null)),
        ("/name", None), ("/note", Some(Value::Null)), ("/note", Some(json!("set"))),
    ], true);
    check(&fixture, &[
        ("/name", Some(json!(5))), ("/tags", Some(json!("x"))), ("/tags", Some(json!([{"id": "x"}]))),
        ("/child", Some(json!(5))), ("/child", Some(json!({"id": -1}))), ("/child", Some(json!({}))),
        ("/child/nick", Some(json!(1))), ("/child/sizes", Some(json!([256]))), ("/note", Some(json!(false))),
    ], false);
}

#[test]
fn enums_keep_their_renamed_value_domain() {
    let (components, _) = exported();
    assert_eq!(components["Tier"]["enum"], json!(["freeTier", "proTier"]));
    let fixture = validator(&components, "Fixture");
    check(&fixture, &[("/tier", Some(json!("proTier"))), ("/child/level", Some(json!("freeTier")))], true);
    check(&fixture, &[
        ("/tier", Some(json!("FreeTier"))), ("/tier", Some(json!("gold"))), ("/child/level", Some(json!("ProTier"))),
    ], false);
}

#[test]
fn an_integer_and_number_union_accepts_both() {
    let (components, _) = exported();
    let fixture = validator(&components, "Fixture");
    // `oneOf` would reject `3`, which is both an integer and a number.
    check(&fixture, &[("/amount", Some(json!(3))), ("/amount", Some(json!(2.5))), ("/amount", Some(Value::Null))], true);
    check(&fixture, &[("/amount", Some(json!("3"))), ("/amount", Some(json!([3])))], false);
}

#[test]
fn tagged_enums_flattened_structs_and_maps_keep_their_shape() {
    let (components, _) = exported();
    let fixture = validator(&components, "Fixture");
    check(&fixture, &[
        ("/shape", Some(json!({"kind": "square", "side": 2.0}))),
        ("/labels", Some(json!({}))), ("/extra", Some(json!({"z": null, "w": "anything"}))),
    ], true);
    check(&fixture, &[
        ("/shape", Some(json!({"kind": "triangle", "side": 2.0}))), ("/shape", Some(json!({"kind": "circle"}))),
        ("/shape", Some(json!({"radius": 1.0}))),
        // The flattened `Base.created` is still required.
        ("/created", None), ("/created", Some(json!(7))),
        ("/labels", Some(json!({"a": 5}))), ("/labels", Some(json!({"a": {"id": "x"}}))), ("/extra", Some(json!([]))),
    ], false);
}

#[test]
fn bounds_and_lengths_are_enforced() {
    let (components, _) = exported();
    let fixture = validator(&components, "Fixture");
    check(&fixture, &[("/count", Some(json!(1))), ("/count", Some(json!(10))), ("/code", Some(json!("abc")))], true);
    check(&fixture, &[
        ("/count", Some(json!(0))), ("/count", Some(json!(11))), ("/count", Some(json!(2.5))), ("/code", Some(json!("a"))),
        ("/count", None),
    ], false);
}

#[test]
fn a_vec_response_exports_inline_with_an_element_reference() {
    let (components, document) = exported();
    let schema = &document["paths"]["/children"]["get"]["responses"]["200"]["content"]["application/json"]["schema"];
    assert_eq!(schema, &json!({"type": "array", "items": {"$ref": "#/components/schemas/Child"}}));
    let names: Vec<&String> = components.as_object().unwrap().keys().collect();
    assert!(names.iter().all(|name| !name.contains('<') && !name.contains("_of_")), "no container component: {names:?}");
    assert!(names.iter().any(|name| *name == "Child"));
}
