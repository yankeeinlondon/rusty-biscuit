//! Validator construction and caching.
//!
//! This module is the bridge between a fully-formed JSON Schema
//! `serde_json::Value` and a `jsonschema::Validator` that can be exercised
//! against frontmatter data. Two responsibilities live here:
//!
//! - **Validator construction** — wires up Darkmatter's custom formats
//!   ([`format::DARKMATTER_FILE_FORMAT`],
//!   [`format::DARKMATTER_FILE_REFERENCE_FORMAT`]) and the
//!   [`format::DARKMATTER_URL_SCHEME_KEYWORD`] keyword on top of Draft
//!   2020-12. (`match(...)` is suggestion metadata only and has no validation
//!   keyword.)
//! - **Caching** — compiling a `Validator` is several milliseconds of work;
//!   the [`ValidatorCache`] hashes the canonicalised schema bytes and reuses
//!   compiled validators across calls. The default bound (64 entries) is
//!   adjustable via `DARKMATTER_SCHEMA_CACHE_SIZE`.
//!
//! Validation problems are mapped from `jsonschema::ValidationError` into the
//! public [`super::ValidationProblem`] shape (JSON-pointer path, friendly
//! message, optional source line/column).

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

use indexmap::IndexMap;
use jsonschema::{
    Draft, JsonType, PatternOptions, Validator,
    error::{TypeKind, ValidationErrorKind},
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    FileReferenceDiagnostic, JsonPointer, ValidationProblem, ValidationProblemCode,
    ValidationProblemKind, errors::SchemaError, format,
};

/// Map of top-level frontmatter key → 1-based `(line, column)` within the
/// serialised frontmatter YAML, used to annotate [`ValidationProblem`] sites
/// with source coordinates.
pub type PositionMap = IndexMap<String, (u32, u32)>;

/// Environment variable for overriding the validator-cache bound.
pub const CACHE_SIZE_ENV: &str = "DARKMATTER_SCHEMA_CACHE_SIZE";

/// Default validator-cache bound. Matches the value documented in the spec.
pub const DEFAULT_CACHE_SIZE: usize = 64;

/// Process-wide cache of compiled validators keyed by canonicalised schema
/// hash.
///
/// The hashmap entry stores the compiled validator together with an internal
/// LRU "tick" so eviction is `O(n)` over the cache rather than requiring a
/// linked structure. With the default cap of 64 entries this is fast enough
/// and trades implementation complexity for predictable behaviour.
///
/// ## File-reference resolution invariant
///
/// A single cache carries one [`Self::file_ref_fallback_dir`] (the launch-area
/// fallback), set at construction via [`Self::with_file_ref_fallback_dir`].
/// The prompt document directory (`base_dir`) is supplied per call to
/// [`Self::validator_for`] because it varies per document, and is folded into
/// the cache key alongside the schema JSON: two documents sharing the same
/// schema JSON but living in different directories get distinct cached
/// validators, each resolving `format: darkmatter-file` values document-first
/// against its own directory. Mixing fallbacks still requires separate caches
/// (i.e. separate `DarkmatterSchemas` instances).
#[derive(Clone)]
pub struct ValidatorCache {
    inner: Arc<Mutex<CacheInner>>,
    /// Launch-area fallback for `format: darkmatter-file` value resolution,
    /// tried after the per-document `base_dir`. `None` (the default) leaves
    /// the fallback unset; with no `base_dir` either, resolution preserves the
    /// legacy ambient-process-CWD behavior.
    file_ref_fallback_dir: Option<PathBuf>,
}

impl Default for ValidatorCache {
    fn default() -> Self {
        Self::new()
    }
}

struct CacheInner {
    entries: HashMap<[u8; 32], CacheEntry>,
    tick: u64,
    capacity: usize,
}

struct CacheEntry {
    validator: Arc<Validator>,
    last_used: u64,
}

impl ValidatorCache {
    /// Creates a cache with the default capacity (overridable via the
    /// `DARKMATTER_SCHEMA_CACHE_SIZE` environment variable).
    pub fn new() -> Self {
        Self::with_capacity(default_capacity())
    }

    /// Creates a cache with an explicit capacity. Capacity values of `0` are
    /// promoted to `1` so the cache always holds at least the most recently
    /// built validator.
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            inner: Arc::new(Mutex::new(CacheInner {
                entries: HashMap::new(),
                tick: 0,
                capacity: cap,
            })),
            file_ref_fallback_dir: None,
        }
    }

    /// Sets the directory used to resolve `format: darkmatter-file` property
    /// values during validation. When set, file references resolve via
    /// [`FileReference::resolve_from`] against this directory instead of the
    /// ambient process working directory.
    ///
    /// Must be set before any validator is built. Changing it after the cache
    /// holds validators produces stale results because the cache keys on
    /// schema JSON alone (see the struct-level invariant note).
    #[must_use]
    pub fn with_file_ref_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.file_ref_fallback_dir = Some(dir.into());
        self
    }

    /// Returns a compiled validator for the given JSON Schema, building it on
    /// first use and reusing the cached one thereafter.
    ///
    /// `base_dir`, when `Some`, is the prompt document directory used as the
    /// document-first anchor for `format: darkmatter-file` value resolution.
    /// It is folded into the cache key so two documents that share a schema
    /// but live in different directories do not share a validator.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError::BuildValidator`] when `jsonschema` rejects
    /// the schema (bad draft features, malformed keywords, etc.).
    pub fn validator_for(
        &self,
        schema: &Value,
        base_dir: Option<&Path>,
    ) -> Result<Arc<Validator>, SchemaError> {
        let key = canonical_hash(schema, base_dir);
        // Fast path: hit.
        if let Some(hit) = self.lookup(&key) {
            return Ok(hit);
        }
        // Miss: build outside the lock to keep contention low.
        let validator = Arc::new(build_validator(
            schema,
            base_dir,
            self.file_ref_fallback_dir.as_deref(),
        )?);
        self.insert(key, validator.clone());
        Ok(validator)
    }

    fn lookup(&self, key: &[u8; 32]) -> Option<Arc<Validator>> {
        let mut guard = self.inner.lock().expect("validator cache lock poisoned");
        guard.tick = guard.tick.wrapping_add(1);
        let tick = guard.tick;
        let entry = guard.entries.get_mut(key)?;
        entry.last_used = tick;
        Some(entry.validator.clone())
    }

    fn insert(&self, key: [u8; 32], validator: Arc<Validator>) {
        let mut guard = self.inner.lock().expect("validator cache lock poisoned");
        guard.tick = guard.tick.wrapping_add(1);
        let tick = guard.tick;
        let cap = guard.capacity;
        guard.entries.insert(
            key,
            CacheEntry {
                validator,
                last_used: tick,
            },
        );
        while guard.entries.len() > cap {
            if let Some(victim_key) = guard
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_used)
                .map(|(k, _)| *k)
            {
                guard.entries.remove(&victim_key);
            } else {
                break;
            }
        }
    }

    /// Returns the launch-area fallback directory this cache was configured
    /// with, if any. Used by [`super::EffectiveSchema`] to mirror the
    /// validator's anchors when re-resolving a `format: darkmatter-file`
    /// diagnostic.
    pub(super) fn file_ref_fallback_dir(&self) -> Option<&Path> {
        self.file_ref_fallback_dir.as_deref()
    }

    /// Returns the current number of cached validators. Mainly a testing aid.
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("validator cache lock poisoned")
            .entries
            .len()
    }

    /// Reports whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Builds a `Validator` configured with darkmatter's custom format and
/// keywords.
///
/// `base_dir` (the prompt document directory) and `file_ref_fallback_dir`
/// (the captured launch area) anchor `format: darkmatter-file` value
/// resolution document-first then fallback, matching the expression path.
/// When both are `None`, resolution falls back to the ambient process working
/// directory (legacy behavior). Threading `base_dir` here lets schema
/// validation agree with expression-side `file_exists`/`frontmatter`
/// resolution: a `file` value beside the prompt resolves even after the
/// wrapper has `chdir`'d away from the launch area.
/// Reports whether any `patternProperties` key in the compiled JSON Schema
/// carries a regex lookaround (Feature C literal precedence). Such schemas need
/// the backtracking `fancy-regex` engine; everything else stays on the linear
/// `regex` engine.
fn schema_uses_lookaround(schema: &Value) -> bool {
    match schema {
        Value::Object(map) => {
            if let Some(Value::Object(pattern_props)) = map.get("patternProperties")
                && pattern_props
                    .keys()
                    .any(|key| super::simplified::convert::has_lookaround(key))
            {
                return true;
            }
            map.values().any(schema_uses_lookaround)
        }
        Value::Array(items) => items.iter().any(schema_uses_lookaround),
        _ => false,
    }
}

pub(super) fn build_validator(
    schema: &Value,
    base_dir: Option<&Path>,
    file_ref_fallback_dir: Option<&Path>,
) -> Result<Validator, SchemaError> {
    // Feature C literal-precedence emits negative-lookahead `patternProperties`
    // keys, which the linear `regex` engine rejects. Opt such schemas into the
    // backtracking `fancy-regex` engine while keeping every lookaround-free
    // schema on the ReDoS-safe linear engine (byte-identical to prior behavior).
    // The two `PatternOptions` values are distinct types, so the branch is on
    // the builder call, not a shared binding.
    let base = jsonschema::options().with_draft(Draft::Draft202012);
    let opts = if schema_uses_lookaround(schema) {
        base.with_pattern_options(PatternOptions::fancy_regex())
    } else {
        base.with_pattern_options(PatternOptions::regex())
    };
    let opts = format::register_darkmatter_formats(
        opts,
        base_dir.map(PathBuf::from),
        file_ref_fallback_dir.map(PathBuf::from),
    )
        .should_validate_formats(true)
        .with_keyword(
            format::DARKMATTER_URL_SCHEME_KEYWORD,
            format::url_scheme_keyword_factory,
        );
    opts.build(schema)
        .map_err(|err| SchemaError::BuildValidator {
            message: err.to_string(),
        })
}

/// Maps `jsonschema::ValidationError` into the public `ValidationProblem`
/// shape (JSON pointer + plain message). Line/column come from `positions`,
/// which the caller is expected to derive from the frontmatter source via
/// [`build_position_map`].
pub fn collect_problems(
    validator: &Validator,
    instance: &Value,
    positions: &PositionMap,
) -> Vec<ValidationProblem> {
    collect_problems_with_anchors(validator, instance, positions, FileRefAnchors::default())
}

/// Like [`collect_problems`] but carries the document-first / launch-area
/// anchors so a substituted `format: darkmatter-file` diagnostic re-resolves
/// against the same directories the validator was built with.
pub(super) fn collect_problems_with_anchors(
    validator: &Validator,
    instance: &Value,
    positions: &PositionMap,
    anchors: FileRefAnchors<'_>,
) -> Vec<ValidationProblem> {
    validator
        .iter_errors(instance)
        .flat_map(|err| build_problems(&err, positions, None, anchors))
        .collect()
}

/// Per-arm validation of a root-union schema.
///
/// Each `arm_validator` is the compiled validator for the corresponding
/// `anyOf` arm of the document schema. The arm producing the fewest problems
/// "wins"; its problems are returned with `arm_index` populated. When every
/// arm validates successfully (which should not happen for a failed instance)
/// the empty problem list is returned with no arm tag.
pub fn collect_root_union_problems(
    arm_validators: &[Arc<Validator>],
    instance: &Value,
    positions: &PositionMap,
) -> Vec<ValidationProblem> {
    collect_root_union_problems_with_anchors(
        arm_validators,
        instance,
        positions,
        FileRefAnchors::default(),
    )
}

/// Like [`collect_root_union_problems`] but carries the document-first /
/// launch-area anchors for `format: darkmatter-file` diagnostic re-resolution.
pub(super) fn collect_root_union_problems_with_anchors(
    arm_validators: &[Arc<Validator>],
    instance: &Value,
    positions: &PositionMap,
    anchors: FileRefAnchors<'_>,
) -> Vec<ValidationProblem> {
    if arm_validators.is_empty() {
        return Vec::new();
    }
    let mut per_arm: Vec<Vec<ValidationProblem>> = Vec::with_capacity(arm_validators.len());
    for (idx, validator) in arm_validators.iter().enumerate() {
        let problems: Vec<ValidationProblem> = validator
            .iter_errors(instance)
            .flat_map(|err| build_problems(&err, positions, Some(idx), anchors))
            .collect();
        if problems.is_empty() {
            // Instance satisfies this arm — overall validation passes.
            return Vec::new();
        }
        per_arm.push(problems);
    }
    // Closest-matching arm: the one with the fewest problems. Ties broken by
    // arm order (stable: smaller index wins).
    per_arm
        .into_iter()
        .enumerate()
        .min_by_key(|(idx, problems)| (problems.len(), *idx))
        .map(|(_, problems)| problems)
        .unwrap_or_default()
}

/// Maps one `jsonschema::ValidationError` into one-or-more public problems.
///
/// Most errors map to a single problem. An `anyOf` failure is drilled into so
/// the precise offending sub-path surfaces instead of the parent: when the
/// `coerce`/`convert` pipeline wraps an optional typed property as
/// `anyOf: [{ "type": "null" }, <typed-fragment>]` (see
/// [`super::simplified::convert`]), a non-null value that fails the typed arm
/// otherwise reports only at the wrapper path with the generic "is not valid
/// under any of the schemas" message — pointing at the property instead of the
/// field that actually failed. We recurse into the arm context, keeping only
/// nested errors that drill *below* the wrapper path (this drops the `null`
/// sentinel's same-path type error), so an author sees `/config/name` rather
/// than `/config`. When no arm drills deeper (e.g. `[null, string]` against a
/// wrong-typed scalar — every arm fails at the wrapper path) the original
/// `anyOf` problem is kept.
fn build_problems(
    err: &jsonschema::ValidationError<'_>,
    positions: &PositionMap,
    arm_index: Option<usize>,
    anchors: FileRefAnchors<'_>,
) -> Vec<ValidationProblem> {
    if let ValidationErrorKind::AnyOf { context } = err.kind() {
        let parent = err.instance_path().as_str();
        let mut deeper: Vec<ValidationProblem> = context
            .iter()
            .flat_map(|arm| arm.iter())
            .filter(|nested| {
                let path = nested.instance_path().as_str();
                path.len() > parent.len() && path.starts_with(parent)
            })
            .flat_map(|nested| build_problems(nested, positions, arm_index, anchors))
            .collect();
        if !deeper.is_empty() {
            // Distinct arms can fail identically against the same value (e.g.
            // overlapping inline-object arms); collapse exact duplicates so the
            // report points at each offending site once. Duplicates need not be
            // adjacent, so dedup by a seen set rather than `dedup_by`.
            let mut seen: HashSet<(String, String)> = HashSet::new();
            deeper.retain(|p| seen.insert((p.path.clone(), p.message.clone())));
            return deeper;
        }
        let mut same_path_file_errors: Vec<ValidationProblem> = context
            .iter()
            .flat_map(|arm| arm.iter())
            .filter(|nested| {
                nested.instance_path().as_str() == parent && is_file_format_error(nested)
            })
            .flat_map(|nested| build_problems(nested, positions, arm_index, anchors))
            .collect();
        if !same_path_file_errors.is_empty() {
            let mut seen: HashSet<(String, String)> = HashSet::new();
            same_path_file_errors.retain(|p| seen.insert((p.path.clone(), p.message.clone())));
            return same_path_file_errors;
        }
        // Nullable scalar wrapper: `anyOf: [{ "type": "null" }, <typed atom>]`
        // (see [`super::simplified::convert::wrap_optional_null`]). A non-null
        // value that fails the typed arm at the *same* path — a format, pattern,
        // enum, or type mismatch, none of which drills deeper — otherwise
        // reports the opaque "is not valid under any of the schemas listed in
        // the 'anyOf' keyword" wrapper message. Surface the typed arm's precise
        // error instead, dropping the null sentinel's own "expected null" type
        // error. Scoped to the two-arm optional wrapper so multi-arm unions
        // (`numberlike`/`boolish`/root unions) keep their aggregate message.
        if context.len() == 2
            && context
                .iter()
                .any(|arm| arm.len() == 1 && is_null_type_sentinel(&arm[0]))
        {
            let mut typed: Vec<ValidationProblem> = context
                .iter()
                .flat_map(|arm| arm.iter())
                .filter(|nested| {
                    nested.instance_path().as_str() == parent && !is_null_type_sentinel(nested)
                })
                .flat_map(|nested| build_problems(nested, positions, arm_index, anchors))
                .collect();
            if !typed.is_empty() {
                let mut seen: HashSet<(String, String)> = HashSet::new();
                typed.retain(|p| seen.insert((p.path.clone(), p.message.clone())));
                return typed;
            }
        }
    }
    vec![build_problem(err, positions, arm_index, anchors)]
}

fn is_file_format_error(err: &jsonschema::ValidationError<'_>) -> bool {
    let ValidationErrorKind::Format { format } = err.kind() else {
        return false;
    };
    format == format::DARKMATTER_FILE_FORMAT
        || format == format::DARKMATTER_FILE_REFERENCE_FORMAT
}

/// Replaces the generic `"<value>" is not a "darkmatter-datetime"` /
/// `"darkmatter-time"` format-error text with a reader-facing ISO 8601
/// diagnostic. `None` for every other error kind (the caller keeps
/// `err.to_string()`).
fn datetime_format_message(err: &jsonschema::ValidationError<'_>) -> Option<String> {
    let ValidationErrorKind::Format { format } = err.kind() else {
        return None;
    };
    let noun = match format.as_str() {
        format::DARKMATTER_DATETIME_FORMAT => "an ISO-8601 datetime",
        format::DARKMATTER_TIME_FORMAT => "an ISO-8601 time",
        _ => return None,
    };
    Some(format!("{} is not {noun}", err.instance()))
}

/// `true` when `err` is the "expected null" type error contributed by the
/// `{ "type": "null" }` arm of an optional-property wrapper against a non-null
/// value.
fn is_null_type_sentinel(err: &jsonschema::ValidationError<'_>) -> bool {
    matches!(
        err.kind(),
        ValidationErrorKind::Type {
            kind: TypeKind::Single(JsonType::Null)
        }
    )
}

fn build_problem(
    err: &jsonschema::ValidationError<'_>,
    positions: &PositionMap,
    arm_index: Option<usize>,
    anchors: FileRefAnchors<'_>,
) -> ValidationProblem {
    let path = err.instance_path().as_str().to_string();
    let key = identify_key(&path, err.kind());
    let property = match err.kind() {
        ValidationErrorKind::Required { property } => property.as_str().map(str::to_string),
        _ => None,
    };
    let kind = classify_kind(err.kind());
    let code = classify_code(err.kind());
    let offending_property = offending_property_of(err.kind());
    let (line, column) = key
        .as_deref()
        .and_then(|k| positions.get(k).copied())
        .map(|(l, c)| (Some(l), Some(c)))
        .unwrap_or((None, None));
    let (message, file_reference) = match darkmatter_file_reference_outcome(err, anchors) {
        Some((message, diagnostic)) => (message, Some(diagnostic)),
        None => (
            datetime_format_message(err).unwrap_or_else(|| err.to_string()),
            None,
        ),
    };
    let instance_path = JsonPointer::parse(&path);
    let schema_path = {
        let raw = err.schema_path().as_str();
        (!raw.is_empty()).then(|| JsonPointer::parse(raw))
    };
    ValidationProblem {
        path,
        message,
        kind,
        property,
        line,
        column,
        arm_index,
        description: None,
        code,
        instance_path,
        schema_path,
        offending_property,
        file_reference,
    }
}

/// Maps a `jsonschema::ValidationErrorKind` onto the fine-grained
/// [`ValidationProblemCode`] (R-5 Priority 1). Unlike [`classify_kind`], this
/// separates unknown keys and file-reference failures from the generic
/// constraint bucket.
fn classify_code(kind: &ValidationErrorKind) -> ValidationProblemCode {
    match kind {
        ValidationErrorKind::Required { .. } => ValidationProblemCode::MissingRequired,
        ValidationErrorKind::Type { .. } => ValidationProblemCode::TypeMismatch,
        ValidationErrorKind::AdditionalProperties { .. }
        | ValidationErrorKind::UnevaluatedProperties { .. } => ValidationProblemCode::UnknownKey,
        ValidationErrorKind::Format { format } if is_darkmatter_file_format_name(format) => {
            ValidationProblemCode::InvalidFileReference
        }
        _ => ValidationProblemCode::ConstraintViolation,
    }
}

fn is_darkmatter_file_format_name(format: &str) -> bool {
    format == format::DARKMATTER_FILE_FORMAT || format == format::DARKMATTER_FILE_REFERENCE_FORMAT
}

/// The specific undeclared key for an `additionalProperties` /
/// `unevaluatedProperties` failure, whose instance path points at the *parent*
/// object. `None` for every other error kind.
fn offending_property_of(kind: &ValidationErrorKind) -> Option<String> {
    match kind {
        ValidationErrorKind::AdditionalProperties { unexpected }
        | ValidationErrorKind::UnevaluatedProperties { unexpected } => unexpected.first().cloned(),
        _ => None,
    }
}

/// Document-first / launch-area-fallback anchors used to re-resolve a
/// `format: darkmatter-file` value when substituting a targeted diagnostic.
///
/// Mirrors the anchors the compiled validator was built with so the
/// re-resolution reproduces the same failure mode rather than resolving
/// against an unrelated (e.g. ambient-CWD) directory.
#[derive(Clone, Copy, Default)]
pub(super) struct FileRefAnchors<'a> {
    /// Prompt document directory, tried first.
    pub base_dir: Option<&'a Path>,
    /// Captured launch-area fallback, tried second.
    pub fallback: Option<&'a Path>,
}

/// Returns the substituted message *and* the structured
/// [`FileReferenceDiagnostic`] classification for `format: darkmatter-file`
/// (eager) and `format: darkmatter-file-reference` (lazy) failures, so users
/// see a targeted diagnostic instead of the generic `"<value>" is not a
/// "<format>"` text produced by `jsonschema`. The message is preserved
/// byte-for-byte; the classification rides alongside on the problem (R-5
/// Priority 4).
///
/// The two formats fail for different reasons, so the substitution differs:
///
/// - **eager `darkmatter-file`** — re-runs [`format::resolve_file_reference`]
///   against the failing instance string using the same `anchors` the
///   validator was built with, reproducing the exact existence/resolution
///   failure (invalid syntax, unresolvable context, or no matching file).
/// - **lazy `darkmatter-file-reference`** — the lazy validator only fails on
///   malformed syntax (it never resolves or checks existence), so the targeted
///   diagnostic is purely the [`FileReference::new`] parse error. No
///   resolution is attempted here, keeping the lazy contract intact.
///
/// The format validator already ran during `jsonschema` validation (returning
/// `false` on failure), so the outcome here reproduces the exact path that
/// produced the `false`. Returns `None` for non-string instances, unrelated
/// formats, or any other error kind so the caller falls back to
/// `err.to_string()`.
fn darkmatter_file_reference_outcome(
    err: &jsonschema::ValidationError<'_>,
    anchors: FileRefAnchors<'_>,
) -> Option<(String, FileReferenceDiagnostic)> {
    let ValidationErrorKind::Format { format } = err.kind() else {
        return None;
    };
    let instance = err.instance();
    let Value::String(value) = instance.as_ref() else {
        return None;
    };
    if format == format::DARKMATTER_FILE_FORMAT {
        return match format::resolve_file_reference(value, anchors.base_dir, anchors.fallback) {
            Ok(_) => None,
            // The rendered `message` is preserved byte-for-byte from the
            // failure's `Display`; the structured classification rides
            // alongside on the problem (R-5 Priority 4).
            Err(failure) => Some((failure.to_string(), file_reference_diagnostic(&failure))),
        };
    }
    if format == format::DARKMATTER_FILE_REFERENCE_FORMAT {
        // Lazy: the only way to reach here is a malformed reference. Report the
        // construction error verbatim, without resolving or touching the
        // filesystem.
        return match biscuit_file::FileReference::new(value) {
            Ok(_) => None,
            Err(err) => Some((
                format!("`{value}` is not a valid file reference: {err}"),
                FileReferenceDiagnostic::InvalidSyntax {
                    raw: value.to_string(),
                },
            )),
        };
    }
    None
}

/// Maps the crate-private [`format::FileReferenceFailure`] to the public
/// [`FileReferenceDiagnostic`] classification.
fn file_reference_diagnostic(failure: &format::FileReferenceFailure) -> FileReferenceDiagnostic {
    match failure {
        format::FileReferenceFailure::InvalidSyntax { raw, .. } => {
            FileReferenceDiagnostic::InvalidSyntax { raw: raw.clone() }
        }
        format::FileReferenceFailure::Resolution { raw, .. } => {
            FileReferenceDiagnostic::ResolutionFailed { raw: raw.clone() }
        }
        format::FileReferenceFailure::NoMatch { raw, cwd } => FileReferenceDiagnostic::NoMatch {
            raw: raw.clone(),
            resolved_from: cwd.clone(),
        },
    }
}

/// Maps a `jsonschema::ValidationErrorKind` onto the coarse
/// [`ValidationProblemKind`] used by renderers.
///
/// Renderers must not infer the category from `property.is_some()` or by
/// substring-matching `message`; both lose information as soon as the
/// validator grows new error shapes.
fn classify_kind(kind: &ValidationErrorKind) -> ValidationProblemKind {
    match kind {
        ValidationErrorKind::Required { .. } => ValidationProblemKind::Missing,
        ValidationErrorKind::Type { .. } => ValidationProblemKind::Type,
        _ => ValidationProblemKind::Invalid,
    }
}

/// Top-level frontmatter key a validation error is attributable to, if any.
///
/// Mirrors the attribution [`collect_problems`] uses (the missing-property name
/// for `Required`, otherwise the first JSON-pointer segment), so callers that
/// reason about which key caused a failure stay consistent with reported
/// problems without re-deriving the rule.
pub(super) fn error_top_level_key(err: &jsonschema::ValidationError<'_>) -> Option<String> {
    identify_key(err.instance_path().as_str(), err.kind())
}

/// Picks the top-level frontmatter key to attribute a problem to.
///
/// - For `Required` failures the key comes from the error kind (the path
///   points at the parent object, not the missing property).
/// - Otherwise the first JSON-pointer segment of `path` is used.
fn identify_key(path: &str, kind: &ValidationErrorKind) -> Option<String> {
    if let ValidationErrorKind::Required { property } = kind
        && let Some(name) = property.as_str()
    {
        return Some(name.to_string());
    }
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let segment = trimmed.split('/').next()?;
    Some(unescape_pointer_segment(segment))
}

fn unescape_pointer_segment(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

/// Resolves the declared description for `problem` by walking `root` (the
/// compiled JSON Schema) along the problem's instance path to the failing
/// property node and reading its `description`.
///
/// Returns `None` when the property declares no description or when the path
/// cannot be resolved. The walk is intentionally defensive: any missing or
/// mis-shaped node yields `None` rather than panicking, so an unresolvable path
/// degrades to "no description" instead of a wrong one.
///
/// ## Notes
///
/// The base node is the winning root-union arm (`root["anyOf"][arm_index]`)
/// when `problem.arm_index` is `Some`, otherwise `root` itself. Descent unwraps
/// nullable `anyOf` wrappers, treats numeric segments as `items`, and reads the
/// description (or a property-level-union articulation) at the final node.
pub(super) fn resolve_problem_description(
    root: &Value,
    problem: &ValidationProblem,
) -> Option<String> {
    // Decision #10: an `additionalProperties: false` failure points its path at
    // the *parent* object, but the offending key is undeclared — reusing the
    // parent's description would misidentify what the author must fix.
    if problem.message.starts_with("Additional properties are not allowed") {
        return None;
    }

    let mut node = match problem.arm_index {
        Some(idx) => root.get("anyOf")?.as_array()?.get(idx)?,
        None => root,
    };

    let mut segments: Vec<String> = problem
        .path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(unescape_pointer_segment)
        .collect();

    // Decision #3: a `Missing` problem's path points at the parent object; the
    // missing property name lives in `problem.property`.
    if problem.kind == ValidationProblemKind::Missing
        && let Some(property) = &problem.property
    {
        segments.push(property.clone());
    }

    for segment in &segments {
        // Decision #5: step into the non-null arm of a nullable wrapper before
        // descending into `properties` / `items`.
        node = unwrap_nullable_arm(node);
        node = if is_numeric_segment(segment) {
            node.get("items")? // Decision #4
        } else {
            node.get("properties")?.get(segment.as_str())?
        };
    }

    description_at_node(node)
}

/// Reads the description at the resolved node: a direct `description` string,
/// else a property-level-union articulation (Decision #6), else `None`.
fn description_at_node(node: &Value) -> Option<String> {
    if let Some(desc) = node.get("description").and_then(Value::as_str) {
        return Some(desc.to_string());
    }
    if let Some(arms) = node.get("anyOf").and_then(Value::as_array) {
        let non_null: Vec<&Value> = arms.iter().filter(|arm| !is_null_schema(arm)).collect();
        // A lone non-null arm is just a nullable wrapper, not a union worth
        // articulating; only two-or-more genuine alternatives qualify.
        if non_null.len() >= 2 {
            return articulate_union(&non_null);
        }
    }
    None
}

/// Articulates a property-level union (Decision #6) over its non-null arms.
///
/// `|D| == 1` returns the lone arm's description verbatim (it speaks for the
/// whole union); `|D| >= 2` and `|D| == 0` synthesize `a union type of: …`,
/// drawing on each arm's `description` or its [`type_label`]. Returns `None`
/// when no description and no derivable type label exist.
fn articulate_union(non_null_arms: &[&Value]) -> Option<String> {
    let described = non_null_arms.iter().filter(|arm| arm.get("description").is_some());
    let describe_count = described.count();

    if describe_count == 1 {
        return non_null_arms
            .iter()
            .find_map(|arm| arm.get("description").and_then(Value::as_str))
            .map(str::to_string);
    }

    let parts: Vec<String> = non_null_arms
        .iter()
        .filter_map(|arm| {
            arm.get("description")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| type_label(arm).map(str::to_string))
        })
        .collect();

    if parts.is_empty() {
        None
    } else {
        Some(format!("a union type of: {}", parts.join(" | ")))
    }
}

/// Maps a compiled arm schema to a short human-readable type keyword for union
/// articulation (display aid only; never affects validation). Returns `None`
/// when no label is derivable.
fn type_label(arm: &Value) -> Option<&'static str> {
    if arm.get("enum").is_some() || arm.get("const").is_some() {
        return Some("enum");
    }
    match arm.get("type").and_then(Value::as_str)? {
        "string" => Some("string"),
        "number" => Some("number"),
        "integer" => Some("integer"),
        "boolean" => Some("boolean"),
        "object" => Some("object"),
        "array" => Some("array"),
        _ => None,
    }
}

/// Returns the lone non-null arm of a nullable `anyOf` wrapper, else the node
/// unchanged. A union with two or more non-null arms is ambiguous to descend
/// into, so it is left as-is (the subsequent `properties`/`items` lookup will
/// fail and resolution degrades to `None`).
fn unwrap_nullable_arm(node: &Value) -> &Value {
    let Some(arms) = node.get("anyOf").and_then(Value::as_array) else {
        return node;
    };
    let mut non_null = arms.iter().filter(|arm| !is_null_schema(arm));
    match (non_null.next(), non_null.next()) {
        (Some(only), None) => only,
        _ => node,
    }
}

/// Whether `arm` is the `{ "type": "null" }` sentinel emitted for optional
/// (nullable) properties.
fn is_null_schema(arm: &Value) -> bool {
    arm.get("type").and_then(Value::as_str) == Some("null")
}

fn is_numeric_segment(segment: &str) -> bool {
    !segment.is_empty() && segment.bytes().all(|b| b.is_ascii_digit())
}

/// Builds a top-level YAML key → (line, column) map by scanning the
/// serialised frontmatter line by line.
///
/// Only top-level keys (column 1, not part of a comment or list item) are
/// captured. Returned coordinates are 1-based. Quoted keys (`"foo"`,
/// `'foo'`) are normalised to the bare key string.
pub fn build_position_map(yaml: &str) -> PositionMap {
    let mut out: PositionMap = PositionMap::new();
    for (idx, line) in yaml.lines().enumerate() {
        let Some(first) = line.chars().next() else {
            continue;
        };
        if first.is_whitespace() || first == '#' || first == '-' || first == '.' {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let raw = line[..colon].trim();
        let key = raw
            .trim_start_matches(['"', '\''])
            .trim_end_matches(['"', '\''])
            .to_string();
        if !key.is_empty() {
            out.insert(key, (idx as u32 + 1, 1));
        }
    }
    out
}

/// Adds the canonical `$schema` URI to an `anyOf` arm so the cache can
/// compile it as a standalone validator.
pub fn wrap_arm_as_root_schema(arm: &Value) -> Value {
    let mut map = arm.as_object().cloned().unwrap_or_default();
    if !map.contains_key("$schema") {
        map.insert(
            "$schema".into(),
            Value::String(super::simplified::DRAFT_2020_12.to_string()),
        );
    }
    Value::Object(map)
}

/// Collects problems for a single already-selected root-union arm, tagging each
/// with `arm_index`. Used to narrow a discriminated **root** union to the arm a
/// shared literal discriminant selects, so only that arm's problems surface
/// (instead of the closest-by-count arm the general union report picks).
pub(super) fn collect_arm_problems_with_anchors(
    validator: &Validator,
    instance: &Value,
    positions: &PositionMap,
    anchors: FileRefAnchors<'_>,
    arm_index: usize,
) -> Vec<ValidationProblem> {
    validator
        .iter_errors(instance)
        .flat_map(|err| build_problems(&err, positions, Some(arm_index), anchors))
        .collect()
}

/// Narrows discriminated **property-level** unions to their selected arm.
///
/// For each top-level property whose schema is an `anyOf` union that a shared
/// literal discriminant selects unambiguously
/// ([`super::discriminant::select_literal_discriminant_arm`]), the problems
/// under that property are replaced with the result of validating the instance
/// against a synthetic single-property schema carrying only the chosen arm — so
/// only the matched arm's missing/unknown/type problems surface. Every property
/// without such a selection keeps its original `anyOf` diagnostics
/// byte-for-byte.
///
/// Validating the whole instance against `{ "properties": { key: arm } }`
/// (rather than the sub-value against the bare arm) keeps problem paths in the
/// `/key/...` form the normal report and the `positions` map already use.
pub(super) fn narrow_property_union_problems(
    schema: &Value,
    instance: &Value,
    positions: &PositionMap,
    anchors: FileRefAnchors<'_>,
    mut problems: Vec<ValidationProblem>,
) -> Vec<ValidationProblem> {
    let (Some(props), Some(obj)) = (
        schema.get("properties").and_then(Value::as_object),
        instance.as_object(),
    ) else {
        return problems;
    };
    for (key, prop_schema) in props {
        let Some(arms) = prop_schema.get("anyOf").and_then(Value::as_array) else {
            continue;
        };
        let Some(value) = obj.get(key).filter(|value| !value.is_null()) else {
            continue;
        };
        let Some(arm_index) = super::discriminant::select_literal_discriminant_arm(arms, value)
        else {
            continue;
        };
        let synthetic = serde_json::json!({
            "$schema": super::simplified::DRAFT_2020_12,
            "type": "object",
            "properties": { key: arms[arm_index] }
        });
        let Ok(validator) = build_validator(&synthetic, anchors.base_dir, anchors.fallback) else {
            continue;
        };
        let narrowed: Vec<ValidationProblem> = validator
            .iter_errors(instance)
            .flat_map(|err| build_problems(&err, positions, None, anchors))
            .collect();
        let prefix = json_pointer_segment(key);
        problems.retain(|problem| !path_is_under(&problem.path, &prefix));
        problems.extend(narrowed);
    }
    problems
}

/// The JSON-pointer prefix (`/<escaped-key>`) for a top-level property.
fn json_pointer_segment(key: &str) -> String {
    format!("/{}", key.replace('~', "~0").replace('/', "~1"))
}

/// Whether `path` names the property at `prefix` or a descendant of it.
fn path_is_under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn default_capacity() -> usize {
    static CAP: OnceLock<usize> = OnceLock::new();
    *CAP.get_or_init(|| {
        std::env::var(CACHE_SIZE_ENV)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .map(|n| n.max(1))
            .unwrap_or(DEFAULT_CACHE_SIZE)
    })
}

/// SHA-256 of the canonicalised JSON Schema bytes (and the document
/// `base_dir`) used as the cache key.
///
/// `base_dir` is folded in because it parameterizes the compiled
/// `format: darkmatter-file` validator: the same schema validated for two
/// documents in different directories must not share a validator, or one
/// document's `file` values would resolve against the other's directory.
fn canonical_hash(schema: &Value, base_dir: Option<&Path>) -> [u8; 32] {
    // `serde_json::to_vec` is stable per the active feature set; this is
    // sufficient for cache identity (false misses are tolerable, false hits
    // are not — which `to_vec` guarantees because identical Values
    // serialise to identical bytes).
    let bytes = serde_json::to_vec(schema).expect("schema serialises to JSON");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    // Domain-separate the base_dir from the schema bytes so a schema ending in
    // bytes that collide with a path prefix cannot alias a different (schema,
    // base_dir) pair.
    hasher.update([0xff]);
    if let Some(dir) = base_dir {
        hasher.update(dir.to_string_lossy().as_bytes());
    }
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn trivial_schema() -> Value {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        })
    }

    #[test]
    fn cache_caches_validator_by_schema_identity() {
        let cache = ValidatorCache::with_capacity(4);
        let schema = trivial_schema();
        let v1 = cache.validator_for(&schema, None).unwrap();
        let v2 = cache.validator_for(&schema, None).unwrap();
        assert!(Arc::ptr_eq(&v1, &v2));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_evicts_when_capacity_exceeded() {
        let cache = ValidatorCache::with_capacity(2);
        for i in 0..5 {
            let schema = json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": { format!("p{i}"): { "type": "string" } }
            });
            cache.validator_for(&schema, None).unwrap();
        }
        assert!(cache.len() <= 2);
    }

    #[test]
    fn cache_zero_capacity_is_promoted_to_one() {
        let cache = ValidatorCache::with_capacity(0);
        let schema = trivial_schema();
        cache.validator_for(&schema, None).unwrap();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_eviction_keeps_recent_entry_alive() {
        let cache = ValidatorCache::with_capacity(2);
        let s1 = json!({"type":"object","properties":{"a":{"type":"string"}}});
        let s2 = json!({"type":"object","properties":{"b":{"type":"string"}}});
        let s3 = json!({"type":"object","properties":{"c":{"type":"string"}}});
        let v1 = cache.validator_for(&s1, None).unwrap();
        cache.validator_for(&s2, None).unwrap();
        // Touch s1 so it's the more recent of the two existing entries.
        let v1_again = cache.validator_for(&s1, None).unwrap();
        assert!(Arc::ptr_eq(&v1, &v1_again));
        // Add s3 — s2 should be evicted as least-recently-used.
        cache.validator_for(&s3, None).unwrap();
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn classify_kind_maps_required_to_missing() {
        let schema = json!({
            "type": "object",
            "properties": { "title": { "type": "string" } },
            "required": ["title"]
        });
        let validator = build_validator(&schema, None, None).unwrap();
        let problems = collect_problems(&validator, &json!({}), &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected one Required error: {problems:?}");
        assert_eq!(problems[0].kind, ValidationProblemKind::Missing);
        assert_eq!(problems[0].property.as_deref(), Some("title"));
    }

    #[test]
    fn classify_kind_maps_type_to_type() {
        let schema = json!({
            "type": "object",
            "properties": { "count": { "type": "number" } }
        });
        let validator = build_validator(&schema, None, None).unwrap();
        let problems = collect_problems(
            &validator,
            &json!({ "count": "not-a-number" }),
            &PositionMap::new(),
        );
        assert_eq!(problems.len(), 1, "expected one Type error: {problems:?}");
        assert_eq!(problems[0].kind, ValidationProblemKind::Type);
    }

    #[test]
    fn classify_kind_maps_other_constraints_to_invalid() {
        let schema = json!({
            "type": "object",
            "properties": { "name": { "type": "string", "minLength": 5 } }
        });
        let validator = build_validator(&schema, None, None).unwrap();
        let problems = collect_problems(
            &validator,
            &json!({ "name": "no" }),
            &PositionMap::new(),
        );
        assert_eq!(problems.len(), 1, "expected one MinLength error: {problems:?}");
        assert_eq!(problems[0].kind, ValidationProblemKind::Invalid);
    }

    #[test]
    fn nullable_any_of_surfaces_nested_failing_subpath() {
        // Mirrors the `convert` output for an optional inline-object property:
        // `anyOf: [{ "type": "null" }, <object with required string field>]`.
        // A non-null value that fails the typed arm must report the nested
        // `/config/name` site, not the wrapper path `/config`.
        let schema = json!({
            "type": "object",
            "properties": {
                "config": { "anyOf": [
                    { "type": "null" },
                    {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": { "name": { "type": "string" } },
                        "required": ["name"]
                    }
                ] }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "config": { "name": ["Ada", "Lovelace"] } });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert!(
            problems.iter().any(|p| p.path == "/config/name"),
            "expected a problem under /config/name, got {problems:?}"
        );
    }

    #[test]
    fn nullable_any_of_keeps_parent_when_no_arm_drills_deeper() {
        // `[null, string]` against a wrong-typed scalar: every arm fails at the
        // wrapper path itself, so there is no deeper site to point at. The
        // typed arm's precise error is surfaced at the wrapper path (dropping
        // the null sentinel's "expected null" type error) — a single problem at
        // `/name`, not the opaque `anyOf` wrapper message.
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "anyOf": [ { "type": "null" }, { "type": "string" } ] }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "name": 42 });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected the wrapper problem: {problems:?}");
        assert_eq!(problems[0].path, "/name");
        assert!(
            !problems[0].message.contains("anyOf"),
            "generic anyOf message should be replaced: {:?}",
            problems[0].message
        );
    }

    #[test]
    fn optional_datetime_accepts_offset_less_value() {
        // Mirrors `convert`'s optional-`datetime` output. The offset-less local
        // datetime is valid ISO 8601 and must not error (it did under the
        // built-in RFC 3339 `date-time` format).
        let schema = json!({
            "type": "object",
            "properties": {
                "created": { "anyOf": [
                    { "type": "null" },
                    { "type": "string", "format": "darkmatter-datetime" }
                ] }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "created": "2026-07-10T15:05:34" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert!(problems.is_empty(), "expected no problems, got {problems:?}");
    }

    #[test]
    fn optional_datetime_reports_clean_message_for_garbage() {
        let schema = json!({
            "type": "object",
            "properties": {
                "created": { "anyOf": [
                    { "type": "null" },
                    { "type": "string", "format": "darkmatter-datetime" }
                ] }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "created": "2026-13-99T25:61:61" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected one problem: {problems:?}");
        assert_eq!(problems[0].path, "/created");
        assert!(
            problems[0].message.contains("is not an ISO-8601 datetime"),
            "expected a reader-facing datetime message, got {:?}",
            problems[0].message
        );
        assert!(
            !problems[0].message.contains("anyOf"),
            "generic anyOf message should be replaced: {:?}",
            problems[0].message
        );
    }

    #[test]
    fn build_validator_accepts_simplified_output() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": true,
            "properties": {
                "title": { "type": "string" }
            },
            "required": ["title"]
        });
        let v = build_validator(&schema, None, None).unwrap();
        assert!(v.is_valid(&json!({ "title": "x" })));
        assert!(!v.is_valid(&json!({})));
    }

    #[test]
    fn build_validator_rejects_bad_schema() {
        let schema = json!({ "type": 42 });
        let err = build_validator(&schema, None, None).unwrap_err();
        let SchemaError::BuildValidator { message } = &err else {
            panic!("expected BuildValidator, got {err:?}");
        };
        assert!(!message.is_empty());
    }

    #[test]
    fn collect_problems_surfaces_path_and_message() {
        let schema = json!({
            "type": "object",
            "properties": {
                "n": { "type": "number" }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "n": "not-a-number" });
        let positions = PositionMap::new();
        let problems = collect_problems(&v, &instance, &positions);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].path, "/n");
        assert!(!problems[0].message.is_empty());
    }

    #[test]
    fn collect_problems_populates_line_for_type_mismatch() {
        let schema = json!({
            "type": "object",
            "properties": {
                "n": { "type": "number" }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "n": "nope" });
        let mut positions = PositionMap::new();
        positions.insert("n".into(), (3, 1));
        let problems = collect_problems(&v, &instance, &positions);
        assert_eq!(problems[0].line, Some(3));
        assert_eq!(problems[0].column, Some(1));
    }

    #[test]
    fn collect_problems_populates_property_for_missing_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            },
            "required": ["title"]
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({});
        let positions = PositionMap::new();
        let problems = collect_problems(&v, &instance, &positions);
        assert_eq!(problems[0].path, "");
        assert_eq!(problems[0].property.as_deref(), Some("title"));
    }

    #[test]
    fn collect_problems_leaves_property_none_for_non_required_failures() {
        let schema = json!({
            "type": "object",
            "properties": {
                "n": { "type": "number" }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "n": "not-a-number" });
        let positions = PositionMap::new();
        let problems = collect_problems(&v, &instance, &positions);
        assert_eq!(problems[0].property, None);
    }

    #[test]
    fn collect_problems_populates_line_for_missing_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            },
            "required": ["title"]
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({});
        let mut positions = PositionMap::new();
        positions.insert("title".into(), (5, 1));
        let problems = collect_problems(&v, &instance, &positions);
        assert_eq!(problems[0].line, Some(5));
    }

    #[test]
    fn build_position_map_captures_top_level_keys() {
        let yaml = "title: hi\nbody: stuff\n  nested: skip\n";
        let map = build_position_map(yaml);
        assert_eq!(map.get("title"), Some(&(1, 1)));
        assert_eq!(map.get("body"), Some(&(2, 1)));
        assert!(map.get("nested").is_none());
    }

    #[test]
    fn root_union_picks_closest_arm() {
        // Arm 0 needs `a` and `b`; arm 1 needs only `c`. An instance with
        // only `c` produces 0 problems against arm 1 but 2 against arm 0 —
        // arm 1 wins.
        let arm0 = json!({
            "$schema": super::super::simplified::DRAFT_2020_12,
            "type":"object",
            "properties": {"a":{"type":"string"},"b":{"type":"string"}},
            "required":["a","b"]
        });
        let arm1 = json!({
            "$schema": super::super::simplified::DRAFT_2020_12,
            "type":"object",
            "properties": {"c":{"type":"string"}},
            "required":["c"]
        });
        let v0 = Arc::new(build_validator(&arm0, None, None).unwrap());
        let v1 = Arc::new(build_validator(&arm1, None, None).unwrap());
        let instance = json!({"c": "x"});
        let problems = collect_root_union_problems(&[v0, v1], &instance, &PositionMap::new());
        assert!(problems.is_empty(), "expected match: {problems:?}");
    }

    #[test]
    fn root_union_tags_arm_index_when_no_arm_matches() {
        let arm0 = json!({
            "type":"object",
            "properties": {"a":{"type":"string"}},
            "required":["a"]
        });
        let arm1 = json!({
            "type":"object",
            "properties": {"b":{"type":"string"}},
            "required":["b","c"]
        });
        let v0 = Arc::new(build_validator(&arm0, None, None).unwrap());
        let v1 = Arc::new(build_validator(&arm1, None, None).unwrap());
        let instance = json!({});
        let problems = collect_root_union_problems(&[v0, v1], &instance, &PositionMap::new());
        assert!(!problems.is_empty());
        assert!(problems.iter().all(|p| p.arm_index == Some(0)));
    }

    // The `darkmatter-file` format tests below mutate the process CWD so the
    // `FileReference` resolver sees a deterministic filesystem state. They are
    // serialised with `serial_test::serial("darkmatter-file-cwd")` so this
    // module and `format::tests` share the same process-global lock.

    struct FileFormatCwdGuard {
        prior: std::path::PathBuf,
    }

    impl FileFormatCwdGuard {
        fn enter(dir: &std::path::Path) -> Self {
            let prior = std::env::current_dir().expect("read CWD");
            std::env::set_current_dir(dir).expect("set CWD");
            Self { prior }
        }
    }

    impl Drop for FileFormatCwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prior);
        }
    }

    fn darkmatter_file_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "doc": { "type": "string", "format": "darkmatter-file" }
            }
        })
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn darkmatter_file_format_error_surfaces_no_match_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let v = build_validator(&darkmatter_file_schema(), None, None).unwrap();
        let instance = json!({ "doc": "./does-not-exist.md" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected one Format problem: {problems:?}");
        let problem = &problems[0];
        assert_eq!(problem.path, "/doc");
        assert_eq!(problem.kind, ValidationProblemKind::Invalid);
        assert!(
            problem.message.contains("no existing file matched reference"),
            "expected NoMatch-style message, got: {}",
            problem.message,
        );
        assert!(
            problem.message.contains("`./does-not-exist.md`"),
            "expected instance value in message, got: {}",
            problem.message,
        );
        assert!(
            !problem.message.contains(r#"is not a "darkmatter-file""#),
            "generic jsonschema message should have been replaced, got: {}",
            problem.message,
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn darkmatter_file_format_error_surfaces_invalid_syntax_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let v = build_validator(&darkmatter_file_schema(), None, None).unwrap();
        // An empty string is rejected at parse time.
        let instance = json!({ "doc": "" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected one Format problem: {problems:?}");
        let problem = &problems[0];
        assert_eq!(problem.path, "/doc");
        assert!(
            problem.message.contains("is not a valid file reference"),
            "expected InvalidSyntax message, got: {}",
            problem.message,
        );
        assert!(
            !problem.message.contains(r#"is not a "darkmatter-file""#),
            "generic jsonschema message should have been replaced, got: {}",
            problem.message,
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn darkmatter_file_format_succeeds_for_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("exists.md");
        std::fs::write(&path, b"x").unwrap();
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let v = build_validator(&darkmatter_file_schema(), None, None).unwrap();
        let instance = json!({ "doc": "./exists.md" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert!(
            problems.is_empty(),
            "expected no problems for existing file, got: {problems:?}",
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn non_darkmatter_file_format_keeps_default_message() {
        // A `format` other than `darkmatter-file` should not be intercepted
        // by the substitution — the default `jsonschema` text must flow
        // through unchanged.
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let schema = json!({
            "type": "object",
            "properties": {
                "v": { "type": "string", "format": "email" }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "v": "not-an-email" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected one Format problem: {problems:?}");
        let problem = &problems[0];
        assert!(
            problem.message.contains(r#"is not a "email""#),
            "expected unmodified default message, got: {}",
            problem.message,
        );
    }

    fn darkmatter_file_match_schema() -> Value {
        // `match(...)` is suggestion metadata only and is never lowered into
        // the compiled JSON Schema, so the eager existence behavior here comes
        // purely from `format: darkmatter-file`. The bare `x-darkmatter-match`
        // annotation is an unknown keyword that JSON Schema ignores.
        json!({
            "type": "object",
            "properties": {
                "doc": {
                    "type": "string",
                    "format": "darkmatter-file"
                }
            }
        })
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn darkmatter_file_match_missing_file_produces_one_file_reference_diagnostic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let v = build_validator(&darkmatter_file_match_schema(), None, None).unwrap();
        let instance = json!({ "doc": "./does-not-exist.md" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected exactly one file-reference diagnostic, got: {problems:?}"
        );
        let problem = &problems[0];
        assert_eq!(problem.path, "/doc");
        assert_eq!(problem.kind, ValidationProblemKind::Invalid);
        assert!(
            problem.message.contains("no existing file matched reference"),
            "expected NoMatch-style message, got: {}",
            problem.message,
        );
        assert!(
            problem.message.contains("`./does-not-exist.md`"),
            "expected instance value in message, got: {}",
            problem.message,
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn lazy_reference_format_missing_file_produces_no_diagnostic() {
        // The lazy `darkmatter-file-reference` is syntax-only: a syntactically
        // valid but not-yet-existing path validates, producing zero existence
        // diagnostics (the eager case above produces one).
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let schema = json!({
            "type": "object",
            "properties": {
                "doc": { "type": "string", "format": "darkmatter-file-reference" }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let problems = collect_problems(&v, &json!({ "doc": "./does-not-exist.md" }), &PositionMap::new());
        assert!(
            problems.is_empty(),
            "lazy reference of a missing file must not produce a diagnostic, got: {problems:?}"
        );
    }

    #[test]
    fn lazy_reference_format_malformed_input_produces_syntax_diagnostic() {
        // A malformed reference (empty string) is fatal even for the lazy
        // format, and the substituted message is the syntax error — never an
        // existence/resolution message, since the lazy validator does not
        // resolve.
        let schema = json!({
            "type": "object",
            "properties": {
                "doc": { "type": "string", "format": "darkmatter-file-reference" }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let problems = collect_problems(&v, &json!({ "doc": "" }), &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected one Format problem: {problems:?}");
        let problem = &problems[0];
        assert_eq!(problem.path, "/doc");
        assert!(
            problem.message.contains("is not a valid file reference"),
            "expected syntax message, got: {}",
            problem.message,
        );
        assert!(
            !problem.message.contains("no existing file matched reference"),
            "lazy diagnostic must not resolve or report existence, got: {}",
            problem.message,
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn darkmatter_file_format_error_retains_nested_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let schema = json!({
            "type": "object",
            "properties": {
                "doc": {
                    "type": "object",
                    "properties": {
                        "cover": { "type": "string", "format": "darkmatter-file" }
                    }
                }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "doc": { "cover": "./missing.md" } });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected one Format problem: {problems:?}");
        assert_eq!(problems[0].path, "/doc/cover");
        assert!(
            problems[0].message.contains("no existing file matched reference"),
            "expected improved message, got: {}",
            problems[0].message,
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn darkmatter_file_format_error_retains_array_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let schema = json!({
            "type": "object",
            "properties": {
                "docs": {
                    "type": "array",
                    "items": { "type": "string", "format": "darkmatter-file" }
                }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let instance = json!({ "docs": ["./missing.md"] });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(problems.len(), 1, "expected one Format problem: {problems:?}");
        assert_eq!(problems[0].path, "/docs/0");
        assert!(
            problems[0].message.contains("no existing file matched reference"),
            "expected improved message, got: {}",
            problems[0].message,
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn root_union_preserves_arm_index_for_file_format_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let arm0 = wrap_arm_as_root_schema(&json!({
            "type": "object",
            "properties": {
                "doc": { "type": "string", "format": "darkmatter-file" }
            },
            "required": ["doc"]
        }));
        let arm1 = wrap_arm_as_root_schema(&json!({
            "type": "object",
            "properties": {
                "doc": { "type": "number" }
            },
            "required": ["doc"]
        }));
        let v0 = Arc::new(build_validator(&arm0, None, None).unwrap());
        let v1 = Arc::new(build_validator(&arm1, None, None).unwrap());
        let instance = json!({ "doc": "./missing.md" });
        let problems = collect_root_union_problems(&[v0, v1], &instance, &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected one problem from the closest matching arm, got: {problems:?}"
        );
        let problem = &problems[0];
        assert_eq!(problem.arm_index, Some(0));
        assert_eq!(problem.path, "/doc");
        assert!(
            problem.message.contains("no existing file matched reference"),
            "expected improved file-reference message in root-union arm, got: {}",
            problem.message,
        );
    }

    // --- description resolution (resolve_problem_description) ---

    fn problem(
        path: &str,
        kind: ValidationProblemKind,
        property: Option<&str>,
        arm_index: Option<usize>,
    ) -> ValidationProblem {
        ValidationProblem {
            path: path.to_string(),
            message: "msg".to_string(),
            kind,
            property: property.map(str::to_string),
            line: None,
            column: None,
            arm_index,
            description: None,
            code: ValidationProblemCode::ConstraintViolation,
            instance_path: JsonPointer::parse(path),
            schema_path: None,
            offending_property: None,
            file_reference: None,
        }
    }

    #[test]
    fn resolve_top_level_type_reads_property_description() {
        let root = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "The headline" }
            }
        });
        let p = problem("/title", ValidationProblemKind::Type, None, None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("The headline")
        );
    }

    #[test]
    fn resolve_missing_reads_through_properties_property() {
        // A Required failure points its path at the parent object; the missing
        // name lives in `property` (Decision #3).
        let root = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "The headline" }
            },
            "required": ["title"]
        });
        let p = problem("", ValidationProblemKind::Missing, Some("title"), None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("The headline")
        );
    }

    #[test]
    fn resolve_decodes_json_pointer_escapes() {
        // A schema key containing literal `/` and `~` is addressed via `~1`/`~0`.
        let root = json!({
            "type": "object",
            "properties": {
                "a/b~c": { "type": "string", "description": "weird key" }
            }
        });
        let p = problem("/a~1b~0c", ValidationProblemKind::Type, None, None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("weird key")
        );
    }

    #[test]
    fn resolve_nested_inline_object_unwraps_nullable_anyof() {
        // `/config/name`: descend properties.config, unwrap the nullable
        // wrapper, then descend properties.name (Decision #5).
        let root = json!({
            "type": "object",
            "properties": {
                "config": { "anyOf": [
                    { "type": "null" },
                    {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "the name" }
                        }
                    }
                ] }
            }
        });
        let p = problem("/config/name", ValidationProblemKind::Type, None, None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("the name")
        );
    }

    #[test]
    fn resolve_array_path_descends_through_items() {
        // `/authors/0/name`: the numeric `0` descends into `items` (Decision #4).
        let root = json!({
            "type": "object",
            "properties": {
                "authors": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "author name" }
                        }
                    }
                }
            }
        });
        let p = problem("/authors/0/name", ValidationProblemKind::Type, None, None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("author name")
        );
    }

    #[test]
    fn resolve_nullable_optional_reads_wrapper_description() {
        // The optional property's description sits on the outer nullable
        // wrapper; it is read directly (Decision #5) — the lone non-null arm is
        // not articulated as a union.
        let root = json!({
            "type": "object",
            "properties": {
                "tag": {
                    "description": "an optional tag",
                    "anyOf": [ { "type": "null" }, { "type": "string" } ]
                }
            }
        });
        let p = problem("/tag", ValidationProblemKind::Type, None, None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("an optional tag")
        );
    }

    #[test]
    fn resolve_union_single_description_used_verbatim() {
        // Decision #6, |D| == 1: the lone description speaks for the union; no
        // `a union type of:` wrapper.
        let root = json!({
            "type": "object",
            "properties": {
                "width": { "anyOf": [
                    { "type": "number", "description": "the element width" },
                    { "type": "string" }
                ] }
            }
        });
        let p = problem("/width", ValidationProblemKind::Invalid, None, None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("the element width")
        );
    }

    #[test]
    fn resolve_union_multiple_descriptions_articulated() {
        // Decision #6, |D| >= 2: arms with a description use it; a third,
        // description-less arm falls back to its type label.
        let root = json!({
            "type": "object",
            "properties": {
                "payload": { "anyOf": [
                    { "type": "string", "description": "a raw message body" },
                    { "type": "array", "description": "a list of templated messages" },
                    { "type": "object" }
                ] }
            }
        });
        let p = problem("/payload", ValidationProblemKind::Invalid, None, None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("a union type of: a raw message body | a list of templated messages | object")
        );
    }

    #[test]
    fn resolve_union_no_descriptions_uses_type_labels() {
        // Decision #6, |D| == 0: articulate from type labels alone.
        let root = json!({
            "type": "object",
            "properties": {
                "value": { "anyOf": [
                    { "type": "number" },
                    { "type": "string" }
                ] }
            }
        });
        let p = problem("/value", ValidationProblemKind::Invalid, None, None);
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("a union type of: number | string")
        );
    }

    #[test]
    fn resolve_union_excludes_null_sentinel() {
        // The `{ "type": "null" }` sentinel is excluded from both the `D` count
        // and the articulation — so a nullable two-arm union with one
        // description reads that description verbatim, and a nullable two-arm
        // union with no descriptions is a lone non-null arm (not articulated).
        let described = json!({
            "type": "object",
            "properties": {
                "value": { "anyOf": [
                    { "type": "null" },
                    { "type": "number", "description": "a count" },
                    { "type": "string" }
                ] }
            }
        });
        let p = problem("/value", ValidationProblemKind::Invalid, None, None);
        assert_eq!(
            resolve_problem_description(&described, &p).as_deref(),
            Some("a count"),
            "null sentinel must not count toward D"
        );

        let labels_only = json!({
            "type": "object",
            "properties": {
                "value": { "anyOf": [
                    { "type": "null" },
                    { "type": "number" },
                    { "type": "string" }
                ] }
            }
        });
        assert_eq!(
            resolve_problem_description(&labels_only, &p).as_deref(),
            Some("a union type of: number | string"),
            "null sentinel must not appear in the articulation"
        );
    }

    #[test]
    fn resolve_property_without_description_is_none() {
        let root = json!({
            "type": "object",
            "properties": { "title": { "type": "string" } }
        });
        let p = problem("/title", ValidationProblemKind::Type, None, None);
        assert_eq!(resolve_problem_description(&root, &p), None);
    }

    #[test]
    fn resolve_additional_properties_failure_is_none() {
        // Decision #10: the resolver must not reuse the parent object's
        // description for an undeclared-key failure. The failure path points at
        // the parent (`/config`), which *does* carry a description here.
        let root = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "description": "the config object",
                    "additionalProperties": false,
                    "properties": { "name": { "type": "string" } }
                }
            }
        });
        let mut p = problem("/config", ValidationProblemKind::Invalid, None, None);
        p.message = "Additional properties are not allowed ('bogus' was unexpected)".to_string();
        assert_eq!(resolve_problem_description(&root, &p), None);
    }

    #[test]
    fn resolve_unresolvable_path_is_none_no_panic() {
        let root = json!({
            "type": "object",
            "properties": { "title": { "type": "string", "description": "x" } }
        });
        // Path descends into a property the schema does not declare.
        let p = problem("/missing/deeper", ValidationProblemKind::Type, None, None);
        assert_eq!(resolve_problem_description(&root, &p), None);
    }

    #[test]
    fn resolve_root_union_reads_winning_arm() {
        // arm_index: Some(1) resolves against anyOf[1] (Decision: base schema).
        let root = json!({
            "anyOf": [
                {
                    "type": "object",
                    "properties": { "doc": { "type": "string", "description": "arm 0 doc" } }
                },
                {
                    "type": "object",
                    "properties": { "doc": { "type": "number", "description": "arm 1 doc" } }
                }
            ]
        });
        let p = problem("/doc", ValidationProblemKind::Type, None, Some(1));
        assert_eq!(
            resolve_problem_description(&root, &p).as_deref(),
            Some("arm 1 doc")
        );
    }

    // --- R-5 Priority 1/4: fine-grained code, parsed paths, offending key ---

    #[test]
    fn code_and_instance_path_populated_for_type_mismatch() {
        let schema = json!({
            "type": "object",
            "properties": { "count": { "type": "number" } }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let problems = collect_problems(&v, &json!({ "count": "nope" }), &PositionMap::new());
        assert_eq!(problems.len(), 1, "{problems:?}");
        let p = &problems[0];
        assert_eq!(p.code, ValidationProblemCode::TypeMismatch);
        assert_eq!(p.instance_path.segments(), ["count"]);
        assert!(p.schema_path.is_some(), "type mismatch should carry a schema path");
        assert_eq!(p.offending_property, None);
        assert_eq!(p.file_reference, None);
    }

    #[test]
    fn code_missing_required_and_nested_instance_path() {
        let schema = json!({
            "type": "object",
            "properties": {
                "author": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                    "required": ["name"]
                }
            }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let problems = collect_problems(&v, &json!({ "author": {} }), &PositionMap::new());
        let missing = problems
            .iter()
            .find(|p| p.code == ValidationProblemCode::MissingRequired)
            .expect("missing-required problem");
        // The path points at the parent object; the missing name is `property`.
        assert_eq!(missing.instance_path.segments(), ["author"]);
        assert_eq!(missing.property.as_deref(), Some("name"));
    }

    #[test]
    fn unknown_key_code_and_offending_property() {
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "properties": { "known": { "type": "string" } }
        });
        let v = build_validator(&schema, None, None).unwrap();
        let problems = collect_problems(
            &v,
            &json!({ "known": "x", "bogus": 1 }),
            &PositionMap::new(),
        );
        let unknown = problems
            .iter()
            .find(|p| p.code == ValidationProblemCode::UnknownKey)
            .expect("unknown-key problem");
        assert_eq!(unknown.offending_property.as_deref(), Some("bogus"));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_reference_diagnostic_classifies_no_match() {
        let dir = tempfile::tempdir().unwrap();
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let v = build_validator(&darkmatter_file_schema(), None, None).unwrap();
        let problems =
            collect_problems(&v, &json!({ "doc": "./missing.md" }), &PositionMap::new());
        assert_eq!(problems.len(), 1, "{problems:?}");
        let p = &problems[0];
        assert_eq!(p.code, ValidationProblemCode::InvalidFileReference);
        match &p.file_reference {
            Some(FileReferenceDiagnostic::NoMatch { raw, .. }) => {
                assert_eq!(raw, "./missing.md");
            }
            other => panic!("expected NoMatch classification, got {other:?}"),
        }
        // The rendered message is unchanged by the added classification.
        assert!(p.message.contains("no existing file matched reference"));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_reference_diagnostic_classifies_invalid_syntax() {
        let dir = tempfile::tempdir().unwrap();
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let v = build_validator(&darkmatter_file_schema(), None, None).unwrap();
        let problems = collect_problems(&v, &json!({ "doc": "" }), &PositionMap::new());
        assert_eq!(problems.len(), 1, "{problems:?}");
        match &problems[0].file_reference {
            Some(FileReferenceDiagnostic::InvalidSyntax { raw }) => assert_eq!(raw, ""),
            other => panic!("expected InvalidSyntax classification, got {other:?}"),
        }
    }

    #[test]
    fn json_pointer_round_trips_escaped_segments() {
        let p = JsonPointer::parse("/a~1b~0c/2");
        assert_eq!(p.segments(), ["a/b~c", "2"]);
        assert_eq!(p.as_pointer_string(), "/a~1b~0c/2");
        assert!(JsonPointer::parse("").is_empty());
    }
}
