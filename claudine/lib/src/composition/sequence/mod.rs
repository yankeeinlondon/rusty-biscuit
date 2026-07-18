//! Sequence Plus: detection, normalization, sources, and the reserved overlay.
//!
//! [`resolve_sequence_plan`] detects whether a resolved composition source
//! defines a `sequence`, and if so normalizes it into a typed [`SequencePlan`]
//! of generated [`StepState`]s. [`build_step_overlay`] constructs the reserved
//! per-step overlay (`state`/`previous`/`next`/`sequence_id`/`outputs`) injected
//! into each composition run.
//!
//! The module is split by responsibility:
//!
//! - [`model`] — the typed domain vocabulary (states, sources, overlay, the
//!   executable/task/output/runtime representations later phases consume);
//! - [`normalize`] — strict authored-state normalization, id suffixing, and the
//!   `sequence_id` invocation token;
//! - [`source`] — external file-reference resolution and loading;
//! - [`reserved`] — the canonical reserved-key catalog shared by every layer.
//!
//! See `claudine/features/2026-07-11-sequence-plus/spec.md`.

pub mod model;
pub mod normalize;
pub mod reserved;
pub mod source;

use super::json_util::json_type_name;
use super::types::ResolvedCompositionSource;

pub use model::{
    ExecutableField, ExternalTaskRef, GroupRef, OutputEntry, RuntimeMutation, SequencePlan,
    SequenceSource, SequenceStep, SequenceStepOverlay, StepExecutable, StepState,
};

use super::error::CompositionError;
use model::SequenceSource as Source;

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
    let fm = source.markdown.frontmatter();
    let sequence_value = match fm.as_map().get("sequence") {
        Some(v) => v.clone(),
        None => return Ok(None),
    };

    let fail_fast = match fm.as_map().get("fail_fast") {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(other) => {
            return Err(CompositionError::SequenceInvalid(format!(
                "`fail_fast` must be a boolean, got {}",
                json_type_name(other)
            )));
        }
        None => true,
    };

    match sequence_value {
        serde_json::Value::Array(items) => Ok(Some(normalize::normalize_plan(
            &items,
            Source::Inline,
            &source.resolved_path,
            fail_fast,
        )?)),
        serde_json::Value::String(ref path_str) => {
            let yaml_path = source::resolve_sequence_reference(path_str, &source.resolved_path)?;
            let plan =
                source::load_external_sequence(&yaml_path, &source.resolved_path, fail_fast)?;
            Ok(Some(plan))
        }
        other => Err(CompositionError::SequenceInvalid(format!(
            "expected a list or file path string, got {}",
            json_type_name(&other)
        ))),
    }
}

/// Build the reserved overlay for `step_index` within `plan`.
///
/// `state` is always the current step's [`StepState`]; `previous`/`next` are the
/// neighboring states, or `None` on the boundaries (they render as `null`, never
/// an empty-named state). `outputs` is empty here — the executor appends entries
/// as tasks complete.
pub fn build_step_overlay(plan: &SequencePlan, step_index: usize) -> SequenceStepOverlay {
    let total = plan.steps.len();
    SequenceStepOverlay {
        state: plan.steps[step_index].state.clone(),
        previous: (step_index > 0).then(|| plan.steps[step_index - 1].state.clone()),
        next: (step_index + 1 < total).then(|| plan.steps[step_index + 1].state.clone()),
        sequence_id: plan.sequence_id.clone(),
        outputs: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
