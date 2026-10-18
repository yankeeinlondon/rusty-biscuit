//! The loop protocol a looping container runs after each pass.
//!
//! A sequence or group settles a pass through its own `ContainerRunner`
//! (`start`, children, terminal event, `finalize`). When the pass succeeded,
//! [`finish_container_pass`] ends the loop if a `break` is pending, else runs
//! the gate, reads the condition, classifies the cap, and stages the action
//! into the container's runtime `set` layer, in the order a document loop
//! uses. Pass results are the container's own:
//! the caller reports them to the frame before the gate.

use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::super::lifecycle::LifecycleRunGuard;
use super::super::lifecycle_executor::StackExecutionContext;
use super::super::runtime_state::{RuntimeSnapshot, RuntimeState};
use super::engine::{LoopGateOutcome, run_loop_gate};
use super::expression::LoopExpressionLookup;
use super::frame::{LoopDecision, LoopFrame};

/// What a looping container does after a successful pass.
#[derive(Debug)]
pub enum ContainerPassEnd {
    /// The condition no longer asks for a pass: publish the result.
    Complete,
    /// The action was staged and the frame advanced: enter `start` again.
    Continue,
    /// A `break` ended the loop (in the pass or in its gate): publish the
    /// pass's partial result without reading the condition.
    Broken(super::frame::LoopBreak),
}

/// Run the gate, condition, cap, and action after a container pass.
///
/// `gate` is the owner's `loop` event context: its `frontmatter` is the
/// container's current state (base values under runtime mutations), and its
/// `loop_values` are `frame`'s values after the pass was recorded. The staged
/// action writes only the keys it changed into `runtime`, so every child of
/// the next pass reads them as ordinary frontmatter.
///
/// ## Errors
///
/// The gate's failure, the condition's located failure, the typed
/// exhaustion ([`CompositionError::LoopLimitExceeded`]), or the action's
/// located failure. None of them commits anything, and none re-enters an
/// event of the pass that already finished.
pub fn finish_container_pass(
    frame: &mut LoopFrame,
    guard: &mut LifecycleRunGuard<'_>,
    gate: &StackExecutionContext<'_>,
    runtime: &RuntimeState,
) -> Result<ContainerPassEnd, CompositionError> {
    let ambient = gate
        .loop_values
        .expect("a container loop gate carries its loop values");
    if let Some(broken) = frame.pending_break() {
        return Ok(ContainerPassEnd::Broken(broken));
    }
    if let LoopGateOutcome::Fail(error) = run_loop_gate(guard, gate, gate.source_path, frame.count()) {
        return Err(error);
    }
    if let Some(broken) = frame.pending_break() {
        return Ok(ContainerPassEnd::Broken(broken));
    }
    let lookup = LoopExpressionLookup::new(
        gate.frontmatter,
        ambient,
        gate.file_resolution_context,
        gate.source_path,
    )
    .with_file_ref_fallback_dir(gate.ctx_base_dir)
    .with_prepared_context(gate.prepared_context)
    .with_runtime_state(Some(runtime));
    match frame.decide(&lookup)? {
        LoopDecision::Complete => Ok(ContainerPassEnd::Complete),
        LoopDecision::Exhausted(error) => Err(error),
        LoopDecision::Continue => {
            let staged = frame.stage_actions(gate.frontmatter, &lookup)?;
            let changed: IndexMap<String, Value> = staged
                .into_iter()
                .filter(|(key, value)| gate.frontmatter.get(key) != Some(value))
                .collect();
            if !changed.is_empty() {
                runtime
                    .set_batch(gate.effect_engine, &changed, gate.frontmatter)
                    .map_err(|source| CompositionError::LoopActionWriteRefused {
                        source_path: gate.source_path.to_path_buf(),
                        property: frame.action_property(),
                        iteration: frame.count(),
                        source,
                    })?;
            }
            frame.advance();
            Ok(ContainerPassEnd::Continue)
        }
    }
}

/// The runtime state a looping container's pass starts from.
///
/// A retry of the pass restores exactly this: the mutation layer as the pass
/// entered it and the output entries of earlier passes. Lexical position and
/// active handoffs are not part of a pass and stay as they are.
#[derive(Debug, Clone)]
pub struct PassCheckpoint {
    mutations: darkmatter::markdown::FrontmatterMap,
    outputs: usize,
}

impl PassCheckpoint {
    /// Record the pass entry state of `runtime`.
    pub fn take(runtime: &RuntimeState) -> Self {
        let RuntimeSnapshot { mutations, outputs, .. } = runtime.snapshot();
        Self { mutations, outputs: outputs.len() }
    }

    /// How many output entries earlier passes committed.
    pub fn outputs(&self) -> usize {
        self.outputs
    }

    /// Return `runtime` to this pass's entry state.
    pub fn restore(&self, runtime: &RuntimeState) {
        runtime.restore_pass(self.mutations.clone(), self.outputs);
    }
}

/// The current state of a container: `base` under the runtime mutations.
pub fn container_state(base: &Map<String, Value>, runtime: &RuntimeState) -> Map<String, Value> {
    let mut state = base.clone();
    state.extend(runtime.snapshot().mutations);
    state
}
