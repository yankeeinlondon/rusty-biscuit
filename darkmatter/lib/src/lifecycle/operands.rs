//! Passive validation of an action's operands against a function's
//! registration, before any event runs.
//!
//! An operand written as a `{{ … }}` template has no value until event time,
//! so it is deferred: taken as matching whatever its parameter (or the
//! structure around it) declares. Every other operand, including a literal
//! sibling of a deferred one, is bound exactly as the runtime binds it.
//! Arity and missing required parameters are never deferred, and a template
//! whose span does not parse is judged as the literal text it is (see
//! [`is_deferred_template`](crate::markdown::compose::expression::is_deferred_template)).
//!
//! Validation never evaluates an expression and never runs a handler: it
//! uses only the template scanner, the expression parser, and the call
//! binder.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::lifecycle::{OperandVerdict, validate_expression_function_operands};
//! use serde_json::json;
//!
//! let verdict = validate_expression_function_operands("pr_list", &[json!("{{ fm.count }}")]);
//! assert_eq!(verdict, Some(OperandVerdict::Deferred { overloads: vec![0, 1] }));
//!
//! let verdict = validate_expression_function_operands("pr_list", &[json!(101)]).unwrap();
//! assert!(matches!(verdict, OperandVerdict::Rejected(_)));
//! ```

use serde_json::Value;

use crate::markdown::coercion::{BindOptions, Defer};
use crate::markdown::compose::expression::call_binding::{
    BoundCall, CallBindingError, FunctionRegistration, OverloadSelection,
};
use crate::markdown::compose::expression::functions::function_registration;
use crate::markdown::compose::expression::defer_templates;

/// What passive validation concluded about a call's operands.
#[derive(Debug, Clone, PartialEq)]
pub enum OperandVerdict {
    /// Every operand is known and the call binds to this overload, as it
    /// will at event time.
    Bound { overload: usize },
    /// Some operand is deferred and nothing known fails. These overloads, in
    /// registration order, remain possible; the runtime chooses among them
    /// once the deferred values exist.
    Deferred { overloads: Vec<usize> },
    /// The call cannot bind whatever the deferred operands turn out to be,
    /// or (with nothing deferred) it would fail at event time.
    /// [`CallBindingError::argument_errors`] lists every failing argument
    /// with its index and failing paths.
    Rejected(CallBindingError),
}

/// Validates `args` against `registration`, deferring `{{ … }}` templates.
///
/// Pass the runtime registration ([`function_registration`] for an
/// expression function) so the registry's source exclusions and numeric
/// recoveries apply.
pub fn validate_operands(registration: &FunctionRegistration, args: &[Value]) -> OperandVerdict {
    validate_operands_with(registration, args, &defer_templates)
}

/// [`validate_operands`] with a caller-chosen deferral predicate, which sees
/// each value's path relative to its argument.
pub fn validate_operands_with(registration: &FunctionRegistration, args: &[Value], defer: &Defer<'_>) -> OperandVerdict {
    let options = BindOptions {
        defer: Some(defer),
        ..BindOptions::default()
    };
    match registration.select_overload_with(args, &options) {
        Ok(OverloadSelection::Selected(call)) => OperandVerdict::Bound {
            overload: call.overload(),
        },
        Ok(OverloadSelection::Deferred(calls)) => OperandVerdict::Deferred {
            overloads: calls.iter().map(BoundCall::overload).collect(),
        },
        Err(error) => OperandVerdict::Rejected(error),
    }
}

/// [`validate_operands`] against the runtime registration of the expression
/// function `verb` (canonical name or alias, any case); `None` when no
/// expression function has that name.
pub fn validate_expression_function_operands(verb: &str, args: &[Value]) -> Option<OperandVerdict> {
    function_registration(verb).map(|registration| validate_operands(registration, args))
}
