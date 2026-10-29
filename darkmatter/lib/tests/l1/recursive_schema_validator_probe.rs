//! The pinned validator's answers on recursive local references.
//!
//! Recursive named types lower to a root `$defs` table used through local
//! `#/$defs/<key>` references. These cases pin what the repository's
//! `jsonschema` (Draft 2020-12) does with such a schema, independently of
//! Darkmatter's lowering: a self-referencing and a mutually recursive
//! definition validate a depth-8 instance and reject a bad value at depth 8
//! with that node's instance path, a constraint beside a `$ref` applies
//! together with the target, and an escaped JSON Pointer key resolves.

use serde_json::{Value, json};

const DEPTH: usize = 8;

fn compile(schema: &Value) -> jsonschema::Validator {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(schema)
        .expect("the recursive schema compiles")
}

/// The instance paths of every error, sorted.
fn error_paths(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    let mut paths: Vec<String> = validator
        .iter_errors(instance)
        .map(|error| error.instance_path().as_str().to_string())
        .collect();
    paths.sort();
    paths
}

fn item_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "root": { "$ref": "#/$defs/item" } },
        "required": ["root"],
        "$defs": {
            "item": {
                "type": "object",
                "properties": {
                    "value": { "type": "string" },
                    "children": { "type": "array", "items": { "$ref": "#/$defs/item" } }
                },
                "required": ["value", "children"],
                "additionalProperties": false
            }
        }
    })
}

/// An item chain `depth` levels deep whose deepest item carries `leaf_value`.
fn item_chain(depth: usize, leaf_value: Value) -> Value {
    let mut item = json!({ "value": leaf_value, "children": [] });
    for level in (1..depth).rev() {
        item = json!({ "value": format!("level {level}"), "children": [item] });
    }
    item
}

/// The instance path of the item at `depth` below `/root`.
fn item_path(depth: usize) -> String {
    let mut path = String::from("/root");
    for _ in 1..depth {
        path.push_str("/children/0");
    }
    path
}

#[test]
fn a_self_referencing_definition_validates_depth_eight() {
    let validator = compile(&item_schema());

    let valid = json!({ "root": item_chain(DEPTH, json!("leaf")) });
    assert!(validator.is_valid(&valid), "{:?}", error_paths(&validator, &valid));

    let invalid = json!({ "root": item_chain(DEPTH, json!(42)) });
    assert_eq!(
        error_paths(&validator, &invalid),
        [format!("{}/value", item_path(DEPTH))]
    );
}

#[test]
fn a_required_child_missing_at_depth_eight_is_reported_there() {
    let validator = compile(&item_schema());
    let mut instance = json!({ "root": item_chain(DEPTH, json!("leaf")) });
    let leaf = instance
        .pointer_mut(&item_path(DEPTH))
        .and_then(Value::as_object_mut)
        .expect("the leaf item");
    leaf.remove("children");

    let errors: Vec<_> = validator.iter_errors(&instance).collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].instance_path().as_str(), item_path(DEPTH));
    assert!(errors[0].to_string().contains("children"), "{}", errors[0]);
}

#[test]
fn mutually_recursive_definitions_validate_depth_eight() {
    let schema = json!({
        "type": "object",
        "properties": { "start": { "$ref": "#/$defs/even" } },
        "$defs": {
            "even": {
                "type": "object",
                "properties": {
                    "n": { "type": "integer" },
                    "next": { "$ref": "#/$defs/odd" }
                },
                "required": ["n"]
            },
            "odd": {
                "type": "object",
                "properties": {
                    "n": { "type": "integer" },
                    "next": { "$ref": "#/$defs/even" }
                },
                "required": ["n"]
            }
        }
    });
    let validator = compile(&schema);
    let chain = |leaf: Value| {
        let mut node = json!({ "n": leaf });
        for level in (1..DEPTH).rev() {
            node = json!({ "n": level, "next": node });
        }
        json!({ "start": node })
    };
    let deepest = format!("/start{}/n", "/next".repeat(DEPTH - 1));

    let valid = chain(json!(DEPTH));
    assert!(validator.is_valid(&valid), "{:?}", error_paths(&validator, &valid));
    assert_eq!(error_paths(&validator, &chain(json!("eight"))), [deepest]);
}

#[test]
fn a_constraint_beside_a_reference_applies_with_its_target() {
    let schema = json!({
        "type": "object",
        "properties": {
            "limited": {
                "$ref": "#/$defs/item",
                "properties": { "value": { "maxLength": 3 } }
            }
        },
        "$defs": {
            "item": {
                "type": "object",
                "properties": {
                    "value": { "type": "string" },
                    "children": { "type": "array", "items": { "$ref": "#/$defs/item" } }
                },
                "required": ["value", "children"]
            }
        }
    });
    let validator = compile(&schema);

    let fits = json!({ "limited": { "value": "abc", "children": [] } });
    assert!(validator.is_valid(&fits), "{:?}", error_paths(&validator, &fits));
    // The sibling rejects a value the target alone accepts...
    let too_long = json!({ "limited": { "value": "abcd", "children": [] } });
    assert_eq!(error_paths(&validator, &too_long), ["/limited/value"]);
    // ...and the target still rejects what the sibling alone accepts.
    let wrong_type = json!({ "limited": { "value": "ok", "children": [{ "value": 1, "children": [] }] } });
    assert_eq!(error_paths(&validator, &wrong_type), ["/limited/children/0/value"]);
}

#[test]
fn an_escaped_pointer_key_resolves() {
    // `~` and `/` in a definition key are written `~0` and `~1` in the pointer.
    let schema = json!({
        "type": "object",
        "properties": { "root": { "$ref": "#/$defs/a~1b~0c" } },
        "$defs": {
            "a/b~c": {
                "type": "object",
                "properties": { "next": { "$ref": "#/$defs/a~1b~0c" }, "n": { "type": "integer" } }
            }
        }
    });
    let validator = compile(&schema);
    let valid = json!({ "root": { "n": 1, "next": { "n": 2, "next": { "n": 3 } } } });
    assert!(validator.is_valid(&valid), "{:?}", error_paths(&validator, &valid));
    let invalid = json!({ "root": { "n": 1, "next": { "n": 2, "next": { "n": "3" } } } });
    assert_eq!(error_paths(&validator, &invalid), ["/root/next/next/n"]);
}

#[test]
fn a_reference_to_a_missing_definition_does_not_compile() {
    let schema = json!({
        "type": "object",
        "properties": { "root": { "$ref": "#/$defs/absent" } },
        "$defs": {}
    });
    assert!(
        jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&schema)
            .is_err(),
        "an unresolvable local reference must fail at compile time, not validate as unconstrained"
    );
}
