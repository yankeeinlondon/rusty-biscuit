//! Frontmatter conversion against a compiled or raw JSON Schema, through the
//! one conversion core ([`crate::coercion`]).
//!
//! The schema is translated into a [`Shape`] by
//! [`super::json_shape::shape_from_json_schema`]; the core alone decides
//! whether each value is kept, converted, or rejected. The complete schema
//! stays authoritative:
//!
//! - an instance the schema already accepts is kept as it is;
//! - a converted candidate is committed when the complete schema accepts it,
//!   unmodeled keywords included (with the shell-pending allowance below);
//! - otherwise each top-level property is its own value: a property whose
//!   conversion succeeds, and on which the complete schema then reports no
//!   problem, is committed, while a failing property stays as written and a
//!   missing required one stays missing (presence is the validator's verdict,
//!   not conversion's). A value is never written back partially. A root
//!   union keeps the whole-document rule, since choosing its arm needs the
//!   whole document.
//!
//! Compose's pre-shell pass passes the top-level keys whose values are still
//! composition-pending (`$(...)`, an unresolved `{{ ... }}`). The core treats
//! such a value as matching whatever its key declares, and a candidate whose
//! only remaining problems are on pending keys is still committed, so resolved
//! siblings convert now and the pending values are judged after expansion.
//!
//! Validation also passes the `{{ … }}` templates nested below the
//! instance's top-level keys at places the schema marks `deferred`: each is
//! treated as matching whatever its path declares, so its literal siblings
//! still convert, and a problem the complete schema reports only because a
//! template is unresolved is excused (see `NestedTemplates::excuses`). A
//! template anywhere else is judged as written. Conversion for write-back
//! passes none.
//!
//! Every entry point is pure: it builds a converted copy and never mutates its
//! inputs. The rules were decided in `2026-10-28-schema-enhancements` ("The
//! Coercion Engine", "Raw JSON Schema Conversion Boundaries").

use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use jsonschema::Validator;
use serde_json::Value;

use super::json_shape::shape_from_json_schema;
use super::simplified::{DARKMATTER_DEFERRED_KEYWORD, DARKMATTER_OPERANDS_KEYWORD};
use super::validate::{self, SchemaIdentity, ValidatorCache, error_top_level_key};
use super::ValidationProblem;
use crate::coercion::{
    ActualKind, Additional, BindOptions, Bound, ConversionError, Defer, ExpectedType, FailureReason, ObjectShape,
    PathSegment, PatternSlot, PropertySlot, RefSlot, Shape, ShapeRef, TupleShape, TupleSlot, ValueFailure, bind_with, json_pointer,
    locate_failures,
};

/// Process-wide cache of the structural validators conversion compiles to
/// judge candidates. Kept apart from the effective-schema validator cache so
/// the many small root-union arm schemas do not churn its LRU budget.
static CONVERSION_VALIDATOR_CACHE: LazyLock<ValidatorCache> = LazyLock::new(ValidatorCache::new);

/// How many conversion plans the plan cache holds before it starts over.
const PLAN_CACHE_CAPACITY: usize = 64;

static PLAN_CACHE: LazyLock<Mutex<HashMap<SchemaIdentity, Arc<ConversionPlan>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Empties this module's process-wide caches (see
/// [`super::reset_process_caches_for_bench`]).
pub(crate) fn reset_process_caches() {
    CONVERSION_VALIDATOR_CACHE.clear();
    PLAN_CACHE.lock().expect("conversion plan cache poisoned").clear();
}

/// What conversion derives from one schema: its translated shape, that shape
/// with nothing required, and its structural validator. A plan is found by
/// the schema's identity: its content hash, or a host's known identity
/// ([`Self::for_identity`]). A host that keeps a schema's plan (such as
/// Darkmatter's `EffectiveSchema`) keeps its plan, so repeated validations of
/// large schemas never hash them again.
pub struct ConversionPlan {
    /// The schema's identity, the key of its structural validator.
    identity: SchemaIdentity,
    shape: OnceLock<Shape>,
    relaxed: OnceLock<Shape>,
    validator: OnceLock<Option<Arc<Validator>>>,
    /// Whether the schema marks any node `deferred`.
    deferral: OnceLock<bool>,
    /// Whether the schema marks any node `x-darkmatter-operands`.
    operands: OnceLock<bool>,
}

impl ConversionPlan {
    /// The plan for `schema`, built once per distinct schema content.
    pub fn for_schema(schema: &Value) -> Arc<Self> {
        Self::for_hashed(validate::schema_content_hash(schema))
    }

    /// [`Self::for_schema`] for a schema whose canonical content hash `key`
    /// is known. Each part is derived on first use, from the same schema.
    pub fn for_hashed(key: u64) -> Arc<Self> {
        Self::for_identity(SchemaIdentity::canonical(key))
    }

    /// [`Self::for_schema`] for a schema whose identity is known, such as
    /// the [`LayeredIdentity`](validate::LayeredIdentity) a host compiled
    /// its validators under, which then also finds its structural validator.
    pub fn for_identity(identity: SchemaIdentity) -> Arc<Self> {
        if let Some(plan) = PLAN_CACHE.lock().expect("plan cache poisoned").get(&identity) {
            return plan.clone();
        }
        let plan = Arc::new(Self {
            identity,
            shape: OnceLock::new(),
            relaxed: OnceLock::new(),
            validator: OnceLock::new(),
            deferral: OnceLock::new(),
            operands: OnceLock::new(),
        });
        let mut cache = PLAN_CACHE.lock().expect("plan cache poisoned");
        if cache.len() >= PLAN_CACHE_CAPACITY {
            cache.clear();
        }
        cache.insert(identity, plan.clone());
        plan
    }

    pub fn shape(&self, schema: &Value) -> &Shape {
        self.shape.get_or_init(|| shape_from_json_schema(schema))
    }

    /// Whether `schema` marks any node `deferred`, so validation has nested
    /// templates to look for.
    pub fn marks_deferral(&self, schema: &Value) -> bool {
        *self.deferral.get_or_init(|| marks_deferral(schema))
    }

    /// Whether `schema` types any value as a generated lifecycle verb's
    /// operands, so validation has calls to bind (a host such as Darkmatter
    /// binds them).
    pub fn marks_operands(&self, schema: &Value) -> bool {
        *self.operands.get_or_init(|| marks_operands(schema))
    }

    fn relaxed(&self, schema: &Value) -> &Shape {
        self.relaxed.get_or_init(|| relax_presence(self.shape(schema)))
    }

    /// The schema's structural validator. Validation compiles the same one
    /// into the shared cache for a schema that judges no file value, so it is
    /// reused from there rather than built a second time.
    fn validator(&self, schema: &Value) -> Option<&Validator> {
        self.validator
            .get_or_init(|| {
                validate::SHARED_VALIDATOR_CACHE
                    .cached_structural(self.identity)
                    .or_else(|| CONVERSION_VALIDATOR_CACHE.structural_validator_identified(schema, self.identity).ok())
            })
            .as_deref()
    }
}

impl std::fmt::Debug for ConversionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConversionPlan").finish_non_exhaustive()
    }
}

fn structural_validator(schema: &Value) -> Option<Arc<Validator>> {
    CONVERSION_VALIDATOR_CACHE.structural_validator_for(schema).ok()
}

/// Result of converting a whole instance against a schema.
#[derive(Debug, Clone)]
pub struct CoercionOutcome {
    /// The converted instance (a clone when nothing changed).
    pub value: Value,
    /// Whether any value was converted.
    pub changed: bool,
}

impl CoercionOutcome {
    fn unchanged(instance: &Value) -> Self {
        Self {
            value: instance.clone(),
            changed: false,
        }
    }
}

/// What converting one instance produced.
enum Conversion {
    /// The schema accepts the instance as it is, or the core kept it.
    Kept,
    /// A candidate the complete schema accepts.
    Converted(Value),
    /// The core found a converted reading the complete schema rejects.
    Rejected,
    /// The core could not bind the instance. `diagnostic` is its partial
    /// copy ([`partial_copy`]), which validation reports on; `committed`
    /// keeps only the top-level properties of it the complete schema accepts
    /// ([`property_wise`]), which write-back commits.
    Failed {
        committed: Option<Value>,
        diagnostic: Option<Value>,
    },
    /// The schema does not compile; its validator reports why.
    Unavailable,
}

fn convert(
    plan: &ConversionPlan,
    schema: &Value,
    instance: &Value,
    pending: &HashSet<String>,
    templates: &NestedTemplates,
) -> Conversion {
    let Some(validator) = plan.validator(schema) else {
        return Conversion::Unavailable;
    };
    if validator.is_valid(instance) {
        return Conversion::Kept;
    }
    let shape = plan.shape(schema);
    let defer = |path: &[PathSegment], _: &Value| is_pending(path, pending) || templates.defers(path);
    let options = BindOptions {
        defer: (!pending.is_empty() || !templates.is_empty()).then_some(&defer as &Defer<'_>),
        ..BindOptions::default()
    };
    match bind_with(shape, instance, &options) {
        Ok(Bound::Kept(_)) => Conversion::Kept,
        Ok(Bound::Converted(candidate)) => {
            if accepts(schema, validator, &candidate, pending, templates) {
                Conversion::Converted(candidate)
            } else {
                match property_wise(schema, validator, instance, &candidate, pending, templates, &[]) {
                    Some(committed) => Conversion::Converted(committed),
                    None => Conversion::Rejected,
                }
            }
        }
        Err(error) => {
            let diagnostic = partial_copy(plan, schema, instance, &error, pending, templates);
            let committed = diagnostic.as_ref().and_then(|candidate| {
                property_wise(schema, validator, instance, candidate, pending, templates, &error.failures)
            });
            Conversion::Failed { committed, diagnostic }
        }
    }
}

/// The top-level properties of `candidate` that converted cleanly: each
/// converted property is kept only when the complete schema reports no
/// problem on it that `templates` does not excuse (given the core's
/// `failures` binding the instance), and every other property stays as
/// written. `None` for a root union, a non-object instance, or when nothing
/// converted cleanly.
fn property_wise(
    schema: &Value,
    validator: &Validator,
    instance: &Value,
    candidate: &Value,
    pending: &HashSet<String>,
    templates: &NestedTemplates,
    failures: &[ValueFailure],
) -> Option<Value> {
    if schema.get("anyOf").is_some() || schema.get("oneOf").is_some() {
        return None;
    }
    let (Value::Object(original), Value::Object(converted)) = (instance, candidate) else {
        return None;
    };
    let troubled: HashSet<String> = validator
        .iter_errors(candidate)
        .filter(|error| !templates.excuses(error.instance_path().as_str(), failures))
        .filter_map(|error| error_top_level_key(&error))
        .collect();
    let mut committed = original.clone();
    let mut changed = false;
    for (key, value) in converted {
        if original.get(key) != Some(value) && !troubled.contains(key) && !pending.contains(key) {
            committed.insert(key.clone(), value.clone());
            changed = true;
        }
    }
    changed.then_some(Value::Object(committed))
}

fn is_pending(path: &[PathSegment], pending: &HashSet<String>) -> bool {
    matches!(path, [PathSegment::Key(key)] if pending.contains(key))
}

/// The `{{ … }}` templates nested below the top-level keys of a frontmatter
/// instance that sit where the schema marks a value `deferred`
/// (`x-darkmatter-deferred`, a [`Shape::Deferrable`] node): each is a value
/// that exists only once its event or composition runs. Validation treats
/// each as matching whatever its path declares and never evaluates it. A
/// template anywhere else is ordinary text and is judged as written. A
/// top-level value is never listed here: a whole top-level template keeps its
/// pending-key rule (a host such as Darkmatter applies it when validating).
///
/// [`Self::with_settled`] adds the operands a generated lifecycle verb's
/// runtime registration accepted (as a host such as Darkmatter binds them); they are set
/// aside exactly as a template is, but are not templates and are never
/// reported as [`Self::pointers`].
#[derive(Debug, Default, Clone)]
pub struct NestedTemplates {
    /// Each template's path and its JSON Pointer, then each settled operand's.
    paths: Vec<(Vec<PathSegment>, String)>,
    /// How many leading entries of `paths` are templates.
    templates: usize,
}

impl NestedTemplates {
    /// The templates listed at `paths`, each the path of a `{{ … }}`
    /// template nested below a top-level key at a place the schema marks
    /// `deferred`. Deciding which values are such templates is the host's
    /// pending classification; the host walks the instance with
    /// [`ConversionPlan::shape`] and [`bind_with_marked`](crate::coercion::bind_with_marked) and hands the paths
    /// here.
    pub fn from_template_paths(paths: Vec<Vec<PathSegment>>) -> Self {
        let paths: Vec<_> = paths
            .into_iter()
            .map(|path| {
                let pointer = json_pointer(&path);
                (path, pointer)
            })
            .collect();
        Self {
            templates: paths.len(),
            paths,
        }
    }

    /// These templates plus `settled`, the operands a lifecycle verb's
    /// runtime registration accepted: a problem the schema reports on one of
    /// them, or on a value holding it, is set aside unless the core still
    /// fails there with them all taken as matching.
    pub fn with_settled(&self, settled: &[Vec<PathSegment>]) -> Self {
        let mut widened = self.clone();
        for path in settled {
            if !widened.paths.iter().any(|(known, _)| known == path) {
                widened.paths.push((path.clone(), json_pointer(path)));
            }
        }
        widened
    }

    /// The JSON Pointer of every listed template.
    pub fn pointers(&self) -> Vec<String> {
        self.paths[..self.templates].iter().map(|(_, pointer)| pointer.clone()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Whether the core should treat the value at `path` as matching.
    /// Membership, not reclassification: the value bound may be a decoded
    /// literal token that reads like a template.
    fn defers(&self, path: &[PathSegment]) -> bool {
        self.paths.iter().any(|(deferred, _)| deferred == path)
    }

    /// Whether a problem the complete schema reports at `at` is owed to an
    /// unresolved template rather than to the authored document.
    ///
    /// A problem at a template or below it, or at one of its ancestors (a
    /// union or tuple failure is reported on the holder), is excused unless
    /// `failures`, the core's verdict binding the same instance with every
    /// template deferred, holds a failure inside the shallower of the two
    /// paths. So a misspelled key or extra element beside a template is
    /// still reported, while the template itself never is.
    fn excuses(&self, at: &str, failures: &[ValueFailure]) -> bool {
        self.paths.iter().any(|(_, deferred)| {
            let root = if within(deferred, at) {
                at
            } else if within(at, deferred) {
                deferred.as_str()
            } else {
                return false;
            };
            !self.fails_within(failures, root)
        })
    }

    /// Whether some failure lies inside `root` or rejects a value holding
    /// it. A union failure there counts only when every option fails so, an
    /// option failing only beside `root` being one the value could take; a
    /// complete-schema assertion over a value holding a template is no
    /// verdict, since it judged the template's text.
    fn fails_within(&self, failures: &[ValueFailure], root: &str) -> bool {
        failures.iter().any(|failure| {
            let at = json_pointer(&failure.path);
            if !within(&at, root) && !within(root, &at) {
                return false;
            }
            match &failure.reason {
                FailureReason::AssertionFailed { .. }
                    if self.paths.iter().any(|(_, deferred)| within(deferred, &at)) =>
                {
                    false
                }
                FailureReason::NoOptionAccepts { options } => {
                    options.iter().all(|option| self.fails_within(&option.failures, root))
                }
                _ => true,
            }
        })
    }

    /// Whether `problem`, reported on `instance`, is owed to a template
    /// ([`Self::excuses`]). A missing or undeclared property is located at
    /// the property itself rather than at the holder its path names, so it is
    /// judged by its own place relative to the templates. `failures` caches
    /// the core's verdict, which is bound only once some problem is near a
    /// template.
    pub fn excuses_problem(
        &self,
        problem: &ValidationProblem,
        plan: &ConversionPlan,
        json_schema: &Value,
        instance: &Value,
        failures: &OnceCell<Vec<ValueFailure>>,
    ) -> bool {
        if self.is_empty() {
            return false;
        }
        let at = match problem.property.as_ref().or(problem.offending_property.as_ref()) {
            Some(name) => format!("{}/{}", problem.path, name.replace('~', "~0").replace('/', "~1")),
            None => problem.path.clone(),
        };
        let near = self
            .paths
            .iter()
            .any(|(_, deferred)| within(deferred, &at) || within(&at, deferred));
        near && self.excuses(&at, failures.get_or_init(|| self.failures(plan, json_schema, instance)))
    }

    /// The core's failures binding `instance` to the schema's shape with
    /// every template deferred; empty when it binds.
    fn failures(&self, plan: &ConversionPlan, json_schema: &Value, instance: &Value) -> Vec<ValueFailure> {
        let defer = |path: &[PathSegment], _: &Value| self.defers(path);
        let options = BindOptions {
            defer: Some(&defer),
            ..BindOptions::default()
        };
        bind_with(plan.shape(json_schema), instance, &options).map_or_else(|error| error.failures, |_| Vec::new())
    }
}

/// Whether some node of `schema` carries `x-darkmatter-operands`.
fn marks_operands(schema: &Value) -> bool {
    match schema {
        Value::Object(map) => map.contains_key(DARKMATTER_OPERANDS_KEYWORD) || map.values().any(marks_operands),
        Value::Array(items) => items.iter().any(marks_operands),
        _ => false,
    }
}

/// Whether some node of `schema` carries `x-darkmatter-deferred: true`.
fn marks_deferral(schema: &Value) -> bool {
    match schema {
        Value::Object(map) => {
            map.get(DARKMATTER_DEFERRED_KEYWORD) == Some(&Value::Bool(true)) || map.values().any(marks_deferral)
        }
        Value::Array(items) => items.iter().any(marks_deferral),
        _ => false,
    }
}

/// Whether the JSON Pointer `path` is `ancestor` or lies below it.
fn within(path: &str, ancestor: &str) -> bool {
    path.strip_prefix(ancestor)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// Whether the complete schema accepts a converted candidate: fully, or, when
/// keys are pending or templates nested, with every remaining problem on a
/// pending key or excused by a template. Only a candidate the core bound
/// without failure is judged here, so no failure stands against an excuse. A
/// root union is judged arm by arm, since its own failure names no key.
fn accepts(
    schema: &Value,
    validator: &Validator,
    candidate: &Value,
    pending: &HashSet<String>,
    templates: &NestedTemplates,
) -> bool {
    if validator.is_valid(candidate) {
        return true;
    }
    if pending.is_empty() && templates.is_empty() {
        return false;
    }
    match schema.get("anyOf").and_then(Value::as_array) {
        Some(arms) => arms.iter().any(|arm| {
            structural_validator(&validate::wrap_arm_as_root_schema(arm, schema))
                .is_some_and(|arm_validator| only_set_aside_problems(&arm_validator, candidate, pending, templates))
        }),
        None => only_set_aside_problems(validator, candidate, pending, templates),
    }
}

fn only_set_aside_problems(
    validator: &Validator,
    candidate: &Value,
    pending: &HashSet<String>,
    templates: &NestedTemplates,
) -> bool {
    let mut saw_problem = false;
    for error in validator.iter_errors(candidate) {
        saw_problem = true;
        if templates.excuses(error.instance_path().as_str(), &[]) {
            continue;
        }
        match error_top_level_key(&error) {
            Some(key) if pending.contains(&key) => {}
            _ => return false,
        }
    }
    saw_problem
}

/// Converts `instance` against `json_schema`; see the module docs for when a
/// conversion is committed. Pure: never mutates its inputs.
pub fn coerce_frontmatter(json_schema: &Value, instance: &Value) -> CoercionOutcome {
    coerce_frontmatter_with_pending(json_schema, instance, &HashSet::new())
}

/// Like [`coerce_frontmatter`], for compose's pre-shell stage, where the
/// top-level keys in `shell_pending` still hold composition-pending values.
/// Those values are kept as they are, and a candidate whose only remaining
/// problems are on those keys is committed. An empty set reduces this to
/// [`coerce_frontmatter`].
pub fn coerce_frontmatter_with_pending(
    json_schema: &Value,
    instance: &Value,
    shell_pending: &HashSet<String>,
) -> CoercionOutcome {
    coerce_with_plan(&ConversionPlan::for_schema(json_schema), json_schema, instance, shell_pending)
}

/// [`coerce_frontmatter_with_pending`] with the schema's plan already found.
pub fn coerce_with_plan(
    plan: &ConversionPlan,
    json_schema: &Value,
    instance: &Value,
    shell_pending: &HashSet<String>,
) -> CoercionOutcome {
    match convert(plan, json_schema, instance, shell_pending, &NestedTemplates::default()) {
        Conversion::Converted(value)
        | Conversion::Failed {
            committed: Some(value), ..
        } => CoercionOutcome { value, changed: true },
        _ => CoercionOutcome::unchanged(instance),
    }
}

/// Converts a lone value against a schema that describes the value itself
/// (an `example(...)` artifact's `returns` against its annotated target). A
/// clone of `value` when nothing converts.
pub fn coerce_value_against_schema(schema: &Value, value: &Value) -> Value {
    coerce_frontmatter(schema, value).value
}

/// The working copy validation reports on: the conversion the complete
/// schema accepts when the core binds the instance, otherwise every
/// conversion that succeeds on its own, with each failing value left as
/// written and each unexpected key or tuple element kept, so the report names
/// only real failures.
///
/// Never written back anywhere; compose writes back only what
/// [`coerce_frontmatter_with_pending`] commits.
#[cfg(test)]
pub fn validation_copy(json_schema: &Value, instance: &Value) -> Value {
    validation_copy_with_plan(
        &ConversionPlan::for_schema(json_schema),
        json_schema,
        instance,
        &NestedTemplates::default(),
    )
}

/// The working copy validation reports on, with the schema's plan already
/// found; each value in `templates` is kept as written and does not keep its
/// siblings from converting.
pub fn validation_copy_with_plan(
    plan: &ConversionPlan,
    json_schema: &Value,
    instance: &Value,
    templates: &NestedTemplates,
) -> Value {
    validation_working_copy(plan, json_schema, instance, templates).into_owned()
}

/// [`validation_copy_with_plan`] that borrows `instance` whenever no value
/// converted, so a caller can tell a converted copy from the input without
/// comparing them.
pub(crate) fn validation_working_copy<'input>(
    plan: &ConversionPlan,
    json_schema: &Value,
    instance: &'input Value,
    templates: &NestedTemplates,
) -> Cow<'input, Value> {
    let none = HashSet::new();
    match convert(plan, json_schema, instance, &none, templates) {
        Conversion::Converted(value)
        | Conversion::Failed {
            diagnostic: Some(value), ..
        } => Cow::Owned(value),
        Conversion::Failed { diagnostic: None, .. }
        | Conversion::Kept
        | Conversion::Rejected
        | Conversion::Unavailable => Cow::Borrowed(instance),
    }
}

/// How many times [`partial_copy`] sets newly failing values aside before it
/// gives up.
const PARTIAL_COPY_ROUNDS: usize = 4;

/// Binds again with every failing value treated as already matching and with
/// presence relaxed, so the values that do convert can be reported converted.
///
/// An unexpected object key or tuple element cannot be deferred: the
/// relaxed shape keeps the closed-object and fixed-tuple rules, so it would
/// fail again. Each is masked out of the retry input instead, and put back
/// into the copy at its original place, so judging the copy against the
/// complete schema still reports it while its converted siblings are judged
/// converted. The copy is diagnostic: a caller commits only what the
/// complete schema then accepts.
fn partial_copy(
    plan: &ConversionPlan,
    json_schema: &Value,
    instance: &Value,
    error: &ConversionError,
    pending: &HashSet<String>,
    templates: &NestedTemplates,
) -> Option<Value> {
    let relaxed = plan.relaxed(json_schema);
    let FailingPaths { mut failing, mut unexpected } = failing_paths(&error.failures);
    for _ in 0..PARTIAL_COPY_ROUNDS {
        let masked: Vec<&[PathSegment]> = unexpected.iter().map(Vec::as_slice).collect();
        let retry = without_entries(instance, &masked);
        let set_aside = failing.clone();
        let pending = pending.clone();
        let defer = move |path: &[PathSegment], _: &Value| {
            is_pending(path, &pending) || templates.defers(path) || set_aside.iter().any(|failed| failed == path)
        };
        let options = BindOptions {
            defer: Some(&defer),
            ..BindOptions::default()
        };
        match bind_with(relaxed, &retry, &options) {
            Ok(bound) => return with_entries(bound.into_value(), instance, &masked),
            Err(again) => {
                // Only the root's complete-schema assertion is left: its
                // converted reading is the copy, and the caller judges each
                // property of it against the complete schema.
                if let [ValueFailure {
                    path,
                    converted: Some(candidate),
                    reason: FailureReason::AssertionFailed { .. },
                    ..
                }] = again.failures.as_slice()
                    && path.is_empty()
                {
                    return with_entries(candidate.clone(), instance, &masked);
                }
                let FailingPaths {
                    failing: more_failing,
                    unexpected: more_unexpected,
                } = failing_paths(&again.failures);
                let more_failing: Vec<_> = more_failing.into_iter().filter(|path| !failing.contains(path)).collect();
                let more_unexpected: Vec<_> =
                    more_unexpected.into_iter().filter(|path| !unexpected.contains(path)).collect();
                if more_failing.is_empty() && more_unexpected.is_empty() {
                    return None;
                }
                failing.extend(more_failing);
                unexpected.extend(more_unexpected);
            }
        }
    }
    None
}

/// The paths [`partial_copy`] sets aside, never the root: setting it aside
/// would keep every property as written, and report a value that does
/// convert as if it could not.
struct FailingPaths {
    /// The deepest paths at which a present value failed, looking inside
    /// union arms ([`locate_failures`]): setting a union aside whole would
    /// keep every element under it unconverted, when only one element failed.
    failing: Vec<Vec<PathSegment>>,
    /// Unexpected object keys and tuple elements, in failure order.
    unexpected: Vec<Vec<PathSegment>>,
}

fn failing_paths(failures: &[ValueFailure]) -> FailingPaths {
    let flat = locate_failures(failures);
    let deeper = |path: &[PathSegment]| {
        flat.iter()
            .any(|other| other.failure.path.len() > path.len() && other.failure.path.starts_with(path))
    };
    let mut paths = FailingPaths {
        failing: Vec::new(),
        unexpected: Vec::new(),
    };
    for located in &flat {
        let path = &located.failure.path;
        let list = match located.expected {
            ExpectedType::Absent => &mut paths.unexpected,
            _ => &mut paths.failing,
        };
        if !path.is_empty() && located.actual != ActualKind::Missing && !deeper(path) && !list.contains(path) {
            list.push(path.clone());
        }
    }
    paths
}

/// The entries of `value` whose path (relative to `value`) begins with
/// `segment`, as the rest of each path.
fn entries_below<'p>(masked: &[&'p [PathSegment]], segment: &PathSegment) -> Vec<&'p [PathSegment]> {
    masked
        .iter()
        .filter_map(|path| path.split_first().filter(|(first, _)| *first == segment).map(|(_, rest)| rest))
        .collect()
}

/// `value` without the entries at `masked`. Only unexpected entries are
/// masked, and an unexpected tuple element lies past every declared slot, so
/// no kept element changes index.
fn without_entries(value: &Value, masked: &[&[PathSegment]]) -> Value {
    if masked.is_empty() {
        return value.clone();
    }
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter_map(|(key, item)| {
                    let below = entries_below(masked, &PathSegment::Key(key.clone()));
                    (!below.iter().any(|rest| rest.is_empty())).then(|| (key.clone(), without_entries(item, &below)))
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    let below = entries_below(masked, &PathSegment::Index(index));
                    (!below.iter().any(|rest| rest.is_empty())).then(|| without_entries(item, &below))
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

/// `copy`, a binding of [`without_entries`]`(original, masked)`, with each
/// masked entry of `original` back at its original place and in its original
/// key order. `None` when the copy no longer has the shape to hold one, so a
/// value missing an unexpected entry is never judged as if it were complete.
fn with_entries(copy: Value, original: &Value, masked: &[&[PathSegment]]) -> Option<Value> {
    if masked.is_empty() {
        return Some(copy);
    }
    match (copy, original) {
        (Value::Object(mut converted), Value::Object(map)) => {
            let mut out = serde_json::Map::new();
            for (key, item) in map {
                let below = entries_below(masked, &PathSegment::Key(key.clone()));
                if below.iter().any(|rest| rest.is_empty()) {
                    out.insert(key.clone(), item.clone());
                } else if let Some(value) = converted.remove(key) {
                    out.insert(key.clone(), with_entries(value, item, &below)?);
                }
            }
            out.extend(converted);
            Some(Value::Object(out))
        }
        (Value::Array(converted), Value::Array(items)) => {
            let mut converted = converted.into_iter();
            let mut out = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                let below = entries_below(masked, &PathSegment::Index(index));
                if below.iter().any(|rest| rest.is_empty()) {
                    out.push(item.clone());
                } else {
                    out.push(with_entries(converted.next()?, item, &below)?);
                }
            }
            out.extend(converted);
            Some(Value::Array(out))
        }
        _ => None,
    }
}

/// A copy of `shape` in which nothing is required, so a missing value does
/// not keep its siblings from converting. Recursive targets are copied
/// too, with their back references pointing at the copies.
fn relax_presence(shape: &Shape) -> Shape {
    relax(shape, &mut Vec::new())
}

/// [`relax_presence`]; `targets` pairs each reference target being copied
/// with its copy.
fn relax(shape: &Shape, targets: &mut Vec<(ShapeRef, RefSlot)>) -> Shape {
    match shape {
        Shape::Array(array) => {
            let mut array = array.clone();
            array.items = Box::new(relax(&array.items, targets));
            Shape::Array(array)
        }
        Shape::Tuple(tuple) => Shape::Tuple(TupleShape {
            prefix: tuple
                .prefix
                .iter()
                .map(|slot| TupleSlot {
                    shape: relax(&slot.shape, targets),
                    optional: true,
                })
                .collect(),
            rest: tuple.rest.as_ref().map(|rest| Box::new(relax(rest, targets))),
        }),
        Shape::Object(object) => Shape::Object(ObjectShape {
            properties: object
                .properties
                .iter()
                .map(|property| PropertySlot {
                    name: property.name.clone(),
                    shape: relax(&property.shape, targets),
                    required: false,
                })
                .collect(),
            patterns: object
                .patterns
                .iter()
                .map(|pattern| PatternSlot {
                    matcher: pattern.matcher.clone(),
                    shape: relax(&pattern.shape, targets),
                })
                .collect(),
            pattern_scope: object.pattern_scope,
            additional: match &object.additional {
                Additional::Shape(inner) => Additional::Shape(Box::new(relax(inner, targets))),
                other => other.clone(),
            },
            min_properties: object.min_properties,
            max_properties: object.max_properties,
            uncounted: object.uncounted.clone(),
        }),
        Shape::Union(options) => Shape::Union(options.iter().map(|option| relax(option, targets)).collect()),
        Shape::Assert(inner, assertion) => Shape::Assert(Box::new(relax(inner, targets)), assertion.clone()),
        Shape::Deferrable(inner) => relax(inner, targets).deferrable(),
        Shape::Ref(reference) if reference.is_owner() => match reference.target() {
            Some(target) => {
                targets.push((reference.clone(), RefSlot::new(reference.name())));
                let body = relax(&target, targets);
                let (_, copy) = targets.pop().expect("the target pushed above");
                Shape::Ref(copy.finish(body))
            }
            None => shape.clone(),
        },
        Shape::Ref(reference) => match targets.iter().rev().find(|(original, _)| original.same_target(reference)) {
            Some((_, copy)) => Shape::Ref(copy.back_reference()),
            None => shape.clone(),
        },
        other => other.clone(),
    }
}

/// Adds the core's reason to each problem whose value a conversion was
/// attempted on and failed (`"9007199254740993"` against `number`: "the
/// number cannot be represented exactly"), so the report says why the text
/// did not convert rather than only that its type is wrong.
pub fn annotate_problems(
    plan: &ConversionPlan,
    json_schema: &Value,
    instance: &Value,
    templates: &NestedTemplates,
    problems: &mut [ValidationProblem],
) {
    if problems.is_empty() {
        return;
    }
    let defer = |path: &[PathSegment], _: &Value| templates.defers(path);
    let options = BindOptions {
        defer: (!templates.is_empty()).then_some(&defer as &Defer<'_>),
        ..BindOptions::default()
    };
    let Err(error) = bind_with(plan.shape(json_schema), instance, &options) else {
        return;
    };
    let mut reasons: Vec<(String, String)> = Vec::new();
    for located in error.located_failures() {
        if let Some(reason) = conversion_reason(located.failure) {
            let entry = (located.pointer, reason);
            if !reasons.contains(&entry) {
                reasons.push(entry);
            }
        }
    }
    for problem in problems {
        let matching: Vec<&str> = reasons
            .iter()
            .filter(|(path, _)| *path == problem.path)
            .map(|(_, reason)| reason.as_str())
            .collect();
        if !matching.is_empty() {
            problem.message = format!("{}: {}", problem.message, matching.join("; "));
        }
    }
}

/// The reason text for a failure that a conversion attempt explains; `None`
/// when the plain type or constraint error already says everything.
fn conversion_reason(failure: &ValueFailure) -> Option<String> {
    match &failure.reason {
        FailureReason::NumberNotAdmitted(reason) => Some(format!("the number {reason}")),
        FailureReason::ConstraintNotMet(constraint) => failure
            .converted
            .as_ref()
            .map(|converted| format!("converted to {converted}, which fails {constraint}")),
        FailureReason::AssertionFailed { description } => failure
            .converted
            .as_ref()
            .map(|converted| format!("converted to {converted}, which fails {description}")),
        reason @ (FailureReason::AmbiguousUnion { .. } | FailureReason::ConflictingDeclarations { .. }) => {
            Some(reason.to_string())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_nested_missing_property_keeps_its_holder_as_written() {
        // The nested missing property shares its name with a top-level key,
        // so attributing it by name would wrongly let `outer` commit.
        let schema = json!({
            "type": "object",
            "properties": {
                "outer": {
                    "type": "object",
                    "properties": { "label": { "type": "string" }, "n": { "type": "number" } },
                    "required": ["n"]
                },
                "n": { "type": "number" }
            }
        });
        let instance = json!({ "outer": { "label": 7 }, "n": "3" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert_eq!(outcome.value, json!({ "outer": { "label": 7 }, "n": 3 }));
    }

    fn implement_like_union() -> Value {
        json!({
            "anyOf": [
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": {
                        "kind": {"type": "string"},
                        "has_spec": {"type": "boolean"},
                        "has_plan": {"type": "boolean"}
                    },
                    "required": ["kind"]
                },
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": { "other": {"type": "string"} },
                    "required": ["other"]
                }
            ]
        })
    }

    fn pending_union() -> Value {
        json!({
            "anyOf": [
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": {
                        "kind": {"type": "string"},
                        "n": {"type": "number"},
                        "flag": {"type": "boolean"}
                    },
                    "required": ["kind"]
                },
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": { "other": {"type": "string"} },
                    "required": ["other"]
                }
            ]
        })
    }

    #[test]
    fn root_union_commits_the_one_reachable_arm() {
        let instance = json!({ "kind": "implement", "has_spec": "true", "has_plan": "no" });
        let outcome = coerce_frontmatter(&implement_like_union(), &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value, json!({ "kind": "implement", "has_spec": true, "has_plan": false }));
    }

    #[test]
    fn root_union_commits_when_only_pending_keys_remain() {
        let instance = json!({ "kind": "implement", "n": "$(echo 1)", "flag": "false" });
        let pending: HashSet<String> = ["n".to_string()].into_iter().collect();
        let outcome = coerce_frontmatter_with_pending(&pending_union(), &instance, &pending);
        assert!(outcome.changed);
        assert_eq!(outcome.value["flag"], json!(false));
        assert_eq!(outcome.value["n"], json!("$(echo 1)"));
    }

    #[test]
    fn without_pending_keys_a_pending_value_blocks_the_commit() {
        let instance = json!({ "kind": "implement", "n": "$(echo 1)", "flag": "false" });
        let outcome = coerce_frontmatter(&pending_union(), &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn pending_keys_do_not_mask_another_failure() {
        let instance = json!({ "kind": "implement", "n": "$(echo 1)", "flag": [1, 2] });
        let pending: HashSet<String> = ["n".to_string()].into_iter().collect();
        let outcome = coerce_frontmatter_with_pending(&pending_union(), &instance, &pending);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    /// A raw root union whose `unevaluatedProperties` reads every accepting
    /// arm: the pending-key fallback judges each arm with the others'
    /// evaluated fields, so only the pending key's problem remains.
    #[test]
    fn root_union_arm_fallback_sees_every_accepting_arm_beside_a_pending_key() {
        let schema = json!({
            "$schema": crate::simplified::DRAFT_2020_12,
            "properties": { "later": {"type": "number"} },
            "anyOf": [
                { "properties": { "name": {"type": "string"} }, "required": ["name"] },
                { "properties": { "age": {"type": "number"} }, "required": ["age"] }
            ],
            "unevaluatedProperties": false
        });
        let validator = structural_validator(&schema).expect("schema compiles");
        let pending: HashSet<String> = ["later".to_string()].into_iter().collect();
        let templates = NestedTemplates::default();

        let both = json!({ "name": "Ada", "age": 36, "later": "$(echo 1)" });
        assert!(accepts(&schema, &validator, &both, &pending, &templates));
        let extra = json!({ "name": "Ada", "age": 36, "later": "$(echo 1)", "extra": true });
        assert!(!accepts(&schema, &validator, &extra, &pending, &templates));
    }

    #[test]
    fn a_failing_property_keeps_its_siblings_conversions() {
        let schema = json!({
            "type": "object",
            "properties": { "count": {"type": "number"}, "flag": {"type": "boolean"} },
            "required": ["title"]
        });
        let instance = json!({ "count": "3", "flag": "maybe" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value, json!({ "count": 3, "flag": "maybe" }));
        // The validation copy still converts what converts on its own.
        assert_eq!(validation_copy(&schema, &instance), json!({ "count": 3, "flag": "maybe" }));
    }

    #[test]
    fn validation_copy_relaxes_a_missing_required_value() {
        let schema = json!({
            "type": "object",
            "properties": { "count": {"type": "number"}, "title": {"type": "string"} },
            "required": ["title"]
        });
        assert_eq!(validation_copy(&schema, &json!({ "count": "3" })), json!({ "count": 3 }));
    }

    #[test]
    fn a_valid_instance_is_kept_and_conversion_is_idempotent() {
        let schema = json!({ "type": "object", "properties": { "flag": {"type": "boolean"} } });
        let first = coerce_frontmatter(&schema, &json!({ "flag": "On" }));
        assert_eq!(first.value, json!({ "flag": true }));
        let second = coerce_frontmatter(&schema, &first.value);
        assert!(!second.changed);
    }

    #[test]
    fn a_candidate_the_complete_schema_rejects_is_not_committed() {
        let schema = json!({ "type": "object", "properties": { "n": {"oneOf": [{"type": "number"}, {"type": "integer"}]} } });
        let instance = json!({ "n": "3" });
        assert!(!coerce_frontmatter(&schema, &instance).changed);
    }

    #[test]
    fn lone_value_serializes_into_content_text() {
        let schema = json!({ "type": "string", "format": "darkmatter-json" });
        assert_eq!(coerce_value_against_schema(&schema, &json!({ "a": 1 })), json!("{\"a\":1}"));
        assert_eq!(coerce_value_against_schema(&schema, &json!(null)), json!(null));
    }

    #[test]
    fn annotation_names_the_admission_reason() {
        let schema = json!({ "type": "object", "properties": { "n": {"type": "number"} } });
        let instance = json!({ "n": "9007199254740993" });
        let validator = structural_validator(&schema).expect("validator");
        let mut problems = validate::collect_problems(&validator, &instance, &validate::PositionMap::new());
        annotate_problems(
            &ConversionPlan::for_schema(&schema),
            &schema,
            &instance,
            &NestedTemplates::default(),
            &mut problems,
        );
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("the number cannot be represented exactly"), "{}", problems[0].message);
    }
}
