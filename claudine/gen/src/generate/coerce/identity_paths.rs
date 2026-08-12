//! Identity- and path-domain coercions: the research logging surfaces and
//! agent-cli config-path records projected to bare `PathTemplate::Static`
//! strings. Non-bare records (annotated pseudo-paths, prose, placeholders)
//! are skipped loudly.

use serde_json::Value;

use crate::errors::GenError;
use crate::generate::CoercionSkip;
use crate::registry::RegistryEntry;

pub(super) fn surfaces_to_session_log_paths(
    entry: &RegistryEntry,
    raw: &Value,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: "expected an array of logging-surface records".into(),
    })?;
    let mut paths = Vec::new();
    let mut dropped = Vec::new();
    for record in records {
        if record.get("role").and_then(Value::as_str) != Some("session_transcript") {
            continue;
        }
        let Some(path) = record.get("path_macos").and_then(Value::as_str) else {
            continue;
        };
        // " (" marks an annotated pseudo-path (e.g. a SQLite table
        // note), not a filesystem template.
        if path.contains(" (") {
            dropped.push(path.to_string());
        } else {
            paths.push(Value::String(path.to_string()));
        }
    }
    if !dropped.is_empty() {
        skips.push(CoercionSkip {
            field: entry.field,
            reason: "path_macos is an annotated pseudo-path, not a path template",
            records: dropped,
        });
    }
    Ok(Value::Array(paths))
}

pub(super) fn config_path_records_to_config_paths(
    entry: &RegistryEntry,
    raw: &Value,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    let records = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: "expected an array of config-path records".into(),
    })?;
    let mut paths = Vec::new();
    let mut dropped = Vec::new();
    for record in records {
        if record.get("os").and_then(Value::as_str) != Some("macos") {
            continue;
        }
        if !matches!(
            record.get("scope").and_then(Value::as_str),
            Some("user") | Some("repo")
        ) {
            continue;
        }
        let Some(path) = record.get("path").and_then(Value::as_str) else {
            continue;
        };
        if !is_bare_config_path(path) {
            dropped.push(path.to_string());
        } else {
            paths.push(Value::String(path.to_string()));
        }
    }
    if !dropped.is_empty() {
        skips.push(CoercionSkip {
            field: entry.field,
            reason: "path is not a bare filesystem template",
            records: dropped,
        });
    }
    Ok(Value::Array(paths))
}

/// A bare, usable config path: rejects prose annotations ("a; b",
/// "a, b", "a or b"), `<placeholder>` grammar, and env-var references
/// (`$VAR`) that no `PathContext` can resolve. Rejected records are
/// skipped loudly and, where they hollow out the projection, pinned back
/// via field-keyed overrides.
fn is_bare_config_path(path: &str) -> bool {
    !(path.contains(';')
        || path.contains(", ")
        || path.contains(" or ")
        || path.contains('<')
        || path.contains('$'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_config_path_rejects_annotations_and_placeholders() {
        assert!(is_bare_config_path("~/.claude/settings.json"));
        assert!(is_bare_config_path(".claude/settings.local.json"));
        assert!(is_bare_config_path("~/.qwen/debug/*.txt"));
        assert!(!is_bare_config_path(
            "$CODEX_HOME/config.toml; default /Users/<user>/.codex/config.toml"
        ));
        assert!(!is_bare_config_path("/Users/<name>/.kimi-code/config.toml"));
        assert!(!is_bare_config_path("AGENTS.override.md, AGENTS.md"));
        assert!(!is_bare_config_path("a.md or b.md"));
        assert!(!is_bare_config_path("$CODEX_HOME/auth.json"));
    }
}
