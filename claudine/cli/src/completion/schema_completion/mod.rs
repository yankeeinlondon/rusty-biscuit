//! Schema-aware completion for composition `name=value` setters.
//!
//! Phase 4 of the `2026-05-15-schemas` feature. When the cursor sits in a
//! setter slot of `claudine compose`, `claudine inline-compose`, or
//! `claudine sequence` AND a positional prompt-file argument is already
//! committed, this module consults the prompt's `$schema` declaration via
//! Darkmatter to produce:
//!
//! - **Property names** (before `=`) — required properties first, then
//!   optional, in declaration order. Already-supplied names are filtered out.
//!   See [`keys`].
//! - **Property values** (after `=`) — enum members for `enum` properties,
//!   filesystem paths constrained by `match(...)` globs for `file` properties.
//!   See [`candidates`].
//!
//! All entry points are pure functions over schema state plus filesystem
//! reads — no shell execution, no provider launches. When the schema cannot
//! be resolved or the file_arg cannot be loaded, every function returns an
//! empty `Vec`, signalling to the caller that the shell's native fallback
//! should take over.

use std::path::PathBuf;

use biscuit_file::{FileReference, FileResolutionContext};
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use darkmatter::markdown::Markdown;

use super::scopes::{self, ScopeContext};

mod candidates;
mod keys;
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

pub(crate) use candidates::{file_candidate_paths, property_value};
pub(crate) use keys::{declared_property_order, property_names};

/// Resolve `file_arg` to a loaded [`EffectiveSchema`].
///
/// Returns `None` when:
/// - the committed prompt reference does not resolve to a file,
/// - the file is not valid Markdown,
/// - the document has no `$schema`,
/// - the schema cannot be parsed or compiled.
///
/// All failure modes are silent: completion is best-effort and falls back to
/// shell-native behavior on any error.
pub(crate) fn load_effective_schema(file_arg: &str, ctx: &ScopeContext) -> Option<EffectiveSchema> {
    let prompt = CommittedPrompt::resolve(file_arg, ctx)?;
    let text = std::fs::read_to_string(&prompt.path).ok()?;
    let markdown: Markdown =
        Markdown::from(text).with_source(ComposeSource::infer_from_path(&prompt.path));
    DarkmatterSchemas::new(prompt.context).effective_for(&markdown).ok()?
}

/// The prompt-file argument already committed in argv, resolved the way
/// composition resolves it.
pub(super) struct CommittedPrompt {
    pub(super) path: PathBuf,
    /// The completion context derived for the prompt document, so the
    /// document's own references (`$schema` files, `file` values) resolve as
    /// they will when the prompt is composed.
    pub(super) context: FileResolutionContext,
}

impl CommittedPrompt {
    /// Resolve `file_arg` through the shared file-reference grammar and the
    /// completion request's context.
    ///
    /// The argument keeps whatever form the user committed, including the
    /// `@`/`&`/`^`/`~/` tokens the positional completer emits, so a prefixed
    /// prompt and an explicit `./` keep their composition semantics: `./`
    /// never falls back to the repository root.
    pub(super) fn resolve(file_arg: &str, ctx: &ScopeContext) -> Option<Self> {
        let trimmed = file_arg.trim_matches(['"', '\'']);
        if trimmed.is_empty() {
            return None;
        }
        let reference = FileReference::new(trimmed).ok()?;
        let request = scopes::file_resolution_context(ctx).ok()?;
        let path = reference.resolve_in_context(&request).ok()??;
        if !path.is_file() {
            return None;
        }
        let path = in_repository_spelling(&request, path);
        let ordinary = request.for_source_reference(&reference, &path);
        if ordinary.validate().is_ok() && request.repository_root().is_some() {
            return Some(Self { path, context: ordinary });
        }
        // Outside the launch repository (or launched from none) the prompt
        // gets composition's source derivation: its own repository anchors
        // `&`, `^`, and bare references, while `@` keeps the launch scope.
        let context = claudine::composition::derive_request_context_for_source(&request, &path)
            .ok()?
            .for_source_reference(&reference, &path);
        Some(Self { path, context })
    }
}

/// `path` spelled under the request's repository root when it names a file
/// inside that repository through another spelling (`/var` against
/// `/private/var`, a symlinked checkout).
///
/// Composition re-derives a source's repository by canonical directory; a
/// derived context only keeps its repository for a lexically contained source,
/// so without this an absolute prompt spelled differently would lose its
/// repository-relative `$schema` references.
fn in_repository_spelling(request: &FileResolutionContext, path: PathBuf) -> PathBuf {
    let Some(root) = request.repository_root() else {
        return path;
    };
    if path.starts_with(root) {
        return path;
    }
    let (Ok(canonical_root), Ok(canonical_path)) = (root.canonicalize(), path.canonicalize())
    else {
        return path;
    };
    match canonical_path.strip_prefix(&canonical_root) {
        Ok(relative) => root.join(relative),
        Err(_) => path,
    }
}

#[cfg(test)]
fn ordered_completable_suggestions(
    effective: &EffectiveSchema,
) -> Vec<darkmatter::markdown::schemas::CompletionSuggestion> {
    use darkmatter::markdown::schemas::completion as dm_completion;
    dm_completion::completable_properties(effective)
        .into_iter()
        .filter_map(|name| dm_completion::for_property(effective, &name))
        .collect()
}
