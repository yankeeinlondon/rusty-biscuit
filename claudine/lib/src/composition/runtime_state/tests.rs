use super::*;
use serde_json::json;

fn engine() -> EffectEngine {
    EffectEngine::builder().build()
}

fn base() -> Map<String, Value> {
    Map::new()
}

#[test]
fn set_returns_document_prior_then_mutation_prior() {
    let state = RuntimeState::new();
    let engine = engine();
    let mut document = base();
    document.insert("phase".into(), json!("plan"));

    let first = state.set(&engine, "phase", json!("build"), &document).unwrap();
    assert_eq!(first, json!("plan"), "first write reports the document's value");

    let second = state.set(&engine, "phase", json!("ship"), &document).unwrap();
    assert_eq!(second, json!("build"), "later writes report the prior mutation");

    assert_eq!(state.snapshot().mutations.get("phase"), Some(&json!("ship")));
}

#[test]
fn set_reports_null_for_a_key_absent_everywhere() {
    let state = RuntimeState::new();
    let prior = state.set(&engine(), "fresh", json!(1), &base()).unwrap();
    assert_eq!(prior, Value::Null);
}

#[test]
fn set_preserves_whole_value_types() {
    let state = RuntimeState::new();
    let engine = engine();
    for (key, value) in [
        ("flag", json!(true)),
        ("count", json!(3)),
        ("list", json!(["a", "b"])),
        ("obj", json!({"nested": 1})),
        ("nil", json!(null)),
    ] {
        state.set(&engine, key, value.clone(), &base()).unwrap();
        assert_eq!(state.snapshot().mutations.get(key), Some(&value), "{key} kept its type");
    }
}

#[test]
fn set_rejects_every_reserved_root_key() {
    let state = RuntimeState::new();
    let engine = engine();
    for key in ["state", "previous", "next", "outputs", "sequence_id"] {
        let error = state
            .set(&engine, key, json!("x"), &base())
            .expect_err("reserved key must be refused");
        assert!(
            matches!(&error, RuntimeMutationError::ReservedKey { key: refused } if refused == key),
            "{key} produced {error:?}"
        );
    }
    assert!(state.snapshot().mutations.is_empty(), "a refused write leaves no trace");
}

/// A *generated step-state* key is reserved inside a step's `state` object, not
/// at the frontmatter root, so an ordinary document key of the same name stays
/// writable.
#[test]
fn set_allows_generated_state_key_names_at_the_document_root() {
    let state = RuntimeState::new();
    let engine = engine();
    for key in ["id", "index", "count", "is_first", "is_last"] {
        state
            .set(&engine, key, json!(1), &base())
            .unwrap_or_else(|error| panic!("{key} must stay writable at the root: {error}"));
    }
}

#[test]
fn set_rejects_empty_and_dotted_keys() {
    let state = RuntimeState::new();
    let engine = engine();
    for key in ["", "a.b"] {
        let error = state
            .set(&engine, key, json!("x"), &base())
            .expect_err("malformed key must be refused");
        assert!(
            matches!(error, RuntimeMutationError::Effect(_)),
            "{key:?} produced {error:?}"
        );
    }
}

#[test]
fn outputs_accumulate_in_commit_order() {
    let state = RuntimeState::new();
    state.append_output("first");
    state.append_output("second");
    state.append_output_entry(json!(["a", "b"]));

    assert_eq!(state.outputs_value(), json!(["first", "second", ["a", "b"]]));
    assert_eq!(state.output_count(), 3);
}

#[test]
fn one_trailing_transport_newline_is_removed_and_other_whitespace_kept() {
    assert_eq!(trim_transport_newline("done\n"), "done");
    assert_eq!(trim_transport_newline("done\r\n"), "done");
    assert_eq!(trim_transport_newline("done\n\n"), "done\n");
    assert_eq!(trim_transport_newline("  done  "), "  done  ");
    assert_eq!(trim_transport_newline("a\nb\n"), "a\nb");
    assert_eq!(trim_transport_newline(""), "");
    assert_eq!(trim_transport_newline("\n"), "");
}

#[test]
fn appended_output_uses_the_transport_newline_policy() {
    let state = RuntimeState::new();
    state.append_output("summary text\n");
    assert_eq!(state.outputs_value(), json!(["summary text"]));
}

#[test]
fn layer_precedence_is_setters_then_mutations_then_overlay() {
    let state = RuntimeState::new();
    let engine = engine();
    state.set(&engine, "shared", json!("mutation"), &base()).unwrap();
    state.append_output("prior");

    let overrides = layered_set_overrides(
        Some(&json!({"shared": "setter", "only_setter": 1})),
        Some(&state.snapshot()),
        Some(&json!({"state": {"name": "blue"}})),
    );

    assert_eq!(overrides["shared"], json!("mutation"), "mutations outrank user setters");
    assert_eq!(overrides["only_setter"], json!(1));
    assert_eq!(overrides["state"], json!({"name": "blue"}), "the overlay is layered last");
    assert_eq!(overrides[OUTPUTS_KEY], json!(["prior"]));
}

#[test]
fn a_reserved_overlay_key_cannot_be_displaced_by_a_setter() {
    let overrides = layered_set_overrides(
        Some(&json!({"state": "hijacked", "outputs": ["hijacked"]})),
        Some(&RuntimeState::new().snapshot()),
        Some(&json!({"state": {"name": "blue"}})),
    );
    assert_eq!(overrides["state"], json!({"name": "blue"}));
    assert_eq!(overrides[OUTPUTS_KEY], json!([]), "the accumulator wins over a setter");
}

#[test]
fn outputs_is_initialized_even_with_no_runtime_state() {
    let overrides = layered_set_overrides(None, None, None);
    assert_eq!(overrides[OUTPUTS_KEY], json!([]));
}

#[test]
fn with_initialized_outputs_seeds_only_when_absent() {
    let seeded = with_initialized_outputs(Some(json!({"topic": "rust"})));
    assert_eq!(seeded[OUTPUTS_KEY], json!([]));
    assert_eq!(seeded["topic"], json!("rust"));

    let preserved = with_initialized_outputs(Some(json!({OUTPUTS_KEY: ["kept"]})));
    assert_eq!(preserved[OUTPUTS_KEY], json!(["kept"]));

    assert_eq!(with_initialized_outputs(None), json!({ OUTPUTS_KEY: [] }));
}
