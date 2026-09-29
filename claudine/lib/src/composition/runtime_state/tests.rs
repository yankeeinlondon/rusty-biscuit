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
fn set_batch_is_atomic_reports_priors_and_preserves_explicit_null_presence() {
    let state = RuntimeState::new();
    let engine = engine();
    let mut document = base();
    document.insert("left".into(), json!("A"));
    document.insert("right".into(), json!("B"));
    document.insert("nullable".into(), json!("document fallback"));

    state.set(&engine, "nullable", Value::Null, &document).unwrap();
    let mut updates = IndexMap::new();
    updates.insert("left".into(), json!("B"));
    updates.insert("right".into(), json!("A"));
    updates.insert("nullable".into(), json!("replacement"));

    let prior = state.set_batch(&engine, &updates, &document).unwrap();
    assert_eq!(prior["left"], json!("A"));
    assert_eq!(prior["right"], json!("B"));
    assert_eq!(prior["nullable"], Value::Null);
    let first_read = state.snapshot();
    assert_eq!(first_read.mutations.get("left"), Some(&json!("B")));
    assert_eq!(first_read.mutations.get("right"), Some(&json!("A")));
    assert_eq!(first_read.mutations.get("nullable"), Some(&json!("replacement")));
    assert_eq!(state.snapshot().mutations, first_read.mutations, "read/write/read is stable");

    let empty = IndexMap::new();
    assert!(state.set_batch(&engine, &empty, &document).unwrap().is_empty());
}

#[test]
fn set_batch_rejects_a_late_bad_key_without_publishing_earlier_values() {
    let state = RuntimeState::new();
    let engine = engine();
    let mut updates = IndexMap::new();
    updates.insert("valid".into(), json!(1));
    updates.insert("a.b".into(), json!(2));

    state
        .set_batch(&engine, &updates, &base())
        .expect_err("the complete batch must be refused");
    assert!(state.snapshot().mutations.is_empty());
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
        LayeredOverrides::authored(Some(&json!({"shared": "setter", "only_setter": 1}))),
        Some(&state.snapshot()),
        Some(&json!({"state": {"name": "blue"}})),
    );
    let values = overrides.values();

    assert_eq!(values["shared"], json!("mutation"), "mutations outrank user setters");
    assert_eq!(values["only_setter"], json!(1));
    assert_eq!(values["state"], json!({"name": "blue"}), "the overlay is layered last");
    assert_eq!(values[OUTPUTS_KEY], json!(["prior"]));
}

#[test]
fn only_user_setters_are_authored_and_every_runtime_layer_is_data() {
    let state = RuntimeState::new();
    state
        .set(&engine(), "carried", json!("see {{ title }}"), &base())
        .unwrap();
    state.append_output("INJECTED");

    let overrides = layered_set_overrides(
        LayeredOverrides::authored(Some(&json!({"typed": "{{ title }}", "carried": "x"}))),
        Some(&state.snapshot()),
        Some(&json!({"state": {"name": "{{ title }}"}})),
    );

    assert_eq!(overrides.origin_of("typed"), OverrideOrigin::Authored);
    for key in ["carried", OUTPUTS_KEY, "state"] {
        assert_eq!(overrides.origin_of(key), OverrideOrigin::Data, "`{key}` is run output");
    }
    assert_eq!(
        overrides.values()["carried"],
        json!("see {{ title }}"),
        "a data value stays raw: no token, no escaping"
    );
    assert_eq!(overrides.values()[OUTPUTS_KEY], json!(["INJECTED"]));
}

#[test]
fn a_later_layer_gives_a_key_its_own_origin() {
    let mut overrides = LayeredOverrides::data(Some(&json!({"note": "from proxy", "kept": 1})));
    overrides.push(OverrideOrigin::Authored, Some(&json!({"note": "typed"})));
    assert_eq!(overrides.origin_of("note"), OverrideOrigin::Authored);
    assert_eq!(overrides.values()["note"], json!("typed"));
    assert_eq!(overrides.origin_of("kept"), OverrideOrigin::Data);

    overrides.push(OverrideOrigin::Data, Some(&json!({"note": "produced"})));
    assert_eq!(overrides.origin_of("note"), OverrideOrigin::Data);

    let mut extended = LayeredOverrides::authored(Some(&json!({"kept": 2, "own": 3})));
    extended.extend(&overrides);
    assert_eq!(extended.origin_of("kept"), OverrideOrigin::Data);
    assert_eq!(extended.origin_of("own"), OverrideOrigin::Authored);
}

#[test]
fn from_parts_ignores_data_keys_that_are_no_longer_present() {
    let data_keys = ["gone".to_string(), "kept".to_string()].into_iter().collect();
    let overrides = LayeredOverrides::from_parts(Some(&json!({"kept": 1, "typed": 2})), &data_keys);
    assert_eq!(overrides.data_keys().iter().collect::<Vec<_>>(), vec!["kept"]);
    assert_eq!(overrides.origin_of("typed"), OverrideOrigin::Authored);
    assert_eq!(overrides.origin_of("gone"), OverrideOrigin::Authored, "absent reads as authored");
}

/// The Darkmatter boundary: authored keys still fill in, data keys stay exact.
#[test]
fn apply_to_hands_authored_keys_as_templates_and_data_keys_verbatim() {
    let mut overrides = LayeredOverrides::authored(Some(&json!({"typed": "{{ title }}"})));
    overrides.push(
        OverrideOrigin::Data,
        Some(&json!({"produced": "{{ title }} and {{…}} and $(echo X)"})),
    );
    let markdown: darkmatter::markdown::Markdown =
        "---\ntitle: t\n---\n[{{ typed }}] [{{ produced }}]\n".into();
    let (composed, _) = markdown
        .compose_with(overrides.apply_to(ComposeOptions::new()))
        .unwrap();
    assert_eq!(
        composed.content().trim(),
        "[t] [{{ title }} and {{…}} and $(echo X)]"
    );
}

#[test]
fn a_reserved_overlay_key_cannot_be_displaced_by_a_setter() {
    let overrides = layered_set_overrides(
        LayeredOverrides::authored(Some(&json!({"state": "hijacked", "outputs": ["hijacked"]}))),
        Some(&RuntimeState::new().snapshot()),
        Some(&json!({"state": {"name": "blue"}})),
    );
    assert_eq!(overrides.values()["state"], json!({"name": "blue"}));
    assert_eq!(overrides.values()[OUTPUTS_KEY], json!([]), "the accumulator wins over a setter");
}

#[test]
fn outputs_is_initialized_even_with_no_runtime_state() {
    let overrides = layered_set_overrides(LayeredOverrides::new(), None, None);
    assert_eq!(overrides.values()[OUTPUTS_KEY], json!([]));
    assert_eq!(overrides.origin_of(OUTPUTS_KEY), OverrideOrigin::Data);
}

#[test]
fn initialize_outputs_seeds_only_when_absent() {
    let mut seeded = LayeredOverrides::authored(Some(&json!({"topic": "rust"})));
    seeded.initialize_outputs();
    assert_eq!(seeded.values()[OUTPUTS_KEY], json!([]));
    assert_eq!(seeded.origin_of(OUTPUTS_KEY), OverrideOrigin::Data);
    assert_eq!(seeded.values()["topic"], json!("rust"));

    let mut preserved = LayeredOverrides::authored(Some(&json!({OUTPUTS_KEY: ["kept"]})));
    preserved.initialize_outputs();
    assert_eq!(preserved.values()[OUTPUTS_KEY], json!(["kept"]));

    let mut empty = LayeredOverrides::new();
    empty.initialize_outputs();
    assert_eq!(empty.to_value(), json!({ OUTPUTS_KEY: [] }));
}
