use super::super::error::CompositionError;
use super::super::json_util::json_type_name;
use super::super::types::LoopAction;

/// Split a `verb(arg1, arg2, …)` argument list on top-level commas.
///
/// Commas inside balanced delimiters or quoted strings are preserved.
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

pub(super) fn parse_actions(value: &serde_json::Value) -> Result<Vec<LoopAction>, CompositionError> {
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| parse_action(item).map_err(|err| annotate_action_error(err, index)))
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
            let property = super::super::error::indexed_property("loop.actions", index);
            CompositionError::LoopInvalid(format!("`{property}`: {message}"))
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
        "prepend" => parse_value_action(op, args, |prop, value| LoopAction::Prepend { prop, value }),
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
        "set" => Ok(LoopAction::Set { prop, value: parse_structured_action_value(map)? }),
        "append" => Ok(LoopAction::Append { prop, value: parse_structured_action_value(map)? }),
        "prepend" => Ok(LoopAction::Prepend { prop, value: parse_structured_action_value(map)? }),
        "merge" => Ok(LoopAction::Merge { prop, value: parse_structured_action_value(map)? }),
        other => Err(CompositionError::LoopInvalid(format!("unknown loop action op `{other}`"))),
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
    Ok(build(prop, parse_dsl_value(&args[1])))
}

pub(super) fn parse_string(
    field: &str,
    value: &serde_json::Value,
) -> Result<String, CompositionError> {
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
        return Err(CompositionError::LoopInvalid(format!("`{field}` must not be empty")));
    }
    Ok(prop.trim().to_string())
}

pub(super) fn parse_positive_usize(
    field: &str,
    value: &serde_json::Value,
) -> Result<usize, CompositionError> {
    let Some(raw) = value.as_u64() else {
        return Err(CompositionError::LoopInvalid(format!(
            "`{field}` must be a positive integer, got {}",
            json_type_name(value)
        )));
    };
    if raw == 0 {
        return Err(CompositionError::LoopInvalid(format!("`{field}` must be greater than zero")));
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
        trimmed.trim_matches(|ch| ch == '\'' || ch == '"').to_string(),
    )
}
