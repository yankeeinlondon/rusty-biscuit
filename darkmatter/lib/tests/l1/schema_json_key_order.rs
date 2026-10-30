//! Lowered JSON Schema keeps the key order it was built in.
//!
//! Maps keep insertion order, and `serde_json::Map::remove` is a swap
//! remove that moves the last key into the vacated slot. Pulling an
//! annotation or `$schema` off a fragment must not reorder what remains.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::{parse_yaml_schema, resolve::resolve_yaml_schema, to_json_schema};
use serde_json::Value;

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .expect("schema object")
        .keys()
        .map(String::as_str)
        .collect()
}

/// A root-union arm loses its `$schema` when nested under `anyOf`; the rest
/// of the arm keeps the order a single shape lowers to.
#[test]
fn root_union_arms_keep_their_lowered_key_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        "- title: \"string(required)\"\n  body: string\n- name: \"string(required)\"\n",
    )
    .expect("yaml");
    let resolved = resolve_yaml_schema(&yaml, dir.path(), &FileResolutionContext::new(dir.path()))
        .expect("the root union resolves");

    assert_eq!(keys(&resolved.json_schema), ["$schema", "anyOf"]);
    let arms = resolved.json_schema["anyOf"].as_array().expect("anyOf arms");
    assert_eq!(arms.len(), 2);
    for arm in arms {
        assert_eq!(keys(arm), ["type", "additionalProperties", "properties", "required"]);
    }
    assert_eq!(keys(&arms[0]["properties"]), ["title", "body"]);
    assert_eq!(arms[0]["required"], serde_json::json!(["title"]));
    assert_eq!(arms[1]["required"], serde_json::json!(["name"]));
}

/// A property union hoists its arms' `default` onto the wrapper; the arm
/// keeps its remaining annotations in order, and the wrapper leads with
/// `anyOf`, as an optional single atom does.
#[test]
fn property_union_arm_and_wrapper_keep_their_built_key_order() {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        "flag:\n  - \"string(default(a); generated; example(x)) -> A label\"\n  - number\n",
    )
    .expect("yaml");
    let schema = to_json_schema(&parse_yaml_schema(&yaml).expect("schema parses")).expect("converts");
    let wrapper = &schema["properties"]["flag"];

    assert_eq!(keys(wrapper), ["anyOf", "default", "x-darkmatter-generated"]);
    assert_eq!(wrapper["default"], "a");
    let arms = wrapper["anyOf"].as_array().expect("anyOf arms");
    assert_eq!(arms[0], serde_json::json!({ "type": "null" }));
    assert_eq!(
        keys(&arms[1]),
        ["type", "description", "x-darkmatter-generated", "x-darkmatter-example"]
    );
    assert_eq!(arms[2], serde_json::json!({ "type": "number" }));
}
