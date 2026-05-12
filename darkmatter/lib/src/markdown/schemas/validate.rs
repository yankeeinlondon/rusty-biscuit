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

use jsonschema::{Draft, Validator};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{ValidationProblem, errors::SchemaError, format};

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
        self.inner.lock().expect("validator cache lock poisoned").entries.len()
    }

    /// Reports whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Builds a `Validator` configured with darkmatter's custom format and
/// keywords.
fn build_validator(schema: &Value) -> Result<Validator, SchemaError> {
    let opts = jsonschema::options().with_draft(Draft::Draft202012);
    let opts = format::register_darkmatter_formats(opts)
        .should_validate_formats(true)
        .with_keyword(format::DARKMATTER_MATCH_KEYWORD, format::match_keyword_factory)
        .with_keyword(
            format::DARKMATTER_URL_SCHEME_KEYWORD,
            format::url_scheme_keyword_factory,
        );
    opts.build(schema).map_err(|err| SchemaError::BuildValidator {
        message: err.to_string(),
    })
}

/// Maps `jsonschema::ValidationError` into the public `ValidationProblem`
/// shape (JSON pointer + plain message). Line/column are unresolved at this
/// layer; the resolver fills them in if a frontmatter source map is
/// available.
pub fn collect_problems(validator: &Validator, instance: &Value) -> Vec<ValidationProblem> {
    validator
        .iter_errors(instance)
        .map(|err| ValidationProblem {
            path: err.instance_path().as_str().to_string(),
            message: err.to_string(),
            line: None,
            column: None,
            arm_index: None,
        })
        .collect()
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
        let problems = collect_problems(&v, &instance);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].path, "/n");
        assert!(!problems[0].message.is_empty());
    }
}
