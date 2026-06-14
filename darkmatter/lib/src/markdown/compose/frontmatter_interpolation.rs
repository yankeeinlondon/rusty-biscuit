//! Frontmatter interpolation engine.
//!
//! Resolves `{{ variable }}` expressions inside frontmatter values using
//! non-templated (seed) frontmatter values, `ctx.*`, and `env.*` as inputs.
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

use super::expression::{EvaluationLookup, Expr, ExpressionFinder, parse};
use super::interpolation::{Evaluator, ScanMode, interpolate_text};
use super::types::{ComposeContext, ComposeWarning};
use crate::markdown::frontmatter::Frontmatter;
use crate::markdown::types::MarkdownError;
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
/// Contains only non-templated (seed) top-level frontmatter values,
/// plus `ctx.*` and `env.*` from the runtime context.
pub(crate) struct FrontmatterSeedState {
    data: HashMap<String, Value>,
    context: ComposeContext,
}

impl FrontmatterSeedState {
    pub(crate) fn new(data: HashMap<String, Value>, context: ComposeContext) -> Self {
        Self { data, context }
    }
}

impl EvaluationLookup for FrontmatterSeedState {
    fn get(&self, path: &str) -> Option<Value> {
        // ctx.* prefix
        if let Some(ctx_key) = path.strip_prefix("ctx.") {
            return self.context.get(ctx_key).cloned();
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
        match self.get(path) {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s,
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(v) => v.to_string(),
        }
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
            let result = interpolate_text(
                s,
                evaluator,
                ScanMode::Plain,
                fail_fast,
                "frontmatter-interpolation",
            )?;
            Ok((
                Value::String(result.output),
                result.replacements,
                result.warnings,
            ))
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

/// Result of frontmatter interpolation.
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
pub(crate) fn interpolate_frontmatter(
    frontmatter: &mut Frontmatter,
    context: &ComposeContext,
    fail_fast: bool,
    defer_shell_pending: bool,
) -> Result<FrontmatterInterpolationReport, MarkdownError> {
    let fm = frontmatter.as_map();

    let shell_pending_keys: HashSet<String> = if defer_shell_pending {
        fm.iter()
            .filter_map(|(k, v)| match v {
                Value::String(s) if s.starts_with("$(") => Some(k.clone()),
                _ => None,
            })
            .collect()
    } else {
        HashSet::new()
    };

    if std::env::var("DM_DEBUG_DEFER").is_ok() {
        eprintln!("DM_DEBUG: defer_shell_pending={defer_shell_pending} shell_pending_keys={shell_pending_keys:?}");
        for (k, v) in fm.iter() {
            if let Value::String(s) = v {
                let refs = extract_frontmatter_key_refs(v);
                eprintln!("DM_DEBUG: key={k:?} value={s:?} refs={refs:?}");
            }
        }
    }

    let mut seed_map: HashMap<String, Value> = HashMap::new();
    let templated_keys: Vec<String> = fm
        .iter()
        .filter(|(_, v)| contains_interpolation(v))
        .map(|(k, _)| k.clone())
        .collect();

    for (key, value) in fm.iter() {
        if !contains_interpolation(value) {
            seed_map.insert(key.clone(), value.clone());
        }
    }

    if templated_keys.is_empty() {
        return Ok(FrontmatterInterpolationReport {
            replacements: 0,
            warnings: vec![],
        });
    }

    let original_values: HashMap<String, Value> = templated_keys
        .iter()
        .filter_map(|k| fm.get(k).cloned().map(|v| (k.clone(), v)))
        .collect();

    let templated_set: HashSet<String> = templated_keys.iter().cloned().collect();
    let mut resolved: HashSet<String> = HashSet::new();
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

            let seed_state = FrontmatterSeedState::new(seed_map.clone(), context.clone());
            let evaluator = Evaluator::new(&seed_state);
            let (new_value, count, mut warnings) = rewrite_value(original, &evaluator, fail_fast)?;

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
    let shell_blocked =
        transitively_shell_blocked_keys(&templated_keys, &original_values, &shell_pending_keys);

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

        let seed_state = FrontmatterSeedState::new(seed_map.clone(), context.clone());
        let evaluator = Evaluator::new(&seed_state);
        let (new_value, count, mut warnings) = rewrite_value(original, &evaluator, fail_fast)?;

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
            let is_blocked = extract_frontmatter_key_refs(original)
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
fn collect_variable_roots(expr: &Expr, refs: &mut Vec<String>) {
    match expr {
        Expr::Variable(path) => {
            if path.starts_with("ctx.") || path.starts_with("env.") {
                return;
            }
            let root = path.split('.').next().unwrap_or(path);
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
    }

    mod seed_state_tests {
        use super::*;
        use crate::markdown::compose::types::ComposeContext;

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
    }

    mod interpolate_frontmatter_tests {
        use super::*;
        use crate::markdown::compose::types::ComposeContext;
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
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
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

        #[test]
        fn no_templated_keys_returns_zero() {
            let mut fm = fm_from_json(json!({
                "title": "Hello",
                "count": 42
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
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
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
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
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
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
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
            assert_eq!(report.replacements, 1);
            assert_eq!(fm.as_map().get("spec"), Some(&json!("/spec.md")));
        }

        #[test]
        fn ctx_lookup() {
            let mut fm = fm_from_json(json!({
                "date": "{{ctx.today}}"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
            assert_eq!(report.replacements, 1);
            assert_eq!(fm.as_map().get("date"), Some(&json!("2024-06-15")));
        }

        #[test]
        fn chained_reference_resolved_incrementally() {
            // spec is templated but resolved before plan, so plan can reference spec
            let mut fm = fm_from_json(json!({
                "base": "/root",
                "spec": "{{base}}/spec.md",
                "plan": "{{spec}}.plan.md"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
            assert_eq!(fm.as_map().get("spec"), Some(&json!("/root/spec.md")));
            assert_eq!(
                fm.as_map().get("plan"),
                Some(&json!("/root/spec.md.plan.md"))
            );
            assert!(report.replacements >= 2);
        }

        #[test]
        fn unknown_function_errors_instead_of_leaking_raw_template() {
            // Regression: an unknown function (`dir`) used to be demoted to a
            // warning in non-fail-fast mode, leaving the raw `{{ … }}` text in
            // `review` to poison the downstream `review_path` file reference.
            let mut fm = fm_from_json(json!({
                "spec": "features/x/spec.md",
                "review": "{{ dir(spec) + '/review.md' }}",
                "review_path": "@area/{{review}}"
            }));
            let result = interpolate_frontmatter(&mut fm, &test_context(), false, false);
            let Err(err) = result else {
                panic!("unknown function must abort interpolation");
            };
            assert!(err.to_string().contains("Unknown function: dir"));
        }

        #[test]
        fn fail_fast_returns_error() {
            let mut fm = fm_from_json(json!({
                "bad": "{{ > invalid }}"
            }));
            let result = interpolate_frontmatter(&mut fm, &test_context(), true, false);
            assert!(result.is_err());
        }

        #[test]
        fn non_fail_fast_records_warnings() {
            let mut fm = fm_from_json(json!({
                "bad": "{{ > invalid }}"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
            assert!(!report.warnings.is_empty());
        }

        #[test]
        fn defer_shell_pending_leaves_dependent_keys_unresolved() {
            let mut fm = fm_from_json(json!({
                "pwd": "$(pwd)",
                "uname": "$(uname)",
                "combined": "cwd is {{pwd}} and os is {{uname}}",
                "plain": "literal"
            }));
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, true).unwrap();
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

            interpolate_frontmatter(&mut fm, &test_context(), false, true).unwrap();
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
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, true).unwrap();
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
        fn defer_shell_pending_resolves_after_shell_values_become_concrete() {
            let mut fm = fm_from_json(json!({
                "pwd": "$(pwd)",
                "combined": "cwd is {{pwd}}"
            }));
            // First pass: defer because pwd is shell-pending.
            interpolate_frontmatter(&mut fm, &test_context(), false, true).unwrap();
            assert_eq!(fm.as_map().get("combined"), Some(&json!("cwd is {{pwd}}")));

            // Simulate frontmatter shell expansion completing.
            fm.as_map_mut()
                .insert("pwd".to_string(), json!("/real/path"));

            // Second pass: pwd is now concrete (no longer starts with `$(`).
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, true).unwrap();
            assert_eq!(
                fm.as_map().get("combined"),
                Some(&json!("cwd is /real/path"))
            );
            assert_eq!(report.replacements, 1);
        }

        #[test]
        fn defer_shell_pending_false_resolves_against_literal_shell_text() {
            let mut fm = fm_from_json(json!({
                "pwd": "$(pwd)",
                "combined": "cwd is {{pwd}}"
            }));
            // With defer disabled, the literal `$(pwd)` flows through.
            let report = interpolate_frontmatter(&mut fm, &test_context(), false, false).unwrap();
            assert_eq!(fm.as_map().get("combined"), Some(&json!("cwd is $(pwd)")));
            assert_eq!(report.replacements, 1);
        }
    }
}
