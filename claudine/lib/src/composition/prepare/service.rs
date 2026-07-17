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
use super::{PrepareOptions, prepare_direct_with_prompt, prepare_inline};
use crate::composition::error::CompositionError;
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
/// ## Errors
///
/// Propagates the composer's typed [`CompositionError`] — a compose failure,
/// a shell-expansion denial, a removed validation key, an empty composed body,
/// or a lifecycle parse/pre-flight failure.
pub fn prepare_document(
    request: DocumentPreparation<'_>,
) -> Result<PreparedComposition, CompositionError> {
    let DocumentPreparation {
        entry,
        mode,
        source,
        prompt_source,
        options,
    } = request;

    let mut prepared = match mode {
        CompositionMode::ChainedDocument => {
            prepare_direct_with_prompt(source, options, prompt_source)?
        }
        CompositionMode::InlineFrontmatterPrompt => prepare_inline(source, options)?,
    };
    prepared.entry = entry;
    Ok(prepared)
}
