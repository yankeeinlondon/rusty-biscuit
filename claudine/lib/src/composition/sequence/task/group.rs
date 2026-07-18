//! Group scheduling — the only place in a sequence where work runs concurrently.
//!
//! A group is a bundle of tasks executed under one sequence step. `execution:`
//! picks the scheduler; everything else about a group is shared between the two.
//!
//! Common to both (spec → *Groups*):
//!
//! - **`group.*` is a scope, not state.** Group variables are evaluated once at
//!   group start, enter as a lexical scope for the duration of the group, and are
//!   gone the moment it finishes; the next sequence step cannot see them.
//! - **First failure decides the group, `fail_fast` decides the sequence.** A
//!   group has no fail-fast control of its own.
//!
//! ## `execution: serial` (the default)
//!
//! Member tasks run one at a time against the *same* runtime cell as the rest of
//! the sequence, so task 2 sees task 1's `set` writes and reads task 1's entry
//! through `{{ last(outputs) }}` — exactly as if the two had been sequence steps.
//! Each member commits its own entry, so a serial group grows `outputs` entry by
//! entry rather than adding a wrapper entry. The first failure stops the group
//! and the remaining tasks do not run.
//!
//! ## `execution: parallel`
//!
//! Members run on scoped threads, admitted in declaration order and bounded by
//! `max_parallel` when set. Three rules make the observable result independent of
//! completion order (spec → *Concurrency*):
//!
//! - **Snapshot isolation.** State and `outputs` are captured once at group start
//!   and shared by every member; each member's own writes go to a private buffer.
//!   No live file is re-read between siblings.
//! - **All siblings finish.** A failure never cancels in-flight work — discarding
//!   a half-finished agent run costs more than letting it land — and every member
//!   keeps an `outputs` slot holding whatever stdout it produced.
//! - **Declaration-ordered folding.** The nested `outputs` entry, the mutation
//!   merge (later-declared wins, with a warning on a contested key), and the
//!   summary all walk the tasks in declaration order, never completion order.
//!
//! An interrupted parallel group commits neither mutations nor outputs: the run
//! is being torn down, and a half-scheduled group's state is not a state any
//! later step should read.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use darkmatter::markdown::compose::{EffectiveState, EffectiveStateBuilder};
use serde_json::{Map, Value};

use super::super::super::error::CompositionError;
use super::super::super::lifecycle::context::LifecycleErrorInfo;
use super::super::super::lifecycle::executor::StackExecutionContext;
use super::super::super::runtime_state::{OUTPUTS_KEY, RuntimeSnapshot, RuntimeState};
use super::super::preflight::{GroupExecution, PreflightGroup};
use super::{PrimaryOutcome, TaskDiagnostic, TaskExecution, TaskOutcome, TaskStage, TaskStatus};

/// A poisoned result slot means a member task panicked; the group's result is
/// incomplete and cannot be reported honestly.
const SLOT_POISONED: &str = "group result slot poisoned by a panicking task";

/// One finished member, with the wall clock the scheduler measured around it.
struct MemberRun {
    outcome: TaskOutcome,
    duration: Duration,
}

/// The `group` global's name at the frontmatter root and in the lifecycle
/// injected-globals layer.
pub(super) const GROUP_SCOPE_KEY: &str = "group";

/// One member task's contribution to the owning step's summary.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupTaskResult {
    /// The task's authored `name:`, or its generated label.
    pub name: String,
    /// How the task finished.
    pub status: TaskStatus,
    /// Wall-clock time across the task's three stages.
    pub duration: Duration,
}

impl TaskExecution<'_> {
    /// Schedule a group's tasks and report the group as one primary outcome.
    pub(super) fn run_group(&self, group: &PreflightGroup) -> PrimaryOutcome {
        let variables = match self.resolve_group_variables(group) {
            Ok(variables) => variables,
            Err(error) => {
                return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                    &error,
                ));
            }
        };

        let scope_overlay = self.overlay_with_group(&variables);
        let scope_stack = self.stack.with_group(&variables);

        match group.execution {
            GroupExecution::Serial => self.run_serial(group, &variables, &scope_overlay, &scope_stack),
            GroupExecution::Parallel => {
                self.run_parallel(group, &variables, &scope_overlay, &scope_stack)
            }
        }
    }

    /// Declaration-ordered execution against the sequence's own live layers.
    fn run_serial(
        &self,
        group: &PreflightGroup,
        variables: &Map<String, Value>,
        scope_overlay: &Value,
        scope_stack: &StackExecutionContext<'_>,
    ) -> PrimaryOutcome {
        let mut results = Vec::with_capacity(group.tasks.len());
        let mut collected: Vec<String> = Vec::with_capacity(group.tasks.len());
        let mut failure: Option<TaskDiagnostic> = None;
        let mut secondary: Vec<TaskDiagnostic> = Vec::new();
        let mut interrupted = false;

        for task in &group.tasks {
            let started = Instant::now();
            // Rebuilt per task, not once per group: a serial member reads the
            // `set` writes and the `outputs` entry its predecessor produced.
            let member_state = match self.member_state(variables) {
                Ok(state) => state,
                Err(error) => {
                    failure = Some(TaskDiagnostic::from_composition(TaskStage::Primary, &error));
                    break;
                }
            };
            let member = TaskExecution {
                task,
                state: &member_state,
                stack: scope_stack,
                overlay: Some(scope_overlay),
                ..*self
            };
            let outcome = member.run();
            results.push(GroupTaskResult {
                name: task.name.clone().unwrap_or_else(|| task.label.clone()),
                status: outcome.status,
                duration: started.elapsed(),
            });
            collected.push(outcome.stdout);
            secondary.extend(outcome.secondary_errors);

            match outcome.status {
                TaskStatus::Succeeded => {}
                TaskStatus::Failed => {
                    failure = outcome.error;
                    break;
                }
                TaskStatus::Interrupted => {
                    interrupted = true;
                    break;
                }
            }
        }

        // The group's own text is informational: each member already committed
        // its entry, so nothing here reaches `outputs`.
        let stdout = collected.join("\n");
        let outcome = match (interrupted, failure) {
            (true, _) => PrimaryOutcome::interrupted(stdout),
            (false, Some(diagnostic)) => PrimaryOutcome::failed_with(stdout, diagnostic),
            (false, None) => PrimaryOutcome::succeeded(stdout),
        };
        outcome.with_group_tasks(results).with_secondary(secondary)
    }

    /// Bounded-concurrency execution with snapshot isolation.
    ///
    /// Three things make the result independent of completion order (spec →
    /// *Concurrency*): every member composes against **one** state built at group
    /// start, every member writes into its **own** runtime buffer, and every
    /// post-run fold — outputs, mutations, summaries — walks the tasks in
    /// declaration order.
    fn run_parallel(
        &self,
        group: &PreflightGroup,
        variables: &Map<String, Value>,
        scope_overlay: &Value,
        scope_stack: &StackExecutionContext<'_>,
    ) -> PrimaryOutcome {
        // Taken once. A sibling's `set` or `outputs` entry is invisible until the
        // merge below, and no live file is re-read between siblings.
        let base = self
            .runtime
            .map_or_else(RuntimeSnapshot::default, |runtime| runtime.snapshot());
        let member_state = match self.member_state_from(variables, &base) {
            Ok(state) => state,
            Err(error) => {
                return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                    &error,
                ));
            }
        };

        let buffers: Vec<Arc<RuntimeState>> = group
            .tasks
            .iter()
            .map(|_| Arc::new(RuntimeState::from_snapshot(base.clone())))
            .collect();
        let slots: Vec<Mutex<Option<MemberRun>>> =
            group.tasks.iter().map(|_| Mutex::new(None)).collect();

        // Absent cap means "launch all"; declaration-order admission falls out of
        // a single shared cursor, so a freed slot always takes the next task.
        let workers = group
            .max_parallel
            .unwrap_or(group.tasks.len())
            .clamp(1, group.tasks.len().max(1));
        let cursor = AtomicUsize::new(0);

        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| {
                    loop {
                        let index = cursor.fetch_add(1, Ordering::SeqCst);
                        let Some(task) = group.tasks.get(index) else {
                            break;
                        };
                        let started = Instant::now();
                        // The stack's cell must be the member's buffer too: a
                        // lifecycle `set` inside the task accumulates through the
                        // stack, not through `TaskExecution::runtime`.
                        let member_stack = scope_stack.with_runtime_state(&buffers[index]);
                        let member = TaskExecution {
                            task,
                            state: &member_state,
                            stack: &member_stack,
                            overlay: Some(scope_overlay),
                            runtime: Some(&buffers[index]),
                            ..*self
                        };
                        // A sibling's failure never cancels in-flight work:
                        // discarding a half-finished agent run costs more than
                        // letting it land in its slot.
                        let outcome = member.run();
                        *slots[index].lock().expect(SLOT_POISONED) = Some(MemberRun {
                            duration: started.elapsed(),
                            outcome,
                        });
                    }
                });
            }
        });

        self.fold_parallel_results(group, slots)
    }

    /// Reduce the finished members into one group outcome, in declaration order.
    fn fold_parallel_results(
        &self,
        group: &PreflightGroup,
        slots: Vec<Mutex<Option<MemberRun>>>,
    ) -> PrimaryOutcome {
        let runs: Vec<MemberRun> = slots
            .into_iter()
            .filter_map(|slot| slot.into_inner().expect(SLOT_POISONED))
            .collect();

        let mut results = Vec::with_capacity(runs.len());
        let mut entries: Vec<Value> = Vec::with_capacity(runs.len());
        let mut secondary: Vec<TaskDiagnostic> = Vec::new();
        let mut failure: Option<TaskDiagnostic> = None;
        let mut interrupted = false;

        for (task, run) in group.tasks.iter().zip(&runs) {
            let name = task.name.clone().unwrap_or_else(|| task.label.clone());
            results.push(GroupTaskResult {
                name,
                status: run.outcome.status,
                duration: run.duration,
            });
            // Every task keeps a slot, including a failed one — its partial
            // stdout is the only record of what the work managed to produce.
            entries.push(Value::String(run.outcome.stdout.clone()));
            secondary.extend(run.outcome.secondary_errors.clone());
            match run.outcome.status {
                TaskStatus::Succeeded => {}
                TaskStatus::Interrupted => interrupted = true,
                TaskStatus::Failed => {
                    if failure.is_none() {
                        failure = run.outcome.error.clone();
                    } else {
                        secondary.extend(run.outcome.error.clone());
                    }
                }
            }
        }

        let stdout = entries
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("\n");

        // An interrupted group commits nothing: the run is being torn down, and a
        // half-scheduled group's state is not a state any later step should read.
        let outcome = if interrupted {
            PrimaryOutcome::interrupted(stdout)
        } else {
            secondary.extend(self.merge_parallel_mutations(group, &runs));
            let outcome = match failure {
                Some(diagnostic) => PrimaryOutcome::failed_with(stdout, diagnostic),
                None => PrimaryOutcome::succeeded(stdout),
            };
            outcome.with_group_output(Value::Array(entries))
        };
        outcome.with_group_tasks(results).with_secondary(secondary)
    }

    /// Fold every member's mutation delta into the sequence cell.
    ///
    /// Declaration order, not completion order, so the surviving value for a
    /// contested key is the same on every run. The expected case is disjoint
    /// keys; a collision is legal but warned about, because "both tasks wrote
    /// `count`" is nearly always an authoring mistake.
    ///
    /// ## Returns
    ///
    /// Diagnostics for writes the cell refused. A member accepted each of these
    /// keys against the same reserved-key policy, so a refusal here means the two
    /// checks disagree — reportable, but never the reason the group failed.
    fn merge_parallel_mutations(
        &self,
        group: &PreflightGroup,
        runs: &[MemberRun],
    ) -> Vec<TaskDiagnostic> {
        let mut refused = Vec::new();
        let Some(runtime) = self.runtime else {
            return refused;
        };
        let mut written: HashMap<&str, String> = HashMap::new();
        for (task, run) in group.tasks.iter().zip(runs) {
            let name = task.name.clone().unwrap_or_else(|| task.label.clone());
            for mutation in &run.outcome.mutations {
                if let Some(earlier) = written.get(mutation.key.as_str()) {
                    self.stack.emitter.emit_warn(
                        &format!(
                            "group `{}`: `{}` and `{name}` both wrote `{}`; \
                             the later-declared task wins",
                            group.name, earlier, mutation.key
                        ),
                        self.stack.term,
                    );
                }
                written.insert(mutation.key.as_str(), name.clone());
                if let Err(error) = runtime.set(
                    self.stack.effect_engine,
                    &mutation.key,
                    mutation.value.clone(),
                    &Map::new(),
                ) {
                    refused.push(TaskDiagnostic {
                        stage: TaskStage::Primary,
                        info: LifecycleErrorInfo::from_error_or_action("set", &error),
                    });
                }
            }
        }
        refused
    }

    /// Evaluate the group's `variables:` against the state the group starts in.
    ///
    /// Interpolation happens once, at group start, so every member task reads
    /// the same values regardless of what a sibling mutates.
    fn resolve_group_variables(
        &self,
        group: &PreflightGroup,
    ) -> Result<Map<String, Value>, CompositionError> {
        let mut resolved = Map::new();
        for (key, value) in &group.variables {
            resolved.insert(
                key.clone(),
                self.resolve_value(value, &format!("group `{}` variables.{key}", group.name))?,
            );
        }
        Ok(resolved)
    }

    /// The effective state one member task resolves `params`/`timeout`/action
    /// stacks against.
    ///
    /// Layered lowest-first, matching
    /// [`layered_set_overrides`][super::super::super::runtime_state::layered_set_overrides]:
    /// the step's own state, the mutations accumulated so far, `outputs` as it
    /// stands, the reserved overlay, and the group scope.
    fn member_state(
        &self,
        variables: &Map<String, Value>,
    ) -> Result<EffectiveState, CompositionError> {
        let snapshot = self
            .runtime
            .map_or_else(RuntimeSnapshot::default, |runtime| runtime.snapshot());
        self.member_state_from(variables, &snapshot)
    }

    /// [`Self::member_state`] against a caller-supplied snapshot.
    ///
    /// A serial member passes the *live* snapshot so it reads its predecessor's
    /// writes; a parallel group passes the group-start snapshot once and shares
    /// the result with every sibling.
    fn member_state_from(
        &self,
        variables: &Map<String, Value>,
        snapshot: &RuntimeSnapshot,
    ) -> Result<EffectiveState, CompositionError> {
        let mut data = self.state.data().clone();
        if self.runtime.is_some() {
            for (key, value) in &snapshot.mutations {
                data.insert(key.clone(), value.clone());
            }
            data.insert(OUTPUTS_KEY.to_string(), snapshot.outputs_value());
        }
        if let Some(Value::Object(overlay)) = self.overlay {
            for (key, value) in overlay {
                data.insert(key.clone(), value.clone());
            }
        }
        data.insert(
            GROUP_SCOPE_KEY.to_string(),
            Value::Object(variables.clone()),
        );
        EffectiveStateBuilder::new()
            .with_frontmatter(data)
            .with_context(self.state.context().clone())
            .with_allow_ctx_override(true)
            .build()
            .map_err(|source| CompositionError::PreFlightStateBuildFailed { source })
    }

    /// The reserved overlay a member task composes a prompt document under.
    ///
    /// `group` rides the overlay because that is the layer rebuilt per task —
    /// which is precisely what makes the scope end with the group.
    fn overlay_with_group(&self, variables: &Map<String, Value>) -> Value {
        let mut overlay = match self.overlay {
            Some(Value::Object(map)) => map.clone(),
            _ => Map::new(),
        };
        overlay.insert(
            GROUP_SCOPE_KEY.to_string(),
            Value::Object(variables.clone()),
        );
        Value::Object(overlay)
    }
}
