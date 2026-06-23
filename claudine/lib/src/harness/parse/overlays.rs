//! Handler overlay parsing and shell command tokenization.

use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

use crate::harness::error::HarnessError;
use crate::harness::model::{ApprovedRuntimeCommand, FailureEvent};

/// Parse an optional `set` overlay from a handler object.
pub(super) fn parse_set_overlay(
    obj: &serde_json::Map<String, Value>,
) -> Option<IndexMap<String, Value>> {
    obj.get("set")
        .and_then(|v| v.as_object())
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

/// Normalize a handler subject key.
///
/// Validation path events have been removed, so subject keys are kept as
/// authored for the remaining handler events.
pub(super) fn normalize_handler_subject_key(raw: &str, _event: &FailureEvent) -> String {
    raw.to_string()
}

/// Tokenize a raw command string into an `ApprovedRuntimeCommand`.
///
/// Uses Darkmatter's shell tokenizer which rejects shell metacharacters,
/// unterminated quotes, and empty input.
pub(super) fn tokenize_to_approved_command(
    raw: &str,
    source_path: &Path,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    let tokens = crate::harness::shell::tokenize_words_strict(raw).map_err(|_e| {
        HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "shell_command".to_string(),
            detail: format!("invalid command string: \"{raw}\""),
        }
    })?;

    if tokens.is_empty() {
        return Err(HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "shell_command".to_string(),
            detail: "command string is empty".to_string(),
        });
    }

    Ok(ApprovedRuntimeCommand {
        raw: raw.to_string(),
        executable: tokens[0].clone(),
        args: tokens[1..].to_vec(),
    })
}
