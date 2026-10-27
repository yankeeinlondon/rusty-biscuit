//! Schema-driven eager-`file` value rewrite (normalization).
//!
//! Walks a compiled JSON Schema and rewrites every present, non-null value
//! under an eager `format: darkmatter-file` marker to its document-relative
//! resolved path — the same projection `relative(value)` / `dirname(value)`
//! already produce. Pure: builds a rewritten copy and never mutates inputs.
//!
//! The trigger is the compiled-schema marker `format: darkmatter-file` (the
//! eager `file(eager)` arm of [`super::simplified::SimplifiedType::File`]),
//! never the lazy `format: darkmatter-file-reference` marker (Decision #1).
//! Walking the compiled schema (rather than the SimplifiedSchema AST) means
//! raw JSON Schema authors who use Darkmatter's eager format get the same
//! normalization as SimplifiedSchema authors.
//!
//! Resolution consumes the shared document-backed context anchored at the
//! document `base_dir` — the same resolver the eager `file` validator resolves
//! through (implicit paths document-first then repository, no launch-area
//! fallback; D2) — so the stored value resolves identically at prepare time and
//! at every later read (Decision #5). The projection itself goes through
//! [`make_portable_relative`] so the stored string uses `/` separators on every
//! OS (Decision #2 / cross-platform stability).
//!
//! A local `$ref` is followed to its definition (recursive types lower to
//! root `$defs`), applying the reference's sibling keywords along with the
//! target. A reference that names nothing never authorizes a rewrite.
//! Revisiting a definition at a deeper value is normal for a recursive type;
//! only re-entering the same definition with the same value, which descends
//! nowhere, is cut short.
//!
//! See `darkmatter/features/2026-06-27-file-property-rewrite/spec.md` for
//! the full contract (Decisions #1–#9).
//!
//! [`ResolutionContext`]: crate::markdown::compose::expression::ResolutionContext
//! [`make_portable_relative`]: crate::markdown::compose::expression::path_projection::make_portable_relative

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use biscuit_file::FileReference;
use jsonschema::Validator;
use serde_json::{Map, Value};

use super::format::DARKMATTER_FILE_FORMAT;
use super::scope::SchemaScope;
use super::validate::{self, CallerOrigins, ValidatorCache, error_top_level_key};
use crate::markdown::compose::expression::path_projection::make_portable_relative_in_context;
use crate::markdown::compose::expression::resolve_ctx::{
    is_remote_url, normalize_path_arg, resolve_document_file_ref,
};

/// Outcome of normalizing a frontmatter instance against a schema.
///
/// Mirrors [`super::coerce::CoercionOutcome`]'s shape: the (possibly
/// rewritten) instance plus a flag telling the caller whether anything
/// actually changed. Pure — the caller's input is never mutated.
#[derive(Debug, Clone)]
pub struct NormalizationOutcome {
    /// The rewritten instance (a clone when nothing changed).
    pub value: Value,
    /// Whether at least one eager-`file` value was actually rewritten.
    pub changed: bool,
}

/// Top-level dispatcher: rewrites every present eager-`file` value in
/// `instance` according to `json_schema`.
///
/// Pure: never mutates inputs. `base_dir` is the prompt document directory the
/// eager-`file` format validator resolves against, so the rewrite resolves
/// through the identical document-first-then-repository order the read side uses.
/// `fallback` (the launch area) is part of the arm validators' cache identity
/// but is not a resolution input (D2).
///
/// `composition_pending` carries the set of top-level keys still holding a
/// `$(...)` shell expression (compose pre-shell stage). The rewrite skips
/// those keys verbatim so a not-yet-expanded value is never rewritten
/// (Decision #4); their post-shell re-validation will handle them.
///
/// For a root `anyOf` union, the rewrite runs through every arm that accepts
/// the (already-converted) instance and is committed only when those arms
/// agree on the result, so the written order of the arms never matters
/// (Decision #8/#9).
///
/// Union arms are judged with validators from the process-wide
/// [`ValidatorCache`] (see [`ArmValidators`]); every value, and every file's
/// existence, is still judged afresh on each call.
pub fn rewrite_eager_file_values(
    json_schema: &Value,
    instance: &Value,
    base_dir: &Path,
    fallback: Option<&Path>,
    composition_pending: &HashSet<String>,
    request_context: &biscuit_file::FileResolutionContext,
) -> NormalizationOutcome {
    let mut cache = super::SHARED_VALIDATOR_CACHE.clone();
    if let Some(fallback) = fallback {
        cache = cache.with_file_ref_fallback_dir(fallback);
    }
    let mut arms = ArmValidators::new(&cache, None, base_dir, request_context);
    rewrite_with(json_schema, instance, base_dir, composition_pending, request_context, &mut arms)
}

/// [`rewrite_eager_file_values`] for an effective schema: its root-union arms
/// are judged with the validators its own validation compiled (the same
/// wrapped arms, fallback, and caller origins), so they are cache hits. A
/// property-level union arm is judged from the origin of the top-level
/// property it sits under, so a caller-supplied value selects the arm its
/// validation accepted.
pub(super) fn rewrite_for_effective(
    effective: &super::EffectiveSchema,
    instance: &Value,
    base_dir: &Path,
    composition_pending: &HashSet<String>,
) -> NormalizationOutcome {
    let mut cache = super::SHARED_VALIDATOR_CACHE.clone().with_caller_origins(effective.caller_origins.clone());
    if let Some(fallback) = effective.file_ref_fallback_dir.as_deref() {
        cache = cache.with_file_ref_fallback_dir(fallback);
    }
    let context = &effective.file_resolution_context;
    let wrapped = super::wrapped_root_arms(&effective.json_schema);
    let mut arms = ArmValidators::new(&cache, wrapped, base_dir, context);
    rewrite_with(&effective.json_schema, instance, base_dir, composition_pending, context, &mut arms)
}

fn rewrite_with(
    json_schema: &Value,
    instance: &Value,
    base_dir: &Path,
    composition_pending: &HashSet<String>,
    request_context: &biscuit_file::FileResolutionContext,
    arms: &mut ArmValidators<'_>,
) -> NormalizationOutcome {
    if let Some(root_arms) = json_schema.get("anyOf").and_then(Value::as_array) {
        return rewrite_root_union(
            root_arms,
            json_schema,
            instance,
            base_dir,
            arms,
            composition_pending,
            request_context,
        );
    }
    rewrite_object(
        json_schema,
        instance,
        base_dir,
        arms,
        composition_pending,
        request_context,
        json_schema,
    )
}

/// The union-arm validators of one rewrite invocation.
///
/// Each arm's validator is taken from the [`ValidatorCache`] at most once per
/// invocation and origin, and its handle retained, so a list of union items
/// costs one lookup per distinct arm and origin, not one per item. A
/// validator is compiled for a complete arm schema carrying its root's
/// `$defs`, never for a fragment with dangling references, and the cache keys
/// it by the document directory, fallback, request context, and caller
/// origins it judges with. Only compiled validators are shared: no verdict is
/// retained, so each value and each file's existence is judged on every call.
struct ArmValidators<'a> {
    /// Judges root-union arms, which see the whole instance, so each
    /// caller-supplied top-level property is judged from its own origin.
    root_cache: ValidatorCache,
    /// The origin of the top-level property being walked, as the origins of
    /// a validator whose whole instance is a value inside it (see
    /// [`Self::enter_property`]). A property-level arm compiles alone below
    /// that property, so its location no longer names it; the arm is judged
    /// from the property's origin instead, as the surrounding validation
    /// judged it.
    owner: CallerOrigins,
    /// The root arms as the effective schema's validators were compiled from
    /// (shared allocations whose digests the cache already holds), if known.
    wrapped_root_arms: Option<Arc<[Arc<Value>]>>,
    base_dir: &'a Path,
    context: &'a biscuit_file::FileResolutionContext,
    /// By arm address, then owning property's origin: the arm's validator,
    /// or `None` for an arm without the eager marker or one that does not
    /// compile. Every arm is borrowed from the invocation's schema, so an
    /// address names one arm for the whole invocation; the same arm reached
    /// under properties of different origins judges with different
    /// validators.
    property_arms: HashMap<*const Value, Vec<RetainedArm>>,
}

/// A property-level arm's validator for one owning-property origin.
type RetainedArm = (CallerOrigins, Option<Arc<Validator>>);

impl<'a> ArmValidators<'a> {
    fn new(
        cache: &ValidatorCache,
        wrapped_root_arms: Option<Arc<[Arc<Value>]>>,
        base_dir: &'a Path,
        context: &'a biscuit_file::FileResolutionContext,
    ) -> Self {
        Self {
            root_cache: cache.clone(),
            owner: CallerOrigins::default(),
            wrapped_root_arms,
            base_dir,
            context,
            property_arms: HashMap::new(),
        }
    }

    /// The validator for root-union arm `index`, or `None` when it does not
    /// compile (the surrounding validation reports that).
    fn root_arm(&self, index: usize, arm: &Value, root: &Value) -> Option<Arc<Validator>> {
        let wrapped = self.wrapped_root_arms.as_deref().and_then(|wrapped| wrapped.get(index));
        let looked_up = match wrapped {
            Some(wrapped) => self
                .root_cache
                .validator_for_reporting_compile(wrapped, Some(self.base_dir), self.context),
            None => self.root_cache.validator_for_reporting_compile(
                &validate::wrap_arm_as_root_schema(arm, root),
                Some(self.base_dir),
                self.context,
            ),
        };
        Self::note(looked_up)
    }

    /// Judges the property-level arms met from here on from the origin of
    /// top-level property `name`.
    fn enter_property(&mut self, name: &str) {
        self.owner = CallerOrigins::whole_instance(self.root_cache.caller_origins().origin_of(name));
    }

    /// The validator for an eligible property-level arm, judged from the
    /// current property's origin, or `None` for an ineligible arm or one that
    /// does not compile.
    fn property_arm(&mut self, arm: &Value, root: &Value) -> Option<Arc<Validator>> {
        let retained = self.property_arms.entry(std::ptr::from_ref(arm)).or_default();
        if let Some((_, validator)) = retained.iter().find(|(origin, _)| *origin == self.owner) {
            return validator.clone();
        }
        let validator = contains_eager_file_fragment(arm, root)
            .then(|| {
                let cache = self.root_cache.clone().with_caller_origins(self.owner.clone());
                Self::note(cache.validator_for_reporting_compile(
                    &super::scope::with_root_definitions(arm, root),
                    Some(self.base_dir),
                    self.context,
                ))
            })
            .flatten();
        retained.push((self.owner.clone(), validator.clone()));
        validator
    }

    fn note(looked_up: Result<(Arc<Validator>, bool), super::errors::SchemaError>) -> Option<Arc<Validator>> {
        let (validator, compiled) = looked_up.ok()?;
        if compiled {
            crate::note_work!(RewriteArmCompile);
        }
        Some(validator)
    }
}

/// Root-union pass: rewrite through each arm that accepts the
/// (already-converted) instance, and commit only when every accepting arm
/// produces the same result. Arms that disagree leave the instance untouched
/// (Decision #9: "no guessing"), whatever order they are written in.
///
/// Rewriting against an arm the validator never accepted would normalize a
/// value under the wrong declaration, so only accepting arms take part.
fn rewrite_root_union(
    root_arms: &[Value],
    root: &Value,
    instance: &Value,
    base_dir: &Path,
    arms: &mut ArmValidators<'_>,
    composition_pending: &HashSet<String>,
    request_context: &biscuit_file::FileResolutionContext,
) -> NormalizationOutcome {
    let mut agreed: Option<NormalizationOutcome> = None;
    for (index, arm) in root_arms.iter().enumerate() {
        // A schema that fails to build cannot be a valid arm; skip it. The
        // surrounding validator already surfaces build failures elsewhere.
        let Some(validator) = arms.root_arm(index, arm, root) else {
            continue;
        };
        if !arm_accepts(&validator, instance, composition_pending) {
            continue;
        }
        let outcome = rewrite_object(
            arm,
            instance,
            base_dir,
            arms,
            composition_pending,
            request_context,
            root,
        );
        match &agreed {
            None => agreed = Some(outcome),
            Some(previous) if previous.value == outcome.value => {}
            Some(_) => {
                return NormalizationOutcome {
                    value: instance.clone(),
                    changed: false,
                };
            }
        }
    }
    agreed.unwrap_or_else(|| NormalizationOutcome {
        value: instance.clone(),
        changed: false,
    })
}

/// Whether a root-union arm accepts the (already-converted) instance.
///
/// A fully valid candidate is always accepted; otherwise — only when
/// `composition_pending` is non-empty — the arm is still accepted if every
/// residual validation problem points at a pending `$(...)` top-level key.
/// Those keys are re-validated and converted after shell expansion, so
/// deferring them here is sound; a problem on any non-pending key rejects the
/// arm. The same allowance [`super::coerce::coerce_frontmatter_with_pending`]
/// applies.
fn arm_accepts(
    validator: &Validator,
    candidate: &Value,
    composition_pending: &HashSet<String>,
) -> bool {
    if validator.is_valid(candidate) {
        return true;
    }
    if composition_pending.is_empty() {
        return false;
    }
    let mut saw_problem = false;
    for err in validator.iter_errors(candidate) {
        saw_problem = true;
        match error_top_level_key(&err) {
            Some(key) if composition_pending.contains(&key) => {}
            _ => return false,
        }
    }
    saw_problem
}

/// Non-union object pass: walk each declared property and rewrite any
/// present eager-file value under it. Properties absent from the schema or
/// the instance are untouched.
fn rewrite_object(
    schema: &Value,
    instance: &Value,
    base_dir: &Path,
    arms: &mut ArmValidators<'_>,
    composition_pending: &HashSet<String>,
    request_context: &biscuit_file::FileResolutionContext,
    root: &Value,
) -> NormalizationOutcome {
    let (Some(props), Some(obj)) = (
        schema.get("properties").and_then(Value::as_object),
        instance.as_object(),
    ) else {
        return NormalizationOutcome {
            value: instance.clone(),
            changed: false,
        };
    };

    let mut out: Map<String, Value> = obj.clone();
    let mut changed = false;
    let mut active = Vec::new();
    for (name, value) in obj {
        // Skip top-level keys still holding a `$(...)` shell expression —
        // the post-shell re-validation will rewrite them once they expand
        // (Decision #4).
        if composition_pending.contains(name) {
            continue;
        }
        let Some(prop_schema) = props.get(name) else {
            continue;
        };
        arms.enter_property(name);
        if let Some(rewritten) = rewrite_property(
            prop_schema,
            value,
            base_dir,
            arms,
            request_context,
            root,
            &mut active,
        ) {
            out.insert(name.clone(), rewritten);
            changed = true;
        }
    }

    NormalizationOutcome {
        value: Value::Object(out),
        changed,
    }
}

/// If `schema` is one of Darkmatter's optional-property wrappers, the typed
/// inner arm: `{"anyOf":[{"type":"null"}, <typed>]}`, or the optional `file`
/// form `{"anyOf":[{"type":"null"}, {"const":""}, <file-typed>]}`. Any other
/// schema is `None`.
fn unwrap_nullable_arm(schema: &Value) -> Option<&Value> {
    let arms = schema.as_object()?.get("anyOf").and_then(Value::as_array)?;
    let null_first = arms.first().and_then(|arm| arm.get("type")).and_then(Value::as_str) == Some("null");
    match arms.as_slice() {
        [_, typed] if null_first => Some(typed),
        [_, empty, file] if null_first && empty.get("const").and_then(Value::as_str) == Some("") => Some(file),
        _ => None,
    }
}

/// The `(definition, value)` pairs a walk is currently inside: the schema
/// a reference was followed to and the address of the value it was applied
/// to. Both are borrowed for as long as an entry is on the stack, so their
/// addresses identify a location.
type ActiveReferences = Vec<(*const Value, *const Value)>;

/// Rewrites a single property's value (or array item), descending through
/// nullable wrappers, references, unions, inline objects, and arrays to reach
/// any eager-file markers.
///
/// Returns `Some(new_value)` when the value (or a nested sub-value) was
/// rewritten, `None` to leave it untouched. Absent / null / non-string
/// scalars never match the eager-file marker (Decision #7).
///
/// A local `$ref` applies its sibling keywords, then its target. A chain
/// that cannot be resolved (missing target, reference-only loop) leaves the
/// value untouched: the validator reports it, and an unresolvable schema is
/// never read as permission to rewrite. Re-entering a definition already
/// `active` for the same value descends nowhere, so it rewrites nothing.
fn rewrite_property(
    prop_schema: &Value,
    value: &Value,
    base_dir: &Path,
    arms: &mut ArmValidators<'_>,
    request_context: &biscuit_file::FileResolutionContext,
    root: &Value,
    active: &mut ActiveReferences,
) -> Option<Value> {
    if value.is_null() {
        return None;
    }

    // Look through the nullable `anyOf` wrapper so a `file(eager) | null`
    // property still triggers on its present, non-null arm.
    let effective = unwrap_nullable_arm(prop_schema).unwrap_or(prop_schema);

    let hops = SchemaScope::new(root).chain(effective, "").ok()?;
    let [through @ .., target] = hops.as_slice() else {
        return None;
    };
    if through.is_empty() {
        return rewrite_shape(effective, value, base_dir, arms, request_context, root, active);
    }
    let location = (std::ptr::from_ref::<Value>(target), std::ptr::from_ref(value));
    if active.contains(&location) {
        return None;
    }
    active.push(location);
    let mut current: Option<Value> = None;
    for hop in through {
        let next = rewrite_shape(
            hop,
            current.as_ref().unwrap_or(value),
            base_dir,
            arms,
            request_context,
            root,
            active,
        );
        if next.is_some() {
            current = next;
        }
    }
    let next = rewrite_property(
        target,
        current.as_ref().unwrap_or(value),
        base_dir,
        arms,
        request_context,
        root,
        active,
    );
    active.pop();
    next.or(current)
}

/// Rewrites `value` through the keywords of `effective` alone, ignoring any
/// `$ref` on it ([`rewrite_property`] follows references).
fn rewrite_shape(
    effective: &Value,
    value: &Value,
    base_dir: &Path,
    arms: &mut ArmValidators<'_>,
    request_context: &biscuit_file::FileResolutionContext,
    root: &Value,
    active: &mut ActiveReferences,
) -> Option<Value> {
    // Union (property-level, item-level, or an arm's own sentinel wrapper).
    // Rewrite only through the one validating arm that carries the eager-file
    // marker (Decision #9: "no guessing").
    if let Some(union_arms) = effective.get("anyOf").and_then(Value::as_array) {
        return rewrite_property_union(union_arms, value, base_dir, arms, request_context, root, active);
    }

    // Direct eager-file marker on the effective schema.
    if is_eager_file_fragment(effective) {
        return rewrite_file_value(value, base_dir, request_context);
    }

    // Array of eager-file elements: descend through `items`.
    if effective.get("type").and_then(Value::as_str) == Some("array")
        && let Some(items_schema) = effective.get("items")
    {
        return rewrite_array_items(items_schema, value, base_dir, arms, request_context, root, active);
    }

    // Inline object with declared sub-properties: descend into each present
    // sub-property (Decision #8).
    if effective.get("type").and_then(Value::as_str) == Some("object")
        && let Some(sub_props) = effective.get("properties").and_then(Value::as_object)
    {
        return rewrite_inline_object(sub_props, value, base_dir, arms, request_context, root, active);
    }

    None
}

/// Union pass, shared by property unions and union items (`items.anyOf`):
/// rewrite only through the one validating arm that contains an eager-file
/// marker. Zero or multiple such arms leave the value untouched (Decision #9:
/// "no guessing"); an arm that merely scores closest gains no file effects.
///
/// An arm is eligible when its compiled fragment contains
/// `format: darkmatter-file` anywhere, including through the definitions it
/// references: a scalar `file(eager)` arm, the optional
/// `[{const: ""}, file(eager)]` sentinel wrapper, or an object arm (inline or
/// a recursive type) with a nested eager field. Arms without the marker are never counted, even
/// if they validate, because they do not identify the value as
/// eager-file-typed. The eligible arm is then walked by [`rewrite_property`],
/// so object arms rewrite only their own eager fields.
fn rewrite_property_union(
    union_arms: &[Value],
    value: &Value,
    base_dir: &Path,
    arms: &mut ArmValidators<'_>,
    request_context: &biscuit_file::FileResolutionContext,
    root: &Value,
    active: &mut ActiveReferences,
) -> Option<Value> {
    let mut selected: Option<&Value> = None;
    for arm in union_arms {
        let Some(validator) = arms.property_arm(arm, root) else {
            continue;
        };
        if validator.is_valid(value) {
            if selected.is_some() {
                // Ambiguous: more than one validating eager-file arm.
                return None;
            }
            selected = Some(arm);
        }
    }
    rewrite_property(selected?, value, base_dir, arms, request_context, root, active)
}

/// Rewrites a single present eager-file value to its document-relative resolved
/// path. Returns `Some(new_value)` when the value was rewritten, `None` to
/// leave it untouched.
///
/// Leaves the value verbatim when:
/// - the value is not a string (Decision #7),
/// - the value is a remote URL ([`is_remote_url`], Decision #7),
/// - the resolved path is `None` (a non-local reference validation accepted,
///   Decision #7),
/// - rewriting produces a byte-identical string (idempotence fast path,
///   Decision #6).
fn rewrite_file_value(
    value: &Value,
    base_dir: &Path,
    request_context: &biscuit_file::FileResolutionContext,
) -> Option<Value> {
    let Value::String(raw) = value else {
        return None;
    };
    // Remote URLs are left verbatim — there is no local path to project
    // (Decision #7).
    if is_remote_url(raw) {
        return None;
    }
    // Resolve through the shared document-backed context — the same resolver
    // the eager-file format validator used to accept this value: implicit paths
    // document-first then repository, no launch-area fallback (D2). The rewrite
    // must consume the same resolver so the stored value resolves identically at
    // every later read (Decision #5).
    let normalized = normalize_path_arg(raw);
    let file_ref = match FileReference::new(&normalized) {
        Ok(r) => r,
        // Invalid syntax shouldn't surface here (validation already accepted
        // the value), but degrade gracefully rather than panic.
        Err(_) => return None,
    };
    let resolved = match resolve_document_file_ref(&file_ref, base_dir, request_context) {
        Ok(Some(abs)) => abs,
        // `None`: the reference is well-formed but resolves to nothing local
        // (e.g. a remote-resolving value validation accepted). Leave verbatim
        // (Decision #7).
        Ok(None) => return None,
        // Resolution error: the value shouldn't have passed validation, but
        // degrade gracefully.
        Err(_) => return None,
    };
    let portable = make_portable_relative_in_context(&resolved, base_dir, request_context);
    // Idempotence fast path: if the rewritten value equals the input, no
    // mutation is needed (Decision #6).
    if &portable == raw {
        return None;
    }
    Some(Value::String(portable))
}

/// Recurses into each item of an array value via the `items` fragment. The
/// fragment may itself be an eager-file marker (for `file(eager)[]`), an
/// inline object, an `anyOf` of union arms (for `U[]`, decided per item), or
/// any other recursable shape `rewrite_property` handles.
fn rewrite_array_items(
    items_schema: &Value,
    value: &Value,
    base_dir: &Path,
    arms: &mut ArmValidators<'_>,
    request_context: &biscuit_file::FileResolutionContext,
    root: &Value,
    active: &mut ActiveReferences,
) -> Option<Value> {
    let Value::Array(items) = value else {
        return None;
    };
    let mut out = Vec::with_capacity(items.len());
    let mut changed = false;
    for item in items {
        match rewrite_property(items_schema, item, base_dir, arms, request_context, root, active) {
            Some(rewritten) => {
                out.push(rewritten);
                changed = true;
            }
            None => out.push(item.clone()),
        }
    }
    changed.then_some(Value::Array(out))
}

/// Recurses into the declared sub-properties of an inline object value
/// (Decision #8). Sub-properties not declared in the schema are passed
/// through untouched.
fn rewrite_inline_object(
    sub_props: &Map<String, Value>,
    value: &Value,
    base_dir: &Path,
    arms: &mut ArmValidators<'_>,
    request_context: &biscuit_file::FileResolutionContext,
    root: &Value,
    active: &mut ActiveReferences,
) -> Option<Value> {
    let Value::Object(obj) = value else {
        return None;
    };
    let mut out = obj.clone();
    let mut changed = false;
    for (name, sub_value) in obj {
        let Some(sub_schema) = sub_props.get(name) else {
            continue;
        };
        if let Some(rewritten) =
            rewrite_property(sub_schema, sub_value, base_dir, arms, request_context, root, active)
        {
            out.insert(name.clone(), rewritten);
            changed = true;
        }
    }
    changed.then_some(Value::Object(out))
}

/// Whether a compiled schema fragment carries the eager
/// `format: darkmatter-file` marker — the sole rewrite trigger (Decision
/// #1). The lazy `darkmatter-file-reference` marker returns `false`.
fn is_eager_file_fragment(schema: &Value) -> bool {
    schema.get("format").and_then(Value::as_str) == Some(DARKMATTER_FILE_FORMAT)
}

/// Whether the eager marker appears anywhere inside a compiled fragment or
/// the definitions it references in `root`. Each definition is searched once,
/// and a reference that names nothing contributes no marker.
fn contains_eager_file_fragment(schema: &Value, root: &Value) -> bool {
    contains_eager_marker(schema, SchemaScope::new(root), &mut Vec::new())
}

fn contains_eager_marker<'a>(
    schema: &'a Value,
    scope: SchemaScope<'a>,
    searched: &mut Vec<*const Value>,
) -> bool {
    let found = match schema {
        Value::Object(map) => {
            is_eager_file_fragment(schema)
                || map.values().any(|child| contains_eager_marker(child, scope, searched))
        }
        Value::Array(values) => values.iter().any(|child| contains_eager_marker(child, scope, searched)),
        _ => return false,
    };
    if found {
        return true;
    }
    match scope.resolve(schema, "") {
        Ok(Some(target)) if !searched.contains(&std::ptr::from_ref(target)) => {
            searched.push(std::ptr::from_ref(target));
            contains_eager_marker(target, scope, searched)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::functions::call_function;
    use crate::markdown::compose::expression::resolve_ctx::ResolutionContext;
    use serde_json::json;
    use tempfile::TempDir;

    /// `@spec.md` from an external document resolves through the unchanged
    /// launch `@` scope to the request repository, never to the child
    /// repository's decoy. The external document has no repository of its own,
    /// so the rewritten value is not repository-relative.
    #[test]
    fn eager_rewrite_uses_the_launch_scope_instead_of_rediscovering_from_child() {
        let request_repo = TempDir::new().unwrap();
        let child_repo = TempDir::new().unwrap();
        std::fs::create_dir_all(request_repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(child_repo.path().join(".git")).unwrap();
        let child_base = child_repo.path().join("docs");
        std::fs::create_dir_all(&child_base).unwrap();
        let request_target = request_repo.path().join("spec.md");
        std::fs::write(&request_target, "request").unwrap();
        std::fs::write(child_repo.path().join("spec.md"), "child decoy").unwrap();
        // No home: on Windows the temp directory is under it, which would
        // render the value as `~/…` instead of the path asserted below.
        let context = biscuit_file::FileResolutionContext::new(request_repo.path())
            .with_repository_root(request_repo.path())
            .without_home_dir()
            .for_trusted_external_cwd(&child_base);
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": {"type": "string", "format": DARKMATTER_FILE_FORMAT}
            }
        });
        let input = json!({"spec": "@spec.md"});

        let outcome = rewrite_eager_file_values(
            &schema,
            &input,
            &child_base,
            None,
            &HashSet::new(),
            &context,
        );

        assert_eq!(context.repository_root(), None);
        assert!(outcome.changed);
        assert_eq!(
            outcome.value["spec"],
            json!(biscuit_file::to_portable_string(&request_target))
        );
    }

    /// Creates a temp directory that looks like a git repository root, with
    /// an optional subdirectory the "document" lives in.
    ///
    /// The request context treats this temp directory as its repository root.
    fn repo_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    /// Schema fragment for a required `file(eager)` property named `spec`.
    fn eager_file_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "spec": { "type": "string", "format": "darkmatter-file" }
            }
        })
    }

    /// Schema fragment for a required bare (lazy) `file` property named `spec`.
    fn lazy_file_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "spec": { "type": "string", "format": "darkmatter-file-reference" }
            }
        })
    }

    /// Optional `file(eager)` property: nullable wrapper around the eager
    /// fragment. Mirrors `convert::atom_to_schema`'s optional-`file` shape.
    fn optional_eager_file_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "spec": { "anyOf": [
                    { "type": "null" },
                    { "const": "" },
                    { "type": "string", "format": "darkmatter-file" }
                ] }
            }
        })
    }

    // ── Decision #1: only the eager marker triggers ───────────────────────

    #[test]
    fn eager_file_property_is_rewritten_to_document_relative() {
        let repo = repo_fixture();
        std::fs::create_dir_all(repo.path().join("area/sub")).unwrap();
        std::fs::write(repo.path().join("area/sub/spec.md"), "# Spec\n").unwrap();
        // base_dir is the document directory — a subdirectory of the repo.
        let base_dir = repo.path().join("area/sub");

        let schema = eager_file_schema();
        let instance = json!({ "spec": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            &base_dir,
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(&base_dir),
        );
        assert!(outcome.changed);
        assert_eq!(outcome.value["spec"], json!("spec.md"));
    }

    #[test]
    fn rewritten_value_equals_relative_fn_output() {
        // Consistency with the existing projection (Decision #2): the
        // rewritten value must be byte-identical to what `relative(value)`
        // produces for the same input.
        let repo = repo_fixture();
        std::fs::create_dir_all(repo.path().join("docs")).unwrap();
        let abs = repo.path().join("docs/guide.md");
        std::fs::write(&abs, "# Guide\n").unwrap();
        let base_dir = repo.path().join("docs");

        // Projection via the shared helper the expression path uses.
        let ctx = ResolutionContext::at(base_dir.clone());
        let relative = call_function("relative", &[json!("./guide.md")], Some(&ctx))
            .expect("relative is registered")
            .expect("relative resolves");
        assert_eq!(relative, json!("guide.md"));

        // Projection via the rewrite pass.
        let schema = eager_file_schema();
        let instance = json!({ "spec": "./guide.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            &base_dir,
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(&base_dir),
        );
        assert!(outcome.changed);
        assert_eq!(outcome.value["spec"], relative);
    }

    #[test]
    fn string_property_holding_path_shaped_value_is_not_rewritten() {
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": { "type": "string" }
            }
        });
        let instance = json!({ "spec": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn raw_json_schema_with_eager_format_is_rewritten() {
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "doc": { "type": "string", "format": "darkmatter-file" }
            }
        });
        let instance = json!({ "doc": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(outcome.changed);
        assert_eq!(outcome.value["doc"], json!("spec.md"));
    }

    #[test]
    fn raw_json_schema_with_other_format_is_not_rewritten() {
        let repo = repo_fixture();
        let schema = json!({
            "type": "object",
            "properties": {
                "email": { "type": "string", "format": "email" }
            }
        });
        let instance = json!({ "email": "user@example.com" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn lazy_file_reference_is_not_rewritten_even_when_file_exists() {
        // Decision #1: bare (lazy) `file` lowers to
        // `format: darkmatter-file-reference`, which is syntax-only and is
        // never a trigger.
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = lazy_file_schema();
        let instance = json!({ "spec": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value["spec"], json!("./spec.md"));
    }

    // ── Decision #6: idempotence ──────────────────────────────────────────

    #[test]
    fn rewrite_is_idempotent() {
        let repo = repo_fixture();
        std::fs::create_dir_all(repo.path().join("area")).unwrap();
        std::fs::write(repo.path().join("area/spec.md"), "# Spec\n").unwrap();
        let base_dir = repo.path();

        let schema = eager_file_schema();
        // First rewrite: `./area/spec.md` → repo-relative `area/spec.md`.
        let raw = json!({ "spec": "./area/spec.md" });
        let pending = HashSet::new();
        let first = rewrite_eager_file_values(
            &schema,
            &raw,
            base_dir,
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(base_dir),
        );
        assert!(first.changed);
        assert_eq!(first.value["spec"], json!("area/spec.md"));

        // Second rewrite: already-canonical value is a fixpoint.
        let second = rewrite_eager_file_values(
            &schema,
            &first.value,
            base_dir,
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(base_dir),
        );
        assert!(!second.changed, "second rewrite must be a no-op");
        assert_eq!(first.value, second.value);
    }

    // ── Decision #4: unresolvable values are never rewritten ──────────────

    #[test]
    fn unresolvable_eager_value_is_not_rewritten() {
        // A `file(eager)` value that does not resolve to an existing file
        // would have failed validation; the rewrite pass must not be invoked
        // on a failed-validation instance. This test double-belts the
        // contract: even if it were invoked, the missing file produces no
        // rewrite (the resolver returns `None`/`Err`).
        let repo = repo_fixture();
        let schema = eager_file_schema();
        let instance = json!({ "spec": "./does-not-exist.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value["spec"], json!("./does-not-exist.md"));
    }

    // ── Decision #7: non-local and absent values pass through unchanged ───

    #[test]
    fn remote_url_eager_value_is_left_verbatim() {
        let repo = repo_fixture();
        let schema = eager_file_schema();
        let instance = json!({ "spec": "https://example.com/spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value["spec"], json!("https://example.com/spec.md"));
    }

    #[test]
    fn absent_eager_file_property_is_unchanged() {
        let repo = repo_fixture();
        let schema = optional_eager_file_schema();
        let instance = json!({});
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn null_optional_eager_file_property_is_unchanged() {
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = optional_eager_file_schema();
        let instance = json!({ "spec": null });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value["spec"], json!(null));
    }

    #[test]
    fn optional_eager_file_with_present_value_is_rewritten() {
        // The nullable wrapper's `unwrap_nullable_arm` reaches the eager arm
        // so a present, non-null value still triggers the rewrite.
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = optional_eager_file_schema();
        let instance = json!({ "spec": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(outcome.changed);
        assert_eq!(outcome.value["spec"], json!("spec.md"));
    }

    // ── Decision #8: nested eager-file properties participate ─────────────

    #[test]
    fn inline_object_sub_property_eager_file_is_rewritten() {
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "path": { "type": "string", "format": "darkmatter-file" },
                        "label": { "type": "string" }
                    }
                }
            }
        });
        let instance = json!({
            "config": { "path": "./spec.md", "label": "untouched" }
        });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(outcome.changed);
        assert_eq!(outcome.value["config"]["path"], json!("spec.md"));
        assert_eq!(outcome.value["config"]["label"], json!("untouched"));
    }

    #[test]
    fn array_of_eager_file_elements_are_rewritten() {
        let repo = repo_fixture();
        std::fs::write(repo.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(repo.path().join("b.md"), "# B\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "refs": {
                    "type": "array",
                    "items": { "type": "string", "format": "darkmatter-file" }
                }
            }
        });
        let instance = json!({ "refs": ["./a.md", "./b.md"] });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(outcome.changed);
        assert_eq!(outcome.value["refs"], json!(["a.md", "b.md"]));
    }

    #[test]
    fn root_union_rewrites_committed_arm_eager_file() {
        // Two arms: the first carries an eager-file property plus a required
        // discriminator; the second is an unrelated object. The instance
        // satisfies arm 0 (carries `kind: "feature"`), so only arm 0's
        // `spec` is rewritten.
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = json!({
            "anyOf": [
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": {
                        "kind": { "type": "string" },
                        "spec": { "type": "string", "format": "darkmatter-file" }
                    },
                    "required": ["kind"]
                },
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": { "other": { "type": "string" } },
                    "required": ["other"]
                }
            ]
        });
        let instance = json!({ "kind": "feature", "spec": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(outcome.changed);
        assert_eq!(outcome.value["spec"], json!("spec.md"));
        assert_eq!(outcome.value["kind"], json!("feature"));
    }

    #[test]
    fn root_union_without_matching_arm_leaves_instance_unchanged() {
        // No arm validates (missing every required discriminator). The
        // rewrite is a no-op — validation would have reported the problem.
        let repo = repo_fixture();
        let schema = json!({
            "anyOf": [
                {
                    "type": "object",
                    "properties": { "kind": { "type": "string" } },
                    "required": ["kind"]
                }
            ]
        });
        let instance = json!({ "spec": "spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn property_union_rewrites_when_exactly_one_eager_arm_validates() {
        // A property-level union `[file(eager), string]`. The value resolves
        // as a file so only the eager arm validates → rewrite is committed.
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": { "anyOf": [
                    { "type": "string", "format": "darkmatter-file" },
                    { "type": "number" }
                ] }
            }
        });
        let instance = json!({ "spec": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(outcome.changed);
        assert_eq!(outcome.value["spec"], json!("spec.md"));
    }

    // ── Decision #9: ambiguous property unions do not guess ───────────────

    #[test]
    fn ambiguous_property_union_is_not_rewritten() {
        // Two arms both carry the eager-file marker and both validate the
        // same string value. With two matching eager arms the rewrite cannot
        // pick one without guessing, so the value is left verbatim.
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": { "anyOf": [
                    { "type": "string", "format": "darkmatter-file" },
                    { "type": "string", "format": "darkmatter-file" }
                ] }
            }
        });
        let instance = json!({ "spec": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed, "ambiguous union must not be rewritten");
        assert_eq!(outcome.value["spec"], json!("./spec.md"));
    }

    // ── Decision #4: composition-pending values are skipped ───────────────

    #[test]
    fn pending_eager_file_value_is_skipped_while_sibling_is_rewritten() {
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        std::fs::write(repo.path().join("design.md"), "# Design\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": { "type": "string", "format": "darkmatter-file" },
                "design": { "type": "string", "format": "darkmatter-file" }
            }
        });
        // `spec` is pending (holds a `$(...)` shell expression); `design` is
        // concrete. The rewrite must skip `spec` and still rewrite `design`.
        let instance = json!({
            "spec": "$(echo spec.md)",
            "design": "./design.md"
        });
        let pending: HashSet<String> = ["spec".to_string()].into_iter().collect();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(outcome.changed);
        assert_eq!(
            outcome.value["spec"],
            json!("$(echo spec.md)"),
            "pending value must be left verbatim"
        );
        assert_eq!(outcome.value["design"], json!("design.md"));
    }

    /// A pending value reachable through a non-pending parent key is also
    /// skipped: `rewrite_file_value` checks for `$(`/`{{` markers in the raw
    /// string as a defensive backstop (Decision #4: "for nested shapes,
    /// skips a pending scalar in place").
    #[test]
    fn pending_marker_inside_nested_value_is_skipped() {
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "path": { "type": "string", "format": "darkmatter-file" }
                    }
                }
            }
        });
        let instance = json!({
            "config": { "path": "$(echo spec.md)" }
        });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed, "pending nested scalar must be skipped");
        assert_eq!(outcome.value["config"]["path"], json!("$(echo spec.md)"));
    }

    // ── Non-string values under an eager-file marker are untouched ────────

    #[test]
    fn non_string_value_under_eager_file_marker_is_untouched() {
        // A non-string slipped under an eager-file marker (shouldn't happen
        // post-validation, but the rewrite degrades gracefully).
        let repo = repo_fixture();
        let schema = eager_file_schema();
        let instance = json!({ "spec": 42 });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value["spec"], json!(42));
    }

    /// A definition that reaches itself without descending into the value
    /// (here through its own optional wrapper) is entered once per value, so
    /// the walk ends and rewrites nothing, while a sibling eager field still
    /// rewrites.
    #[test]
    fn a_definition_reentered_for_the_same_value_rewrites_nothing() {
        let repo = repo_fixture();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "looped": {"$ref": "#/$defs/looped"},
                "spec": {"$ref": "#/$defs/file"}
            },
            "$defs": {
                "looped": {"anyOf": [{"type": "null"}, {"$ref": "#/$defs/looped"}]},
                "file": {"type": "string", "format": DARKMATTER_FILE_FORMAT}
            }
        });
        let instance = json!({"looped": "./spec.md", "spec": "./spec.md"});

        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &HashSet::new(),
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );

        assert_eq!(outcome.value["looped"], json!("./spec.md"));
        assert_eq!(outcome.value["spec"], json!("spec.md"));
    }

    /// R1-F2.11: one union arm, reached under a document-authored property
    /// and a caller-supplied one, is judged from each property's own origin.
    /// `other` is walked first; a validator retained by arm address alone
    /// would judge `spec` from the document's folder too, where `./*.md`
    /// does not admit `launch/a.md`.
    #[test]
    fn a_shared_arm_is_judged_from_each_owning_property_origin() {
        let repo = repo_fixture();
        let root = biscuit_file::canonicalize_simplified(repo.path()).unwrap();
        let launch = root.join("launch");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::write(root.join("a.md"), "# A\n").unwrap();
        std::fs::write(launch.join("a.md"), "# launch A\n").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "other": {"$ref": "#/$defs/pick"},
                "spec": {"$ref": "#/$defs/pick"}
            },
            "$defs": {
                "pick": {"anyOf": [
                    {
                        "type": "string",
                        "format": DARKMATTER_FILE_FORMAT,
                        "x-darkmatter-match": ["./*.md"]
                    },
                    {"type": "number"}
                ]}
            }
        });
        let launch_file = biscuit_file::to_portable_string(&launch.join("a.md"));
        let instance = json!({"other": "./a.md", "spec": launch_file});
        let context = biscuit_file::FileResolutionContext::new(&root);
        let cache = ValidatorCache::new().with_caller_origins(CallerOrigins::new([(
            "spec".to_string(),
            biscuit_file::FileResolutionContext::new(&launch),
        )]));
        let mut arms = ArmValidators::new(&cache, None, &root, &context);

        let outcome = rewrite_with(&schema, &instance, &root, &HashSet::new(), &context, &mut arms);

        assert_eq!(outcome.value["other"], json!("a.md"));
        assert_eq!(outcome.value["spec"], json!("launch/a.md"));
        let retained = arms.property_arms.values().map(Vec::len).max();
        assert_eq!(retained, Some(2), "the file arm is retained once per origin");
    }

    /// Sanity: a non-object instance (e.g. a bare string at the root) is
    /// returned unchanged since the schema's `properties` walk has nothing
    /// to iterate.
    #[test]
    fn non_object_instance_is_returned_unchanged() {
        let repo = repo_fixture();
        let schema = eager_file_schema();
        let instance = json!("not-an-object");
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(
            &schema,
            &instance,
            repo.path(),
            None,
            &pending,
            &biscuit_file::FileResolutionContext::new(repo.path()),
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }
}
