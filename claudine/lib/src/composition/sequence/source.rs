//! External sequence-source resolution and loading.
//!
//! Resolves a `sequence: <file-ref> [-> offset] [::op(args)]` reference through
//! [`biscuit_file::FileReference`], loads the target in any supported format,
//! narrows it to a list, applies the operator, and normalizes the result into a
//! [`SequencePlan`].
//!
//! Whether the result is normalized strictly or leniently is decided here, by
//! [`is_formal_sequence`]: a root `sequence:` list reached without an offset or
//! operator is a document authored *for* sequences and stays strict; everything
//! else — an arbitrary data file, a sequence document read through an offset, or
//! any line-delimited file — is foreign data.

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext};
use serde_json::{Map, Value};

use super::super::error::{CompositionError, SequenceLoadCause};
use super::super::json_util::json_type_name;
use super::data::{self, SourceFormat};
use super::expr::{SourceExpressionLookup, render_interpolated};
use super::grammar::{SequenceReference, SourceOperator};
use super::model::{SequencePlan, SequenceSource};
use super::normalize::normalize_plan;
use super::reserved;

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

/// Load a referenced sequence source and normalize it into a plan.
///
/// `invocation_path` is the resolved path of the document that referenced this
/// file; it seeds the `sequence_id` payload so the token stays keyed to the
/// invocation, not the data file. `frontmatter` is that document's frontmatter,
/// available to `template` expressions as globals.
pub fn load_referenced_sequence(
    reference: &SequenceReference,
    path: &Path,
    invocation_path: &Path,
    frontmatter: &Map<String, Value>,
    document_fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    let format = SourceFormat::for_path(path);
    let document = data::load_document(path)?;

    if let Some(offset) = &reference.offset
        && format.is_line_delimited()
    {
        let _ = offset;
        return Err(CompositionError::SequenceOffsetUnsupported {
            path: path.to_path_buf(),
            format: line_delimited_label(path),
        });
    }

    let formal = is_formal_sequence(&document, reference);
    let SelectedItems { items, template } = select_items(&document, reference, formal)?;

    let items = match &reference.operator {
        Some(operator) => apply_source_operator(items, operator, frontmatter, invocation_path)?,
        None => items,
    };

    // Formal `template:` values fill in before generated fields so a templated
    // key is ordinary authored state by the time ids and positions are made.
    let items = match template {
        Some(template) => apply_template(&template, items, frontmatter, invocation_path)?,
        None => items,
    };

    let source = if formal {
        SequenceSource::External {
            path: path.to_path_buf(),
        }
    } else {
        SequenceSource::DataFile {
            path: path.to_path_buf(),
        }
    };

    let plan = normalize_plan(&items, source, invocation_path, document_fail_fast)?;
    if formal {
        validate_state_schema(&plan, &document, path)?;
    }
    Ok(plan)
}

/// `JSONL` or `NDJSON`, for the offset-unsupported error.
fn line_delimited_label(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("ndjson") => "NDJSON".to_string(),
        _ => "JSONL".to_string(),
    }
}

/// A document is a *formal* sequence when its root carries `sequence:` and the
/// reference reads that root directly. Reaching into it with an offset, or
/// reshaping it with an operator, means the caller is treating it as data.
fn is_formal_sequence(document: &Value, reference: &SequenceReference) -> bool {
    reference.offset.is_none()
        && reference.operator.is_none()
        && document
            .as_object()
            .is_some_and(|root| root.contains_key("sequence"))
}

/// A loaded document narrowed to the parts the plan is built from.
struct SelectedItems {
    /// The list the steps come from.
    items: Vec<Value>,
    /// The formal document's `template:` map, when it has one. Only a formal
    /// root carries one — an offset or operator means the caller is treating
    /// the document as data, where `template:` is just another key.
    template: Option<Map<String, Value>>,
}

/// Narrow a loaded document to its item list, plus the formal `template:` map.
fn select_items(
    document: &Value,
    reference: &SequenceReference,
    formal: bool,
) -> Result<SelectedItems, CompositionError> {
    if let Some(offset) = &reference.offset {
        let node = data::apply_offset(document, offset)?;
        return Ok(SelectedItems {
            items: data::expect_list(node, Some(offset))?,
            template: None,
        });
    }

    let Some(root) = document.as_object() else {
        // A root array is the ordinary shape for JSON/JSONL data files.
        return Ok(SelectedItems {
            items: data::expect_list(document, None)?,
            template: None,
        });
    };

    if formal {
        let list = root.get("sequence").expect("formal implies `sequence`");
        let template = match root.get("template") {
            Some(Value::Object(map)) => Some(map.clone()),
            Some(other) => {
                return Err(CompositionError::SequenceExternalWrongType(format!(
                    "`template` must be an object, got {}",
                    json_type_name(other)
                )));
            }
            None => None,
        };
        return Ok(SelectedItems {
            items: data::expect_list(list, Some("sequence"))?,
            template,
        });
    }

    // Clean break (RATIFIED 2026-07-12): the external-only `kind: sequence` +
    // `list:` form is retired. Naming it explicitly beats "must have `sequence`"
    // for the authors who have to migrate.
    if root.contains_key("list") {
        return Err(CompositionError::SequenceExternalWrongType(
            "the `list:` sequence shape has been removed; rename the property to `sequence:` \
             (the same shape now works both when referenced and when invoked directly)"
                .to_string(),
        ));
    }

    Err(CompositionError::SequenceExternalWrongType(
        "sequence file must have a root `sequence:` list, be a list at its root, or be \
         reached with a `-> offset` path"
            .to_string(),
    ))
}

/// Apply a `::map`/`::name`/`::template` operator to every item.
fn apply_source_operator(
    items: Vec<Value>,
    operator: &SourceOperator,
    frontmatter: &Map<String, Value>,
    invocation_path: &Path,
) -> Result<Vec<Value>, CompositionError> {
    let base_dir = invocation_path.parent().unwrap_or_else(|| Path::new("."));
    data::apply_operator(items, operator, &|expression, item| {
        let lookup = SourceExpressionLookup::new(frontmatter, base_dir).with_item(item);
        super::expr::evaluate_whole(expression, &lookup)
    })
}

/// Validate a formal document's `template` and apply it to every item.
///
/// A template key never overwrites a value the item already defines, and every
/// value is rendered through the Darkmatter expression engine with the item's
/// fields shadowing the invoking document's frontmatter.
fn apply_template(
    template: &Map<String, Value>,
    items: Vec<Value>,
    frontmatter: &Map<String, Value>,
    invocation_path: &Path,
) -> Result<Vec<Value>, CompositionError> {
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

    let base_dir = invocation_path.parent().unwrap_or_else(|| Path::new("."));

    items
        .into_iter()
        .map(|item| {
            let step_map = item
                .as_object()
                .ok_or(CompositionError::SequenceTemplateRequiresObjectItems)?;
            let lookup = SourceExpressionLookup::new(frontmatter, base_dir).with_item(step_map);

            let mut new_map = step_map.clone();
            for (key, value) in template {
                if new_map.contains_key(key) {
                    continue;
                }
                let raw = value.as_str().expect("validated string above");
                new_map.insert(key.clone(), render_interpolated(raw, &lookup)?);
            }
            Ok(Value::Object(new_map))
        })
        .collect()
}

/// Validate every step's normalized state against a formal document's
/// `$schema`.
///
/// Only the state portion is validated: executable and task keys are not state
/// and were already partitioned out during normalization. The schema is applied
/// through the ordinary Darkmatter document validator by presenting each state
/// as a frontmatter-shaped document, so `$schema` behaves identically here and
/// on a normal composition source.
fn validate_state_schema(
    plan: &SequencePlan,
    document: &Value,
    path: &Path,
) -> Result<(), CompositionError> {
    let Some(schema) = document.as_object().and_then(|root| root.get("$schema")) else {
        return Ok(());
    };

    let schemas = darkmatter::markdown::schemas::DarkmatterSchemas::new();

    let load_failure = |source: SequenceLoadCause| CompositionError::SequenceExternalLoad {
        context: path.display().to_string(),
        source,
    };

    for step in &plan.steps {
        let mut frontmatter = darkmatter::markdown::Frontmatter::new();
        frontmatter
            .insert("$schema", schema.clone())
            .map_err(|e| load_failure(SequenceLoadCause::Frontmatter(Box::new(e))))?;
        let Value::Object(state) = step.state.to_value() else {
            unreachable!("StepState::to_value always produces an object");
        };
        for (key, value) in state {
            frontmatter
                .insert(&key, value)
                .map_err(|e| load_failure(SequenceLoadCause::Frontmatter(Box::new(e))))?;
        }

        let markdown = darkmatter::markdown::Markdown::with_frontmatter(frontmatter, "");
        let report = schemas
            .validate(&markdown)
            .map_err(|e| load_failure(SequenceLoadCause::Schema(Box::new(e))))?;

        if let Some(problem) = report.problems.first() {
            // `property` is set only for missing-required failures; every other
            // kind locates itself with the instance pointer instead.
            let property = problem
                .property
                .clone()
                .unwrap_or_else(|| problem.path.clone());
            return Err(CompositionError::SequenceStateSchemaViolation {
                index: step.index,
                id: step.state.id.clone(),
                property,
                message: problem.message.clone(),
            });
        }
    }

    Ok(())
}
