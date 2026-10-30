//! The result of strict and soft validation
//! ([`crate::prepared::Prepared::validate_strict`] and
//! [`crate::prepared::Prepared::validate_soft`]).

use std::borrow::Cow;

use serde_json::Value;

use crate::{SchemaAdvisory, ValidationProblem};

/// An accepted value and the non-fatal findings beside it.
#[derive(Debug, Clone)]
pub struct ValidationSuccess<'input> {
    /// The accepted effective value: the caller's own input, borrowed, when
    /// it was accepted as written; the converted value, owned, when soft
    /// validation had to convert it. The input itself is never changed.
    pub value: Cow<'input, Value>,
    /// Non-fatal findings from resolving the schema.
    pub advisories: Vec<SchemaAdvisory>,
}

impl ValidationSuccess<'_> {
    /// Whether the accepted value is a conversion of the input rather than
    /// the input itself.
    pub fn is_converted(&self) -> bool {
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
    /// ([`ValidationProblem::is_unavailable_check`]), so the value is
    /// neither accepted nor shown to be invalid.
    UnavailableCheck {
        /// The problems of the checks that could not run.
        problems: Vec<ValidationProblem>,
    },
}

impl ValidationFailure {
    /// Classifies a non-empty problem list.
    pub(crate) fn from_problems(problems: Vec<ValidationProblem>) -> Self {
        if problems.iter().all(ValidationProblem::is_unavailable_check) {
            Self::UnavailableCheck { problems }
        } else {
            Self::InvalidData { problems }
        }
    }

    /// The structured problems accumulated before failing.
    pub fn problems(&self) -> &[ValidationProblem] {
        match self {
            Self::InvalidData { problems } | Self::UnavailableCheck { problems } => problems,
        }
    }

    /// [`Self::problems`], owned.
    pub fn into_problems(self) -> Vec<ValidationProblem> {
        match self {
            Self::InvalidData { problems } | Self::UnavailableCheck { problems } => problems,
        }
    }
}
