//! Execution- and prompting-domain coercions: the agent-models
//! `model_selection[]` records projected to bare env-var identifiers and
//! the single launch-model CLI flag, plus the non-interactive flag list.
//! Compound / annotated / prose sites are skipped loudly.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::errors::GenError;
use crate::generate::CoercionSkip;
use crate::registry::RegistryEntry;

pub(super) fn env_var_sites_to_string_slice(
    entry: &RegistryEntry,
    raw: &Value,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: "expected an array of selection records".into(),
    })?;
    let mut seen = BTreeSet::new();
    let mut vars = Vec::new();
    let mut dropped = Vec::new();
    for record in records {
        if record.get("method").and_then(Value::as_str) != Some("env_var") {
            continue;
        }
        let Some(site) = record.get("site").and_then(Value::as_str) else {
            continue;
        };
        if !is_env_var_ident(site) {
            dropped.push(site.to_string());
        } else if seen.insert(site.to_string()) {
            vars.push(Value::String(site.to_string()));
        }
    }
    if !dropped.is_empty() {
        skips.push(CoercionSkip {
            field: entry.field,
            reason: "site is not a single env-var identifier",
            records: dropped,
        });
    }
    Ok(Value::Array(vars))
}

pub(super) fn cli_flag_sites_to_flag(
    entry: &RegistryEntry,
    raw: &Value,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: "expected an array of selection records".into(),
    })?;
    let mut flag = None;
    let mut dropped = Vec::new();
    for record in records {
        if record.get("method").and_then(Value::as_str) != Some("cli_flag") {
            continue;
        }
        let Some(site) = record.get("site").and_then(Value::as_str) else {
            continue;
        };
        if !is_bare_flag_token(site) {
            dropped.push(site.to_string());
        } else if flag.is_none() {
            flag = Some(site.to_string());
        }
    }
    if !dropped.is_empty() {
        skips.push(CoercionSkip {
            field: entry.field,
            reason: "site is not a single bare flag token",
            records: dropped,
        });
    }
    Ok(flag.map(Value::String).unwrap_or(Value::Null))
}

pub(super) fn flag_list_to_string_slice(
    entry: &RegistryEntry,
    raw: &Value,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    let items = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: "expected an array of flag entries".into(),
    })?;
    let mut flags = Vec::new();
    let mut dropped = Vec::new();
    for item in items {
        let text = item.as_str().ok_or_else(|| GenError::UnmappableValue {
            field: entry.field,
            message: format!("expected string elements, got `{item}`"),
        })?;
        if is_bare_flag_token(text) {
            flags.push(Value::String(text.to_string()));
        } else {
            dropped.push(text.to_string());
        }
    }
    if !dropped.is_empty() {
        skips.push(CoercionSkip {
            field: entry.field,
            reason: "entry is not a single bare flag token",
            records: dropped,
        });
    }
    Ok(Value::Array(flags))
}

/// A single bare env-var identifier: `[A-Z][A-Z0-9_]*`. Compound sites
/// ("A / B") and annotated sites are excluded by construction.
fn is_env_var_ident(site: &str) -> bool {
    let mut chars = site.chars();
    matches!(chars.next(), Some('A'..='Z'))
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// A single bare flag token: starts with `-` and contains no whitespace.
/// Compound sites ("--model / -m", "--model, -m"), annotated sites
/// ("--interactive when ..."), and env-var entries are excluded by
/// construction.
fn is_bare_flag_token(site: &str) -> bool {
    site.starts_with('-') && !site.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_ident_accepts_bare_identifiers_only() {
        assert!(is_env_var_ident("ANTHROPIC_MODEL"));
        assert!(is_env_var_ident("CLAUDE_CODE_SUBAGENT_MODEL"));
        assert!(!is_env_var_ident("A / B"));
        assert!(!is_env_var_ident("lowercase"));
        assert!(!is_env_var_ident(""));
        assert!(!is_env_var_ident("1ABC"));
    }

    #[test]
    fn bare_flag_token_accepts_flags_and_rejects_annotations() {
        assert!(is_bare_flag_token("--model"));
        assert!(is_bare_flag_token("-m"));
        assert!(is_bare_flag_token("--prompt-interactive"));
        assert!(!is_bare_flag_token("--model / -m"));
        assert!(!is_bare_flag_token("--model, -m"));
        assert!(!is_bare_flag_token("--model  (goose run)"));
        assert!(!is_bare_flag_token("--output-format json for live wrapping"));
        assert!(!is_bare_flag_token("GOOSE_MODE=approve"));
        assert!(!is_bare_flag_token("no --json for live parsing"));
        assert!(!is_bare_flag_token(""));
    }
}
