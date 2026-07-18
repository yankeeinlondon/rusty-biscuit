//! External sequence-source resolution and loading.
//!
//! Resolves a `sequence: <file-ref>` reference through
//! [`biscuit_file::FileReference`] and loads the external YAML into a normalized
//! [`SequencePlan`]. The Sequence Plus source grammar (offsets/operators,
//! dynamic sources, `ListFormat` classification) lands in phase 4; this module
//! carries the pre-existing plain-reference + `kind: sequence`/`list:` loading
//! forward onto the new [`normalize_plan`] path.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext};
use regex::Regex;
use serde_json::{Map, Value};

use super::super::error::{CompositionError, SequenceLoadCause};
use super::super::json_util::json_type_name;
use super::model::SequenceSource;
use super::normalize::normalize_plan;
use super::reserved;

/// Matches `{{key}}` and `{{key || default}}` placeholder patterns.
static PLACEHOLDER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{\s*([^{}|]+?)(?:\s*\|\|\s*([^{}]*?))?\s*\}\}")
        .expect("placeholder regex is valid")
});

/// Resolve an external sequence reference string to an absolute existing path.
///
/// Delegates all grammar and candidate ordering to [`FileReference`] and the
/// shared [`FileResolutionContext`] (D5/D11): implicit refs probe the
/// repository root then the source directory; explicit refs pin to the source
/// directory; `@`/`!`/`vault:`/`~`/absolute keep their usual meanings. The
/// reference is authored inside the composition source, so the source
/// document's directory is the base and the launch directory is never a
/// fallback here.
pub fn resolve_sequence_reference(raw: &str, source_path: &Path) -> Result<PathBuf, CompositionError> {
    // A leading `@/` is the magic-root search for `x`, identical to `@x`;
    // normalize it as explicit `FileReference` input rather than string surgery.
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
    let ctx = build_sequence_resolution_context(&file_ref, base_dir, source_path);

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

/// Capture the request-scoped resolution anchors for an external sequence
/// reference authored inside a composition source.
///
/// The worktree root and package area are discovered once here (via `sniff`) and
/// passed in, so `FileReference` resolution never re-probes git or Cargo state
/// per reference. Package-area discovery is skipped for every reference kind
/// except `!`, whose resolution is the only one that consumes it.
fn build_sequence_resolution_context(
    file_ref: &FileReference,
    base_dir: &Path,
    source_path: &Path,
) -> FileResolutionContext {
    let mut ctx = FileResolutionContext::new(base_dir).with_source_path(source_path);

    let Ok(Some(repo_root)) = sniff::filesystem::git::repo_root(base_dir) else {
        return ctx;
    };
    if !base_dir.starts_with(&repo_root) {
        return ctx;
    }
    ctx = ctx.with_repository_root(repo_root.clone());

    if file_ref.class().kind == FileReferenceKind::Package
        && let Ok(Some(repo)) = sniff::filesystem::detect_repo(&repo_root)
        && let Some(area) = repo.package_area_label_for_dir(base_dir)
    {
        ctx = ctx.with_package_area(repo_root.join(area.as_ref()));
    }
    ctx
}

/// Load and normalize an external YAML sequence file.
///
/// `invocation_path` is the resolved path of the document that referenced this
/// file; it seeds the `sequence_id` payload so the token stays keyed to the
/// invocation, not the data file.
pub fn load_external_sequence(
    yaml_path: &Path,
    invocation_path: &Path,
    document_fail_fast: bool,
) -> Result<super::model::SequencePlan, CompositionError> {
    let yaml = biscuit_file::Yaml::new(yaml_path).map_err(|e| {
        CompositionError::SequenceExternalLoad {
            context: yaml_path.display().to_string(),
            source: SequenceLoadCause::Yaml(e),
        }
    })?;
    let json_value = yaml
        .as_json()
        .map_err(|e| CompositionError::SequenceExternalLoad {
            context: yaml_path.display().to_string(),
            source: SequenceLoadCause::Yaml(e),
        })?;
    let root = json_value.as_object().ok_or_else(|| {
        CompositionError::SequenceExternalWrongType("root must be an object".to_string())
    })?;

    let source = SequenceSource::External {
        path: yaml_path.to_path_buf(),
    };

    // Form 1: { sequence: [...] }
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
        return normalize_plan(items, source, invocation_path, document_fail_fast);
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

    let templated = apply_template(root.get("template"), items)?;
    normalize_plan(&templated, source, invocation_path, document_fail_fast)
}

/// Validate an optional `template` object and apply it to the list items,
/// returning the item values with template-derived fields filled in where the
/// item did not already define them.
fn apply_template(
    template: Option<&Value>,
    items: &[Value],
) -> Result<Vec<Value>, CompositionError> {
    let template = match template {
        Some(Value::Object(map)) => map,
        Some(other) => {
            return Err(CompositionError::SequenceExternalWrongType(format!(
                "`template` must be an object, got {}",
                json_type_name(other)
            )));
        }
        None => return Ok(items.to_vec()),
    };

    for (key, value) in template {
        if reserved::is_reserved_state_key(key) {
            return Err(CompositionError::SequenceReservedTemplateKey(key.clone()));
        }
        if !value.is_string() {
            return Err(CompositionError::SequenceTemplateWrongType {
                key: key.clone(),
                found: json_type_name(value).to_string(),
            });
        }
    }

    items
        .iter()
        .map(|item| {
            let step_map = item
                .as_object()
                .ok_or(CompositionError::SequenceTemplateRequiresObjectItems)?;
            let mut new_map = step_map.clone();
            for (tmpl_key, tmpl_value) in template {
                let template_str = tmpl_value.as_str().expect("validated string above");
                let rendered = render_simple_template(template_str, step_map);
                new_map
                    .entry(tmpl_key.clone())
                    .or_insert(Value::String(rendered));
            }
            Ok(Value::Object(new_map))
        })
        .collect()
}

/// Simple `{{key}}` / `{{key || default}}` renderer over an item's top-level
/// fields. Intentionally smaller than the Darkmatter expression engine; the
/// full engine takes over template evaluation in phase 4.
pub(super) fn render_simple_template(template: &str, fields: &Map<String, Value>) -> String {
    PLACEHOLDER_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = caps[1].trim();
            let default = caps.get(2).map(|m| m.as_str().trim().trim_matches('\''));

            match fields.get(key) {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                Some(Value::Null) | None => default.unwrap_or("").to_string(),
                Some(other) => other.to_string(),
            }
        })
        .into_owned()
}
