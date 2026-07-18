//! Sequence Plus: detection, normalization, sources, and the reserved overlay.
//!
//! [`resolve_sequence_plan`] detects whether a resolved composition source
//! defines a `sequence`, and if so normalizes it into a typed [`SequencePlan`]
//! of generated [`StepState`]s. [`build_step_overlay`] constructs the reserved
//! per-step overlay (`state`/`previous`/`next`/`sequence_id`) injected
//! into each composition run.
//!
//! The module is split by responsibility:
//!
//! - [`model`] — the typed domain vocabulary (states, sources, overlay, the
//!   executable/task/output/runtime representations later phases consume);
//! - [`grammar`] — source classification and the
//!   `<file-ref> [-> offset] [::op(args)]` suffix parser;
//! - [`data`] — multi-format loading, offset traversal, operator application;
//! - [`expr`] — expression evaluation for dynamic sources and templates;
//! - [`normalize`] — strict/lenient state normalization, id suffixing, and the
//!   `sequence_id` invocation token;
//! - [`preflight`] — the recursive task graph: external task/group/catalog/
//!   prompt loading, cycle detection, and early-binding shell resolution;
//! - [`source`] — file-reference resolution and referenced-source loading;
//! - [`task`] — atomic task execution: the setup/primary/teardown contract and
//!   the one [`task::TaskOutcome`] every task variant reports;
//! - [`reserved`] — the canonical reserved-key catalog shared by every layer.
//!
//! See `claudine/features/2026-07-11-sequence-plus/spec.md`.

pub mod data;
pub mod expr;
pub mod grammar;
pub mod model;
pub mod normalize;
pub mod preflight;
pub mod reserved;
pub mod source;
pub mod task;

use std::path::Path;

use biscuit_file::classify_list;
use serde_json::{Map, Value};

use super::json_util::json_type_name;
use super::types::ResolvedCompositionSource;

pub use grammar::{SequenceReference, SequenceSourceSpec, SourceOperator};
pub use model::{
    ExecutableField, ExternalTaskRef, GroupRef, OutputEntry, RuntimeMutation, SequencePlan,
    SequenceSource, SequenceStep, SequenceStepOverlay, StepExecutable, StepState, Strictness,
};

use super::error::CompositionError;
use expr::SourceExpressionLookup;
use model::SequenceSource as Source;

/// Expands one approved `$( … )` sequence source to its stdout.
///
/// Injected rather than called directly so the library never owns the approval
/// prompt: the CLI supplies a runner backed by the shared approval cache and
/// the interactive handler. Without one, a `$( … )` source is a typed error
/// rather than a silent unapproved execution.
pub type ShellSourceRunner<'a> = &'a dyn Fn(&str) -> Result<String, CompositionError>;

/// Caller-supplied capabilities for sequence-source resolution.
#[derive(Default, Clone, Copy)]
pub struct SequenceSourceOptions<'a> {
    /// Expander for `$( … )` sources. `None` rejects them.
    pub shell_runner: Option<ShellSourceRunner<'a>>,
}

/// Detect and resolve a sequence plan from a resolved composition source.
///
/// Returns `Ok(None)` when the source has no `sequence` frontmatter key, and
/// `Ok(Some(plan))` for a valid inline or externally-referenced sequence.
///
/// ## Errors
///
/// Returns `Err` for invalid sequence definitions: wrong types, missing `name`
/// on object steps, empty lists, more than one executable field, reserved-key
/// collisions, or external file load failures.
pub fn resolve_sequence_plan(
    source: &ResolvedCompositionSource,
) -> Result<Option<SequencePlan>, CompositionError> {
    resolve_sequence_plan_with(source, SequenceSourceOptions::default())
}

/// [`resolve_sequence_plan`] with caller-supplied capabilities.
///
/// ## Errors
///
/// As [`resolve_sequence_plan`], plus
/// [`CompositionError::SequenceShellFailed`] when a `$( … )` source is present
/// and `options` carries no runner.
pub fn resolve_sequence_plan_with(
    source: &ResolvedCompositionSource,
    options: SequenceSourceOptions<'_>,
) -> Result<Option<SequencePlan>, CompositionError> {
    let fm = source.markdown.frontmatter();
    let sequence_value = match fm.as_map().get("sequence") {
        Some(v) => v.clone(),
        None => return Ok(None),
    };

    let fail_fast = match fm.as_map().get("fail_fast") {
        Some(Value::Bool(b)) => *b,
        Some(other) => {
            return Err(CompositionError::SequenceInvalid(format!(
                "`fail_fast` must be a boolean, got {}",
                json_type_name(other)
            )));
        }
        None => true,
    };

    // The frontmatter's insertion-ordered map is projected into a plain JSON
    // object once here: expression lookups only ever read it by key, and the
    // conversion keeps every downstream signature on one map type.
    let frontmatter: Map<String, Value> = fm
        .as_map()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let plan = match grammar::classify_source(&sequence_value)? {
        SequenceSourceSpec::Inline(items) => normalize::normalize_plan(
            &items,
            Source::Inline,
            &source.resolved_path,
            fail_fast,
        )?,
        SequenceSourceSpec::Expression(expression) => resolve_expression_source(
            &expression,
            &frontmatter,
            &source.resolved_path,
            fail_fast,
        )?,
        SequenceSourceSpec::Shell(command) => {
            resolve_shell_source(&command, &options, &source.resolved_path, fail_fast)?
        }
        SequenceSourceSpec::Reference(reference) => {
            let path =
                source::resolve_sequence_reference(&reference.reference, &source.resolved_path)?;
            source::load_referenced_sequence(
                &reference,
                &path,
                &source.resolved_path,
                &frontmatter,
                fail_fast,
            )?
        }
    };

    Ok(Some(plan))
}

/// Resolve a whole-value `{{ … }}` source.
///
/// A typed array result is already a list of entries and bypasses `ListFormat`
/// classification entirely; any other value is rendered to a string and
/// classified, which is what makes the CSV-by-default `ctx.*` list variables
/// work without asking the author to convert them.
fn resolve_expression_source(
    expression: &str,
    frontmatter: &Map<String, Value>,
    invocation_path: &Path,
    fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    let base_dir = invocation_path.parent().unwrap_or_else(|| Path::new("."));
    let lookup = SourceExpressionLookup::new(frontmatter, base_dir);
    let value = expr::evaluate_whole(expression, &lookup)?;

    let items = match value {
        Value::Array(items) => items,
        Value::Null => Vec::new(),
        Value::String(text) => classify_string_items(&text),
        other => classify_string_items(&other.to_string()),
    };

    normalize::normalize_plan(&items, Source::Expression, invocation_path, fail_fast)
}

/// Resolve a `$( … )` source through the caller's approved-command runner.
fn resolve_shell_source(
    command: &str,
    options: &SequenceSourceOptions<'_>,
    invocation_path: &Path,
    fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    let runner = options
        .shell_runner
        .ok_or_else(|| CompositionError::SequenceShellFailed {
            command: command.to_string(),
            source: super::error::SequenceShellCause::Unavailable,
        })?;

    let output = runner(command)?;
    let items = classify_string_items(&output);
    normalize::normalize_plan(&items, Source::Shell, invocation_path, fail_fast)
}

/// Classify a textual list and turn its entries into item values.
fn classify_string_items(text: &str) -> Vec<Value> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    classify_list(text)
        .1
        .into_iter()
        .map(Value::String)
        .collect()
}

/// Build the reserved overlay for `step_index` within `plan`.
///
/// `state` is always the current step's [`StepState`]; `previous`/`next` are the
/// neighboring states, or `None` on the boundaries (they render as `null`, never
/// an empty-named state). `outputs` is *not* part of this overlay: it belongs to
/// the runtime layer beneath it, so the accumulator survives from step to step.
pub fn build_step_overlay(plan: &SequencePlan, step_index: usize) -> SequenceStepOverlay {
    let total = plan.steps.len();
    SequenceStepOverlay {
        state: plan.steps[step_index].state.clone(),
        previous: (step_index > 0).then(|| plan.steps[step_index - 1].state.clone()),
        next: (step_index + 1 < total).then(|| plan.steps[step_index + 1].state.clone()),
        sequence_id: plan.sequence_id.clone(),
    }
}

#[cfg(test)]
mod tests;
