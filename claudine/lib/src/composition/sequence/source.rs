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
use std::sync::Arc;

use biscuit_file::{FileReference, FileResolutionContext};
use serde_json::{Map, Value};

use super::super::authored_order::AuthoredOrder;
use super::super::error::{CompositionError, SequenceLoadCause, SequenceSite};
use super::data::{self, SourceFormat};
use super::expr::{DocumentFrontmatter, SourceExpressionLookup};
use super::formal;
use super::grammar::{SequenceReference, SourceOperator};
use super::model::{ExecutableField, SequencePlan, SequenceSource};
use super::SequenceSourceOptions;
use super::normalize::normalize_plan_at;

/// Resolve an external sequence reference string to an absolute existing path.
///
/// Delegates all grammar and candidate ordering to [`FileReference`] and the
/// shared [`FileResolutionContext`] (D5/D11): implicit refs probe the source
/// directory before repository scopes; explicit refs pin to the source
/// directory; `@`/`&`/`^`/`vault:`/`~`/absolute keep their usual meanings. The
/// reference is authored inside the composition source, so the source
/// document's directory is the base and the launch directory is never a
/// fallback here. The source's context is derived from the request's
/// (`for_source`), so repository anchors come from the request, not from a
/// fresh discovery.
///
/// ## Errors
///
/// [`CompositionError::SequenceReferenceResolution`] naming `field`'s
/// construct, with the resolver's probe record once the reference parses.
/// Its site is unlocated: the caller knows the property that authored the
/// reference ([`CompositionError::locate_sequence`]).
pub fn resolve_sequence_reference(
    raw: &str,
    field: ExecutableField,
    source_path: &Path,
    request_context: &FileResolutionContext,
) -> Result<PathBuf, CompositionError> {
    let failure = |file_ref: Option<&FileReference>, source: SequenceLoadCause| {
        // The probe record is rebuilt only for a failure, so a resolving
        // reference costs one resolution.
        let resolution = file_ref.map(|file_ref| {
            let detailed = file_ref.resolve_detailed(&request_context.for_source(source_path));
            Box::new(crate::harness::ResolutionDetail::from_detailed(&detailed))
        });
        CompositionError::SequenceReferenceResolution {
            field,
            reference: raw.to_string(),
            resolution,
            site: Box::new(SequenceSite::unlocated()),
            source,
        }
    };
    let file_ref = FileReference::new(raw).map_err(|e| failure(None, e.into()))?;
    match file_ref.resolve_in_context(&request_context.for_source(source_path)) {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(failure(Some(&file_ref), SequenceLoadCause::NotFound {
            glob_hint: biscuit_file::ResolutionFailure::NoMatch.glob_hint(raw),
        })),
        Err(error) => Err(failure(Some(&file_ref), error.into())),
    }
}

/// The [`missing_document`] finding for `raw`, authored in `source_path`,
/// when [`resolve_sequence_reference`] would fail because no candidate
/// exists: what a check reports in place of the typed failure. Any other
/// failure is `None`; the walk keeps its typed error. Its hint
/// names paths relative to `launch_dir`, and its base is `source_path`, the
/// file the reference is written relative to.
///
/// [`missing_document`]: darkmatter::findings::reference::missing_document
pub(crate) fn reference_finding(
    raw: &str,
    source_path: &Path,
    request_context: &FileResolutionContext,
    construct: darkmatter::findings::reference::DocumentConstruct<'_>,
    memo: &mut darkmatter::findings::reference::ListingMemo,
) -> Option<darkmatter::findings::Diagnostic> {
    use darkmatter::findings::reference::{DocumentCheck, check_document};
    let reference = FileReference::new(raw).ok()?;
    // Darkmatter's `file-read` permission governs schema slot reads, not
    // the documents a sequence names, so every location is admitted.
    let admit_all = |_: &Path| Ok::<(), std::convert::Infallible>(());
    let mut finding = match check_document(&reference, &request_context.for_source(source_path), construct, memo, admit_all) {
        DocumentCheck::Missing(finding) => *finding,
        DocumentCheck::Refused(never) => match never {},
        DocumentCheck::Found(_) | DocumentCheck::Failed(_) => return None,
    };
    if let Some(details) = finding.reference_details.as_mut() {
        details.base = Some(biscuit_file::to_forward_slash_string(source_path));
    }
    Some(finding)
}

/// In a sequence check, why the reference `raw`, authored in `source_path`,
/// that resolves to no file is not checked instead of missing: a candidate
/// it would resolve to is a file the run creates first, which the check's
/// passive session declares
/// ([`PassiveSession::defer_created`](darkmatter::markdown::compose::passive::PassiveSession::defer_created)).
/// `None` outside a check and for every other reference.
pub(crate) fn created_reference(
    raw: &str,
    source_path: &Path,
    request_context: &FileResolutionContext,
) -> Option<darkmatter::markdown::compose::passive::DeferralReason> {
    let session = crate::composition::passive_scope::current()?;
    let reference = FileReference::new(raw).ok()?;
    let detailed = reference.resolve_detailed(&request_context.for_source(source_path));
    session.created_among(detailed.probed_paths())
}

/// In a sequence check, why the existing file `path` a reference resolved
/// to is not read now: the run's root initializer appends to it first
/// ([`PassiveSession::initializer_changes`](darkmatter::markdown::compose::passive::PassiveSession::initializer_changes)),
/// so its rows, or its text, are known only at launch. `None` outside a
/// check and for every other file.
pub(crate) fn changed_reference(path: &Path) -> Option<darkmatter::markdown::compose::passive::DeferralReason> {
    crate::composition::passive_scope::current()?.initializer_changes(path)
}

/// The failure a check records for a file [`changed_reference`] withholds;
/// the walk reports the deferral beside it, never this error.
pub(crate) fn changed_reference_failure(path: &Path) -> CompositionError {
    CompositionError::sequence_invalid(
        "the run's initializer writes this file before it is read",
        crate::composition::SequenceSite::file(path),
    )
}

/// Load a referenced sequence source and normalize it into a plan.
///
/// `invocation_path` is the resolved path of the document that referenced this
/// file; it seeds the `sequence_id` payload so the token stays keyed to the
/// invocation, not the data file. `frontmatter` is that document's frontmatter,
/// which a `::map`/`::name`/`::template` operator reads below each item. A
/// whole referenced document is its own owner: its `template:` reads its own
/// values under the caller's inputs in `options`, never `frontmatter`, so a
/// key neither defines stays undefined.
///
/// ## Errors
///
/// The load, selection, and normalization errors of the source.
pub fn load_referenced_sequence(
    reference: &SequenceReference,
    path: &Path,
    invocation_path: &Path,
    frontmatter: &dyn DocumentFrontmatter,
    document_fail_fast: bool,
    options: &SequenceSourceOptions<'_>,
) -> Result<SequencePlan, CompositionError> {
    let loaded = data::load_document(path)?;
    plan_referenced_sequence(
        reference, path, &loaded.value, Arc::new(loaded.orders), invocation_path,
        frontmatter, document_fail_fast, options,
    )
}

/// [`load_referenced_sequence`] over a `document` (and its authored `orders`)
/// the caller already loaded from `path`; only the selected items are copied.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_referenced_sequence(
    reference: &SequenceReference,
    path: &Path,
    document: &Value,
    orders: Arc<darkmatter::markdown::MappingOrders>,
    invocation_path: &Path,
    frontmatter: &dyn DocumentFrontmatter,
    document_fail_fast: bool,
    options: &SequenceSourceOptions<'_>,
) -> Result<SequencePlan, CompositionError> {
    plan_referenced(reference, path, document, orders, invocation_path, frontmatter, document_fail_fast, options, false)
        .map(|(recovered, _)| recovered.plan)
}

/// [`plan_referenced_sequence`] that sets an unreadable item aside instead
/// of failing the list, for a check that reports every item. Also returns
/// the selected items as authored, which the failures index.
///
/// ## Errors
///
/// As [`plan_referenced_sequence`], less the per-item failures.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_referenced_sequence_recovering(
    reference: &SequenceReference,
    path: &Path,
    document: &Value,
    orders: Arc<darkmatter::markdown::MappingOrders>,
    invocation_path: &Path,
    frontmatter: &dyn DocumentFrontmatter,
    document_fail_fast: bool,
    options: &SequenceSourceOptions<'_>,
) -> Result<(super::normalize::RecoveredPlan, Vec<Value>), CompositionError> {
    plan_referenced(reference, path, document, orders, invocation_path, frontmatter, document_fail_fast, options, true)
}

#[allow(clippy::too_many_arguments)]
fn plan_referenced(
    reference: &SequenceReference,
    path: &Path,
    document: &Value,
    orders: Arc<darkmatter::markdown::MappingOrders>,
    invocation_path: &Path,
    frontmatter: &dyn DocumentFrontmatter,
    document_fail_fast: bool,
    options: &SequenceSourceOptions<'_>,
    recover: bool,
) -> Result<(super::normalize::RecoveredPlan, Vec<Value>), CompositionError> {
    let file_resolution_context = options.file_resolution_context();
    let format = SourceFormat::for_path(path);

    if let Some(offset) = &reference.offset
        && format.is_line_delimited()
    {
        let _ = offset;
        return Err(CompositionError::SequenceOffsetUnsupported {
            path: path.to_path_buf(),
            format: line_delimited_label(path),
            site: Box::new(SequenceSite::file(path)),
        });
    }

    let formal = is_formal_sequence(document, reference);
    let list_property = reference.offset.as_deref().unwrap_or("");
    // The data layer does not know the file it reads items from.
    let located = |error: CompositionError| error.locate_sequence(path, list_property, None);
    let items = select_items(document, reference, formal, path).map_err(located)?;

    let items = match &reference.operator {
        Some(operator) => apply_source_operator(
            items,
            operator,
            frontmatter,
            invocation_path,
            file_resolution_context,
        )
        .map_err(located)?,
        None => items,
    };

    if formal {
        let root = document.as_object().expect("formal implies an object root");
        if let Some(value) = root.get("fail_fast") {
            return Err(super::removed::removed_key(super::removed::RemovedOwner::Root, "fail_fast", value, path));
        }
        super::grammar::validate_sequence_root(root, path)?;
        let authored: Arc<Map<String, Value>> = Arc::new(
            root.iter().map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value))).collect(),
        );
        let keys = formal::formal_keys(root, path)?;
        let own = super::document::owner_values(&authored, &keys, options, Some(path), || {
            whole_document_source(path, root, &orders).map(std::borrow::Cow::Owned)
        })?;
        // Only the document's own values and the caller's inputs: the
        // referrer's `frontmatter` never enters them (it serves the data
        // operators above), so a key neither defines stays undefined.
        let values = Arc::new(own);
        let owner = formal::FormalOwner { keys, declaring_path: path, list_property: "sequence", values: &*values };
        let source = SequenceSource::External { path: path.to_path_buf() };
        let authored_items = if recover { items.clone() } else { Vec::new() };
        let mut recovered = if recover {
            let state_schema = formal::prepare_state_schema(&owner, file_resolution_context)?;
            formal::normalize_formal_plan_recovering(
                items, owner, state_schema.as_ref(), source, invocation_path, document_fail_fast, file_resolution_context,
            )?
        } else {
            let plan = formal::normalize_formal_plan(
                items, owner, source, invocation_path, document_fail_fast, file_resolution_context,
            )?;
            super::normalize::RecoveredPlan::whole(plan)
        };
        let plan = &mut recovered.plan;
        // The steps are the referenced document's own `sequence:` list, read
        // without an offset or operator, so its authored key order applies
        // element for element. Data read through either of those is reshaped
        // before it becomes a step list and keeps canonical order instead.
        plan.authored = AuthoredOrder::new(orders, "/sequence");
        plan.document_scope = Some(Arc::new(super::document::SequenceDocumentScope {
            declaring_path: path.to_path_buf(),
            property: "sequence".into(),
            authored,
            values,
        }));
        return Ok((recovered, authored_items));
    }

    let source = SequenceSource::DataFile { path: path.to_path_buf(), list: list_property.to_string() };
    if recover {
        let recovered = super::normalize::normalize_plan_recovering_at(
            &items, source, invocation_path, document_fail_fast, list_property,
        );
        return Ok((recovered, items));
    }
    let plan = normalize_plan_at(&items, source, invocation_path, document_fail_fast, list_property)?;
    Ok((super::normalize::RecoveredPlan::whole(plan), Vec::new()))
}

/// The whole document at `path`, whose parsed `root` (and authored `orders`)
/// the caller holds, as a composition source for its owner preparation.
///
/// A Markdown document is reread, so its frontmatter keeps its own positions;
/// its body is never composed for values. A data document's root becomes the
/// frontmatter of a document with no body, as a directly invoked YAML file's
/// does ([`crate::composition::load_yaml_document`]).
pub(crate) fn whole_document_source(
    path: &Path,
    root: &Map<String, Value>,
    orders: &darkmatter::markdown::MappingOrders,
) -> Result<crate::composition::ResolvedCompositionSource, CompositionError> {
    let load_error = |source| CompositionError::MarkdownLoad {
        path: path.to_path_buf(),
        source: super::super::error::MarkdownLoadCause::Parse(Box::new(source)),
    };
    let markdown = if data::is_markdown(path) {
        darkmatter::markdown::Markdown::try_from(path).map_err(load_error)?
    } else {
        let mut frontmatter = darkmatter::markdown::Frontmatter::new();
        for (key, value) in root {
            frontmatter.insert(key, value.clone()).map_err(load_error)?;
        }
        darkmatter::markdown::Markdown::with_frontmatter(frontmatter.with_mapping_orders(orders.clone()), "")
    };
    Ok(crate::composition::ResolvedCompositionSource {
        original_ref: path.to_string_lossy().into_owned(),
        resolved_path: path.to_path_buf(),
        original_text: markdown.loaded_text().map(str::to_owned).unwrap_or_default(),
        markdown,
    })
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
    path: &Path,
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
        return Err(CompositionError::SequenceExternalWrongType {
            path: path.to_path_buf(),
            problem: "the `list:` sequence shape has been removed; rename the property to `sequence:` \
             (the same shape now works both when referenced and when invoked directly)"
                .to_string(),
            site: Box::new(SequenceSite::at(path, "list")),
        });
    }

    Err(CompositionError::SequenceExternalWrongType {
        path: path.to_path_buf(),
        problem: "it must have a root `sequence:` list, be a list at its root, or be \
         reached with a `-> offset` path"
            .to_string(),
        site: Box::new(SequenceSite::file(path)),
    })
}

/// Apply a `::map`/`::name`/`::template` operator to every item.
fn apply_source_operator(
    items: Vec<Value>,
    operator: &SourceOperator,
    frontmatter: &dyn DocumentFrontmatter,
    invocation_path: &Path,
    file_resolution_context: &FileResolutionContext,
) -> Result<Vec<Value>, CompositionError> {
    data::apply_operator(items, operator, &|expression, item| {
        let lookup = SourceExpressionLookup::new(frontmatter, file_resolution_context, invocation_path)
            .with_item(item);
        super::expr::evaluate_whole(expression, &lookup)
    })
}
