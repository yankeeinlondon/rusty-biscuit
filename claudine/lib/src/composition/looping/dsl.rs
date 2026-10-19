//! The `loop:` value grammar: action strings and objects, conditions, and
//! caps. Problems come back as [`LoopIssue`]s, which the caller locates.

use super::super::json_util::json_type_name;
use super::super::types::LoopAction;

/// A loop configuration problem, before its file is attached.
#[derive(Debug)]
pub(super) struct LoopIssue {
    /// The authored path of the offending key (`loop.actions[1]`).
    pub(super) property: String,
    pub(super) message: String,
}

impl LoopIssue {
    pub(super) fn new(property: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            property: property.into(),
            message: message.into(),
        }
    }
}

/// An action-grammar problem, located by the action it occurs in.
type ActionError = String;

/// Split a `verb(arg1, arg2, …)` argument list on top-level commas.
///
/// Commas inside balanced delimiters or quoted strings are preserved.
fn split_action_args(input: &str) -> Result<Vec<String>, ActionError> {
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
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| "unbalanced action argument delimiters".to_string())?;
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
        return Err("unterminated action argument".to_string());
    }
    if !current.trim().is_empty() || input.ends_with(',') {
        args.push(current.trim().to_string());
    }
    Ok(args)
}

/// Parse an `action`/`actions` value authored at `field` (`loop.action`).
///
/// `[]` is the explicit empty list; an empty string or map is an error.
pub(super) fn parse_actions(
    field: &str,
    value: &serde_json::Value,
) -> Result<Vec<LoopAction>, LoopIssue> {
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let property = super::super::error::indexed_property(field, index);
                parse_action(item)
                    .map_err(|message| LoopIssue::new(&property, format!("`{property}`: {message}")))
            })
            .collect(),
        serde_json::Value::String(_) | serde_json::Value::Object(_) => parse_action(value)
            .map(|action| vec![action])
            .map_err(|message| LoopIssue::new(field, format!("`{field}`: {message}"))),
        other => Err(LoopIssue::new(
            field,
            format!(
                "`{field}` must be a string, object, or list of strings/objects, got {}",
                json_type_name(other)
            ),
        )),
    }
}

fn parse_action(value: &serde_json::Value) -> Result<LoopAction, ActionError> {
    match value {
        serde_json::Value::String(raw) => parse_dsl_action(raw),
        serde_json::Value::Object(map) => parse_structured_action(map),
        other => Err(format!("action must be a string or object, got {}", json_type_name(other))),
    }
}

fn parse_dsl_action(raw: &str) -> Result<LoopAction, ActionError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("an action must not be empty; use `[]` for no action".to_string());
    }
    let open = trimmed
        .find('(')
        .ok_or_else(|| format!("invalid action `{raw}`; expected op(...)"))?;
    let close = trimmed
        .rfind(')')
        .ok_or_else(|| format!("invalid action `{raw}`; missing closing `)`"))?;
    if close != trimmed.len() - 1 || open == 0 {
        return Err(format!("invalid action `{raw}`; expected op(...)"));
    }

    let op = trimmed[..open].trim();
    let args = split_action_args(&trimmed[open + 1..close])?;
    match op {
        "increment" => parse_unary_action(op, args, LoopAction::Increment),
        "decrement" => parse_unary_action(op, args, LoopAction::Decrement),
        "set" => parse_value_action(op, args, |prop, value| LoopAction::Set { prop, value }),
        "append" => parse_value_action(op, args, |prop, value| LoopAction::Append { prop, value }),
        "prepend" => parse_value_action(op, args, |prop, value| LoopAction::Prepend { prop, value }),
        "merge" => parse_value_action(op, args, |prop, value| LoopAction::Merge { prop, value }),
        other => Err(format!("unknown loop action op `{other}`")),
    }
}

fn parse_structured_action(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<LoopAction, ActionError> {
    let op = map
        .get("op")
        .ok_or_else(|| "action object is missing `op`".to_string())
        .and_then(|value| action_string("action.op", value))?;
    let prop = map
        .get("prop")
        .ok_or_else(|| "action object is missing `prop`".to_string())
        .and_then(|value| parse_property("action.prop", value))?;

    match op.as_str() {
        "increment" => Ok(LoopAction::Increment(prop)),
        "decrement" => Ok(LoopAction::Decrement(prop)),
        "set" => Ok(LoopAction::Set { prop, value: parse_structured_action_value(map)? }),
        "append" => Ok(LoopAction::Append { prop, value: parse_structured_action_value(map)? }),
        "prepend" => Ok(LoopAction::Prepend { prop, value: parse_structured_action_value(map)? }),
        "merge" => Ok(LoopAction::Merge { prop, value: parse_structured_action_value(map)? }),
        other => Err(format!("unknown loop action op `{other}`")),
    }
}

fn parse_structured_action_value(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, ActionError> {
    map.get("value")
        .cloned()
        .ok_or_else(|| "action object is missing `value`".to_string())
}

fn parse_unary_action(
    op: &str,
    args: Vec<String>,
    build: impl FnOnce(String) -> LoopAction,
) -> Result<LoopAction, ActionError> {
    if args.len() != 1 {
        return Err(format!("`{op}` expects 1 argument, got {}", args.len()));
    }
    parse_property(op, &serde_json::Value::String(args[0].clone())).map(build)
}

fn parse_value_action(
    op: &str,
    args: Vec<String>,
    build: impl FnOnce(String, serde_json::Value) -> LoopAction,
) -> Result<LoopAction, ActionError> {
    if args.len() != 2 {
        return Err(format!("`{op}` expects 2 arguments, got {}", args.len()));
    }
    let prop = parse_property(op, &serde_json::Value::String(args[0].clone()))?;
    Ok(build(prop, parse_dsl_value(&args[1])))
}

fn action_string(field: &str, value: &serde_json::Value) -> Result<String, ActionError> {
    match value {
        serde_json::Value::String(value) => Ok(value.clone()),
        other => Err(format!("`{field}` must be a string, got {}", json_type_name(other))),
    }
}

fn parse_property(field: &str, value: &serde_json::Value) -> Result<String, ActionError> {
    let prop = action_string(field, value)?;
    if prop.trim().is_empty() {
        return Err(format!("`{field}` must not be empty"));
    }
    Ok(prop.trim().to_string())
}

/// A `while`/`until` expression authored at `field`; it must be a non-empty
/// string.
pub(super) fn parse_condition_expression(
    field: &str,
    value: &serde_json::Value,
) -> Result<String, LoopIssue> {
    match value {
        serde_json::Value::String(source) if source.trim().is_empty() => {
            Err(LoopIssue::new(field, format!("`{field}` must not be empty")))
        }
        serde_json::Value::String(source) => Ok(source.clone()),
        other => Err(LoopIssue::new(
            field,
            format!("`{field}` must be a string, got {}", json_type_name(other)),
        )),
    }
}

/// A cap authored at `field`: a JSON integer of at least 1 that fits `usize`.
pub(super) fn parse_positive_usize(
    field: &str,
    value: &serde_json::Value,
) -> Result<usize, LoopIssue> {
    let Some(raw) = value.as_u64() else {
        return Err(LoopIssue::new(
            field,
            format!("`{field}` must be a positive integer, got {}", json_type_name(value)),
        ));
    };
    if raw == 0 {
        return Err(LoopIssue::new(field, format!("`{field}` must be greater than zero")));
    }
    usize::try_from(raw).map_err(|_| {
        LoopIssue::new(field, format!("`{field}` is too large for this platform"))
    })
}

fn parse_dsl_value(raw: &str) -> serde_json::Value {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return value;
    }
    serde_json::Value::String(
        trimmed.trim_matches(|ch| ch == '\'' || ch == '"').to_string(),
    )
}
