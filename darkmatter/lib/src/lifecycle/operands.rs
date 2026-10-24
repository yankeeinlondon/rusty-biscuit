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
//! Schema validation runs this check on every value a generated verb type
//! marks (`operands(<verb>-short)`), reading the value with [`action_call`]
//! and binding it with [`validate_action_operands_with`], so the
//! registration's verdict, not the projected types', decides its literal
//! operands; its predicate never defers the decoded text of a stored literal
//! token, which is data. It also
//! reports a malformed operand template as a syntax problem of its own, since
//! this binder reads one as text. A verb the parser owns itself (`say`,
//! `shell`, `proxy`, `set`, …) has no registration to bind; its marked value
//! is only scanned for malformed templates, at the operands
//! [`template_operands`] names.
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

use super::actions::{
    CommunicationChannel, expression_function_signature, is_lifecycle_control_verb, side_effect_signature,
};
use crate::effects::effect_registration;
use crate::markdown::coercion::{BindOptions, Defer};
use crate::markdown::compose::expression::call_binding::{
    BoundCall, CallBindingError, FunctionRegistration, OverloadSelection,
};
use crate::markdown::compose::expression::functions::{function_registration, inspect_registration};
use crate::markdown::compose::expression::defer_templates;
use crate::markdown::schemas::simplified::OperandForm;

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

/// Where one operand of an action value sits, relative to that value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperandLocation {
    /// The action value itself: the one operand of a scalar short form.
    Whole,
    /// Item `index` of a positional list.
    Index(usize),
    /// The named operand `key` of a long form.
    Key(String),
}

/// An action value read as the call the lifecycle parser makes of it: its
/// operands in positional order, each with its place in the value.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionCall {
    #[allow(missing_docs)]
    pub args: Vec<Value>,
    /// One per [`Self::args`] entry.
    pub locations: Vec<OperandLocation>,
}

/// Reads the `form` spelling of `verb`'s action `value` as the lifecycle
/// parser reads it: in short form `null` is no operand, a list holds the
/// positional operands, and any other scalar is the one operand; in long
/// form the keys other than `no_error` select one overload
/// ([`VerbSignature::select_named`](super::actions::VerbSignature::select_named))
/// and are ordered as its parameters.
///
/// `None` when the value's shape is not `form`, or when a long form's keys
/// select no overload of a generated verb; the schema reports those keys
/// itself, as the parser does.
pub fn action_call(verb: &str, form: OperandForm, value: &Value) -> Option<ActionCall> {
    match (form, value) {
        (OperandForm::Short, Value::Object(_)) | (OperandForm::Long, Value::Null | Value::Array(_)) => None,
        (OperandForm::Short, Value::Null) => Some(ActionCall {
            args: Vec::new(),
            locations: Vec::new(),
        }),
        (OperandForm::Short, Value::Array(items)) => Some(ActionCall {
            args: items.clone(),
            locations: (0..items.len()).map(OperandLocation::Index).collect(),
        }),
        (OperandForm::Short, scalar) => Some(ActionCall {
            args: vec![scalar.clone()],
            locations: vec![OperandLocation::Whole],
        }),
        (OperandForm::Long, Value::Object(map)) => {
            let signature = side_effect_signature(verb).or_else(|| expression_function_signature(verb))?;
            if signature.is_variadic() {
                return None;
            }
            let keys: Vec<&str> = map.keys().map(String::as_str).filter(|key| *key != "no_error").collect();
            let overload = signature.select_named(&keys).ok()?;
            let (args, locations) = overload
                .names()
                .filter_map(|name| map.get(name).map(|operand| (operand.clone(), OperandLocation::Key(name.to_string()))))
                .unzip();
            Some(ActionCall { args, locations })
        }
        (OperandForm::Long, _) => None,
    }
}

/// The spelling `value` is of `verb`: a mapping is the long form, except for
/// `set`, whose only (short) form is a mapping of assignments.
pub fn operand_form(verb: &str, value: &Value) -> OperandForm {
    if value.is_object() && verb != "set" { OperandForm::Long } else { OperandForm::Short }
}

/// One place in a hand-written verb's action value that the lifecycle parser
/// reads through [`action_value_to_expr`](super::action_value_to_expr),
/// which refuses a `{{ … }}` span that does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateOperand {
    #[allow(missing_docs)]
    pub location: OperandLocation,
    /// Whether every string below the operand is read too (`set` values and
    /// a `with:` overlay), or only the operand itself when it is a string
    /// (any other operand: a list or mapping there is refused whole).
    pub nested: bool,
}

/// The operands of the `form` spelling of `verb`'s action `value` that the
/// lifecycle parser reads as templates, for a verb the parser owns itself
/// (communication, `shell`, the flow-control directives, and `set`); `None`
/// for a catalog verb, whose operands [`action_call`] reads.
///
/// Values the parser takes as something else are left out: `no_error`,
/// `break.code`, the removed communication keys, a `set` value that is a
/// whole-value `$( … )` command, and a `break` or `prep` list.
pub fn template_operands(verb: &str, form: OperandForm, value: &Value) -> Option<Vec<TemplateOperand>> {
    let communication = CommunicationChannel::from_verb(verb).is_some();
    if !(communication || verb == "shell" || verb == "set" || is_lifecycle_control_verb(verb)) {
        return None;
    }
    let operand = |location, nested| TemplateOperand { location, nested };
    let operands = match (form, value) {
        (OperandForm::Short, Value::Object(entries)) if verb == "set" => entries
            .iter()
            .filter(|(_, value)| !value.as_str().is_some_and(|text| text.trim_start().starts_with("$(")))
            .map(|(key, _)| operand(OperandLocation::Key(key.clone()), true))
            .collect(),
        (OperandForm::Short, Value::Array(items)) if verb != "set" && !matches!(verb, "break" | "prep") => {
            (0..items.len()).map(|index| operand(OperandLocation::Index(index), false)).collect()
        }
        (OperandForm::Short, Value::String(_)) if verb != "set" => vec![operand(OperandLocation::Whole, false)],
        (OperandForm::Long, Value::Object(entries)) if verb != "set" => entries
            .keys()
            .filter(|key| {
                !(key.as_str() == "no_error"
                    || (verb == "break" && key.as_str() == "code")
                    || (communication && matches!(key.as_str(), "message" | "sound" | "route")))
            })
            .map(|key| operand(OperandLocation::Key(key.clone()), key == "with"))
            .collect(),
        _ => Vec::new(),
    };
    Some(operands)
}

/// [`validate_operands`] of `call` against the registration `verb` binds
/// against at event time: its side-effect registration
/// ([`effect_registration`]), else its expression-function registration (a
/// thread's test function first, as dispatch resolves it). `None` when
/// `verb` has neither.
pub fn validate_action_operands(verb: &str, call: &ActionCall) -> Option<OperandVerdict> {
    validate_action_operands_with(verb, call, &defer_templates)
}

/// [`validate_action_operands`] with a caller-chosen deferral predicate, as
/// [`validate_operands_with`] takes. The binder walks `call.args` in place,
/// so a predicate may recognize a value by its address in them.
pub fn validate_action_operands_with(verb: &str, call: &ActionCall, defer: &Defer<'_>) -> Option<OperandVerdict> {
    match effect_registration(verb) {
        Some(registration) => Some(validate_operands_with(registration, &call.args, defer)),
        None => inspect_registration(verb, |registration| validate_operands_with(registration, &call.args, defer)),
    }
}
