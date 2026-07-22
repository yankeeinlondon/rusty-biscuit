//! Frontmatter interpolation engine.
//!
//! Resolves `{{ variable }}` expressions inside frontmatter values. Lookups
//! read non-templated (seed) frontmatter values, the reserved `doc` / `doc.*`
//! namespace (the incrementally-resolved frontmatter object), `ctx.*`, and
//! `env.*`. When a [`ResolutionContext`] is attached, the read-side functions
//! (`file_exists`, `frontmatter`, `absolute`, `relative`, …) resolve their path
//! arguments here too — the same capability every other expression surface has.
//!
//! ## Incremental Seed Semantics
//!
//! Top-level frontmatter entries are partitioned into:
//! - **Seed** values: contain no `{{ }}` expressions
//! - **Templated** values: contain at least one `{{ }}` expression
//! - **Shell-pending** values: top-level strings that start with `$(` and
//!   await frontmatter shell expansion
//!
//! Templated keys are resolved in dependency order: a key is only processed
//! once all other templated keys it references have been resolved. After
//! each key is resolved its value is added to the seed map so that
//! subsequent keys can reference it. This allows a frontmatter variable
//! like `area` to be defined as `{{ctx.current_package_area}}` and then
//! referenced by `success.stderr` as `{{area}}`.
//!
//! When `defer_shell_pending` is enabled, templated keys whose references
//! include shell-pending keys are deferred — they remain unresolved so the
//! caller can run frontmatter shell expansion and then call this function a
//! second time to resolve those keys against the shell-expanded values.

use super::context::catalog::CONTEXT_VARIABLE_DESCRIPTORS;
use super::expression::{EvaluationLookup, Expr, ExpressionFinder, ResolutionContext, doc_namespace, parse};
use super::interpolation::{Evaluator, convert_literals, interpolate_value, ScanMode};
use super::{ComposeContext, ComposeWarning};
use crate::markdown::frontmatter::Frontmatter;
use crate::markdown::types::{MarkdownError, SourceRef};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Returns `true` if the JSON value tree contains any `{{ }}` interpolation expressions.
pub(crate) fn contains_interpolation(value: &Value) -> bool {
    match value {
        Value::String(s) => !ExpressionFinder::find_all_plain(s).is_empty(),
        Value::Array(arr) => arr.iter().any(contains_interpolation),
        Value::Object(obj) => obj.values().any(contains_interpolation),
        _ => false,
    }
}

/// Lookup state for frontmatter interpolation.
///
/// Resolves the reserved `doc` namespace (the incrementally-resolved seed map),
/// the non-templated (seed) top-level frontmatter values, and `ctx.*` / `env.*`
/// from the runtime context. An optional [`ResolutionContext`] enables the
/// read-side expression functions (`file_exists`, `frontmatter`, …) during
/// frontmatter interpolation; it is `None` for context-free callers.
pub(crate) struct FrontmatterSeedState {
    data: HashMap<String, Value>,
    context: ComposeContext,
    resolution_context: Option<ResolutionContext>,
    name_coercion_keys: Vec<String>,
}

impl FrontmatterSeedState {
    pub(crate) fn new(data: HashMap<String, Value>, context: ComposeContext) -> Self {
        Self {
            data,
            context,
            resolution_context: None,
            name_coercion_keys: Vec::new(),
        }
    }

    /// Attaches a [`ResolutionContext`] so the read-side expression functions
    /// (`file_exists`, `frontmatter`, `absolute`, `relative`, …) can evaluate
    /// during frontmatter interpolation. Passing `None` leaves the state
    /// context-free (read-side functions remain disabled).
    #[must_use]
    pub(crate) fn with_resolution_context(
        mut self,
        resolution_context: Option<ResolutionContext>,
    ) -> Self {
        self.resolution_context = resolution_context;
        self
    }

    /// Sets the frontmatter keys whose object values render their `name` field
    /// when interpolated in inline string context. Empty by default; only the
    /// inline `get_string` path consults it.
    #[must_use]
    pub(crate) fn with_name_coercion_keys(mut self, keys: Vec<String>) -> Self {
        self.name_coercion_keys = keys;
        self
    }
}

impl EvaluationLookup for FrontmatterSeedState {
    fn get(&self, path: &str) -> Option<Value> {
        // Reserved `doc` namespace, intercepted before key/ctx/env lookup.
        if doc_namespace::is_doc_namespace(path) {
            let root = Value::Object(self.data.clone().into_iter().collect());
            return doc_namespace::resolve_doc_namespace(path, &root);
        }

        // ctx.* prefix
        if let Some(ctx_key) = path.strip_prefix("ctx.") {
            return self.context.get_effective(ctx_key);
        }

        // env.* prefix
        if let Some(env_key) = path.strip_prefix("env.") {
            return self
                .context
                .env()
                .get(env_key)
                .map(|v| Value::String(v.clone()));
        }

        // Dotted nested path in seed data
        if let Some(dot_pos) = path.find('.') {
            let root = &path[..dot_pos];
            let rest = &path[dot_pos + 1..];
            let root_val = self.data.get(root)?;
            return get_nested(root_val, rest);
        }

        // Simple key in seed data
        self.data.get(path).cloned()
    }

    fn get_string(&self, path: &str) -> String {
        let value = self.get(path);
        if let Some(resolved) = &value
            && let Some(name) = super::context::effective_state::coerce_named_object(
                path,
                resolved,
                &self.name_coercion_keys,
            )
        {
            return name;
        }
        match value {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s,
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(v) => v.to_string(),
        }
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.resolution_context.clone()
    }

    /// Mirrors `EffectiveState`'s catalog-aware validation so frontmatter
    /// interpolation does not flag valid `ctx.*` references as unknown. Without
    /// this override the trait default (`false`) marks *every* `ctx.*` reference
    /// in frontmatter as unknown, and the catalog-backed suggester then suggests
    /// the same name back ("unknown context variable 'ctx.area' — did you mean:
    /// area"). The body path already validates against the catalog via
    /// `EffectiveState`, so the bug surfaced only on frontmatter keys.
    fn is_valid_context_variable(&self, name: &str) -> bool {
        if CONTEXT_VARIABLE_DESCRIPTORS.iter().any(|d| d.name == name) {
            return true;
        }
        self.context.get_effective(name).is_some()
    }

    fn context_variable_names(&self) -> &[&'static str] {
        use std::sync::LazyLock;
        static NAMES: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| CONTEXT_VARIABLE_DESCRIPTORS.iter().map(|d| d.name).collect());
        NAMES.as_slice()
    }
}

/// Walks a dotted path through a JSON value.
fn get_nested(value: &Value, path: &str) -> Option<Value> {
    let mut current = value;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

/// Recursively rewrites interpolation expressions in a JSON value tree.
fn rewrite_value<L: EvaluationLookup>(
    value: &Value,
    evaluator: &Evaluator<L>,
    fail_fast: bool,
) -> Result<(Value, usize, Vec<ComposeWarning>), MarkdownError> {
    match value {
        Value::String(s) => {
            // A whole-value `{{ expr }}` is executable state, not text: it is
            // parsed and evaluated directly (preserving its typed result), and
            // a parse/eval failure is fatal regardless of `fail_fast`. Mixed
            // text rewrites leniently as a string. See [`interpolate_value`].
            interpolate_value(s, evaluator, fail_fast, "frontmatter-interpolation")
        }
        Value::Array(arr) => {
            let mut new_arr = Vec::with_capacity(arr.len());
            let mut total_count = 0;
            let mut all_warnings = Vec::new();
            for item in arr {
                let (new_val, count, warnings) = rewrite_value(item, evaluator, fail_fast)?;
                new_arr.push(new_val);
                total_count += count;
                all_warnings.extend(warnings);
            }
            Ok((Value::Array(new_arr), total_count, all_warnings))
        }
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::with_capacity(obj.len());
            let mut total_count = 0;
            let mut all_warnings = Vec::new();
            for (key, val) in obj {
                let (new_val, count, warnings) = rewrite_value(val, evaluator, fail_fast)?;
                new_obj.insert(key.clone(), new_val);
                total_count += count;
                all_warnings.extend(warnings);
            }
            Ok((Value::Object(new_obj), total_count, all_warnings))
        }
        // Number, Bool, Null — pass through
        other => Ok((other.clone(), 0, vec![])),
    }
}

/// Converts `{{{ ... }}}` interpolation literals in every string value of
/// `frontmatter` to `{{ ... }}`.
///
/// This is applied once, after the final interpolation pass, so literals
/// survive both frontmatter passes when shell expansion is enabled. The
/// replacement is not counted as an interpolation replacement.
pub(crate) fn convert_frontmatter_literals(frontmatter: &mut Frontmatter) {
    fn convert_value(value: &mut Value) {
        match value {
            Value::String(s) => {
                *value = Value::String(convert_literals(s, ScanMode::Plain));
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    convert_value(item);
                }
            }
            Value::Object(obj) => {
                for (_, val) in obj.iter_mut() {
                    convert_value(val);
                }
            }
            _ => {}
        }
    }
    for val in frontmatter.as_map_mut().values_mut() {
        convert_value(val);
    }
}

/// Attaches the receiving frontmatter `key` to a whole-value interpolation
/// failure so the rendered error names the offending property as structured
/// scope rather than as a prose prefix. Non-`Interpolation` errors pass through
/// untouched.
fn key_scoped_error(key: &str, err: MarkdownError) -> MarkdownError {
    match err {
        MarkdownError::Interpolation {
            expression,
            source,
            cause,
            ..
        } => MarkdownError::Interpolation {
            key: Some(key.to_string()),
            expression,
            // Record the origin key on the late-binding fallback; the on-disk
            // excerpt (when available) is layered on at the pipeline boundary.
            source: match *source {
                SourceRef::Effective { rendered, .. } => Box::new(SourceRef::Effective {
                    rendered,
                    origin_key: Some(key.to_string()),
                }),
                on_disk => Box::new(on_disk),
            },
            cause,
        },
        other => other,
    }
}

/// Result of frontmatter interpolation.
#[derive(Debug)]
pub(crate) struct FrontmatterInterpolationReport {
    /// Number of expressions successfully replaced.
    pub replacements: usize,
    /// Warnings generated during rewrite.
    pub warnings: Vec<ComposeWarning>,
}

/// Interpolates templated frontmatter values using seed (non-templated) values.
///
/// Classifies top-level frontmatter entries into seed values (no `{{ }}`)
/// and templated values (contain `{{ }}`). Templated keys are resolved in
/// dependency order: a key is only processed once all other templated keys
/// it references have been resolved. After each key is resolved its value
/// is added to the seed map so that later keys can reference it.
///
/// When `defer_shell_pending` is `true`, top-level string values that begin
/// with `$(` are treated as shell-pending. Templated keys that reference any
/// shell-pending key are left unresolved so a caller can run frontmatter
/// shell expansion and invoke this function again to finish the work.
///
/// `resolution_context` enables the read-side expression functions
/// (`file_exists`, `frontmatter`, `absolute`, `relative`, …) during both
/// interpolation passes. Pass `None` for context-free callers (e.g. tests).
///
/// `exclude_keys` names top-level keys deferred from every compose-time
/// resolution pass (DM1). An excluded key is neither classified as seed nor
/// templated — its value survives raw and is invisible to other keys'
/// resolution. A non-excluded templated key that references an excluded key
/// is rejected during dependency analysis (DM1a).
pub(crate) fn interpolate_frontmatter(
    frontmatter: &mut Frontmatter,
    context: &ComposeContext,
    fail_fast: bool,
    defer_shell_pending: bool,
    resolution_context: Option<ResolutionContext>,
    exclude_keys: &HashSet<String>,
    name_coercion_keys: &[String],
) -> Result<FrontmatterInterpolationReport, MarkdownError> {
    interpolate_frontmatter_impl(
        frontmatter,
        context,
        fail_fast,
        defer_shell_pending,
        resolution_context,
        false,
        exclude_keys,
        name_coercion_keys,
    )
}

/// Best-effort variant for condition-blind pre-flight shell-command collection.
///
/// Identical to [`interpolate_frontmatter`] except that a per-key evaluation
/// failure is swallowed for that one key instead of aborting the whole pass.
/// The motivating case: pre-flight collection runs context-free
/// (`resolution_context = None`), so a key invoking a filesystem function such
/// as `frontmatter()` or `file_exists()` (which require a resolution context)
/// errors. Aborting the pass on that error starves the post-main *fallback*
/// pass — the only place a shell-pending `$(...)` key whose template is
/// transitively blocked (e.g. `dir: "$(dirname '{{ spec || design }}')"` where
/// `design` references `dir`) gets resolved. The collector would then approve
/// the command with its raw `{{ … }}` template intact while execution runs the
/// resolved form, producing a spurious "command not pre-approved" failure.
///
/// Tolerating per-key failures keeps the approval set a faithful superset of
/// what execution will run; keys that legitimately cannot evaluate here are
/// irrelevant to shell-command discovery, and the real run surfaces their
/// errors with full diagnostics.
pub(crate) fn interpolate_frontmatter_best_effort(
    frontmatter: &mut Frontmatter,
    context: &ComposeContext,
    fail_fast: bool,
    defer_shell_pending: bool,
    resolution_context: Option<ResolutionContext>,
    exclude_keys: &HashSet<String>,
    name_coercion_keys: &[String],
) -> Result<FrontmatterInterpolationReport, MarkdownError> {
    interpolate_frontmatter_impl(
        frontmatter,
        context,
        fail_fast,
        defer_shell_pending,
        resolution_context,
        true,
        exclude_keys,
        name_coercion_keys,
    )
}

/// Shared implementation behind [`interpolate_frontmatter`] and
/// [`interpolate_frontmatter_best_effort`]. When `best_effort` is `true`, a
/// per-key rewrite error is skipped (the key is left unresolved) rather than
/// propagated, so the remaining keys — including fallback-resolved shell-pending
/// keys — still reach their final shape.
#[allow(clippy::too_many_arguments)]
fn interpolate_frontmatter_impl(
    frontmatter: &mut Frontmatter,
    context: &ComposeContext,
    fail_fast: bool,
    defer_shell_pending: bool,
    resolution_context: Option<ResolutionContext>,
    best_effort: bool,
    exclude_keys: &HashSet<String>,
    name_coercion_keys: &[String],
) -> Result<FrontmatterInterpolationReport, MarkdownError> {
    let fm = frontmatter.as_map();

    let shell_pending_keys: HashSet<String> = if defer_shell_pending {
        fm.iter()
            .filter_map(|(k, v)| {
                if exclude_keys.contains(k) {
                    return None;
                }
                match v {
                    Value::String(s) if s.starts_with("$(") => Some(k.clone()),
                    _ => None,
                }
            })
            .collect()
    } else {
        HashSet::new()
    };

    // Excluded keys are deferred from resolution (DM1): they are neither
    // seed nor templated. Their values survive raw in frontmatter and are
    // invisible to other keys' resolution (a non-excluded key that
    // references one is rejected by the DM1a dependency check below).
    let mut seed_map: HashMap<String, Value> = HashMap::new();
    let templated_keys: Vec<String> = fm
        .iter()
        .filter(|(k, v)| !exclude_keys.contains(k.as_str()) && contains_interpolation(v))
        .map(|(k, _)| k.clone())
        .collect();

    for (key, value) in fm.iter() {
        if exclude_keys.contains(key) {
            continue;
        }
        if !contains_interpolation(value) {
            seed_map.insert(key.clone(), value.clone());
        }
    }

    if templated_keys.is_empty() {
        if shell_pending_keys.is_empty() {
            convert_frontmatter_literals(frontmatter);
        }
        return Ok(FrontmatterInterpolationReport {
            replacements: 0,
            warnings: vec![],
        });
    }

    let original_values: HashMap<String, Value> = templated_keys
        .iter()
        .filter_map(|k| fm.get(k).cloned().map(|v| (k.clone(), v)))
        .collect();

    // DM1a: a compose-time (non-deferred) key must not read a deferred
    // (excluded) key. Such a reference would inject a raw lifecycle subtree
    // into an early-bound value and make the result depend on a binding-time
    // accident. Reject with a clear error naming both keys before resolution.
    if !exclude_keys.is_empty() {
        for (composed_key, original) in &original_values {
            if let Some(deferred_key) =
                collect_deferred_key_references(original, exclude_keys).into_iter().next()
            {
                return Err(MarkdownError::Transform(format!(
                    "frontmatter key '{composed_key}' references deferred key \
                     '{deferred_key}': a compose-time value must not read a \
                     deferred (event-time) key"
                )));
            }
        }
    }

    let templated_set: HashSet<String> = templated_keys.iter().cloned().collect();
    let mut resolved: HashSet<String> = HashSet::new();
    // Best-effort only: keys whose evaluation failed here (e.g. a context-free
    // call to a filesystem function such as `file_exists`). A key that errors
    // contributes no value to `seed_map`, so any sibling that references it must
    // NOT be finalized — resolving it would substitute the errored key as
    // empty/undefined and bake a value execution will never produce into the
    // result (and, for a `$(...)` key, into the pre-flight approval set). Such
    // dependents are propagated into this set and left unresolved, mirroring the
    // shell-pending deferral.
    let mut errored: HashSet<String> = HashSet::new();
    let mut total_replacements = 0;
    let mut all_warnings = Vec::new();

    loop {
        let mut made_progress = false;

        for key in &templated_keys {
            if resolved.contains(key) {
                continue;
            }

            let original = match original_values.get(key) {
                Some(v) => v,
                None => continue,
            };

            let refs = extract_frontmatter_key_refs(original);
            let has_unresolved = refs
                .iter()
                .any(|r| templated_set.contains(r) && !resolved.contains(r));
            let has_shell_pending = refs.iter().any(|r| shell_pending_keys.contains(r));
            if has_unresolved || has_shell_pending {
                continue;
            }

            // A dependency errored in best-effort mode: this key cannot resolve
            // to a faithful value. Mark it errored too (so its own dependents
            // defer transitively) and leave its original text in place. The
            // dependency-order loop guarantees the errored ref is already known
            // by the time this key becomes eligible.
            if best_effort && refs.iter().any(|r| errored.contains(r)) {
                errored.insert(key.clone());
                resolved.insert(key.clone());
                made_progress = true;
                continue;
            }

            let seed_state = FrontmatterSeedState::new(seed_map.clone(), context.clone())
                .with_resolution_context(resolution_context.clone())
                .with_name_coercion_keys(name_coercion_keys.to_vec());
            let evaluator = Evaluator::new(&seed_state);
            let (new_value, count, mut warnings) =
                match rewrite_value(original, &evaluator, fail_fast)
                    .map_err(|e| key_scoped_error(key, e))
                {
                    Ok(triple) => triple,
                    Err(_) if best_effort => {
                        // Record the failure and mark resolved so the fixpoint
                        // loop makes progress and the fallback pass still runs
                        // for the other keys; leave this key's original
                        // (unresolved) value in place.
                        errored.insert(key.clone());
                        resolved.insert(key.clone());
                        made_progress = true;
                        continue;
                    }
                    Err(e) => return Err(e),
                };

            for w in &mut warnings {
                w.message = format!("key '{}': {}", key, w.message);
            }

            frontmatter
                .as_map_mut()
                .insert(key.clone(), new_value.clone());
            seed_map.insert(key.clone(), new_value);
            resolved.insert(key.clone());
            total_replacements += count;
            all_warnings.extend(warnings);
            made_progress = true;
        }

        if !made_progress {
            break;
        }
    }

    // Keys that transitively depend on a shell-pending value must survive the
    // fallback pass untouched so the post-shell second pass can resolve them
    // against expanded values. A direct-ref check is not enough: a key like
    // `review_path: "@{{area}}/{{review}}"` reaches a shell-pending `dir` only
    // through `review`, and finalizing it here would bake in an empty `review`.
    let shell_blocked = transitively_shell_blocked_keys(
        &templated_keys,
        &original_values,
        &shell_pending_keys,
        &seed_map,
    );

    for key in &templated_keys {
        if resolved.contains(key) {
            continue;
        }

        let original = match original_values.get(key) {
            Some(v) => v,
            None => continue,
        };

        if shell_blocked.contains(key) {
            continue;
        }

        // Same errored-dependency guard as the main loop: a key reaching the
        // fallback that still references an errored key must stay raw rather
        // than finalize with the errored key substituted as empty.
        if best_effort && extract_frontmatter_key_refs(original).iter().any(|r| errored.contains(r)) {
            errored.insert(key.clone());
            continue;
        }

        let seed_state = FrontmatterSeedState::new(seed_map.clone(), context.clone())
            .with_resolution_context(resolution_context.clone())
            .with_name_coercion_keys(name_coercion_keys.to_vec());
        let evaluator = Evaluator::new(&seed_state);
        let (new_value, count, mut warnings) = match rewrite_value(original, &evaluator, fail_fast)
            .map_err(|e| key_scoped_error(key, e))
        {
            Ok(triple) => triple,
            // Best-effort: leave this key's original value in place and keep
            // resolving the rest (see [`interpolate_frontmatter_best_effort`]).
            Err(_) if best_effort => continue,
            Err(e) => return Err(e),
        };

        for w in &mut warnings {
            w.message = format!("key '{}': {}", key, w.message);
        }

        frontmatter
            .as_map_mut()
            .insert(key.clone(), new_value.clone());
        seed_map.insert(key.clone(), new_value);
        total_replacements += count;
        all_warnings.extend(warnings);
    }

    if shell_pending_keys.is_empty() {
        convert_frontmatter_literals(frontmatter);
    }

    Ok(FrontmatterInterpolationReport {
        replacements: total_replacements,
        warnings: all_warnings,
    })
}

/// Computes the templated keys that transitively depend on a shell-pending
/// (`$(...)`) value.
///
/// A key is blocked when it references a shell-pending key directly, or
/// references another templated key that is itself blocked; the result is the
/// fixpoint of that relation. Returns an empty set when no keys are
/// shell-pending (e.g. the post-shell second pass), so the fallback resolution
/// pass behaves exactly as before for non-deferred runs.
fn transitively_shell_blocked_keys(
    templated_keys: &[String],
    original_values: &HashMap<String, Value>,
    shell_pending_keys: &HashSet<String>,
    seed_map: &HashMap<String, Value>,
) -> HashSet<String> {
    let mut blocked: HashSet<String> = HashSet::new();
    if shell_pending_keys.is_empty() {
        return blocked;
    }

    loop {
        let mut changed = false;
        for key in templated_keys {
            if blocked.contains(key) {
                continue;
            }
            let Some(original) = original_values.get(key) else {
                continue;
            };
            let is_blocked = extract_frontmatter_key_refs_for_shell_blocking(
                original,
                shell_pending_keys,
                &blocked,
                seed_map,
            )
                .iter()
                .any(|r| shell_pending_keys.contains(r) || blocked.contains(r));
            if is_blocked {
                blocked.insert(key.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    blocked
}

fn extract_frontmatter_key_refs_for_shell_blocking(
    value: &Value,
    shell_pending_keys: &HashSet<String>,
    blocked_keys: &HashSet<String>,
    seed_map: &HashMap<String, Value>,
) -> Vec<String> {
    let mut refs = Vec::new();
    collect_frontmatter_key_refs_for_shell_blocking(
        value,
        &mut refs,
        shell_pending_keys,
        blocked_keys,
        seed_map,
    );
    refs.sort();
    refs.dedup();
    refs
}

fn collect_frontmatter_key_refs_for_shell_blocking(
    value: &Value,
    refs: &mut Vec<String>,
    shell_pending_keys: &HashSet<String>,
    blocked_keys: &HashSet<String>,
    seed_map: &HashMap<String, Value>,
) {
    match value {
        Value::String(s) => {
            for loc in ExpressionFinder::find_all_plain(s) {
                let Ok(expr) = parse(&loc.expression) else {
                    continue;
                };
                collect_variable_roots_for_shell_blocking(
                    &expr,
                    refs,
                    shell_pending_keys,
                    blocked_keys,
                    seed_map,
                );
            }
        }
        Value::Array(arr) => arr.iter().for_each(|v| {
            collect_frontmatter_key_refs_for_shell_blocking(
                v,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            )
        }),
        Value::Object(obj) => obj.values().for_each(|v| {
            collect_frontmatter_key_refs_for_shell_blocking(
                v,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            )
        }),
        _ => {}
    }
}

fn collect_variable_roots_for_shell_blocking(
    expr: &Expr,
    refs: &mut Vec<String>,
    shell_pending_keys: &HashSet<String>,
    blocked_keys: &HashSet<String>,
    seed_map: &HashMap<String, Value>,
) {
    match expr {
        Expr::Fallback { primary, fallback } => match static_truthiness(primary, seed_map) {
            Some(true) => collect_variable_roots(primary, refs),
            Some(false) => collect_variable_roots_for_shell_blocking(
                fallback,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            ),
            None => {
                let mut primary_refs = Vec::new();
                collect_variable_roots(primary, &mut primary_refs);
                let primary_depends_on_shell = primary_refs
                    .iter()
                    .any(|r| shell_pending_keys.contains(r) || blocked_keys.contains(r));
                refs.extend(primary_refs);
                if primary_depends_on_shell {
                    collect_variable_roots_for_shell_blocking(
                        fallback,
                        refs,
                        shell_pending_keys,
                        blocked_keys,
                        seed_map,
                    );
                } else {
                    collect_variable_roots(fallback, refs);
                }
            }
        },
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_variable_roots_for_shell_blocking(
                condition,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            );
            collect_variable_roots_for_shell_blocking(
                then_branch,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            );
            collect_variable_roots_for_shell_blocking(
                else_branch,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            );
        }
        Expr::UnaryNot(inner)
        | Expr::UnaryMinus(inner)
        | Expr::Paren(inner)
        | Expr::MemberAccess { base: inner, .. } => collect_variable_roots_for_shell_blocking(
            inner,
            refs,
            shell_pending_keys,
            blocked_keys,
            seed_map,
        ),
        Expr::Comparison { left, right, .. } | Expr::Binary { left, right, .. } => {
            collect_variable_roots_for_shell_blocking(
                left,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            );
            collect_variable_roots_for_shell_blocking(
                right,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            );
        }
        Expr::Index { base, index } => {
            collect_variable_roots_for_shell_blocking(
                base,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            );
            collect_variable_roots_for_shell_blocking(
                index,
                refs,
                shell_pending_keys,
                blocked_keys,
                seed_map,
            );
        }
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                collect_variable_roots_for_shell_blocking(
                    arg,
                    refs,
                    shell_pending_keys,
                    blocked_keys,
                    seed_map,
                );
            }
        }
        _ => collect_variable_roots(expr, refs),
    }
}

fn static_truthiness(expr: &Expr, seed_map: &HashMap<String, Value>) -> Option<bool> {
    match expr {
        Expr::StringLiteral(value) => Some(!value.is_empty()),
        Expr::NumberLiteral(value) => Some(*value != 0.0),
        Expr::BoolLiteral(value) => Some(*value),
        Expr::Paren(inner) => static_truthiness(inner, seed_map),
        Expr::Variable(path) => {
            let root = path.strip_prefix("doc.").unwrap_or(path);
            if root == "doc" || root.starts_with("ctx.") || root.starts_with("env.") {
                return None;
            }
            let (root, rest) = root.split_once('.').unwrap_or((root, ""));
            let value = seed_map.get(root)?;
            if rest.is_empty() {
                Some(super::expression::is_truthy(value))
            } else {
                get_nested(value, rest).map(|nested| super::expression::is_truthy(&nested))
            }
        }
        Expr::Fallback { primary, fallback } => match static_truthiness(primary, seed_map) {
            Some(true) => Some(true),
            Some(false) => static_truthiness(fallback, seed_map),
            None => None,
        },
        _ => None,
    }
}

/// Extracts frontmatter-key references from a JSON value tree.
///
/// Parses each interpolation expression as an AST and collects the root names
/// of every `Expr::Variable` that resolves against frontmatter (i.e. not
/// prefixed with `ctx.` or `env.`). This descends through ternary conditions
/// and both branches, fallback expressions, comparisons, parenthesized
/// expressions, unary operators, and function arguments — so that
/// `{{ use ? spec : 'none' }}` is recognised as depending on `use` and `spec`.
///
/// Expressions that fail to parse contribute no dependencies; the rewrite
/// pass surfaces those errors with full diagnostics.
fn extract_frontmatter_key_refs(value: &Value) -> Vec<String> {
    let mut refs = Vec::new();
    collect_frontmatter_key_refs(value, &mut refs);
    refs.sort();
    refs.dedup();
    refs
}

fn collect_frontmatter_key_refs(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            for loc in ExpressionFinder::find_all_plain(s) {
                let Ok(expr) = parse(&loc.expression) else {
                    continue;
                };
                collect_variable_roots(&expr, refs);
            }
        }
        Value::Array(arr) => arr
            .iter()
            .for_each(|v| collect_frontmatter_key_refs(v, refs)),
        Value::Object(obj) => obj
            .values()
            .for_each(|v| collect_frontmatter_key_refs(v, refs)),
        _ => {}
    }
}

/// Walks an `Expr` AST and pushes the root segment of every `Variable` node
/// onto `refs`, skipping `ctx.*` and `env.*` since those resolve from the
/// runtime context, not seed frontmatter.
///
/// The reserved `doc` namespace mirrors bare-name dependencies (Decision C):
/// `doc.<root>` contributes the same dependency root as bare `<root>`, so
/// `{{ doc.a }}` waits for templated key `a` and `doc.doc` waits for a literal
/// key named `doc`. Bare `doc` is a snapshot of the currently-resolved values
/// and contributes no dependency, avoiding all-key dependencies and self-cycles.
fn collect_variable_roots(expr: &Expr, refs: &mut Vec<String>) {
    match expr {
        Expr::Variable(path) => {
            if path.starts_with("ctx.") || path.starts_with("env.") {
                return;
            }
            if path == "doc" {
                return;
            }
            let effective = path.strip_prefix("doc.").unwrap_or(path);
            let root = effective.split('.').next().unwrap_or(effective);
            if !root.is_empty() {
                refs.push(root.to_string());
            }
        }
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {}
        Expr::UnaryNot(inner)
        | Expr::UnaryMinus(inner)
        | Expr::Paren(inner)
        | Expr::MemberAccess { base: inner, .. } => collect_variable_roots(inner, refs),
        Expr::Fallback { primary, fallback } => {
            collect_variable_roots(primary, refs);
            collect_variable_roots(fallback, refs);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_variable_roots(condition, refs);
            collect_variable_roots(then_branch, refs);
            collect_variable_roots(else_branch, refs);
        }
        Expr::Comparison { left, right, .. } | Expr::Binary { left, right, .. } => {
            collect_variable_roots(left, refs);
            collect_variable_roots(right, refs);
        }
        Expr::Index { base, index } => {
            collect_variable_roots(base, refs);
            collect_variable_roots(index, refs);
        }
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                collect_variable_roots(arg, refs);
            }
        }
    }
}

/// Collects the roots of variable references inside a JSON value's
/// interpolation expressions that match an excluded (deferred) key.
///
/// Handles both bare references (`{{ failure }}`, `{{ failure.message }}`) and
/// `doc.<key>` references (`{{ doc.failure.message }}`), mirroring the root
/// extraction in [`collect_variable_roots`]. Returns the matching deferred
/// keys in order of first discovery, deduplicated.
fn collect_deferred_key_references(
    value: &Value,
    exclude_keys: &HashSet<String>,
) -> Vec<String> {
    let mut roots = Vec::new();
    collect_variable_roots_in_value(value, &mut roots);
    let mut seen = HashSet::new();
    let mut matched = Vec::new();
    for root in &roots {
        if exclude_keys.contains(root) && seen.insert(root.clone()) {
            matched.push(root.clone());
        }
    }
    matched
}

fn collect_variable_roots_in_value(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            for loc in ExpressionFinder::find_all_plain(s) {
                let Ok(expr) = parse(&loc.expression) else {
                    continue;
                };
                collect_variable_roots(&expr, refs);
            }
        }
        Value::Array(arr) => arr
            .iter()
            .for_each(|v| collect_variable_roots_in_value(v, refs)),
        Value::Object(obj) => obj
            .values()
            .for_each(|v| collect_variable_roots_in_value(v, refs)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod contains_interpolation_tests {
        use super::*;

        #[test]
        fn plain_string_returns_false() {
            assert!(!contains_interpolation(&json!("hello world")));
        }

        #[test]
        fn string_with_expression_returns_true() {
            assert!(contains_interpolation(&json!("{{ foo }}")));
        }

        #[test]
        fn nested_object_with_expression_returns_true() {
            assert!(contains_interpolation(
                &json!({"outer": {"inner": "{{ bar }}"}})
            ));
        }

        #[test]
        fn array_with_expression_returns_true() {
            assert!(contains_interpolation(&json!(["plain", "{{ x }}"])));
        }

        #[test]
        fn number_returns_false() {
            assert!(!contains_interpolation(&json!(42)));
        }

        #[test]
        fn bool_returns_false() {
            assert!(!contains_interpolation(&json!(true)));
        }

        #[test]
        fn null_returns_false() {
            assert!(!contains_interpolation(&json!(null)));
        }

        #[test]
        fn literal_returns_false() {
            assert!(!contains_interpolation(&json!("{{{ x }}}")));
        }
    }

    mod seed_state_tests {
        use super::*;
        use crate::markdown::compose::ComposeContext;

        fn test_context() -> ComposeContext {
            ComposeContext::fixed_for_testing()
        }

        #[test]
        fn simple_key_resolves() {
            let mut data = HashMap::new();
            data.insert("base".to_string(), json!("/path"));
            let state = FrontmatterSeedState::new(data, test_context());
            assert_eq!(state.get("base"), Some(json!("/path")));
        }

        #[test]
        fn ctx_today_resolves() {
            let state = FrontmatterSeedState::new(HashMap::new(), test_context());
            assert_eq!(state.get("ctx.today"), Some(json!("2024-06-15")));
        }

        #[test]
        fn env_resolves() {
            // fixed_for_testing has empty env, so use a live context
            let ctx = ComposeContext::capture();
            let state = FrontmatterSeedState::new(HashMap::new(), ctx);
            // env should have HOME on any unix system
            let result = state.get("env.HOME");
            assert!(result.is_some());
        }

        #[test]
        fn env_missing_returns_none() {
            let state = FrontmatterSeedState::new(HashMap::new(), test_context());
            assert_eq!(state.get("env.NONEXISTENT_VAR_12345"), None);
        }

        #[test]
        fn dotted_path_resolves_nested() {
            let mut data = HashMap::new();
            data.insert("meta".to_string(), json!({"author": "Alice"}));
            let state = FrontmatterSeedState::new(data, test_context());
            assert_eq!(state.get("meta.author"), Some(json!("Alice")));
        }

        #[test]
        fn missing_key_returns_none() {
            let state = FrontmatterSeedState::new(HashMap::new(), test_context());
            assert_eq!(state.get("nonexistent"), None);
        }

        #[test]
        fn doc_namespace_resolves_against_seed_only() {
            let mut data = HashMap::new();
            data.insert("build".to_string(), json!("seed-build"));
            data.insert("doc".to_string(), json!({ "child": "literal-doc" }));
            let state = FrontmatterSeedState::new(data, test_context());

            // doc.<path> resolves a seed property.
            assert_eq!(state.get("doc.build"), Some(json!("seed-build")));

            // bare doc returns the whole seed object.
            let obj = state.get("doc").expect("bare doc resolves");
            assert!(obj.is_object());
            assert_eq!(obj.get("build"), Some(&json!("seed-build")));

            // a literal property named `doc` is reached as doc.doc.
            assert_eq!(state.get("doc.doc.child"), Some(json!("literal-doc")));

            // missing doc.* values do not fall back to ctx.*.
            assert_eq!(state.get("ctx.today"), Some(json!("2024-06-15")));
            assert_eq!(state.get("doc.today"), None);
        }

        #[test]
        fn resolution_context_defaults_to_none() {
            let state = FrontmatterSeedState::new(HashMap::new(), test_context());
            assert!(state.resolution_context().is_none());
        }

        #[test]
        fn get_string_returns_empty_for_missing() {
            let state = FrontmatterSeedState::new(HashMap::new(), test_context());
            assert_eq!(state.get_string("nonexistent"), "");
        }

        #[test]
        fn get_string_coerces_number() {
            let mut data = HashMap::new();
            data.insert("count".to_string(), json!(42));
            let state = FrontmatterSeedState::new(data, test_context());
            assert_eq!(state.get_string("count"), "42");
        }

        #[test]
        fn get_string_coerces_named_object_when_key_is_set() {
            let mut data = HashMap::new();
            data.insert("state".to_string(), json!({ "name": "alpha", "index": 1 }));
            let state = FrontmatterSeedState::new(data, test_context())
                .with_name_coercion_keys(vec!["state".to_string()]);
            // Inline string context renders the `name` field.
            assert_eq!(state.get_string("state"), "alpha");
            // Typed lookup keeps the object.
            assert_eq!(
                state.get("state"),
                Some(json!({ "name": "alpha", "index": 1 }))
            );
        }

        #[test]
        fn get_string_named_object_without_keys_renders_json() {
            let value = json!({ "name": "alpha", "index": 1 });
            let mut data = HashMap::new();
            data.insert("state".to_string(), value.clone());
            let state = FrontmatterSeedState::new(data, test_context());
            assert_eq!(state.get_string("state"), value.to_string());
        }
    }

    mod interpolate_frontmatter_tests {
        use super::*;
        use crate::markdown::compose::ComposeContext;
        use crate::markdown::frontmatter::Frontmatter;

        fn test_context() -> ComposeContext {
            ComposeContext::fixed_for_testing()
        }

        fn fm_from_json(data: serde_json::Value) -> Frontmatter {
            let map: crate::markdown::types::FrontmatterMap = match data {
                Value::Object(obj) => obj.into_iter().collect(),
                _ => Default::default(),
            };
            Frontmatter::from_map(map)
        }

        #[test]
        fn spec_example() {
            let mut fm = fm_from_json(json!({
                "base": "/path/to/something",
                "spec": "{{base}}/spec.md",
                "plan": "{{base}}/plan.md"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(report.replacements, 2);
            assert_eq!(
                fm.as_map().get("spec"),
                Some(&json!("/path/to/something/spec.md"))
            );
            assert_eq!(
                fm.as_map().get("plan"),
                Some(&json!("/path/to/something/plan.md"))
            );
            // base is unchanged
            assert_eq!(fm.as_map().get("base"), Some(&json!("/path/to/something")));
        }

        /// Frontmatter mirroring the `review-feature.md` shape that surfaced the
        /// pre-flight "command not pre-approved" bug:
        ///
        /// - `dir` is a shell-pending `$(...)` directive whose template
        ///   (`{{ spec || design }}`) is transitively blocked, so it resolves
        ///   only in the post-main fallback pass;
        /// - `iteration` invokes the `frontmatter()` filesystem function, which
        ///   errors in a context-free pass (`resolution_context = None`).
        fn review_feature_shape() -> Frontmatter {
            fm_from_json(json!({
                "spec": "fixes/x/spec.md",
                "dir": "$(dirname '{{ spec || design }}')",
                "design": "{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}",
                "iteration": "{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') + 1 : 1 }}",
            }))
        }

        #[test]
        fn best_effort_resolves_shell_pending_when_unrelated_key_errors() {
            // Regression: a context-requiring key (`iteration` -> `frontmatter()`)
            // errors in the context-free pre-flight pass, but that must NOT abort
            // before the fallback pass resolves the shell-pending `dir`. The
            // command collector relies on `dir` reaching its final shape so the
            // approval set matches what execution runs.
            let mut fm = review_feature_shape();
            interpolate_frontmatter_best_effort(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[])                .expect("best-effort tolerates the per-key error");

            // `dir`'s `{{ spec || design }}` is fully resolved — no template left.
            assert_eq!(
                fm.as_map().get("dir"),
                Some(&json!("$(dirname 'fixes/x/spec.md')")),
                "shell-pending dir must resolve via the fallback pass"
            );
            // The unrelated context-requiring key is left untouched (its real
            // error is surfaced later by the context-bearing execution pass).
            assert_eq!(
                fm.as_map().get("iteration"),
                Some(&json!(
                    "{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') + 1 : 1 }}"
                )),
                "context-requiring key stays unresolved in best-effort mode"
            );
        }

        #[test]
        fn best_effort_does_not_finalize_command_depending_on_errored_key() {
            // Regression (review-4): a context-requiring key (`exists` ->
            // `file_exists`) errors in the context-free best-effort pass. A
            // sibling shell-pending `$(...)` command interpolates that key
            // (`{{ exists }}`). The best-effort pass must NOT substitute the
            // errored key as empty/undefined — doing so would collect
            // `$(echo '')` into the approval set while execution runs
            // `$(echo 'true')`. The command must keep its raw template so the
            // collector can reject it as a dynamic shape rather than approving a
            // value execution will never produce.
            let mut fm = fm_from_json(json!({
                "exists": "{{ file_exists('existing.md') }}",
                "cmd": "$(echo '{{ exists }}')",
            }));
            interpolate_frontmatter_best_effort(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[])                .expect("best-effort tolerates the per-key error");

            // The errored key is left untouched (its real error surfaces later
            // with a resolution context).
            assert_eq!(
                fm.as_map().get("exists"),
                Some(&json!("{{ file_exists('existing.md') }}")),
                "context-requiring key stays unresolved in best-effort mode"
            );
            // The dependent command is NOT finalized with an empty `exists` —
            // its template survives verbatim rather than collapsing to
            // `$(echo '')`.
            assert_eq!(
                fm.as_map().get("cmd"),
                Some(&json!("$(echo '{{ exists }}')")),
                "command depending on an errored key must not substitute it as empty"
            );
        }

        #[test]
        fn best_effort_defers_transitive_dependent_of_errored_key() {
            // The errored-dependency deferral is transitive: `mid` references the
            // errored `exists`, and `cmd` references `mid`. Neither `mid` nor the
            // `$(...)` command may finalize against the missing value.
            let mut fm = fm_from_json(json!({
                "exists": "{{ file_exists('existing.md') }}",
                "mid": "{{ exists }}-suffix",
                "cmd": "$(echo '{{ mid }}')",
            }));
            interpolate_frontmatter_best_effort(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[])                .expect("best-effort tolerates the per-key error");

            assert_eq!(
                fm.as_map().get("mid"),
                Some(&json!("{{ exists }}-suffix")),
                "transitive dependent of an errored key must stay unresolved"
            );
            assert_eq!(
                fm.as_map().get("cmd"),
                Some(&json!("$(echo '{{ mid }}')")),
                "command transitively depending on an errored key must not finalize"
            );
        }

        #[test]
        fn plain_interpolate_aborts_when_unrelated_key_errors() {
            // The non-resilient variant must still propagate the per-key error
            // (so the execution pipeline surfaces it). This guards against the
            // best-effort behavior accidentally leaking into the default path.
            let mut fm = review_feature_shape();
            let result = interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]);
            let err = match result {
                Ok(_) => panic!("plain variant must abort on the context-requiring key error"),
                Err(e) => e,
            };
            assert!(
                err.to_string().contains("frontmatter"),
                "error should name the failing filesystem function; got: {err}"
            );
            // Because the pass aborted before the fallback, `dir` is left raw —
            // exactly the state that produced the spurious approval mismatch.
            assert_eq!(
                fm.as_map().get("dir"),
                Some(&json!("$(dirname '{{ spec || design }}')")),
                "aborted pass leaves dir's template unresolved"
            );
        }

        #[test]
        fn best_effort_matches_plain_when_no_key_errors() {
            // With no context-requiring key, best-effort and plain must agree:
            // best-effort only changes behavior on a per-key evaluation error.
            let make = || {
                fm_from_json(json!({
                    "spec": "fixes/x/spec.md",
                    "dir": "$(dirname '{{ spec || design }}')",
                    "design": "{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}",
                }))
            };
            let mut a = make();
            let mut b = make();
            interpolate_frontmatter(&mut a, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();
            interpolate_frontmatter_best_effort(&mut b, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(a.as_map().get("dir"), b.as_map().get("dir"));
            assert_eq!(
                a.as_map().get("dir"),
                Some(&json!("$(dirname 'fixes/x/spec.md')"))
            );
        }

        #[test]
        fn no_templated_keys_returns_zero() {
            let mut fm = fm_from_json(json!({
                "title": "Hello",
                "count": 42
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(report.replacements, 0);
        }

        #[test]
        fn nested_object_rewrite() {
            let mut fm = fm_from_json(json!({
                "base": "/docs",
                "metadata": {
                    "home": "{{base}}/home",
                    "owner": "Alice"
                }
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(report.replacements, 1);
            let meta = fm.as_map().get("metadata").unwrap();
            assert_eq!(meta.get("home"), Some(&json!("/docs/home")));
            assert_eq!(meta.get("owner"), Some(&json!("Alice")));
        }

        #[test]
        fn array_rewrite() {
            let mut fm = fm_from_json(json!({
                "base": "/root",
                "paths": ["{{base}}/a", "{{base}}/b"]
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(report.replacements, 2);
            let paths = fm.as_map().get("paths").unwrap().as_array().unwrap();
            assert_eq!(paths[0], json!("/root/a"));
            assert_eq!(paths[1], json!("/root/b"));
        }

        #[test]
        fn missing_variable_resolves_to_empty() {
            let mut fm = fm_from_json(json!({
                "spec": "{{missing}}/spec.md"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(report.replacements, 1);
            assert_eq!(fm.as_map().get("spec"), Some(&json!("/spec.md")));
        }

        #[test]
        fn ctx_lookup() {
            let mut fm = fm_from_json(json!({
                "date": "{{ctx.today}}"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(report.replacements, 1);
            assert_eq!(fm.as_map().get("date"), Some(&json!("2024-06-15")));
        }

        #[test]
        fn valid_ctx_vars_in_frontmatter_do_not_warn() {
            // Regression: frontmatter interpolation ran through
            // `FrontmatterSeedState`, which did not override
            // `is_valid_context_variable`, so the trait default (`false`) flagged
            // every `ctx.*` reference in frontmatter as unknown — even catalog
            // variables like `ctx.area`. The catalog-backed suggester then
            // suggested the same name back ("unknown context variable 'ctx.area'
            // — did you mean: area"). The body path was unaffected because it
            // validates against the catalog via `EffectiveState`.
            let mut fm = fm_from_json(json!({
                "review_file": "{{ctx.area}}/review.md",
                "summary": "{{ctx.current_package_area}}"
            }));
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert!(
                !report
                    .warnings
                    .iter()
                    .any(|w| w.message.contains("unknown context variable")),
                "valid ctx.* vars must not warn, got: {:?}",
                report.warnings
            );
        }

        #[test]
        fn unknown_ctx_var_in_frontmatter_still_warns() {
            // The override must not blanket-approve every `ctx.*`: a genuine typo
            // still warns and suggests the nearest catalog variable.
            let mut fm = fm_from_json(json!({
                "review_file": "{{ctx.aera}}/review.md"
            }));
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            let warning = report
                .warnings
                .iter()
                .find(|w| w.message.contains("unknown context variable"))
                .expect("typo'd ctx.* must warn");
            assert!(
                warning.message.contains("ctx.aera"),
                "warning should name the offending reference, got: {}",
                warning.message
            );
            assert!(
                warning.message.contains("did you mean: area"),
                "warning should suggest the nearest catalog variable, got: {}",
                warning.message
            );
        }

        #[test]
        fn chained_reference_resolved_incrementally() {
            // spec is templated but resolved before plan, so plan can reference spec
            let mut fm = fm_from_json(json!({
                "base": "/root",
                "spec": "{{base}}/spec.md",
                "plan": "{{spec}}.plan.md"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(fm.as_map().get("spec"), Some(&json!("/root/spec.md")));
            assert_eq!(
                fm.as_map().get("plan"),
                Some(&json!("/root/spec.md.plan.md"))
            );
            assert!(report.replacements >= 2);
        }

        #[test]
        fn unknown_function_errors_instead_of_leaking_raw_template() {
            // Regression: an unknown function used to be demoted to a warning
            // in non-fail-fast mode, leaving the raw `{{ … }}` text in
            // `review` to poison the downstream `review_path` file reference.
            let mut fm = fm_from_json(json!({
                "spec": "features/x/spec.md",
                "review": "{{ unknown_fn(spec) + '/review.md' }}",
                "review_path": "@area/{{review}}"
            }));
            let result = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]);
            let Err(err) = result else {
                panic!("unknown function must abort interpolation");
            };
            assert!(err.to_string().contains("Unknown function: unknown_fn"));
        }

        #[test]
        fn fail_fast_returns_error() {
            let mut fm = fm_from_json(json!({
                "bad": "{{ > invalid }}"
            }));
            let result = interpolate_frontmatter(&mut fm, &test_context(), true, false, None, &HashSet::new(), &[]);
            assert!(result.is_err());
        }

        #[test]
        fn whole_value_parse_failure_is_fatal_without_fail_fast() {
            // A frontmatter value that is exactly one malformed `{{ … }}` is
            // executable state: it must abort composition on a parse failure
            // even when fail_fast is off, instead of leaking the raw template
            // downstream. The receiving key is captured as structured scope (no
            // prose prefix), and the typed cause is a parse error.
            use crate::markdown::compose::expression::ExpressionError;

            let mut fm = fm_from_json(json!({
                "bad": "{{ > invalid }}"
            }));
            let result = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]);
            let Err(err) = result else {
                panic!("malformed whole-value interpolation must abort");
            };
            let MarkdownError::Interpolation { key, cause, .. } = &err else {
                panic!("expected Interpolation error, got: {err:?}");
            };
            assert_eq!(key.as_deref(), Some("bad"), "error must capture the key");
            assert!(
                matches!(cause.as_ref(), ExpressionError::Parse(_)),
                "cause must be a parse error, got: {cause:?}"
            );
        }

        #[test]
        fn mixed_malformed_interpolation_stays_warning_without_fail_fast() {
            // A malformed `{{ … }}` embedded in surrounding text is NOT
            // whole-value executable state, so it stays lenient when fail_fast
            // is off: warn and leave the raw span in place.
            let mut fm = fm_from_json(json!({
                "note": "prefix {{ > invalid }}"
            }));
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert!(!report.warnings.is_empty());
            assert_eq!(
                fm.as_map().get("note"),
                Some(&json!("prefix {{ > invalid }}"))
            );
        }

        #[test]
        fn whole_value_array_reference_preserves_typed_value() {
            // Decision: a whole-value `{{ expr }}` keeps its evaluated JSON type
            // for arrays and objects too (not just scalars). A reference to a
            // seed array resolves to that typed array rather than a stringified
            // form.
            let mut fm = fm_from_json(json!({
                "tags": ["a", "b"],
                "copy": "{{ tags }}",
            }));
            interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(fm.as_map().get("copy"), Some(&json!(["a", "b"])));
        }

        #[test]
        fn defer_shell_pending_leaves_dependent_keys_unresolved() {
            let mut fm = fm_from_json(json!({
                "pwd": "$(pwd)",
                "uname": "$(uname)",
                "combined": "cwd is {{pwd}} and os is {{uname}}",
                "plain": "literal"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();
            // combined depends on shell-pending keys, so it stays templated.
            assert_eq!(
                fm.as_map().get("combined"),
                Some(&json!("cwd is {{pwd}} and os is {{uname}}"))
            );
            // shell-pending values are untouched.
            assert_eq!(fm.as_map().get("pwd"), Some(&json!("$(pwd)")));
            assert_eq!(fm.as_map().get("uname"), Some(&json!("$(uname)")));
            assert_eq!(report.replacements, 0);
        }

        #[test]
        fn defer_shell_pending_defers_transitive_dependents() {
            // Regression: `review_path` reaches the shell-pending `dir` only
            // through `review`. The fallback pass used to finalize `review_path`
            // against an empty `review`, yielding "@area/" before shell
            // expansion ever ran. Both `review` (direct) and `review_path`
            // (transitive) must stay deferred in the first pass.
            let mut fm = fm_from_json(json!({
                "spec": "features/x/spec.md",
                "iteration": 1,
                "dir": "$(dirname '{{spec}}')",
                "review": "{{ dir + '/review-' + iteration + '.md' }}",
                "review_path": "@area/{{review}}"
            }));

            interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(
                fm.as_map().get("review"),
                Some(&json!("{{ dir + '/review-' + iteration + '.md' }}"))
            );
            assert_eq!(
                fm.as_map().get("review_path"),
                Some(&json!("@area/{{review}}"))
            );

            // Simulate frontmatter shell expansion producing a concrete dir.
            fm.as_map_mut()
                .insert("dir".to_string(), json!("features/x"));

            // Second pass resolves the whole chain against the expanded value.
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(
                fm.as_map().get("review"),
                Some(&json!("features/x/review-1.md"))
            );
            assert_eq!(
                fm.as_map().get("review_path"),
                Some(&json!("@area/features/x/review-1.md"))
            );
            assert!(report.replacements >= 2);
        }

        #[test]
        fn fallback_with_truthy_seed_does_not_shell_block_primary_key() {
            let mut fm = fm_from_json(json!({
                "spec": "features/example/spec.md",
                "dir": "$(dirname '{{spec || design}}')",
                "design": "{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}"
            }));

            interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();

            assert_eq!(
                fm.as_map().get("dir"),
                Some(&json!("$(dirname 'features/example/spec.md')"))
            );
            assert_eq!(
                fm.as_map().get("design"),
                Some(&json!(
                    "{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}"
                ))
            );
        }

        #[test]
        fn defer_shell_pending_resolves_after_shell_values_become_concrete() {
            let mut fm = fm_from_json(json!({
                "pwd": "$(pwd)",
                "combined": "cwd is {{pwd}}"
            }));
            // First pass: defer because pwd is shell-pending.
            interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(fm.as_map().get("combined"), Some(&json!("cwd is {{pwd}}")));

            // Simulate frontmatter shell expansion completing.
            fm.as_map_mut()
                .insert("pwd".to_string(), json!("/real/path"));

            // Second pass: pwd is now concrete (no longer starts with `$(`).
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(
                fm.as_map().get("combined"),
                Some(&json!("cwd is /real/path"))
            );
            assert_eq!(report.replacements, 1);
        }

        #[test]
        fn whole_value_interpolation_preserves_scalar_type() {
            // A frontmatter value that is exactly one `{{ expr }}` keeps its
            // scalar type (bool/number/null) instead of stringifying. Mixed
            // text and string results stay strings.
            let mut fm = fm_from_json(json!({
                "b": "{{ false }}",
                "n": "{{ 1 + 1 }}",
                "nul": "{{ null }}",
                "s": "{{ 'x' }}",
                "mixed": "prefix {{ false }}",
            }));
            interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();

            let map = fm.as_map();
            assert_eq!(map.get("b"), Some(&json!(false)));
            assert_eq!(map.get("n"), Some(&json!(2)));
            assert_eq!(map.get("nul"), Some(&Value::Null));
            assert_eq!(map.get("s"), Some(&json!("x")));
            assert_eq!(map.get("mixed"), Some(&json!("prefix false")));
        }

        #[test]
        fn read_side_functions_resolve_with_context_pre_shell_pass() {
            // The pre-shell pass (`defer_shell_pending = true`) carries the
            // resolution context, so read-side functions resolve in frontmatter.
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(dir.path().join("spec.md"), "# Spec").unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());

            let mut fm = fm_from_json(json!({
                "exists": "{{ file_exists('spec.md') }}",
                "missing": "{{ file_exists('nope.md') }}",
                "abs": "{{ absolute('spec.md') }}",
                "rel": "{{ relative('spec.md') }}",
            }));
            interpolate_frontmatter(&mut fm, &test_context(), false, true, Some(ctx), &HashSet::new(), &[]).unwrap();

            assert_eq!(fm.as_map().get("exists"), Some(&json!(true)));
            assert_eq!(fm.as_map().get("missing"), Some(&json!(false)));
            assert_eq!(
                fm.as_map().get("abs"),
                Some(&json!(
                    dir.path().join("spec.md").to_string_lossy().to_string()
                ))
            );
            assert_eq!(fm.as_map().get("rel"), Some(&json!("spec.md")));
        }

        #[test]
        fn read_side_functions_resolve_with_context_post_shell_pass() {
            // The post-shell pass (`defer_shell_pending = false`) carries the
            // same resolution context.
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(dir.path().join("spec.md"), "# Spec").unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());

            let mut fm = fm_from_json(json!({
                "exists": "{{ file_exists('spec.md') }}",
                "abs": "{{ absolute('spec.md') }}",
                "rel": "{{ relative('spec.md') }}",
            }));
            interpolate_frontmatter(&mut fm, &test_context(), false, false, Some(ctx), &HashSet::new(), &[]).unwrap();

            assert_eq!(fm.as_map().get("exists"), Some(&json!(true)));
            assert_eq!(
                fm.as_map().get("abs"),
                Some(&json!(
                    dir.path().join("spec.md").to_string_lossy().to_string()
                ))
            );
            assert_eq!(fm.as_map().get("rel"), Some(&json!("spec.md")));
        }

        #[test]
        fn read_side_functions_unavailable_without_context() {
            // Without a resolution context, read-side functions fail to evaluate
            // (the recoverable "requires a document resolution context" error),
            // which aborts interpolation rather than silently leaking.
            let mut fm = fm_from_json(json!({
                "exists": "{{ file_exists('spec.md') }}",
            }));
            let result = interpolate_frontmatter(&mut fm, &test_context(), true, false, None, &HashSet::new(), &[]);
            assert!(result.is_err());
        }

        #[test]
        fn doc_root_dependency_orders_like_bare_name() {
            // `{{ doc.a }}` must wait for the templated key `a` exactly as a bare
            // `{{ a }}` reference would (Decision C).
            let mut fm = fm_from_json(json!({
                "base": "/root",
                "a": "{{ base }}/spec.md",
                "b": "{{ doc.a }}",
            }));
            interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(fm.as_map().get("a"), Some(&json!("/root/spec.md")));
            assert_eq!(fm.as_map().get("b"), Some(&json!("/root/spec.md")));
        }

        #[test]
        fn defer_shell_pending_false_resolves_against_literal_shell_text() {
            let mut fm = fm_from_json(json!({
                "pwd": "$(pwd)",
                "combined": "cwd is {{pwd}}"
            }));
            // With defer disabled, the literal `$(pwd)` flows through.
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(fm.as_map().get("combined"), Some(&json!("cwd is $(pwd)")));
            assert_eq!(report.replacements, 1);
        }

        #[test]
        fn pure_literal_is_converted_after_final_pass() {
            let mut fm = fm_from_json(json!({
                "title": "{{{ name }}}"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(fm.as_map().get("title"), Some(&json!("{{ name }}")));
            assert_eq!(report.replacements, 0);
        }

        #[test]
        fn literal_containing_expression_is_not_evaluated() {
            let mut fm = fm_from_json(json!({
                "x": "replaced",
                "title": "{{{ {{ x }} }}}"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(fm.as_map().get("title"), Some(&json!("{{ {{ x }} }}")));
            assert_eq!(report.replacements, 0);
        }

        #[test]
        fn literal_in_templated_value_survives_deferral_and_converts_after_second_pass() {
            // Simulate a document with shell-pending value and a templated key
            // that also contains a literal.
            let mut fm = fm_from_json(json!({
                "y": "$(echo hi)",
                "greeting": "{{{ x }}} {{ y }}"
            }));

            // First pass: shell-pending `y` defers `greeting`; literal untouched.
            interpolate_frontmatter(&mut fm, &test_context(), false, true, None, &HashSet::new(), &[]).unwrap();
            assert_eq!(fm.as_map().get("greeting"), Some(&json!("{{{ x }}} {{ y }}")));

            // Simulate frontmatter shell expansion completing.
            fm.as_map_mut().insert("y".to_string(), json!("hi"));

            // Second pass: resolve expression and convert literal.
            let report = interpolate_frontmatter(
                &mut fm,
                &test_context(),
                false,
                false,
                None,
                &HashSet::new(),
                &[],
            )
            .unwrap();
            assert_eq!(fm.as_map().get("greeting"), Some(&json!("{{ x }} hi")));
            assert_eq!(report.replacements, 1);
        }
    }

    /// DM1 / DM1a tests: excluding top-level keys from frontmatter interpolation.
    mod exclude_keys_tests {
        use super::*;
        use crate::markdown::compose::ComposeContext;
        use crate::markdown::frontmatter::Frontmatter;

        fn test_context() -> ComposeContext {
            ComposeContext::fixed_for_testing()
        }

        fn fm_from_json(data: serde_json::Value) -> Frontmatter {
            let map: crate::markdown::types::FrontmatterMap = match data {
                Value::Object(obj) => obj.into_iter().collect(),
                _ => Default::default(),
            };
            Frontmatter::from_map(map)
        }

        fn empty_exclude() -> HashSet<String> {
            HashSet::new()
        }

        // ── DM1: excluded key is left raw ───────────────────────────

        #[test]
        fn excluded_key_left_raw_for_interpolation() {
            let mut fm = fm_from_json(json!({
                "base": "/root",
                "failure": "{{base}}/failed"
            }));
            let exclude = ["failure"].into_iter().map(String::from).collect::<HashSet<_>>();
            interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[]).unwrap();
            // The excluded key keeps its raw `{{ }}` span.
            assert_eq!(
                fm.as_map().get("failure"),
                Some(&json!("{{base}}/failed")),
                "excluded key must survive raw"
            );
            // The non-excluded seed key is unaffected (it has no interpolation).
            assert_eq!(fm.as_map().get("base"), Some(&json!("/root")));
        }

        #[test]
        fn excluded_key_left_raw_for_whole_value_expansion() {
            let mut fm = fm_from_json(json!({
                "area": "docs",
                "failure": "{{ area }}"
            }));
            let exclude = ["failure"].into_iter().map(String::from).collect::<HashSet<_>>();
            interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[]).unwrap();
            // A whole-value `{{ area }}` would normally resolve to "docs" (typed
            // string), but as an excluded key it must stay raw.
            assert_eq!(
                fm.as_map().get("failure"),
                Some(&json!("{{ area }}")),
                "whole-value expansion must be skipped for excluded keys"
            );
        }

        #[test]
        fn non_excluded_key_still_resolves_normally() {
            let mut fm = fm_from_json(json!({
                "base": "/root",
                "failure": "{{base}}/failed",
                "summary": "{{base}}/summary"
            }));
            let exclude = ["failure"].into_iter().map(String::from).collect::<HashSet<_>>();
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[]).unwrap();
            // Non-excluded templated key still resolves.
            assert_eq!(fm.as_map().get("summary"), Some(&json!("/root/summary")));
            // Excluded key stays raw.
            assert_eq!(fm.as_map().get("failure"), Some(&json!("{{base}}/failed")));
            // Only one replacement (summary), not two.
            assert_eq!(report.replacements, 1);
        }

        #[test]
        fn excluded_value_object_type_preserved() {
            let mut fm = fm_from_json(json!({
                "initialize": {
                    "say": "hello {{base}}",
                    "notify": true
                },
                "base": "world"
            }));
            let exclude = ["initialize"].into_iter().map(String::from).collect::<HashSet<_>>();
            interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[]).unwrap();
            // The excluded key preserves its object shape with raw spans intact.
            let init = fm.as_map().get("initialize").expect("initialize present");
            assert!(init.is_object(), "object type preserved");
            assert_eq!(init.get("say"), Some(&json!("hello {{base}}")));
            assert_eq!(init.get("notify"), Some(&json!(true)));
        }

        #[test]
        fn excluded_value_array_type_preserved() {
            let mut fm = fm_from_json(json!({
                "steps": ["{{base}}", "literal"],
                "base": "root"
            }));
            let exclude = ["steps"].into_iter().map(String::from).collect::<HashSet<_>>();
            interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[]).unwrap();
            let steps = fm.as_map().get("steps").expect("steps present");
            assert!(steps.is_array(), "array type preserved");
            assert_eq!(steps.get(0), Some(&json!("{{base}}")));
            assert_eq!(steps.get(1), Some(&json!("literal")));
        }

        #[test]
        fn empty_exclude_set_is_no_op() {
            let mut fm = fm_from_json(json!({
                "base": "/root",
                "spec": "{{base}}/spec.md"
            }));
            interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &empty_exclude(), &[]).unwrap();
            assert_eq!(fm.as_map().get("spec"), Some(&json!("/root/spec.md")));
        }

        // ── DM1a: composed key referencing a deferred key is rejected ──

        #[test]
        fn composed_key_referencing_deferred_bare_root_rejected() {
            let mut fm = fm_from_json(json!({
                "summary": "{{ failure.message }}",
                "failure": {
                    "message": "{{err.msg}}"
                }
            }));
            let exclude = ["failure"].into_iter().map(String::from).collect::<HashSet<_>>();
            let err = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[])
                .unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("summary"),
                "error names the composed key: {msg}"
            );
            assert!(
                msg.contains("failure"),
                "error names the deferred key: {msg}"
            );
        }

        #[test]
        fn composed_key_referencing_deferred_doc_namespace_rejected() {
            let mut fm = fm_from_json(json!({
                "summary": "{{ doc.failure.message }}",
                "failure": {
                    "message": "{{err.msg}}"
                }
            }));
            let exclude = ["failure"].into_iter().map(String::from).collect::<HashSet<_>>();
            let err = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[])
                .unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("summary"), "error names the composed key: {msg}");
            assert!(msg.contains("failure"), "error names the deferred key: {msg}");
        }

        #[test]
        fn composed_key_referencing_deferred_root_only_rejected() {
            // Bare `{{ failure }}` (root-only reference) should also be rejected.
            let mut fm = fm_from_json(json!({
                "echo": "{{ failure }}",
                "failure": "raw value"
            }));
            let exclude = ["failure"].into_iter().map(String::from).collect::<HashSet<_>>();
            let err = interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[])
                .unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("echo"), "error names the composed key: {msg}");
            assert!(msg.contains("failure"), "error names the deferred key: {msg}");
        }

        #[test]
        fn composed_key_not_referencing_deferred_key_unaffected() {
            let mut fm = fm_from_json(json!({
                "summary": "{{ base }}/summary",
                "base": "/root",
                "failure": "{{err.msg}}"
            }));
            let exclude = ["failure"].into_iter().map(String::from).collect::<HashSet<_>>();
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false, false, None, &exclude, &[]).unwrap();
            // summary resolves normally (it does not reference the deferred key).
            assert_eq!(fm.as_map().get("summary"), Some(&json!("/root/summary")));
            assert_eq!(report.replacements, 1);
            // failure stays raw.
            assert_eq!(fm.as_map().get("failure"), Some(&json!("{{err.msg}}")));
        }
    }
}
