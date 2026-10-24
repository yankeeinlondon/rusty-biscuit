//! The call binder: binds a function call's evaluated arguments to the
//! function's declared signatures through the one coercion engine
//! ([`crate::markdown::coercion`]) before any handler runs.
//!
//! - [`translate`] compiles SimplifiedSchema grammar types into coercion
//!   shapes, meaning exactly what the JSON Schema lowering means.
//! - A [`FunctionRegistration`] holds a function's overloads, each compiled
//!   once into a [`CompiledSignature`], plus the facts a signature cannot
//!   express: [`EvaluationMode`], registered source exclusions, and
//!   [`NumericRecovery`].
//! - [`FunctionRegistration::bind_call`] checks arity and every argument and
//!   selects one overload, or reports a [`CallBindingError`]; the handler
//!   reads the result through [`BoundArgs`], whose accessors never parse or
//!   convert. Lazy functions bind through [`LazyCall`].
//! - [`FunctionRegistration::select_overload_with`] binds passively with a
//!   deferral predicate: a value supplied later (a `{{ … }}` template) is
//!   taken as matching, and the overloads it leaves open are reported as an
//!   [`OverloadSelection::Deferred`] rather than as an ambiguity.
//! - [`CompiledSignature::check_return`] holds a handler to its declared
//!   return type without converting the result.
//!
//! The rules are the spec's (`2026-10-28-schema-enhancements`, "Overload
//! Selection", "Type Errors", "Function Contracts Carried from the Charter").
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
//! use serde_json::json;
//!
//! let min = FunctionRegistration::builder("min", "function(parameters(a: number, b: number); returns(number))")
//!     .build()
//!     .unwrap();
//! let args = [json!("4"), json!(5)];
//! let call = min.bind_call(&args).unwrap();
//! assert_eq!(call.args().number(0), Some(4.0));
//! assert!(min.bind_call(&[json!("pear"), json!(5)]).is_err());
//! ```

mod bind;
mod error;
mod registration;
mod signature;
pub mod translate;

pub use bind::{BoundArgs, BoundCall, LazyCall, OverloadSelection, Recovery};
pub use error::{
    ArgumentError, ArgumentProblem, CallBindingError, CallErrorKind, CandidateFailure, ContractViolation, SignatureProblem,
};
pub use registration::{EvaluationMode, FunctionRegistration, NumericRecovery, RegistrationBuilder, RegistrationError};
pub use signature::{CompiledParameter, CompiledSignature};
pub use translate::{
    NoNamedTypes, PropertyShape, TranslateError, TypeResolver, property_shape_from_def,
    property_shape_from_def_with_definitions, shape_from_property_def, shape_from_property_def_with_definitions,
    shape_from_simplified_schema, shape_from_simplified_schema_with_definitions,
};
