//! Darkmatter's [`SlotHost`]: admits each slot passively and evaluates it
//! with the [`SlotEvaluator`].
//!
//! A failure keeps its typed cause. The [`SlotFailure`] carries the
//! expression's own [`ExpressionError`] (so a `load_data` failure keeps its
//! `DataError`, its `dm.data.*` code, and its data location), and its code is
//! [`failure_code`]'s.

use std::sync::Mutex;

use serde_json::Value;
use simplified_schema::expression::parse_spanned;
use simplified_schema::simplified::Slot;
use simplified_schema::slots::{SlotAdmissionError, SlotFailure, SlotHost, SlotRequest};
use simplified_schema::span::SourceSpan;

use super::admission::slot_class_report;
use super::codes::{SLOT_APPROVAL_REQUIRED_CODE, SLOT_DENIED_CODE, SLOT_FAILED_CODE, SLOT_SYNTAX_CODE};
use super::evaluator::{SlotEvaluation, SlotEvaluator, slot_context};
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::expression::read_gate::GateRefusal;

/// Resolves a template's slots on Darkmatter's expression engine.
#[derive(Debug)]
pub struct DarkmatterSlotHost {
    evaluator: SlotEvaluator,
    evaluations: Mutex<Vec<(Slot, SlotEvaluation)>>,
}

// The slot session builds hosts; until it lands only tests do.
#[cfg_attr(not(test), allow(dead_code))]
impl DarkmatterSlotHost {
    /// A host evaluating with `evaluator`.
    pub(crate) fn new(evaluator: SlotEvaluator) -> Self {
        Self { evaluator, evaluations: Mutex::new(Vec::new()) }
    }

    /// Every successful evaluation so far, in order, with what it depended
    /// on.
    pub(crate) fn evaluations(&self) -> Vec<(Slot, SlotEvaluation)> {
        self.evaluations.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
}

/// `span`, an offset in `slot`'s expression text, in the slot's own source
/// coordinates.
fn in_slot(slot: &Slot, span: &SourceSpan) -> SourceSpan {
    let start = slot.expression_span.start;
    (start + span.start)..(start + span.end)
}

impl SlotHost for DarkmatterSlotHost {
    fn admit(&self, slot: &Slot) -> Result<(), SlotAdmissionError> {
        let parsed = parse_spanned(&slot.expression).map_err(|error| {
            let at = slot.expression_span.start + error.position.min(slot.expression.len());
            SlotAdmissionError {
                code: Some(SLOT_SYNTAX_CODE),
                message: format!("the slot is not an expression: {}", error.message),
                span: Some(at..at),
            }
        })?;
        let report = slot_class_report(&parsed);
        match report.rejections.into_iter().next() {
            Some(rejection) => Err(SlotAdmissionError {
                code: Some(rejection.code),
                message: rejection.message,
                span: Some(in_slot(slot, &rejection.span)),
            }),
            None => Ok(()),
        }
    }

    fn resolve(&self, request: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        let slot = request.slot;
        let parsed = parse_spanned(&slot.expression)
            .map_err(|error| SlotFailure::new(ExpressionError::Parse(error.message), Some(SLOT_SYNTAX_CODE)))?;
        let context = slot_context(request.context, request.defining_file);
        match self.evaluator.evaluate(&parsed, context) {
            Ok(evaluation) => {
                let value = evaluation.value.clone();
                self.evaluations
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push((slot.clone(), evaluation));
                Ok(value)
            }
            Err(error) => {
                let code = failure_code(&error);
                Err(SlotFailure::new(error, Some(code)))
            }
        }
    }
}

/// The code a failed slot reports: a refused read's slot code, a
/// `load_data` failure's `dm.data.*` code, an exhausted budget's
/// `dm.iteration.*` code, and [`SLOT_FAILED_CODE`] for any other failure (a
/// missing file, a failed call), whose typed cause is still kept.
pub(crate) fn failure_code(error: &ExpressionError) -> &'static str {
    match error.without_callbacks() {
        ExpressionError::SlotReadRefused { refusal: GateRefusal::Denied, .. } => SLOT_DENIED_CODE,
        ExpressionError::SlotReadRefused { .. } => SLOT_APPROVAL_REQUIRED_CODE,
        ExpressionError::Data(data) => data.code().unwrap_or(SLOT_FAILED_CODE),
        ExpressionError::IterationLimit(limit) => limit.kind.code(),
        _ => SLOT_FAILED_CODE,
    }
}
