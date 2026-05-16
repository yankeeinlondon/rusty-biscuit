//! Loop frontmatter detection and parsing.

use super::error::CompositionError;
use super::types::{LoopAction, LoopCondition, LoopConfig, OnRateLimit, ResolvedCompositionSource};

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
        Some(serde_json::Value::String(raw)) => Some(OnRateLimit::parse(raw).map_err(|why| {
            CompositionError::LoopInvalid(format!("`loop.on_rate_limit` {why}"))
        })?),
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

/// Recognized keys under the `loop:` frontmatter object.
///
/// `action` is the canonical key for action mutators; `actions` is accepted
/// as an alias for backwards compatibility. Any other key is rejected at
/// parse time so silent typos surface as a clear error rather than being
/// ignored.
const KNOWN_LOOP_KEYS: &[&str] = &[
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
                KNOWN_LOOP_KEYS.join(", ")
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

fn parse_actions(value: &serde_json::Value) -> Result<Vec<LoopAction>, CompositionError> {
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                parse_action(item).map_err(|err| annotate_action_error(err, index))
            })
            .collect(),
        serde_json::Value::String(_) | serde_json::Value::Object(_) => {
            parse_action(value).map(|action| vec![action])
        }
        other => Err(CompositionError::LoopInvalid(format!(
            "`loop.actions` must be a string, object, or list of strings/objects, got {}",
            json_type_name(other)
        ))),
    }
}

fn annotate_action_error(err: CompositionError, index: usize) -> CompositionError {
    match err {
        CompositionError::LoopInvalid(message) => {
            CompositionError::LoopInvalid(format!("`loop.actions[{index}]`: {message}"))
        }
        other => other,
    }
}

fn parse_action(value: &serde_json::Value) -> Result<LoopAction, CompositionError> {
    match value {
        serde_json::Value::String(raw) => parse_dsl_action(raw),
        serde_json::Value::Object(map) => parse_structured_action(map),
        other => Err(CompositionError::LoopInvalid(format!(
            "action must be a string or object, got {}",
            json_type_name(other)
        ))),
    }
}

fn parse_dsl_action(raw: &str) -> Result<LoopAction, CompositionError> {
    let trimmed = raw.trim();
    let open = trimmed.find('(').ok_or_else(|| {
        CompositionError::LoopInvalid(format!("invalid action `{raw}`; expected op(...)"))
    })?;
    let close = trimmed.rfind(')').ok_or_else(|| {
        CompositionError::LoopInvalid(format!("invalid action `{raw}`; missing closing `)`"))
    })?;
    if close != trimmed.len() - 1 || open == 0 {
        return Err(CompositionError::LoopInvalid(format!(
            "invalid action `{raw}`; expected op(...)"
        )));
    }

    let op = trimmed[..open].trim();
    let args = split_action_args(&trimmed[open + 1..close])?;
    match op {
        "increment" => parse_unary_action(op, args, LoopAction::Increment),
        "decrement" => parse_unary_action(op, args, LoopAction::Decrement),
        "set" => parse_value_action(op, args, |prop, value| LoopAction::Set { prop, value }),
        "append" => parse_value_action(op, args, |prop, value| LoopAction::Append { prop, value }),
        "prepend" => {
            parse_value_action(op, args, |prop, value| LoopAction::Prepend { prop, value })
        }
        "merge" => parse_value_action(op, args, |prop, value| LoopAction::Merge { prop, value }),
        other => Err(CompositionError::LoopInvalid(format!(
            "unknown loop action op `{other}`"
        ))),
    }
}

fn parse_structured_action(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<LoopAction, CompositionError> {
    let op = map
        .get("op")
        .ok_or_else(|| CompositionError::LoopInvalid("action object is missing `op`".to_string()))
        .and_then(|value| parse_string("action.op", value))?;
    let prop = map
        .get("prop")
        .ok_or_else(|| CompositionError::LoopInvalid("action object is missing `prop`".to_string()))
        .and_then(|value| parse_property("action.prop", value))?;

    match op.as_str() {
        "increment" => Ok(LoopAction::Increment(prop)),
        "decrement" => Ok(LoopAction::Decrement(prop)),
        "set" => Ok(LoopAction::Set {
            prop,
            value: parse_structured_action_value(map)?,
        }),
        "append" => Ok(LoopAction::Append {
            prop,
            value: parse_structured_action_value(map)?,
        }),
        "prepend" => Ok(LoopAction::Prepend {
            prop,
            value: parse_structured_action_value(map)?,
        }),
        "merge" => Ok(LoopAction::Merge {
            prop,
            value: parse_structured_action_value(map)?,
        }),
        other => Err(CompositionError::LoopInvalid(format!(
            "unknown loop action op `{other}`"
        ))),
    }
}

fn parse_structured_action_value(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, CompositionError> {
    map.get("value").cloned().ok_or_else(|| {
        CompositionError::LoopInvalid("action object is missing `value`".to_string())
    })
}

fn parse_unary_action(
    op: &str,
    args: Vec<String>,
    build: impl FnOnce(String) -> LoopAction,
) -> Result<LoopAction, CompositionError> {
    if args.len() != 1 {
        return Err(CompositionError::LoopInvalid(format!(
            "`{op}` expects 1 argument, got {}",
            args.len()
        )));
    }
    parse_property(op, &serde_json::Value::String(args[0].clone())).map(build)
}

fn parse_value_action(
    op: &str,
    args: Vec<String>,
    build: impl FnOnce(String, serde_json::Value) -> LoopAction,
) -> Result<LoopAction, CompositionError> {
    if args.len() != 2 {
        return Err(CompositionError::LoopInvalid(format!(
            "`{op}` expects 2 arguments, got {}",
            args.len()
        )));
    }
    let prop = parse_property(op, &serde_json::Value::String(args[0].clone()))?;
    let value = parse_dsl_value(&args[1]);
    Ok(build(prop, value))
}

fn parse_string(field: &str, value: &serde_json::Value) -> Result<String, CompositionError> {
    match value {
        serde_json::Value::String(value) => Ok(value.clone()),
        other => Err(CompositionError::LoopInvalid(format!(
            "`{field}` must be a string, got {}",
            json_type_name(other)
        ))),
    }
}

fn parse_property(field: &str, value: &serde_json::Value) -> Result<String, CompositionError> {
    let prop = parse_string(field, value)?;
    if prop.trim().is_empty() {
        return Err(CompositionError::LoopInvalid(format!(
            "`{field}` must not be empty"
        )));
    }
    Ok(prop.trim().to_string())
}

fn parse_positive_usize(field: &str, value: &serde_json::Value) -> Result<usize, CompositionError> {
    let Some(raw) = value.as_u64() else {
        return Err(CompositionError::LoopInvalid(format!(
            "`{field}` must be a positive integer, got {}",
            json_type_name(value)
        )));
    };
    if raw == 0 {
        return Err(CompositionError::LoopInvalid(format!(
            "`{field}` must be greater than zero"
        )));
    }
    usize::try_from(raw).map_err(|_| {
        CompositionError::LoopInvalid(format!("`{field}` is too large for this platform"))
    })
}

fn parse_dsl_value(raw: &str) -> serde_json::Value {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return value;
    }
    serde_json::Value::String(
        trimmed
            .trim_matches(|ch| ch == '\'' || ch == '"')
            .to_string(),
    )
}

fn split_action_args(input: &str) -> Result<Vec<String>, CompositionError> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escape = false;
    let mut depth = 0usize;

    for ch in input.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        if ch == '\\' {
            current.push(ch);
            escape = true;
            continue;
        }

        if let Some(quote_ch) = quote {
            current.push(ch);
            if ch == quote_ch {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' => {
                quote = Some(ch);
                current.push(ch);
            }
            '[' | '{' | '(' => {
                depth += 1;
                current.push(ch);
            }
            ']' | '}' | ')' => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    CompositionError::LoopInvalid(
                        "unbalanced action argument delimiters".to_string(),
                    )
                })?;
                current.push(ch);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if quote.is_some() || depth != 0 {
        return Err(CompositionError::LoopInvalid(
            "unterminated action argument".to_string(),
        ));
    }

    if !current.trim().is_empty() || input.ends_with(',') {
        args.push(current.trim().to_string());
    }

    Ok(args)
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
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
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "on_rate_limit": "pause"}),
        )]);
        let config = resolve_loop_config(&source).unwrap().unwrap();
        assert_eq!(config.on_rate_limit, Some(OnRateLimit::Pause));
    }

    #[test]
    fn on_rate_limit_abort_parses() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "on_rate_limit": "abort"}),
        )]);
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
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "on_rate_limit": "halt"}),
        )]);
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
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "on_rate_limit": true}),
        )]);
        let err = resolve_loop_config(&source).unwrap_err();
        assert!(
            matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("must be a string"))
        );
    }

    #[test]
    fn on_rate_limit_typo_suggests_canonical_key() {
        let source = make_source(&[(
            "loop",
            json!({"while": "true", "on-rate-limit": "abort"}),
        )]);
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
}
