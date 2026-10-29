//! A group leg's private runtime branch is built when the leg is admitted,
//! always from the group-start seed, and released before its worker admits
//! the next leg: live branches never exceed the active cap, and a completed
//! group builds exactly one branch per admitted leg per attempt.
//!
//! Admission and release boundaries are exposed with [`ProbeShell`], whose
//! commands record the live branch count when they start and can be held
//! until other commands have started. Nothing here sleeps.

use super::*;
use crate::work_counters::{Gauge, Work, WorkSnapshot, live, peak, reset_peak, snapshot};
use std::collections::HashSet;
use std::sync::Condvar;

/// How long a held command waits for the commands it is held on before the
/// test fails; never reached when scheduling is correct.
const HOLD_LIMIT: Duration = Duration::from_secs(4);

/// A shell double for admission tests. Each command records the number of live
/// branches when it starts; a held command waits until the commands it names
/// have started, or until a number of commands have.
#[derive(Default)]
struct ProbeShell {
    /// `(command, live branches when it started)`, in start order.
    seen: Mutex<Vec<(String, u64)>>,
    changed: Condvar,
    /// `command → commands that must start before it returns`.
    holds: HashMap<String, Vec<String>>,
    /// Every command returns only once this many commands have started.
    rendezvous: usize,
    /// Commands that exit 1 the first time they run.
    fail_once: HashSet<String>,
    /// Commands that exit 1 every time.
    failing: HashSet<String>,
    /// Commands that raise the run's interrupt flag and report interruption.
    interrupting: HashSet<String>,
}

impl ProbeShell {
    fn hold(mut self, command: &str, until_started: &[&str]) -> Self {
        self.holds.insert(
            command.into(),
            until_started
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
        );
        self
    }

    fn commands(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(command, _)| command.clone())
            .collect()
    }

    /// The live branch count `command` saw when it (first) started.
    fn live_at(&self, command: &str) -> u64 {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .find(|(seen, _)| seen == command)
            .unwrap_or_else(|| panic!("`{command}` never ran"))
            .1
    }
}

impl TaskShellRunner for ProbeShell {
    fn run_structured(
        &self,
        chain: &darkmatter::markdown::compose::shell_expansion::template::FilledChain,
        timeout: Duration,
        interrupt: Option<&AtomicBool>,
        live: Option<&Arc<TaskLiveOutput>>,
    ) -> Result<ShellCommandOutput, TaskShellError> {
        let segment = &chain.segments[0];
        let command = std::iter::once(segment.program.as_str())
            .chain(segment.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        self.run(&command, timeout, interrupt, live)
    }

    fn run(
        &self,
        command: &str,
        _timeout: Duration,
        interrupt: Option<&AtomicBool>,
        _live: Option<&Arc<TaskLiveOutput>>,
    ) -> Result<ShellCommandOutput, TaskShellError> {
        let mut seen = self.seen.lock().unwrap();
        let first_run = !seen.iter().any(|(earlier, _)| earlier == command);
        seen.push((
            command.to_string(),
            crate::work_counters::live(Gauge::RuntimeStateBranch),
        ));
        self.changed.notify_all();
        let awaited = self.holds.get(command).cloned().unwrap_or_default();
        let (seen, wait) = self
            .changed
            .wait_timeout_while(seen, HOLD_LIMIT, |seen| {
                seen.len() < self.rendezvous
                    || awaited
                        .iter()
                        .any(|name| !seen.iter().any(|(started, _)| started == name))
            })
            .unwrap();
        assert!(
            !wait.timed_out(),
            "`{command}` was held for {awaited:?} / {} starts; saw {seen:?}",
            self.rendezvous
        );
        drop(seen);
        let interrupted = self.interrupting.contains(command);
        if interrupted {
            interrupt
                .expect("the run carries an interrupt flag")
                .store(true, Ordering::SeqCst);
        }
        let failed =
            self.failing.contains(command) || (first_run && self.fail_once.contains(command));
        Ok(ShellCommandOutput {
            stdout: command.to_string(),
            exit_code: if interrupted { 130 } else { i32::from(failed) },
            timed_out: false,
            interrupted,
            runaway: None,
        })
    }
}

/// What one execution did to the branch counters.
struct Run {
    outcome: TaskOutcome,
    work: WorkSnapshot,
    peak: u64,
    runtime: Arc<RuntimeState>,
    recorder: Recorder,
}

impl Run {
    fn built(&self) -> u64 {
        self.work.get(Work::RuntimeStateBranch)
    }
}

/// Run a one-step sequence whose step is `step`, with `frontmatter` beside it.
fn run_with(
    frontmatter: &[(&str, Value)],
    step: Value,
    shell: &ProbeShell,
    interrupt: Option<&AtomicBool>,
) -> Run {
    let dir = TempDir::new().unwrap();
    let mut keys = frontmatter.to_vec();
    keys.push(("sequence", json!([step])));
    let source = write_source(dir.path(), "seq.md", &keys, "Body.\n");
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let runtime = Arc::new(RuntimeState::new());
    let mut wiring = Wiring::new(&recorder, &fake);
    wiring.shell = shell;
    wiring.runtime = Some(&runtime);
    wiring.interrupt = interrupt;
    reset_peak(Gauge::RuntimeStateBranch);
    let before = snapshot();
    let outcome = fixture.execute(&wiring);
    let work = snapshot().since(&before);
    assert_eq!(
        live(Gauge::RuntimeStateBranch),
        0,
        "every branch is released once the group settles"
    );
    Run {
        outcome,
        work,
        peak: peak(Gauge::RuntimeStateBranch),
        runtime,
        recorder,
    }
}

fn shell_legs(names: &[&str]) -> Vec<Value> {
    names
        .iter()
        .map(|name| json!({ "name": name, "shell": name }))
        .collect()
}

/// With a cap of two, the leg admitted when `b` settles runs beside `a`, and
/// only those two branches are alive: `b`'s was released before `c` was
/// admitted. The cap is reached, never exceeded, and every leg built one.
#[test]
fn a_capped_group_keeps_at_most_cap_branches_alive_and_builds_one_per_leg() {
    let names = ["a", "b", "c", "d", "e", "f"];
    let shell = ProbeShell::default().hold("a", &["c"]);
    let run = run_with(
        &[],
        json!({ "name": "step", "group": {
        "name": "capped", "max_parallel": 2, "tasks": shell_legs(&names) } }),
        &shell,
        None,
    );
    assert!(run.outcome.succeeded(), "{}", failure_message(&run.outcome));
    assert_eq!(
        shell.live_at("c"),
        2,
        "`a` and `c` alive, `b` already released: {:?}",
        shell.seen.lock().unwrap()
    );
    assert!(
        shell
            .seen
            .lock()
            .unwrap()
            .iter()
            .all(|(_, live)| *live <= 2),
        "{:?}",
        shell.seen.lock().unwrap()
    );
    assert_eq!(run.peak, 2, "the cap was reached and never exceeded");
    assert_eq!(
        run.built(),
        names.len() as u64,
        "one branch per admitted leg"
    );
    assert_eq!(
        run.runtime.outputs_value(),
        json!([{ "a": "a", "b": "b", "c": "c", "d": "d", "e": "e", "f": "f" }])
    );
}

/// Without a cap every leg is admitted at once, so every branch is alive
/// together, each built once.
#[test]
fn an_uncapped_group_admits_every_leg_and_builds_one_branch_each() {
    let names = ["a", "b", "c", "d"];
    let shell = ProbeShell {
        rendezvous: names.len(),
        ..Default::default()
    };
    let run = run_with(
        &[],
        json!({ "name": "step", "group": {
        "name": "uncapped", "tasks": shell_legs(&names) } }),
        &shell,
        None,
    );
    assert!(run.outcome.succeeded(), "{}", failure_message(&run.outcome));
    assert_eq!(run.peak, names.len() as u64);
    assert_eq!(run.built(), names.len() as u64);
}

/// A leg admitted after its sibling settled still reads the group-start
/// value: its branch comes from the seed, not from the sibling's released
/// branch, and the sibling's write reaches the parent only at the fold.
#[test]
fn a_delayed_leg_reads_the_group_start_seed_after_its_sibling_released_its_branch() {
    let shell = ProbeShell::default();
    let run = run_with(
        &[("marker", json!("initial"))],
        json!({ "name": "step", "group": {
        "name": "isolated", "max_parallel": 1, "tasks": [
            { "name": "writer", "side_effect": { "set": { "marker": "written" } } },
            { "name": "reader", "sequence": [{ "name": "read", "shell": "probe {{ marker }}" }] },
        ] } }),
        &shell,
        None,
    );
    assert!(run.outcome.succeeded(), "{}", failure_message(&run.outcome));
    assert_eq!(
        shell.commands(),
        vec!["probe initial"],
        "the reader sees the group-start value"
    );
    assert_eq!(
        shell.live_at("probe initial"),
        1,
        "the writer's branch was released before the reader was admitted"
    );
    assert_eq!(
        run.runtime.mutations().get("marker"),
        Some(&json!("written")),
        "the write lands at the fold"
    );
    assert_eq!((run.peak, run.built()), (1, 2));
}

/// Each leg of a capped outer group runs a capped inner group: the outer
/// leg's branch and at most the inner cap of inner branches are alive, so the
/// peak is the product of the active caps, not the declared leg count.
#[test]
fn a_nested_group_bounds_live_branches_by_its_active_caps() {
    let inner = |index: usize| {
        let names: Vec<String> = ["a", "b", "c"]
            .iter()
            .map(|leg| format!("i{index}-{leg}"))
            .collect();
        json!({ "name": format!("outer-{index}"), "group": {
            "name": format!("inner-{index}"), "max_parallel": 2,
            "tasks": names.iter().map(|name| json!({ "name": name, "shell": name })).collect::<Vec<_>>() } })
    };
    // Each inner group's first two legs overlap, so the inner cap is reached.
    let shell = (0..3).fold(ProbeShell::default(), |shell, index| {
        shell.hold(&format!("i{index}-a"), &[&format!("i{index}-b")])
    });
    let run = run_with(
        &[],
        json!({ "name": "step", "group": {
        "name": "outer", "max_parallel": 1, "tasks": (0..3).map(inner).collect::<Vec<_>>() } }),
        &shell,
        None,
    );
    assert!(run.outcome.succeeded(), "{}", failure_message(&run.outcome));
    assert_eq!(run.peak, 3, "one outer branch beside two inner ones");
    assert_eq!(
        run.built(),
        3 + 9,
        "one branch per admitted outer and inner leg"
    );
    assert_eq!(shell.commands().len(), 9);
}

/// A retried group rebuilds every leg's branch from the retry attempt's seed:
/// the failed attempt's private writes are gone, so the reader sees the
/// group-start value again, and each attempt builds one branch per leg.
#[test]
fn a_retried_group_rebuilds_its_legs_from_the_seed() {
    let shell = ProbeShell {
        fail_once: HashSet::from(["probe initial".to_string()]),
        ..Default::default()
    };
    let run = run_with(
        &[("marker", json!("initial"))],
        json!({ "name": "step", "group": {
        "name": "retrying", "max_parallel": 1,
        "failure": [{ "retry": { "max_attempts": 1 } }],
        "tasks": [
            { "name": "writer", "side_effect": { "set": { "marker": "written" } } },
            { "name": "reader", "sequence": [{ "name": "read", "shell": "probe {{ marker }}" }] },
        ] } }),
        &shell,
        None,
    );
    assert!(run.outcome.succeeded(), "{}", failure_message(&run.outcome));
    assert_eq!(
        shell.commands(),
        vec!["probe initial", "probe initial"],
        "both attempts read the seed"
    );
    assert_eq!(run.built(), 2 * 2, "two legs per attempt");
    assert_eq!(run.peak, 1);
    assert_eq!(
        run.runtime.mutations().get("marker"),
        Some(&json!("written"))
    );
    assert_eq!(
        run.runtime.outputs_value()[0]["reader"],
        json!(["probe initial"]),
        "one published result, from the retry"
    );
    assert_eq!(run.runtime.output_count(), 1);
}

/// An atomic (`fail_fast`) group stops admitting after a failure: the legs
/// never admitted build no branch and are recorded cancelled without running.
#[test]
fn a_failure_stops_admission_and_unadmitted_legs_build_no_branch() {
    let shell = ProbeShell {
        failing: HashSet::from(["bad".to_string()]),
        ..Default::default()
    };
    let run = run_with(
        &[],
        json!({ "name": "step", "group": {
        "name": "atomic", "max_parallel": 1,
        "failure": [{ "info": "cancelled={{ as_csv(group.cancelled) }}" }],
        "tasks": shell_legs(&["bad", "never-1", "never-2"]) } }),
        &shell,
        None,
    );
    assert_eq!(run.outcome.status, TaskStatus::Failed);
    assert_eq!(shell.commands(), vec!["bad"]);
    assert_eq!((run.built(), run.peak), (1, 1));
    assert_eq!(
        run.recorder.events(),
        vec!["info:cancelled=never-1, never-2"]
    );
    assert_eq!(run.runtime.outputs_value(), json!([]));
}

/// A tolerant group admits every leg after a failure, builds one branch for
/// each, and folds their writes in declaration order whichever settles first.
#[test]
fn a_tolerant_group_folds_every_admitted_leg_in_declaration_order() {
    // `bad` settles last: it is held until `good` has started.
    let shell = ProbeShell {
        failing: HashSet::from(["bad".to_string()]),
        ..Default::default()
    }
    .hold("bad", &["good"]);
    let run = run_with(
        &[],
        json!({ "name": "step", "group": {
        "name": "tolerant", "fail_fast": false, "tasks": [
            { "name": "first", "sequence": [{ "name": "w", "shell": "bad" }], "initialize": [{ "set": { "marker": "first" } }] },
            { "name": "second", "sequence": [{ "name": "w", "shell": "good" }], "initialize": [{ "set": { "marker": "second" } }] },
        ] } }),
        &shell,
        None,
    );
    assert!(run.outcome.succeeded(), "{}", failure_message(&run.outcome));
    assert_eq!(
        run.runtime.mutations().get("marker"),
        Some(&json!("second")),
        "the later-declared leg wins"
    );
    assert_eq!(run.built(), 2);
    assert!(run.peak <= 2);
}

/// An interrupt during the first leg stops admission: no further branch is
/// built, the admitted one is released, and nothing is committed.
#[test]
fn an_interrupt_stops_admission_and_releases_the_admitted_branch() {
    let shell = ProbeShell {
        interrupting: HashSet::from(["a".to_string()]),
        ..Default::default()
    };
    let interrupt = AtomicBool::new(false);
    let run = run_with(
        &[],
        json!({ "name": "step", "group": {
        "name": "interrupted", "max_parallel": 1, "tasks": shell_legs(&["a", "b", "c"]) } }),
        &shell,
        Some(&interrupt),
    );
    assert_eq!(run.outcome.status, TaskStatus::Interrupted);
    assert_eq!(shell.commands(), vec!["a"]);
    assert_eq!((run.built(), run.peak), (1, 1));
    assert_eq!(run.runtime.output_count(), 0);
    assert!(run.runtime.mutations().is_empty());
}
