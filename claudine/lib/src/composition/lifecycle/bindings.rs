//! The lifecycle binding catalog: which Claudine globals each lifecycle scope
//! declares, and the runtime entries that satisfy those declarations.
//!
//! Claudine owns this policy (spec R4); Darkmatter owns everything else. A
//! scope's [`BindingView`] is handed to Darkmatter's passive validation
//! (`validate_prepared`) and to runtime association (`EvaluationSession`), so
//! prepare time and event time cannot disagree about whether a bare name is a
//! lifecycle global or a document property. Claudine never walks an expression
//! to enforce it.
//!
//! | Scope | `err` | `timing` | `group` | `outputs` |
//! | --- | --- | --- | --- | --- |
//! | `initialize`, `start`, `success`, `loop`, task setup | unavailable: event-has-no-error | available | lexical | document |
//! | `blocked`, `failure`, `finalize`, task teardown | available (`null` when no error) | available | lexical | document |
//! | lifecycle shell approval | unavailable: preflight-unavailable | unavailable: preflight-unavailable | unavailable: preflight-unavailable | document |
//! | sequence shell approval | unavailable: preflight-unavailable | unavailable: preflight-unavailable | unavailable: preflight-unavailable | unavailable: preflight-unavailable |
//!
//! "Lexical" is the group rule: an established group supplies its variables
//! (an empty object included); known absence is unavailable with
//! `claudine.outside-group`; unknown membership is execution-dependent during
//! passive checks. `doc.<name>` always reads the document.
//!
//! `current` and `current_env` are Darkmatter reserved namespaces, not
//! lifecycle globals, so a view cannot declare them. The approval scopes refuse
//! them separately ([`approval_diagnostics`]).

use std::collections::HashMap;
use std::sync::Arc;

use darkmatter::markdown::compose::expression::{
    Availability, AuthoredMode, BindingView, ExpressionError, PreparationError, RuntimeBinding,
    ScopeId, UnavailabilityReason, ValidationDiagnostic, prepare_value, static_variable_reads,
    validate_prepared,
};
use serde_json::{Map, Value};

use super::LifecycleSignal;
use super::context::{LifecycleErrorInfo, LifecycleTiming};

/// `err` read where no error can exist.
pub const EVENT_HAS_NO_ERROR: &str = "claudine.event-has-no-error";
/// A late value read while shell bytes are fixed, before any event fires.
pub const PREFLIGHT_UNAVAILABLE: &str = "claudine.preflight-unavailable";
/// `group` read outside any group.
pub const OUTSIDE_GROUP: &str = "claudine.outside-group";

/// The reserved namespaces whose values exist only once an event fires.
const LATE_NAMESPACES: [&str; 2] = ["current", "current_env"];

/// One place lifecycle expressions are evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleScope {
    /// A document lifecycle event.
    Event(LifecycleSignal),
    /// A sequence task's `setup:` stack.
    TaskSetup,
    /// A sequence task's `teardown:` stack.
    TaskTeardown,
    /// Fixing a document's lifecycle shell bytes at preparation.
    LifecycleShellApproval,
    /// Fixing every shell command a sequence can run, before its first step.
    SequenceShellApproval,
}

impl LifecycleScope {
    /// Every scope, events in [`LifecycleSignal::ALL`] order first.
    pub const ALL: [Self; 11] = [
        Self::Event(LifecycleSignal::Initialize),
        Self::Event(LifecycleSignal::Start),
        Self::Event(LifecycleSignal::Success),
        Self::Event(LifecycleSignal::Blocked),
        Self::Event(LifecycleSignal::Failure),
        Self::Event(LifecycleSignal::Finalize),
        Self::Event(LifecycleSignal::Loop),
        Self::TaskSetup,
        Self::TaskTeardown,
        Self::LifecycleShellApproval,
        Self::SequenceShellApproval,
    ];

    /// The scope's identity, carried by every unavailable-binding error.
    pub fn id(self) -> &'static str {
        match self {
            Self::Event(LifecycleSignal::Initialize) => "claudine.lifecycle.initialize",
            Self::Event(LifecycleSignal::Start) => "claudine.lifecycle.start",
            Self::Event(LifecycleSignal::Success) => "claudine.lifecycle.success",
            Self::Event(LifecycleSignal::Blocked) => "claudine.lifecycle.blocked",
            Self::Event(LifecycleSignal::Failure) => "claudine.lifecycle.failure",
            Self::Event(LifecycleSignal::Finalize) => "claudine.lifecycle.finalize",
            Self::Event(LifecycleSignal::Loop) => "claudine.lifecycle.loop",
            Self::TaskSetup => "claudine.task.setup",
            Self::TaskTeardown => "claudine.task.teardown",
            Self::LifecycleShellApproval => "claudine.approval.lifecycle-shell",
            Self::SequenceShellApproval => "claudine.approval.sequence-shell",
        }
    }

    fn is_approval(self) -> bool {
        matches!(self, Self::LifecycleShellApproval | Self::SequenceShellApproval)
    }

    fn carries_error(self) -> bool {
        match self {
            Self::Event(signal) => signal.can_carry_error(),
            Self::TaskTeardown => true,
            Self::TaskSetup | Self::LifecycleShellApproval | Self::SequenceShellApproval => false,
        }
    }
}

/// What is known about the enclosing sequence group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMembership {
    /// Inside an established group.
    Member,
    /// Known to be outside any group.
    Outside,
    /// Not knowable yet: a reusable document checked before invocation.
    Unknown,
}

fn reason(code: &str) -> UnavailabilityReason {
    UnavailabilityReason::new(code)
}

/// The scope's declaration of every lifecycle global.
pub fn binding_view(scope: LifecycleScope, group: GroupMembership) -> Arc<BindingView> {
    let builder = BindingView::builder(ScopeId::new(scope.id()));
    let builder = if scope.is_approval() {
        let preflight = || Availability::Unavailable(reason(PREFLIGHT_UNAVAILABLE));
        let builder = builder
            .declare("err", "The active failure.", preflight())
            .declare("timing", "Observed durations.", preflight())
            .declare("group", "The enclosing group's variables.", preflight());
        if scope == LifecycleScope::SequenceShellApproval {
            builder.declare("outputs", "Earlier steps' outputs.", preflight())
        } else {
            builder
        }
    } else {
        let err = if scope.carries_error() {
            Availability::Available
        } else {
            Availability::Unavailable(reason(EVENT_HAS_NO_ERROR))
        };
        let group = match group {
            GroupMembership::Member => Availability::Available,
            GroupMembership::Outside => Availability::Unavailable(reason(OUTSIDE_GROUP)),
            GroupMembership::Unknown => Availability::ExecutionDependent,
        };
        builder
            .declare("err", "The active failure; `null` when there is none.", err)
            .declare("timing", "Observed durations.", Availability::Available)
            .declare("group", "The enclosing group's variables.", group)
    };
    Arc::new(
        builder
            .build()
            .expect("the lifecycle catalog declares single, unreserved roots once each"),
    )
}

/// The values one evaluation in a lifecycle scope can read.
#[derive(Debug, Clone, Copy, Default)]
pub struct LifecycleValues<'a> {
    /// The active failure, when the scope has one.
    pub err: Option<&'a LifecycleErrorInfo>,
    /// Durations captured so far.
    pub timing: Option<&'a LifecycleTiming>,
    /// The enclosing group's variables; `None` outside any group.
    pub group: Option<&'a Map<String, Value>>,
}

/// The scope's view and one runtime entry per declared global.
///
/// Every declared global gets an entry; unavailability is an explicit entry,
/// never an omission. `err` in an error-carrying scope with no error, and
/// `timing` when nothing was measured, are an explicit `null`.
pub fn runtime_bindings(
    scope: LifecycleScope,
    values: LifecycleValues<'_>,
) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
    let membership = if values.group.is_some() {
        GroupMembership::Member
    } else {
        GroupMembership::Outside
    };
    let view = binding_view(scope, membership);
    let entries = view
        .globals()
        .map(|global| {
            let entry = match global.availability() {
                Availability::Unavailable(reason) => RuntimeBinding::unavailable(reason.clone()),
                Availability::Available | Availability::ExecutionDependent => {
                    RuntimeBinding::eager(match global.root() {
                        "err" => values.err.map_or(Value::Null, LifecycleErrorInfo::to_value),
                        "timing" => values.timing.map_or(Value::Null, LifecycleTiming::to_value),
                        "group" => values
                            .group
                            .map_or(Value::Null, |group| Value::Object(group.clone())),
                        other => unreachable!("undeclared lifecycle global `{other}`"),
                    })
                }
            };
            (global.root().to_string(), entry)
        })
        .collect();
    (view, entries)
}

/// Every reason the authored shell text `raw` cannot have its bytes fixed in
/// approval `scope`, in source order.
///
/// Darkmatter's passive validation reports each definitely unavailable global
/// and unknown function in every branch; each read of a late namespace
/// (`current`, `current_env`) follows as an unavailable binding.
/// Nothing is evaluated and no provider runs.
///
/// ## Errors
///
/// The first expression in `raw` that does not parse.
pub fn approval_diagnostics(
    raw: &str,
    scope: LifecycleScope,
) -> Result<Vec<ValidationDiagnostic>, PreparationError> {
    debug_assert!(scope.is_approval(), "{scope:?} is not an approval scope");
    let view = binding_view(scope, GroupMembership::Outside);
    let prepared = prepare_value(&Value::String(raw.to_string()), AuthoredMode::InterpolatedValue)?;
    let mut diagnostics = validate_prepared(&prepared, &view);
    for expression in prepared.expressions() {
        for read in static_variable_reads(expression.expr()) {
            if LATE_NAMESPACES.contains(&read.root()) {
                diagnostics.push(ValidationDiagnostic {
                    pointer: expression.pointer().to_string(),
                    span: read.span.clone(),
                    error: unavailable(read.root(), read.path, &view),
                });
            }
        }
    }
    Ok(diagnostics)
}

/// The unavailable-binding error for a late namespace read at approval.
fn unavailable(root: &str, path: &str, view: &BindingView) -> ExpressionError {
    use darkmatter::markdown::compose::expression::{BindingError, UnavailableBinding};
    ExpressionError::Binding(Box::new(BindingError::Unavailable(Box::new(UnavailableBinding {
        root: root.to_string(),
        path: path.to_string(),
        scope: view.scope().clone(),
        reason: reason(PREFLIGHT_UNAVAILABLE),
        span: None,
    }))))
}

/// The global root an unavailable-binding diagnostic names, if it is one.
pub fn unavailable_root(error: &ExpressionError) -> Option<&str> {
    use darkmatter::markdown::compose::expression::BindingError;
    match error {
        ExpressionError::Binding(binding) => match binding.as_ref() {
            BindingError::Unavailable(read) => Some(&read.root),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests;
