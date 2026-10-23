//! Darkmatter's one coercion engine: the rule core that binds a runtime value
//! to a target [`Shape`].
//!
//! For every value the shape declares, the engine takes exactly one action:
//! **keep** it when it already matches the type and constraints, **convert**
//! it with one direct conversion and check the result, or report a **type
//! error**. Constraints are enforced, never repaired; conversions never
//! chain; the original value is never mutated; and every failing path is
//! reported, not only the first.
//!
//! The conversions are number → text, boolean → text, text → number, text →
//! boolean, and a native non-null value → `json`/`yaml` text. Unions keep an
//! exact match, else take the one reachable option, else apply the two
//! tie-breakers (number over boolean, JSON over YAML) field by field, else
//! fail naming every reading.
//!
//! The core knows nothing about SimplifiedSchema or JSON Schema syntax:
//! translators build shapes, and the engine alone decides what happens to a
//! value. The rules were decided in `2026-10-28-schema-enhancements`
//! ("The Coercion Engine").
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::coercion::{bind, Shape};
//! use serde_json::json;
//!
//! let value = json!(["1", "2"]);
//! let bound = bind(&Shape::array(Shape::number()), &value).unwrap();
//! assert_eq!(bound.value(), &json!([1, 2]));
//! assert!(bind(&Shape::number(), &json!("pear")).is_err());
//! ```

mod engine;
mod error;
mod shape;
mod union;

pub(crate) use shape::compact;

use serde_json::Value;

pub use error::{
    Constraint, ConversionError, FailureReason, OptionFailure, PathSegment, ValueFailure, ValueKind, render_path,
};
pub use shape::{
    DESCRIPTION_LIMIT,    Additional, ArrayShape, Assertion, ContentFormat, IntegerRule, KeyMatcher, NumberShape, NumericBounds,
    ObjectShape, PatternScope, PatternSlot, PropertySlot, Shape, TextPattern, TextShape, TupleShape, TupleSlot,
};

pub use engine::content_error;

use engine::{Engine, Path, Step};

/// The deepest value nesting the engine walks by default; deeper values fail
/// with [`FailureReason::ResourceLimit`] rather than being accepted.
pub const MAX_DEPTH: usize = 64;

/// A successfully bound value.
#[derive(Debug, Clone, PartialEq)]
pub enum Bound<'v> {
    /// The original value already matched; nothing was copied.
    Kept(&'v Value),
    /// A converted copy; the original is unchanged.
    Converted(Value),
}

impl Bound<'_> {
    pub fn value(&self) -> &Value {
        match self {
            Self::Kept(value) => value,
            Self::Converted(value) => value,
        }
    }

    pub fn into_value(self) -> Value {
        match self {
            Self::Kept(value) => value.clone(),
            Self::Converted(value) => value,
        }
    }

    pub fn is_kept(&self) -> bool {
        matches!(self, Self::Kept(_))
    }
}

/// The deferral predicate of [`BindOptions`]: the path is relative to the
/// bound value.
pub type Defer = dyn Fn(&[PathSegment], &Value) -> bool;

/// Options for [`bind_with`].
#[derive(Clone, Copy)]
pub struct BindOptions<'a> {
    /// A value at a path for which this returns `true` is treated as an exact
    /// match of whatever shape is there (frontmatter keeps shell-pending
    /// `$(...)` values for later re-validation).
    pub defer: Option<&'a Defer>,
    pub max_depth: usize,
}

impl Default for BindOptions<'_> {
    fn default() -> Self {
        Self {
            defer: None,
            max_depth: MAX_DEPTH,
        }
    }
}

impl std::fmt::Debug for BindOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindOptions")
            .field("defer", &self.defer.is_some())
            .field("max_depth", &self.max_depth)
            .finish()
    }
}

/// Binds `value` to `shape` with the default options.
///
/// ## Errors
///
/// A [`ConversionError`] holding every failing path.
pub fn bind<'v>(shape: &Shape, value: &'v Value) -> Result<Bound<'v>, ConversionError> {
    bind_with(shape, value, &BindOptions::default())
}

/// Binds `value` to `shape`: kept, converted, or a type error.
///
/// ## Errors
///
/// A [`ConversionError`] holding every failing path.
pub fn bind_with<'v>(shape: &Shape, value: &'v Value, options: &BindOptions<'_>) -> Result<Bound<'v>, ConversionError> {
    let engine = Engine {
        defer: options.defer,
        max_depth: options.max_depth,
        convert: true,
    };
    match engine.walk(shape, value, Path::root()) {
        Ok(Step::Kept) => Ok(Bound::Kept(value)),
        Ok(Step::Converted(converted, _)) => Ok(Bound::Converted(converted)),
        Err(failures) => Err(ConversionError { failures }),
    }
}

/// Checks that `value` matches `shape` without any conversion, for contracts
/// such as a function's declared return type.
///
/// ## Errors
///
/// A [`ConversionError`] holding every failing path; a value that would bind
/// only after a conversion fails with [`FailureReason::ConversionNotAllowed`].
pub fn check_exact(shape: &Shape, value: &Value) -> Result<(), ConversionError> {
    let engine = Engine {
        defer: None,
        max_depth: MAX_DEPTH,
        convert: false,
    };
    match engine.walk(shape, value, Path::root()) {
        Ok(Step::Kept) => Ok(()),
        // An exact walk performs no conversion.
        Ok(Step::Converted(..)) => Err(ConversionError { failures: Vec::new() }),
        Err(failures) => Err(ConversionError { failures }),
    }
}

/// Whether `value` matches `shape` unchanged; the same answer as
/// [`check_exact`], without building an error.
pub fn accepts_exactly(shape: &Shape, value: &Value) -> bool {
    Engine {
        defer: None,
        max_depth: MAX_DEPTH,
        convert: false,
    }
    .exact(shape, value, Path::root())
}
