//! Binding a call: arity, every argument, recovery, and order-independent
//! overload selection; the incremental path for lazy functions; and the
//! bound-argument accessors handlers read.

use std::borrow::Cow;
use std::cell::Cell;

use serde_json::{Map, Value};

use super::error::{ArgumentError, ArgumentProblem, CallBindingError, CandidateFailure, SignatureProblem};
use super::registration::{EvaluationMode, FunctionRegistration};
use super::signature::{CompiledParameter, CompiledSignature};
use crate::markdown::coercion::{BindOptions, Bound, PathSegment, ValueKind, bind_with};

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
    /// Indices of the arguments in which the caller's deferral predicate
    /// deferred a value; always empty without one.
    deferred: Vec<usize>,
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

    /// The arguments, by index, holding a value the deferral predicate of
    /// [`FunctionRegistration::bind_call_with`] deferred (the whole argument
    /// or any value inside it). A deferred value is kept as written and is
    /// not yet known to bind.
    pub fn deferred_arguments(&self) -> &[usize] {
        &self.deferred
    }

    /// Whether any argument holds a deferred value.
    pub fn is_deferred(&self) -> bool {
        !self.deferred.is_empty()
    }

    /// Detaches the call from the caller's values (see
    /// [`BoundArgs::into_owned`]).
    pub fn into_owned(self) -> BoundCall<'static> {
        BoundCall {
            overload: self.overload,
            exact: self.exact,
            args: self.args.into_owned(),
            deferred: self.deferred,
        }
    }
}

/// The outcome of [`FunctionRegistration::select_overload_with`].
#[derive(Debug, Clone, PartialEq)]
pub enum OverloadSelection<'v> {
    /// No argument holds a deferred value, and the call binds to this
    /// overload by the same rule as [`FunctionRegistration::bind_call`].
    Selected(Box<BoundCall<'v>>),
    /// Some binding holds a deferred value, so the overload is chosen only
    /// once the value exists. Every overload that binds with the deferred
    /// values taken as matching remains, in registration order; each holds
    /// at least one binding (a deferred value never fails, so a binding
    /// without one would bind at event time too and still competes).
    Deferred(Vec<BoundCall<'v>>),
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
        self.bind_call_with(args, &BindOptions::default())
    }

    /// [`FunctionRegistration::bind_call`] with coercion options.
    ///
    /// A value the deferral predicate defers binds as kept, so it never
    /// fails and never makes a binding converted; the predicate sees the
    /// path relative to the argument. It is consulted for a whole argument
    /// before the source exclusions and the `null` rule, which judge the
    /// value supplied later, not the placeholder. Arity is never deferred.
    /// Overloads still select by the runtime rule, so a deferred argument
    /// that several overloads accept is ambiguous here; passive callers use
    /// [`FunctionRegistration::select_overload_with`].
    ///
    /// ## Errors
    ///
    /// As [`FunctionRegistration::bind_call`].
    pub fn bind_call_with<'v>(&self, args: &'v [Value], options: &BindOptions<'_>) -> Result<BoundCall<'v>, CallBindingError> {
        let bindings = self.bind_each(args, options)?;
        self.select(bindings)
    }

    /// Binds like [`FunctionRegistration::bind_call_with`], but leaves the
    /// overload open when a deferred value is involved: see
    /// [`OverloadSelection`]. Without a deferral, the result is exactly
    /// `bind_call_with`'s.
    ///
    /// ## Errors
    ///
    /// As [`FunctionRegistration::bind_call`]; ambiguity only when no binding
    /// holds a deferred value.
    pub fn select_overload_with<'v>(
        &self,
        args: &'v [Value],
        options: &BindOptions<'_>,
    ) -> Result<OverloadSelection<'v>, CallBindingError> {
        let bindings = self.bind_each(args, options)?;
        if bindings.iter().any(BoundCall::is_deferred) {
            return Ok(OverloadSelection::Deferred(bindings));
        }
        self.select(bindings).map(|call| OverloadSelection::Selected(Box::new(call)))
    }

    /// Every overload that binds, in registration order, or the failure of
    /// every candidate when none does.
    fn bind_each<'v>(&self, args: &'v [Value], options: &BindOptions<'_>) -> Result<Vec<BoundCall<'v>>, CallBindingError> {
        let mut bindings = Vec::new();
        let mut failures = Vec::new();
        for (index, signature) in self.overloads.iter().enumerate() {
            match bind_signature(signature, args, options) {
                Ok((bound, exact, deferred)) => bindings.push(BoundCall {
                    overload: index,
                    exact,
                    args: bound,
                    deferred,
                }),
                Err(problem) => failures.push(candidate(signature, problem)),
            }
        }
        if bindings.is_empty() {
            return Err(self.failure(failures, args.len()));
        }
        Ok(bindings)
    }

    /// The runtime selection rule: the exact tier if it is not empty,
    /// otherwise the converted one; that tier must hold exactly one binding.
    fn select<'v>(&self, bindings: Vec<BoundCall<'v>>) -> Result<BoundCall<'v>, CallBindingError> {
        let any_exact = bindings.iter().any(BoundCall::is_exact);
        let mut tier: Vec<BoundCall<'v>> = bindings.into_iter().filter(|call| call.exact == any_exact).collect();
        if tier.len() == 1 {
            return Ok(tier.pop().expect("one binding"));
        }
        Err(CallBindingError::AmbiguousOverload {
            function: self.name.clone(),
            exact: any_exact,
            signatures: tier.iter().map(|call| self.overloads[call.overload].to_string()).collect(),
        })
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

    /// Binds a value argument of a callback registration without inventing a
    /// JSON value for its inline callback. The evaluator checks callback syntax
    /// and arity separately; current callback registrations have one overload.
    ///
    /// ## Errors
    /// A type or arity error from the same binder used by eager calls.
    pub fn bind_callback_value(&self, index: usize, value: Value) -> Result<Value, CallBindingError> {
        let signature = &self.overloads[0];
        let Some(parameter) = signature.parameter_for(index) else {
            return Err(self.failure(vec![candidate(signature, arity_problem(signature, index + 1))], index + 1));
        };
        match bind_argument(parameter, index, &value, &BindOptions::default()) {
            Ok((Bound::Kept(_), _)) => Ok(value),
            Ok((Bound::Converted(value), _)) => Ok(value),
            Err(error) => Err(self.failure(
                vec![candidate(signature, SignatureProblem::Arguments(vec![*error]))], index + 1,
            )),
        }
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
    crate::note_work!(CandidateFailureBuild);
    CandidateFailure {
        signature: signature.to_string(),
        problem,
    }
}

fn arity_problem(signature: &CompiledSignature, supplied: usize) -> SignatureProblem {
    let (min, max) = signature.arity();
    SignatureProblem::Arity { min, max, supplied }
}

/// Binds every argument to `signature`: the bound arguments, whether every
/// one was kept unchanged, and the indices of the arguments holding a
/// deferred value.
fn bind_signature<'v>(
    signature: &CompiledSignature,
    args: &'v [Value],
    options: &BindOptions<'_>,
) -> Result<(BoundArgs<'v>, bool, Vec<usize>), SignatureProblem> {
    if !signature.accepts_count(args.len()) {
        return Err(arity_problem(signature, args.len()));
    }
    let mut slots = Vec::with_capacity(args.len());
    let mut errors = Vec::new();
    let mut exact = true;
    let mut subject_error = None;
    let mut deferred = Vec::new();
    for (index, value) in args.iter().enumerate() {
        let parameter = signature.parameter_for(index).expect("arity admits every argument");
        match bind_argument(parameter, index, value, options) {
            Ok((bound, true)) => {
                deferred.push(index);
                push_bound(&mut slots, &mut exact, bound);
            }
            Ok((Bound::Kept(kept), false)) => slots.push(Slot::Bound(Cow::Borrowed(kept))),
            Ok((Bound::Converted(converted), false)) => {
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
        deferred,
    ))
}

fn push_bound<'v>(slots: &mut Vec<Slot<'v>>, exact: &mut bool, bound: Bound<'v>) {
    match bound {
        Bound::Kept(kept) => slots.push(Slot::Bound(Cow::Borrowed(kept))),
        Bound::Converted(converted) => {
            *exact = false;
            slots.push(Slot::Bound(Cow::Owned(converted)));
        }
    }
}

impl CompiledParameter {
    /// Whether `value` binds to this parameter exactly as a call argument
    /// would: source exclusions, the `null` rule, then the coercion engine.
    /// Passive; the value is never changed.
    pub fn binds(&self, value: &Value) -> bool {
        bind_argument(self, self.index(), value, &BindOptions::default()).is_ok()
    }
}

/// Binds one argument: a deferred whole value first, then registered source
/// exclusions, then the parameter-level `null` rule, then the coercion
/// engine. The flag says whether the deferral predicate deferred the value
/// or any value inside it.
fn bind_argument<'v>(
    parameter: &CompiledParameter,
    index: usize,
    value: &'v Value,
    options: &BindOptions<'_>,
) -> Result<(Bound<'v>, bool), Box<ArgumentError>> {
    let deferred = Cell::new(false);
    let recording = |path: &[PathSegment], value: &Value| {
        let defer = options.defer.is_some_and(|defer| defer(path, value));
        if defer {
            deferred.set(true);
        }
        defer
    };
    let options = BindOptions {
        defer: options.defer.is_some().then_some(&recording as &dyn Fn(&[PathSegment], &Value) -> bool),
        max_depth: options.max_depth,
    };
    let problem = if options.defer.is_some() && recording(&[], value) {
        return Ok((Bound::Kept(value), true));
    } else if parameter.excluded_sources().contains(&ValueKind::of(value)) {
        ArgumentProblem::SourceExcluded
    } else if value.is_null() && !parameter.admits_null() {
        ArgumentProblem::NullNotAdmitted
    } else {
        match bind_with(parameter.shape(), value, &options) {
            Ok(bound) => return Ok((bound, deferred.get())),
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
        let bound = match bind_argument(parameter, index, &value, &BindOptions::default()) {
            Ok((Bound::Kept(_), _)) => value,
            Ok((Bound::Converted(converted), _)) => {
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
            deferred: Vec::new(),
        })
    }
}
