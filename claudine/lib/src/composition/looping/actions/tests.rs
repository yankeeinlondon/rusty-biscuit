use super::*;
use serde_json::json;

fn object(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

/// Minimal lookup that resolves a flat string→Value map. Used to drive the
/// template-rendering tests without spinning up a full LoopExpressionLookup.
struct MapLookup(Map<String, Value>);

impl EvaluationLookup for MapLookup {
    fn get(&self, path: &str) -> Option<Value> {
        self.0.get(path).cloned()
    }
}

fn ctx() -> ActionContext {
    ActionContext::default()
}

// ── template rendering ────────────────────────────────────────────

#[test]
fn render_single_template_preserves_number_type() {
    let lookup = MapLookup(object(json!({"count": 3})));
    let value = Value::String("{{count}}".into());
    let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
    assert_eq!(rendered, json!(3));
}

#[test]
fn render_single_template_preserves_bool_type() {
    let lookup = MapLookup(object(json!({"flag": true})));
    let value = Value::String("{{flag}}".into());
    let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
    assert_eq!(rendered, json!(true));
}

#[test]
fn render_single_template_preserves_string_type() {
    let lookup = MapLookup(object(json!({"name": "alice"})));
    let value = Value::String("{{name}}".into());
    let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
    assert_eq!(rendered, json!("alice"));
}

#[test]
fn render_template_inside_string_concatenates_to_string() {
    let lookup = MapLookup(object(json!({"count": 3})));
    let value = Value::String("iter-{{count}}".into());
    let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
    assert_eq!(rendered, json!("iter-3"));
}

#[test]
fn render_multiple_templates_join_with_text_to_string() {
    let lookup = MapLookup(object(json!({"a": 1, "b": 2})));
    let value = Value::String("{{a}}+{{b}}".into());
    let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
    assert_eq!(rendered, json!("1+2"));
}

#[test]
fn render_with_no_lookup_returns_input_unchanged() {
    let value = Value::String("{{count}}".into());
    let rendered = render_action_value(&value, None, ctx()).unwrap();
    assert_eq!(rendered, json!("{{count}}"));
}

#[test]
fn render_string_without_templates_passes_through() {
    let lookup = MapLookup(object(json!({})));
    let value = Value::String("plain literal".into());
    let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
    assert_eq!(rendered, json!("plain literal"));
}

#[test]
fn render_object_walks_into_string_leaves() {
    let lookup = MapLookup(object(json!({"count": 7, "stage": "review"})));
    let value = json!({
        "phase": "{{stage}}",
        "remaining": "{{count}}",
        "literal": 42
    });
    let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
    assert_eq!(
        rendered,
        json!({
            "phase": "review",
            "remaining": 7,
            "literal": 42
        })
    );
}

#[test]
fn render_array_walks_into_string_leaves() {
    let lookup = MapLookup(object(json!({"x": 1, "y": 2})));
    let value = json!(["{{x}}", "{{y}}", "static"]);
    let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
    assert_eq!(rendered, json!([1, 2, "static"]));
}

#[test]
fn render_invalid_template_returns_invalid_action_error() {
    let lookup = MapLookup(object(json!({})));
    let value = Value::String("{{ a &&& b }}".into());
    let err = render_action_value(&value, Some(&lookup), ctx()).unwrap_err();
    assert!(
        matches!(err, CompositionError::InvalidAction { ref message, .. } if message.contains("invalid template")),
        "got {err}"
    );
}

// ── set/append integration with rendering ─────────────────────────

#[test]
fn set_with_lookup_stores_typed_number() {
    let lookup = MapLookup(object(json!({"count": 4})));
    let mut fm = Map::new();
    let mut stage = ActionStaging::new(&fm.clone(), 1, 1);
    stage
        .apply_action(
            &LoopAction::Set {
                prop: "stamp".into(),
                value: Value::String("{{count}}".into()),
            },
            1,
            Some(&lookup),
        )
        .unwrap();
    fm = stage.commit_map();
    assert_eq!(fm.get("stamp"), Some(&json!(4)));
}

#[test]
fn set_without_lookup_stores_raw_template_string() {
    let mut fm = Map::new();
    let mut stage = ActionStaging::new(&fm.clone(), 1, 1);
    stage
        .apply_action(
            &LoopAction::Set {
                prop: "stamp".into(),
                value: Value::String("{{count}}".into()),
            },
            1,
            None,
        )
        .unwrap();
    fm = stage.commit_map();
    assert_eq!(fm.get("stamp"), Some(&json!("{{count}}")));
}

#[test]
fn append_with_lookup_renders_template_into_string() {
    let lookup = MapLookup(object(json!({"count": 2, "out": "ran-2"})));
    let mut fm = object(json!({"log": ""}));
    let mut stage = ActionStaging::new(&fm.clone(), 2, 1);
    stage
        .apply_action(
            &LoopAction::Append {
                prop: "log".into(),
                value: Value::String("iter {{count}}: {{out}}\n".into()),
            },
            1,
            Some(&lookup),
        )
        .unwrap();
    fm = stage.commit_map();
    assert_eq!(fm.get("log"), Some(&json!("iter 2: ran-2\n")));
}

#[test]
fn merge_with_lookup_renders_inside_object_value() {
    let lookup = MapLookup(object(json!({"count": 5})));
    let mut fm = object(json!({"state": {"phase": "draft"}}));
    let mut stage = ActionStaging::new(&fm.clone(), 5, 1);
    stage
        .apply_action(
            &LoopAction::Merge {
                prop: "state".into(),
                value: json!({"iteration": "{{count}}", "phase": "review"}),
            },
            1,
            Some(&lookup),
        )
        .unwrap();
    fm = stage.commit_map();
    assert_eq!(
        fm.get("state"),
        Some(&json!({"phase": "review", "iteration": 5}))
    );
}

#[test]
fn increment_sets_missing_and_null_to_one() {
    let mut fm = Map::new();
    apply_increment(&mut fm, "counter").unwrap();
    assert_eq!(fm.get("counter"), Some(&json!(1)));

    fm.insert("counter".into(), Value::Null);
    apply_increment(&mut fm, "counter").unwrap();
    assert_eq!(fm.get("counter"), Some(&json!(1)));
}

#[test]
fn increment_accepts_numbers_and_numeric_strings() {
    let mut fm = object(json!({"int": 5, "str": "5", "float": 1.5}));
    apply_increment(&mut fm, "int").unwrap();
    apply_increment(&mut fm, "str").unwrap();
    apply_increment(&mut fm, "float").unwrap();

    assert_eq!(fm.get("int"), Some(&json!(6)));
    assert_eq!(fm.get("str"), Some(&json!(6)));
    assert_eq!(fm.get("float"), Some(&json!(2.5)));
}

#[test]
fn increment_rejects_non_numeric_strings() {
    let mut fm = object(json!({"counter": "abc"}));
    let err = apply_increment(&mut fm, "counter").unwrap_err();
    assert!(matches!(
        err,
        CompositionError::InvalidIncrementType {
            iteration: 1,
            action_index: 1,
            total_actions: 1,
            property,
            found,
            ..
        } if property == "counter" && found == "string"
    ));
}

#[test]
fn decrement_sets_missing_and_accepts_numeric_strings() {
    let mut fm = object(json!({"counter": "5"}));
    apply_decrement(&mut fm, "missing").unwrap();
    apply_decrement(&mut fm, "counter").unwrap();

    assert_eq!(fm.get("missing"), Some(&json!(-1)));
    assert_eq!(fm.get("counter"), Some(&json!(4)));
}

#[test]
fn set_rejects_reserved_properties() {
    for prop in ["loop", "replace", "_loop_count", "_loop_is_first"] {
        let mut fm = Map::new();
        let err = apply_action(
            &mut fm,
            &LoopAction::Set {
                prop: prop.into(),
                value: json!(true),
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, CompositionError::InvalidAction { ref message, .. } if message.contains("reserved")),
            "got {err}"
        );
    }
}

#[test]
fn set_assigns_new_value() {
    let mut fm = object(json!({"stage": "draft"}));
    apply_action(
        &mut fm,
        &LoopAction::Set {
            prop: "stage".into(),
            value: json!("review"),
        },
    )
    .unwrap();
    assert_eq!(fm.get("stage"), Some(&json!("review")));
}

#[test]
fn append_handles_scalars_and_json_values() {
    let mut fm = object(json!({"log": "start"}));
    apply_action(
        &mut fm,
        &LoopAction::Append {
            prop: "log".into(),
            value: json!(true),
        },
    )
    .unwrap();
    apply_action(
        &mut fm,
        &LoopAction::Append {
            prop: "log".into(),
            value: json!({"event": "tick"}),
        },
    )
    .unwrap();
    apply_action(
        &mut fm,
        &LoopAction::Append {
            prop: "log".into(),
            value: json!([1, 2]),
        },
    )
    .unwrap();

    assert_eq!(
        fm.get("log"),
        Some(&json!("starttrue\n{\"event\":\"tick\"}\n[1,2]"))
    );
}

#[test]
fn append_empty_preserves_jsonl_shape() {
    let mut fm = object(json!({"objects": "{\"a\":1}", "arrays": "[1]"}));
    apply_action(
        &mut fm,
        &LoopAction::Append {
            prop: "objects".into(),
            value: Value::Null,
        },
    )
    .unwrap();
    apply_action(
        &mut fm,
        &LoopAction::Append {
            prop: "arrays".into(),
            value: json!(""),
        },
    )
    .unwrap();

    assert_eq!(fm.get("objects"), Some(&json!("{\"a\":1}{}")));
    assert_eq!(fm.get("arrays"), Some(&json!("[1][]")));
}

#[test]
fn prepend_is_append_in_reverse_for_json_values() {
    let mut fm = object(json!({"log": "tail"}));
    apply_action(
        &mut fm,
        &LoopAction::Prepend {
            prop: "log".into(),
            value: json!({"event": "tick"}),
        },
    )
    .unwrap();

    assert_eq!(fm.get("log"), Some(&json!("{\"event\":\"tick\"}\ntail")));
}

#[test]
fn merge_shallow_merges_and_replaces_arrays() {
    let mut fm = object(json!({"state": {"a": 1, "items": [1], "keep": true}}));
    apply_action(
        &mut fm,
        &LoopAction::Merge {
            prop: "state".into(),
            value: json!({"b": 2, "items": [2]}),
        },
    )
    .unwrap();

    assert_eq!(
        fm.get("state"),
        Some(&json!({"a": 1, "b": 2, "items": [2], "keep": true}))
    );
}

#[test]
fn merge_rejects_non_object_target_or_value() {
    let mut fm = object(json!({"state": "nope"}));
    let err = apply_action(
        &mut fm,
        &LoopAction::Merge {
            prop: "state".into(),
            value: json!({"b": 2}),
        },
    )
    .unwrap_err();
    assert!(matches!(err, CompositionError::InvalidAction { .. }));

    let mut fm = Map::new();
    let err = apply_action(
        &mut fm,
        &LoopAction::Merge {
            prop: "state".into(),
            value: json!("nope"),
        },
    )
    .unwrap_err();
    assert!(matches!(err, CompositionError::InvalidAction { .. }));
}

#[test]
fn staging_discards_partial_mutations_on_error() {
    let fm = object(json!({"counter": 1, "bad": "abc"}));
    let original = fm.clone();
    let actions = [
        LoopAction::Increment("counter".into()),
        LoopAction::Increment("bad".into()),
        LoopAction::Set {
            prop: "stage".into(),
            value: json!("done"),
        },
    ];

    let mut stage = ActionStaging::new(&fm, 7, actions.len());
    stage.apply_action(&actions[0], 1, None).unwrap();
    let err = stage.apply_action(&actions[1], 2, None).unwrap_err();

    assert!(matches!(
        err,
        CompositionError::InvalidIncrementType {
            iteration: 7,
            action_index: 2,
            total_actions: 3,
            ..
        }
    ));
    assert_eq!(fm, original);
}

#[test]
fn staging_commits_after_all_actions_succeed() {
    let fm = object(json!({"counter": 1}));
    let actions = [
        LoopAction::Increment("counter".into()),
        LoopAction::Set {
            prop: "stage".into(),
            value: json!("done"),
        },
    ];

    let mut stage = ActionStaging::new(&fm, 1, actions.len());
    for (index, action) in actions.iter().enumerate() {
        stage.apply_action(action, index + 1, None).unwrap();
    }

    assert_eq!(stage.commit(), json!({"counter": 2, "stage": "done"}));
    assert_eq!(fm, object(json!({"counter": 1})));
}

#[test]
fn increment_rejects_boolean() {
    let mut fm = object(json!({"flag": true}));
    let err = apply_increment(&mut fm, "flag").unwrap_err();
    assert!(matches!(
        err,
        CompositionError::InvalidIncrementType {
            iteration: 1,
            action_index: 1,
            total_actions: 1,
            property,
            found,
            ..
        } if property == "flag" && found == "boolean"
    ));
}

#[test]
fn decrement_rejects_boolean() {
    let mut fm = object(json!({"flag": true}));
    let err = apply_decrement(&mut fm, "flag").unwrap_err();
    assert!(matches!(
        err,
        CompositionError::InvalidDecrementType {
            iteration: 1,
            action_index: 1,
            total_actions: 1,
            property,
            found,
            ..
        } if property == "flag" && found == "boolean"
    ));
}

#[test]
fn decrement_rejects_non_numeric_string() {
    let mut fm = object(json!({"counter": "abc"}));
    let err = apply_decrement(&mut fm, "counter").unwrap_err();
    assert!(matches!(
        err,
        CompositionError::InvalidDecrementType {
            iteration: 1,
            action_index: 1,
            total_actions: 1,
            property,
            found,
            ..
        } if property == "counter" && found == "string"
    ));
}

#[test]
fn increment_error_includes_unresolved_template_excerpt() {
    let template = "{{ frontmatter(plan, 'start_phase') || 1 }}";
    let mut fm = object(json!({"phase": template}));
    let err = apply_increment(&mut fm, "phase").unwrap_err();
    let (property, found, excerpt) = match err {
        CompositionError::InvalidIncrementType {
            property,
            found,
            value_excerpt,
            ..
        } => (property, found, value_excerpt),
        other => panic!("expected InvalidIncrementType, got {other}"),
    };
    assert_eq!(property, "phase");
    assert_eq!(found, "string");
    assert!(
        excerpt.contains("{{ frontmatter(plan, 'start_phase')"),
        "excerpt should quote the template start: {excerpt}"
    );
    assert!(
        excerpt.contains("unresolved template"),
        "excerpt should explain the stage: {excerpt}"
    );
}

#[test]
fn increment_error_includes_resolved_non_numeric_excerpt() {
    let mut fm = object(json!({"area": "claudine-cli"}));
    let err = apply_increment(&mut fm, "area").unwrap_err();
    let (property, found, excerpt) = match err {
        CompositionError::InvalidIncrementType {
            property,
            found,
            value_excerpt,
            ..
        } => (property, found, value_excerpt),
        other => panic!("expected InvalidIncrementType, got {other}"),
    };
    assert_eq!(property, "area");
    assert_eq!(found, "string");
    assert!(excerpt.contains("claudine-cli"), "excerpt should quote the value: {excerpt}");
}

#[test]
fn decrement_error_includes_unresolved_template_excerpt() {
    let template = "{{ frontmatter(plan, 'start_phase') || 1 }}";
    let mut fm = object(json!({"phase": template}));
    let err = apply_decrement(&mut fm, "phase").unwrap_err();
    let (property, found, excerpt) = match err {
        CompositionError::InvalidDecrementType {
            property,
            found,
            value_excerpt,
            ..
        } => (property, found, value_excerpt),
        other => panic!("expected InvalidDecrementType, got {other}"),
    };
    assert_eq!(property, "phase");
    assert_eq!(found, "string");
    assert!(
        excerpt.contains("{{ frontmatter(plan, 'start_phase')"),
        "excerpt should quote the template start: {excerpt}"
    );
    assert!(
        excerpt.contains("unresolved template"),
        "excerpt should explain the stage: {excerpt}"
    );
}

#[test]
fn decrement_error_includes_resolved_non_numeric_excerpt() {
    let mut fm = object(json!({"area": "claudine-cli"}));
    let err = apply_decrement(&mut fm, "area").unwrap_err();
    let (property, found, excerpt) = match err {
        CompositionError::InvalidDecrementType {
            property,
            found,
            value_excerpt,
            ..
        } => (property, found, value_excerpt),
        other => panic!("expected InvalidDecrementType, got {other}"),
    };
    assert_eq!(property, "area");
    assert_eq!(found, "string");
    assert!(excerpt.contains("claudine-cli"), "excerpt should quote the value: {excerpt}");
}

#[test]
fn append_to_empty_string() {
    let mut fm = object(json!({"log": ""}));
    apply_action(
        &mut fm,
        &LoopAction::Append {
            prop: "log".into(),
            value: json!("first"),
        },
    )
    .unwrap();
    assert_eq!(fm.get("log"), Some(&json!("first")));
}

#[test]
fn prepend_to_empty_string() {
    let mut fm = object(json!({"log": ""}));
    apply_action(
        &mut fm,
        &LoopAction::Prepend {
            prop: "log".into(),
            value: json!("first"),
        },
    )
    .unwrap();
    assert_eq!(fm.get("log"), Some(&json!("first")));
}

#[test]
fn prepend_handles_scalars_and_json_values() {
    let mut fm = object(json!({"log": "tail"}));
    apply_action(
        &mut fm,
        &LoopAction::Prepend {
            prop: "log".into(),
            value: json!(true),
        },
    )
    .unwrap();
    apply_action(
        &mut fm,
        &LoopAction::Prepend {
            prop: "log".into(),
            value: json!({"event": "tick"}),
        },
    )
    .unwrap();
    apply_action(
        &mut fm,
        &LoopAction::Prepend {
            prop: "log".into(),
            value: json!([1, 2]),
        },
    )
    .unwrap();

    assert_eq!(
        fm.get("log"),
        Some(&json!("[1,2]\n{\"event\":\"tick\"}\ntruetail"))
    );
}

#[test]
fn prepend_empty_preserves_jsonl_shape() {
    let mut fm = object(json!({"objects": "{\"a\":1}", "arrays": "[1]"}));
    apply_action(
        &mut fm,
        &LoopAction::Prepend {
            prop: "objects".into(),
            value: Value::Null,
        },
    )
    .unwrap();
    apply_action(
        &mut fm,
        &LoopAction::Prepend {
            prop: "arrays".into(),
            value: json!(""),
        },
    )
    .unwrap();

    assert_eq!(fm.get("objects"), Some(&json!("{}\n{\"a\":1}")));
    assert_eq!(fm.get("arrays"), Some(&json!("[]\n[1]")));
}

#[test]
fn merge_onto_null_creates_object() {
    let mut fm = object(json!({"state": null}));
    apply_action(
        &mut fm,
        &LoopAction::Merge {
            prop: "state".into(),
            value: json!({"a": 1}),
        },
    )
    .unwrap();
    assert_eq!(fm.get("state"), Some(&json!({"a": 1})));
}

#[test]
fn merge_onto_missing_creates_object() {
    let mut fm = Map::new();
    apply_action(
        &mut fm,
        &LoopAction::Merge {
            prop: "state".into(),
            value: json!({"a": 1}),
        },
    )
    .unwrap();
    assert_eq!(fm.get("state"), Some(&json!({"a": 1})));
}
