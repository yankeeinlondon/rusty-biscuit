//! Binding a call: arity, every argument, recovery, and order-independent
//! overload selection; the incremental path for lazy functions; and the
//! bound-argument accessors handlers read.

use std::borrow::Cow;

use serde_json::{Map, Value};

use super::error::{ArgumentError, ArgumentProblem, CallBindingError, CandidateFailure, SignatureProblem};
use super::registration::{EvaluationMode, FunctionRegistration};
use super::signature::{CompiledParameter, CompiledSignature};
use crate::markdown::coercion::{Bound, ValueKind, bind};

/// A numeric recovery that bound the call with the fallback selected.
#[derive(Debug, Clone, PartialEq)]
pub struct Recovery {
    /// The subject's argument index.
    pub subject: usize,
    /// The fallback's argument index.
    pub fallback: usize,
    /// Why the subject did not bind.
    pub subject_error: ArgumentError,
}

#[derive(Debug, Clone, PartialEq)]
enum Slot<'v> {
    Bound(Cow<'v, Value>),
    /// A recovered subject: the original value, which did not bind.
    Recovered(Cow<'v, Value>),
}

/// The bound arguments of one call: values of exactly the declared types.
///
/// Accessors only read; they never parse or convert. Omission stays distinct
/// from `null`: an omitted optional parameter has no value at all.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundArgs<'v> {
    slots: Vec<Slot<'v>>,
    /// The index of the first argument a rest parameter collects.
    rest_start: Option<usize>,
    recovery: Option<Recovery>,
}

impl<'v> BoundArgs<'v> {
    /// How many arguments were supplied.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The bound value at `index`; `None` when it was omitted or is a
    /// recovered subject (see [`BoundArgs::selected`]).
    pub fn get(&self, index: usize) -> Option<&Value> {
        match self.slots.get(index)? {
            Slot::Bound(value) => Some(value),
            Slot::Recovered(_) => None,
        }
    }

    /// Whether the argument at `index` was left out of the call.
    pub fn is_omitted(&self, index: usize) -> bool {
        index >= self.slots.len()
    }

    /// The value the handler computes with for `index`: the bound value, or
    /// the fallback when `index` is a recovered subject.
    pub fn selected(&self, index: usize) -> Option<&Value> {
        match &self.recovery {
            Some(recovery) if recovery.subject == index => self.get(recovery.fallback),
            _ => self.get(index),
        }
    }

    /// The recovery that bound this call, if any.
    pub fn recovery(&self) -> Option<&Recovery> {
        self.recovery.as_ref()
    }

    /// The arguments a rest parameter collected, in order; empty when the
    /// signature has none.
    pub fn rest(&self) -> impl ExactSizeIterator<Item = &Value> {
        let start = self.rest_start.map_or(self.slots.len(), |start| start.min(self.slots.len()));
        self.slots[start..].iter().map(|slot| match slot {
            Slot::Bound(value) | Slot::Recovered(value) => value.as_ref(),
        })
    }

    pub fn number(&self, index: usize) -> Option<f64> {
        self.get(index).and_then(Value::as_f64)
    }

    pub fn text(&self, index: usize) -> Option<&str> {
        self.get(index).and_then(Value::as_str)
    }

    pub fn boolean(&self, index: usize) -> Option<bool> {
        self.get(index).and_then(Value::as_bool)
    }

    pub fn list(&self, index: usize) -> Option<&[Value]> {
        self.get(index).and_then(Value::as_array).map(Vec::as_slice)
    }

    pub fn object(&self, index: usize) -> Option<&Map<String, Value>> {
        self.get(index).and_then(Value::as_object)
    }

    /// Detaches the arguments from the caller's values, cloning only those
    /// that were kept.
    pub fn into_owned(self) -> BoundArgs<'static> {
        let own = |value: Cow<'v, Value>| Cow::Owned(value.into_owned());
        BoundArgs {
            slots: self
                .slots
                .into_iter()
                .map(|slot| match slot {
                    Slot::Bound(value) => Slot::Bound(own(value)),
                    Slot::Recovered(value) => Slot::Recovered(own(value)),
                })
                .collect(),
            rest_start: self.rest_start,
            recovery: self.recovery,
        }
    }
}

/// A call bound to one overload.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundCall<'v> {
    overload: usize,
    exact: bool,
    args: BoundArgs<'v>,
}

impl<'v> BoundCall<'v> {
    /// The index of the selected overload in the registration.
    pub fn overload(&self) -> usize {
        self.overload
    }

    /// Whether every argument was kept unchanged.
    pub fn is_exact(&self) -> bool {
        self.exact
    }

    pub fn args(&self) -> &BoundArgs<'v> {
        &self.args
    }

    pub fn into_args(self) -> BoundArgs<'v> {
        self.args
    }

    /// Detaches the call from the caller's values (see
    /// [`BoundArgs::into_owned`]).
    pub fn into_owned(self) -> BoundCall<'static> {
        BoundCall {
            overload: self.overload,
            exact: self.exact,
            args: self.args.into_owned(),
        }
    }
}

impl FunctionRegistration {
    /// Binds evaluated arguments to one overload.
    ///
    /// Every supplied argument is checked against every overload whose arity
    /// fits; the caller's values are never mutated. An overload that keeps
    /// every argument unchanged wins if it is the only one; otherwise exactly
    /// one overload must bind. Written order never matters.
    ///
    /// ## Errors
    ///
    /// A [`CallBindingError`] of kind type, arity, or ambiguous overload.
    pub fn bind_call<'v>(&self, args: &'v [Value]) -> Result<BoundCall<'v>, CallBindingError> {
        if let [signature] = self.overloads.as_slice() {
            return bind_signature(signature, args)
                .map(|(args, exact)| BoundCall { overload: 0, exact, args })
                .map_err(|problem| self.failure(vec![candidate(signature, problem)], args.len()));
        }
        let mut exact = Vec::new();
        let mut converted = Vec::new();
        let mut failures = Vec::new();
        for (index, signature) in self.overloads.iter().enumerate() {
            match bind_signature(signature, args) {
                Ok((bound, true)) => exact.push((index, bound)),
                Ok((bound, false)) => converted.push((index, bound)),
                Err(problem) => failures.push(candidate(signature, problem)),
            }
        }
        let (tier, is_exact) = if exact.is_empty() { (converted, false) } else { (exact, true) };
        match tier.len() {
            0 => Err(self.failure(failures, args.len())),
            1 => {
                let (overload, args) = tier.into_iter().next().expect("one binding");
                Ok(BoundCall {
                    overload,
                    exact: is_exact,
                    args,
                })
            }
            _ => Err(CallBindingError::AmbiguousOverload {
                function: self.name.clone(),
                exact: is_exact,
                signatures: tier.iter().map(|(index, _)| self.overloads[*index].to_string()).collect(),
            }),
        }
    }

    /// Checks only the argument count, before any argument is evaluated.
    ///
    /// ## Errors
    ///
    /// An arity error when no overload takes `supplied` arguments.
    pub fn check_arity(&self, supplied: usize) -> Result<(), CallBindingError> {
        if self.overloads.iter().any(|signature| signature.accepts_count(supplied)) {
            return Ok(());
        }
        let candidates = self
            .overloads
            .iter()
            .map(|signature| candidate(signature, arity_problem(signature, supplied)))
            .collect();
        Err(self.failure(candidates, supplied))
    }

    /// Starts an incremental binding for a lazy function; `None` for an eager
    /// one.
    pub fn lazy_call(&self) -> Option<LazyCall<'_>> {
        (self.evaluation == EvaluationMode::Lazy).then(|| LazyCall {
            registration: self,
            signature: &self.overloads[0],
            values: Vec::new(),
            exact: true,
        })
    }

    /// Checks a handler's successful result against the selected overload's
    /// `returns(...)`, without converting it.
    ///
    /// ## Errors
    ///
    /// [`CallBindingError::ImplementationContract`].
    pub fn check_return(&self, call: &BoundCall<'_>, value: &Value) -> Result<(), CallBindingError> {
        self.overloads[call.overload].check_return(value)
    }

    fn failure(&self, candidates: Vec<CandidateFailure>, supplied: usize) -> CallBindingError {
        let function = self.name.clone();
        if candidates.iter().all(|candidate| matches!(candidate.problem, SignatureProblem::Arity { .. })) {
            CallBindingError::Arity {
                function,
                supplied,
                candidates,
            }
        } else {
            CallBindingError::Type { function, candidates }
        }
    }
}

fn candidate(signature: &CompiledSignature, problem: SignatureProblem) -> CandidateFailure {
    CandidateFailure {
        signature: signature.to_string(),
        problem,
    }
}

fn arity_problem(signature: &CompiledSignature, supplied: usize) -> SignatureProblem {
    let (min, max) = signature.arity();
    SignatureProblem::Arity { min, max, supplied }
}

/// Binds every argument to `signature`; the flag says whether every one
/// was kept unchanged.
fn bind_signature<'v>(signature: &CompiledSignature, args: &'v [Value]) -> Result<(BoundArgs<'v>, bool), SignatureProblem> {
    if !signature.accepts_count(args.len()) {
        return Err(arity_problem(signature, args.len()));
    }
    let mut slots = Vec::with_capacity(args.len());
    let mut errors = Vec::new();
    let mut exact = true;
    let mut subject_error = None;
    for (index, value) in args.iter().enumerate() {
        let parameter = signature.parameter_for(index).expect("arity admits every argument");
        match bind_argument(parameter, index, value) {
            Ok(Bound::Kept(kept)) => slots.push(Slot::Bound(Cow::Borrowed(kept))),
            Ok(Bound::Converted(converted)) => {
                exact = false;
                slots.push(Slot::Bound(Cow::Owned(converted)));
            }
            Err(error) => {
                slots.push(Slot::Recovered(Cow::Borrowed(value)));
                if signature.recovery.is_some_and(|(subject, _)| subject == index) {
                    subject_error = Some(*error);
                } else {
                    errors.push(*error);
                }
            }
        }
    }
    let mut recovery = None;
    if let Some(error) = subject_error {
        let (subject, fallback) = signature.recovery.expect("a subject error implies a recovery");
        if errors.is_empty() && matches!(slots.get(fallback), Some(Slot::Bound(_))) {
            exact = false;
            recovery = Some(Recovery {
                subject,
                fallback,
                subject_error: error,
            });
        } else {
            errors.insert(0, error);
            errors.sort_by_key(|error| error.index);
        }
    }
    if !errors.is_empty() {
        return Err(SignatureProblem::Arguments(errors));
    }
    let rest_start = signature.parameters().last().filter(|parameter| parameter.is_rest()).map(CompiledParameter::index);
    Ok((
        BoundArgs {
            slots,
            rest_start,
            recovery,
        },
        exact,
    ))
}

/// Binds one argument: registered source exclusions first, then the
/// parameter-level `null` rule, then the coercion engine.
fn bind_argument<'v>(parameter: &CompiledParameter, index: usize, value: &'v Value) -> Result<Bound<'v>, Box<ArgumentError>> {
    let problem = if parameter.excluded_sources().contains(&ValueKind::of(value)) {
        ArgumentProblem::SourceExcluded
    } else if value.is_null() && !parameter.admits_null() {
        ArgumentProblem::NullNotAdmitted
    } else {
        match bind(parameter.shape(), value) {
            Ok(bound) => return Ok(bound),
            Err(error) => ArgumentProblem::Conversion(error),
        }
    };
    Err(Box::new(ArgumentError {
        parameter: parameter.name().to_string(),
        index,
        expected: parameter.type_text().to_string(),
        received: value.clone(),
        problem,
    }))
}

/// An incremental binding for a lazy function: each argument binds as the
/// evaluator produces it, so operands it never evaluates are never bound
/// and never select an overload.
#[derive(Debug)]
pub struct LazyCall<'r> {
    registration: &'r FunctionRegistration,
    signature: &'r CompiledSignature,
    values: Vec<Value>,
    exact: bool,
}

impl LazyCall<'_> {
    /// Binds the next argument and returns its bound value, for the
    /// evaluator to decide whether to continue.
    ///
    /// ## Errors
    ///
    /// An arity error past the last parameter, or a type error for this
    /// argument.
    pub fn bind_next(&mut self, value: Value) -> Result<&Value, CallBindingError> {
        let index = self.values.len();
        let Some(parameter) = self.signature.parameter_for(index) else {
            return Err(self.registration.failure(
                vec![candidate(self.signature, arity_problem(self.signature, index + 1))],
                index + 1,
            ));
        };
        let bound = match bind_argument(parameter, index, &value) {
            Ok(Bound::Kept(_)) => value,
            Ok(Bound::Converted(converted)) => {
                self.exact = false;
                converted
            }
            Err(error) => {
                return Err(self.registration.failure(
                    vec![candidate(self.signature, SignatureProblem::Arguments(vec![*error]))],
                    index + 1,
                ));
            }
        };
        self.values.push(bound);
        Ok(self.values.last().expect("just pushed"))
    }

    /// How many arguments have been bound.
    pub fn bound(&self) -> usize {
        self.values.len()
    }

    /// Ends the call with the arguments bound so far.
    ///
    /// ## Errors
    ///
    /// An arity error when fewer arguments were bound than the signature
    /// requires.
    pub fn finish(self) -> Result<BoundCall<'static>, CallBindingError> {
        let supplied = self.values.len();
        if !self.signature.accepts_count(supplied) {
            return Err(self
                .registration
                .failure(vec![candidate(self.signature, arity_problem(self.signature, supplied))], supplied));
        }
        let rest_start = self
            .signature
            .parameters()
            .last()
            .filter(|parameter| parameter.is_rest())
            .map(CompiledParameter::index);
        Ok(BoundCall {
            overload: 0,
            exact: self.exact,
            args: BoundArgs {
                slots: self.values.into_iter().map(|value| Slot::Bound(Cow::Owned(value))).collect(),
                rest_start,
                recovery: None,
            },
        })
    }
}
