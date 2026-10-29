//! A looping or speaking container reads its state view in place: resolving a
//! templated cap, staging a pass's action, and classifying a start phrase never
//! copy the definitions the state holds, so the work per container does not
//! grow with how many sibling containers the sequence declares.

use super::*;
use crate::composition::lifecycle::prewarm::PrewarmWorker;
use crate::composition::lifecycle::prewarm::fakes::FakePreparer;
use crate::invocation_context::{DocumentEnvironment, InvocationContext};
use crate::work_counters::{Work, snapshot};
use darkmatter::markdown::compose::RequestSnapshot;

/// A one-step sequence whose step holds `containers` sibling inline
/// sequences, each built by `container` from its index.
fn siblings(containers: usize, container: impl Fn(usize) -> Value) -> Fixture {
    let dir = TempDir::new().unwrap();
    let children: Vec<Value> = (0..containers).map(container).collect();
    let step = json!({ "name": "outer", "sequence": children });
    let source = write_source(
        dir.path(),
        "seq.md",
        &[("title", json!("Release")), ("cap", json!(3)), ("n", json!(0)), ("sequence", json!([step]))],
        "Body.\n",
    );
    Fixture::build(dir, &source).unwrap()
}

/// A container that loops twice under a templated cap, incrementing `n`.
fn looping(index: usize) -> Value {
    json!({
        "name": format!("c{index}"),
        "loop": { "max": "{{ cap }}", "until": "loop.count >= 2", "action": "increment(n)" },
        "sequence": [{ "name": "work", "shell": format!("echo {index}") }],
    })
}

/// A container whose `start` speaks a frontmatter read.
fn speaking(index: usize) -> Value {
    json!({
        "name": format!("c{index}"),
        "start": [{ "say": "{{ title }} starts" }],
        "sequence": [{ "name": "work", "shell": format!("echo {index}") }],
    })
}

/// An invocation view that submits pre-warm candidates to `worker`.
fn speaking_document(dir: &Path, worker: &PrewarmWorker) -> InvocationContext {
    let root = biscuit_file::canonicalize_simplified(dir).unwrap();
    let environment = DocumentEnvironment::from_iter([("CLAUDINE_CONTAINER_STATE_PROBE", "1")]);
    InvocationContext::capture(
        &RequestSnapshot::new(&root).with_home(Some(root.join("home"))).with_env(environment.vars().clone()),
    )
    .unwrap()
    .with_prewarm(worker.handle())
}

/// The container counts each test runs at. The middle size sits next to the
/// small one, which keeps the test's total work low (each container runs a
/// full preparation) while still measuring the work per container at both
/// ends of the range.
const SIZES: [usize; 3] = [10, 11, 100];

/// Assert `totals` (one per [`SIZES`] entry) grow linearly: each container
/// adds the same work at 10 and at 100 siblings. A copy of the definitions
/// per container makes the later containers add more than the earlier ones.
fn assert_linear(totals: &[u64], what: &str) {
    let [small, middle, large] = totals[..] else { unreachable!() };
    let [small_size, middle_size, large_size] = SIZES.map(|size| size as u64);
    assert!(small > 0, "{what}: the counter observed nothing");
    assert_eq!(
        (middle - small) * (large_size - middle_size),
        (large - middle) * (middle_size - small_size),
        "{what}: {small} → {middle} → {large} at {SIZES:?} containers",
    );
}

#[test]
fn a_looping_container_copies_the_same_state_beside_10_and_100_siblings() {
    let mut copied = Vec::new();
    for containers in SIZES {
        let fixture = siblings(containers, looping);
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let before = snapshot();
        let outcome = fixture.execute(&wiring);
        let work = snapshot().since(&before);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.commands().len(), 2 * containers, "every container ran two passes");
        assert_eq!(runtime.snapshot().mutations.get("n"), Some(&json!(containers)), "one increment per container");
        copied.push(work.get(Work::ContainerStateNodes));
    }
    assert_linear(&copied, "container state nodes for a templated cap and a staged action");
}

/// The cap reads a value an earlier task wrote, and the condition reads both
/// the action's write and a lifecycle `set` of the pass: the shared view still
/// carries every runtime write.
#[test]
fn a_container_loop_reads_runtime_writes_through_its_view() {
    let dir = TempDir::new().unwrap();
    let step = json!({ "name": "outer", "sequence": [
        { "name": "raise", "side_effect": { "set": { "cap": 4 } } },
        {
            "name": "inner",
            "loop": { "max": "{{ cap }}", "until": "seen == 'go' && n >= 3", "action": "increment(n)" },
            "success": [{ "set": { "seen": "{{ n >= 2 ? 'go' : 'wait' }}" } }],
            "sequence": [{ "name": "work", "shell": "echo pass" }],
        },
    ] });
    let source = write_source(
        dir.path(),
        "seq.md",
        &[("cap", json!(1)), ("n", json!(0)), ("seen", json!("no")), ("sequence", json!([step]))],
        "Body.\n",
    );
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let runtime = Arc::new(RuntimeState::new());
    let mut wiring = Wiring::new(&recorder, &shell);
    wiring.runtime = Some(&runtime);

    let outcome = fixture.execute(&wiring);

    // With the authored cap of 1 the loop would be exhausted after one pass.
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(shell.commands().len(), 4, "passes 1-4: n is 0, 1, 2, 3 at each pass's end");
    let mutations = runtime.snapshot().mutations;
    assert_eq!(mutations.get("n"), Some(&json!(3)));
    assert_eq!(mutations.get("seen"), Some(&json!("go")));
}

#[test]
fn a_speaking_container_start_copies_the_same_state_beside_10_and_100_siblings() {
    let mut copied = Vec::new();
    for containers in SIZES {
        let fixture = siblings(containers, speaking);
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::start_with(Arc::new(fake.clone()));
        let document = speaking_document(fixture._dir.path(), &worker);
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.document = Some(&document);

        let before = snapshot();
        let outcome = fixture.execute(&wiring);
        let work = snapshot().since(&before);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.commands().len(), containers);
        // Every container submits its phrase at `initialize` and at `start`;
        // each submission resolves it once, and de-duplication queues the one
        // text once.
        assert_eq!(work.get(Work::PrewarmStateBuild), 2 * containers as u64, "two early resolutions per container");
        copied.push(work.get(Work::PrewarmStateNodes));
        let prepared = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap()
            .block_on(async { tokio::time::timeout(Duration::from_secs(5), fake.wait_for_calls(1)).await });
        assert!(prepared.is_ok(), "the queued phrase was never prepared");
        drop(worker);
        assert_eq!(fake.texts(), ["Release starts"], "the `fm` phrase is early, with its resolved text");
    }
    assert_linear(&copied, "pre-warm state nodes for a container's start phrase");
}
