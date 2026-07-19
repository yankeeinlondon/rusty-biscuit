//! Loop frontmatter detection and parsing.

use std::collections::BTreeSet;

use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder, parse};
use serde_json::Value;

use super::super::error::CompositionError;
use super::super::json_util::json_type_name;
use super::super::types::{LoopAction, LoopCondition, LoopConfig, OnRateLimit, ResolvedCompositionSource};

use super::dsl::{parse_actions, parse_positive_usize, parse_string};

/// Read the fail-fast setting from env vars with deprecation support.
///
/// Checks `CLAUDINE_FAIL_FAST` first, then falls back to the legacy
/// `FAIL_FAST` name with a one-time deprecation warning.
pub fn resolve_fail_fast_from_env() -> Option<bool> {
    if let Ok(value) = std::env::var("CLAUDINE_FAIL_FAST") {
        return value.parse().ok();
    }
    if let Ok(value) = std::env::var("FAIL_FAST") {
        static WARN_ONCE: std::sync::Once = std::sync::Once::new();
        WARN_ONCE.call_once(|| {
            tracing::warn!("The FAIL_FAST env var is deprecated. Use CLAUDINE_FAIL_FAST instead.");
        });
        return value.parse().ok();
    }
    None
}

/// Read the max-iterations setting from env.
///
/// Checks `CLAUDINE_MAX_ITERATIONS`.
pub fn resolve_max_iterations_from_env() -> Option<usize> {
    std::env::var("CLAUDINE_MAX_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
}

/// Read the rate-limit pause reset-margin override from env.
///
/// Checks `CLAUDINE_PAUSE_RESET_MARGIN`, parsed with the shared timeout
/// grammar (e.g. `5s`, `2m`, `0.5s`). Invalid values are ignored so the
/// engine falls back to its built-in margin. Primarily a test knob to keep
/// pause-policy coverage fast; also lets operators tune skew tolerance.
pub fn resolve_pause_reset_margin_from_env() -> Option<std::time::Duration> {
    let raw = std::env::var("CLAUDINE_PAUSE_RESET_MARGIN").ok()?;
    crate::harness::parse_timeout(&raw, std::path::Path::new("<CLAUDINE_PAUSE_RESET_MARGIN>")).ok()
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
/// runtime rather than resolved from frontmatter: `doc.<head>` references
/// lift `<head>` as a control variable (the `doc` namespace traverses the
/// loop's frontmatter state), while bare `doc`, `env`, boolean literals,
/// and any identifier starting with `_loop_` remain excluded.
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
            if head == "doc" {
                // `doc.<path>` traverses the frontmatter object the loop
                // owns, so lift the first segment after `doc` as a control
                // variable. Bare `doc` (the whole object) has no single key
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
        || name.starts_with("_loop_")
}

/// Recognized iteration-control keys under the `loop:` frontmatter object.
///
/// `action` is the canonical key for action mutators; `actions` is accepted
/// as an alias for backwards compatibility. Any other iteration-control key
/// is rejected at parse time so silent typos surface as a clear error rather
/// than being ignored.
///
/// The lifecycle-concern keys (`say`, `say_first`, `effect`, `message`,
/// `stderr`, `notify`, `info`, `warn`, `stack`) are also accepted but
/// skipped by this parser — they are validated and stored by
/// [`super::lifecycle::parse_lifecycle_config`], which extracts them as a
/// `LifecycleNotification` on `LifecycleConfig::loop_concerns`. The two
/// parsers operate on disjoint key sets inside the same `loop:` block.
const KNOWN_LOOP_KEYS: &[&str] = &[
    "while",
    "until",
    "action",
    "actions",
    "max",
    "fail_fast",
    "on_rate_limit",
    // Lifecycle-concern keys — parsed by `parse_lifecycle_config` into
    // `LifecycleConfig::loop_concerns`. Listed here so this parser does
    // not reject them as unknown keys.
    "say",
    "say_first",
    "effect",
    "message",
    "stderr",
    "notify",
    "info",
    "warn",
    "success",
    "stdout",
    "stack",
];

/// Iteration-control keys only, for diagnostic listings. The lifecycle
/// concerns are valid keys too but are surfaced through a different error
/// path (see [`super::lifecycle::parse_lifecycle_config`]).
const ITERATION_CONTROL_KEYS: &[&str] = &[
    "while",
    "until",
    "action",
    "actions",
    "max",
    "fail_fast",
    "on_rate_limit",
];

fn reject_unknown_loop_keys(
    loop_map: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), CompositionError> {
    for key in loop_map.keys() {
        if !KNOWN_LOOP_KEYS.contains(&key.as_str()) {
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
mod tests {
    use super::*;
    use darkmatter::markdown::{Frontmatter, Markdown};
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    fn make_source(frontmatter: &[(&str, serde_json::Value)]) -> ResolvedCompositionSource {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("loop.md");
        let mut fm = Frontmatter::new();
        for (key, value) in frontmatter {
            fm.insert(key, value.clone()).unwrap();
        }
        let md = Markdown::with_frontmatter(fm, "Body");
        fs::write(&file, md.as_string()).unwrap();
        let original_text = fs::read_to_string(&file).unwrap();
        let markdown: Markdown = original_text.clone().into();
        ResolvedCompositionSource {
            original_ref: file.to_string_lossy().to_string(),
            resolved_path: file,
            original_text,
            markdown,
        }
    }

    #[test]
    fn singular_action_key_is_accepted() {
        let source = make_source(&[(
            "loop",
            json!({"until": "done", "action": "increment(counter)"}),
        )]);
        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(
            config.actions,
            vec![LoopAction::Increment("counter".into())]
        );
    }

    #[test]
    fn plural_actions_key_is_accepted_as_alias() {
        let source = make_source(&[(
            "loop",
            json!({"until": "done", "actions": "increment(counter)"}),
        )]);
        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(
            config.actions,
            vec![LoopAction::Increment("counter".into())]
        );
    }

    #[test]
    fn action_and_actions_together_are_rejected() {
        let source = make_source(&[(
            "loop",
            json!({
                "until": "done",
                "action": "increment(a)",
                "actions": "increment(b)"
            }),
        )]);
        let err = resolve_loop_config(&source).unwrap_err();
        let CompositionError::LoopInvalid(message) = err else {
            panic!("expected LoopInvalid");
        };
        assert!(
            message.contains("aliases; specify only one"),
            "got: {message}"
        );
    }

    #[test]
    fn unknown_loop_key_without_suggestion_lists_valid_keys() {
        let source = make_source(&[("loop", json!({"until": "done", "frequency": 5}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        let CompositionError::LoopInvalid(message) = err else {
            panic!("expected LoopInvalid");
        };
        assert!(
            message.contains("unknown `loop.frequency` key"),
            "got: {message}"
        );
        assert!(message.contains("actions"), "got: {message}");
        assert!(!message.contains("did you mean"), "got: {message}");
    }

    #[test]
    fn no_loop_returns_none() {
        let source = make_source(&[("title", json!("No loop"))]);
        assert!(resolve_loop_config(&source).unwrap().is_none());
    }

    #[test]
    fn loop_stdout_is_accepted_as_lifecycle_concern() {
        // `loop.stdout` is a lifecycle-concern key, not an iteration control.
        // `resolve_loop_config` must accept it (ignoring it as a concern) rather
        // than rejecting it as an unknown `LoopInvalid` key. The `while` key
        // keeps the iteration controls valid.
        let source = make_source(&[("loop", json!({"while": "true", "stdout": "hello"}))]);
        let config = resolve_loop_config(&source).unwrap();
        assert!(config.is_some());
    }

    #[test]
    fn scalar_dsl_action_parses() {
        let source = make_source(&[(
            "loop",
            json!({
                "while": "counter < 3",
                "actions": "increment(counter)",
                "max": 3,
                "fail_fast": false
            }),
        )]);

        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(config.condition, LoopCondition::While("counter < 3".into()));
        assert_eq!(
            config.actions,
            vec![LoopAction::Increment("counter".into())]
        );
        assert_eq!(config.max_iterations, Some(3));
        assert_eq!(config.fail_fast, Some(false));
    }

    #[test]
    fn list_of_dsl_actions_parses() {
        let source = make_source(&[(
            "loop",
            json!({
                "until": "done",
                "actions": [
                    "append(log, {\"event\":\"tick\"})",
                    "set(done, true)"
                ]
            }),
        )]);

        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(config.condition, LoopCondition::Until("done".into()));
        assert_eq!(
            config.actions,
            vec![
                LoopAction::Append {
                    prop: "log".into(),
                    value: json!({"event": "tick"})
                },
                LoopAction::Set {
                    prop: "done".into(),
                    value: json!(true)
                }
            ]
        );
    }

    #[test]
    fn structured_actions_parse() {
        let source = make_source(&[(
            "loop",
            json!({
                "while": "counter < 5",
                "actions": [
                    {"op": "merge", "prop": "state", "value": {"phase": "review"}},
                    {"op": "decrement", "prop": "remaining"}
                ]
            }),
        )]);

        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(
            config.actions,
            vec![
                LoopAction::Merge {
                    prop: "state".into(),
                    value: json!({"phase": "review"})
                },
                LoopAction::Decrement("remaining".into())
            ]
        );
    }

    #[test]
    fn condition_keys_are_mutually_exclusive() {
        let source = make_source(&[("loop", json!({"while": "true", "until": "done"}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("mutually exclusive"))
        );
    }

    #[test]
    fn condition_is_required() {
        let source = make_source(&[("loop", json!({"actions": "increment(counter)"}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("either `while` or `until`"))
        );
    }

    #[test]
    fn invalid_action_syntax_fails() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "actions": ["increment(counter)", "wat"]}),
        )]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("loop.actions[1]")),
            "got {err}"
        );
    }

    #[test]
    fn max_must_be_positive() {
        let source = make_source(&[("loop", json!({"while": "true", "max": 0}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("greater than zero"))
        );
    }

    #[test]
    fn fail_fast_must_be_boolean() {
        let source = make_source(&[("loop", json!({"while": "true", "fail_fast": "false"}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("loop.fail_fast"))
        );
    }

    #[test]
    fn on_rate_limit_pause_parses() {
        let source = make_source(&[("loop", json!({"while": "true", "on_rate_limit": "pause"}))]);
        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(config.on_rate_limit, Some(OnRateLimit::Pause));
    }

    #[test]
    fn on_rate_limit_abort_parses() {
        let source = make_source(&[("loop", json!({"while": "true", "on_rate_limit": "abort"}))]);
        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(config.on_rate_limit, Some(OnRateLimit::Abort));
    }

    #[test]
    fn on_rate_limit_continue_parses() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "on_rate_limit": "continue"}),
        )]);
        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(config.on_rate_limit, Some(OnRateLimit::Continue));
    }

    #[test]
    fn on_rate_limit_unknown_value_is_rejected() {
        let source = make_source(&[("loop", json!({"while": "true", "on_rate_limit": "halt"}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        let CompositionError::LoopInvalid(message) = err else {
            panic!("expected LoopInvalid");
        };
        assert!(
            message.contains("loop.on_rate_limit") && message.contains("halt"),
            "got: {message}"
        );
    }

    #[test]
    fn on_rate_limit_non_string_is_rejected() {
        let source = make_source(&[("loop", json!({"while": "true", "on_rate_limit": true}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("must be a string"))
        );
    }

    #[test]
    fn on_rate_limit_typo_suggests_canonical_key() {
        let source = make_source(&[("loop", json!({"while": "true", "on-rate-limit": "abort"}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        let CompositionError::LoopInvalid(message) = err else {
            panic!("expected LoopInvalid");
        };
        assert!(message.contains("on_rate_limit"), "got: {message}");
    }

    #[test]
    fn on_rate_limit_default_is_none() {
        let source = make_source(&[("loop", json!({"while": "true"}))]);
        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(config.on_rate_limit, None);
    }

    #[test]
    fn action_object_value_is_required_for_value_ops() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "actions": {"op": "set", "prop": "done"}}),
        )]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("missing `value`"))
        );
    }

    #[test]
    fn max_must_be_positive_integer() {
        let source = make_source(&[("loop", json!({"while": "true", "max": "five"}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("loop.max") && msg.contains("positive integer"))
        );
    }

    #[test]
    fn actions_must_be_string_object_or_list() {
        let source = make_source(&[("loop", json!({"while": "true", "actions": 42}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("loop.actions") && msg.contains("string, object, or list"))
        );
    }

    #[test]
    fn invalid_dsl_action_missing_closing_paren() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "actions": "increment(counter"}),
        )]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("missing closing")),
            "got {err}"
        );
    }

    #[test]
    fn invalid_dsl_action_unknown_op() {
        let source = make_source(&[("loop", json!({"while": "true", "actions": "unknown(prop)"}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("unknown loop action op")),
            "got {err}"
        );
    }

    #[test]
    fn invalid_dsl_action_wrong_arg_count() {
        let source = make_source(&[("loop", json!({"while": "true", "actions": "increment()"}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("expects 1 argument")),
            "got {err}"
        );
    }

    #[test]
    fn structured_action_missing_op() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "actions": {"prop": "counter"}}),
        )]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("missing `op`")));
    }

    #[test]
    fn structured_action_missing_prop() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "actions": {"op": "increment"}}),
        )]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("missing `prop`"))
        );
    }

    #[test]
    fn structured_action_unknown_op() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "actions": {"op": "unknown", "prop": "counter"}}),
        )]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("unknown loop action op")),
            "got {err}"
        );
    }

    #[test]
    fn empty_loop_object_requires_condition() {
        let source = make_source(&[("loop", json!({}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("either `while` or `until`"))
        );
    }

    #[test]
    fn loop_must_be_an_object() {
        let source = make_source(&[("loop", json!("while counter < 3"))]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("must be an object")),
            "got {err}"
        );
    }

    #[test]
    fn scalar_object_action_parses_json_value() {
        let source = make_source(&[(
            "loop",
            json!({
                "while": "counter < 3",
                "actions": "set(counter, 5)"
            }),
        )]);

        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(
            config.actions,
            vec![LoopAction::Set {
                prop: "counter".into(),
                value: json!(5)
            }]
        );
    }

    #[test]
    fn scalar_string_action_parses_quoted_string() {
        let source = make_source(&[(
            "loop",
            json!({
                "while": "counter < 3",
                "actions": "set(msg, 'hello world')"
            }),
        )]);

        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(
            config.actions,
            vec![LoopAction::Set {
                prop: "msg".into(),
                value: json!("hello world")
            }]
        );
    }

    // ── Control-variable extraction tests ────────────────────────────────

    fn control_config(condition: &str, actions: Vec<LoopAction>) -> LoopConfig {
        LoopConfig {
            condition: LoopCondition::Until(condition.to_string()),
            actions,
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        }
    }

    #[test]
    fn extract_control_variables_repro_shape() {
        let config = control_config(
            "phase > total_phases",
            vec![LoopAction::Increment("phase".into())],
        );
        assert_eq!(
            extract_control_variables(&config),
            vec!["phase".to_string(), "total_phases".to_string()]
        );
    }

    #[test]
    fn extract_control_variables_action_value_template() {
        let config = control_config(
            "phase < max",
            vec![LoopAction::Set {
                prop: "next".into(),
                value: json!("{{ phase + 1 }}"),
            }],
        );
        assert_eq!(
            extract_control_variables(&config),
            vec!["max".to_string(), "next".to_string(), "phase".to_string()]
        );
    }

    #[test]
    fn extract_control_variables_excludes_reserved_namespaces() {
        let config = control_config("_loop_count < 3 && env.DEBUG", vec![]);
        assert!(extract_control_variables(&config).is_empty());
    }

    #[test]
    fn extract_control_variables_dotted_condition_path() {
        let config = control_config("state.done", vec![]);
        assert_eq!(
            extract_control_variables(&config),
            vec!["state".to_string()]
        );
    }

    #[test]
    fn extract_control_variables_empty_identity() {
        let config = control_config("true", vec![]);
        assert!(extract_control_variables(&config).is_empty());
    }

    #[test]
    fn extract_control_variables_lifts_doc_namespace_head() {
        let config = control_config("doc.counter < doc.total", vec![]);
        assert_eq!(
            extract_control_variables(&config),
            vec!["counter".to_string(), "total".to_string()]
        );
    }

    #[test]
    fn extract_control_variables_dotted_doc_path_lifts_head_only() {
        let config = control_config("doc.config.retries > 0", vec![]);
        assert_eq!(
            extract_control_variables(&config),
            vec!["config".to_string()]
        );
    }

    #[test]
    fn extract_control_variables_bare_doc_lifts_nothing() {
        let config = control_config("doc && counter < 3", vec![]);
        assert_eq!(
            extract_control_variables(&config),
            vec!["counter".to_string()]
        );
    }

    #[test]
    fn extract_control_variables_descends_array_literal_elements() {
        // An array literal contributes the union of the identifiers its
        // elements reference; a no-op arm would drop `phase` and `max`.
        let config = control_config("length([phase, max]) > 0", vec![]);
        assert_eq!(
            extract_control_variables(&config),
            vec!["max".to_string(), "phase".to_string()]
        );
    }

    #[test]
    fn extract_control_variables_descends_object_literal_values_not_keys() {
        // Object *values* are expressions and contribute identifiers; the
        // keys are authored text and must not be lifted as control
        // variables (`stage` below is a key, not a reference).
        let config = control_config("{ stage: phase, limit: max }", vec![]);
        assert_eq!(
            extract_control_variables(&config),
            vec!["max".to_string(), "phase".to_string()]
        );
    }

    #[test]
    fn extract_control_variables_doc_and_action_target_merge() {
        let config = control_config(
            "doc.counter < doc.total",
            vec![LoopAction::Increment("counter".into())],
        );
        assert_eq!(
            extract_control_variables(&config),
            vec!["counter".to_string(), "total".to_string()]
        );
    }
}
