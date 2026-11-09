//! Darkmatter's [`SlotHost`]: admits each slot passively and evaluates it
//! with the [`SlotEvaluator`].
//!
//! A failure keeps its typed cause. The [`SlotFailure`] carries the
//! expression's own [`ExpressionError`] (so a `load_data` failure keeps its
//! `DataError`, its `dm.data.*` code, and its data location), and its code is
//! [`failure_code`]'s.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use simplified_schema::expression::parse_spanned;
use simplified_schema::simplified::Slot;
use simplified_schema::slots::{SlotAdmissionError, SlotFailure, SlotHost, SlotRequest};
use simplified_schema::span::SourceSpan;

use super::admission::slot_class_report;
use super::codes::{
    SLOT_APPROVAL_REQUIRED_CODE, SLOT_CANCELED_CODE, SLOT_CYCLE_CODE, SLOT_DENIED_CODE, SLOT_FAILED_CODE,
    SLOT_GRANT_STORE_INVALID_CODE, SLOT_SYNTAX_CODE, slot_result_code,
};
use crate::markdown::compose::expression::data::DataError;
use super::cache::ContextCheck;
use super::dependencies;
use super::evaluator::{ContextDigest, SlotEvaluation, SlotEvaluator, context_digest_now, context_inputs, slot_context};
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::expression::read_gate::GateRefusal;

/// Resolves a template's slots on Darkmatter's expression engine.
#[derive(Debug)]
pub struct DarkmatterSlotHost {
    evaluator: SlotEvaluator,
    evaluations: Mutex<Vec<(Slot, SlotEvaluation)>>,
    cancel: Option<Arc<AtomicBool>>,
}

impl DarkmatterSlotHost {
    /// A host evaluating with `evaluator`.
    pub(crate) fn new(evaluator: SlotEvaluator) -> Self {
        Self { evaluator, evaluations: Mutex::new(Vec::new()), cancel: None }
    }

    /// Reports the resolution canceled once `cancel` is set.
    pub(crate) fn with_cancel(mut self, cancel: Option<Arc<AtomicBool>>) -> Self {
        self.cancel = cancel;
        self
    }

    /// Every successful evaluation so far, in order, with what it depended
    /// on.
    #[cfg(test)]
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
    fn is_canceled(&self) -> bool {
        self.cancel.as_ref().is_some_and(|cancel| cancel.load(Ordering::Acquire))
    }

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
        // Recorded for every slot that runs (a later union round, a nested
        // contract's), whether it succeeds or fails, so a cached outcome is
        // re-checked against each context input it could have read.
        for input in context_inputs(&parsed) {
            dependencies::note_context(ContextCheck {
                defining: request.defining_file.map(std::path::Path::to_path_buf),
                digest: ContextDigest { digest: context_digest_now(&input, &context), input },
            });
        }
        let defining = request.defining_file.unwrap_or_else(|| request.context.base_dir());
        crate::note_work!(SlotResolution);
        match self.evaluator.evaluate(&parsed, context, defining) {
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
/// `load_data` failure's `dm.data.*` code (or, for a contract naming a
/// template whose own slots failed, the code of that failure), an exhausted
/// budget's `dm.iteration.*` code, and [`SLOT_FAILED_CODE`] for any other
/// failure (a missing file, a failed call), whose typed cause is still kept.
pub(crate) fn failure_code(error: &ExpressionError) -> &'static str {
    if let Some(data) = error.data_failure() {
        return match contract_template_failure(data) {
            Some(failure) => nested_resolution_code(&failure.error),
            None => data.code().unwrap_or(SLOT_FAILED_CODE),
        };
    }
    match error.without_callbacks() {
        ExpressionError::SlotReadRefused { refusal: GateRefusal::Denied, .. } => SLOT_DENIED_CODE,
        ExpressionError::SlotReadRefused { .. } => SLOT_APPROVAL_REQUIRED_CODE,
        ExpressionError::IterationLimit(limit) => limit.kind.code(),
        _ => SLOT_FAILED_CODE,
    }
}

/// The failed resolution of a `load_data` contract naming a template, when
/// that is why `error` happened: the slots of the template the contract
/// named, resolved inside the slot that read the data.
pub(crate) fn contract_template_failure(error: &DataError) -> Option<&simplified_schema::SlotResolutionFailure> {
    match error {
        DataError::ContractSchema { source, .. } => match source.as_ref() {
            simplified_schema::SchemaError::SlotResolution(failure) => Some(failure),
            _ => None,
        },
        _ => None,
    }
}

/// The code of a slot whose contract template's resolution failed with
/// `error`: the code the template's failing slot reports, as if it had
/// failed in this slot.
///
/// When several of the template's slots failed, a cycle outranks a missing
/// or invalid grant, which outranks an exhausted work limit, which outranks
/// the first failure: this code decides whether the outer failure may be
/// cached, deferred for a grant, or handed to a command with other limits
/// (`cache::caches_failure`, `session::needs_grant`, `session::exhausted_work`).
fn nested_resolution_code(error: &simplified_schema::slots::SlotResolutionError) -> &'static str {
    use simplified_schema::slots::SlotResolutionError;
    let codes: Vec<&'static str> = match error {
        SlotResolutionError::Refused(refused) => {
            refused.iter().map(|(_, refusal)| refusal.code.unwrap_or(SLOT_FAILED_CODE)).collect()
        }
        SlotResolutionError::Failed(failed) => {
            failed.iter().map(|(_, failure)| failure.code.unwrap_or(SLOT_FAILED_CODE)).collect()
        }
        SlotResolutionError::Schema(schema) => match schema.as_ref() {
            simplified_schema::SchemaError::SlotResult { kind, .. } => vec![slot_result_code(kind)],
            simplified_schema::SchemaError::SlotResolution(failure) => vec![nested_resolution_code(&failure.error)],
            _ => Vec::new(),
        },
        SlotResolutionError::Canceled => vec![SLOT_CANCELED_CODE],
        _ => Vec::new(),
    };
    let grant_dependent = [SLOT_APPROVAL_REQUIRED_CODE, SLOT_DENIED_CODE, SLOT_GRANT_STORE_INVALID_CODE];
    codes
        .iter()
        .find(|code| **code == SLOT_CYCLE_CODE)
        .or_else(|| codes.iter().find(|code| grant_dependent.contains(code)))
        .or_else(|| codes.iter().find(|code| code.starts_with("dm.iteration.")))
        .or_else(|| codes.first())
        .copied()
        .unwrap_or(SLOT_FAILED_CODE)
}
