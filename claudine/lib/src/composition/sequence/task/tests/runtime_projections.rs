//! Task, leg, and container execution read the runtime fields they use through
//! narrow projections: a full `RuntimeState` snapshot is taken only where a
//! group seeds its private state, and a task or nested sequence never copies
//! the enclosing output history it does not read.

use super::*;
use crate::composition::runtime_state::RuntimeSnapshot;
use crate::work_counters::{Work, WorkSnapshot, snapshot};

/// The committed-output counts each test seeds; the work must not differ.
const HISTORY: [usize; 2] = [10, 100];

/// A runtime cell holding `outputs` committed outputs of 10 KB each and one
/// `set` write.
fn seeded_runtime(outputs: usize) -> Arc<RuntimeState> {
    let mut seed = RuntimeSnapshot::default();
    seed.mutations.insert("base_state".into(), json!("seeded"));
    seed.outputs = (0..outputs).map(|index| Value::String(format!("{index}:{}", "x".repeat(10 * 1024)))).collect();
    Arc::new(RuntimeState::from_snapshot(seed))
}

/// Run a one-step sequence whose step is `step` over a runtime seeded with
/// `outputs` committed outputs; returns the work done and the runtime after.
fn run_over(step: Value, outputs: usize) -> (WorkSnapshot, Arc<RuntimeState>) {
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), step);
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let runtime = seeded_runtime(outputs);
    let mut wiring = Wiring::new(&recorder, &shell);
    wiring.runtime = Some(&runtime);
    let before = snapshot();
    let outcome = fixture.execute(&wiring);
    let work = snapshot().since(&before);
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    (work, runtime)
}

/// The step's work at each [`HISTORY`] size: no full snapshot, and the same
/// number of output entries copied however long the enclosing history is.
/// The enclosing outputs come back intact, followed by the step's result.
fn assert_history_independent(step: &Value, what: &str) {
    let mut copied = Vec::new();
    for outputs in HISTORY {
        let (work, runtime) = run_over(step.clone(), outputs);
        assert_eq!(work.get(Work::RuntimeSnapshot), 0, "{what} over {outputs} outputs: {:?}", work.nonzero());
        copied.push(work.get(Work::RuntimeOutputsCopy));
        let after = runtime.snapshot();
        assert_eq!(after.outputs.len(), outputs + 1, "{what}: the enclosing outputs and the step's result");
        assert_eq!(after.outputs[..outputs], seeded_runtime(outputs).snapshot().outputs[..], "{what}: enclosing outputs restored in order");
        assert_eq!(after.mutations.get("base_state"), Some(&json!("seeded")), "{what}: earlier writes kept");
    }
    assert_eq!(copied[0], copied[1], "{what}: output entries copied at {HISTORY:?} enclosing outputs");
}

#[test]
fn a_shell_task_reads_runtime_state_without_a_snapshot_or_an_output_copy() {
    let step = json!({ "name": "t", "shell": "echo a" });
    assert_history_independent(&step, "a shell task");
    let (work, _) = run_over(step, 100);
    assert_eq!(work.get(Work::RuntimeOutputsCopy), 0, "an unguarded task without templates reads no `seq`");
}

#[test]
fn a_nested_sequence_moves_the_enclosing_outputs_instead_of_copying_them() {
    let step = json!({ "name": "inner", "sequence": [
        { "name": "a", "shell": "echo a" },
        { "name": "b", "side_effect": { "set": { "seen": "b" } } },
        { "name": "c", "shell": "echo c" },
    ] });
    assert_history_independent(&step, "a nested sequence of three");
    let (_, runtime) = run_over(step, 10);
    assert_eq!(runtime.mutations().get("seen"), Some(&json!("b")), "a child's write reaches the parent");
}

#[test]
fn a_looping_sequence_copies_only_its_own_passes() {
    let step = json!({
        "name": "inner",
        "loop": { "max": 3, "until": "loop.count >= 3" },
        "sequence": [{ "name": "a", "shell": "echo a" }],
    });
    assert_history_independent(&step, "a three-pass looping sequence");
}

/// A group seeds its own private state and each parallel leg's buffer from
/// one full snapshot each; no leg takes another.
#[test]
fn a_group_takes_two_full_snapshots_at_any_leg_count() {
    for legs in [10usize, 50] {
        let tasks: Vec<Value> = (0..legs)
            .map(|index| json!({ "name": format!("leg-{index}"), "shell": format!("echo {index}") }))
            .collect();
        let step = json!({ "name": "fan-out", "group": { "name": "legs", "max_parallel": 4, "tasks": tasks } });
        let (work, runtime) = run_over(step, 10);
        assert_eq!(work.get(Work::RuntimeSnapshot), 2, "{legs} legs: {:?}", work.nonzero());
        assert_eq!(runtime.output_count(), 11, "{legs} legs publish one group result");
    }
}
