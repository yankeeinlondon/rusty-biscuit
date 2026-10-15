//! Collect a sequence's candidates from its static preflight graph.
//!
//! Before step one launches, every prompt document the graph reached and every
//! retained internal task stack is walked from what preflight already parsed:
//! nothing is composed, no reference is resolved, nothing is fetched, and no
//! action runs. What preflight cannot know yet is left late:
//!
//! - every key a sequence injects (the per-step overlay, the caller's
//!   setters, and a prompt task's `params`, judged per task, so one document
//!   reached by two tasks is classified against each task's inputs);
//! - a frontmatter value still holding a template or shell expansion, since
//!   resolving it would mean executing it;
//! - `ctx`/`env`, whose document capture does not exist yet.
//!
//! Each document's own preparation submits again later. [`submit_task_stacks`]
//! reconsiders internal stacks when the task's context is established, using
//! the same invocation-wide
//! de-duplication, finding what preflight had to skip.

use std::collections::BTreeSet;
use std::path::Path;

use darkmatter::markdown::compose::ComposeContext;
use serde_json::{Map, Value};

use super::super::bindings::LifecycleScope;
use super::super::actions::LifecycleNode;
use super::super::{LifecycleSignal, parse_lifecycle_config, parse_task_action_stack};
use super::{DocumentContext, Phrase, PrewarmHandle, PrewarmInputs, lifecycle_phrases, stack_phrases};
use crate::composition::sequence::reserved::ROOT_OVERLAY_KEYS;
use crate::composition::sequence::task::task_playback_settings;
use crate::composition::{PreflightAction, PreflightGraph, PreflightTask, ResolvedCompositionSource};
use crate::events::GlobalSettings;

/// Submit every early phrase `graph` statically reaches.
///
/// `settings` are the invocation's bridged user settings: they decide the
/// pre-warm gates and are what prompt document lifecycles play with. Retained
/// internal task stacks use [`task_playback_settings`] for both preparation
/// and playback. `sequence` supplies their authored frontmatter. `caller_keys`
/// are the top-level keys the caller set for every step. `context` only satisfies the evaluator: no phrase reading
/// `ctx` or `env` is resolved here.
///
/// Returns how many new candidates were queued.
pub fn submit_sequence_preflight(
    handle: &PrewarmHandle,
    settings: &GlobalSettings,
    graph: &PreflightGraph,
    sequence: &ResolvedCompositionSource,
    caller_keys: impl IntoIterator<Item = String>,
    context: &ComposeContext,
) -> usize {
    if !handle.is_enabled() {
        return 0;
    }
    let walk = Walk {
        handle,
        user: settings,
        task_playback: task_playback_settings(),
        graph,
        sequence: authored_frontmatter(sequence),
        injected: ROOT_OVERLAY_KEYS
            .iter()
            .map(|key| (*key).to_string())
            .chain(caller_keys)
            .collect(),
        context,
    };
    let mut queued = 0;
    if graph.steps.iter().any(|step| step.task.is_none()) {
        queued += walk.document(&walk.sequence, &sequence.resolved_path, &walk.injected);
    }
    for task in graph.steps.iter().filter_map(|step| step.task.as_ref()) {
        queued += walk.task(task);
    }
    queued
}

struct Walk<'a> {
    handle: &'a PrewarmHandle,
    /// The user's settings: the gates, and prompt document playback.
    user: &'a GlobalSettings,
    /// Playback settings for retained internal task stacks.
    task_playback: GlobalSettings,
    graph: &'a PreflightGraph,
    /// The sequence document's authored frontmatter for internal task stacks.
    sequence: Map<String, Value>,
    injected: BTreeSet<String>,
    context: &'a ComposeContext,
}

impl Walk<'_> {
    fn task(&self, task: &PreflightTask) -> usize {
        let mut queued = 0;
        for (scope, nodes) in parsed_task_stacks(task) {
            queued += self.submit(
                &self.task_playback,
                stack_phrases(scope, &nodes),
                &self.sequence,
                &self.injected,
            );
        }
        match &task.action {
            PreflightAction::Prompt { path, .. } => {
                if let Some(document) = self.graph.prompt_document(path) {
                    let mut injected = self.injected.clone();
                    injected.extend(task.params.keys().cloned());
                    let frontmatter = document
                        .markdown
                        .frontmatter()
                        .as_map()
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect();
                    queued += self.document(&frontmatter, &document.path, &injected);
                }
            }
            PreflightAction::Group(group) => {
                queued += group.tasks.iter().map(|member| self.task(member)).sum::<usize>();
            }
            PreflightAction::Sequence(sequence) => {
                queued += sequence.steps.iter().filter_map(|step| step.task.as_ref())
                    .map(|child| self.task(child)).sum::<usize>();
            }
            PreflightAction::Shell { .. } | PreflightAction::SideEffect { .. } => {}
        }
        queued
    }

    fn document(&self, frontmatter: &Map<String, Value>, path: &Path, injected: &BTreeSet<String>) -> usize {
        // A lifecycle that does not parse is reported when its document runs.
        match parse_lifecycle_config(&Value::Object(frontmatter.clone()), path) {
            Ok(lifecycle) => self.submit(self.user, lifecycle_phrases(&lifecycle), frontmatter, injected),
            Err(_) => 0,
        }
    }

    /// Submit `phrases`, to be prepared with what their path plays with.
    fn submit(
        &self,
        playback: &GlobalSettings,
        phrases: Vec<Phrase<'_>>,
        frontmatter: &Map<String, Value>,
        injected: &BTreeSet<String>,
    ) -> usize {
        if phrases.is_empty() {
            return 0;
        }
        let mut late = injected.clone();
        let resolved = frontmatter
            .iter()
            .filter(|(key, value)| {
                let unresolved = holds_expansion(value);
                if unresolved {
                    late.insert((*key).clone());
                }
                !unresolved
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let inputs = PrewarmInputs {
            frontmatter: resolved,
            context: DocumentContext::Pending(self.context.clone()),
            sequence_injected_keys: late,
        };
        self.handle.submit(self.user, playback, phrases, &inputs)
    }
}

/// Submit early phrases from internal task stacks, now that
/// the task's inputs exist.
///
/// Called wherever a task's context is established: a top-level task, a
/// serial group member, and a parallel group member. Preflight had to skip a
/// phrase reading `ctx`, `env`, or a frontmatter value it could only get by
/// evaluating a template; here `frontmatter` is the effective frontmatter the
/// task's stacks are evaluated against and `context` the `ctx`/`env` binding
/// they are spoken with, so those phrases resolve. Group members and prompt
/// documents are not walked: each member submits when its own context is
/// established, and a prompt document when it is prepared.
///
/// `user` are the invocation's bridged user settings (the gates only); every
/// candidate is prepared with [`task_playback_settings`]. The sequence's
/// per-step overlay keys stay late, as do the keys `handle` was scoped with
/// (the step's setters and the runtime writes it composed with). A phrase
/// preflight already queued is not queued again.
///
/// Returns how many new candidates were queued.
pub fn submit_task_stacks(
    handle: &PrewarmHandle,
    user: &GlobalSettings,
    task: &PreflightTask,
    frontmatter: &Map<String, Value>,
    context: &ComposeContext,
) -> usize {
    if !handle.is_enabled() {
        return 0;
    }
    let stacks = parsed_task_stacks(task);
    let phrases: Vec<Phrase<'_>> = stacks
        .iter()
        .flat_map(|(scope, nodes)| stack_phrases(*scope, nodes))
        .collect();
    if phrases.is_empty() {
        return 0;
    }
    let inputs = PrewarmInputs {
        frontmatter: frontmatter.clone(),
        context: DocumentContext::Captured(context.clone()),
        sequence_injected_keys: ROOT_OVERLAY_KEYS.iter().map(|key| (*key).to_string()).collect(),
    };
    handle.submit(user, &task_playback_settings(), phrases, &inputs)
}

/// A task's own `setup:` and `teardown:` stacks, parsed as the task parses
/// them, each with the scope it is spoken in. A stack that does not parse is
/// left out: the task reports it at its turn.
fn parsed_task_stacks(task: &PreflightTask) -> Vec<(LifecycleScope, Vec<LifecycleNode>)> {
    [
        (&task.setup, LifecycleSignal::Start, LifecycleScope::TaskSetup, "setup"),
        (&task.teardown, LifecycleSignal::Finalize, LifecycleScope::TaskTeardown, "teardown"),
    ]
    .into_iter()
    .filter_map(|(raw, signal, scope, property)| {
        let raw = raw.as_ref().filter(|raw| !raw.is_null())?;
        let nodes = parse_task_action_stack(signal, raw, &task.origin_path, property).ok()?;
        Some((scope, nodes))
    })
    .collect()
}

fn authored_frontmatter(source: &ResolvedCompositionSource) -> Map<String, Value> {
    source
        .markdown
        .frontmatter()
        .as_map()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// Whether an authored value still holds a template or a shell expansion,
/// which only composition may evaluate.
fn holds_expansion(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("{{") || text.contains("$(") || text.contains('`'),
        Value::Array(items) => items.iter().any(holds_expansion),
        Value::Object(map) => map.values().any(holds_expansion),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

#[cfg(test)]
mod tests;
