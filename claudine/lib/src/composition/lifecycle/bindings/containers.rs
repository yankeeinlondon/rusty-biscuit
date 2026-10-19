//! Owned lexical roots. Borrowing an inner value replaces the entire outer
//! root; dropping that borrow restores the caller's unchanged root.

use serde_json::{Map, Value};

use super::super::LifecycleSignal;
use crate::composition::sequence::SequenceStepOverlay;

/// The innermost sequence's read-only expression value, separate from frontmatter.
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceBinding(Value);

impl SequenceBinding {
    /// Standalone prompts expose only their accumulated outputs.
    pub fn standalone(outputs: Vec<Value>) -> Self {
        Self(serde_json::json!({"outputs": outputs}))
    }

    /// A sequence's own events have no active child position.
    pub fn container(run_id: &str, outputs: Vec<Value>) -> Self {
        Self(serde_json::json!({
            "name": null, "id": null, "index": null, "count": null,
            "is_first": null, "is_last": null, "previous": null, "next": null,
            "run_id": run_id, "outputs": outputs,
        }))
    }

    /// A child sees its own row and shallow neighboring rows.
    pub fn step(overlay: &SequenceStepOverlay, outputs: Vec<Value>) -> Self {
        let mut row = overlay.state.to_value();
        let map = row.as_object_mut().expect("step state is an object");
        map.remove("sequence_id");
        map.insert("run_id".into(), Value::String(overlay.sequence_id.clone()));
        let neighbor = |state: &crate::composition::sequence::StepState| {
            let mut row = state.to_value();
            row.as_object_mut().expect("step state is an object").remove("sequence_id");
            row
        };
        map.insert("previous".into(), overlay.previous.as_ref().map_or(Value::Null, neighbor));
        map.insert("next".into(), overlay.next.as_ref().map_or(Value::Null, neighbor));
        map.insert("outputs".into(), Value::Array(outputs));
        Self(row)
    }

    /// The full typed expression object; string shorthand is presentation only.
    pub fn to_value(&self) -> Value {
        self.0.clone()
    }
}

/// Final classification of one named leg, including a leg that never started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegOutcome {
    /// Completed successfully.
    Succeeded,
    /// Completed with an error.
    Failed,
    /// Cancelled, possibly before admission.
    Cancelled,
    /// Explicitly skipped.
    Skipped,
}

impl LegOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }
}

/// Settled leg data. Store records in declaration order, not completion order.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupLegResult {
    /// Unique authored leg name, validated at preflight.
    pub name: String,
    /// Terminal classification.
    pub outcome: LegOutcome,
    /// Result, including partial failure output; null when never admitted.
    pub output: Value,
    /// Structured error/cancellation cause; null for success or skip.
    pub err: Value,
}

/// The innermost group's variables and settled children.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GroupBinding {
    /// Authored variables, excluding the four reserved outcome names.
    pub variables: Map<String, Value>,
    /// Settled children in declaration order.
    pub legs: Vec<GroupLegResult>,
}

impl GroupBinding {
    /// A leg sees variables only, never concurrently changing sibling results.
    pub fn leg_value(&self) -> Map<String, Value> {
        self.variables.clone()
    }

    /// Only terminal events and the loop gate expose settled results.
    pub fn event_value(&self, event: LifecycleSignal) -> Map<String, Value> {
        let populated = matches!(event, LifecycleSignal::Success | LifecycleSignal::Failure
            | LifecycleSignal::Cancelled | LifecycleSignal::Finalize | LifecycleSignal::Loop);
        let mut value = self.variables.clone();
        let mut legs = Map::new();
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        let mut cancelled = Vec::new();
        if populated {
            for leg in &self.legs {
                let name = Value::String(leg.name.clone());
                match leg.outcome {
                    LegOutcome::Succeeded => succeeded.push(name),
                    LegOutcome::Failed => failed.push(name),
                    LegOutcome::Cancelled => cancelled.push(name),
                    LegOutcome::Skipped => {}
                }
                legs.insert(leg.name.clone(), serde_json::json!({
                    "name": leg.name, "outcome": leg.outcome.as_str(),
                    "output": leg.output, "err": leg.err,
                }));
            }
        }
        value.insert("legs".into(), Value::Object(legs));
        value.insert("succeeded".into(), Value::Array(succeeded));
        value.insert("failed".into(), Value::Array(failed));
        value.insert("cancelled".into(), Value::Array(cancelled));
        value
    }
}
