//! Frontmatter-to-plan parser for harness configuration.
//!
//! Accepts `pre_checks` and `post_checks` in both list form (canonical) and
//! map form (shorthand), parses handler declarations, and builds a typed
//! [`HarnessPlan`].

use std::ops::RangeInclusive;
use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;
use tracing::debug;

use self::span::{SpanIndex, find_rule_spans};
use crate::harness::error::HarnessError;
use crate::harness::model::{
    ApprovedRuntimeCommand, FailureEvent, HandlerAction, HandlerRule, HandlerTable, HarnessPlan,
    RuleSource, StructuredShape, ValidationEvent, ValidationKind, ValidationPhase, ValidationRule,
    ValidationRuleId,
};
use crate::harness::resolve::{HarnessResolutionContext, resolve_harness_path};
use crate::harness::timeout::{format_duration, parse_timeout};

/// Harness-relevant frontmatter keys.
const HARNESS_KEYS: &[&str] = &[
    "pre_checks",
    "post_checks",
    "timeout",
    "step_timeout",
    "timeout_warn",
    "step_timeout_warn",
    "handle",
];

/// Check whether composed frontmatter contains any harness-relevant keys.
///
/// Returns `true` if any of `pre_checks`, `post_checks`, `timeout`,
/// `step_timeout`, `timeout_warn`, `step_timeout_warn`, `handle`, or any
/// `handle_*` key is present.
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
    let obj = frontmatter
        .as_object()
        .ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "(root)".to_string(),
            detail: "frontmatter must be an object".to_string(),
        })?;

    let mut next_id: u32 = 0;
    let mut alloc_id = || {
        let id = ValidationRuleId(next_id);
        next_id += 1;
        id
    };

    // Best-effort source-span recovery. Read the markdown from disk and
    // extract the YAML frontmatter slice; on any IO or framing failure
    // fall back to an empty `SpanIndex` so `line_range` stays `None` and
    // `yaml_snippet` falls back to reconstructed YAML.
    let raw_source = std::fs::read_to_string(source_path).ok();
    let frontmatter_slice: Option<(&str, usize)> = raw_source
        .as_deref()
        .and_then(extract_frontmatter_text);
    let span_index = frontmatter_slice
        .map(|(text, base_line)| find_rule_spans(text, base_line))
        .unwrap_or_default();

    // Parse pre_checks
    let pre_checks = if let Some(v) = obj.get("pre_checks") {
        parse_checks(
            v,
            true,
            source_path,
            ctx,
            &span_index,
            frontmatter_slice,
            &mut alloc_id,
        )?
    } else {
        Vec::new()
    };

    // Parse post_checks
    let post_checks = if let Some(v) = obj.get("post_checks") {
        parse_checks(
            v,
            false,
            source_path,
            ctx,
            &span_index,
            frontmatter_slice,
            &mut alloc_id,
        )?
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

    // Parse step_timeout (silence deadline). Same syntax as `timeout`.
    let step_timeout = if let Some(v) = obj.get("step_timeout") {
        let raw = v.as_str().ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "step_timeout".to_string(),
            detail: "step_timeout must be a string (e.g. \"30s\", \"5m\")".to_string(),
        })?;
        Some(parse_timeout(raw, source_path)?)
    } else {
        None
    };

    // Relational validation: step_timeout must not exceed the wall-clock
    // timeout when both are present. A step budget greater than the wall
    // clock is always unreachable.
    if let (Some(step), Some(total)) = (step_timeout, timeout)
        && step > total
    {
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: format_duration(step),
            detail: format!(
                "step_timeout ({}) must not exceed timeout ({})",
                format_duration(step),
                format_duration(total),
            ),
        });
    }

    // Parse timeout_warn (wall-clock warning threshold). Same syntax as
    // `timeout`.
    let timeout_warn = if let Some(v) = obj.get("timeout_warn") {
        let raw = v.as_str().ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "timeout_warn".to_string(),
            detail: "timeout_warn must be a string (e.g. \"30s\", \"5m\")".to_string(),
        })?;
        Some(parse_timeout(raw, source_path)?)
    } else {
        None
    };

    // Parse step_timeout_warn (silence warning threshold). Same syntax as
    // `step_timeout`.
    let step_timeout_warn = if let Some(v) = obj.get("step_timeout_warn") {
        let raw = v.as_str().ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "step_timeout_warn".to_string(),
            detail: "step_timeout_warn must be a string (e.g. \"30s\", \"5m\")".to_string(),
        })?;
        Some(parse_timeout(raw, source_path)?)
    } else {
        None
    };

    // Relational validation: each `*_warn` must be strictly less than its
    // corresponding hard threshold when both are present. A warn equal
    // to or greater than the hard timeout would never fire before the
    // kill signal, defeating the purpose of an early warning. Rejection
    // is a preflight hard error and emits a `tracing::error!` adjacent
    // to the user-facing message per the timing feature spec.
    if let (Some(warn), Some(hard)) = (timeout_warn, timeout)
        && warn >= hard
    {
        tracing::error!(
            source_path = %source_path.display(),
            timeout_warn_secs = warn.as_secs(),
            timeout_secs = hard.as_secs(),
            "preflight rejection: timeout_warn must be less than timeout",
        );
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: format_duration(warn),
            detail: format!(
                "timeout_warn ({}) must be less than timeout ({})",
                format_duration(warn),
                format_duration(hard),
            ),
        });
    }
    if let (Some(warn), Some(hard)) = (step_timeout_warn, step_timeout)
        && warn >= hard
    {
        tracing::error!(
            source_path = %source_path.display(),
            step_timeout_warn_secs = warn.as_secs(),
            step_timeout_secs = hard.as_secs(),
            "preflight rejection: step_timeout_warn must be less than step_timeout",
        );
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: format_duration(warn),
            detail: format!(
                "step_timeout_warn ({}) must be less than step_timeout ({})",
                format_duration(warn),
                format_duration(hard),
            ),
        });
    }

    // Parse handlers
    let (handlers, programmatic_handler) = parse_handlers(obj, source_path, ctx)?;

    let plan = HarnessPlan {
        source_path: source_path.to_path_buf(),
        timeout,
        step_timeout,
        timeout_warn,
        step_timeout_warn,
        pre_checks,
        post_checks,
        handlers,
        programmatic_handler,
    };
    debug!(
        source = %source_path.display(),
        pre_checks = plan.pre_checks.len(),
        post_checks = plan.post_checks.len(),
        timeout_secs = plan.timeout.map(|d| d.as_secs()),
        "parsed harness plan",
    );
    Ok(plan)
}

/// Create a system-owned `has_write_permission` pre-check rule for the
/// source document itself.
///
/// Inline composition requires write access to the source file. When a
/// harness is active, this rule participates in the normal pre-check
/// pipeline so that handler recovery paths (redirect, deviate, etc.)
/// can respond to permission failures instead of hard-failing before
/// the handler system exists.
pub fn inline_writability_pre_check(source_path: &Path) -> ValidationRule {
    ValidationRule {
        id: ValidationRuleId(u32::MAX),
        event: ValidationEvent::HasWritePermission,
        phase: ValidationPhase::PreOnly,
        kind: ValidationKind::HasWritePermission {
            file: source_path.to_path_buf(),
        },
        message_template: None,
        subject_key: Some(source_path.display().to_string()),
        source: None,
    }
}

/// Parse a `pre_checks` or `post_checks` value in list or map form.
fn parse_checks(
    value: &Value,
    is_pre: bool,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
    spans: &SpanIndex,
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

/// Metadata for a validation kind: its event, allowed phase, and name.
struct ValidationMeta {
    event: ValidationEvent,
    phase: ValidationPhase,
}

/// Look up validation metadata by name string.
fn validation_meta(name: &str) -> Option<ValidationMeta> {
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

/// Parse a single validation from its name and YAML value.
#[allow(clippy::too_many_arguments)]
fn parse_single_validation(
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

/// Build a [`RuleSource`] for an author-declared validation rule.
///
/// Prefers the original source slice authored by the user when a
/// [`SpanIndex`] line range and the raw frontmatter text are both
/// available, preserving comments, quoting, anchors, and indentation
/// exactly as written. Falls back to a `serde_yaml_ng`-reconstructed
/// single-key mapping when span recovery is unavailable.
///
/// ## Returns
///
/// `None` only when the reconstruction fallback also fails to produce
/// a YAML snippet (e.g. the JSON value is not representable as YAML),
/// leaving the reporter to fall back to its legacy single-line failure
/// rendering.
fn build_rule_source(
    source_path: &Path,
    rule_name: &str,
    value: &Value,
    line_range: Option<RangeInclusive<usize>>,
    frontmatter_slice: Option<(&str, usize)>,
) -> Option<RuleSource> {
    let yaml_snippet = original_yaml_slice(line_range.as_ref(), frontmatter_slice)
        .or_else(|| reconstruct_yaml_snippet(rule_name, value))?;
    Some(RuleSource {
        file: source_path.to_path_buf(),
        line_range,
        yaml_snippet,
    })
}

/// Slice the original YAML lines for a rule out of the raw frontmatter text.
///
/// `line_range` is the 1-indexed inclusive line range of the rule **within
/// the source file**, as recovered by the conservative span finder.
/// `frontmatter_slice` carries the frontmatter text and the 1-indexed line
/// number of its first body line (i.e. the line immediately after the
/// opening `---` fence). When either is absent, or the requested range
/// falls outside the available frontmatter body, returns `None` so the
/// caller can fall back to YAML reconstruction.
///
/// The returned slice preserves all interior whitespace, comments,
/// quoting, and indentation exactly as authored, with at most a single
/// trailing newline trimmed.
fn original_yaml_slice(
    line_range: Option<&RangeInclusive<usize>>,
    frontmatter_slice: Option<(&str, usize)>,
) -> Option<String> {
    let range = line_range?;
    let (text, base_line) = frontmatter_slice?;
    let start = *range.start();
    let end = *range.end();
    if start < base_line || end < start {
        return None;
    }
    // Translate 1-indexed source-file line numbers into 0-indexed offsets
    // within `text` (whose first line corresponds to `base_line`).
    let start_idx = start - base_line;
    let end_idx = end - base_line;
    let lines: Vec<&str> = text.split('\n').collect();
    if end_idx >= lines.len() {
        return None;
    }
    let mut snippet = lines[start_idx..=end_idx].join("\n");
    // Trim a single trailing newline only; preserve all other whitespace.
    if snippet.ends_with('\n') {
        snippet.pop();
    }
    Some(snippet)
}

/// Reconstruct a YAML mapping (`name: value`) for the rule via
/// `serde_yaml_ng`. Used as the fallback when the original source slice
/// is unavailable.
fn reconstruct_yaml_snippet(rule_name: &str, value: &Value) -> Option<String> {
    use biscuit_file::serde_yaml_ng;
    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(
        serde_yaml_ng::Value::String(rule_name.to_string()),
        serde_yaml_ng::to_value(value).ok()?,
    );
    serde_yaml_ng::to_string(&serde_yaml_ng::Value::Mapping(map)).ok()
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
fn parse_failure_event(name: &str, source_path: &Path) -> Result<FailureEvent, HarnessError> {
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
fn parse_handler_entry(
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

/// Parse an optional `set` overlay from a handler object.
fn parse_set_overlay(obj: &serde_json::Map<String, Value>) -> Option<IndexMap<String, Value>> {
    obj.get("set")
        .and_then(|v| v.as_object())
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
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
    Err(HarnessError::PathResolutionFailed {
        raw: value.to_string(),
        detail: format!("expected a file path string or an object with a `{field}` field"),
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
    value
        .as_object()
        .and_then(|o| o.get(field))
        .and_then(|v| v.as_bool())
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
fn extract_usize(value: &Value, name: &str, source_path: &Path) -> Result<usize, HarnessError> {
    match value {
        Value::Number(n) => {
            n.as_u64()
                .map(|v| v as usize)
                .ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a positive integer".to_string(),
                })
        }
        Value::Object(obj) => {
            // Expanded form: look for a primary field
            let length_val = obj
                .get("length")
                .or_else(|| obj.get("value"))
                .ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a number or an object with a `length` field".to_string(),
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

/// Extract the YAML frontmatter slice from a markdown source, returning the
/// text between the opening and closing `---` fences and the 1-indexed line
/// number of the first frontmatter content line.
///
/// ## Returns
///
/// `Some((yaml_text, base_line))` where `base_line` is the 1-indexed line
/// number of the first body line of the frontmatter (the line immediately
/// after the opening `---`). `None` if the file does not start with a `---`
/// fence on its first line, or if no closing fence is found.
fn extract_frontmatter_text(source: &str) -> Option<(&str, usize)> {
    // Frontmatter must start at the very first byte of the file with `---`
    // followed by a newline. Anything else (BOMs, leading whitespace, CRLF
    // before the first delimiter) falls through to the `None` fallback.
    let rest = source.strip_prefix("---\n")?;
    // Find the closing `---` line. Accept either `---\n` or a trailing
    // `---` at end of string. Look for it as a line by itself.
    let mut byte_offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        if trimmed == "---" {
            // Slice inside the fences.
            let yaml = &rest[..byte_offset];
            return Some((yaml, 2));
        }
        byte_offset += line.len();
    }
    None
}

mod span {
    //! Conservative line-range recovery for `pre_checks` / `post_checks`
    //! frontmatter blocks.
    //!
    //! Given the raw YAML frontmatter text and the 1-indexed line number of
    //! its first line, produce per-rule line ranges keyed by declaration
    //! order. The finder is intentionally conservative: when it cannot
    //! unambiguously match a rule's span, it omits the range rather than
    //! guessing.

    use std::ops::RangeInclusive;

    /// Per-rule line ranges keyed by declaration order.
    #[derive(Debug, Default, Clone)]
    pub(crate) struct SpanIndex {
        pre: Vec<Option<RangeInclusive<usize>>>,
        post: Vec<Option<RangeInclusive<usize>>>,
    }

    impl SpanIndex {
        /// Look up the line range of the *i*-th `pre_checks` rule in
        /// declaration order. Returns `None` if no unambiguous span was
        /// recovered or the index is out of bounds.
        pub(crate) fn pre_check(&self, i: usize) -> Option<RangeInclusive<usize>> {
            self.pre.get(i).cloned().flatten()
        }

        /// Look up the line range of the *i*-th `post_checks` rule in
        /// declaration order. Returns `None` if no unambiguous span was
        /// recovered or the index is out of bounds.
        pub(crate) fn post_check(&self, i: usize) -> Option<RangeInclusive<usize>> {
            self.post.get(i).cloned().flatten()
        }
    }

    /// Build a [`SpanIndex`] for the given frontmatter text.
    ///
    /// `base_line` is the 1-indexed line number of the first line of
    /// `frontmatter_text`. The frontmatter slice does NOT include the
    /// opening `---` fence; the caller is expected to have stripped it.
    ///
    /// ## Returns
    ///
    /// A `SpanIndex` with one slot per rule in declaration order. Slots are
    /// `Some(range)` when the finder is confident; `None` otherwise.
    pub(crate) fn find_rule_spans(frontmatter_text: &str, base_line: usize) -> SpanIndex {
        let mut index = SpanIndex::default();

        let lines: Vec<&str> = frontmatter_text.split('\n').collect();
        // Locate the `pre_checks:` / `post_checks:` parent lines. These must
        // appear at indent 0 (top-level frontmatter keys) with no prefix
        // whitespace; anything more nested is rejected as ambiguous.
        for (parent_key, slot) in [
            ("pre_checks", &mut index.pre),
            ("post_checks", &mut index.post),
        ] {
            let Some(parent_idx) = find_top_level_key(&lines, parent_key) else {
                continue;
            };
            *slot = collect_rule_spans(&lines, parent_idx, base_line);
        }

        index
    }

    /// Find the index in `lines` of a top-level frontmatter key matching
    /// `key`. Top-level means zero leading whitespace and `key:` immediately
    /// at column 0. Returns the *first* match. If the key appears more than
    /// once at the top level, returns `None` (ambiguous).
    fn find_top_level_key(lines: &[&str], key: &str) -> Option<usize> {
        let mut found: Option<usize> = None;
        let needle_eq = format!("{key}:");
        let needle_prefix = format!("{key}: ");
        for (i, line) in lines.iter().enumerate() {
            if *line == needle_eq || line.starts_with(&needle_prefix) {
                if found.is_some() {
                    return None;
                }
                found = Some(i);
            }
        }
        found
    }

    /// Walk lines after `parent_idx` and produce per-rule line ranges in
    /// declaration order. Handles list form (entries beginning with `-`) and
    /// map form (indented `name:` keys).
    fn collect_rule_spans(
        lines: &[&str],
        parent_idx: usize,
        base_line: usize,
    ) -> Vec<Option<RangeInclusive<usize>>> {
        // Determine the body indent by looking at the first non-blank,
        // non-comment line after the parent. If none exists, no rules.
        let mut body_indent: Option<usize> = None;
        for line in lines.iter().skip(parent_idx + 1) {
            if is_blank_or_comment(line) {
                continue;
            }
            let indent = leading_spaces(line);
            if indent == 0 {
                // Sibling top-level key — no body.
                break;
            }
            body_indent = Some(indent);
            break;
        }
        let Some(body_indent) = body_indent else {
            return Vec::new();
        };

        // Detect form: list form starts each entry with `- `; map form does not.
        let mut entries: Vec<RangeInclusive<usize>> = Vec::new();
        let mut current_start: Option<usize> = None;
        let mut current_end: usize = 0;
        let mut is_list_form: Option<bool> = None;

        let body_iter = lines.iter().enumerate().skip(parent_idx + 1);
        for (i, line) in body_iter {
            if is_blank_or_comment(line) {
                continue;
            }
            let indent = leading_spaces(line);
            if indent < body_indent {
                // Either back to top level or to a shallower scope; rules end.
                break;
            }
            if indent == body_indent {
                // Possible new rule entry.
                let trimmed = &line[indent..];
                let starts_with_dash = trimmed.starts_with("- ") || trimmed == "-";
                match is_list_form {
                    None => {
                        is_list_form = Some(starts_with_dash);
                    }
                    Some(true) => {
                        if !starts_with_dash {
                            // Inconsistent with list form; abort.
                            return mark_all_ambiguous(entries.len() + 1);
                        }
                    }
                    Some(false) => {
                        if starts_with_dash {
                            // Inconsistent with map form; abort.
                            return mark_all_ambiguous(entries.len() + 1);
                        }
                    }
                }
                // Close out the previous entry.
                if let Some(start) = current_start.take() {
                    entries.push(start..=current_end);
                }
                current_start = Some(i);
                current_end = i;
            } else if indent > body_indent {
                // Continuation of the current rule (multi-line value).
                if current_start.is_some() {
                    current_end = i;
                }
            }
        }
        if let Some(start) = current_start.take() {
            entries.push(start..=current_end);
        }

        // Detect ambiguity: duplicate top-level rule names within this block.
        // Map form: the key on the entry's start line (after indent, before `:`).
        // List form: the first inline key after `- ` on the entry's start line.
        let mut seen: Vec<&str> = Vec::with_capacity(entries.len());
        let mut ambiguous = false;
        let names: Vec<Option<&str>> = entries
            .iter()
            .map(|r| extract_rule_name(lines[*r.start()], body_indent, is_list_form))
            .collect();
        for name in names.iter().flatten() {
            if seen.contains(name) {
                ambiguous = true;
                break;
            }
            seen.push(name);
        }

        if ambiguous {
            return entries.iter().map(|_| None).collect();
        }

        entries
            .into_iter()
            .map(|r| Some((r.start() + base_line)..=(r.end() + base_line)))
            .collect()
    }

    /// Build an all-`None` vector of length `n` to signal ambiguity.
    fn mark_all_ambiguous(n: usize) -> Vec<Option<RangeInclusive<usize>>> {
        vec![None; n]
    }

    /// Return the count of leading space characters in `line`.
    fn leading_spaces(line: &str) -> usize {
        line.bytes().take_while(|b| *b == b' ').count()
    }

    /// Return `true` for blank lines and comment-only lines (lines whose
    /// first non-space character is `#`).
    fn is_blank_or_comment(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.is_empty() || trimmed.starts_with('#')
    }

    /// Extract the rule name from the first line of a rule entry.
    ///
    /// For map form, the line looks like `  rule_name:` or `  rule_name: value`.
    /// For list form, the line looks like `  - rule_name: value` or
    /// `  - rule_name:`.
    fn extract_rule_name(
        line: &str,
        body_indent: usize,
        is_list_form: Option<bool>,
    ) -> Option<&str> {
        if line.len() < body_indent {
            return None;
        }
        let after_indent = &line[body_indent..];
        let key_part = if is_list_form == Some(true) {
            // strip `- ` or `-` prefix
            after_indent
                .strip_prefix("- ")
                .or_else(|| after_indent.strip_prefix('-'))?
        } else {
            after_indent
        };
        let colon = key_part.find(':')?;
        let raw_name = key_part[..colon].trim();
        if raw_name.is_empty() {
            return None;
        }
        // Comments between `- ` and the key are unusual but possible; bail
        // out by rejecting any name containing whitespace or `#`.
        if raw_name
            .bytes()
            .any(|b| b == b' ' || b == b'#' || b == b'\t')
        {
            return None;
        }
        Some(raw_name)
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
        assert!(matches!(
            plan.pre_checks[0].kind,
            ValidationKind::FileExists { .. }
        ));
        assert!(matches!(
            plan.pre_checks[1].kind,
            ValidationKind::DirExists { .. }
        ));
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
                    "msg": "{{file}} data file is valid"
                }
            }]
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.pre_checks.len(), 1);
        assert!(matches!(
            plan.pre_checks[0].kind,
            ValidationKind::JsonFileExists {
                shape: Some(StructuredShape::Object),
                ..
            }
        ));
        assert_eq!(
            plan.pre_checks[0].message_template.as_deref(),
            Some("{{file}} data file is valid")
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
        assert!(matches!(
            plan.handlers.generic[0].action,
            HandlerAction::Resume { .. }
        ));
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

    #[test]
    fn reject_file_validation_object_without_file_field() {
        let fm = json!({
            "post_checks": {
                "file_exists": {
                    "name": "goose.md",
                    "say": "missing"
                }
            }
        });

        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        let message = err.to_string();

        assert!(
            message.contains("`file` field"),
            "unexpected error: {message}"
        );
        assert!(
            message.contains("\"name\":\"goose.md\""),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn parse_harness_plan_extracts_step_timeout() {
        let fm = json!({ "step_timeout": "2m" });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.step_timeout, Some(std::time::Duration::from_secs(120)));
        assert!(plan.timeout.is_none());
    }

    #[test]
    fn parse_harness_plan_rejects_non_string_step_timeout() {
        let fm = json!({ "step_timeout": 120 });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        assert!(
            matches!(err, HarnessError::InvalidFrontmatter { ref property, .. } if property == "step_timeout"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_harness_plan_rejects_step_timeout_exceeding_timeout() {
        let fm = json!({
            "timeout": "1m",
            "step_timeout": "5m"
        });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        match err {
            HarnessError::InvalidTimeout { ref detail, .. } => {
                assert!(
                    detail.contains("must not exceed"),
                    "unexpected detail: {detail}"
                );
                assert!(detail.contains("5m"), "expected 5m in detail: {detail}");
                assert!(detail.contains("1m"), "expected 1m in detail: {detail}");
            }
            other => panic!("expected InvalidTimeout, got: {other:?}"),
        }
    }

    #[test]
    fn parse_harness_plan_accepts_step_timeout_without_timeout() {
        let fm = json!({ "step_timeout": "30s" });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.step_timeout, Some(std::time::Duration::from_secs(30)));
        assert!(plan.timeout.is_none());
    }

    #[test]
    fn parse_harness_plan_accepts_timeout_without_step_timeout() {
        let fm = json!({ "timeout": "5m" });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.timeout, Some(std::time::Duration::from_secs(300)));
        assert!(plan.step_timeout.is_none());
    }

    #[test]
    fn parse_harness_plan_accepts_equal_step_and_wall_timeouts() {
        let fm = json!({
            "timeout": "2m",
            "step_timeout": "2m"
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.timeout, Some(std::time::Duration::from_secs(120)));
        assert_eq!(plan.step_timeout, Some(std::time::Duration::from_secs(120)));
    }

    #[test]
    fn has_harness_properties_returns_true_for_step_timeout_only() {
        let fm = json!({ "step_timeout": "30s" });
        assert!(has_harness_properties(&fm));
    }

    #[test]
    fn harness_plan_default_step_timeout_is_none() {
        let fm = json!({});
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert!(plan.step_timeout.is_none());
    }

    #[test]
    fn parse_harness_plan_extracts_timeout_warn() {
        let fm = json!({ "timeout_warn": "30s" });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.timeout_warn, Some(std::time::Duration::from_secs(30)));
    }

    #[test]
    fn parse_harness_plan_extracts_step_timeout_warn() {
        let fm = json!({ "step_timeout_warn": "45s" });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(
            plan.step_timeout_warn,
            Some(std::time::Duration::from_secs(45))
        );
    }

    #[test]
    fn parse_harness_plan_rejects_timeout_warn_equal_to_timeout() {
        let fm = json!({ "timeout": "5m", "timeout_warn": "5m" });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        match err {
            HarnessError::InvalidTimeout { detail, .. } => {
                assert!(
                    detail.contains("timeout_warn") && detail.contains("less than"),
                    "detail must name both values: {detail}"
                );
            }
            other => panic!("expected InvalidTimeout, got: {other:?}"),
        }
    }

    #[test]
    fn parse_harness_plan_rejects_timeout_warn_greater_than_timeout() {
        let fm = json!({ "timeout": "1m", "timeout_warn": "2m" });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        match err {
            HarnessError::InvalidTimeout { detail, .. } => {
                assert!(detail.contains("1m"), "detail must contain 1m: {detail}");
                assert!(detail.contains("2m"), "detail must contain 2m: {detail}");
            }
            other => panic!("expected InvalidTimeout, got: {other:?}"),
        }
    }

    #[test]
    fn parse_harness_plan_rejects_step_timeout_warn_greater_than_step_timeout() {
        let fm = json!({ "step_timeout": "30s", "step_timeout_warn": "2m" });
        let err = parse_harness_plan(&fm, source(), &test_ctx()).unwrap_err();
        match err {
            HarnessError::InvalidTimeout { detail, .. } => {
                assert!(
                    detail.contains("step_timeout_warn") && detail.contains("less than"),
                    "detail must name both values: {detail}"
                );
            }
            other => panic!("expected InvalidTimeout, got: {other:?}"),
        }
    }

    #[test]
    fn parse_harness_plan_rejects_zero_or_negative_warn_values() {
        // Zero is rejected by the underlying timeout parser.
        let fm_zero = json!({ "timeout_warn": "0s" });
        assert!(parse_harness_plan(&fm_zero, source(), &test_ctx()).is_err());

        // Strings without a number parse as errors — the parser requires
        // a positive integer or float followed by a unit.
        let fm_neg = json!({ "step_timeout_warn": "-5s" });
        assert!(parse_harness_plan(&fm_neg, source(), &test_ctx()).is_err());
    }

    #[test]
    fn parse_harness_plan_accepts_timeout_warn_without_timeout() {
        let fm = json!({ "timeout_warn": "30s" });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.timeout_warn, Some(std::time::Duration::from_secs(30)));
        assert!(plan.timeout.is_none());
    }

    #[test]
    fn parse_harness_plan_accepts_step_timeout_warn_without_step_timeout() {
        let fm = json!({ "step_timeout_warn": "15s" });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(
            plan.step_timeout_warn,
            Some(std::time::Duration::from_secs(15))
        );
        assert!(plan.step_timeout.is_none());
    }

    #[test]
    fn has_harness_properties_detects_timeout_warn() {
        let fm = json!({ "timeout_warn": "30s" });
        assert!(has_harness_properties(&fm));
    }

    #[test]
    fn has_harness_properties_detects_step_timeout_warn() {
        let fm = json!({ "step_timeout_warn": "15s" });
        assert!(has_harness_properties(&fm));
    }

    #[test]
    fn parse_rules_carry_source_with_yaml_snippet() {
        let fm = json!({
            "pre_checks": [
                { "file_exists": "Cargo.toml" }
            ]
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.pre_checks.len(), 1);
        let src = plan.pre_checks[0]
            .source
            .as_ref()
            .expect("rule source should be populated");
        assert_eq!(src.file, source());
        assert!(src.line_range.is_none());
        assert!(
            src.yaml_snippet.contains("file_exists"),
            "snippet should contain rule name: {}",
            src.yaml_snippet,
        );
        assert!(
            src.yaml_snippet.contains("Cargo.toml"),
            "snippet should contain rule value: {}",
            src.yaml_snippet,
        );
    }

    #[test]
    fn parse_rules_source_field_uses_source_path() {
        let fm = json!({
            "pre_checks": { "file_exists": "@docs/plan.md" }
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        assert_eq!(plan.pre_checks[0].source.as_ref().unwrap().file, source());
    }

    #[test]
    fn parse_rules_yaml_snippet_round_trips() {
        let fm = json!({
            "pre_checks": [
                { "dir_exists": "@docs" }
            ]
        });
        let plan = parse_harness_plan(&fm, source(), &test_ctx()).unwrap();
        let snippet = &plan.pre_checks[0].source.as_ref().unwrap().yaml_snippet;
        let value: biscuit_file::serde_yaml_ng::Value =
            biscuit_file::serde_yaml_ng::from_str(snippet)
                .expect("snippet should round-trip through serde_yaml_ng");
        let map = value.as_mapping().expect("snippet should be a mapping");
        assert_eq!(map.len(), 1, "snippet should have a single key");
        let key = map
            .keys()
            .next()
            .and_then(|k| k.as_str())
            .expect("key should be a string");
        assert_eq!(key, "dir_exists");
    }

    #[test]
    fn inline_writability_rule_has_no_source() {
        let rule = inline_writability_pre_check(source());
        assert!(rule.source.is_none());
    }

    // ---- Phase 2: span-finder tests -------------------------------------

    /// List form with three rules — each on its own line. Lines are
    /// 1-indexed against the full source file, where the opening `---`
    /// fence is line 1.
    #[test]
    fn find_rule_spans_list_form_returns_per_rule_ranges() {
        let frontmatter = "pre_checks:\n  - file_exists: a\n  - dir_exists: b\n  - shell_command: \"ls\"\n";
        let spans = span::find_rule_spans(frontmatter, 2);
        assert_eq!(spans.pre_check(0), Some(3..=3));
        assert_eq!(spans.pre_check(1), Some(4..=4));
        assert_eq!(spans.pre_check(2), Some(5..=5));
        assert!(spans.pre_check(3).is_none());
        // Ranges are non-overlapping.
        assert!(spans.pre_check(0).unwrap().end() < spans.pre_check(1).unwrap().start());
        assert!(spans.pre_check(1).unwrap().end() < spans.pre_check(2).unwrap().start());
    }

    /// Map form with three rules.
    #[test]
    fn find_rule_spans_map_form_returns_per_rule_ranges() {
        let frontmatter =
            "pre_checks:\n  file_exists: a\n  dir_exists: b\n  shell_command: \"ls\"\n";
        let spans = span::find_rule_spans(frontmatter, 2);
        assert_eq!(spans.pre_check(0), Some(3..=3));
        assert_eq!(spans.pre_check(1), Some(4..=4));
        assert_eq!(spans.pre_check(2), Some(5..=5));
    }

    /// Multi-line map values (e.g. `file_exists:` followed by an indented
    /// `message:` key) extend the rule's range to cover the continuation
    /// lines.
    #[test]
    fn find_rule_spans_multiline_map_value_extends_range() {
        let frontmatter = "pre_checks:\n  - file_exists: a\n    message: \"first\"\n  - dir_exists: b\n";
        let spans = span::find_rule_spans(frontmatter, 2);
        // Rule 0 spans both `- file_exists` (line 3) and `  message:` (line 4).
        assert_eq!(spans.pre_check(0), Some(3..=4));
        assert_eq!(spans.pre_check(1), Some(5..=5));
    }

    /// Duplicate top-level rule names within the same block produce
    /// `None` for every rule in declaration order — the conservative
    /// fallback.
    #[test]
    fn find_rule_spans_returns_none_when_ambiguous() {
        let frontmatter = "pre_checks:\n  - file_exists: a\n  - file_exists: b\n";
        let spans = span::find_rule_spans(frontmatter, 2);
        assert!(spans.pre_check(0).is_none());
        assert!(spans.pre_check(1).is_none());
    }

    /// `parse_harness_plan` populates `line_range` when the source file is
    /// readable on disk.
    #[test]
    fn parse_rules_carry_line_range_when_recoverable() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        writeln!(
            tmp,
            "---\npre_checks:\n  - file_exists: \"Cargo.toml\"\n  - dir_exists: \"docs\"\n---\nbody"
        )
        .expect("write tempfile");
        tmp.flush().expect("flush tempfile");

        // A resolution context whose repo_root matches the tempfile's
        // parent so `@`-prefixed paths resolve cleanly. The validation
        // values above use bare paths so this is irrelevant for parsing,
        // but we still need a context.
        let ctx = HarnessResolutionContext {
            source_path: tmp.path(),
            repo_root: None,
        };
        let frontmatter = json!({
            "pre_checks": [
                { "file_exists": "Cargo.toml" },
                { "dir_exists": "docs" }
            ]
        });
        let plan = parse_harness_plan(&frontmatter, tmp.path(), &ctx).unwrap();
        let lr_a = plan.pre_checks[0]
            .source
            .as_ref()
            .and_then(|s| s.line_range.clone())
            .expect("first rule should have a recovered line range");
        let lr_b = plan.pre_checks[1]
            .source
            .as_ref()
            .and_then(|s| s.line_range.clone())
            .expect("second rule should have a recovered line range");
        assert_eq!(lr_a, 3..=3);
        assert_eq!(lr_b, 4..=4);
    }

    /// When the source file cannot be read, parsing still succeeds and
    /// `line_range` is `None`.
    #[test]
    fn parse_rules_no_line_range_when_source_file_unreadable() {
        let bogus =
            Path::new("/this/path/does/not/exist/__claudine_phase2_missing__/run.md");
        let ctx = HarnessResolutionContext {
            source_path: bogus,
            repo_root: None,
        };
        let frontmatter = json!({
            "pre_checks": [
                { "file_exists": "Cargo.toml" }
            ]
        });
        let plan = parse_harness_plan(&frontmatter, bogus, &ctx).unwrap();
        let src = plan.pre_checks[0]
            .source
            .as_ref()
            .expect("source should still be populated via reconstruction");
        assert!(src.line_range.is_none());
    }

    // ---- Phase 3: original YAML slice tests -----------------------------

    /// Single-quoted scalar values are preserved verbatim from the source
    /// frontmatter rather than being re-emitted in serde_yaml_ng's default
    /// (unquoted) form.
    #[test]
    fn parse_rules_yaml_snippet_preserves_authored_quoting() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        writeln!(
            tmp,
            "---\npre_checks:\n  - file_exists: 'Cargo.toml'\n---\nbody"
        )
        .expect("write tempfile");
        tmp.flush().expect("flush tempfile");

        let ctx = HarnessResolutionContext {
            source_path: tmp.path(),
            repo_root: None,
        };
        let frontmatter = json!({
            "pre_checks": [
                { "file_exists": "Cargo.toml" }
            ]
        });
        let plan = parse_harness_plan(&frontmatter, tmp.path(), &ctx).unwrap();
        let snippet = &plan.pre_checks[0]
            .source
            .as_ref()
            .expect("rule source should be populated")
            .yaml_snippet;
        assert!(
            snippet.contains("'Cargo.toml'"),
            "snippet should preserve single quotes: {snippet}",
        );
    }

    /// Inline trailing comments authored on the rule line are preserved in
    /// the snippet.
    #[test]
    fn parse_rules_yaml_snippet_preserves_inline_comments() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        writeln!(
            tmp,
            "---\npre_checks:\n  - file_exists: \"Cargo.toml\" # author comment\n---\nbody"
        )
        .expect("write tempfile");
        tmp.flush().expect("flush tempfile");

        let ctx = HarnessResolutionContext {
            source_path: tmp.path(),
            repo_root: None,
        };
        let frontmatter = json!({
            "pre_checks": [
                { "file_exists": "Cargo.toml" }
            ]
        });
        let plan = parse_harness_plan(&frontmatter, tmp.path(), &ctx).unwrap();
        let snippet = &plan.pre_checks[0]
            .source
            .as_ref()
            .expect("rule source should be populated")
            .yaml_snippet;
        assert!(
            snippet.contains("# author comment"),
            "snippet should retain inline comment: {snippet}",
        );
    }

    /// When the span finder cannot recover an unambiguous range (e.g.
    /// duplicate top-level rule names), the snippet falls back to the
    /// reconstructed YAML, which must still parse back through
    /// `serde_yaml_ng::from_str`.
    #[test]
    fn parse_rules_yaml_snippet_falls_back_when_span_unrecoverable() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        writeln!(
            tmp,
            "---\npre_checks:\n  - file_exists: \"a\"\n  - file_exists: \"b\"\n---\nbody"
        )
        .expect("write tempfile");
        tmp.flush().expect("flush tempfile");

        let ctx = HarnessResolutionContext {
            source_path: tmp.path(),
            repo_root: None,
        };
        let frontmatter = json!({
            "pre_checks": [
                { "file_exists": "a" },
                { "file_exists": "b" }
            ]
        });
        let plan = parse_harness_plan(&frontmatter, tmp.path(), &ctx).unwrap();
        let src = plan.pre_checks[0]
            .source
            .as_ref()
            .expect("rule source should be populated");
        // Span recovery declined to commit a range, so the slice path is
        // unavailable and we must have used reconstruction.
        assert!(
            src.line_range.is_none(),
            "ambiguous span should yield no line range",
        );
        let value: biscuit_file::serde_yaml_ng::Value =
            biscuit_file::serde_yaml_ng::from_str(&src.yaml_snippet)
                .expect("reconstructed snippet should parse as YAML");
        let map = value.as_mapping().expect("snippet should be a mapping");
        assert_eq!(map.len(), 1, "reconstructed snippet should be single-key");
        let key = map
            .keys()
            .next()
            .and_then(|k| k.as_str())
            .expect("key should be a string");
        assert_eq!(key, "file_exists");
    }
}
