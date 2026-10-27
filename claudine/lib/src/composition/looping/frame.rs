//! The nearest-loop frame every looping primitive shares.
//!
//! A document, a group, and a sequence repeat by the same rules: the frame is
//! allocated before the owner's `initialize`, a pass runs through its own
//! `start`/terminal/`finalize`, the owner's loop gate fires, and then
//! [`LoopFrame::decide`] reads the condition before it classifies the cap. The
//! staged action ([`LoopFrame::stage_actions`]) applies only when another pass
//! will run. Each primitive keeps its own events, state, and result shape; the
//! frame owns only what the loop decides.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::expression::EvaluationLookup;
use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::super::types::{LoopCondition, LoopConfig};
use super::actions::{ActionStaging, ActionTarget};
use super::engine::DEFAULT_MAX_ITERATIONS;
use super::expression::{LoopAmbient, LoopExpressionLookup, evaluate_condition};

/// The shape of one pass's result, which is also `loop.last_output`'s value
/// before any pass has finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopResultKind {
    /// A document pass yields its output text.
    Document,
    /// A sequence pass yields the array of that pass's child results.
    Sequence,
    /// A group pass yields the dictionary of its leg results.
    Group,
}

impl LoopResultKind {
    /// The result before any pass exists: `""`, `[]`, or `{}`.
    pub fn empty(self) -> Value {
        match self {
            Self::Document => Value::String(String::new()),
            Self::Sequence => Value::Array(Vec::new()),
            Self::Group => Value::Object(Map::new()),
        }
    }
}

/// How a loop ended, when it ended on its own terms.
///
/// Failures, handoffs, cancellation, and timeouts are not loop outcomes: they
/// leave the loop without running its gate or condition and are reported as
/// what they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopOutcome {
    /// `until` became true or `while` became false after a pass.
    ConditionMet,
    /// The final permitted pass still asked for another one: a typed failure
    /// of the loop owner.
    Exhausted,
    /// A `break` ended the loop: a successful outcome.
    Broken,
}

impl LoopOutcome {
    /// The outcome as a summary reports it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConditionMet => "condition met",
            Self::Exhausted => "cap reached",
            Self::Broken => "broken",
        }
    }
}

/// A `break` waiting for its loop to settle: the loop ends after the
/// interrupted pass's cleanup, without its gate, condition, or action.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoopBreak {
    /// The evaluated reason, shown in the loop's summary.
    pub reason: Option<String>,
    /// The process exit code when the loop it ends is the root invocation's.
    pub code: u8,
}

impl LoopBreak {
    /// The summary line for a loop this break ended.
    pub fn summary(&self) -> String {
        match &self.reason {
            Some(reason) => format!("loop broken: {reason} (code {})", self.code),
            None => format!("loop broken (code {})", self.code),
        }
    }
}

/// The channel through which a `break` reaches its nearest loop.
///
/// A loop owner allocates one slot per invocation, and every event, gate,
/// and descendant page whose innermost loop it is records into that same
/// slot. A container between the `break` and its loop sees the slot pending,
/// stops starting children, runs only its `finalize`, and returns; the loop
/// owner then ends with [`LoopOutcome::Broken`]. The first `break` recorded
/// wins. A failure that defeats a pending break discards it.
#[derive(Debug, Clone, Default)]
pub struct LoopBreakSlot(std::sync::Arc<std::sync::Mutex<Option<LoopBreak>>>);

impl LoopBreakSlot {
    /// Record a `break`, unless one is already pending.
    pub fn record(&self, broken: LoopBreak) {
        self.0.lock().expect("loop break slot poisoned").get_or_insert(broken);
    }

    /// The pending `break`, if one fired.
    pub fn pending(&self) -> Option<LoopBreak> {
        self.0.lock().expect("loop break slot poisoned").clone()
    }

    /// Whether a `break` is pending.
    pub fn is_pending(&self) -> bool {
        self.0.lock().expect("loop break slot poisoned").is_some()
    }

    /// Forget a pending `break`: a failure, cancellation, or retry replaces it.
    pub fn discard(&self) {
        self.0.lock().expect("loop break slot poisoned").take();
    }
}

/// The preview line for a loop a dry run visits once: the body runs one
/// pass, and the condition, gate, and action are shown, never evaluated.
pub fn dry_run_loop_note(property: &str, config: &LoopConfig, max: usize) -> String {
    let condition = match &config.condition {
        LoopCondition::Until(expr) => format!("until `{expr}`"),
        LoopCondition::While(expr) => format!("while `{expr}`"),
    };
    format!("`{property}` previews one pass (dry-run); repetition is not evaluated: {condition}, max {max}")
}

/// What the loop does after a pass's gate.
#[derive(Debug)]
pub enum LoopDecision {
    /// The condition no longer asks for a pass: the loop is complete.
    Complete,
    /// The condition asks for another pass and the cap permits it.
    Continue,
    /// The condition asks for another pass on the final permitted pass.
    Exhausted(CompositionError),
}

/// The state of the innermost loop around a pass.
///
/// `count` is the pass being run (1-based); `last_output` and
/// `last_exit_code` describe the previous pass until
/// [`Self::record_pass`] reports the one that just finished, so a pass's body
/// and `start` read the previous result while its terminal events, `finalize`,
/// the gate, the condition, and the action read its own.
#[derive(Debug, Clone)]
pub struct LoopFrame {
    config: LoopConfig,
    max: usize,
    count: usize,
    last_output: Value,
    last_exit_code: i32,
    /// The previous pass's result while the current pass's is recorded, so
    /// a retried attempt of the pass reads what the pass started with.
    previous: Option<(Value, i32)>,
    breaks: LoopBreakSlot,
    source_path: PathBuf,
    property: String,
}

impl LoopFrame {
    /// Allocate the frame for one loop invocation, before its `initialize`.
    ///
    /// The cap is `cap_override` (the runtime option or environment), else the
    /// authored `max`, else [`DEFAULT_MAX_ITERATIONS`]. `property` names the
    /// authored `loop:` block (`loop`, `sequence[1].loop`) in diagnostics.
    ///
    /// ## Errors
    ///
    /// [`CompositionError::LoopInvalid`] when the configuration still carries
    /// a templated `max`: a cap is resolved once, before the loop starts, and
    /// never falls back to the default.
    pub fn new(
        config: &LoopConfig,
        cap_override: Option<usize>,
        kind: LoopResultKind,
        source_path: &Path,
        property: &str,
    ) -> Result<Self, CompositionError> {
        if let Some(template) = &config.max_template {
            return Err(CompositionError::LoopInvalid {
                source_path: source_path.to_path_buf(),
                property: format!("{property}.max"),
                message: format!(
                    "`{property}.max` template `{template}` was not resolved before the loop started"
                ),
            });
        }
        Ok(Self {
            config: config.clone(),
            max: cap_override
                .or(config.max_iterations)
                .unwrap_or(DEFAULT_MAX_ITERATIONS),
            count: 1,
            last_output: kind.empty(),
            last_exit_code: 0,
            previous: None,
            breaks: LoopBreakSlot::default(),
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
        })
    }

    /// The frozen configuration this loop runs with.
    pub fn config(&self) -> &LoopConfig {
        &self.config
    }

    /// The pass being run, 1-based.
    pub fn count(&self) -> usize {
        self.count
    }

    /// The cap this loop runs to.
    pub fn max(&self) -> usize {
        self.max
    }

    /// The `loop` global: `is_last` is true only on the final pass the cap
    /// permits; it never predicts the condition.
    pub fn ambient(&self) -> LoopAmbient {
        LoopAmbient {
            breaks: self.breaks.clone(),
            ..LoopAmbient::new(
                self.count,
                self.count == 1,
                self.count == self.max,
                self.last_output.clone(),
                self.last_exit_code,
            )
        }
    }

    /// The `break` that ends this loop, once one fired in a page whose
    /// innermost loop this is.
    pub fn pending_break(&self) -> Option<LoopBreak> {
        self.breaks.pending()
    }

    /// Forget a pending `break`: the pass failed, was cancelled, or retries.
    pub fn discard_break(&self) {
        self.breaks.discard();
    }

    /// Report the result of the pass that just finished, before its terminal
    /// event: that event, `finalize`, the gate, the condition, and the action
    /// read it.
    pub fn record_pass(&mut self, output: Value, exit_code: i32) {
        let previous = (
            std::mem::replace(&mut self.last_output, output),
            std::mem::replace(&mut self.last_exit_code, exit_code),
        );
        self.previous.get_or_insert(previous);
    }

    /// Discard a failed attempt's recorded result when the pass is retried:
    /// the retried attempt starts from the previous pass's result again.
    pub fn retry_pass(&mut self) {
        if let Some((output, exit_code)) = self.previous.take() {
            self.last_output = output;
            self.last_exit_code = exit_code;
        }
    }

    /// The result the most recent pass reported.
    pub fn last_output(&self) -> &Value {
        &self.last_output
    }

    /// Read the condition after the gate, then classify the cap.
    ///
    /// A condition met on the final permitted pass completes the loop: the
    /// cap is classified only when the condition still asks for a pass.
    ///
    /// ## Errors
    ///
    /// [`CompositionError::LoopEvaluationFailed`] when the condition fails to
    /// parse or evaluate, located at the authored condition and this pass.
    pub fn decide(&self, lookup: &LoopExpressionLookup<'_>) -> Result<LoopDecision, CompositionError> {
        let another = evaluate_condition(&self.config.condition, lookup)
            .map_err(|source| self.evaluation_failed(self.condition_key(), source))?;
        if !another {
            return Ok(LoopDecision::Complete);
        }
        if self.count >= self.max {
            return Ok(LoopDecision::Exhausted(CompositionError::LoopLimitExceeded {
                cap: self.max,
                prompt_path: self.source_path.clone(),
                iteration: self.count,
            }));
        }
        Ok(LoopDecision::Continue)
    }

    /// Stage the loop's action against `frontmatter`, all or nothing.
    ///
    /// A shared view stages into a clone of it, so only the values the
    /// action writes are copied; an owned map is copied whole.
    ///
    /// Call only after [`Self::decide`] returned [`LoopDecision::Continue`].
    ///
    /// ## Errors
    ///
    /// [`CompositionError::LoopEvaluationFailed`] wrapping the first failing
    /// action; nothing is committed.
    pub fn stage_actions<F: ActionTarget>(
        &self,
        frontmatter: &F,
        lookup: &dyn EvaluationLookup,
    ) -> Result<F, CompositionError> {
        let mut stage = ActionStaging::new(frontmatter, self.count, self.config.actions.len());
        for (index, action) in self.config.actions.iter().enumerate() {
            stage
                .apply_action(action, index + 1, Some(lookup))
                .map_err(|source| self.evaluation_failed("action", source))?;
        }
        Ok(stage.into_staged())
    }

    /// Move to the next pass.
    pub fn advance(&mut self) {
        self.count += 1;
        self.previous = None;
    }

    /// The authored path of this loop's action (`sequence[1].loop.action`).
    pub fn action_property(&self) -> String {
        format!("{}.action", self.property)
    }

    fn condition_key(&self) -> &'static str {
        match self.config.condition {
            LoopCondition::While(_) => "while",
            LoopCondition::Until(_) => "until",
        }
    }

    fn evaluation_failed(&self, key: &str, source: CompositionError) -> CompositionError {
        // A retired loop name is already located at the authored read, and
        // names its replacement; it stays that typed error.
        if matches!(source, CompositionError::RemovedGrammar { .. }) {
            return source;
        }
        CompositionError::LoopEvaluationFailed {
            source_path: self.source_path.clone(),
            property: format!("{}.{key}", self.property),
            iteration: self.count,
            source: Box::new(source),
        }
    }
}

#[cfg(test)]
mod tests;
