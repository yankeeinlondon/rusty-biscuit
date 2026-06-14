//! Validator construction and caching.
//!
//! This module is the bridge between a fully-formed JSON Schema
//! `serde_json::Value` and a `jsonschema::Validator` that can be exercised
//! against frontmatter data. Two responsibilities live here:
//!
//! - **Validator construction** — wires up Darkmatter's custom format
//!   ([`format::DARKMATTER_FILE_FORMAT`]) and keyword
//!   ([`format::DARKMATTER_MATCH_KEYWORD`],
//!   [`format::DARKMATTER_URL_SCHEME_KEYWORD`]) on top of Draft 2020-12.
//! - **Caching** — compiling a `Validator` is several milliseconds of work;
//!   the [`ValidatorCache`] hashes the canonicalised schema bytes and reuses
//!   compiled validators across calls. The default bound (64 entries) is
//!   adjustable via `DARKMATTER_SCHEMA_CACHE_SIZE`.
//!
//! Validation problems are mapped from `jsonschema::ValidationError` into the
//! public [`super::ValidationProblem`] shape (JSON-pointer path, friendly
//! message, optional source line/column).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use indexmap::IndexMap;
use jsonschema::{Draft, PatternOptions, Validator, error::ValidationErrorKind};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{ValidationProblem, ValidationProblemKind, errors::SchemaError, format};

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
#[derive(Clone)]
pub struct ValidatorCache {
    inner: Arc<Mutex<CacheInner>>,
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
        }
    }

    /// Returns a compiled validator for the given JSON Schema, building it on
    /// first use and reusing the cached one thereafter.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError::BuildValidator`] when `jsonschema` rejects
    /// the schema (bad draft features, malformed keywords, etc.).
    pub fn validator_for(&self, schema: &Value) -> Result<Arc<Validator>, SchemaError> {
        let key = canonical_hash(schema);
        // Fast path: hit.
        if let Some(hit) = self.lookup(&key) {
            return Ok(hit);
        }
        // Miss: build outside the lock to keep contention low.
        let validator = Arc::new(build_validator(schema)?);
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
pub(super) fn build_validator(schema: &Value) -> Result<Validator, SchemaError> {
    let opts = jsonschema::options()
        .with_draft(Draft::Draft202012)
        .with_pattern_options(PatternOptions::regex());
    let opts = format::register_darkmatter_formats(opts)
        .should_validate_formats(true)
        .with_keyword(
            format::DARKMATTER_MATCH_KEYWORD,
            format::match_keyword_factory,
        )
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
    validator
        .iter_errors(instance)
        .map(|err| build_problem(&err, positions, None))
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
    if arm_validators.is_empty() {
        return Vec::new();
    }
    let mut per_arm: Vec<Vec<ValidationProblem>> = Vec::with_capacity(arm_validators.len());
    for (idx, validator) in arm_validators.iter().enumerate() {
        let problems: Vec<ValidationProblem> = validator
            .iter_errors(instance)
            .map(|err| build_problem(&err, positions, Some(idx)))
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

fn build_problem(
    err: &jsonschema::ValidationError<'_>,
    positions: &PositionMap,
    arm_index: Option<usize>,
) -> ValidationProblem {
    let path = err.instance_path().as_str().to_string();
    let key = identify_key(&path, err.kind());
    let property = match err.kind() {
        ValidationErrorKind::Required { property } => property.as_str().map(str::to_string),
        _ => None,
    };
    let kind = classify_kind(err.kind());
    let (line, column) = key
        .as_deref()
        .and_then(|k| positions.get(k).copied())
        .map(|(l, c)| (Some(l), Some(c)))
        .unwrap_or((None, None));
    let message = darkmatter_file_format_message(err).unwrap_or_else(|| err.to_string());
    ValidationProblem {
        path,
        message,
        kind,
        property,
        line,
        column,
        arm_index,
    }
}

/// Returns a substituted message for `format: darkmatter-file` failures so
/// users see the targeted [`format::FileReferenceFailure`] diagnostic instead
/// of the generic `"<value>" is not a "darkmatter-file"` text produced by
/// `jsonschema`.
///
/// Re-runs [`format::resolve_file_reference`] against the failing instance
/// string. The format validator already ran during `jsonschema` validation
/// (returning `false` on failure), so any resolution outcome here reproduces
/// the exact path that produced the `false`. Returns `None` for non-string
/// instances, non-darkmatter-file formats, or any other error kind so the
/// caller falls back to `err.to_string()`.
fn darkmatter_file_format_message(err: &jsonschema::ValidationError<'_>) -> Option<String> {
    let ValidationErrorKind::Format { format } = err.kind() else {
        return None;
    };
    if format != format::DARKMATTER_FILE_FORMAT {
        return None;
    }
    let instance = err.instance();
    let Value::String(value) = instance.as_ref() else {
        return None;
    };
    match format::resolve_file_reference(value) {
        Ok(_) => None,
        Err(failure) => Some(failure.to_string()),
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

/// SHA-256 of the canonicalised JSON Schema bytes used as the cache key.
fn canonical_hash(schema: &Value) -> [u8; 32] {
    // `serde_json::to_vec` is stable per the active feature set; this is
    // sufficient for cache identity (false misses are tolerable, false hits
    // are not — which `to_vec` guarantees because identical Values
    // serialise to identical bytes).
    let bytes = serde_json::to_vec(schema).expect("schema serialises to JSON");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
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
        let v1 = cache.validator_for(&schema).unwrap();
        let v2 = cache.validator_for(&schema).unwrap();
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
            cache.validator_for(&schema).unwrap();
        }
        assert!(cache.len() <= 2);
    }

    #[test]
    fn cache_zero_capacity_is_promoted_to_one() {
        let cache = ValidatorCache::with_capacity(0);
        let schema = trivial_schema();
        cache.validator_for(&schema).unwrap();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_eviction_keeps_recent_entry_alive() {
        let cache = ValidatorCache::with_capacity(2);
        let s1 = json!({"type":"object","properties":{"a":{"type":"string"}}});
        let s2 = json!({"type":"object","properties":{"b":{"type":"string"}}});
        let s3 = json!({"type":"object","properties":{"c":{"type":"string"}}});
        let v1 = cache.validator_for(&s1).unwrap();
        cache.validator_for(&s2).unwrap();
        // Touch s1 so it's the more recent of the two existing entries.
        let v1_again = cache.validator_for(&s1).unwrap();
        assert!(Arc::ptr_eq(&v1, &v1_again));
        // Add s3 — s2 should be evicted as least-recently-used.
        cache.validator_for(&s3).unwrap();
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn classify_kind_maps_required_to_missing() {
        let schema = json!({
            "type": "object",
            "properties": { "title": { "type": "string" } },
            "required": ["title"]
        });
        let validator = build_validator(&schema).unwrap();
        let problems = collect_problems(&validator, &json!({}), &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected one Required error: {problems:?}"
        );
        assert_eq!(problems[0].kind, ValidationProblemKind::Missing);
        assert_eq!(problems[0].property.as_deref(), Some("title"));
    }

    #[test]
    fn classify_kind_maps_type_to_type() {
        let schema = json!({
            "type": "object",
            "properties": { "count": { "type": "number" } }
        });
        let validator = build_validator(&schema).unwrap();
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
        let validator = build_validator(&schema).unwrap();
        let problems = collect_problems(&validator, &json!({ "name": "no" }), &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected one MinLength error: {problems:?}"
        );
        assert_eq!(problems[0].kind, ValidationProblemKind::Invalid);
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
        let v = build_validator(&schema).unwrap();
        assert!(v.is_valid(&json!({ "title": "x" })));
        assert!(!v.is_valid(&json!({})));
    }

    #[test]
    fn build_validator_rejects_bad_schema() {
        let schema = json!({ "type": 42 });
        let err = build_validator(&schema).unwrap_err();
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
        let v = build_validator(&schema).unwrap();
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
        let v = build_validator(&schema).unwrap();
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
        let v = build_validator(&schema).unwrap();
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
        let v = build_validator(&schema).unwrap();
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
        let v = build_validator(&schema).unwrap();
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
        let v0 = Arc::new(build_validator(&arm0).unwrap());
        let v1 = Arc::new(build_validator(&arm1).unwrap());
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
        let v0 = Arc::new(build_validator(&arm0).unwrap());
        let v1 = Arc::new(build_validator(&arm1).unwrap());
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
        let v = build_validator(&darkmatter_file_schema()).unwrap();
        let instance = json!({ "doc": "./does-not-exist.md" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected one Format problem: {problems:?}"
        );
        let problem = &problems[0];
        assert_eq!(problem.path, "/doc");
        assert_eq!(problem.kind, ValidationProblemKind::Invalid);
        assert!(
            problem
                .message
                .contains("no existing file matched reference"),
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
        let v = build_validator(&darkmatter_file_schema()).unwrap();
        // An empty string is rejected at parse time.
        let instance = json!({ "doc": "" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected one Format problem: {problems:?}"
        );
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
        let v = build_validator(&darkmatter_file_schema()).unwrap();
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
        let v = build_validator(&schema).unwrap();
        let instance = json!({ "v": "not-an-email" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected one Format problem: {problems:?}"
        );
        let problem = &problems[0];
        assert!(
            problem.message.contains(r#"is not a "email""#),
            "expected unmodified default message, got: {}",
            problem.message,
        );
    }

    #[test]
    fn x_darkmatter_match_without_darkmatter_file_format_fails_to_build() {
        // Schema-build guard: pairing `x-darkmatter-match` with anything
        // other than `format: darkmatter-file` must surface a build error so
        // authors cannot silently produce a validator that runs the glob
        // check without the parse-and-exists format check alongside it.
        let schema = json!({
            "type": "object",
            "properties": {
                "doc": {
                    "type": "string",
                    "x-darkmatter-match": ["*.md"]
                }
            }
        });
        let err = build_validator(&schema).expect_err("expected build failure");
        let SchemaError::BuildValidator { message } = &err else {
            panic!("expected BuildValidator, got {err:?}");
        };
        assert!(
            message.contains("darkmatter-file"),
            "expected error to name the required format, got: {message}",
        );
    }

    #[test]
    fn x_darkmatter_match_with_darkmatter_file_format_builds() {
        // Sanity counterpart to the negative case above — the guard must
        // not reject the legitimate combination emitted by the
        // SimplifiedSchema converter.
        let schema = json!({
            "type": "object",
            "properties": {
                "doc": {
                    "type": "string",
                    "format": "darkmatter-file",
                    "x-darkmatter-match": ["*.md"]
                }
            }
        });
        let v = build_validator(&schema).expect("expected build success");
        // We don't need a real file for this smoke test — we only need the
        // schema to compile and the validator to be a working object.
        let _ = v.is_valid(&json!({}));
    }

    fn darkmatter_file_match_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "doc": {
                    "type": "string",
                    "format": "darkmatter-file",
                    "x-darkmatter-match": ["*.md"]
                }
            }
        })
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn darkmatter_file_match_missing_file_produces_one_file_reference_diagnostic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let v = build_validator(&darkmatter_file_match_schema()).unwrap();
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
            problem
                .message
                .contains("no existing file matched reference"),
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
    fn darkmatter_file_match_existing_mismatch_produces_one_glob_diagnostic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("exists.txt");
        std::fs::write(&path, b"x").unwrap();
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let v = build_validator(&darkmatter_file_match_schema()).unwrap();
        let instance = json!({ "doc": "./exists.txt" });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected exactly one glob diagnostic, got: {problems:?}"
        );
        let problem = &problems[0];
        assert_eq!(problem.path, "/doc");
        assert_eq!(problem.kind, ValidationProblemKind::Invalid);
        assert!(
            problem
                .message
                .contains("does not match the configured file globs"),
            "expected glob mismatch message, got: {}",
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
        let v = build_validator(&schema).unwrap();
        let instance = json!({ "doc": { "cover": "./missing.md" } });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected one Format problem: {problems:?}"
        );
        assert_eq!(problems[0].path, "/doc/cover");
        assert!(
            problems[0]
                .message
                .contains("no existing file matched reference"),
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
        let v = build_validator(&schema).unwrap();
        let instance = json!({ "docs": ["./missing.md"] });
        let problems = collect_problems(&v, &instance, &PositionMap::new());
        assert_eq!(
            problems.len(),
            1,
            "expected one Format problem: {problems:?}"
        );
        assert_eq!(problems[0].path, "/docs/0");
        assert!(
            problems[0]
                .message
                .contains("no existing file matched reference"),
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
        let v0 = Arc::new(build_validator(&arm0).unwrap());
        let v1 = Arc::new(build_validator(&arm1).unwrap());
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
            problem
                .message
                .contains("no existing file matched reference"),
            "expected improved file-reference message in root-union arm, got: {}",
            problem.message,
        );
    }
}
