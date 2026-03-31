//! Frontmatter interpolation engine.
//!
//! Resolves `{{ variable }}` expressions inside frontmatter values using
//! non-templated (seed) frontmatter values, `ctx.*`, and `env.*` as inputs.
//!
//! ## Seed-Only Semantics
//!
//! Top-level frontmatter entries are partitioned into:
//! - **Seed** values: contain no `{{ }}` expressions
//! - **Templated** values: contain at least one `{{ }}` expression
//!
//! The lookup state is built from seed values only. This means chained
//! references between templated values are not supported — if `spec`
//! references `base`, and `plan` references `spec`, then `plan` will
//! see the raw (unrewritten) value of `spec`, not its resolved form.

use super::interpolation::{
    Evaluator, ExpressionFinder, InterpolationLookup, ScanMode, interpolate_text,
};
use super::types::{ComposeContext, ComposeWarning};
use crate::markdown::frontmatter::Frontmatter;
use crate::markdown::types::MarkdownError;
use serde_json::Value;
use std::collections::HashMap;

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

impl InterpolationLookup for FrontmatterSeedState {
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
fn rewrite_value<L: InterpolationLookup>(
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
/// and templated values (contain `{{ }}`). Builds a lookup state from the
/// seed values plus the runtime context, then rewrites the templated values.
pub(crate) fn interpolate_frontmatter(
    frontmatter: &mut Frontmatter,
    context: &ComposeContext,
    fail_fast: bool,
) -> Result<FrontmatterInterpolationReport, MarkdownError> {
    let fm = frontmatter.as_map();

    // Partition: seed keys have no interpolation, templated keys have at least one.
    let mut seed_map: HashMap<String, Value> = HashMap::new();
    let mut templated_keys: Vec<String> = Vec::new();

    for (key, value) in fm.iter() {
        if contains_interpolation(value) {
            templated_keys.push(key.clone());
        } else {
            seed_map.insert(key.clone(), value.clone());
        }
    }

    if templated_keys.is_empty() {
        return Ok(FrontmatterInterpolationReport {
            replacements: 0,
            warnings: vec![],
        });
    }

    // Build seed state
    let seed_state = FrontmatterSeedState::new(seed_map, context.clone());
    let evaluator = Evaluator::new(&seed_state);

    let mut total_replacements = 0;
    let mut all_warnings = Vec::new();

    // Rewrite each templated key's value tree
    let fm_mut = frontmatter.as_map_mut();
    for key in &templated_keys {
        if let Some(value) = fm_mut.get(key).cloned() {
            let (new_value, count, mut warnings) =
                rewrite_value(&value, &evaluator, fail_fast)?;

            // Add key context to warnings
            for w in &mut warnings {
                w.message = format!("key '{}': {}", key, w.message);
            }

            fm_mut.insert(key.clone(), new_value);
            total_replacements += count;
            all_warnings.extend(warnings);
        }
    }

    Ok(FrontmatterInterpolationReport {
        replacements: total_replacements,
        warnings: all_warnings,
    })
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
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false).unwrap();
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
            assert_eq!(
                fm.as_map().get("base"),
                Some(&json!("/path/to/something"))
            );
        }

        #[test]
        fn no_templated_keys_returns_zero() {
            let mut fm = fm_from_json(json!({
                "title": "Hello",
                "count": 42
            }));
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false).unwrap();
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
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false).unwrap();
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
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false).unwrap();
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
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false).unwrap();
            assert_eq!(report.replacements, 1);
            assert_eq!(fm.as_map().get("spec"), Some(&json!("/spec.md")));
        }

        #[test]
        fn ctx_lookup() {
            let mut fm = fm_from_json(json!({
                "date": "{{ctx.today}}"
            }));
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false).unwrap();
            assert_eq!(report.replacements, 1);
            assert_eq!(fm.as_map().get("date"), Some(&json!("2024-06-15")));
        }

        #[test]
        fn chained_reference_not_supported() {
            // spec is templated, so it's excluded from seed state
            // plan references spec, which won't resolve
            let mut fm = fm_from_json(json!({
                "base": "/root",
                "spec": "{{base}}/spec.md",
                "plan": "{{spec}}.plan.md"
            }));
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false).unwrap();
            // base resolved in spec and plan, but spec itself is not available as seed
            assert_eq!(
                fm.as_map().get("spec"),
                Some(&json!("/root/spec.md"))
            );
            // plan resolves {{spec}} to empty string since spec is templated
            assert_eq!(fm.as_map().get("plan"), Some(&json!(".plan.md")));
            assert!(report.replacements >= 2);
        }

        #[test]
        fn fail_fast_returns_error() {
            let mut fm = fm_from_json(json!({
                "bad": "{{ > invalid }}"
            }));
            let result = interpolate_frontmatter(&mut fm, &test_context(), true);
            assert!(result.is_err());
        }

        #[test]
        fn non_fail_fast_records_warnings() {
            let mut fm = fm_from_json(json!({
                "bad": "{{ > invalid }}"
            }));
            let report =
                interpolate_frontmatter(&mut fm, &test_context(), false).unwrap();
            assert!(!report.warnings.is_empty());
        }
    }
}
