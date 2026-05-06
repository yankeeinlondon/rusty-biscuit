//! Handler overlay parsing and shell command tokenization.

use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

use crate::harness::error::HarnessError;
use crate::harness::model::{ApprovedRuntimeCommand, FailureEvent, ValidationEvent};
use crate::harness::resolve::{HarnessResolutionContext, resolve_harness_path};

/// Parse an optional `set` overlay from a handler object.
pub(super) fn parse_set_overlay(
    obj: &serde_json::Map<String, Value>,
) -> Option<IndexMap<String, Value>> {
    obj.get("set")
        .and_then(|v| v.as_object())
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

/// Normalize a handler subject key using the same path resolution as
/// validation subjects. For path-based events, resolve through
/// `resolve_harness_path`; for non-path events, return as-is.
pub(super) fn normalize_handler_subject_key(
    raw: &str,
    event: &FailureEvent,
    ctx: &HarnessResolutionContext<'_>,
) -> String {
    let is_path_event = matches!(
        event,
        FailureEvent::Validation(
            ValidationEvent::FileExists
                | ValidationEvent::DirExists
                | ValidationEvent::JsonFileExists
                | ValidationEvent::YamlFileExists
                | ValidationEvent::TomlFileExists
                | ValidationEvent::HasWritePermission
                | ValidationEvent::FileChanged
                | ValidationEvent::FileUnchanged
                | ValidationEvent::NoDirtySourceCode
                | ValidationEvent::HasDirtySourceCode
        )
    );
    if is_path_event {
        resolve_harness_path(raw, ctx)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| raw.to_string())
    } else {
        raw.to_string()
    }
}

/// Tokenize a raw command string into an `ApprovedRuntimeCommand`.
///
/// Uses Darkmatter's shell tokenizer which rejects shell metacharacters,
/// unterminated quotes, and empty input.
pub(super) fn tokenize_to_approved_command(
    raw: &str,
    source_path: &Path,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    let tokens =
        darkmatter::markdown::compose::shell_expansion::tokenize::tokenize(raw).map_err(|_e| {
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
