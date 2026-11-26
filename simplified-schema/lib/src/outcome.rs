//! The result of strict and soft validation
//! ([`crate::prepared::Prepared::validate_strict`] and
//! [`crate::prepared::Prepared::validate_soft`]).

use std::borrow::Cow;
use std::sync::Arc;

use serde_json::Value;

use crate::defaults::AppliedDefault;
use crate::{SchemaAdvisory, SchemaError, ValidationProblem};

/// An accepted value and the non-fatal findings beside it.
#[derive(Debug, Clone)]
pub struct ValidationSuccess<'input> {
    /// The accepted effective value: the caller's own input, borrowed, when
    /// it was accepted as written, and always after strict validation; an
    /// owned copy when soft validation converted a value, inserted a
    /// default, or both. The input itself is never changed.
    pub value: Cow<'input, Value>,
    /// Non-fatal findings from resolving the schema.
    pub advisories: Vec<SchemaAdvisory>,
    /// The defaults default preparation applied: each missing or `null`
    /// property, required or not, that declares a `default(...)`, in walk
    /// order (see [`crate::defaults`]). Empty when no default applies, which
    /// may still be a value accepted only after conversion. Soft validation
    /// has applied them to `value`; strict validation leaves `value` as the
    /// input, so the caller decides whether to apply them.
    pub defaults: Vec<AppliedDefault>,
    /// Near-miss key advice on the accepted value
    /// ([`crate::ValidationProblemCode::NearMissKey`]): never a failure, so it
    /// is reported beside an accepted value.
    pub key_advice: Vec<ValidationProblem>,
}

impl ValidationSuccess<'_> {
    /// Whether the accepted value is a modified copy of the input (a
    /// conversion, an inserted default, or both) rather than the input
    /// itself.
    pub fn is_modified(&self) -> bool {
        matches!(self.value, Cow::Owned(_))
    }
}

/// Why a value was not accepted. Neither variant carries a value: a failure
/// never offers a partially converted one.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ValidationFailure {
    /// The value violates the schema: at least one problem is a decided
    /// violation, whatever else could not be checked.
    InvalidData {
        /// Every problem found, in validator order.
        problems: Vec<ValidationProblem>,
    },
    /// Validation could not be completed: every problem comes from a check
    /// the schema requires and this build cannot run
    /// ([`ValidationProblem::is_unavailable_check`]), or a file check of a
    /// schema built from in-memory content, which has no file context to
    /// resolve in. The value is neither accepted nor shown to be invalid.
    UnavailableCheck {
        /// The problems of the checks that could not run.
        problems: Vec<ValidationProblem>,
    },
    /// The schema could not be prepared for validation, so no value was
    /// judged: it does not lower to JSON Schema (a function-typed property),
    /// `jsonschema` rejects the lowered form, or it names a file an in-memory
    /// schema cannot read. Validation of every value fails the same way until
    /// the schema changes. Also a defect in the schema's defaults: different
    /// defaults for one property across union arms (every value fails),
    /// different defaults from the patterns or catch-alls selecting a key the
    /// value needed a default for, or a default the value needed that breaks
    /// its own definition (see [`crate::defaults`]).
    Unprepared {
        /// Why preparation failed; shared, so repeated validations report the
        /// one error the schema holds.
        error: Arc<SchemaError>,
    },
}

impl ValidationFailure {
    /// Classifies a non-empty problem list. `file_checks` is whether file
    /// values could be resolved at all; without it, file checks are
    /// unavailable rather than violated.
    pub(crate) fn from_problems(problems: Vec<ValidationProblem>, file_checks: bool) -> Self {
        let unavailable = |problem: &ValidationProblem| {
            problem.is_unavailable_check() || (!file_checks && problem.is_file_check())
        };
        if problems.iter().all(unavailable) {
            Self::UnavailableCheck { problems }
        } else {
            Self::InvalidData { problems }
        }
    }

    /// The structured problems accumulated before failing; empty for
    /// [`Self::Unprepared`], which judged no value.
    pub fn problems(&self) -> &[ValidationProblem] {
        match self {
            Self::InvalidData { problems } | Self::UnavailableCheck { problems } => problems,
            Self::Unprepared { .. } => &[],
        }
    }

    /// [`Self::problems`], owned.
    pub fn into_problems(self) -> Vec<ValidationProblem> {
        match self {
            Self::InvalidData { problems } | Self::UnavailableCheck { problems } => problems,
            Self::Unprepared { .. } => Vec::new(),
        }
    }
}
