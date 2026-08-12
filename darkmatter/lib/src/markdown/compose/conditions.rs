//! Shared condition evaluation for `when=` expressions.
//!
//! This module provides the condition evaluator used by both page blocks
//! and transclusion `when` clauses. It was promoted from
//! `transclusion/conditions.rs` to avoid cross-module coupling.
//!
//! Conditions parse and evaluate using the shared
//! [`expression`](super::expression) engine; the same operators, helpers,
//! truthiness rules, and access semantics are available here as in
//! `{{ ... }}` interpolation. See the
//! [Darkmatter Expressions](../../../../docs/topics/darkmatter-expressions.md)
//! topic for the full grammar.

use super::expression::{
    CtxLookup, EvaluationLookup, ResolutionContext, doc_namespace, evaluate, is_truthy,
    parse_condition,
};
use super::interpolation::Evaluator;
use crate::markdown::compose::ComposeWarning;
use biscuit_terminal::errors::SourceContext;
use serde_json::Value;
use std::ops::Range;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

/// Errors from condition parsing or evaluation.
#[derive(Debug, thiserror::Error)]
pub enum ConditionError {
    /// Failed to parse a condition expression.
    #[error("Failed to parse condition '{expr}' at line {line}: {message}")]
    Parse {
        ctx: Box<SourceContext>,
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
        ctx: Box<SourceContext>,
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
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        let operator_hint = "Operators: <cyan>&&  ||  !  ==  !=  >  >=  <  <=  +  -  *  /  %  []  .</cyan> | Helpers: <cyan>has_key, contains, length, number, round, min, max, abs, first, last, is_string, is_number, is_array, is_null, is_object, is_empty, starts_with, ends_with, lower, upper, capitalize, kebab_case, snake_case, camel_case, pascal_case, title_case, is_date, is_date_time, is_today, is_yesterday, is_tomorrow, is_this_month, is_this_year</cyan>";

        match self {
            ConditionError::Parse {
                ctx,
                expr,
                line,
                message,
                span,
            } => {
                let body = vec![
                    Prose::new(format!(
                        "Condition parsing failed for <cyan>when=\"{}\"</cyan>:",
                        expr
                    )),
                    ctx.excerpt_prose(*line, 1, "md"),
                    Prose::new(format!(
                        "{} <red><b>^</b></red> (near position {})",
                        " ".repeat(span.start),
                        span.start
                    )),
                    Prose::new(format!("<dim>Message:</dim> {message}")),
                ];

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ConditionError", "parse failed"))
                    .body(body)
                    .hint(operator_hint)
            }

            ConditionError::Eval {
                ctx,
                expr,
                line,
                message,
            } => {
                let body = vec![
                    Prose::new(format!(
                        "Condition evaluation failed for <cyan>when=\"{}\"</cyan>:",
                        expr
                    )),
                    ctx.excerpt_prose(*line, 1, "md"),
                ];

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ConditionError", "evaluation failed"))
                    .body(body)
                    .hint(format!("Error: {message}"))
            }
        }
    }
}

/// Evaluates a `when` condition expression against an effective state.
///
/// This is the primary entry point for condition evaluation in the compose
/// pipeline. It parses the expression in condition mode (where `||` is logical
/// OR and `&&` is logical AND) and evaluates it against the provided state.
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
/// Both variants include the source expression, line number, and source context.
pub fn evaluate_condition<L: EvaluationLookup>(
    expr: &str,
    state: &L,
    line: usize,
    ctx: SourceContext,
) -> Result<bool, ConditionError> {
    trace!(expr = %expr, line, "conditions: evaluating");

    let parsed = parse_condition(expr).map_err(|e| ConditionError::Parse {
        ctx: Box::new(ctx.clone()),
        expr: expr.to_string(),
        line,
        message: e.message.clone(),
        span: parse_error_span(expr, e.position),
    })?;

    let value = evaluate(&parsed, state).map_err(|error| ConditionError::Eval {
        ctx: Box::new(ctx),
        expr: expr.to_string(),
        line,
        message: error.to_string(),
    })?;

    let result = is_truthy(&value);
    debug!(expr = %expr, result, "conditions: evaluated");

    Ok(result)
}

/// Collects context-typo warnings for a `when=` condition expression.
///
/// Parses `expr` in condition mode and walks the resulting AST for `ctx.*`
/// references that do not resolve to a known runtime context variable, reusing
/// the same AST-based check as `{{ ... }}` interpolation. Because the walk is
/// parser-aware, a string literal such as `state == "ctx.toady"` never triggers
/// a warning — only genuine `ctx.*` variable references do.
///
/// Parse failures yield no warnings here: a malformed condition is surfaced as
/// a fatal [`ConditionError::Parse`] by [`evaluate_condition`] at evaluation
/// time, so this additive diagnostic stays silent rather than double-reporting.
///
/// `warning_stage` labels the produced [`ComposeWarning`]s (e.g. `"condition"`).
pub fn collect_condition_context_warnings<L: EvaluationLookup>(
    expr: &str,
    state: &L,
    warning_stage: &'static str,
) -> Vec<ComposeWarning> {
    let Ok(parsed) = parse_condition(expr) else {
        return Vec::new();
    };
    Evaluator::new(state).collect_context_warnings(&parsed, warning_stage)
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

#[allow(dead_code)]
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

    // Shortcut API uses a synthetic source context since it's typically
    // evaluating in-memory or transient expressions.
    let ctx = SourceContext::new(
        work_dir.join("<transient>"),
        PathBuf::from("<transient>"),
        expr.to_string(),
    );

    let parsed = parse_condition(expr).map_err(|e| ConditionError::Parse {
        ctx: Box::new(ctx.clone()),
        expr: expr.to_string(),
        line: 1,
        message: e.message.clone(),
        span: parse_error_span(expr, e.position),
    })?;

    let lookup = ShortcutLookup::new(data, work_dir);
    let value = evaluate(&parsed, &lookup).map_err(|error| ConditionError::Eval {
        ctx: Box::new(ctx),
        expr: expr.to_string(),
        line: 1,
        message: error.to_string(),
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
    /// Lazy-capturing `ctx.*` resolver.
    ctx: CtxLookup<'a>,
    /// Optional document-relative context enabling read-side functions. `None`
    /// leaves the lookup context-free.
    resolution_context: Option<ResolutionContext>,
}

impl<'a> ShortcutLookup<'a> {
    fn new(data: &'a Value, work_dir: &'a Path) -> Self {
        Self {
            data,
            ctx: CtxLookup::new(work_dir),
            // The shortcut API resolves read-side functions against `work_dir`
            // (local-only: `absolute`/`relative`/`file_exists`/… need a base
            // directory but no remote runtime). This is a public-API capability
            // addition for external callers of `evaluate_condition_against`.
            resolution_context: Some(ResolutionContext::new(work_dir.to_path_buf())),
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

    #[cfg(test)]
    fn captured_groups(&self) -> Vec<super::context::capture::ContextGroup> {
        self.ctx.captured_groups()
    }
}

impl EvaluationLookup for ShortcutLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        // Reserved `doc` namespace, intercepted before the bare-name `ctx.*`
        // fallback so a missing `doc.*` never collapses into `ctx.*`.
        if doc_namespace::is_doc_namespace(path) {
            return doc_namespace::resolve_doc_namespace(path, self.data);
        }

        // Handle ctx.* prefixes with lazy capture
        if path == "ctx" || path.starts_with("ctx.") {
            return self.ctx.resolve_ctx(path);
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

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.resolution_context.clone()
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        // Borrowed path (Finding 12) — condition evaluation reuses the context
        // without cloning it per read-side function call.
        self.resolution_context.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::{ComposeContext, EffectiveState, EffectiveStateBuilder};
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

    fn dummy_ctx() -> SourceContext {
        SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), "")
    }

    /// Test shim that supplies a dummy `SourceContext` so call sites stay terse.
    fn eval_cond(expr: &str, state: &EffectiveState, line: usize) -> Result<bool, ConditionError> {
        super::evaluate_condition(expr, state, line, dummy_ctx())
    }

    #[test]
    fn evaluates_unary_not() {
        let state = test_state(json!({}));
        assert!(evaluate_condition("!missing", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn evaluates_has_key() {
        let state = test_state(json!({ "user": {"name": "Alice"} }));
        assert!(evaluate_condition(r#"has_key(user, "name")"#, &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn evaluates_and_or() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("and(a, b)", &state, 1, dummy_ctx()).unwrap());
        assert!(evaluate_condition("or(a, b)", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn numeric_comparison_coerces_non_numeric_to_zero() {
        let state = test_state(json!({ "name": "Alice" }));
        assert!(evaluate_condition("name >= 0", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn null_equal_null_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("missing_a == missing_b", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn null_not_equal_null_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("missing_a != missing_b", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn defined_equal_null_is_false() {
        let state = test_state(json!({ "color": "red" }));
        assert!(!evaluate_condition("color == missing", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn defined_not_equal_null_is_true() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition("color != missing", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn equality_with_string_literal() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition(r#"color == "red""#, &state, 1, dummy_ctx()).unwrap());
        assert!(!evaluate_condition(r#"color == "blue""#, &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn equality_with_single_quoted_string() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition("color == 'red'", &state, 1, dummy_ctx()).unwrap());
        assert!(!evaluate_condition("color == 'blue'", &state, 1, dummy_ctx()).unwrap());
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

        assert!(evaluate_condition("env.AGENT == 'claude'", &state, 1, dummy_ctx()).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn unset_env_equality_with_string_literal_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("env.AGENT == 'claude'", &state, 1, dummy_ctx()).unwrap());
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

        assert!(evaluate_condition("env.AGENT == 'claude'", &state, 1, dummy_ctx()).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1, dummy_ctx()).unwrap());
        assert!(!evaluate_condition("!env.AGENT", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn mutual_exclusion_pattern_unset() {
        let state = test_state(json!({}));

        assert!(!evaluate_condition("env.AGENT == 'claude'", &state, 1, dummy_ctx()).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1, dummy_ctx()).unwrap());
        assert!(evaluate_condition("!env.AGENT", &state, 1, dummy_ctx()).unwrap());
    }

    // ── Infix `&&` / `||` coverage ────────────────────────────────────────

    #[test]
    fn infix_and_both_true() {
        let state = test_state(json!({ "a": true, "b": true }));
        assert!(evaluate_condition("a && b", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn infix_and_one_false() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("a && b", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn infix_or_both_false() {
        let state = test_state(json!({ "a": false, "b": false }));
        assert!(!evaluate_condition("a || b", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn infix_or_one_true() {
        let state = test_state(json!({ "a": false, "b": true }));
        assert!(evaluate_condition("a || b", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn infix_and_binds_tighter_than_or() {
        // (false && true) || true => true
        let state = test_state(json!({ "a": false, "b": true, "c": true }));
        assert!(evaluate_condition("a && b || c", &state, 1, dummy_ctx()).unwrap());

        // false || (true && false) => false
        let state = test_state(json!({ "a": false, "b": true, "c": false }));
        assert!(!evaluate_condition("a || b && c", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn infix_parenthesized_or_then_and() {
        // (a || b) && c
        let state = test_state(json!({ "a": false, "b": true, "c": true }));
        assert!(evaluate_condition("(a || b) && c", &state, 1, dummy_ctx()).unwrap());

        let state = test_state(json!({ "a": false, "b": true, "c": false }));
        assert!(!evaluate_condition("(a || b) && c", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn infix_or_with_literal() {
        // a || (missing || "default") — Or short-circuits on `a`.
        let state = test_state(json!({ "a": true }));
        assert!(
            evaluate_condition(r#"a || (missing || "default")"#, &state, 1, dummy_ctx()).unwrap()
        );
    }

    #[test]
    fn legacy_and_or_function_still_works() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("and(a, b)", &state, 1, dummy_ctx()).unwrap());
        assert!(evaluate_condition("or(a, b)", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn infix_and_short_circuits_on_false() {
        // UnknownFn would raise `Unknown function` — but short-circuit on
        // leading `false` means the rhs is never evaluated.
        let state = test_state(json!({}));
        assert!(!evaluate_condition("false_flag && UnknownFn(x)", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn infix_or_short_circuits_on_true() {
        let state = test_state(json!({ "truthy_flag": true }));
        assert!(evaluate_condition("truthy_flag || UnknownFn(x)", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn function_and_short_circuits_on_false() {
        let state = test_state(json!({}));
        assert!(
            !evaluate_condition("and(false_flag, UnknownFn(x))", &state, 1, dummy_ctx()).unwrap()
        );
    }

    #[test]
    fn function_or_short_circuits_on_true() {
        let state = test_state(json!({ "truthy_flag": true }));
        assert!(
            evaluate_condition("or(truthy_flag, UnknownFn(x))", &state, 1, dummy_ctx()).unwrap()
        );
    }

    #[test]
    fn infix_and_without_short_circuit_propagates_eval_error() {
        // When short-circuit doesn't kick in, unknown functions must surface
        // as an evaluation error.
        let state = test_state(json!({ "truthy_flag": true }));
        let result = evaluate_condition("truthy_flag && UnknownFn(x)", &state, 1, dummy_ctx());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConditionError::Eval { .. }));
    }

    #[test]
    fn infix_with_comparison_operands() {
        let state = test_state(json!({ "count": 5, "name": "alice" }));
        assert!(
            evaluate_condition(r#"count > 0 && name == "alice""#, &state, 1, dummy_ctx()).unwrap()
        );
        assert!(
            !evaluate_condition(r#"count > 0 && name == "bob""#, &state, 1, dummy_ctx()).unwrap()
        );
        assert!(
            evaluate_condition(r#"count > 10 || name == "alice""#, &state, 1, dummy_ctx()).unwrap()
        );
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
            evaluate_condition_against("has_key(user, 'name')", &data, std::path::Path::new("."))
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
            !evaluate_condition_against("and(a, b)", &data, std::path::Path::new(".")).unwrap()
        );
        assert!(evaluate_condition_against("or(a, b)", &data, std::path::Path::new(".")).unwrap());
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

    #[test]
    fn shortcut_doc_namespace_resolves_against_data_only() {
        let data = json!({
            "build": "from-data",
            "doc": { "child": "literal-doc" },
        });
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));

        // doc.<path> resolves a data property.
        assert_eq!(lookup.get("doc.build"), Some(json!("from-data")));

        // bare doc returns the whole data object.
        let obj = lookup.get("doc").expect("bare doc resolves");
        assert!(obj.is_object());
        assert_eq!(obj.get("build"), Some(&json!("from-data")));

        // a literal property named `doc` is reached as doc.doc.
        assert_eq!(lookup.get("doc.doc.child"), Some(json!("literal-doc")));

        // missing doc.* values do not fall back to ctx.*.
        assert_eq!(lookup.get("doc.year"), None);
        // The shortcut lookup now carries a work-dir-rooted resolution context
        // so read-side functions resolve for external callers.
        assert!(lookup.resolution_context().is_some());
    }

    #[test]
    fn shortcut_read_side_functions_resolve_against_work_dir() {
        // `evaluate_condition_against` now supplies a `work_dir`-rooted
        // resolution context, so read-side functions resolve for external
        // callers.
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("spec.md"), "# Spec").unwrap();
        let data = json!({});

        assert!(evaluate_condition_against("file_exists('spec.md')", &data, dir.path()).unwrap());
        assert!(!evaluate_condition_against("file_exists('nope.md')", &data, dir.path()).unwrap());
        assert!(
            evaluate_condition_against("absolute('spec.md') != ''", &data, dir.path()).unwrap()
        );
        assert!(
            evaluate_condition_against("relative('spec.md') == 'spec.md'", &data, dir.path())
                .unwrap()
        );
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

    #[test]
    fn shortcut_ternary_short_circuits_then_branch() {
        use crate::markdown::compose::expression;

        let data = json!({ "false_flag": false });
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
        // Condition is false, so `ctx.repo` (then-branch) should NOT be evaluated or captured
        let parsed = expression::parse_condition("false_flag ? ctx.repo : 'default'").unwrap();
        let result = expression::evaluate(&parsed, &lookup).unwrap();
        let captured = lookup.captured_groups();
        assert_eq!(
            result,
            serde_json::Value::String("default".to_string()),
            "Ternary should return else-branch when condition is false"
        );
        assert!(
            captured.is_empty(),
            "Repo context should NOT be captured when ternary short-circuits then-branch: captured {:?}",
            captured
        );
    }

    #[test]
    fn shortcut_ternary_short_circuits_else_branch() {
        use crate::markdown::compose::expression;

        let data = json!({ "true_flag": true });
        let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
        // Condition is true, so `ctx.repo` (else-branch) should NOT be evaluated or captured
        let parsed = expression::parse_condition("true_flag ? 'default' : ctx.repo").unwrap();
        let result = expression::evaluate(&parsed, &lookup).unwrap();
        let captured = lookup.captured_groups();
        assert_eq!(
            result,
            serde_json::Value::String("default".to_string()),
            "Ternary should return then-branch when condition is true"
        );
        assert!(
            captured.is_empty(),
            "Repo context should NOT be captured when ternary short-circuits else-branch: captured {:?}",
            captured
        );
    }

    // ── Condition context-typo diagnostics ───────────────────────────────
    //
    // `collect_condition_context_warnings` surfaces the same AST-based ctx-typo
    // check used by interpolation, so `when=` conditions (transclusion /
    // page-block / loop) warn on unknown `ctx.*` references without changing
    // evaluation results.

    #[test]
    fn condition_ctx_typo_emits_warning_with_suggestion() {
        let state = test_state(json!({}));
        let warnings = collect_condition_context_warnings("ctx.toady", &state, "condition");
        assert!(warnings.iter().any(|w| {
            w.message.contains("unknown context variable") && w.message.contains("today")
        }));
    }

    #[test]
    fn condition_ctx_typo_does_not_change_evaluation() {
        // The warning is additive: the typo'd `ctx.toady` still evaluates to a
        // falsy null, exactly as before, without erroring.
        let state = test_state(json!({}));
        assert!(!evaluate_condition("ctx.toady", &state, 1, dummy_ctx()).unwrap());
    }

    #[test]
    fn condition_string_literal_does_not_warn() {
        // Comparing against the literal string "ctx.toady" must not warn —
        // the check walks the AST, so a string literal is never a ctx ref.
        let state = test_state(json!({ "name": "x" }));
        let warnings =
            collect_condition_context_warnings(r#"name == "ctx.toady""#, &state, "condition");
        assert!(!warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    #[test]
    fn condition_valid_ctx_does_not_warn() {
        let state = test_state(json!({}));
        let warnings = collect_condition_context_warnings("ctx.repo", &state, "condition");
        assert!(!warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    #[test]
    fn condition_parse_failure_yields_no_warnings() {
        // A malformed condition is reported fatally by `evaluate_condition`;
        // the additive diagnostic stays silent rather than double-reporting.
        let state = test_state(json!({}));
        let warnings = collect_condition_context_warnings("&& invalid", &state, "condition");
        assert!(warnings.is_empty());
    }

    /// Integration coverage for the operators, access forms, and helpers
    /// added during the expression-syntax expansion. These tests exercise
    /// the same parser and evaluator from the `when=` surface to confirm
    /// behavioral parity with `{{ ... }}` interpolation.
    mod expression_syntax_integration {
        use super::*;

        #[test]
        fn less_than_or_equal_in_when_clause() {
            let state = test_state(json!({ "count": 3 }));
            assert!(eval_cond("count <= 5", &state, 1).unwrap());
            assert!(eval_cond("count <= 3", &state, 1).unwrap());
            assert!(!eval_cond("count <= 2", &state, 1).unwrap());
        }

        #[test]
        fn arithmetic_in_when_clause() {
            let state = test_state(json!({ "count": 10 }));
            assert!(eval_cond("count + 5 == 15", &state, 1).unwrap());
            assert!(eval_cond("count - 5 == 5", &state, 1).unwrap());
            assert!(eval_cond("count * 2 > 15", &state, 1).unwrap());
            assert!(eval_cond("count % 3 == 1", &state, 1).unwrap());
        }

        #[test]
        fn bracket_index_in_when_clause() {
            let state = test_state(json!({ "items": ["a", "b", "c"] }));
            assert!(eval_cond("items[0] == 'a'", &state, 1).unwrap());
            assert!(eval_cond("items[-1] == 'c'", &state, 1).unwrap());
            // Out-of-range -> null -> string compare against literal is false
            assert!(!eval_cond("items[99] == 'a'", &state, 1).unwrap());
        }

        #[test]
        fn bracket_object_key_in_when_clause() {
            let state = test_state(json!({ "config": { "theme": "dark" } }));
            assert!(eval_cond(r#"config["theme"] == 'dark'"#, &state, 1).unwrap());
        }

        #[test]
        fn type_predicates_in_when_clause() {
            let state = test_state(json!({
                "tags": ["one", "two"], "title": "doc", "missing_field": null
            }));
            assert!(eval_cond("is_array(tags)", &state, 1).unwrap());
            assert!(eval_cond("is_string(title)", &state, 1).unwrap());
            assert!(eval_cond("is_null(missing_field)", &state, 1).unwrap());
            assert!(eval_cond("is_empty(missing)", &state, 1).unwrap());
            assert!(!eval_cond("is_empty(tags)", &state, 1).unwrap());
        }

        #[test]
        fn math_helpers_in_when_clause() {
            let state = test_state(json!({ "a": 7, "b": 3 }));
            assert!(eval_cond("min(a, b) == 3", &state, 1).unwrap());
            assert!(eval_cond("max(a, b) == 7", &state, 1).unwrap());
            assert!(eval_cond("abs(-4) == 4", &state, 1).unwrap());
        }

        #[test]
        fn collection_helpers_in_when_clause() {
            let state = test_state(json!({ "items": [10, 20, 30] }));
            assert!(eval_cond("first(items) == 10", &state, 1).unwrap());
            assert!(eval_cond("last(items) == 30", &state, 1).unwrap());
        }

        #[test]
        fn string_predicates_in_when_clause() {
            let state = test_state(json!({ "title": "Hello World" }));
            assert!(eval_cond(r#"starts_with(title, "Hello")"#, &state, 1).unwrap());
            assert!(eval_cond(r#"ends_with(title, "World")"#, &state, 1).unwrap());
            assert!(!eval_cond(r#"starts_with(title, "world")"#, &state, 1).unwrap());
        }

        #[test]
        fn string_mutations_in_when_clause() {
            let state = test_state(json!({ "title": "Hello World" }));
            assert!(eval_cond("lower(title) == 'hello world'", &state, 1).unwrap());
            assert!(eval_cond("kebab_case(title) == 'hello-world'", &state, 1).unwrap());
            assert!(eval_cond("upper(title) == 'HELLO WORLD'", &state, 1).unwrap());
        }

        #[test]
        fn strict_date_validators_in_when_clause() {
            let state = test_state(json!({
                "good_date": "2024-06-15",
                "bad_date": "06-15-2024",
                "dt": "2024-06-15T12:30:00Z"
            }));
            assert!(eval_cond("is_date(good_date)", &state, 1).unwrap());
            assert!(!eval_cond("is_date(bad_date)", &state, 1).unwrap());
            assert!(eval_cond("is_date_time(dt)", &state, 1).unwrap());
            assert!(!eval_cond("is_date(missing)", &state, 1).unwrap());
        }

        #[test]
        fn shortcut_arithmetic_and_bracket_access() {
            let data = json!({
                "count": 5,
                "items": ["a", "b", "c"],
                "config": { "theme": "dark" }
            });
            assert!(
                evaluate_condition_against("count * 2 == 10", &data, std::path::Path::new("."))
                    .unwrap()
            );
            assert!(
                evaluate_condition_against("items[-1] == 'c'", &data, std::path::Path::new("."))
                    .unwrap()
            );
            assert!(
                evaluate_condition_against(
                    r#"config["theme"] == 'dark'"#,
                    &data,
                    std::path::Path::new(".")
                )
                .unwrap()
            );
        }

        #[test]
        fn shortcut_helpers_in_complex_expression() {
            let data = json!({ "items": [1, 2, 3], "title": "Important Notes" });
            assert!(
                evaluate_condition_against(
                    "is_array(items) && length(items) >= 2",
                    &data,
                    std::path::Path::new(".")
                )
                .unwrap()
            );
            assert!(
                evaluate_condition_against(
                    r#"starts_with(lower(title), "important")"#,
                    &data,
                    std::path::Path::new(".")
                )
                .unwrap()
            );
        }

        #[test]
        fn condition_diagnostic_includes_new_operators() {
            // Force a parse error and confirm the operator hint includes <=,
            // arithmetic operators, bracket access, and the expanded helper
            // catalog so users can discover the new syntax from diagnostics.
            use biscuit_terminal::errors::BlockError;
            use biscuit_terminal::terminal::Terminal;

            let err = ConditionError::Parse {
                ctx: Box::new(dummy_ctx()),
                expr: "&& invalid".to_string(),
                line: 1,
                message: "Unexpected token".to_string(),
                span: 0..1,
            };
            let terminal = Terminal::default();
            let block = err.status_block(&terminal);
            let rendered = format!("{block:?}");
            for needle in [
                "<=",
                "+",
                "*",
                "%",
                "[]",
                "min, max, abs",
                "first, last",
                "is_string",
                "starts_with",
                "kebab_case",
                "is_date",
            ] {
                assert!(
                    rendered.contains(needle),
                    "operator hint missing {needle:?}: {rendered}"
                );
            }
        }
    }
}
