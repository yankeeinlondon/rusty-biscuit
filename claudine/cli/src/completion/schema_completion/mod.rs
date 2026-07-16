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

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use darkmatter::markdown::Markdown;

use super::scopes::{self, ScopeContext};

mod candidates;
mod keys;
#[cfg(test)]
mod tests;

pub(crate) use candidates::{file_candidate_paths, property_value};
pub(crate) use keys::{declared_property_order, property_names};

/// Resolve `file_arg` to a loaded [`EffectiveSchema`].
///
/// Returns `None` when:
/// - the file cannot be located on disk,
/// - the file is not valid Markdown,
/// - the document has no `$schema`,
/// - the schema cannot be parsed or compiled.
///
/// All failure modes are silent: completion is best-effort and falls back to
/// shell-native behavior on any error.
pub(crate) fn load_effective_schema(file_arg: &str, ctx: &ScopeContext) -> Option<EffectiveSchema> {
    let path = resolve_prompt_path(file_arg, ctx)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let markdown: Markdown = Markdown::from(text).with_source(ComposeSource::infer_from_path(&path));
    // Completions lack a launch-area context, so file-typed values fall back
    // to ambient CWD here (consistent with pre-`chdir` timing).
    DarkmatterSchemas::new().effective_for(&markdown).ok()?
}

/// Resolve a setter-token file_arg to an absolute path on disk.
///
/// Tries the following interpretations in order:
/// 1. As-is when absolute.
/// 2. Relative to cwd.
/// 3. Relative to the effective repo root (when one is detected).
///
/// `@`-prefixed magic paths are not expanded — the file_arg here is the
/// value the user has already committed in argv, which the shell completion
/// engine would have rewritten to a concrete path on selection.
pub(super) fn resolve_prompt_path(file_arg: &str, ctx: &ScopeContext) -> Option<PathBuf> {
    if file_arg.is_empty() {
        return None;
    }
    let trimmed = file_arg.trim_matches(['"', '\'']);
    let raw = Path::new(trimmed);
    if raw.is_absolute() && raw.is_file() {
        return Some(raw.to_path_buf());
    }
    let from_cwd = ctx.cwd.join(raw);
    if from_cwd.is_file() {
        return Some(from_cwd);
    }
    if let Some(root) = scopes::effective_repo_root(ctx) {
        let from_repo = root.join(raw);
        if from_repo.is_file() {
            return Some(from_repo);
        }
    }
    None
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
