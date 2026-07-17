//! Sequence detection, parsing, normalization, and overlay generation.
//!
//! Provides [`resolve_sequence_plan`] to detect whether a resolved composition
//! source defines a sequence, and if so, parse and normalize it into a typed
//! [`SequencePlan`]. Also provides [`build_step_overlay`] to construct the
//! per-step variable overlay for each composition run.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use biscuit_file::{FileReference, FileResolutionContext};
use regex::Regex;

use super::error::{CompositionError, SequenceLoadCause};
use super::json_util::json_type_name;
use super::types::{SequencePlan, SequenceSource, SequenceStep, SequenceStepOverlay};

/// Matches `{{key}}` and `{{key || default}}` placeholder patterns.
static PLACEHOLDER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{\s*([^{}|]+?)(?:\s*\|\|\s*([^{}]*?))?\s*\}\}")
        .expect("placeholder regex is valid")
});

/// Detect and resolve a sequence plan from a resolved composition source.
///
/// Returns `Ok(None)` if the source has no `sequence` frontmatter key.
/// Returns `Ok(Some(plan))` if a valid sequence is found.
///
/// ## Errors
///
/// Returns `Err` for invalid sequence definitions: wrong types, missing
/// `name` on object steps, empty lists, or external file load failures.
pub fn resolve_sequence_plan(
    source: &super::types::ResolvedCompositionSource,
) -> Result<Option<SequencePlan>, CompositionError> {
    let fm = source.markdown.frontmatter();
    let sequence_value = match fm.as_map().get("sequence") {
        Some(v) => v.clone(),
        None => return Ok(None),
    };

    let fail_fast = match fm.as_map().get("fail_fast") {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(other) => {
            return Err(CompositionError::SequenceInvalid(format!(
                "`fail_fast` must be a boolean, got {}",
                json_type_name(other)
            )));
        }
        None => true,
    };

    match sequence_value {
        serde_json::Value::Array(items) => {
            let steps = normalize_inline_list(&items)?;
            Ok(Some(SequencePlan {
                source: SequenceSource::Inline,
                steps,
                document_fail_fast: fail_fast,
            }))
        }
        serde_json::Value::String(ref path_str) => {
            let yaml_path = resolve_sequence_reference(path_str, &source.resolved_path)?;
            let plan = load_external_sequence(&yaml_path, fail_fast)?;
            Ok(Some(plan))
        }
        other => Err(CompositionError::SequenceInvalid(format!(
            "expected a list or file path string, got {}",
            json_type_name(&other)
        ))),
    }
}

/// Resolve an external sequence reference string to an absolute existing path.
///
/// Delegates all grammar and candidate ordering to [`FileReference`] and the
/// shared [`FileResolutionContext`]; this module no longer classifies prefixes
/// or joins/expands paths itself (D5/D11):
/// - implicit (`foo.yaml`, `sub/foo.yaml`) — repository root first, then the
///   source document's directory (D4);
/// - explicit (`./foo.yaml`, `../foo.yaml`) — pinned to the source directory;
/// - `@foo` — magic-root search; `!foo` — package-area; `vault:foo` — vault;
/// - `~foo` / `~/foo` — the user's home directory (the shared `Home` kind);
/// - absolute paths resolve to themselves.
///
/// The reference is authored *inside* the composition source, so per D2 the
/// source document's directory is the base and the launch directory is never a
/// fallback here. Resolution probes the filesystem, so only an existing regular
/// file is a match.
fn resolve_sequence_reference(raw: &str, source_path: &Path) -> Result<PathBuf, CompositionError> {
    // Preserve the `@/x` → `@x` normalization as explicit `FileReference`
    // input rather than local string surgery elsewhere: a leading `@/` is the
    // magic-root search for `x`, identical to `@x`.
    let normalized;
    let ref_input = if let Some(rest) = raw.strip_prefix("@/") {
        normalized = format!("@{rest}");
        &normalized
    } else {
        raw
    };

    let file_ref =
        FileReference::new(ref_input).map_err(|e| CompositionError::SequenceExternalLoad {
            context: format!("`{raw}`"),
            source: e.into(),
        })?;

    let base_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    let ctx = FileResolutionContext::new(base_dir).with_source_path(source_path);

    file_ref
        .resolve_in_context(&ctx)
        .map_err(|e| CompositionError::SequenceExternalLoad {
            context: format!("`{raw}`"),
            source: e.into(),
        })?
        .ok_or_else(|| CompositionError::SequenceExternalLoad {
            context: format!("`{raw}`"),
            source: SequenceLoadCause::NotFound,
        })
}

/// Build a step overlay for the given step index within a plan.
pub fn build_step_overlay(plan: &SequencePlan, step_index: usize) -> SequenceStepOverlay {
    let total = plan.steps.len();
    let current = &plan.steps[step_index];

    let previous_state = if step_index > 0 {
        plan.steps[step_index - 1].raw_state.clone()
    } else {
        serde_json::Value::Null
    };

    let next_state = if step_index + 1 < total {
        plan.steps[step_index + 1].raw_state.clone()
    } else {
        serde_json::Value::Null
    };

    SequenceStepOverlay {
        state: current.raw_state.clone(),
        previous_state,
        next_state,
        is_first: step_index == 0,
        is_last: step_index + 1 == total,
        step: step_index + 1,
        total_steps: total,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Normalize an inline YAML list into typed steps.
fn normalize_inline_list(
    items: &[serde_json::Value],
) -> Result<Vec<SequenceStep>, CompositionError> {
    if items.is_empty() {
        return Err(CompositionError::SequenceEmpty);
    }

    items
        .iter()
        .enumerate()
        .map(|(index, item)| match item {
            serde_json::Value::String(s) => Ok(SequenceStep {
                index,
                name: s.clone(),
                raw_state: item.clone(),
            }),
            serde_json::Value::Object(map) => {
                let name = map
                    .get("name")
                    .ok_or(CompositionError::SequenceStepNameMissing { index })?;
                let name_str =
                    name.as_str()
                        .ok_or_else(|| CompositionError::SequenceStepNameWrongType {
                            index,
                            found: json_type_name(name).to_string(),
                        })?;
                Ok(SequenceStep {
                    index,
                    name: name_str.to_string(),
                    raw_state: item.clone(),
                })
            }
            other => Err(CompositionError::SequenceInvalid(format!(
                "step at index {index} must be a string or object, got {}",
                json_type_name(other)
            ))),
        })
        .collect()
}

/// Load and parse an external YAML sequence file.
fn load_external_sequence(
    yaml_path: &Path,
    document_fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    let yaml = biscuit_file::Yaml::new(yaml_path).map_err(|e| {
        CompositionError::SequenceExternalLoad {
            context: yaml_path.display().to_string(),
            source: SequenceLoadCause::Yaml(e),
        }
    })?;
    let json_value = yaml.as_json().map_err(|e| CompositionError::SequenceExternalLoad {
        context: yaml_path.display().to_string(),
        source: SequenceLoadCause::Yaml(e),
    })?;
    let root = json_value.as_object().ok_or_else(|| {
        CompositionError::SequenceExternalWrongType("root must be an object".to_string())
    })?;

    // Detect which external form is used:
    // Form 1: { sequence: [...] }
    // Form 2: { kind: "sequence", list: [...], template?: {...} }
    if let Some(list_value) = root.get("sequence") {
        if root.contains_key("template") {
            return Err(CompositionError::SequenceExternalWrongType(
                "`template` is only supported alongside `list` (use `kind: sequence` + `list:` \
                 form when you need templates)"
                    .to_string(),
            ));
        }
        let items = list_value.as_array().ok_or_else(|| {
            CompositionError::SequenceExternalWrongType("`sequence` must be a list".to_string())
        })?;
        let steps = normalize_inline_list(items)?;
        return Ok(SequencePlan {
            source: SequenceSource::External {
                path: yaml_path.to_path_buf(),
            },
            steps,
            document_fail_fast,
        });
    }

    // Form 2: kind/list/template
    if let Some(kind_value) = root.get("kind") {
        let kind_str = kind_value.as_str().ok_or_else(|| {
            CompositionError::SequenceExternalWrongType("`kind` must be a string".to_string())
        })?;
        if kind_str != "sequence" {
            return Err(CompositionError::SequenceExternalWrongType(format!(
                "`kind` must be \"sequence\", got \"{kind_str}\""
            )));
        }
    }

    let list_value = root.get("list").ok_or_else(|| {
        CompositionError::SequenceExternalWrongType(
            "external file must have `sequence` or `list` key".to_string(),
        )
    })?;
    let items = list_value.as_array().ok_or_else(|| {
        CompositionError::SequenceExternalWrongType("`list` must be a list".to_string())
    })?;

    // `template` is optional, but when present it must be an object.
    let template = match root.get("template") {
        Some(serde_json::Value::Object(map)) => Some(map),
        Some(other) => {
            return Err(CompositionError::SequenceExternalWrongType(format!(
                "`template` must be an object, got {}",
                json_type_name(other)
            )));
        }
        None => None,
    };

    // Validate template keys don't collide with reserved overlay keys
    if let Some(tmpl) = template {
        for key in tmpl.keys() {
            if SequenceStepOverlay::RESERVED_KEYS.contains(&key.as_str()) {
                return Err(CompositionError::SequenceReservedTemplateKey(key.clone()));
            }
        }
        // Validate template values are all strings
        for (key, value) in tmpl {
            if !value.is_string() {
                return Err(CompositionError::SequenceTemplateWrongType {
                    key: key.clone(),
                    found: json_type_name(value).to_string(),
                });
            }
        }
    }

    let mut steps = normalize_inline_list(items)?;

    // Apply template fields to each step
    if let Some(tmpl) = template {
        for step in &mut steps {
            if !step.raw_state.is_object() {
                return Err(CompositionError::SequenceTemplateRequiresObjectItems);
            }
            let step_map = step.raw_state.as_object().unwrap();
            let mut new_map = step_map.clone();

            for (tmpl_key, tmpl_value) in tmpl {
                let template_str = tmpl_value.as_str().unwrap(); // validated above
                let rendered = render_simple_template(template_str, step_map);
                new_map
                    .entry(tmpl_key.clone())
                    .or_insert(serde_json::Value::String(rendered));
            }

            step.raw_state = serde_json::Value::Object(new_map);
        }
    }

    Ok(SequencePlan {
        source: SequenceSource::External {
            path: yaml_path.to_path_buf(),
        },
        steps,
        document_fail_fast,
    })
}

/// Simple `{{key}}` and `{{key || default}}` template renderer.
///
/// Replaces `{{key}}` with the value from the item's fields.
/// Supports `{{key || default}}` fallback syntax.
fn render_simple_template(
    template: &str,
    fields: &serde_json::Map<String, serde_json::Value>,
) -> String {
    PLACEHOLDER_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = caps[1].trim();
            let default = caps.get(2).map(|m| m.as_str().trim().trim_matches('\''));

            match fields.get(key) {
                Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
                Some(serde_json::Value::Null) | None => default.unwrap_or("").to_string(),
                Some(other) => other.to_string(),
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests;
