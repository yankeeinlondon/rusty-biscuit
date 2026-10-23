//! Value constraints on request inputs that no schema component carries.
//!
//! [`Constraints`] holds JSON Schema validation keywords for a parameter
//! ([`ParamDef::constraints`](crate::ParamDef::constraints)), a form field
//! ([`FormField::constraints`](crate::FormField::constraints)), or an inline
//! request body root
//! ([`EndpointBuilder::request_constraints`](crate::EndpointBuilder::request_constraints)).
//! A registered model's rules live on its own fields as Schemars attributes
//! and are not repeated here.
//!
//! Every rule is checked at API build and again when a serialized definition
//! is loaded; a rule that contradicts itself or its value's type is a
//! [`DefinitionError::InvalidConstraint`](crate::DefinitionError::InvalidConstraint).
//! See `schematic/docs/value-constraints.md`.
//!
//! ## Examples
//!
//! ```
//! use schematic_define::{ParamDef, QueryParamType};
//! use schematic_define::constraints::Constraints;
//!
//! let limit = ParamDef::new("limit", QueryParamType::Integer)
//!     .constraints(Constraints::new().minimum(1).maximum(100));
//! let tags = ParamDef::new("tags", QueryParamType::Array(Box::new(QueryParamType::String)))
//!     .constraints(Constraints::new().max_items(10).unique_items().items(Constraints::new().min_length(1)));
//! assert_eq!(limit.constraints.unwrap().maximum.unwrap().as_str(), "100");
//! assert!(tags.constraints.unwrap().items.is_some());
//! ```

mod check;
mod number;
mod pattern;
mod registered;
mod schema;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use check::{ConstraintProblem, constraint_problems};
pub(crate) use check::endpoint_problems;
pub use number::{ConstraintNumber, MAX_EXPONENT};
pub use pattern::{PatternAdmission, admit_pattern};
pub use registered::*;
pub use schema::{EXACT_EXTENSION, NUMERIC_KEYWORDS, exact_number, json_number};
#[cfg(feature = "openapi")]
pub(crate) use schema::{is_data_keyword, mark_inexact_numbers, normalize_exclusive_flags, set_number};

/// JSON Schema validation keywords for one value.
///
/// Absent keywords impose nothing and are omitted when serialized. Rules
/// apply only to values of their own JSON kind: string rules to strings,
/// numeric rules to numbers, array rules (and [`Self::items`]) to arrays.
/// `allowed_values` (`enum`) and `constant` (`const`) compare whole JSON
/// values, and a JSON `null` among them is data, not absence.
///
/// Construct with [`Constraints::new`] and the fluent methods; the struct
/// cannot be written as a literal outside this crate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Constraints {
    /// `minLength`, in Unicode code points.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u64>,
    /// `maxLength`, in Unicode code points.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u64>,
    /// `pattern`, an ECMAScript regular expression that searches unless
    /// anchored. Enforced only within the subset [`admit_pattern`] accepts.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// The pattern is preserved and exported but never enforced; set by
    /// [`Self::unenforced_pattern`] for a source pattern outside the
    /// enforced subset.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pattern_unenforced: bool,
    /// `format`, an annotation only: nothing rejects a value for its format.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// `minimum`, inclusive.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub minimum: Option<ConstraintNumber>,
    /// `maximum`, inclusive.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub maximum: Option<ConstraintNumber>,
    /// `exclusiveMinimum` as a number (JSON Schema 2020-12 meaning).
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<ConstraintNumber>,
    /// `exclusiveMaximum` as a number (JSON Schema 2020-12 meaning).
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<ConstraintNumber>,
    /// `multipleOf`; must be positive.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<ConstraintNumber>,
    /// `minItems`.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u64>,
    /// `maxItems`.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,
    /// `uniqueItems`; `false` is the same as absent.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unique_items: bool,
    /// `enum`: the value must equal one member. Must not be empty.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<Value>>,
    /// `const`: the value must equal this one; `null` is a legitimate value.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub constant: Option<Value>,
    /// Rules for every element of an array value (`items`).
    #[serde(default, deserialize_with = "crate::serde_presence::non_null_map", skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Constraints>>,
}

impl Constraints {
    /// No rules.
    pub fn new() -> Self { Self::default() }

    /// Whether no keyword is present.
    pub fn is_empty(&self) -> bool { self == &Self::default() }

    /// Strings must have at least `count` code points.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().min_length(1).min_length, Some(1));
    /// ```
    pub fn min_length(mut self, count: u64) -> Self { self.min_length = Some(count); self }

    /// Strings must have at most `count` code points.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().max_length(100).max_length, Some(100));
    /// ```
    pub fn max_length(mut self, count: u64) -> Self { self.max_length = Some(count); self }

    /// Strings must contain a match of `source`, which must lie in the
    /// enforced subset ([`admit_pattern`]); anchor it with `^`/`$` to match
    /// the whole string.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// let rules = Constraints::new().pattern(r"^[a-z][a-z0-9-]*$");
    /// assert!(!rules.pattern_unenforced);
    /// ```
    pub fn pattern(mut self, source: impl Into<String>) -> Self {
        self.pattern = Some(source.into()); self.pattern_unenforced = false; self
    }

    /// Records a pattern that is kept for export and documentation but not
    /// enforced, such as an imported pattern outside the enforced subset.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// let rules = Constraints::new().unenforced_pattern(r"^\p{Lu}");
    /// assert!(rules.pattern_unenforced);
    /// ```
    pub fn unenforced_pattern(mut self, source: impl Into<String>) -> Self {
        self.pattern = Some(source.into()); self.pattern_unenforced = true; self
    }

    /// Annotates the value's format (`date-time`, `uuid`, `int64`, ...).
    /// Formats are never enforced.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().format("uuid").format.as_deref(), Some("uuid"));
    /// ```
    pub fn format(mut self, name: impl Into<String>) -> Self { self.format = Some(name.into()); self }

    /// Numbers must be at least `limit`.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().minimum(1).minimum.unwrap().as_str(), "1");
    /// ```
    pub fn minimum(mut self, limit: impl Into<ConstraintNumber>) -> Self { self.minimum = Some(limit.into()); self }

    /// Numbers must be at most `limit`. Pass text for a bound beyond
    /// `i64`/`u64` or a decimal a float cannot hold exactly.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// let rules = Constraints::new().maximum("18446744073709551616");
    /// assert_eq!(rules.maximum.unwrap().as_str(), "18446744073709551616");
    /// ```
    pub fn maximum(mut self, limit: impl Into<ConstraintNumber>) -> Self { self.maximum = Some(limit.into()); self }

    /// Numbers must be greater than `limit`.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().exclusive_minimum(0).exclusive_minimum.unwrap().as_str(), "0");
    /// ```
    pub fn exclusive_minimum(mut self, limit: impl Into<ConstraintNumber>) -> Self { self.exclusive_minimum = Some(limit.into()); self }

    /// Numbers must be less than `limit`.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().exclusive_maximum(1.5).exclusive_maximum.unwrap().as_str(), "1.5");
    /// ```
    pub fn exclusive_maximum(mut self, limit: impl Into<ConstraintNumber>) -> Self { self.exclusive_maximum = Some(limit.into()); self }

    /// Numbers must be an integral multiple of the positive `divisor`,
    /// compared exactly (`0.3` is a multiple of `"0.1"`).
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().multiple_of("0.1").multiple_of.unwrap().as_str(), "0.1");
    /// ```
    pub fn multiple_of(mut self, divisor: impl Into<ConstraintNumber>) -> Self { self.multiple_of = Some(divisor.into()); self }

    /// Arrays must have at least `count` items, checked before a parameter
    /// array is joined or exploded.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().min_items(1).min_items, Some(1));
    /// ```
    pub fn min_items(mut self, count: u64) -> Self { self.min_items = Some(count); self }

    /// Arrays must have at most `count` items.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().max_items(10).max_items, Some(10));
    /// ```
    pub fn max_items(mut self, count: u64) -> Self { self.max_items = Some(count); self }

    /// Array items must be pairwise distinct by deep JSON equality.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert!(Constraints::new().unique_items().unique_items);
    /// ```
    pub fn unique_items(mut self) -> Self { self.unique_items = true; self }

    /// The value must equal one of `values` (`enum`).
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// let rules = Constraints::new().allowed_values(["asc", "desc"]);
    /// assert_eq!(rules.allowed_values.unwrap().len(), 2);
    /// ```
    pub fn allowed_values<I, V>(mut self, values: I) -> Self where I: IntoIterator<Item = V>, V: Into<Value> {
        self.allowed_values = Some(values.into_iter().map(Into::into).collect()); self
    }

    /// The value must equal `value` (`const`).
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// assert_eq!(Constraints::new().constant("v2").constant, Some("v2".into()));
    /// ```
    pub fn constant(mut self, value: impl Into<Value>) -> Self { self.constant = Some(value.into()); self }

    /// Rules every array item must satisfy.
    ///
    /// ```
    /// # use schematic_define::constraints::Constraints;
    /// let rules = Constraints::new().items(Constraints::new().maximum(9));
    /// assert_eq!(rules.items.unwrap().maximum.unwrap().as_str(), "9");
    /// ```
    pub fn items(mut self, rules: Constraints) -> Self { self.items = Some(Box::new(rules)); self }
}
