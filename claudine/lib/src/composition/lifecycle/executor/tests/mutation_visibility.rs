//! mutation visibility executor tests.

use super::*;

/// A later stack action observes frontmatter written by an earlier action.
#[test]
fn stack_action_sees_prior_set_frontmatter() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": [
            {"set_frontmatter": ["t.md", "status", "done"]},
            {"message": "{{status}}"}
        ]}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"status": "pending"}));
    let (dir, engine) = temp_engine();
    std::fs::write(
        dir.path().join("t.md"),
        "---\nstatus: pending\n---\nbody\n",
    )
    .unwrap();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    // `source_path` is the bare file name so it resolves against the engine
    // mutation root identically to the `set_frontmatter` target.
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(recorder.events(), vec![Emitted::Message("done".to_string())]);
}

/// Cross-event visibility (review-2 High finding): a `start.stack`
/// `set_frontmatter` mutation persists into the shared per-attempt live cell,
/// so a *later* event's top-level `success.message` AND `finalize.message`
/// — each built from a separate context sharing the same cell — interpolate
/// the MUTATED value, not the original composed value. This is the harness
/// orchestration contract (`start` → `success`/`finalize`) driven at the
/// `StackExecutionContext` + shared live-frontmatter cell seam.
#[test]
fn frontmatter_mutation_in_start_is_visible_to_later_events() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {"stack": [{"action": {"set_frontmatter": ["t.md", "status", "running"]}}]},
            "success": {"message": "status={{status}}"},
            "finalize": {"message": "final={{status}}"}
        }),
        Path::new("t.md"),
    )
    .unwrap();

    // Composed base frontmatter (the original value the harness would carry
    // immutably) and the shared live cell seeded from it.
    let base = map(json!({"status": "pending"}));
    let live = std::sync::Mutex::new(base.clone());

    let (dir, engine) = temp_engine();
    std::fs::write(dir.path().join("t.md"), "---\nstatus: pending\n---\nbody\n").unwrap();
    let shell = MockShell::new(0);
    let harness = Harness::default();

    // start: runs the stack that mutates the document frontmatter.
    {
        let recorder = Recorder::default();
        let context = ctx_with_live(
            LifecycleSignal::Start,
            &base,
            &live,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        let outcome = context.execute_event(&config);
        assert_eq!(outcome, LifecycleEventOutcome::default());
    }
    // The mutation persisted into the shared cell.
    assert_eq!(live.lock().unwrap().get("status"), Some(&json!("running")));

    // success: a separate context sharing the same live cell sees `running`.
    {
        let recorder = Recorder::default();
        let context = ctx_with_live(
            LifecycleSignal::Success,
            &base,
            &live,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
        assert_eq!(
            recorder.events(),
            vec![Emitted::Message("status=running".to_string())]
        );
    }

    // finalize: likewise sees the mutated value, not the original `pending`.
    {
        let recorder = Recorder::default();
        let context = ctx_with_live(
            LifecycleSignal::Finalize,
            &base,
            &live,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
        assert_eq!(
            recorder.events(),
            vec![Emitted::Message("final=running".to_string())]
        );
    }
}

/// Negative control: with `live_frontmatter: None` (single-event caller),
/// a frontmatter mutation in one event's context does NOT leak into a later
/// event built from its own base frontmatter — behavior is unchanged from
/// before the cross-event cell existed. The `success` context, given the
/// original base, resolves `{{status}}` against `pending`.
#[test]
fn without_live_cell_later_event_resolves_against_its_own_base() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {"stack": [{"action": {"set_frontmatter": ["t.md", "status", "running"]}}]},
            "success": {"message": "status={{status}}"}
        }),
        Path::new("t.md"),
    )
    .unwrap();

    let (dir, engine) = temp_engine();
    std::fs::write(dir.path().join("t.md"), "---\nstatus: pending\n---\nbody\n").unwrap();
    let shell = MockShell::new(0);
    let harness = Harness::default();

    // start: single-event context (no shared cell). Its stack mutation is
    // visible only intra-stack and discarded when the stack returns.
    let start_fm = map(json!({"status": "pending"}));
    {
        let recorder = Recorder::default();
        let context = ctx(
            LifecycleSignal::Start,
            &start_fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
    }

    // success: a fresh single-event context with the ORIGINAL base sees the
    // original value, proving the None path carries no cross-event state.
    let success_fm = map(json!({"status": "pending"}));
    let recorder = Recorder::default();
    let context = ctx(
        LifecycleSignal::Success,
        &success_fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("status=pending".to_string())]
    );
}

