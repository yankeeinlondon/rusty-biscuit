//! Schema-driven eager-`file` value rewrite (normalization).
//!
//! Walks a compiled JSON Schema and rewrites every present, non-null value
//! under an eager `format: darkmatter-file` marker to its repo-relative
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
//! through (implicit paths repository-first then source, no launch-area
//! fallback; D2) — so the stored value resolves identically at prepare time and
//! at every later read (Decision #5). The projection itself goes through
//! [`make_portable_relative`] so the stored string uses `/` separators on every
//! OS (Decision #2 / cross-platform stability).
//!
//! See `darkmatter/features/2026-06-27-file-property-rewrite/spec.md` for
//! the full contract (Decisions #1–#9).
//!
//! [`ResolutionContext`]: crate::markdown::compose::expression::ResolutionContext
//! [`make_portable_relative`]: crate::markdown::compose::expression::path_projection::make_portable_relative

use std::collections::HashSet;
use std::path::Path;

use biscuit_file::FileReference;
use jsonschema::Validator;
use serde_json::{Map, Value};

use super::coerce::unwrap_nullable_arm;
use super::format::DARKMATTER_FILE_FORMAT;
use super::validate::{self, build_validator, error_top_level_key};
use crate::markdown::compose::expression::path_projection::make_portable_relative;
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
/// through the identical repository-first-then-source order the read side uses.
/// `fallback` (the launch area) is threaded for structural parity with the
/// validator but is not a resolution input (D2).
///
/// `composition_pending` carries the set of top-level keys still holding a
/// `$(...)` shell expression (compose pre-shell stage). The rewrite skips
/// those keys verbatim so a not-yet-expanded value is never rewritten
/// (Decision #4); their post-shell re-validation will handle them.
///
/// For a root `anyOf` union, the rewrite commits the first arm that
/// validates the instance (the same selection rule
/// [`super::coerce::coerce_root_union`] uses) and rewrites only the
/// committed arm's eager-file properties (Decision #8/#9).
pub fn rewrite_eager_file_values(
    json_schema: &Value,
    instance: &Value,
    base_dir: &Path,
    fallback: Option<&Path>,
    composition_pending: &HashSet<String>,
) -> NormalizationOutcome {
    if let Some(arms) = json_schema.get("anyOf").and_then(Value::as_array) {
        return rewrite_root_union(arms, instance, base_dir, fallback, composition_pending);
    }
    rewrite_object(
        json_schema,
        instance,
        base_dir,
        fallback,
        composition_pending,
    )
}

/// Root-union pass: commit the first arm that validates the (already-coerced)
/// instance — the same selection rule [`super::coerce::coerce_root_union`]
/// uses — then rewrite only the committed arm's eager-file properties.
///
/// Rewriting against a different arm than the one used for type write-back
/// would risk normalizing a value the validator never accepted. Pinning to
/// the committed arm keeps the rewrite structurally consistent with
/// validation (Decision #9).
fn rewrite_root_union(
    arms: &[Value],
    instance: &Value,
    base_dir: &Path,
    fallback: Option<&Path>,
    composition_pending: &HashSet<String>,
) -> NormalizationOutcome {
    for arm in arms {
        let wrapped = validate::wrap_arm_as_root_schema(arm);
        // A schema that fails to build cannot be a valid arm; skip it. The
        // surrounding validator already surfaces build failures elsewhere.
        // Anchors are threaded so the eager-file format validator resolves
        // document-first against `base_dir` — matching the production
        // validator's arm selection, not an ambient-CWD build.
        let Ok(validator) = build_validator(&wrapped, Some(base_dir), fallback) else {
            continue;
        };
        if arm_accepts(&validator, instance, composition_pending) {
            return rewrite_object(arm, instance, base_dir, fallback, composition_pending);
        }
    }
    NormalizationOutcome {
        value: instance.clone(),
        changed: false,
    }
}

/// Whether a root-union arm accepts the (already-coerced) instance.
///
/// Mirrors [`super::coerce::arm_accepts`]: a fully valid candidate is always
/// accepted; otherwise — only when `composition_pending` is non-empty — the
/// arm is still accepted if every residual validation problem points at a
/// pending `$(...)` top-level key. Those keys are re-validated and coerced
/// after shell expansion, so deferring them here is sound; a problem on any
/// non-pending key rejects the arm.
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
    fallback: Option<&Path>,
    composition_pending: &HashSet<String>,
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
        if let Some(rewritten) = rewrite_property(prop_schema, value, base_dir, fallback) {
            out.insert(name.clone(), rewritten);
            changed = true;
        }
    }

    NormalizationOutcome {
        value: Value::Object(out),
        changed,
    }
}

/// Rewrites a single property's value, descending through nullable wrappers,
/// property-level unions, inline objects, and arrays to reach any
/// eager-file markers.
///
/// Returns `Some(new_value)` when the value (or a nested sub-value) was
/// rewritten, `None` to leave it untouched. Absent / null / non-string
/// scalars never match the eager-file marker (Decision #7).
fn rewrite_property(
    prop_schema: &Value,
    value: &Value,
    base_dir: &Path,
    fallback: Option<&Path>,
) -> Option<Value> {
    if value.is_null() {
        return None;
    }

    // Look through the nullable `anyOf` wrapper so a `file(eager) | null`
    // property still triggers on its present, non-null arm — mirrors
    // `coerce::coerce_object`'s unwrap at the same position.
    let effective = unwrap_nullable_arm(prop_schema).unwrap_or(prop_schema);

    // Property-level union (two or more genuine alternative arms). Rewrite
    // only when exactly one validating arm carries the eager-file marker
    // (Decision #9: "no guessing").
    if let Some(arms) = effective.get("anyOf").and_then(Value::as_array) {
        return rewrite_property_union(arms, value, base_dir, fallback);
    }

    // Direct eager-file marker on the effective schema.
    if is_eager_file_fragment(effective) {
        return rewrite_file_value(value, base_dir, fallback);
    }

    // Array of eager-file elements: descend through `items`.
    if effective.get("type").and_then(Value::as_str) == Some("array")
        && let Some(items_schema) = effective.get("items")
    {
        return rewrite_array_items(items_schema, value, base_dir, fallback);
    }

    // Inline object with declared sub-properties: descend into each present
    // sub-property (Decision #8).
    if effective.get("type").and_then(Value::as_str) == Some("object")
        && let Some(sub_props) = effective.get("properties").and_then(Value::as_object)
    {
        return rewrite_inline_object(sub_props, value, base_dir, fallback);
    }

    None
}

/// Property-union pass: rewrite only when exactly one validating arm carries
/// the eager-file marker. Zero or multiple matching arms leave the value
/// untouched (Decision #9: "no guessing").
///
/// An arm "matches" when (a) its compiled schema carries
/// `format: darkmatter-file` and (b) a validator built from the arm accepts
/// the value. Arms without the eager marker are never counted, even if they
/// validate, because they do not identify the value as eager-file-typed.
fn rewrite_property_union(
    arms: &[Value],
    value: &Value,
    base_dir: &Path,
    fallback: Option<&Path>,
) -> Option<Value> {
    let mut eager_matches = 0;
    let mut rewritten: Option<Value> = None;
    for arm in arms {
        if !is_eager_file_fragment(arm) {
            continue;
        }
        // Anchor the validator so the eager-file format validator resolves
        // document-first against `base_dir` — matching the production
        // validator's arm acceptance, not an ambient-CWD build.
        let Ok(validator) = build_validator(arm, Some(base_dir), fallback) else {
            continue;
        };
        if validator.is_valid(value) {
            eager_matches += 1;
            if eager_matches == 1 {
                rewritten = rewrite_file_value(value, base_dir, fallback);
            } else {
                // Ambiguous: more than one validating eager-file arm. Leave
                // the value verbatim (Decision #9).
                return None;
            }
        }
    }
    // Exactly one matching arm: commit its rewrite (if it produced one).
    (eager_matches == 1).then_some(rewritten).flatten()
}

/// Rewrites a single present eager-file value to its repo-relative resolved
/// path. Returns `Some(new_value)` when the value was rewritten, `None` to
/// leave it untouched.
///
/// Leaves the value verbatim when:
/// - the value is not a string (Decision #7),
/// - the value is a remote URL ([`is_remote_url`], Decision #7),
/// - the value still holds a `$(...)` or unresolved `{{` marker — left for
///   the post-shell re-validation (Decision #4),
/// - the resolved path is `None` (a non-local reference validation accepted,
///   Decision #7),
/// - rewriting produces a byte-identical string (idempotence fast path,
///   Decision #6).
fn rewrite_file_value(value: &Value, base_dir: &Path, _fallback: Option<&Path>) -> Option<Value> {
    let Value::String(raw) = value else {
        return None;
    };
    // Remote URLs are left verbatim — there is no local path to project
    // (Decision #7).
    if is_remote_url(raw) {
        return None;
    }
    // Composition-pending values (still holding `$(...)` or unresolved `{{`)
    // are left verbatim — the post-shell re-validation will rewrite them
    // once they expand (Decision #4).
    if raw.contains("$(") || raw.contains("{{") {
        return None;
    }
    // Resolve through the shared document-backed context — the same resolver
    // the eager-file format validator used to accept this value: implicit paths
    // repository-first then source, no launch-area fallback (D2). The rewrite
    // must consume the same resolver so the stored value resolves identically at
    // every later read (Decision #5).
    let normalized = normalize_path_arg(raw);
    let file_ref = match FileReference::new(&normalized) {
        Ok(r) => r,
        // Invalid syntax shouldn't surface here (validation already accepted
        // the value), but degrade gracefully rather than panic.
        Err(_) => return None,
    };
    let resolved = match resolve_document_file_ref(&file_ref, base_dir, None, &[]) {
        Ok(Some(abs)) => abs,
        // `None`: the reference is well-formed but resolves to nothing local
        // (e.g. a remote-resolving value validation accepted). Leave verbatim
        // (Decision #7).
        Ok(None) => return None,
        // Resolution error: the value shouldn't have passed validation, but
        // degrade gracefully.
        Err(_) => return None,
    };
    let portable = make_portable_relative(&resolved, base_dir);
    // Idempotence fast path: if the rewritten value equals the input, no
    // mutation is needed (Decision #6).
    if &portable == raw {
        return None;
    }
    Some(Value::String(portable))
}

/// Recurses into each item of an array value via the `items` fragment. The
/// fragment may itself be an eager-file marker (for `file(eager)[]`), an
/// inline object, or any other recursable shape `rewrite_property` handles.
fn rewrite_array_items(
    items_schema: &Value,
    value: &Value,
    base_dir: &Path,
    fallback: Option<&Path>,
) -> Option<Value> {
    let Value::Array(items) = value else {
        return None;
    };
    let mut out = Vec::with_capacity(items.len());
    let mut changed = false;
    for item in items {
        match rewrite_property(items_schema, item, base_dir, fallback) {
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
    fallback: Option<&Path>,
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
        if let Some(rewritten) = rewrite_property(sub_schema, sub_value, base_dir, fallback) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::functions::relative_fn;
    use crate::markdown::compose::expression::resolve_ctx::ResolutionContext;
    use serde_json::json;
    use tempfile::TempDir;

    /// Creates a temp directory that looks like a git repository root, with
    /// an optional subdirectory the "document" lives in.
    ///
    /// `find_git_root_from` walks up looking for `.git`, so a temp dir with a
    /// `.git` subdirectory is treated as a repo root regardless of where the
    /// host's real repo boundaries sit. This keeps the rewrite tests
    /// independent of the ambient filesystem.
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
    fn eager_file_property_is_rewritten_to_git_root_relative() {
        let repo = repo_fixture();
        std::fs::create_dir_all(repo.path().join("area/sub")).unwrap();
        std::fs::write(repo.path().join("area/sub/spec.md"), "# Spec\n").unwrap();
        // base_dir is the document directory — a subdirectory of the repo.
        let base_dir = repo.path().join("area/sub");

        let schema = eager_file_schema();
        let instance = json!({ "spec": "./spec.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(&schema, &instance, &base_dir, None, &pending);
        assert!(outcome.changed);
        assert_eq!(outcome.value["spec"], json!("area/sub/spec.md"));
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
        let ctx = ResolutionContext::new(base_dir.clone());
        let relative = relative_fn(&[json!("guide.md")], &ctx).expect("relative_fn resolves");
        assert_eq!(relative, json!("docs/guide.md"));

        // Projection via the rewrite pass.
        let schema = eager_file_schema();
        let instance = json!({ "spec": "guide.md" });
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(&schema, &instance, &base_dir, None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let first = rewrite_eager_file_values(&schema, &raw, base_dir, None, &pending);
        assert!(first.changed);
        assert_eq!(first.value["spec"], json!("area/spec.md"));

        // Second rewrite: already-canonical value is a fixpoint.
        let second = rewrite_eager_file_values(&schema, &first.value, base_dir, None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
        assert!(!outcome.changed);
        assert_eq!(outcome.value["spec"], json!("https://example.com/spec.md"));
    }

    #[test]
    fn absent_eager_file_property_is_unchanged() {
        let repo = repo_fixture();
        let schema = optional_eager_file_schema();
        let instance = json!({});
        let pending = HashSet::new();
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
        assert!(!outcome.changed);
        assert_eq!(outcome.value["spec"], json!(42));
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
        let outcome = rewrite_eager_file_values(&schema, &instance, repo.path(), None, &pending);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }
}
