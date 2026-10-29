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
    for key in ["seq", "group", "loop"] {
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
        Some(&json!({"seq": {"name": "blue"}})),
    );
    let values = overrides.values();

    assert_eq!(values["shared"], json!("mutation"), "mutations outrank user setters");
    assert_eq!(values["only_setter"], json!(1));
    assert_eq!(overrides.sequence_value().unwrap()["name"], json!("blue"), "the overlay is layered last");
    assert_eq!(overrides.sequence_value().unwrap()["outputs"], json!(["prior"]));
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
        Some(&json!({"seq": {"name": "{{ title }}"}})),
    );

    assert_eq!(overrides.origin_of("typed"), OverrideOrigin::Authored);
    assert_eq!(overrides.origin_of("carried"), OverrideOrigin::Data);
    assert_eq!(
        overrides.values()["carried"],
        json!("see {{ title }}"),
        "a data value stays raw: no token, no escaping"
    );
    assert_eq!(overrides.sequence_value().unwrap()["outputs"], json!(["INJECTED"]));
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
        .compose_with(
            &darkmatter::markdown::compose::ComposeRequest::with_context(
                overrides.apply_to(ComposeOptions::new()),
                crate::test_support::context(),
            )
            .unwrap(),
        )
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
        Some(&json!({"seq": {"name": "blue"}})),
    );
    assert_eq!(overrides.sequence_value().unwrap()["name"], json!("blue"));
    assert_eq!(overrides.values()["state"], json!("hijacked"));
    assert_eq!(overrides.values()["outputs"], json!(["hijacked"]));
    assert_eq!(overrides.sequence_value().unwrap()["outputs"], json!([]), "the accumulator wins over a setter");
}

#[test]
fn outputs_is_initialized_even_with_no_runtime_state() {
    let overrides = layered_set_overrides(LayeredOverrides::new(), None, None);
    assert_eq!(overrides.sequence_value().unwrap()["outputs"], json!([]));
    assert!(!overrides.values().contains_key("outputs"));
}

#[test]
fn initialize_outputs_seeds_only_when_absent() {
    let mut seeded = LayeredOverrides::authored(Some(&json!({"topic": "rust"})));
    seeded.initialize_outputs();
    assert_eq!(seeded.sequence_value().unwrap()["outputs"], json!([]));
    assert!(!seeded.values().contains_key("outputs"));
    assert_eq!(seeded.values()["topic"], json!("rust"));

    let mut preserved = LayeredOverrides::authored(Some(&json!({OUTPUTS_KEY: ["kept"]})));
    preserved.initialize_outputs();
    assert_eq!(preserved.values()[OUTPUTS_KEY], json!(["kept"]));

    let mut empty = LayeredOverrides::new();
    empty.initialize_outputs();
    assert_eq!(empty.to_value(), json!({}));
    assert_eq!(empty.sequence_value().unwrap()["outputs"], json!([]));
}

#[test]
fn container_scope_restore_and_retry_keep_native_results_and_mutations() {
    let state = RuntimeState::new();
    state.set(&engine(), "note", json!("retained"), &base()).unwrap();
    state.append_output_entry(json!({"alpha": [1, null], "beta": false}));
    let outer = state.snapshot();
    state.replace_sequence_scope(Some(json!({"name": "inner"})), Vec::new());
    state.append_output_entry(json!([{"deep": 2}]));
    assert_eq!(state.outputs_value(), json!([[{"deep": 2}]]));
    state.clear_outputs();
    assert_eq!(state.outputs_value(), json!([]));
    assert_eq!(state.snapshot().mutations.get("note"), Some(&json!("retained")));
    state.replace_sequence_scope(outer.seq, outer.outputs);
    assert_eq!(state.outputs_value(), json!([{"alpha": [1, null], "beta": false}]));
    assert!(state.last_output_text().is_none());
}

/// A cell with two mutations, three outputs, and a position.
fn populated() -> RuntimeState {
    let state = RuntimeState::new();
    state.set(&engine(), "first", json!(1), &base()).unwrap();
    state.set(&engine(), "second", json!({"nested": [true]}), &base()).unwrap();
    for entry in [json!("a"), json!(["b"]), json!({"c": null})] {
        state.append_output_entry(entry);
    }
    state.set_sequence_binding(json!({"name": "step", "run_id": "r"}));
    state
}

#[test]
fn projections_equal_the_matching_fields_of_a_full_snapshot() {
    let state = populated();
    let full = state.snapshot();

    assert_eq!(state.mutations(), full.mutations);
    assert_eq!(state.mutation_keys(), ["first", "second"], "first-write order");
    assert_eq!(state.mutations_and_outputs(), (full.mutations.clone(), full.outputs.clone()));
    assert_eq!(state.mutations_and_output_count(), (full.mutations.clone(), 3));
    assert_eq!(state.outputs_from(1), full.outputs[1..]);
    assert_eq!(state.outputs_from(7), Vec::<Value>::new(), "a start past the end is empty");
    assert_eq!(state.sequence_scope(), (full.seq.clone(), full.outputs.clone()));
    assert_eq!(state.sequence_value(), full.sequence_value());
    assert_eq!(RuntimeState::new().sequence_value(), RuntimeSnapshot::default().sequence_value(), "standalone");
}

#[test]
fn replace_sequence_scope_moves_the_enclosing_scope_out_and_back() {
    let state = populated();
    let before = state.snapshot();

    let (outer_seq, outer_outputs) = state.replace_sequence_scope(Some(json!({"name": "inner"})), Vec::new());
    assert_eq!((outer_seq.clone(), outer_outputs.clone()), (before.seq.clone(), before.outputs.clone()));
    state.append_output_entry(json!("inner"));
    state.set(&engine(), "child", json!("kept"), &base()).unwrap();

    let (inner_seq, inner_outputs) = state.replace_sequence_scope(outer_seq, outer_outputs);
    assert_eq!((inner_seq, inner_outputs), (Some(json!({"name": "inner"})), vec![json!("inner")]));
    let after = state.snapshot();
    assert_eq!((after.seq, after.outputs), (before.seq, before.outputs), "the enclosing scope is restored");
    assert_eq!(after.mutations.get("child"), Some(&json!("kept")), "child mutations are not undone");
}

/// A writer publishes `len` and an output list of that length under one lock;
/// a reader of either pair projection never sees one without the other.
#[test]
fn multi_field_projections_are_coherent_under_a_concurrent_writer() {
    const WRITES: usize = 2_000;
    let state = RuntimeState::new();
    let done = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            for write in 1..=WRITES {
                let mut inner = state.inner.lock().unwrap();
                inner.mutations.insert("len".into(), json!(write));
                inner.outputs.push(json!(write));
                inner.seq = Some(json!({"len": write}));
            }
            done.store(true, std::sync::atomic::Ordering::Release);
        });
        let len_of = |mutations: &FrontmatterMap| mutations.get("len").and_then(Value::as_u64).unwrap_or(0) as usize;
        let mut reads = 0usize;
        loop {
            let finished = done.load(std::sync::atomic::Ordering::Acquire);
            let (mutations, outputs) = state.mutations_and_outputs();
            assert_eq!(len_of(&mutations), outputs.len(), "mutations and outputs torn");
            let (mutations, count) = state.mutations_and_output_count();
            assert_eq!(len_of(&mutations), count, "mutations and output count torn");
            let (seq, outputs) = state.sequence_scope();
            let position = seq.as_ref().and_then(|seq| seq["len"].as_u64()).unwrap_or(0) as usize;
            assert_eq!(position, outputs.len(), "position and outputs torn");
            reads += 1;
            if finished { break; }
        }
        assert!(reads > 0);
    });
    assert_eq!(state.output_count(), WRITES);
}

#[cfg(feature = "work-counters")]
#[test]
fn field_projections_take_no_full_snapshot_and_copy_only_the_outputs_they_return() {
    use crate::work_counters::{Work, snapshot};
    let state = populated();
    for _ in 0..97 { state.append_output_entry(json!("history")); }

    let before = snapshot();
    let _ = state.mutations();
    let _ = state.mutation_keys();
    let _ = state.mutations_and_output_count();
    let mutation_reads = snapshot().since(&before);
    assert_eq!(mutation_reads.get(Work::RuntimeSnapshot), 0);
    assert_eq!(mutation_reads.get(Work::RuntimeOutputsCopy), 0, "mutation reads copy no output history");

    let before = snapshot();
    let tail = state.outputs_from(95);
    let _ = state.mutations_and_outputs();
    let _ = state.sequence_scope();
    let output_reads = snapshot().since(&before);
    assert_eq!(output_reads.get(Work::RuntimeSnapshot), 0);
    assert_eq!(output_reads.get(Work::RuntimeOutputsCopy), tail.len() as u64 + 2 * 100, "only the entries each read returns");

    let before = snapshot();
    let _ = state.snapshot();
    let full = snapshot().since(&before);
    assert_eq!((full.get(Work::RuntimeSnapshot), full.get(Work::RuntimeOutputsCopy)), (1, 100));
}
