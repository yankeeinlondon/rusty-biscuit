//! Request-side value constraints: the shared checks behind a generated
//! request's `validate()`.
//!
//! A [`Plan`] describes the rules one value must satisfy, as `&'static`
//! data the generator emits: string length and [`Pattern`], exact numeric
//! bounds and `multipleOf`, array item counts and uniqueness, and `enum` and
//! `const` over whole JSON values, with child plans for object properties,
//! map values, array items, and references. The checks append every
//! [`Violation`] they find to a [`Violations`] report:
//!
//! - [`check_value`] for a JSON value, such as a serialized request body;
//! - [`check_str`], [`check_number`], and [`check_array_len`] for typed
//!   parameters, before they are encoded into a query, path, or header.
//!
//! The semantics are JSON Schema's validation vocabulary: lengths count
//! Unicode code points, patterns search unless anchored, numbers compare as
//! exact [`Decimal`]s (`1` equals `1.0`; `0.3` is a multiple of `0.1`), and
//! equality for `enum`, `const`, and `uniqueItems` is JSON equality with
//! object members compared regardless of order. A rule applies only to a
//! value of its own JSON kind; formats are annotations and are not checked
//! here.
//!
//! A violation names a path, a [`Keyword`], and a reason of fixed text and
//! declared limits. It never holds a submitted value, and a map key is shown
//! as [`redact::REDACTED`](crate::redact::REDACTED), so a report is safe to
//! log. A report converts into the [`InvalidArgument`](crate::InvalidArgument)
//! that a refused send returns.
//!
//! ```
//! use schematic_runtime::InvalidArgument;
//! use schematic_runtime::validate::{self, PathSegment, Plan, Property, Violations};
//!
//! static BODY: Plan = Plan::new().properties(&[
//!     Property::new("labels", &Plan::new().values(&Plan::new().max_length(8))),
//!     Property::new("score", &Plan::new().multiple_of("0.5")),
//! ]);
//!
//! let mut violations = Violations::new();
//! let body = serde_json::json!({"labels": {"api-token": "s3cr3t-value"}, "score": 1.25});
//! validate::check_value(&body, &BODY, &[PathSegment::Name("body")], &mut violations);
//!
//! let error = InvalidArgument::from(violations);
//! assert_eq!(
//!     error.reason(),
//!     "2 constraint violations: \
//!      body.labels[[redacted]] violates maxLength: must be at most 8 characters long; \
//!      body.score violates multipleOf: must be a multiple of 0.5"
//! );
//! ```

mod decimal;
mod eval;
mod pattern;
mod plan;
mod violation;

pub use decimal::{Decimal, DecimalError, MAX_EXPONENT};
pub use eval::{Numeric, check_array_len, check_number, check_str, check_value};
pub use pattern::Pattern;
pub use plan::{Literal, Plan, Property};
pub use violation::{Keyword, PathSegment, Violation, Violations};
