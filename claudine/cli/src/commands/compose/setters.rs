//! Positional-token and `--set` override parsing for composition commands.
//!
//! Inline `key=value` setters and `--set` JSON/JSON5 are merged into a single
//! override map; shorthand setters win on overlapping keys.

use color_eyre::eyre::{Result, eyre};

/// Parse `--set` JSON/JSON5, validate it's an object, return as `serde_json::Value`.
pub(crate) fn parse_set_json(raw: Option<&str>) -> Result<Option<serde_json::Value>> {
    let Some(json_str) = raw else {
        return Ok(None);
    };
    let parsed = biscuit_file::Json5::from_str(json_str)
        .map_err(|e| eyre!("Invalid JSON/JSON5 in --set argument: {e}"))?;
    let value = parsed.value().clone();
    if !value.is_object() {
        return Err(eyre!(
            "Invalid --set argument: expected a JSON object like {{\"name\":\"Alice\"}}"
        ));
    }
    Ok(Some(value))
}

/// Parse an inline shorthand setter value: JSON5 first, string fallback.
pub(crate) fn parse_shorthand_value(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::String(String::new());
    }
    match biscuit_file::Json5::from_str(raw) {
        Ok(parsed) => parsed.value().clone(),
        Err(_) => serde_json::Value::String(raw.to_string()),
    }
}

/// Classify a positional token as a shorthand setter.
///
/// ## Returns
/// - `None` — token is not a setter (pass through as file candidate)
/// - `Some(Err)` — setter syntax recognized but invalid (empty key)
/// - `Some(Ok((key, value)))` — valid setter
pub(crate) fn parse_compose_setter(
    token: &str,
) -> Option<std::result::Result<(String, serde_json::Value), String>> {
    let eq_pos = token.find('=')?;
    let key = &token[..eq_pos];
    let raw_value = &token[eq_pos + 1..];

    if key.is_empty() {
        return Some(Err("setter key must not be empty".to_string()));
    }

    let mut chars = key.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }

    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            continue;
        }
        return None;
    }

    let value = parse_shorthand_value(raw_value);
    Some(Ok((key.to_string(), value)))
}

/// Result of classifying composition positional tokens.
#[derive(Debug, Default)]
pub(crate) struct ParsedCompositionPositionals {
    pub file_ref: Option<String>,
    pub shorthand_setters: serde_json::Map<String, serde_json::Value>,
}

/// Classify positional tokens into an optional file reference and setter map.
///
/// ## Errors
/// - empty-key setter (`=foo`)
/// - multiple non-setter tokens (more than one file-ref candidate)
pub(crate) fn parse_composition_positionals(
    args: &[String],
) -> Result<ParsedCompositionPositionals> {
    let mut file_ref: Option<String> = None;
    let mut shorthand_setters = serde_json::Map::new();

    for token in args {
        match parse_compose_setter(token) {
            Some(Ok((key, value))) => {
                shorthand_setters.insert(key, value);
            }
            Some(Err(e)) => {
                return Err(eyre!("Invalid setter '{}': {}", token, e));
            }
            None => {
                if file_ref.is_some() {
                    let candidates: Vec<&str> = args
                        .iter()
                        .filter(|t| parse_compose_setter(t).is_none())
                        .map(|t| t.as_str())
                        .collect();
                    return Err(eyre!(
                        "expected at most one file reference, but got multiple: {}",
                        candidates.join(", ")
                    ));
                }
                file_ref = Some(token.clone());
            }
        }
    }

    Ok(ParsedCompositionPositionals {
        file_ref,
        shorthand_setters,
    })
}

/// Return a stable type name for a `serde_json::Value`.
///
/// Used by `inline-compose` (and the sequence orchestrator) to construct
/// [`CompositionError::PromptPropertyWrongType`] when the frontmatter
/// `prompt` value is present but not a string.
///
/// [`CompositionError::PromptPropertyWrongType`]:
///     claudine::composition::CompositionError::PromptPropertyWrongType
pub(crate) fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Merge `--set` JSON with shorthand setters. Shorthand wins on overlapping keys.
///
/// ## Returns
/// - `Ok(None)` when both sources are empty
/// - `Ok(Some(Value::Object(...)))` otherwise
pub(crate) fn merge_set_overrides(
    raw_set: Option<&str>,
    shorthand: serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Value>> {
    let base = parse_set_json(raw_set)?;
    let mut map = match base {
        Some(serde_json::Value::Object(m)) => m,
        Some(_) => unreachable!("parse_set_json enforces object shape"),
        None => serde_json::Map::new(),
    };
    for (key, value) in shorthand {
        map.insert(key, value);
    }
    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::Value::Object(map)))
    }
}
