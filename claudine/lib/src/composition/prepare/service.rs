//! The canonical document-preparation service.
//!
//! One service prepares every document, whatever brought it here. Before this
//! existed the harness re-composition path (`harness_orch::prompt`) hand-rolled
//! its own compose options, so a proxied or retried document was composed by
//! different code than the direct document it was supposed to be equivalent to
//! — and the two drifted. R1 bans that second composer; this module is the one
//! that replaces it.
//!
//! The stages are explicit rather than implied: [`DocumentEntryReason`] selects
//! a row of the stage matrix, and the prepared document carries that row so no
//! downstream layer re-decides it.

#[cfg(test)]
mod tests;

use super::entry::DocumentEntryReason;
use super::PrepareOptions;
use crate::composition::error::CompositionError;
use crate::composition::schema::{prepare_direct_with_schema_and_prompt, prepare_inline_with_schema};
use crate::composition::types::{CompositionMode, PreparedComposition, ResolvedCompositionSource};

/// Where the delivered prompt text comes from.
#[derive(Debug, Clone)]
pub enum PromptSource {
    /// The composed body is the prompt. Every composition command, and every
    /// re-preparation of one.
    ComposedBody,
    /// The caller supplies the prompt; the document is composed only for its
    /// effective frontmatter.
    ///
    /// This is the direct-wrapper passthrough case: the prompt came from argv
    /// or stdin and the document is a provider memory file (`CLAUDE.md` and
    /// friends), whose body is context rather than the request. The composed
    /// body is therefore not checked for emptiness — it is not the prompt.
    Supplied(String),
}

/// Whether this preparation owns the document's schema verdict.
///
/// The verdict is a *stage*, not a property of the entry reason: a proxied
/// target is prepared twice — once before its `initialize` (which may add or
/// repair a schema property) and once after — and only the second read judges
/// it. R4 fixes that order; putting the choice here is what keeps it out of the
/// callers' hands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaStage {
    /// Reach the verdict: validate, categorize typed schema errors, and apply
    /// the invalid-optional drop-and-retry.
    Validate,
    /// Withhold the verdict for the post-`initialize` stabilized reread.
    ///
    /// Frontmatter is still coerced to its declared types; a violation is
    /// simply not this read's failure to report.
    DeferToStabilizedReread,
}

/// One request to prepare one document.
#[derive(Debug)]
pub struct DocumentPreparation<'a> {
    /// Why this document is being prepared. Selects the stage row.
    pub entry: DocumentEntryReason,
    /// Which composer runs.
    pub mode: CompositionMode,
    /// The resolved, loaded document, with any caller overlay already merged
    /// into its authored frontmatter.
    pub source: &'a ResolvedCompositionSource,
    /// Where the prompt text comes from.
    pub prompt_source: PromptSource,
    /// Whether this read owns the document's schema verdict.
    pub schema: SchemaStage,
    /// The assembled input layers plus this document's target-specific context.
    pub options: PrepareOptions,
}

/// Prepare one document canonically.
///
/// Direct, proxied, retried, resumed, and loop-refreshed documents all arrive
/// here. Given the same resolved source and the same assembled input layers,
/// every entry produces a semantically equivalent [`PreparedComposition`];
/// what the entry reason changes is which *stages around* preparation run, and
/// that is recorded on the result rather than re-derived downstream.
///
/// Preparation runs through the schema layer, so a `$schema`-declaring
/// document reaches the same typed [`CompositionError::SchemaValidation`] /
/// [`CompositionError::MissingProperties`] categorization — and the same
/// invalid-optional drop-and-retry — on every route. The `compose` and
/// `sequence` routes already composed through it; the harness route did not,
/// so a proxied or retried target surfaced an uncategorized
/// `ComposeFailed(SchemaValidationFailed)` where the identical document
/// invoked directly surfaced the typed variant. That is cross-route typed
/// identity, which `features/2026-07-13-proxy-with` Phase 12 owns.
///
/// ## Errors
///
/// Propagates the composer's typed [`CompositionError`] — a compose failure,
/// a schema load/parse/validation failure, a shell-expansion denial, a removed
/// validation key, an empty composed body, or a lifecycle parse/pre-flight
/// failure.
pub fn prepare_document(
    request: DocumentPreparation<'_>,
) -> Result<PreparedComposition, CompositionError> {
    let DocumentPreparation {
        entry,
        mode,
        source,
        prompt_source,
        schema,
        mut options,
    } = request;

    // A deferred read withholds the verdict at both layers that can reach one:
    // Darkmatter's compose-time stage (via the option) and this crate's
    // post-shell re-validation (by taking the schema-free composer). Skipping
    // only one of the two would still fail the document before its `initialize`.
    let mut prepared = match (schema, mode) {
        (SchemaStage::Validate, CompositionMode::ChainedDocument) => {
            prepare_direct_with_schema_and_prompt(source, options, prompt_source)?
        }
        (SchemaStage::Validate, CompositionMode::InlineFrontmatterPrompt) => {
            prepare_inline_with_schema(source, options)?
        }
        (SchemaStage::DeferToStabilizedReread, CompositionMode::ChainedDocument) => {
            options.defer_schema_verdict = true;
            super::prepare_direct_with_prompt(source, options, prompt_source)?
        }
        (SchemaStage::DeferToStabilizedReread, CompositionMode::InlineFrontmatterPrompt) => {
            options.defer_schema_verdict = true;
            super::prepare_inline(source, options)?
        }
    };
    prepared.entry = entry;
    prepared.schema_verdict_deferred = schema == SchemaStage::DeferToStabilizedReread;
    Ok(prepared)
}
