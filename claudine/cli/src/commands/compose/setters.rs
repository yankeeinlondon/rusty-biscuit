//! Positional-token and `--set` override parsing for composition commands.
//!
//! Inline `key=value` setters and `--set` JSON/JSON5 are merged into a single
//! override map; shorthand setters win on overlapping keys. Bare words after
//! the file become the `argv` string array, which neither source may set by
//! name.

use claudine::composition::{ARGV_KEY, OwnershipError};
use color_eyre::eyre::{Result, eyre};

/// Refuse `argv` as a named parameter, from a setter or a `--set` key.
fn reject_reserved_key(key: &str) -> Result<()> {
    if key == ARGV_KEY {
        return Err(color_eyre::eyre::Report::new(OwnershipError::ReservedArgv));
    }
    Ok(())
}

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
    /// Bare words after the file, in order: the `argv` array.
    pub positionals: Vec<String>,
}

/// Classify positional tokens into an optional file reference, a setter map,
/// and the positionals that follow the file.
///
/// The first non-setter token is the file; every later one is a positional.
///
/// ## Errors
/// - empty-key setter (`=foo`)
/// - an `argv=` setter (`argv` holds positionals only)
pub(crate) fn parse_composition_positionals(
    args: &[String],
) -> Result<ParsedCompositionPositionals> {
    let mut parsed = ParsedCompositionPositionals::default();

    for token in args {
        match parse_compose_setter(token) {
            Some(Ok((key, value))) => {
                reject_reserved_key(&key)?;
                parsed.shorthand_setters.insert(key, value);
            }
            Some(Err(e)) => {
                return Err(eyre!("Invalid setter '{}': {}", token, e));
            }
            None if parsed.file_ref.is_none() => parsed.file_ref = Some(token.clone()),
            None => parsed.positionals.push(token.clone()),
        }
    }

    Ok(parsed)
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

/// Merge `--set` JSON with shorthand setters and the positionals. Shorthand
/// wins on overlapping keys; `argv` is set only from positionals, and only
/// when there is at least one.
///
/// ## Returns
/// - `Ok(None)` when every source is empty
/// - `Ok(Some(Value::Object(...)))` otherwise
///
/// ## Errors
/// Invalid `--set` JSON, or a `--set` object holding `argv`.
pub(crate) fn merge_set_overrides(
    raw_set: Option<&str>,
    shorthand: serde_json::Map<String, serde_json::Value>,
    positionals: Vec<String>,
) -> Result<Option<serde_json::Value>> {
    let base = parse_set_json(raw_set)?;
    let mut map = match base {
        Some(serde_json::Value::Object(m)) => m,
        Some(_) => unreachable!("parse_set_json enforces object shape"),
        None => serde_json::Map::new(),
    };
    for key in map.keys().chain(shorthand.keys()) {
        reject_reserved_key(key)?;
    }
    for (key, value) in shorthand {
        map.insert(key, value);
    }
    if !positionals.is_empty() {
        let values = positionals.into_iter().map(serde_json::Value::String).collect();
        map.insert(ARGV_KEY.to_string(), serde_json::Value::Array(values));
    }
    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::Value::Object(map)))
    }
}
