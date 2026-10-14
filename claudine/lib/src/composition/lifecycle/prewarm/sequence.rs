//! Collect a sequence's candidates from its static preflight graph.
//!
//! Before step one launches, every prompt document the graph reached and every
//! task's `setup:`/`teardown:` is walked from what preflight already parsed:
//! nothing is composed, no reference is resolved, nothing is fetched, and no
//! setup runs. What preflight cannot know yet is left late:
//!
//! - every key a sequence injects (the per-step overlay, the caller's
//!   setters, and a prompt task's `params`, judged per task, so one document
//!   reached by two tasks is classified against each task's inputs);
//! - a frontmatter value still holding a template or shell expansion, since
//!   resolving it would mean executing it;
//! - `ctx`/`env`, whose document capture does not exist yet.
//!
//! Each document's own preparation submits again later, through the same
//! invocation-wide de-duplication, and finds what preflight had to skip.

use std::collections::BTreeSet;
use std::path::Path;

use darkmatter::markdown::compose::ComposeContext;
use serde_json::{Map, Value};

use super::super::bindings::LifecycleScope;
use super::super::{LifecycleSignal, parse_lifecycle_config, parse_task_action_stack};
use super::{DocumentContext, Phrase, PrewarmHandle, PrewarmInputs, lifecycle_phrases, stack_phrases};
use crate::composition::sequence::reserved::ROOT_OVERLAY_KEYS;
use crate::composition::{PreflightAction, PreflightGraph, PreflightTask, ResolvedCompositionSource};
use crate::events::GlobalSettings;

/// Submit every early phrase `graph` statically reaches.
///
/// `sequence` is the sequence document; its own lifecycle is walked when a
/// step runs its body. `caller_keys` are the top-level keys the caller set
/// for every step. `context` only satisfies the evaluator: no phrase reading
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
        settings,
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
    settings: &'a GlobalSettings,
    graph: &'a PreflightGraph,
    /// The sequence document's authored frontmatter, which a task's
    /// `setup:`/`teardown:` reads.
    sequence: Map<String, Value>,
    injected: BTreeSet<String>,
    context: &'a ComposeContext,
}

impl Walk<'_> {
    fn task(&self, task: &PreflightTask) -> usize {
        let mut queued = 0;
        for (raw, signal, scope, property) in [
            (&task.setup, LifecycleSignal::Start, LifecycleScope::TaskSetup, "setup"),
            (&task.teardown, LifecycleSignal::Finalize, LifecycleScope::TaskTeardown, "teardown"),
        ] {
            let Some(raw) = raw.as_ref().filter(|raw| !raw.is_null()) else {
                continue;
            };
            // A stack that does not parse is reported by the task at its turn.
            let Ok(nodes) = parse_task_action_stack(signal, raw, &task.origin_path, property) else {
                continue;
            };
            queued += self.submit(stack_phrases(scope, &nodes), &self.sequence, &self.injected);
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
            PreflightAction::Shell { .. } | PreflightAction::SideEffect { .. } => {}
        }
        queued
    }

    fn document(&self, frontmatter: &Map<String, Value>, path: &Path, injected: &BTreeSet<String>) -> usize {
        // A lifecycle that does not parse is reported when its document runs.
        match parse_lifecycle_config(&Value::Object(frontmatter.clone()), path) {
            Ok(lifecycle) => self.submit(lifecycle_phrases(&lifecycle), frontmatter, injected),
            Err(_) => 0,
        }
    }

    fn submit(&self, phrases: Vec<Phrase<'_>>, frontmatter: &Map<String, Value>, injected: &BTreeSet<String>) -> usize {
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
        self.handle.submit(self.settings, phrases, &inputs)
    }
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
