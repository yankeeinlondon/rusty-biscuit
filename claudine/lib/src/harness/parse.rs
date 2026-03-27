//! Frontmatter-to-plan parser for harness configuration.
//!
//! Accepts `pre_checks` and `post_checks` in both list form (canonical) and
//! map form (shorthand), parses handler declarations, and builds a typed
//! [`HarnessPlan`].

use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

use crate::harness::error::HarnessError;
use crate::harness::model::{
    ApprovedRuntimeCommand, FailureEvent, HandlerAction, HandlerRule, HandlerTable, HarnessPlan,
    StructuredShape, ValidationEvent, ValidationKind, ValidationPhase, ValidationRule,
    ValidationRuleId,
};
use crate::harness::resolve::{resolve_harness_path, HarnessResolutionContext};
use crate::harness::timeout::parse_timeout;

/// Harness-relevant frontmatter keys.
const HARNESS_KEYS: &[&str] = &[
    "pre_checks",
    "post_checks",
    "timeout",
    "handle",
];

/// Check whether composed frontmatter contains any harness-relevant keys.
///
/// Returns `true` if any of `pre_checks`, `post_checks`, `timeout`, `handle`,
/// or any `handle_*` key is present.
pub fn has_harness_properties(frontmatter: &Value) -> bool {
    let Some(obj) = frontmatter.as_object() else {
        return false;
    };
    for key in obj.keys() {
        if HARNESS_KEYS.contains(&key.as_str()) || key.starts_with("handle_") {
            return true;
        }
    }
    false
}

/// Parse composed frontmatter into a [`HarnessPlan`].
///
/// ## Errors
///
/// Returns [`HarnessError`] for any structural or semantic issue found at parse
/// time, including unknown validation names, post-only validations in
/// `pre_checks`, invalid timeout strings, and missing handler fields.
pub fn parse_harness_plan(
    frontmatter: &Value,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
) -> Result<HarnessPlan, HarnessError> {
    let obj = frontmatter.as_object().ok_or_else(|| {
        HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "(root)".to_string(),
            detail: "frontmatter must be an object".to_string(),
        }
    })?;

    let mut next_id: u32 = 0;
    let mut alloc_id = || {
        let id = ValidationRuleId(next_id);
        next_id += 1;
        id
    };

    // Parse pre_checks
    let pre_checks = if let Some(v) = obj.get("pre_checks") {
        parse_checks(v, true, source_path, ctx, &mut alloc_id)?
    } else {
        Vec::new()
    };

    // Parse post_checks
    let post_checks = if let Some(v) = obj.get("post_checks") {
        parse_checks(v, false, source_path, ctx, &mut alloc_id)?
    } else {
        Vec::new()
    };

    // Parse timeout
    let timeout = if let Some(v) = obj.get("timeout") {
        let raw = v.as_str().ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "timeout".to_string(),
            detail: "timeout must be a string (e.g. \"30s\", \"5m\")".to_string(),
        })?;
        Some(parse_timeout(raw, source_path)?)
    } else {
        None
    };

    // Parse handlers
    let (handlers, programmatic_handler) = parse_handlers(obj, source_path, ctx)?;

    Ok(HarnessPlan {
        source_path: source_path.to_path_buf(),
        timeout,
        pre_checks,
        post_checks,
        handlers,
        programmatic_handler,
    })
}

/// Parse a `pre_checks` or `post_checks` value in list or map form.
fn parse_checks(
    value: &Value,
    is_pre: bool,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
    alloc_id: &mut impl FnMut() -> ValidationRuleId,
) -> Result<Vec<ValidationRule>, HarnessError> {
    let property = if is_pre { "pre_checks" } else { "post_checks" };

    match value {
        Value::Array(arr) => {
            // List form: each element is a single-key object
            let mut rules = Vec::with_capacity(arr.len());
            for item in arr {
                let obj = item.as_object().ok_or_else(|| {
                    HarnessError::InvalidFrontmatter {
                        source_path: source_path.to_path_buf(),
                        property: property.to_string(),
                        detail: "each item in the list must be an object".to_string(),
                    }
                })?;
                for (name, val) in obj {
                    let rule = parse_single_validation(
                        name, val, is_pre, source_path, ctx, alloc_id,
                    )?;
                    rules.push(rule);
                }
            }
            Ok(rules)
        }
        Value::Object(obj) => {
            // Map form: shorthand
            let mut rules = Vec::with_capacity(obj.len());
            for (name, val) in obj {
                let rule =
                    parse_single_validation(name, val, is_pre, source_path, ctx, alloc_id)?;
                rules.push(rule);
            }
            Ok(rules)
        }
        _ => Err(HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            detail: "must be a list or a mapping".to_string(),
        }),
    }
}

/// Metadata for a validation kind: its event, allowed phase, and name.
struct ValidationMeta {
    event: ValidationEvent,
    phase: ValidationPhase,
}

/// Look up validation metadata by name string.
fn validation_meta(name: &str) -> Option<ValidationMeta> {
    let meta = match name {
        "file_exists" => ValidationMeta { event: ValidationEvent::FileExists, phase: ValidationPhase::Both },
        "dir_exists" => ValidationMeta { event: ValidationEvent::DirExists, phase: ValidationPhase::Both },
        "json_file_exists" => ValidationMeta { event: ValidationEvent::JsonFileExists, phase: ValidationPhase::Both },
        "yaml_file_exists" => ValidationMeta { event: ValidationEvent::YamlFileExists, phase: ValidationPhase::Both },
        "toml_file_exists" => ValidationMeta { event: ValidationEvent::TomlFileExists, phase: ValidationPhase::Both },
        "has_write_permission" => ValidationMeta { event: ValidationEvent::HasWritePermission, phase: ValidationPhase::Both },
        "shell_command" => ValidationMeta { event: ValidationEvent::ShellCommand, phase: ValidationPhase::Both },
        "no_dirty_source_code" => ValidationMeta { event: ValidationEvent::NoDirtySourceCode, phase: ValidationPhase::Both },
        "has_dirty_source_code" => ValidationMeta { event: ValidationEvent::HasDirtySourceCode, phase: ValidationPhase::Both },
        "file_changed" => ValidationMeta { event: ValidationEvent::FileChanged, phase: ValidationPhase::PostOnly },
        "file_unchanged" => ValidationMeta { event: ValidationEvent::FileUnchanged, phase: ValidationPhase::PostOnly },
        "frontmatter_prop_changed" => ValidationMeta { event: ValidationEvent::FrontmatterPropChanged, phase: ValidationPhase::PostOnly },
        "frontmatter_prop_unchanged" => ValidationMeta { event: ValidationEvent::FrontmatterPropUnchanged, phase: ValidationPhase::PostOnly },
        "frontmatter_prop_equals" => ValidationMeta { event: ValidationEvent::FrontmatterPropEquals, phase: ValidationPhase::PostOnly },
        "response_length_at_least" => ValidationMeta { event: ValidationEvent::ResponseLengthAtLeast, phase: ValidationPhase::PostOnly },
        "response_length_at_most" => ValidationMeta { event: ValidationEvent::ResponseLengthAtMost, phase: ValidationPhase::PostOnly },
        "response_includes" => ValidationMeta { event: ValidationEvent::ResponseIncludes, phase: ValidationPhase::PostOnly },
        "response_missing" => ValidationMeta { event: ValidationEvent::ResponseMissing, phase: ValidationPhase::PostOnly },
        _ => return None,
    };
    Some(meta)
}

/// Parse a single validation from its name and YAML value.
fn parse_single_validation(
    name: &str,
    value: &Value,
    is_pre: bool,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
    alloc_id: &mut impl FnMut() -> ValidationRuleId,
) -> Result<ValidationRule, HarnessError> {
    let meta = validation_meta(name).ok_or_else(|| HarnessError::UnknownValidation {
        source_path: source_path.to_path_buf(),
        name: name.to_string(),
    })?;

    // Enforce phase constraint
    if is_pre && meta.phase == ValidationPhase::PostOnly {
        return Err(HarnessError::PostOnlyInPreChecks {
            source_path: source_path.to_path_buf(),
            name: name.to_string(),
        });
    }

    // Extract optional msg from expanded form
    let msg = value
        .as_object()
        .and_then(|o| o.get("msg"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let (kind, subject_key) = parse_validation_kind(name, value, source_path, ctx)?;

    Ok(ValidationRule {
        id: alloc_id(),
        event: meta.event,
        phase: meta.phase,
        kind,
        message_template: msg,
        subject_key,
    })
}

/// Parse a validation's kind-specific parameters from its value.
///
/// Returns `(ValidationKind, Option<subject_key>)`.
fn parse_validation_kind(
    name: &str,
    value: &Value,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
) -> Result<(ValidationKind, Option<String>), HarnessError> {
    match name {
        "file_exists" => {
            let file_ref = extract_file_ref(value, "file")?;
            let path = resolve_harness_path(&file_ref, ctx)?;
            Ok((
                ValidationKind::FileExists { file: path.clone() },
                Some(path.display().to_string()),
            ))
        }
        "dir_exists" => {
            let dir_ref = extract_string_field(value, "dir")
                .or_else(|| extract_scalar_string(value))
                .ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a directory path".to_string(),
                })?;
            let path = resolve_harness_path(&dir_ref, ctx)?;
            Ok((
                ValidationKind::DirExists { dir: path.clone() },
                Some(path.display().to_string()),
            ))
        }
        "json_file_exists" => {
            let file_ref = extract_file_ref(value, "file")?;
            let path = resolve_harness_path(&file_ref, ctx)?;
            let shape = extract_shape(value, source_path)?;
            Ok((
                ValidationKind::JsonFileExists { file: path.clone(), shape },
                Some(path.display().to_string()),
            ))
        }
        "yaml_file_exists" => {
            let file_ref = extract_file_ref(value, "file")?;
            let path = resolve_harness_path(&file_ref, ctx)?;
            let shape = extract_shape(value, source_path)?;
            Ok((
                ValidationKind::YamlFileExists { file: path.clone(), shape },
                Some(path.display().to_string()),
            ))
        }
        "toml_file_exists" => {
            let file_ref = extract_file_ref(value, "file")?;
            let path = resolve_harness_path(&file_ref, ctx)?;
            Ok((
                ValidationKind::TomlFileExists { file: path.clone() },
                Some(path.display().to_string()),
            ))
        }
        "has_write_permission" => {
            let file_ref = extract_file_ref(value, "file")?;
            let path = resolve_harness_path(&file_ref, ctx)?;
            Ok((
                ValidationKind::HasWritePermission { file: path.clone() },
                Some(path.display().to_string()),
            ))
        }
        "shell_command" => {
            let raw = extract_scalar_string(value).ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a command string".to_string(),
                }
            })?;
            let show_stdout = extract_bool_field(value, "show_stdout").unwrap_or(true);
            let show_stderr = extract_bool_field(value, "show_stderr").unwrap_or(true);
            let command = tokenize_to_approved_command(&raw, source_path)?;
            Ok((
                ValidationKind::ShellCommand { command, show_stdout, show_stderr },
                Some(raw),
            ))
        }
        "no_dirty_source_code" => {
            let root_ref = extract_scalar_string(value).unwrap_or_else(|| ".".to_string());
            let path = resolve_harness_path(&root_ref, ctx)?;
            Ok((
                ValidationKind::NoDirtySourceCode { root: path.clone() },
                Some(path.display().to_string()),
            ))
        }
        "has_dirty_source_code" => {
            let root_ref = extract_scalar_string(value).unwrap_or_else(|| ".".to_string());
            let path = resolve_harness_path(&root_ref, ctx)?;
            Ok((
                ValidationKind::HasDirtySourceCode { root: path.clone() },
                Some(path.display().to_string()),
            ))
        }
        "file_changed" => {
            let file_ref = extract_file_ref(value, "file")?;
            let path = resolve_harness_path(&file_ref, ctx)?;
            Ok((
                ValidationKind::FileChanged { file: path.clone() },
                Some(path.display().to_string()),
            ))
        }
        "file_unchanged" => {
            let file_ref = extract_file_ref(value, "file")?;
            let path = resolve_harness_path(&file_ref, ctx)?;
            Ok((
                ValidationKind::FileUnchanged { file: path.clone() },
                Some(path.display().to_string()),
            ))
        }
        "frontmatter_prop_changed" => {
            let prop = extract_scalar_string(value).ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a property name string".to_string(),
                }
            })?;
            Ok((
                ValidationKind::FrontmatterPropChanged { prop: prop.clone() },
                Some(prop),
            ))
        }
        "frontmatter_prop_unchanged" => {
            let prop = extract_scalar_string(value).ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a property name string".to_string(),
                }
            })?;
            Ok((
                ValidationKind::FrontmatterPropUnchanged { prop: prop.clone() },
                Some(prop),
            ))
        }
        "frontmatter_prop_equals" => {
            let obj = value.as_object().ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a mapping of property names to expected values".to_string(),
                }
            })?;
            let mut expected = IndexMap::new();
            for (k, v) in obj {
                if k == "msg" {
                    continue; // skip the message template
                }
                expected.insert(k.clone(), v.clone());
            }
            Ok((ValidationKind::FrontmatterPropEquals { expected }, None))
        }
        "response_length_at_least" => {
            let length = extract_usize(value, name, source_path)?;
            Ok((ValidationKind::ResponseLengthAtLeast { length }, None))
        }
        "response_length_at_most" => {
            let length = extract_usize(value, name, source_path)?;
            Ok((ValidationKind::ResponseLengthAtMost { length }, None))
        }
        "response_includes" => {
            let needle = extract_scalar_string(value).ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a search string".to_string(),
                }
            })?;
            Ok((
                ValidationKind::ResponseIncludes { needle: needle.clone() },
                Some(needle),
            ))
        }
        "response_missing" => {
            let needle = extract_scalar_string(value).ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a search string".to_string(),
                }
            })?;
            Ok((
                ValidationKind::ResponseMissing { needle: needle.clone() },
                Some(needle),
            ))
        }
        _ => unreachable!("unknown validation name should have been caught earlier"),
    }
}

// ---------------------------------------------------------------------------
// Handler parsing
// ---------------------------------------------------------------------------

/// Parse all `handle` and `handle_*` keys from frontmatter.
fn parse_handlers(
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
fn parse_programmatic_handler(
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
fn parse_failure_event(
    name: &str,
    source_path: &Path,
) -> Result<FailureEvent, HarnessError> {
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
fn parse_handler_entry(
    value: &Value,
    event: FailureEvent,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
    table: &mut HandlerTable,
) -> Result<(), HarnessError> {
    let obj = value.as_object().ok_or_else(|| HarnessError::InvalidFrontmatter {
        source_path: source_path.to_path_buf(),
        property: format!("handle_{event}"),
        detail: "must be an object".to_string(),
    })?;

    // Detect whether this is a direct handler or subject-keyed mapping.
    // A direct handler has action keys like "retry", "resume", "redirect", "deviate".
    let is_direct = obj.keys().any(|k| {
        matches!(k.as_str(), "retry" | "resume" | "redirect" | "deviate")
    });

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
            let subject_obj = subject_value.as_object().ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: format!("handle_{event}.{subject_key}"),
                    detail: "must be an object containing a handler action".to_string(),
                }
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

/// Normalize a handler subject key using the same path resolution as
/// validation subjects. For path-based events, resolve through
/// `resolve_harness_path`; for non-path events, return as-is.
fn normalize_handler_subject_key(
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

/// Parse a handler action object into a [`HandlerAction`].
fn parse_handler_action(
    obj: &serde_json::Map<String, Value>,
    source_path: &Path,
    property: &str,
) -> Result<HandlerAction, HarnessError> {
    if let Some(v) = obj.get("retry") {
        let inner = v.as_object().cloned().unwrap_or_default();
        return Ok(HandlerAction::Retry {
            prompt_suffix: inner.get("prompt").and_then(|v| v.as_str()).map(String::from),
            set: parse_set_overlay(&inner),
            msg: inner.get("msg").and_then(|v| v.as_str()).map(String::from),
            say: inner.get("say").and_then(|v| v.as_str()).map(String::from),
            retries: inner.get("retries").and_then(|v| v.as_u64()).map(|n| n as u32),
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
            retries: inner.get("retries").and_then(|v| v.as_u64()).map(|n| n as u32),
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
            resume: inner.get("resume").and_then(|v| v.as_bool()).unwrap_or(false),
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

/// Parse an optional `set` overlay from a handler object.
fn parse_set_overlay(obj: &serde_json::Map<String, Value>) -> Option<IndexMap<String, Value>> {
    obj.get("set").and_then(|v| v.as_object()).map(|m| {
        m.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    })
}

// ---------------------------------------------------------------------------
// Shell command tokenization
// ---------------------------------------------------------------------------

/// Tokenize a raw command string into an `ApprovedRuntimeCommand`.
///
/// Uses Darkmatter's shell tokenizer which rejects shell metacharacters,
/// unterminated quotes, and empty input.
fn tokenize_to_approved_command(
    raw: &str,
    source_path: &Path,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    let tokens = darkmatter::markdown::compose::shell_expansion::tokenize::tokenize(raw)
        .map_err(|_e| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "shell_command".to_string(),
            detail: format!("invalid command string: \"{raw}\""),
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

// ---------------------------------------------------------------------------
// Value extraction helpers
// ---------------------------------------------------------------------------

/// Extract a file reference from a value — either a scalar string or an object
/// with a `file` field (or the named field).
fn extract_file_ref(value: &Value, field: &str) -> Result<String, HarnessError> {
    if let Some(s) = extract_scalar_string(value) {
        return Ok(s);
    }
    if let Some(s) = extract_string_field(value, field) {
        return Ok(s);
    }
    // For simple scalars, the value itself might be a string stored in the field
    Err(HarnessError::PathResolutionFailed {
        raw: value.to_string(),
        detail: "expected a file path string or an object with a `file` field".to_string(),
    })
}

/// Extract a string from a scalar JSON value.
fn extract_scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Extract a string field from an object value.
fn extract_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .as_object()
        .and_then(|o| o.get(field))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Extract a boolean field from an object value.
fn extract_bool_field(value: &Value, field: &str) -> Option<bool> {
    value.as_object().and_then(|o| o.get(field)).and_then(|v| v.as_bool())
}

/// Extract an optional `shape` field and parse it.
fn extract_shape(
    value: &Value,
    source_path: &Path,
) -> Result<Option<StructuredShape>, HarnessError> {
    let Some(shape_str) = extract_string_field(value, "shape") else {
        return Ok(None);
    };
    shape_str
        .parse::<StructuredShape>()
        .map(Some)
        .map_err(|_| HarnessError::InvalidShape {
            source_path: source_path.to_path_buf(),
            raw: shape_str,
        })
}

/// Extract a `usize` from a scalar value.
fn extract_usize(
    value: &Value,
    name: &str,
    source_path: &Path,
) -> Result<usize, HarnessError> {
    match value {
        Value::Number(n) => n.as_u64().map(|v| v as usize).ok_or_else(|| {
            HarnessError::InvalidFrontmatter {
                source_path: source_path.to_path_buf(),
                property: name.to_string(),
                detail: "requires a positive integer".to_string(),
            }
        }),
        Value::Object(obj) => {
            // Expanded form: look for a primary field
            let length_val = obj.get("length").or_else(|| obj.get("value")).ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a number or an object with a `length` field".to_string(),
                }
            })?;
            length_val.as_u64().map(|v| v as usize).ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a positive integer".to_string(),
                }
            })
        }
        _ => Err(HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: name.to_string(),
            detail: "requires a positive integer".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    fn test_ctx() -> HarnessResolutionContext<'static> {
        HarnessResolutionContext {
            source_path: Path::new("/repo/prompts/run.md"),
            repo_root: Some(Path::new("/repo")),
        }
    }

    fn source() -> &'static Path {
        Path::new("/repo/prompts/run.md")
    }

    #[test]
    fn has_harness_properties_detects_pre_checks() {
        let fm = json!({ "pre_checks": [] });
        assert!(has_harness_properties(&fm));
    }

    #[test]
    fn has_harness_properties_detects_post_checks() {
        let fm = json!({ "post_checks": {} });
        assert!(has_harness_properties(&fm));
    }

    #[test]
    fn has_harness_properties_detects_timeout() {
        let fm = json!({ "timeout": "30s" });
        assert!(has_harness_properties(&fm));
    }

    #[test]
    fn has_harness_properties_detects_handle() {
        let fm = json!({ "handle": { "command": ["./fix.sh"] } });
        assert!(has_harness_properties(&fm));
    }

    #[test]
    fn has_harness_properties_detects_handle_event() {
        let fm = json!({ "handle_timeout": {} });
        assert!(has_harness_properties(&fm));
    }

    #[test]
    fn has_harness_properties_returns_false_for_plain() {
        let fm = json!({ "title": "Hello", "agent": "claude" });
        assert!(!has_harness_properties(&fm));
    }

    #[test]
    fn parse_list_form_pre_checks() {
        let fm = json!({
            "pre_checks": [
                { "file_exists": "@docs/plan.md" },
                { "dir_exists": "@docs" }
            ]
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.pre_checks.len(), 2);
        assert!(matches!(plan.pre_checks[0].kind, ValidationKind::FileExists { .. }));
        assert!(matches!(plan.pre_checks[1].kind, ValidationKind::DirExists { .. }));
    }

    #[test]
    fn parse_map_form_pre_checks() {
        let fm = json!({
            "pre_checks": {
                "file_exists": "@docs/plan.md"
            }
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.pre_checks.len(), 1);
    }

    #[test]
    fn parse_expanded_form_with_msg() {
        let fm = json!({
            "pre_checks": [{
                "json_file_exists": {
                    "file": "./data.json",
                    "shape": "object",
                    "msg": "{{status}} data file is valid"
                }
            }]
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.pre_checks.len(), 1);
        assert!(matches!(
            plan.pre_checks[0].kind,
            ValidationKind::JsonFileExists { shape: Some(StructuredShape::Object), .. }
        ));
        assert_eq!(
            plan.pre_checks[0].message_template.as_deref(),
            Some("{{status}} data file is valid")
        );
    }

    #[test]
    fn parse_frontmatter_prop_equals() {
        let fm = json!({
            "post_checks": [{
                "frontmatter_prop_equals": {
                    "status": "complete",
                    "approved": true
                }
            }]
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.post_checks.len(), 1);
        if let ValidationKind::FrontmatterPropEquals { expected } = &plan.post_checks[0].kind {
            assert_eq!(expected.len(), 2);
            assert_eq!(expected["status"], json!("complete"));
            assert_eq!(expected["approved"], json!(true));
        } else {
            panic!("expected FrontmatterPropEquals");
        }
    }

    #[test]
    fn reject_unknown_validation() {
        let fm = json!({ "pre_checks": { "flie_exists": "foo.md" } });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(matches!(err, HarnessError::UnknownValidation { .. }));
    }

    #[test]
    fn reject_post_only_in_pre_checks_file_changed() {
        let fm = json!({ "pre_checks": { "file_changed": "@foo.md" } });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(matches!(err, HarnessError::PostOnlyInPreChecks { .. }));
    }

    #[test]
    fn reject_post_only_in_pre_checks_response_includes() {
        let fm = json!({ "pre_checks": { "response_includes": "hello" } });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(matches!(err, HarnessError::PostOnlyInPreChecks { .. }));
    }

    #[test]
    fn parse_valid_timeout() {
        let fm = json!({ "timeout": "5m" });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.timeout, Some(std::time::Duration::from_secs(300)));
    }

    #[test]
    fn reject_invalid_timeout() {
        let fm = json!({ "timeout": "5x" });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(matches!(err, HarnessError::InvalidTimeout { .. }));
    }

    #[test]
    fn parse_generic_handle_timeout() {
        let fm = json!({
            "handle_timeout": {
                "resume": {
                    "prompt": "Continue from where you left off.",
                    "retries": 2
                }
            }
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.handlers.generic.len(), 1);
        assert_eq!(plan.handlers.generic[0].event, FailureEvent::Timeout);
        assert!(matches!(plan.handlers.generic[0].action, HandlerAction::Resume { .. }));
    }

    #[test]
    fn parse_subject_specific_handler() {
        let fm = json!({
            "handle_file_exists": {
                "@docs/output.md": {
                    "retry": {
                        "prompt": "Create the missing file."
                    }
                }
            }
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.handlers.exact.len(), 1);
        // Subject key should be canonicalized (resolved via repo root)
        assert_eq!(
            plan.handlers.exact[0].subject_key.as_deref(),
            Some("/repo/docs/output.md")
        );
    }

    #[test]
    fn reject_resume_missing_prompt() {
        let fm = json!({
            "handle_timeout": {
                "resume": {
                    "retries": 2
                }
            }
        });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(matches!(err, HarnessError::MissingHandlerField { .. }));
    }

    #[test]
    fn reject_redirect_missing_file() {
        let fm = json!({
            "handle_agent_failure": {
                "redirect": {
                    "msg": "Redirecting..."
                }
            }
        });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(matches!(err, HarnessError::MissingHandlerField { .. }));
    }

    #[test]
    fn parse_response_checks_in_post() {
        let fm = json!({
            "post_checks": {
                "response_missing": "failed",
                "response_length_at_least": 150
            }
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.post_checks.len(), 2);
    }

    #[test]
    fn parse_programmatic_handler_array() {
        let fm = json!({
            "handle": {
                "command": ["./scripts/repair-handler", "--json"]
            }
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        let cmd = plan.programmatic_handler.unwrap();
        assert_eq!(cmd.executable, "./scripts/repair-handler");
        assert_eq!(cmd.args, vec!["--json"]);
    }

    #[test]
    fn stable_rule_ids_preserve_order() {
        let fm = json!({
            "pre_checks": [
                { "file_exists": "@a.md" },
                { "dir_exists": "@b" }
            ],
            "post_checks": [
                { "file_changed": "@c.md" }
            ]
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.pre_checks[0].id, ValidationRuleId(0));
        assert_eq!(plan.pre_checks[1].id, ValidationRuleId(1));
        assert_eq!(plan.post_checks[0].id, ValidationRuleId(2));
    }

    #[test]
    fn reject_invalid_pre_checks_structure() {
        let fm = json!({ "pre_checks": "not a list" });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(matches!(err, HarnessError::InvalidFrontmatter { .. }));
    }

    #[test]
    fn reject_invalid_shape() {
        let fm = json!({
            "pre_checks": [{
                "json_file_exists": {
                    "file": "./data.json",
                    "shape": "invalid"
                }
            }]
        });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(matches!(err, HarnessError::InvalidShape { .. }));
    }
}
