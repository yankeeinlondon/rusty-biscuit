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

use biscuit_file::{FileReference, FileResolutionContext};
use serde_json::{Map, Value};

use super::super::error::{CompositionError, SequenceLoadCause};
use super::data::{self, SourceFormat};
use super::expr::SourceExpressionLookup;
use super::formal;
use super::grammar::{SequenceReference, SourceOperator};
use super::model::{SequencePlan, SequenceSource};
use super::normalize::normalize_plan;

/// Resolve an external sequence reference string to an absolute existing path.
///
/// Delegates all grammar and candidate ordering to [`FileReference`] and the
/// shared [`FileResolutionContext`] (D5/D11): implicit refs probe the source
/// directory before repository scopes; explicit refs pin to the source
/// directory; `@`/`^`/`vault:`/`~`/absolute keep their usual meanings. The
/// reference is authored inside the composition source, so the source
/// document's directory is the base and the launch directory is never a
/// fallback here.
pub fn resolve_sequence_reference(raw: &str, source_path: &Path) -> Result<PathBuf, CompositionError> {
    let file_ref =
        FileReference::new(raw).map_err(|e| CompositionError::SequenceExternalLoad {
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

/// Snapshot-preserving external sequence resolver.
pub fn resolve_sequence_reference_in_context(
    raw: &str,
    source_path: &Path,
    request_context: &FileResolutionContext,
) -> Result<PathBuf, CompositionError> {
    let file_ref = FileReference::new(raw).map_err(|e| {
        CompositionError::SequenceExternalLoad {
            context: format!("`{raw}`"),
            source: e.into(),
        }
    })?;
    file_ref
        .resolve_in_context(&request_context.for_source(source_path))
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
/// The worktree root and package area are discovered once here (via `sniff`)
/// and passed in, so `FileReference` resolution never re-probes repository
/// topology per candidate.
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

    if file_ref.class().kind == biscuit_file::FileReferenceKind::RepositoryScoped
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
    let items = select_items(&document, reference, formal)?;

    let items = match &reference.operator {
        Some(operator) => apply_source_operator(items, operator, frontmatter, invocation_path)?,
        None => items,
    };

    if formal {
        let root = document.as_object().expect("formal implies an object root");
        return formal::normalize_formal_plan(
            items,
            formal::formal_keys(root)?,
            SequenceSource::External {
                path: path.to_path_buf(),
            },
            invocation_path,
            path,
            frontmatter,
            document_fail_fast,
        );
    }

    normalize_plan(
        &items,
        SequenceSource::DataFile {
            path: path.to_path_buf(),
        },
        invocation_path,
        document_fail_fast,
    )
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

/// Narrow a loaded document to the item list the plan is built from.
fn select_items(
    document: &Value,
    reference: &SequenceReference,
    formal: bool,
) -> Result<Vec<Value>, CompositionError> {
    if let Some(offset) = &reference.offset {
        let node = data::apply_offset(document, offset)?;
        return data::expect_list(node, Some(offset));
    }

    let Some(root) = document.as_object() else {
        // A root array is the ordinary shape for JSON/JSONL data files.
        return data::expect_list(document, None);
    };

    if formal {
        let list = root.get("sequence").expect("formal implies `sequence`");
        return data::expect_list(list, Some("sequence"));
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
