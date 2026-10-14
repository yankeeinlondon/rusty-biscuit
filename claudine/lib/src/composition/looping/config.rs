//! Loop frontmatter detection and parsing.

use std::collections::BTreeSet;

use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder, parse};
use serde_json::Value;

use super::super::error::CompositionError;
use crate::invocation_context::DocumentEnvironment;
use super::super::json_util::json_type_name;
use super::super::types::{LoopAction, LoopCondition, LoopConfig, OnRateLimit, ResolvedCompositionSource};

use super::dsl::{parse_actions, parse_positive_usize, parse_string};

/// Read the fail-fast setting from a document's environment, with
/// deprecation support.
///
/// Checks `CLAUDINE_FAIL_FAST` first, then falls back to the legacy
/// `FAIL_FAST` name with a one-time deprecation warning. Like every loop
/// setting, it reads the map the document captured at entry, so
/// re-preparation and later iterations see the same value.
pub fn resolve_fail_fast_from_env(environment: &DocumentEnvironment) -> Option<bool> {
    if let Some(value) = environment.get("CLAUDINE_FAIL_FAST") {
        return value.parse().ok();
    }
    if let Some(value) = environment.get("FAIL_FAST") {
        static WARN_ONCE: std::sync::Once = std::sync::Once::new();
        WARN_ONCE.call_once(|| {
            tracing::warn!("The FAIL_FAST env var is deprecated. Use CLAUDINE_FAIL_FAST instead.");
        });
        return value.parse().ok();
    }
    None
}

/// Read the max-iterations setting (`CLAUDINE_MAX_ITERATIONS`) from a
/// document's environment.
pub fn resolve_max_iterations_from_env(environment: &DocumentEnvironment) -> Option<usize> {
    environment
        .get("CLAUDINE_MAX_ITERATIONS")
        .and_then(|v| v.parse().ok())
}

/// Read the rate-limit pause reset-margin override from a document's
/// environment.
///
/// Checks `CLAUDINE_PAUSE_RESET_MARGIN`, parsed with the shared timeout
/// grammar (e.g. `5s`, `2m`, `0.5s`). Invalid values are ignored so the
/// engine falls back to its built-in margin. Primarily a test knob to keep
/// pause-policy coverage fast; also lets operators tune skew tolerance.
pub fn resolve_pause_reset_margin_from_env(
    environment: &DocumentEnvironment,
) -> Option<std::time::Duration> {
    let raw = environment.get("CLAUDINE_PAUSE_RESET_MARGIN")?;
    crate::harness::parse_timeout(raw, std::path::Path::new("<CLAUDINE_PAUSE_RESET_MARGIN>")).ok()
}

/// Detect and parse a loop configuration from resolved composition frontmatter.
///
/// Returns `Ok(None)` when the document has no `loop` key.
///
/// ## Errors
///
/// Returns `Err` when the `loop` value is not an object, when condition keys
/// are missing or conflicting, or when action/max/fail-fast values have invalid
/// shapes.
pub fn resolve_loop_config(
    source: &ResolvedCompositionSource,
) -> Result<Option<LoopConfig>, CompositionError> {
    let fm = source.markdown.frontmatter();
    let Some(loop_value) = fm.as_map().get("loop") else {
        return Ok(None);
    };

    let loop_map = loop_value.as_object().ok_or_else(|| {
        CompositionError::LoopInvalid(format!(
            "`loop` must be an object, got {}",
            json_type_name(loop_value)
        ))
    })?;

    reject_unknown_loop_keys(loop_map)?;

    let condition = parse_condition(loop_map)?;
    let actions = match (loop_map.get("action"), loop_map.get("actions")) {
        (Some(_), Some(_)) => {
            return Err(CompositionError::LoopInvalid(
                "`loop.action` and `loop.actions` are aliases; specify only one".to_string(),
            ));
        }
        (Some(value), None) | (None, Some(value)) => parse_actions(value)?,
        (None, None) => Vec::new(),
    };
    let max_iterations = match loop_map.get("max") {
        Some(value) => Some(parse_positive_usize("loop.max", value)?),
        None => None,
    };
    let fail_fast = match loop_map.get("fail_fast") {
        Some(serde_json::Value::Bool(value)) => Some(*value),
        Some(other) => {
            return Err(CompositionError::LoopInvalid(format!(
                "`loop.fail_fast` must be a boolean, got {}",
                json_type_name(other)
            )));
        }
        None => None,
    };
    let on_rate_limit = match loop_map.get("on_rate_limit") {
        Some(serde_json::Value::String(raw)) => {
            Some(OnRateLimit::parse(raw).map_err(|why| {
                CompositionError::LoopInvalid(format!("`loop.on_rate_limit` {why}"))
            })?)
        }
        Some(other) => {
            return Err(CompositionError::LoopInvalid(format!(
                "`loop.on_rate_limit` must be a string, got {}",
                json_type_name(other)
            )));
        }
        None => None,
    };

    Ok(Some(LoopConfig {
        condition,
        actions,
        max_iterations,
        fail_fast,
        on_rate_limit,
    }))
}

/// Extract the set of frontmatter keys the loop reads or writes.
///
/// Control variables are the only keys resolved once at seed time and
/// carried as typed state across iterations. Derived/presentation keys are
/// intentionally excluded so they re-resolve each iteration.
///
/// Sources:
/// - every action target (`increment`/`decrement`/`set`/`append`/
///   `prepend`/`merge` `prop`);
/// - every identifier referenced by the `while`/`until` condition;
/// - every identifier referenced inside action-value templates.
///
/// Reserved namespaces are excluded because they are supplied by the
/// runtime rather than resolved from frontmatter: `fm.<head>` references
/// lift `<head>` as a control variable (the `fm` namespace traverses the
/// loop's frontmatter state), while bare `fm`, `env`, boolean literals,
/// and the host-declared `loop` global remain excluded.
pub fn extract_control_variables(config: &LoopConfig) -> Vec<String> {
    let mut names = BTreeSet::new();

    for action in &config.actions {
        match action {
            LoopAction::Increment(prop) | LoopAction::Decrement(prop) => {
                names.insert(prop.clone());
            }
            LoopAction::Set { prop, value }
            | LoopAction::Append { prop, value }
            | LoopAction::Prepend { prop, value }
            | LoopAction::Merge { prop, value } => {
                names.insert(prop.clone());
                if let Value::String(raw) = value {
                    collect_value_template_identifiers(raw, &mut names);
                }
            }
        }
    }

    let condition_source = match &config.condition {
        LoopCondition::While(source) | LoopCondition::Until(source) => source,
    };
    if let Ok(expr) = darkmatter::markdown::compose::expression::parse_condition(condition_source) {
        collect_identifiers(&expr, &mut names);
    }

    Vec::from_iter(names)
}

fn collect_value_template_identifiers(raw: &str, names: &mut BTreeSet<String>) {
    for location in ExpressionFinder::find_all_plain(raw) {
        if let Ok(expr) = parse(&location.expression) {
            collect_identifiers(&expr, names);
        }
    }
}

fn collect_identifiers(expr: &Expr, names: &mut BTreeSet<String>) {
    match expr {
        Expr::Variable(path) => {
            let mut segments = path.split('.');
            let head = segments.next().unwrap_or(path);
            if head == darkmatter::markdown::compose::expression::FRONTMATTER_ROOT {
                // `fm.<path>` traverses the frontmatter object the loop
                // owns, so lift the first segment after `fm` as a control
                // variable. Bare `fm` (the whole object) has no single key
                // to own.
                if let Some(segment) = segments.next() {
                    names.insert(segment.to_string());
                }
            } else if !is_reserved_identifier(head) {
                names.insert(head.to_string());
            }
        }
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            collect_identifiers(inner, names);
        }
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Fallback {
            primary: left,
            fallback: right,
        } => {
            collect_identifiers(left, names);
            collect_identifiers(right, names);
        }
        Expr::Index { base, index } => {
            collect_identifiers(base, names);
            collect_identifiers(index, names);
        }
        Expr::MemberAccess { base, .. } => {
            collect_identifiers(base, names);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_identifiers(condition, names);
            collect_identifiers(then_branch, names);
            collect_identifiers(else_branch, names);
        }
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                collect_identifiers(arg, names);
            }
        }
        // A container literal contributes the union of the identifiers its
        // elements reference. Object keys are authored text, not variable
        // references, so they never name a control variable.
        Expr::ArrayLiteral(elements) => {
            for element in elements {
                collect_identifiers(element, names);
            }
        }
        Expr::ObjectLiteral(entries) => {
            for (_, value) in entries {
                collect_identifiers(value, names);
            }
        }
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {}
    }
}

fn is_reserved_identifier(name: &str) -> bool {
    super::super::reserved::EXPRESSION_RESERVED_ROOTS.contains(&name)
        || name == super::super::reserved::LOOP_ROOT
}

/// Iteration-control keys under the `loop:` frontmatter object.
///
/// `action` is the canonical key for action mutators; `actions` is accepted
/// as an alias. Every other key of a `loop:` block must be a lifecycle concern
/// (see [`is_lifecycle_concern_key`]); anything else is rejected at parse time
/// so a typo surfaces as a clear error rather than being ignored.
const ITERATION_CONTROL_KEYS: &[&str] = super::super::lifecycle::LOOP_ITERATION_KEYS;

/// Whether `key` belongs to the lifecycle parser: a lifecycle verb, `gate:`,
/// the sibling `no_error`, or a spelling the lifecycle parser rejects with a
/// typed diagnostic (`when` at the loop root, the removed `stack` and
/// `say_first`). The two parsers read disjoint key sets of one `loop:` block,
/// and this parser leaves those keys to
/// [`super::super::lifecycle::parse_lifecycle_config`].
fn is_lifecycle_concern_key(key: &str) -> bool {
    matches!(key, "gate" | "no_error" | "when" | "stack" | "say_first" | "set")
        || super::super::lifecycle::actions::is_known_lifecycle_verb(key)
}

fn reject_unknown_loop_keys(
    loop_map: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), CompositionError> {
    for key in loop_map.keys() {
        if !ITERATION_CONTROL_KEYS.contains(&key.as_str()) && !is_lifecycle_concern_key(key) {
            let suggestion = suggest_loop_key(key);
            let suggestion_hint = suggestion
                .map(|s| format!(" (did you mean `{s}`?)"))
                .unwrap_or_default();
            return Err(CompositionError::LoopInvalid(format!(
                "unknown `loop.{key}` key{suggestion_hint}; valid keys are: {}",
                ITERATION_CONTROL_KEYS.join(", ")
            )));
        }
    }
    Ok(())
}

fn suggest_loop_key(unknown: &str) -> Option<&'static str> {
    let lower = unknown.to_ascii_lowercase();
    match lower.as_str() {
        "loops" | "iterations" | "max_iterations" | "max-iterations" => Some("max"),
        "failfast" | "fail-fast" => Some("fail_fast"),
        "whilst" => Some("while"),
        "onratelimit" | "on-rate-limit" | "rate_limit" | "ratelimit" => Some("on_rate_limit"),
        _ => None,
    }
}

fn parse_condition(
    loop_map: &serde_json::Map<String, serde_json::Value>,
) -> Result<LoopCondition, CompositionError> {
    let while_value = loop_map.get("while");
    let until_value = loop_map.get("until");

    match (while_value, until_value) {
        (Some(_), Some(_)) => Err(CompositionError::LoopInvalid(
            "`loop.while` and `loop.until` are mutually exclusive".to_string(),
        )),
        (Some(value), None) => Ok(LoopCondition::While(parse_string("loop.while", value)?)),
        (None, Some(value)) => Ok(LoopCondition::Until(parse_string("loop.until", value)?)),
        (None, None) => Err(CompositionError::LoopInvalid(
            "`loop` must define either `while` or `until`".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests;
