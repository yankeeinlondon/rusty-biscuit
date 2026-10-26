use schematic_define::{AuthStrategy, SchemaComponent};

use super::*;
use crate::test_utils::make_api;

/// A source over the components of `schemas` (`{name: schema}`).
fn source(schemas: Value) -> ExampleSource {
    let mut api = make_api("Examples", "https://examples.test", AuthStrategy::None, vec![]);
    api.schemas = schemas
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, schema)| SchemaComponent { name: name.clone(), rust_path: Some(format!("crate::{name}")), schema: schema.clone() })
        .collect();
    ExampleSource::new(&api)
}

fn named(name: &str) -> TypeRef {
    TypeRef::Named(name.into())
}

#[test]
fn authored_values_win_in_priority_order_once_they_validate() {
    let source = source(json!({
        "Example": {"type": "string", "examples": ["first", "second"], "default": "fallback", "enum": ["first", "fallback"]},
        "Default": {"type": "integer", "default": 7, "minimum": 1},
        "Const": {"const": "fixed"},
        "Enum": {"type": "string", "enum": ["b", "a"]},
    }));
    assert_eq!(source.example(&named("Example")).unwrap(), json!("first"));
    assert_eq!(source.example(&named("Default")).unwrap(), json!(7));
    assert_eq!(source.example(&named("Const")).unwrap(), json!("fixed"));
    assert_eq!(source.example(&named("Enum")).unwrap(), json!("b"));
}

#[test]
fn an_invalid_authored_example_or_default_is_skipped_for_the_next_candidate() {
    let source = source(json!({
        // The example breaks maxLength and the default breaks the enum.
        "Code": {"type": "string", "maxLength": 3, "enum": ["abc", "xyz"], "examples": ["toolong"], "default": "nope"},
        // Every authored value is invalid: the typed placeholder is used.
        "Count": {"type": "integer", "minimum": 5, "examples": [1], "default": 2},
    }));
    assert_eq!(source.example(&named("Code")).unwrap(), json!("abc"));
    assert_eq!(source.example(&named("Count")).unwrap(), json!(5));
}

#[test]
fn objects_hold_required_properties_and_those_declaring_an_example_or_default() {
    let source = source(json!({
        "Widget": {
            "type": "object",
            "required": ["id", "created", "tags"],
            "properties": {
                "id": {"type": "string", "format": "uuid"},
                "created": {"type": "string", "format": "date-time"},
                "tags": {"type": "array", "items": {"type": "string", "minLength": 8}, "minItems": 2},
                "size": {"type": "integer", "default": 3},
                "note": {"type": "string"},
                "label": {"type": ["string", "null"], "examples": ["shown"]},
            },
        },
    }));
    assert_eq!(
        source.example(&named("Widget")).unwrap(),
        json!({
            "id": "00000000-0000-0000-0000-000000000000",
            "created": "2000-01-01T00:00:00Z",
            "tags": ["stringxx", "stringxx"],
            "size": 3,
            "label": "shown",
        })
    );
}

#[test]
fn numbers_move_into_their_bounds_and_onto_their_grid() {
    let source = source(json!({
        "Exclusive": {"type": "integer", "exclusiveMinimum": 10},
        "Negative": {"type": "integer", "maximum": -4},
        "Grid": {"type": "number", "minimum": 0.5, "multipleOf": 0.25},
        "Short": {"type": "string", "maxLength": 3},
    }));
    assert_eq!(source.example(&named("Exclusive")).unwrap(), json!(11));
    assert_eq!(source.example(&named("Negative")).unwrap(), json!(-4));
    assert_eq!(source.example(&named("Grid")).unwrap(), json!(0.5));
    assert_eq!(source.example(&named("Short")).unwrap(), json!("str"));
}

#[test]
fn all_of_members_combine_their_properties_and_constraints() {
    let source = source(json!({
        "Base": {"type": "object", "required": ["id"], "properties": {"id": {"type": "integer", "minimum": 3}}},
        "Named": {"type": "object", "required": ["name"], "properties": {"name": {"type": "string", "minLength": 7}}},
        "Both": {"allOf": [{"$ref": "#/components/schemas/Base"}, {"$ref": "#/components/schemas/Named"}]},
        // Two members whose constraints cannot both hold.
        "Clash": {"allOf": [{"type": "string"}, {"type": "integer"}]},
    }));
    assert_eq!(source.example(&named("Both")).unwrap(), json!({"id": 3, "name": "stringx"}));
    let clash = source.example(&named("Clash")).unwrap_err();
    assert_eq!(clash.schema_path, "#/components/schemas/Clash");
    assert!(clash.reason.contains("fails"), "{clash}");
}

#[test]
fn one_of_takes_the_first_branch_that_matches_exactly_one_branch() {
    let source = source(json!({
        // Branch 0 builds `"string"`, which branch 1 also accepts, so a oneOf
        // rejects it; branch 1's `"string"` is rejected for the same reason,
        // and branch 2's integer is the first exclusive value.
        "Exclusive": {"oneOf": [{"type": "string"}, {"type": "string", "maxLength": 9}, {"type": "integer"}]},
        // anyOf accepts the overlap, so its first branch wins.
        "Overlap": {"anyOf": [{"type": "string"}, {"type": "string", "maxLength": 9}, {"type": "integer"}]},
        "Unsatisfiable": {"oneOf": [{"type": "string"}, {"type": "string"}]},
    }));
    assert_eq!(source.example(&named("Exclusive")).unwrap(), json!(0));
    assert_eq!(source.example(&named("Overlap")).unwrap(), json!("string"));
    let error = source.example(&named("Unsatisfiable")).unwrap_err();
    assert_eq!(error.schema_path, "#/components/schemas/Unsatisfiable");
    assert!(error.reason.contains("no `oneOf` branch yields a valid value"), "{error}");
}

#[test]
fn recursion_stops_at_three_levels_through_optional_or_nullable_members() {
    let source = source(json!({
        // Optional recursion: left out unless it declares an example.
        "Tree": {"type": "object", "required": ["name"], "properties": {"name": {"type": "string"}, "children": {"type": "array", "items": {"$ref": "#/components/schemas/Tree"}}}},
        // Required but nullable recursion: null at the fourth level.
        "Chain": {"type": "object", "required": ["next"], "properties": {"next": {"anyOf": [{"$ref": "#/components/schemas/Chain"}, {"type": "null"}]}}},
        // Required, non-nullable recursion has no finite value.
        "Loop": {"type": "object", "required": ["next"], "properties": {"next": {"$ref": "#/components/schemas/Loop"}}},
    }));
    assert_eq!(source.example(&named("Tree")).unwrap(), json!({"name": "string"}));
    assert_eq!(source.example(&named("Chain")).unwrap(), json!({"next": {"next": {"next": null}}}));
    let error = source.example(&named("Loop")).unwrap_err();
    assert_eq!(error.schema_path, "#/components/schemas/Loop/properties/next");
    assert!(error.reason.contains("more than 3 levels deep"), "{error}");
}

#[test]
fn a_pattern_the_placeholder_misses_is_a_diagnostic_never_an_invalid_value() {
    let source = source(json!({
        "Sha": {"type": "object", "required": ["sha"], "properties": {"sha": {"type": "string", "pattern": "^[0-9a-f]{40}$"}}},
    }));
    let error = source.example(&named("Sha")).unwrap_err();
    assert_eq!(error.schema_path, "#");
    assert!(error.reason.contains("/sha"), "{error}");
}

#[test]
fn openapi_nullable_and_wrapper_references_are_honored() {
    let source = source(json!({
        "Maybe": {"type": "string", "nullable": true, "enum": [null, "x"]},
        "Item": {"type": "object", "required": ["n"], "properties": {"n": {"type": "integer"}}},
        "Odd name<T>": {"type": "boolean"},
    }));
    // `null` is the first enum value and valid only because of `nullable`.
    assert_eq!(source.example(&named("Maybe")).unwrap(), Value::Null);
    let list = TypeRef::Array(Box::new(named("Item")));
    assert_eq!(source.example(&list).unwrap(), json!([]));
    assert_eq!(source.example(&TypeRef::Optional(Box::new(named("Item")))).unwrap(), json!({"n": 0}));
    assert_eq!(source.example(&named("Odd name<T>")).unwrap(), json!(false));
    assert!(source.check(&named("Item"), &json!({"n": "text"})).is_err());
    assert!(source.check(&named("Item"), &json!({"n": 1})).is_ok());
}

#[test]
fn a_schema_the_validator_cannot_compile_is_a_diagnostic() {
    // OpenAPI 3.0's boolean `exclusiveMinimum` is not JSON Schema 2020-12.
    let source = source(json!({"Old": {"type": "integer", "minimum": 10, "exclusiveMinimum": true}}));
    let error = source.example(&named("Old")).unwrap_err();
    assert!(error.reason.contains("does not compile"), "{error}");
}

#[test]
fn an_unresolved_reference_names_its_schema_path() {
    let source = source(json!({"Holder": {"$ref": "#/components/schemas/Missing"}}));
    let error = source.example(&named("Holder")).unwrap_err();
    assert!(error.reason.contains("Missing"), "{error}");
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let schemas = json!({
        "Shape": {"oneOf": [
            {"type": "object", "required": ["kind", "radius"], "properties": {"kind": {"const": "circle"}, "radius": {"type": "number", "minimum": 1}}},
            {"type": "object", "required": ["kind", "side"], "properties": {"kind": {"const": "square"}, "side": {"type": "number"}}},
        ]},
    });
    let first = serde_json::to_string(&source(schemas.clone()).example(&named("Shape")).unwrap()).unwrap();
    let second = serde_json::to_string(&source(schemas).example(&named("Shape")).unwrap()).unwrap();
    assert_eq!(first, r#"{"kind":"circle","radius":1}"#);
    assert_eq!(first, second);
}
