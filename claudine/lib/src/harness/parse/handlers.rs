//! Handler parsing: `handle`, `handle_*`, and action objects.

use std::path::Path;

use serde_json::Value;

use crate::harness::error::HarnessError;
use crate::harness::model::{
    ApprovedRuntimeCommand, FailureEvent, HandlerAction, HandlerRule, HandlerTable, ValidationEvent,
};
use crate::harness::resolve::HarnessResolutionContext;

use super::overlays::{normalize_handler_subject_key, parse_set_overlay, tokenize_to_approved_command};
use super::validations::validation_meta;

/// Parse all `handle` and `handle_*` keys from frontmatter.
pub(super) fn parse_handlers(
    obj: &serde_json::Map<String, Value>,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
) -> Result<(HandlerTable, Option<ApprovedRuntimeCommand>), HarnessError> {
    let mut table = HandlerTable::default();
    let mut programmatic = None;

    for (key, value) in obj {
        if key == "handle" {
            // Programmatic handler: store raw command, defer approval to Phase 7
            programmatic = Some(parse_programmatic_handler(value, source_path)?);
            continue;
        }

        if let Some(event_name) = key.strip_prefix("handle_") {
            let failure_event = parse_failure_event(event_name, source_path)?;
            parse_handler_entry(value, failure_event, source_path, ctx, &mut table)?;
        }
    }

    Ok((table, programmatic))
}

/// Parse the programmatic `handle` value into an `ApprovedRuntimeCommand`.
pub(super) fn parse_programmatic_handler(
    value: &Value,
    source_path: &Path,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    // Accept: { command: ["executable", "arg1", ...] } or { command: "executable arg1 ..." }
    let cmd_value = if let Some(obj) = value.as_object() {
        obj.get("command").unwrap_or(value)
    } else {
        value
    };

    match cmd_value {
        Value::Array(arr) => {
            if arr.is_empty() {
                return Err(HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: "handle".to_string(),
                    detail: "command array must not be empty".to_string(),
                });
            }
            let parts: Vec<String> = arr
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(String::from)
                        .unwrap_or_else(|| v.to_string())
                })
                .collect();
            Ok(ApprovedRuntimeCommand {
                raw: parts.join(" "),
                executable: parts[0].clone(),
                args: parts[1..].to_vec(),
            })
        }
        Value::String(s) => tokenize_to_approved_command(s, source_path),
        _ => Err(HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "handle".to_string(),
            detail: "must be a command string, array, or object with a `command` field".to_string(),
        }),
    }
}

/// Map a `handle_*` suffix to a [`FailureEvent`].
pub(super) fn parse_failure_event(name: &str, source_path: &Path) -> Result<FailureEvent, HarnessError> {
    match name {
        "agent_failure" => Ok(FailureEvent::AgentFailure),
        "timeout" => Ok(FailureEvent::Timeout),
        _ => {
            // Try as a validation event
            if validation_meta(name).is_some() {
                let event = match name {
                    "file_exists" => ValidationEvent::FileExists,
                    "dir_exists" => ValidationEvent::DirExists,
                    "json_file_exists" => ValidationEvent::JsonFileExists,
                    "yaml_file_exists" => ValidationEvent::YamlFileExists,
                    "toml_file_exists" => ValidationEvent::TomlFileExists,
                    "has_write_permission" => ValidationEvent::HasWritePermission,
                    "shell_command" => ValidationEvent::ShellCommand,
                    "no_dirty_source_code" => ValidationEvent::NoDirtySourceCode,
                    "has_dirty_source_code" => ValidationEvent::HasDirtySourceCode,
                    "file_changed" => ValidationEvent::FileChanged,
                    "file_unchanged" => ValidationEvent::FileUnchanged,
                    "frontmatter_prop_changed" => ValidationEvent::FrontmatterPropChanged,
                    "frontmatter_prop_unchanged" => ValidationEvent::FrontmatterPropUnchanged,
                    "frontmatter_prop_equals" => ValidationEvent::FrontmatterPropEquals,
                    "response_length_at_least" => ValidationEvent::ResponseLengthAtLeast,
                    "response_length_at_most" => ValidationEvent::ResponseLengthAtMost,
                    "response_includes" => ValidationEvent::ResponseIncludes,
                    "response_missing" => ValidationEvent::ResponseMissing,
                    "inline_response_empty" => ValidationEvent::InlineResponseEmpty,
                    "inline_body_unchanged" => ValidationEvent::InlineBodyUnchanged,
                    _ => unreachable!(),
                };
                Ok(FailureEvent::Validation(event))
            } else {
                Err(HarnessError::UnknownValidation {
                    source_path: source_path.to_path_buf(),
                    name: format!("handle_{name}"),
                })
            }
        }
    }
}

/// Parse a `handle_{event}` value which may be:
/// - A direct handler action object (generic handler)
/// - A mapping of subject keys to handler action objects (subject-specific)
pub(super) fn parse_handler_entry(
    value: &Value,
    event: FailureEvent,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
    table: &mut HandlerTable,
) -> Result<(), HarnessError> {
    let obj = value
        .as_object()
        .ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: format!("handle_{event}"),
            detail: "must be an object".to_string(),
        })?;

    // Detect whether this is a direct handler or subject-keyed mapping.
    // A direct handler has action keys like "retry", "resume", "redirect", "deviate".
    let is_direct = obj
        .keys()
        .any(|k| matches!(k.as_str(), "retry" | "resume" | "redirect" | "deviate"));

    if is_direct {
        // Generic handler
        let action = parse_handler_action(obj, source_path, &format!("handle_{event}"))?;
        table.generic.push(HandlerRule {
            event,
            subject_key: None,
            action,
        });
    } else {
        // Subject-specific handlers
        for (subject_key, subject_value) in obj {
            let subject_obj =
                subject_value
                    .as_object()
                    .ok_or_else(|| HarnessError::InvalidFrontmatter {
                        source_path: source_path.to_path_buf(),
                        property: format!("handle_{event}.{subject_key}"),
                        detail: "must be an object containing a handler action".to_string(),
                    })?;
            let action = parse_handler_action(
                subject_obj,
                source_path,
                &format!("handle_{event}.{subject_key}"),
            )?;
            // Normalize subject key through the same path resolver used for
            // validation subjects so that handler matching works correctly
            // regardless of whether the author used @-prefixed, relative, or
            // absolute paths.
            let canonical_key = normalize_handler_subject_key(subject_key, &event, ctx);
            table.exact.push(HandlerRule {
                event: event.clone(),
                subject_key: Some(canonical_key),
                action,
            });
        }
    }

    Ok(())
}

/// Parse a handler action object into a [`HandlerAction`].
pub(super) fn parse_handler_action(
    obj: &serde_json::Map<String, Value>,
    source_path: &Path,
    property: &str,
) -> Result<HandlerAction, HarnessError> {
    if let Some(v) = obj.get("retry") {
        let inner = v.as_object().cloned().unwrap_or_default();
        return Ok(HandlerAction::Retry {
            prompt_suffix: inner
                .get("prompt")
                .and_then(|v| v.as_str())
                .map(String::from),
            set: parse_set_overlay(&inner),
            msg: inner.get("msg").and_then(|v| v.as_str()).map(String::from),
            say: inner.get("say").and_then(|v| v.as_str()).map(String::from),
            retries: inner
                .get("retries")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32),
        });
    }

    if let Some(v) = obj.get("resume") {
        let inner = v.as_object().cloned().unwrap_or_default();
        let prompt = inner
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| HarnessError::MissingHandlerField {
                source_path: source_path.to_path_buf(),
                handler: "resume".to_string(),
                field: "prompt".to_string(),
            })?;
        return Ok(HandlerAction::Resume {
            prompt,
            set: parse_set_overlay(&inner),
            msg: inner.get("msg").and_then(|v| v.as_str()).map(String::from),
            say: inner.get("say").and_then(|v| v.as_str()).map(String::from),
            retries: inner
                .get("retries")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32),
        });
    }

    if let Some(v) = obj.get("redirect") {
        let inner = v.as_object().cloned().unwrap_or_default();
        let file = inner
            .get("file")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| HarnessError::MissingHandlerField {
                source_path: source_path.to_path_buf(),
                handler: "redirect".to_string(),
                field: "file".to_string(),
            })?;
        return Ok(HandlerAction::Redirect {
            file,
            set: parse_set_overlay(&inner),
            msg: inner.get("msg").and_then(|v| v.as_str()).map(String::from),
            say: inner.get("say").and_then(|v| v.as_str()).map(String::from),
            resume: inner
                .get("resume")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        });
    }

    if let Some(v) = obj.get("deviate") {
        let inner = v.as_object().cloned().unwrap_or_default();
        let cmd_str = inner
            .get("cmd")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| HarnessError::MissingHandlerField {
                source_path: source_path.to_path_buf(),
                handler: "deviate".to_string(),
                field: "cmd".to_string(),
            })?;
        let command = tokenize_to_approved_command(&cmd_str, source_path)?;
        return Ok(HandlerAction::Deviate {
            command,
            set: parse_set_overlay(&inner),
            msg: inner.get("msg").and_then(|v| v.as_str()).map(String::from),
            say: inner.get("say").and_then(|v| v.as_str()).map(String::from),
        });
    }

    Err(HarnessError::InvalidFrontmatter {
        source_path: source_path.to_path_buf(),
        property: property.to_string(),
        detail: "must contain one of: retry, resume, redirect, deviate".to_string(),
    })
}
