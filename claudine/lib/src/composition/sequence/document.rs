//! The document that owns a sequence's step list, and the values it fills
//! that list with.
//!
//! A sequence document has two views. Its *authored* view keeps the
//! `$state`/`template` declarations, its own and those of every inline
//! container beneath it, and is what the plan is built from. Its
//! *composition* view
//! ([`without_formal_sequence_keys`](crate::composition::without_formal_sequence_keys))
//! drops them, and is what Darkmatter composes. An owner's `template:` is
//! filled against the document's *prepared* values: its own properties under
//! the caller's setters, coerced and defaulted by its `$schema` exactly as its
//! full preparation would, but read from the frontmatter alone, with no shell
//! run and no body read. [`prepare_sequence_document_values`] produces them.
//!
//! [`SequenceDocumentScope`] records which whole document declared a plan, so
//! a root `sequence: child.yaml` is judged by `child.yaml`, never by the
//! invoking file that merely names it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Map, Value};

use super::super::error::{CompositionError, DroppedOptional};
use super::super::prepare::PrepareOptions;
use super::super::types::{CallerInputLayers, CompositionMode, LaunchSchema, ResolvedCompositionSource};

/// The whole sequence document that declared a plan's step list.
///
/// Carried on [`SequencePlan::document_scope`](super::SequencePlan::document_scope)
/// for a list read from a whole document: the invoked file, or a whole file a
/// root `sequence:` references. A list selected out of a data file (an offset
/// or operator) has none, and neither does a dynamic source.
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceDocumentScope {
    /// The file whose root declares the list.
    pub declaring_path: PathBuf,
    /// The list's property path within that file.
    pub property: String,
    /// The file's root as authored, declarations included.
    pub authored: Arc<Map<String, Value>>,
    /// The values the document's `template:` expressions read below each
    /// item's own fields: its prepared values when the caller supplied a
    /// captured context, else its authored values under the caller's setters.
    pub values: Arc<Map<String, Value>>,
}

/// A sequence document's prepared values, with no prompt: nothing launches
/// from it.
#[derive(Debug, Clone)]
pub struct SequenceDocumentPreparation {
    /// The document the values belong to.
    pub resolved_path: PathBuf,
    /// The composed frontmatter: authored properties under the caller's
    /// layers, after schema coercion, defaults, and interpolation. A value a
    /// frontmatter `$( … )` would produce keeps its authored text.
    pub values: Map<String, Value>,
    /// Whether the schema verdict was withheld
    /// ([`PrepareOptions::defer_schema_verdict`]), as for a bootstrap read.
    /// The values are coerced either way.
    pub schema_verdict_deferred: bool,
    /// The document's resolved `$schema` and, unless the verdict was
    /// withheld, its launch report.
    pub launch_schema: Option<LaunchSchema>,
    /// Invalid optional values the verdict dropped from [`Self::values`].
    pub dropped_optionals: Vec<DroppedOptional>,
    /// The caller layers the values were prepared under, with their records.
    pub input_layers: CallerInputLayers,
    /// The snapshot the values' `ctx.*` reads resolved against.
    pub compose_context: darkmatter::markdown::compose::ComposeContext,
}

/// Prepare the values of the sequence document `source` under `options`.
///
/// This is the canonical preparation of `mode` narrowed to the document's
/// frontmatter, in its composition view: the body is never read, no shell
/// runs (frontmatter `$( … )` expansion is disabled, not refused), and
/// nothing is launched. Caller setters keep their recorded origins, supplied
/// values are converted (`file(eager)`, `number`, …) before a derived
/// property reads them, and defaults apply. With
/// [`PrepareOptions::defer_schema_verdict`] the verdict is withheld, as a
/// bootstrap read withholds it; otherwise a missing or invalid document input
/// is the same typed error a full preparation reports.
///
/// ## Errors
///
/// The frontmatter-side errors of a full preparation:
/// [`CompositionError::MissingProperties`],
/// [`CompositionError::SchemaValidation`], [`CompositionError::SchemaLoad`],
/// and composition failures.
pub fn prepare_sequence_document_values(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: CompositionMode,
) -> Result<SequenceDocumentPreparation, CompositionError> {
    let view = super::super::resolve::composition_view(source);
    let deferred = options.defer_schema_verdict;
    let prepared = if deferred {
        super::super::prepare::prepare_values_on(&view, options, mode)?
    } else {
        super::super::schema::prepare_values_with_schema(&view, options, mode)?
    };
    let values = match prepared.effective_frontmatter {
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value)))
            .collect(),
        _ => Map::new(),
    };
    Ok(SequenceDocumentPreparation {
        resolved_path: prepared.resolved_path,
        values,
        schema_verdict_deferred: deferred,
        launch_schema: prepared.launch_schema,
        dropped_optionals: prepared.dropped_optionals,
        input_layers: prepared.input_layers,
        compose_context: prepared.compose_context,
    })
}

/// The values an owner's `template:` reads below each item's own fields.
///
/// `authored` is the owner's root as authored (literal tokens decoded) and
/// `keys` its declarations. When the template has an expression to fill and
/// `options` carries the invocation's captured context, these are the owner's
/// prepared values ([`prepare_sequence_document_values`], verdict withheld:
/// the owner's inputs are judged at its own preparation, not here), prepared
/// from `source`, which is only loaded then. Otherwise they are `authored`
/// under the caller's setters, by top-level key.
///
/// `referenced` is the path of a referenced owner (`None` for the invoked
/// document). With an invocation in `options`, a referenced owner's authored
/// values resolve from its own folder, through the context the invocation
/// derives for it; the caller's setters keep their recorded origins.
///
/// ## Errors
///
/// `source`'s load error, a referenced owner's context that cannot be built,
/// and the preparation's composition errors.
pub(crate) fn owner_values<'s>(
    authored: &Map<String, Value>,
    keys: &super::formal::FormalKeys<'_>,
    options: &super::SequenceSourceOptions<'_>,
    referenced: Option<&Path>,
    source: impl FnOnce() -> Result<std::borrow::Cow<'s, ResolvedCompositionSource>, CompositionError>,
) -> Result<Map<String, Value>, CompositionError> {
    if let Some(context) = options.compose_context
        && super::formal::template_reads_values(keys)
    {
        let mut prepare = PrepareOptions::new(options.file_resolution_context.clone());
        if let Some(layers) = options.caller_layers {
            prepare = layers.apply_to(prepare);
        }
        if let (Some(invocation), Some(path)) = (options.invocation, referenced) {
            prepare = prepare.resolving_document_from(owner_resolution(invocation, path)?);
        }
        prepare.prepared_context = Some(context.clone());
        prepare.defer_schema_verdict = true;
        let source = source()?;
        return Ok(prepare_sequence_document_values(&source, prepare, CompositionMode::ChainedDocument)?.values);
    }
    let mut values = authored.clone();
    if let Some(Value::Object(setters)) = options.caller_layers.and_then(|layers| layers.set_overrides.as_ref()) {
        values.extend(setters.iter().map(|(key, value)| (key.clone(), value.clone())));
    }
    Ok(values)
}

/// The file-resolution context of the referenced owner at `path`, derived as
/// the graph walk derives each origin document's.
fn owner_resolution(
    invocation: &crate::invocation_context::InvocationContext,
    path: &Path,
) -> Result<biscuit_file::FileResolutionContext, CompositionError> {
    use crate::invocation_context::InvocationContextError;
    let path = biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf());
    match invocation.derive_source(&path) {
        Ok(context) => Ok(context.file_resolution_context().clone()),
        Err(InvocationContextError::ResolutionContext(error)) => Err(CompositionError::ResolutionContext(error)),
        Err(InvocationContextError::SourceWithoutParent(path)) => Err(CompositionError::SequenceInvalid(
            format!("referenced sequence document '{}' has no parent directory", path.display()),
        )),
    }
}

#[cfg(test)]
mod tests;
