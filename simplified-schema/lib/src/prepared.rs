//! Prepared validation state and the strict/soft validation contract.
//!
//! A [`Prepared`] schema holds what judging a value needs, built once and
//! reused for every value: the lowered JSON Schema, its compiled validators,
//! its conversion plan, and the file anchors its validators resolve in.
//!
//! - [`Prepared::validate_strict`] judges the value as given; nothing is
//!   converted. It is the exact operation a host's raw validation performs.
//! - [`Prepared::validate_soft`] first applies the conversion engine's rules
//!   ([`crate::coerce`]) and judges the result. It is the coercing operation
//!   a host's ordinary validation performs. Conversion never repairs a
//!   violated constraint: `"2.5"` against `number(integer)` fails both ways.
//!
//! Both borrow the input and never change it. On success the accepted value
//! is the input itself, borrowed, unless soft validation converted it.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use jsonschema::Validator;
use serde_json::Value;

use crate::coerce::{self, ConversionPlan, NestedTemplates};
use crate::outcome::{ValidationFailure, ValidationSuccess};
use crate::validate::{self, FileRefAnchors, SHARED_VALIDATOR_CACHE, ValidatorCache};
use crate::{SchemaAdvisory, SchemaError, ValidationProblem, discriminant};

/// A schema's compiled validators.
#[derive(Clone)]
pub enum CompiledValidators {
    /// An ordinary object schema's validator.
    Object(Arc<Validator>),
    /// One validator per arm of a root `anyOf` union. Validation reports per
    /// arm, so the union as a whole is never compiled.
    RootUnion(Vec<Arc<Validator>>),
}

impl CompiledValidators {
    /// Compiles `schema`, or each arm of its root union, through `cache`,
    /// resolving file values from `base_dir` in `context`.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::BuildValidator`] when `jsonschema` rejects the schema.
    pub fn build(
        schema: &Arc<Value>,
        cache: &ValidatorCache,
        base_dir: Option<&Path>,
        context: &biscuit_file::FileResolutionContext,
    ) -> Result<Self, SchemaError> {
        match wrapped_root_arms(schema) {
            Some(arms) => arms
                .iter()
                .map(|arm| cache.validator_for_shared(arm, base_dir, context))
                .collect::<Result<Vec<_>, SchemaError>>()
                .map(Self::RootUnion),
            None => cache.validator_for_shared(schema, base_dir, context).map(Self::Object),
        }
    }

    /// Whether `instance` is accepted unchanged.
    pub fn accepts(&self, instance: &Value) -> bool {
        match self {
            Self::Object(validator) => validator.is_valid(instance),
            Self::RootUnion(arms) => arms.iter().any(|arm| arm.is_valid(instance)),
        }
    }

    /// The problems of `instance` against `json_schema`, the schema these
    /// validators were built from, with discriminated unions narrowed to
    /// their selected arm. Problems carry no line or column.
    pub fn collect_problems(
        &self,
        json_schema: &Value,
        instance: &Value,
        anchors: FileRefAnchors<'_>,
    ) -> Vec<ValidationProblem> {
        match self {
            // Root `anyOf` union. When a shared literal discriminant selects a
            // single arm, report only that arm's problems; otherwise fall back
            // to the closest-matching-arm report.
            Self::RootUnion(arms) => {
                match json_schema
                    .get("anyOf")
                    .and_then(Value::as_array)
                    .and_then(|root_arms| discriminant::select_literal_discriminant_arm(root_arms, instance))
                {
                    Some(idx) => validate::collect_arm_problems_with_anchors(&arms[idx], instance, anchors, idx),
                    None => validate::collect_root_union_problems_with_anchors(arms, instance, anchors),
                }
            }
            // Ordinary object schema. Narrow any discriminated union (property,
            // item, or nested field) to its selected arm; other locations are
            // untouched.
            Self::Object(validator) => {
                let problems = validate::collect_problems_with_anchors(validator, instance, anchors);
                validate::narrow_discriminated_union_problems(json_schema, instance, anchors, problems)
            }
        }
    }
}

/// Sets each problem's declared property description. Whitespace-only
/// descriptions and descriptions identical to the rendered message are left
/// out, so the description only ever adds to the message.
pub fn describe_problems(json_schema: &Value, instance: &Value, problems: &mut [ValidationProblem]) {
    for problem in problems {
        problem.description = validate::resolve_problem_description(json_schema, problem, instance)
            .filter(|desc| !desc.trim().is_empty())
            .filter(|desc| *desc != problem.message);
    }
}

type Wrapped = (Arc<Value>, Arc<[Arc<Value>]>);
const RECENT_SCHEMAS: usize = 8;
/// [`wrapped_root_arms`]'s memo; module-level so [`reset_process_caches`] can clear it.
static WRAPPED: Mutex<Vec<Wrapped>> = Mutex::new(Vec::new());

/// Each arm of `schema`'s root union, wrapped as a root schema, or `None` for
/// a schema that is not a root union.
///
/// Memoized for the process over a few recent schemas (an entry holds its
/// schema alive, so `Arc` identity is a sound key): a root union merged with
/// a large trigger payload has megabyte-sized arms, and each verdict would
/// otherwise copy and rehash them for the validator cache.
pub fn wrapped_root_arms(schema: &Arc<Value>) -> Option<Arc<[Arc<Value>]>> {
    let arms = schema.get("anyOf").and_then(Value::as_array)?;
    let mut wrapped = WRAPPED.lock().expect("root arm memo poisoned");
    if let Some((_, hit)) = wrapped.iter().find(|(held, _)| Arc::ptr_eq(held, schema)) {
        return Some(Arc::clone(hit));
    }
    let arms: Arc<[Arc<Value>]> = arms
        .iter()
        .map(|arm| Arc::new(validate::wrap_arm_as_root_schema(arm, schema)))
        .collect();
    if wrapped.len() == RECENT_SCHEMAS {
        wrapped.remove(0);
    }
    wrapped.push((Arc::clone(schema), Arc::clone(&arms)));
    Some(arms)
}

pub(crate) fn reset_process_caches() {
    WRAPPED.lock().expect("root arm memo poisoned").clear();
}

/// A lowered schema compiled for validation, reused for every value it
/// judges.
pub struct Prepared {
    json_schema: Arc<Value>,
    validators: CompiledValidators,
    conversion: Arc<ConversionPlan>,
    /// The directory a relative file value resolves from; `context`'s `cwd`
    /// when `None`.
    base_dir: Option<PathBuf>,
    context: biscuit_file::FileResolutionContext,
    advisories: Vec<SchemaAdvisory>,
}

impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prepared").field("json_schema", &self.json_schema).finish_non_exhaustive()
    }
}

impl Prepared {
    /// Compiles `json_schema` (a lowered schema, see [`crate::to_json_schema`])
    /// for validation. File values resolve from `base_dir` (or `context`'s
    /// `cwd`) in `context`.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::BuildValidator`] when `jsonschema` rejects the schema.
    pub fn new(
        json_schema: Arc<Value>,
        base_dir: Option<PathBuf>,
        context: biscuit_file::FileResolutionContext,
    ) -> Result<Self, SchemaError> {
        let validators = CompiledValidators::build(&json_schema, &SHARED_VALIDATOR_CACHE, base_dir.as_deref(), &context)?;
        let conversion = ConversionPlan::for_hashed(SHARED_VALIDATOR_CACHE.shared_schema_hash(&json_schema));
        Ok(Self {
            json_schema,
            validators,
            conversion,
            base_dir,
            context,
            advisories: Vec::new(),
        })
    }

    /// Reports `advisories` with every successful validation.
    #[must_use]
    pub fn with_advisories(mut self, advisories: Vec<SchemaAdvisory>) -> Self {
        self.advisories = advisories;
        self
    }

    /// The lowered JSON Schema values are judged against.
    pub fn json_schema(&self) -> &Arc<Value> {
        &self.json_schema
    }

    /// Accepts `value` only if it already matches the schema's types and
    /// constraints; nothing is converted. On success the accepted value is
    /// `value` itself, borrowed.
    ///
    /// ## Errors
    ///
    /// [`ValidationFailure`] with every problem found.
    pub fn validate_strict<'input>(&self, value: &'input Value) -> Result<ValidationSuccess<'input>, ValidationFailure> {
        self.judge(Cow::Borrowed(value))
    }

    /// Accepts `value` as written or after a conversion the conversion
    /// engine permits (the string `"4"` for a `number`). On success the
    /// accepted value is `value` itself, borrowed, when it was accepted as
    /// written, and the converted value, owned, otherwise.
    ///
    /// ## Errors
    ///
    /// [`ValidationFailure`] with every problem left after conversion. A
    /// failure never carries the partially converted value.
    pub fn validate_soft<'input>(&self, value: &'input Value) -> Result<ValidationSuccess<'input>, ValidationFailure> {
        if self.validators.accepts(value) {
            return Ok(self.accepted(Cow::Borrowed(value)));
        }
        let working = coerce::validation_working_copy(&self.conversion, &self.json_schema, value, &NestedTemplates::default());
        self.judge(working)
    }

    /// Accepts `instance` when it has no problems.
    fn judge<'input>(&self, instance: Cow<'input, Value>) -> Result<ValidationSuccess<'input>, ValidationFailure> {
        let problems = self.problems(&instance);
        if problems.is_empty() {
            Ok(self.accepted(instance))
        } else {
            Err(ValidationFailure::from_problems(problems))
        }
    }

    fn accepted<'input>(&self, value: Cow<'input, Value>) -> ValidationSuccess<'input> {
        ValidationSuccess {
            value,
            advisories: self.advisories.clone(),
        }
    }

    fn problems(&self, instance: &Value) -> Vec<ValidationProblem> {
        let anchors = FileRefAnchors::Resolved {
            base_dir: self.base_dir.as_deref(),
            fallback: None,
            context: &self.context,
            excused: None,
        };
        let mut problems = self.validators.collect_problems(&self.json_schema, instance, anchors);
        let none = NestedTemplates::default();
        coerce::annotate_problems(&self.conversion, &self.json_schema, instance, &none, &mut problems);
        describe_problems(&self.json_schema, instance, &mut problems);
        problems
    }
}
