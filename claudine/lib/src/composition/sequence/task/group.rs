//! Serial group scheduling.
//!
//! A group is a bundle of tasks executed under one sequence step. Phase 9
//! implements the serial mode (the default); `execution: parallel` is scheduled
//! in phase 10 and is a typed refusal until then.
//!
//! Three properties define serial group execution (spec → *Groups*):
//!
//! - **Declaration order, shared live layers.** Member tasks run one at a time
//!   against the *same* runtime cell as the rest of the sequence, so task 2 sees
//!   task 1's `set` writes and reads task 1's entry through
//!   `{{ last(outputs) }}` — exactly as if the two had been sequence steps.
//! - **`group.*` is a scope, not state.** Group variables enter as a lexical
//!   scope for the duration of the group and are gone the moment it finishes;
//!   the next sequence step cannot see them.
//! - **First failure stops the group.** The remaining tasks do not run and the
//!   owning step is failed. Whether the *sequence* continues is decided solely
//!   by sequence-level `fail_fast` — a group has no fail-fast control of its
//!   own.
//!
//! Each member task commits its own `outputs` entry, so the group itself
//! commits none: a serial group grows `outputs` entry by entry rather than
//! adding a wrapper entry.

use std::time::{Duration, Instant};

use darkmatter::markdown::compose::{EffectiveState, EffectiveStateBuilder};
use serde_json::{Map, Value};

use super::super::super::error::CompositionError;
use super::super::super::runtime_state::OUTPUTS_KEY;
use super::super::preflight::{GroupExecution, PreflightGroup};
use super::{PrimaryOutcome, TaskDiagnostic, TaskExecution, TaskStage, TaskStatus};

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
        if group.execution == GroupExecution::Parallel {
            return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                TaskStage::Primary,
                &CompositionError::SequenceTaskUnsupported {
                    task: self.label(),
                    construct: format!("parallel group `{}`", group.name),
                    detail: "concurrent group scheduling is not implemented yet; \
                             remove `execution: parallel` to run the tasks serially"
                        .to_string(),
                },
            ));
        }

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

        let mut results = Vec::with_capacity(group.tasks.len());
        let mut collected: Vec<String> = Vec::with_capacity(group.tasks.len());
        let mut failure: Option<TaskDiagnostic> = None;
        let mut secondary: Vec<TaskDiagnostic> = Vec::new();
        let mut interrupted = false;

        for task in &group.tasks {
            let started = Instant::now();
            // Rebuilt per task, not once per group: a serial member reads the
            // `set` writes and the `outputs` entry its predecessor produced.
            let member_state = match self.member_state(&variables) {
                Ok(state) => state,
                Err(error) => {
                    failure = Some(TaskDiagnostic::from_composition(TaskStage::Primary, &error));
                    break;
                }
            };
            let member = TaskExecution {
                task,
                state: &member_state,
                stack: &scope_stack,
                overlay: Some(&scope_overlay),
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
        let mut data = self.state.data().clone();
        if let Some(runtime) = self.runtime {
            let snapshot = runtime.snapshot();
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
