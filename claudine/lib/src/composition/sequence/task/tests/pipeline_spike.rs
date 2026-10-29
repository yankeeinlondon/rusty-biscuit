//! Spike scenarios for group and container execution (C1, C3, and A4's group
//! step), driven through the same fixture as the task tests. See
//! `crate::pipeline_spike` for how to run them.

use super::*;
use crate::composition::runtime_state::RuntimeSnapshot;
use crate::pipeline_spike::{measure, report, text_of};

/// A runtime cell holding `outputs` committed outputs of `output_bytes` each
/// and one `set` write of `mutation_bytes`.
fn seeded_runtime(outputs: usize, output_bytes: usize, mutation_bytes: usize) -> Arc<RuntimeState> {
    let mut snapshot = RuntimeSnapshot::default();
    if mutation_bytes > 0 {
        snapshot
            .mutations
            .insert("base_state".into(), Value::String(text_of(mutation_bytes)));
    }
    snapshot.outputs = (0..outputs)
        .map(|_| Value::String(text_of(output_bytes)))
        .collect();
    Arc::new(RuntimeState::from_snapshot(snapshot))
}

/// A one-step sequence whose step is a group of `legs` shell legs, the
/// sequence document carrying `frontmatter_bytes` of other properties.
fn group_fixture(legs: usize, max_parallel: usize, frontmatter_bytes: usize) -> Fixture {
    let dir = TempDir::new().unwrap();
    let tasks: Vec<Value> = (0..legs)
        .map(|index| json!({"name": format!("leg-{index}"), "shell": format!("echo {index}")}))
        .collect();
    let step = json!({"name": "fan-out", "group": {"name": "legs", "max_parallel": max_parallel, "tasks": tasks}});
    let source = write_source(
        dir.path(),
        "seq.md",
        &[
            ("payload", Value::String(text_of(frontmatter_bytes))),
            ("sequence", json!([step])),
        ],
        "Body.\n",
    );
    Fixture::build(dir, &source).unwrap()
}

/// The live document view a group starts from, as [`Fixture::execute_step`]
/// seeds it; each admitted leg's cell is a clone of it (`group/branch.rs`).
fn group_start_view(fixture: &Fixture) -> darkmatter::markdown::compose::SharedFrontmatter {
    darkmatter::markdown::compose::SharedFrontmatter::from(fixture.frontmatter.clone())
}

/// C1: a capped group's per-leg state, a private runtime branch and
/// live-frontmatter view built for each leg when it is admitted and released
/// before the next admission.
#[test]
#[ignore = "measurement spike; run explicitly"]
fn pipeline_spike_c1_group_leg_state() {
    for (legs, frontmatter_bytes) in [10usize, 100, 1000]
        .into_iter()
        .flat_map(|legs| [(legs, 0), (legs, 100 * 1024)])
    {
        let fixture = group_fixture(legs, 4, frontmatter_bytes);
        let recorder = Recorder::default();
        let measured = measure(3, || {
            let fake = FakeTaskShell::default();
            let runtime = seeded_runtime(0, 0, 100 * 1024);
            let mut wiring = Wiring::new(&recorder, &fake);
            wiring.runtime = Some(&runtime);
            let outcome = fixture.execute(&wiring);
            assert!(outcome.succeeded(), "{}", failure_message(&outcome));
            outcome.group_tasks.len()
        });
        report(
            "C1",
            &format!(
                "legs={legs}/max_parallel=4/base=100KB/fm-payload={}KB",
                frontmatter_bytes / 1024
            ),
            &measured,
        );
        // One leg's live-frontmatter cell, built as the group builds it: a
        // clone of the group-start view, whose document holds the inline
        // group definition itself.
        let view = group_start_view(&fixture);
        let cell = measure(3, || std::sync::Mutex::new(view.clone()));
        report(
            "C1",
            &format!(
                "legs={legs}/fm-payload={}KB/one-live-frontmatter-view",
                frontmatter_bytes / 1024
            ),
            &cell,
        );
        let runtime = seeded_runtime(0, 0, 100 * 1024);
        let branch = measure(3, || RuntimeState::from_snapshot(runtime.snapshot()));
        report("C1", &format!("legs={legs}/one-runtime-branch"), &branch);
    }
}

/// C3: what one container event's full snapshot copies, and how many full
/// snapshots a nested sequence container takes.
#[test]
#[ignore = "measurement spike; run explicitly"]
fn pipeline_spike_c3_container_snapshots() {
    for outputs in [10usize, 100, 1000] {
        let runtime = seeded_runtime(outputs, 10 * 1024, 1024);
        let single = measure(9, || runtime.snapshot().mutations.len());
        report(
            "C3",
            &format!("outputs={outputs}x10KB/one-snapshot-for-mutations"),
            &single,
        );

        let dir = TempDir::new().unwrap();
        let step = json!({"name": "inner", "sequence": [
            {"name": "a", "shell": "echo a"},
            {"name": "b", "shell": "echo b"},
            {"name": "c", "shell": "echo c"}
        ]});
        let source = one_step_source(dir.path(), step);
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let container = measure(3, || {
            let fake = FakeTaskShell::default();
            let runtime = seeded_runtime(outputs, 10 * 1024, 1024);
            let mut wiring = Wiring::new(&recorder, &fake);
            wiring.runtime = Some(&runtime);
            let outcome = fixture.execute(&wiring);
            assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        });
        report(
            "C3",
            &format!("outputs={outputs}x10KB/nested-sequence-of-3"),
            &container,
        );
    }
}

/// A4's group step: 200 legs at `max_parallel: 4` after 49 committed prompt
/// outputs of 10 KB, under a 100 KB document and 100 KB of runtime writes.
#[test]
#[ignore = "measurement spike; run explicitly"]
fn pipeline_spike_a4_group_step() {
    let fixture = group_fixture(200, 4, 100 * 1024);
    let recorder = Recorder::default();
    let measured = measure(5, || {
        let fake = FakeTaskShell::default();
        let runtime = seeded_runtime(49, 10 * 1024, 100 * 1024);
        let mut wiring = Wiring::new(&recorder, &fake);
        wiring.runtime = Some(&runtime);
        let outcome = fixture.execute(&wiring);
        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    });
    report("A4", "group-step/legs=200/max_parallel=4", &measured);
    // The seeded runtime itself is part of the measured scope; report it so
    // it can be subtracted.
    let seed = measure(5, || seeded_runtime(49, 10 * 1024, 100 * 1024));
    report("A4", "group-step/seed-only", &seed);
    // Per-unit costs at this shape, for the item shares.
    let runtime = seeded_runtime(49, 10 * 1024, 100 * 1024);
    report(
        "A4",
        "group-step/one-full-snapshot",
        &measure(9, || runtime.snapshot()),
    );
    // What a narrow mutations-only projection would copy instead.
    let mutations = runtime.snapshot().mutations;
    report(
        "A4",
        "group-step/one-mutations-only-copy",
        &measure(9, || mutations.clone()),
    );
    let view = group_start_view(&fixture);
    report(
        "A4",
        "group-step/one-live-frontmatter-view",
        &measure(9, || std::sync::Mutex::new(view.clone())),
    );
    let base = seeded_runtime(0, 0, 100 * 1024);
    report(
        "A4",
        "group-step/one-runtime-branch-without-outputs",
        &measure(9, || RuntimeState::from_snapshot(base.snapshot())),
    );
    let outer = runtime.snapshot();
    report(
        "A4",
        "group-step/one-runtime-branch",
        &measure(9, || RuntimeState::from_snapshot(outer.clone())),
    );
}
