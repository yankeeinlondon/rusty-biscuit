//! `phase::merge_with_effective` over an effective schema whose root is a
//! union (a registered host's union layer) and an object document.

use serde_json::json;
use simplified_schema::phase::merge_with_effective;

#[test]
fn an_object_document_projects_into_every_arm_of_an_effective_root_union() {
    let arm = |key: &str, other: &str| {
        json!({
            "type": "object",
            "properties": {
                key: {"type": "string"},
                other: {"type": "null"},
                "model": {"type": "string"},
                "topic": {"$ref": "#/$defs/topic"},
            },
            "required": [key, "topic"],
        })
    };
    let effective = json!({
        "anyOf": [arm("prompt", "sections"), arm("sections", "prompt")],
        "$defs": {"topic": {"type": "string", "minLength": 1}, "kept": {"type": "number"}},
    });
    // The launch projection of `topic: string(required)`: not eager, so not
    // listed as required.
    let projected = json!({
        "type": "object",
        "properties": {"topic": {"$ref": "#/$defs/topic"}},
        "$defs": {"topic": {"type": "string"}},
    });

    let merged = merge_with_effective(&effective, projected).unwrap();

    let arms = merged["anyOf"].as_array().expect("the root union is kept");
    assert_eq!(arms.len(), 2);
    for (arm, key) in arms.iter().zip(["prompt", "sections"]) {
        assert_eq!(arm["properties"]["model"], json!({"type": "string"}), "the layer's rules stay in {key}'s arm");
        assert_eq!(arm["required"], json!([key]), "the document's projected presence wins in {key}'s arm");
        assert!(arm.get("$defs").is_none(), "definitions live at the root");
    }
    assert_eq!(merged["$defs"]["topic"], json!({"type": "string"}), "the projected body replaces its counterpart");
    assert_eq!(merged["$defs"]["kept"], json!({"type": "number"}), "a layer-only definition is kept");
}
