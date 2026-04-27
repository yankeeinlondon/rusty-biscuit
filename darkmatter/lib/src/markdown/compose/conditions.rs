//! Shared condition evaluation for `when=` expressions.
//!
//! This module provides the condition evaluator used by both page blocks
//! and transclusion `when` clauses. It was promoted from
//! `transclusion/conditions.rs` to avoid cross-module coupling.

use super::EffectiveState;
use super::expression::{EvaluationLookup, evaluate, is_truthy, parse_condition};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::Path;
use tracing::{debug, trace};

/// Errors from condition parsing or evaluation.
#[derive(Debug, thiserror::Error)]
pub enum ConditionError {
    /// Failed to parse a condition expression.
    #[error("Failed to parse condition '{expr}' at line {line}: {message}")]
    Parse {
        expr: String,
        line: usize,
        message: String,
        /// Byte range in `expr` that the parser identified as problematic.
        ///
        /// When the parser reports only a single byte position, darkmatter
        /// records a single-point span at that offset. If no position can be
        /// recovered, the span falls back to the full expression.
        span: Range<usize>,
    },
    /// Failed to evaluate a condition expression.
    #[error("Failed to evaluate condition '{expr}' at line {line}: {message}")]
    Eval {
        expr: String,
        line: usize,
        message: String,
    },
}

impl biscuit_terminal::errors::BlockError for ConditionError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        let operator_hint = "Operators: <cyan>&&  ||  !  ==  !=  >  >=  <</cyan> | Helpers: <cyan>HasKey, Contains, Length, number, round</cyan>";

        match self {
            ConditionError::Parse {
                expr,
                line,
                message,
                span,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ConditionError", "parse failed"))
                .body(format!(
                    "<dim>Expression:</dim>\n  <cyan>{expr}</cyan>\n{}\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}",
                    caret_marker(expr, span.start)
                ))
                .hint(operator_hint),

            ConditionError::Eval { expr, line, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ConditionError", "evaluation failed"))
                .body(format!(
                    "<dim>Expression:</dim> <cyan>{expr}</cyan>\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ))
                .hint("Confirm every variable referenced is present in the effective state."),
        }
    }
}

/// Evaluates a `when` condition expression against an effective state.
///
/// This is the primary entry point for condition evaluation in the compose
/// pipeline. It parses the expression in condition mode (where `||` is logical
/// OR and `&&` is logical AND) and evaluates it against the provided state.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::{conditions::evaluate_condition, EffectiveStateBuilder, ComposeContext};
/// use serde_json::json;
/// use std::collections::HashMap;
/// use std::path::Path;
///
/// let state = EffectiveStateBuilder::new()
///     .with_frontmatter([("draft".to_string(), json!(true))].into())
///     .with_context(ComposeContext::capture_for_dir(Path::new(".")))
///     .build()
///     .unwrap();
///
/// assert!(evaluate_condition("draft", &state, 1).unwrap());
/// assert!(!evaluate_condition("!draft", &state, 1).unwrap());
/// ```
///
/// ## Returns
///
/// - `Ok(true)` when the expression evaluates to a truthy value
/// - `Ok(false)` when the expression evaluates to a falsy value
///
/// ## Errors
///
/// Returns [`ConditionError::Parse`] when the expression cannot be parsed,
/// or [`ConditionError::Eval`] when evaluation fails (e.g. unknown function).
/// Both variants include the source expression and line number.
pub fn evaluate_condition(
    expr: &str,
    state: &EffectiveState,
    line: usize,
) -> Result<bool, ConditionError> {
    trace!(expr = %expr, line, "conditions: evaluating");

    let parsed = parse_condition(expr).map_err(|e| ConditionError::Parse {
        expr: expr.to_string(),
        line,
        message: e.message.clone(),
        span: parse_error_span(expr, e.position),
    })?;

    let value = evaluate(&parsed, state).map_err(|message| ConditionError::Eval {
        expr: expr.to_string(),
        line,
        message,
    })?;

    let result = is_truthy(&value);
    debug!(expr = %expr, result, "conditions: evaluated");

    Ok(result)
}

fn parse_error_span(expr: &str, position: usize) -> Range<usize> {
    if expr.is_empty() {
        return 0..0;
    }

    let start = position.min(expr.len());
    let end = (start.saturating_add(1)).min(expr.len());
    if start == end {
        0..expr.len()
    } else {
        start..end
    }
}

fn caret_marker(input: &str, byte_offset: usize) -> String {
    let clamped = byte_offset.min(input.len());
    let column = input
        .char_indices()
        .take_while(|(idx, _)| *idx < clamped)
        .count();
    format!("  {}^", " ".repeat(column))
}

/// Evaluates a condition expression against plain JSON data without
/// constructing an [`EffectiveState`].
///
/// This is a shortcut for external callers who only need the boolean
/// DSL with no frontmatter pipeline involvement.
///
/// ## Resolution Order
///
/// 1. **Top-level properties** are resolved against the provided `data`.
/// 2. **`env.*` properties** are resolved against the system environment.
/// 3. **`ctx.*` properties** are resolved via lazy runtime context capture
///    based on the referenced context group.
/// 4. **Unprefixed missing keys** fall back to the `ctx.*` namespace
///    (same behavior as [`EffectiveState`]).
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::conditions::evaluate_condition_against;
/// use serde_json::json;
/// use std::path::Path;
///
/// let data = json!({ "draft": true, "audience": "internal" });
/// let result = evaluate_condition_against(
///     "draft && audience == 'internal'",
///     &data,
///     Path::new("."),
/// ).unwrap();
/// assert!(result);
/// ```
///
/// ## Returns
///
/// - `Ok(true)` when the expression evaluates to a truthy value
/// - `Ok(false)` when the expression evaluates to a falsy value
///
/// ## Errors
///
/// Returns [`ConditionError::Parse`] when the expression cannot be parsed,
/// or [`ConditionError::Eval`] when evaluation fails (e.g. unknown function).
/// Both variants use line `1` for the shortcut API.
///
/// ## Notes
///
/// - [`ConditionError`] implements [`biscuit_terminal::errors::BlockError`],
///   so parse and evaluation failures can be rendered as status blocks.
/// - Context capture is lazy: only the context groups actually referenced
///   by the expression are captured, and only when needed.
pub fn evaluate_condition_against(
    expr: &str,
    data: &Value,
    work_dir: &Path,
) -> Result<bool, ConditionError> {
    trace!(expr = %expr, "conditions: evaluating against plain data");

    let parsed = parse_condition(expr).map_err(|e| ConditionError::Parse {
        expr: expr.to_string(),
        line: 1,
        message: e.message.clone(),
        span: parse_error_span(expr, e.position),
    })?;

    let lookup = ShortcutLookup::new(data, work_dir);
    let value = evaluate(&parsed, &lookup).map_err(|message| ConditionError::Eval {
        expr: expr.to_string(),
        line: 1,
        message,
    })?;

    let result = is_truthy(&value);
    debug!(expr = %expr, result, "conditions: evaluated against plain data");

    Ok(result)
}

/// Lookup implementation for the shortcut API that resolves variables
/// against plain JSON data, environment variables, and lazily-captured
/// runtime context.
struct ShortcutLookup<'a> {
    /// Plain data payload for top-level and nested lookups.
    data: &'a Value,
    /// Base directory for runtime context capture.
    work_dir: &'a Path,
    /// Cache of lazily-captured context values.
    ctx_cache: RefCell<HashMap<String, Value>>,
    /// Set of context groups that have already been captured.
    captured_groups: RefCell<HashSet<super::context::capture::ContextGroup>>,
}

impl<'a> ShortcutLookup<'a> {
    fn new(data: &'a Value, work_dir: &'a Path) -> Self {
        Self {
            data,
            work_dir,
            ctx_cache: RefCell::new(HashMap::new()),
            captured_groups: RefCell::new(HashSet::new()),
        }
    }

    /// Looks up a value from the plain data payload using dot notation.
    fn get_from_data(&self, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        let mut current = self.data.get(parts[0])?.clone();

        for part in &parts[1..] {
            current = match current {
                Value::Object(obj) => obj.get(*part)?.clone(),
                _ => return None,
            };
        }

        Some(current)
    }

    /// Captures a single context group and merges its values into the cache.
    fn capture_group(&self, group: super::context::capture::ContextGroup) {
        let mut groups = HashSet::new();
        groups.insert(group);

        let (values, _diagnostics, _timings) =
            super::context::capture::capture_runtime_context_for_groups(self.work_dir, &groups);

        let mut cache = self.ctx_cache.borrow_mut();
        let mut captured = self.captured_groups.borrow_mut();
        for (key, value) in values {
            cache.insert(key, value);
        }
        captured.insert(group);
    }

    #[cfg(test)]
    fn captured_groups(&self) -> Vec<super::context::capture::ContextGroup> {
        self.captured_groups.borrow().iter().cloned().collect()
    }
}

impl EvaluationLookup for ShortcutLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        // Handle ctx.* prefixes with lazy capture
        if let Some(ctx_key) = path.strip_prefix("ctx.") {
            // Check cache first
            if let Some(cached) = self.ctx_cache.borrow().get(ctx_key) {
                return Some(cached.clone());
            }

            // Determine which group this key belongs to
            if let Some(group) = super::context::capture::ContextGroup::for_key(ctx_key) {
                let need_capture = !self.captured_groups.borrow().contains(&group);
                if need_capture {
                    self.capture_group(group);
                }
            }

            return self.ctx_cache.borrow().get(ctx_key).cloned();
        }

        // Handle env.* prefixes
        if let Some(env_key) = path.strip_prefix("env.") {
            return std::env::var(env_key).ok().map(Value::String);
        }

        // Try plain data lookup first
        if let Some(value) = self.get_from_data(path) {
            return Some(value);
        }

        // Fall back to ctx.* (same behavior as EffectiveState)
        self.get(&format!("ctx.{path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::{ComposeContext, EffectiveStateBuilder};
    use serde_json::{Value, json};
    use std::collections::HashMap;

    fn test_state(data: Value) -> EffectiveState {
        let fm: HashMap<String, Value> = match data {
            Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };

        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(ComposeContext::fixed_for_testing())
            .build()
            .unwrap()
    }

    #[test]
    fn evaluates_unary_not() {
        let state = test_state(json!({}));
        assert!(evaluate_condition("!missing", &state, 1).unwrap());
    }

    #[test]
    fn evaluates_has_key() {
        let state = test_state(json!({ "user": {"name": "Alice"} }));
        assert!(evaluate_condition(r#"HasKey(user, "name")"#, &state, 1).unwrap());
    }

    #[test]
    fn evaluates_and_or() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("And(a, b)", &state, 1).unwrap());
        assert!(evaluate_condition("Or(a, b)", &state, 1).unwrap());
    }

    #[test]
    fn numeric_comparison_coerces_non_numeric_to_zero() {
        let state = test_state(json!({ "name": "Alice" }));
        assert!(evaluate_condition("name >= 0", &state, 1).unwrap());
    }

    #[test]
    fn null_equal_null_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("missing_a == missing_b", &state, 1).unwrap());
    }

    #[test]
    fn null_not_equal_null_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("missing_a != missing_b", &state, 1).unwrap());
    }

    #[test]
    fn defined_equal_null_is_false() {
        let state = test_state(json!({ "color": "red" }));
        assert!(!evaluate_condition("color == missing", &state, 1).unwrap());
    }

    #[test]
    fn defined_not_equal_null_is_true() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition("color != missing", &state, 1).unwrap());
    }

    #[test]
    fn equality_with_string_literal() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition(r#"color == "red""#, &state, 1).unwrap());
        assert!(!evaluate_condition(r#"color == "blue""#, &state, 1).unwrap());
    }

    #[test]
    fn equality_with_single_quoted_string() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition("color == 'red'", &state, 1).unwrap());
        assert!(!evaluate_condition("color == 'blue'", &state, 1).unwrap());
    }

    #[test]
    fn env_equality_with_string_literal() {
        let mut ctx = ComposeContext::fixed_for_testing();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(HashMap::new())
            .with_context(ctx)
            .build()
            .unwrap();

        assert!(evaluate_condition("env.AGENT == 'claude'", &state, 1).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1).unwrap());
    }

    #[test]
    fn unset_env_equality_with_string_literal_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("env.AGENT == 'claude'", &state, 1).unwrap());
    }

    #[test]
    fn mutual_exclusion_pattern() {
        let mut ctx = ComposeContext::fixed_for_testing();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());
        let state = EffectiveStateBuilder::new()
            .with_frontmatter(HashMap::new())
            .with_context(ctx)
            .build()
            .unwrap();

        assert!(evaluate_condition("env.AGENT == 'claude'", &state, 1).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1).unwrap());
        assert!(!evaluate_condition("!env.AGENT", &state, 1).unwrap());
    }

    #[test]
    fn mutual_exclusion_pattern_unset() {
        let state = test_state(json!({}));

        assert!(!evaluate_condition("env.AGENT == 'claude'", &state, 1).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1).unwrap());
        assert!(evaluate_condition("!env.AGENT", &state, 1).unwrap());
    }

    // ── Infix `&&` / `||` coverage ────────────────────────────────────────

    #[test]
    fn infix_and_both_true() {
        let state = test_state(json!({ "a": true, "b": true }));
        assert!(evaluate_condition("a && b", &state, 1).unwrap());
    }

    #[test]
    fn infix_and_one_false() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("a && b", &state, 1).unwrap());
    }

    #[test]
    fn infix_or_both_false() {
        let state = test_state(json!({ "a": false, "b": false }));
        assert!(!evaluate_condition("a || b", &state, 1).unwrap());
    }

    #[test]
    fn infix_or_one_true() {
        let state = test_state(json!({ "a": false, "b": true }));
        assert!(evaluate_condition("a || b", &state, 1).unwrap());
    }

    #[test]
    fn infix_and_binds_tighter_than_or() {
        // (false && true) || true => true
        let state = test_state(json!({ "a": false, "b": true, "c": true }));
        assert!(evaluate_condition("a && b || c", &state, 1).unwrap());

        // false || (true && false) => false
        let state = test_state(json!({ "a": false, "b": true, "c": false }));
        assert!(!evaluate_condition("a || b && c", &state, 1).unwrap());
    }

    #[test]
    fn infix_parenthesized_or_then_and() {
        // (a || b) && c
        let state = test_state(json!({ "a": false, "b": true, "c": true }));
        assert!(evaluate_condition("(a || b) && c", &state, 1).unwrap());

        let state = test_state(json!({ "a": false, "b": true, "c": false }));
        assert!(!evaluate_condition("(a || b) && c", &state, 1).unwrap());
    }

    #[test]
    fn infix_or_with_literal() {
        // a || (missing || "default") — Or short-circuits on `a`.
        let state = test_state(json!({ "a": true }));
        assert!(evaluate_condition(r#"a || (missing || "default")"#, &state, 1).unwrap());
    }

    #[test]
    fn legacy_and_or_function_still_works() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("And(a, b)", &state, 1).unwrap());
        assert!(evaluate_condition("Or(a, b)", &state, 1).unwrap());
    }

    #[test]
    fn infix_and_short_circuits_on_false() {
        // UnknownFn would raise `Unknown function` — but short-circuit on
        // leading `false` means the rhs is never evaluated.
        let state = test_state(json!({}));
        assert!(!evaluate_condition("false_flag && UnknownFn(x)", &state, 1).unwrap());
    }

    #[test]
    fn infix_or_short_circuits_on_true() {
        let state = test_state(json!({ "truthy_flag": true }));
        assert!(evaluate_condition("truthy_flag || UnknownFn(x)", &state, 1).unwrap());
    }

    #[test]
    fn function_and_short_circuits_on_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("And(false_flag, UnknownFn(x))", &state, 1).unwrap());
    }

    #[test]
    fn function_or_short_circuits_on_true() {
        let state = test_state(json!({ "truthy_flag": true }));
        assert!(evaluate_condition("Or(truthy_flag, UnknownFn(x))", &state, 1).unwrap());
    }

    #[test]
    fn infix_and_without_short_circuit_propagates_eval_error() {
        // When short-circuit doesn't kick in, unknown functions must surface
        // as an evaluation error.
        let state = test_state(json!({ "truthy_flag": true }));
        let result = evaluate_condition("truthy_flag && UnknownFn(x)", &state, 1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConditionError::Eval { .. }));
    }

    #[test]
    fn infix_with_comparison_operands() {
        let state = test_state(json!({ "count": 5, "name": "alice" }));
        assert!(evaluate_condition(r#"count > 0 && name == "alice""#, &state, 1).unwrap());
        assert!(!evaluate_condition(r#"count > 0 && name == "bob""#, &state, 1).unwrap());
        assert!(evaluate_condition(r#"count > 10 || name == "alice""#, &state, 1).unwrap());
    }

    // ── Shortcut API: evaluate_condition_against ──────────────────────

    #[test]
    fn shortcut_top_level_lookup() {
        let data = json!({ "draft": true, "title": "Hello" });
        assert!(evaluate_condition_against("draft", &data, std::path::Path::new(".")).unwrap());
        assert!(
            evaluate_condition_against("title == 'Hello'", &data, std::path::Path::new("."))
                .unwrap()
        );
    }

    #[test]
    fn shortcut_nested_lookup() {
        let data = json!({ "user": { "name": "Alice", "admin": true } });
        assert!(
            evaluate_condition_against("user.name == 'Alice'", &data, std::path::Path::new("."))
                .unwrap()
        );
        assert!(
            evaluate_condition_against("user.admin", &data, std::path::Path::new(".")).unwrap()
        );
    }

    #[test]
    fn shortcut_missing_values() {
        let data = json!({});
        assert!(!evaluate_condition_against("missing", &data, std::path::Path::new(".")).unwrap());
        assert!(evaluate_condition_against("!missing", &data, std::path::Path::new(".")).unwrap());
    }

    #[test]
    fn shortcut_comparisons_and_helpers() {
        let data = json!({ "count": 5, "items": [1, 2, 3], "user": { "name": "Alice" } });
        assert!(evaluate_condition_against("count > 0", &data, std::path::Path::new(".")).unwrap());
        assert!(
            evaluate_condition_against("count >= 5", &data, std::path::Path::new(".")).unwrap()
        );
        assert!(
            evaluate_condition_against("count < 10", &data, std::path::Path::new(".")).unwrap()
        );
        assert!(
            evaluate_condition_against("length(items) == 3", &data, std::path::Path::new("."))
                .unwrap()
        );
        assert!(
            evaluate_condition_against("HasKey(user, 'name')", &data, std::path::Path::new("."))
                .unwrap()
        );
    }

    #[test]
    fn shortcut_short_circuits() {
        let data = json!({ "false_flag": false, "truthy_flag": true });
        // Infix AND short-circuits on false
        assert!(
            !evaluate_condition_against(
                "false_flag && UnknownFn(x)",
                &data,
                std::path::Path::new(".")
            )
            .unwrap()
        );
        // Infix OR short-circuits on true
        assert!(
            evaluate_condition_against(
                "truthy_flag || UnknownFn(x)",
                &data,
                std::path::Path::new(".")
            )
            .unwrap()
        );
    }

    #[test]
    fn shortcut_and_or_functions() {
        let data = json!({ "a": true, "b": false });
        assert!(
            !evaluate_condition_against("And(a, b)", &data, std::path::Path::new(".")).unwrap()
        );
        assert!(evaluate_condition_against("Or(a, b)", &data, std::path::Path::new(".")).unwrap());
    }

    #[test]
    fn shortcut_ternary() {
        let data = json!({ "enabled": true, "yes": "yes", "empty": "" });
        assert!(
            evaluate_condition_against("enabled ? yes : empty", &data, std::path::Path::new("."))
                .unwrap()
        );
        let data = json!({ "enabled": false, "yes": "yes", "empty": "" });
        assert!(
            !evaluate_condition_against("enabled ? yes : empty", &data, std::path::Path::new("."))
                .unwrap()
        );
    }

    #[test]
    fn shortcut_parse_error_shape() {
        let data = json!({});
        let result = evaluate_condition_against("&& invalid", &data, std::path::Path::new("."));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConditionError::Parse { line: 1, .. }));
    }

    #[test]
    fn shortcut_eval_error_shape() {
        let data = json!({ "truthy_flag": true });
        let result = evaluate_condition_against(
            "truthy_flag && UnknownFn(x)",
            &data,
            std::path::Path::new("."),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConditionError::Eval { line: 1, .. }));
    }

    #[test]
    fn shortcut_env_lookup() {
        // Set a known env var for the test
        unsafe {
            std::env::set_var("DM_TEST_AGENT", "claude");
        }
        let data = json!({});
        assert!(
            evaluate_condition_against(
                "env.DM_TEST_AGENT == 'claude'",
                &data,
                std::path::Path::new(".")
            )
            .unwrap()
        );
        assert!(
            !evaluate_condition_against(
                "env.DM_TEST_AGENT == 'opencode'",
                &data,
                std::path::Path::new(".")
            )
            .unwrap()
        );
        unsafe {
            std::env::remove_var("DM_TEST_AGENT");
        }
    }

    #[test]
    fn shortcut_ctx_datetime_lookup() {
        let data = json!({});
        // ctx.today and ctx.year are cheap (no I/O)
        let result = evaluate_condition_against("ctx.today", &data, std::path::Path::new("."));
        assert!(result.is_ok());
        // The result depends on the actual date, but it should evaluate without error
    }

    #[test]
    fn shortcut_unprefixed_fallback_to_ctx() {
        let data = json!({});
        // When a key is not in data, fall back to ctx.* (same as EffectiveState)
        // We test with a datetime key since it's always available
        let result = evaluate_condition_against("year", &data, std::path::Path::new("."));
        assert!(result.is_ok());
    }

    // ── Lazy Context Resolution ─────────────────────────────────────────

    #[test]
    fn shortcut_and_short_circuits_prevents_ctx_capture() {
        use crate::markdown::compose::expression;

        let data = json!({ "false_flag": false });
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
        let parsed = expression::parse_condition("false_flag && ctx.repo == 'x'").unwrap();
        let _ = expression::evaluate(&parsed, &lookup);
        let captured = lookup.captured_groups();
        assert!(
            captured.is_empty(),
            "Repo context should not be captured when short-circuited: captured {:?}",
            captured
        );
    }

    #[test]
    fn shortcut_or_short_circuits_prevents_ctx_capture() {
        use crate::markdown::compose::expression;

        let data = json!({ "true_flag": true });
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
        let parsed = expression::parse_condition("true_flag || ctx.gpu").unwrap();
        let _ = expression::evaluate(&parsed, &lookup);
        let captured = lookup.captured_groups();
        assert!(
            captured.is_empty(),
            "GPU context should not be captured when short-circuited: captured {:?}",
            captured
        );
    }

    #[test]
    fn shortcut_plain_data_expression_does_not_capture_context() {
        use crate::markdown::compose::expression;

        let data = json!({ "draft": true });
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
        let parsed = expression::parse_condition("draft == true").unwrap();
        let _ = expression::evaluate(&parsed, &lookup);
        let captured = lookup.captured_groups();
        assert!(
            captured.is_empty(),
            "No context should be captured for plain-data expressions: captured {:?}",
            captured
        );
    }

    #[test]
    fn shortcut_unknown_ctx_key_does_not_capture() {
        use crate::markdown::compose::expression;

        let data = json!({});
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
        let parsed = expression::parse_condition("missing_repo_name").unwrap();
        let _ = expression::evaluate(&parsed, &lookup);
        let captured = lookup.captured_groups();
        assert!(
            captured.is_empty(),
            "Unknown context keys should not trigger capture: captured {:?}",
            captured
        );
    }

    #[test]
    fn shortcut_ctx_reference_captures_needed_group() {
        use crate::markdown::compose::context::capture::ContextGroup;
        use crate::markdown::compose::expression;

        let data = json!({});
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
        let parsed = expression::parse_condition("ctx.today").unwrap();
        let _ = expression::evaluate(&parsed, &lookup);
        let captured = lookup.captured_groups();
        assert!(
            captured.contains(&ContextGroup::DateTime),
            "DateTime group should be captured when ctx.today is referenced"
        );
    }

    #[test]
    fn shortcut_same_group_captured_only_once() {
        use crate::markdown::compose::context::capture::ContextGroup;
        use crate::markdown::compose::expression;

        let data = json!({});
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
        let parsed = expression::parse_condition("ctx.today && ctx.year").unwrap();
        let _ = expression::evaluate(&parsed, &lookup);
        let captured = lookup.captured_groups();
        assert_eq!(
            captured.len(),
            1,
            "DateTime should only be captured once, but got: {:?}",
            captured
        );
        assert!(
            captured.contains(&ContextGroup::DateTime),
            "DateTime group should be in captured set"
        );
    }
}
