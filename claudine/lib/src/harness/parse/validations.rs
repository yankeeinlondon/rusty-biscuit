//! Validation parsing: `pre_checks` / `post_checks` and per-kind parameters.

use std::ops::RangeInclusive;
use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

use crate::harness::error::HarnessError;
use crate::harness::model::{
    ValidationEvent, ValidationKind, ValidationPhase, ValidationRule, ValidationRuleId,
};
use crate::harness::resolve::{HarnessResolutionContext, resolve_harness_path};

use super::frontmatter::build_rule_source;
use super::overlays::tokenize_to_approved_command;
use super::shapes::{
    extract_bool_field, extract_file_ref, extract_scalar_string, extract_shape,
    extract_string_field, extract_usize,
};

/// Parse a `pre_checks` or `post_checks` value in list or map form.
pub(super) fn parse_checks(
    value: &Value,
    is_pre: bool,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
    spans: &super::span::SpanIndex,
    frontmatter_slice: Option<(&str, usize)>,
    alloc_id: &mut impl FnMut() -> ValidationRuleId,
) -> Result<Vec<ValidationRule>, HarnessError> {
    let property = if is_pre { "pre_checks" } else { "post_checks" };

    match value {
        Value::Array(arr) => {
            // List form: each element is a single-key object
            let mut rules = Vec::with_capacity(arr.len());
            let mut decl_index: usize = 0;
            for item in arr {
                let obj = item
                    .as_object()
                    .ok_or_else(|| HarnessError::InvalidFrontmatter {
                        source_path: source_path.to_path_buf(),
                        property: property.to_string(),
                        detail: "each item in the list must be an object".to_string(),
                    })?;
                for (name, val) in obj {
                    let line_range = if is_pre {
                        spans.pre_check(decl_index)
                    } else {
                        spans.post_check(decl_index)
                    };
                    let rule = parse_single_validation(
                        name,
                        val,
                        is_pre,
                        source_path,
                        ctx,
                        line_range,
                        frontmatter_slice,
                        alloc_id,
                    )?;
                    rules.push(rule);
                    decl_index += 1;
                }
            }
            Ok(rules)
        }
        Value::Object(obj) => {
            // Map form: shorthand
            let mut rules = Vec::with_capacity(obj.len());
            for (decl_index, (name, val)) in obj.iter().enumerate() {
                let line_range = if is_pre {
                    spans.pre_check(decl_index)
                } else {
                    spans.post_check(decl_index)
                };
                let rule = parse_single_validation(
                    name,
                    val,
                    is_pre,
                    source_path,
                    ctx,
                    line_range,
                    frontmatter_slice,
                    alloc_id,
                )?;
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

/// Parse a single validation from its name and YAML value.
#[allow(clippy::too_many_arguments)]
pub(super) fn parse_single_validation(
    name: &str,
    value: &Value,
    is_pre: bool,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
    line_range: Option<RangeInclusive<usize>>,
    frontmatter_slice: Option<(&str, usize)>,
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

    let source = build_rule_source(source_path, name, value, line_range, frontmatter_slice);

    Ok(ValidationRule {
        id: alloc_id(),
        event: meta.event,
        phase: meta.phase,
        kind,
        message_template: msg,
        subject_key,
        source,
    })
}

/// Parse a validation's kind-specific parameters from its value.
///
/// Returns `(ValidationKind, Option<subject_key>)`.
pub(super) fn parse_validation_kind(
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
                ValidationKind::JsonFileExists {
                    file: path.clone(),
                    shape,
                },
                Some(path.display().to_string()),
            ))
        }
        "yaml_file_exists" => {
            let file_ref = extract_file_ref(value, "file")?;
            let path = resolve_harness_path(&file_ref, ctx)?;
            let shape = extract_shape(value, source_path)?;
            Ok((
                ValidationKind::YamlFileExists {
                    file: path.clone(),
                    shape,
                },
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
            let raw = extract_string_field(value, "cmd")
                .or_else(|| extract_scalar_string(value))
                .ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a command string or an object with a `cmd` field".to_string(),
                })?;
            let show_stdout = extract_bool_field(value, "show_stdout").unwrap_or(true);
            let show_stderr = extract_bool_field(value, "show_stderr").unwrap_or(true);
            let command = tokenize_to_approved_command(&raw, source_path)?;
            Ok((
                ValidationKind::ShellCommand {
                    command,
                    show_stdout,
                    show_stderr,
                },
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
            let prop =
                extract_scalar_string(value).ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a property name string".to_string(),
                })?;
            Ok((
                ValidationKind::FrontmatterPropChanged { prop: prop.clone() },
                Some(prop),
            ))
        }
        "frontmatter_prop_unchanged" => {
            let prop =
                extract_scalar_string(value).ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a property name string".to_string(),
                })?;
            Ok((
                ValidationKind::FrontmatterPropUnchanged { prop: prop.clone() },
                Some(prop),
            ))
        }
        "frontmatter_prop_equals" => {
            let obj = value
                .as_object()
                .ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a mapping of property names to expected values".to_string(),
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
            let needle =
                extract_scalar_string(value).ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a search string".to_string(),
                })?;
            Ok((
                ValidationKind::ResponseIncludes {
                    needle: needle.clone(),
                },
                Some(needle),
            ))
        }
        "response_missing" => {
            let needle =
                extract_scalar_string(value).ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a search string".to_string(),
                })?;
            Ok((
                ValidationKind::ResponseMissing {
                    needle: needle.clone(),
                },
                Some(needle),
            ))
        }
        _ => unreachable!("unknown validation name should have been caught earlier"),
    }
}

/// Metadata for a validation kind: its event, allowed phase, and name.
pub(super) struct ValidationMeta {
    event: ValidationEvent,
    phase: ValidationPhase,
}

/// Look up validation metadata by name string.
pub(super) fn validation_meta(name: &str) -> Option<ValidationMeta> {
    let meta = match name {
        "file_exists" => ValidationMeta {
            event: ValidationEvent::FileExists,
            phase: ValidationPhase::Both,
        },
        "dir_exists" => ValidationMeta {
            event: ValidationEvent::DirExists,
            phase: ValidationPhase::Both,
        },
        "json_file_exists" => ValidationMeta {
            event: ValidationEvent::JsonFileExists,
            phase: ValidationPhase::Both,
        },
        "yaml_file_exists" => ValidationMeta {
            event: ValidationEvent::YamlFileExists,
            phase: ValidationPhase::Both,
        },
        "toml_file_exists" => ValidationMeta {
            event: ValidationEvent::TomlFileExists,
            phase: ValidationPhase::Both,
        },
        "has_write_permission" => ValidationMeta {
            event: ValidationEvent::HasWritePermission,
            phase: ValidationPhase::Both,
        },
        "shell_command" => ValidationMeta {
            event: ValidationEvent::ShellCommand,
            phase: ValidationPhase::Both,
        },
        "no_dirty_source_code" => ValidationMeta {
            event: ValidationEvent::NoDirtySourceCode,
            phase: ValidationPhase::Both,
        },
        "has_dirty_source_code" => ValidationMeta {
            event: ValidationEvent::HasDirtySourceCode,
            phase: ValidationPhase::Both,
        },
        "file_changed" => ValidationMeta {
            event: ValidationEvent::FileChanged,
            phase: ValidationPhase::PostOnly,
        },
        "file_unchanged" => ValidationMeta {
            event: ValidationEvent::FileUnchanged,
            phase: ValidationPhase::PostOnly,
        },
        "frontmatter_prop_changed" => ValidationMeta {
            event: ValidationEvent::FrontmatterPropChanged,
            phase: ValidationPhase::PostOnly,
        },
        "frontmatter_prop_unchanged" => ValidationMeta {
            event: ValidationEvent::FrontmatterPropUnchanged,
            phase: ValidationPhase::PostOnly,
        },
        "frontmatter_prop_equals" => ValidationMeta {
            event: ValidationEvent::FrontmatterPropEquals,
            phase: ValidationPhase::PostOnly,
        },
        "response_length_at_least" => ValidationMeta {
            event: ValidationEvent::ResponseLengthAtLeast,
            phase: ValidationPhase::PostOnly,
        },
        "response_length_at_most" => ValidationMeta {
            event: ValidationEvent::ResponseLengthAtMost,
            phase: ValidationPhase::PostOnly,
        },
        "response_includes" => ValidationMeta {
            event: ValidationEvent::ResponseIncludes,
            phase: ValidationPhase::PostOnly,
        },
        "response_missing" => ValidationMeta {
            event: ValidationEvent::ResponseMissing,
            phase: ValidationPhase::PostOnly,
        },
        // Built-in inline closure events. These are never declared as
        // pre/post checks — they are produced by the closure path and
        // only need validation_meta entries so `handle_*` keys resolve.
        "inline_response_empty" => ValidationMeta {
            event: ValidationEvent::InlineResponseEmpty,
            phase: ValidationPhase::PostOnly,
        },
        "inline_body_unchanged" => ValidationMeta {
            event: ValidationEvent::InlineBodyUnchanged,
            phase: ValidationPhase::PostOnly,
        },
        _ => return None,
    };
    Some(meta)
}
