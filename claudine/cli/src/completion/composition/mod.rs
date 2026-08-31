//! Composition positional-argument completer.
//!
//! Phase 3 of the `2026-04-24-improved-shell-completions` feature. This
//! module owns the `<FILE>` slot for `claudine compose`,
//! `claudine inline-compose`, and `claudine sequence`. All three subcommands
//! funnel through one pipeline parameterized by [`ComposeMode`] — scope sets,
//! walker invocation, frontmatter filter, and render all flip on a single
//! field.
//!
//! Pipeline (see spec §5.1):
//!
//! 1. Parse and expand the partial through `biscuit_file::FileReference`.
//! 2. Resolve [`ScopeSet`] for the mode (see [`scopes`]).
//! 3. Walk each scope under the shared completion entry form
//!    ([`walker::walk_scope`]).
//! 4. Filter each file by mode contract ([`frontmatter::valid_for_mode`]).
//! 5. Dedup across scopes by canonical path.
//! 6. Sort by source rank then candidate text.
//! 7. Render tokens.
//!
//! Sigil paths (`@...`, `&...`, and `^...`) are filename searches through the
//! exact ordered roots supplied by `FileReference`; their sigil is retained in
//! the rendered candidate. Magic candidates are deduped by basename, and the
//! committed value resolves to the closest matching file at launch. A
//! committed directory token (ending in `/`) shortcuts the pipeline to walk
//! only inside that directory.

use std::collections::HashSet;
use std::path::Path;

use biscuit_file::{CompletionEntryForm, FileReference, PartialCompletion};

use super::fuzzy;
use super::scopes::{self, ComposeMode, ScopeContext};

mod compose;
mod inline_compose;
mod magic_at;
mod sequence;
mod setter_value;

use compose::gather_empty_or_word;
use magic_at::gather_magic;
use setter_value::gather_committed;

/// Source rank for candidates emitted by the repo-wide directory walk.
///
/// Set well above the highest scope-iteration rank so high-profile-rooted
/// candidates always sort first. Phase 3 of review-plan-1 introduces the
/// repo-wide walk to surface directories outside the curated scope set
/// (spec §5.3); these candidates are still useful but are intentionally
/// de-prioritized so the existing prompt scopes win on dedup ties.
const REPO_DIR_WALK_RANK: u8 = 10;

/// A rendered completion candidate ready for stdout emission.
///
/// Kept as a struct rather than a bare string so the rendering layer can
/// sort by `source_rank` before collapsing into strings. `source_rank`
/// mirrors the scope priority ordering: 0 = repo, 1 = area, 2 = package,
/// 3 = repo `.claudine`, 4 = user `.claudine`, 5 = extras.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    insert: String,
    source_rank: u8,
}

/// Run the composition completer for a given mode, context, and partial.
///
/// Returns candidate strings, one per line, in priority order. An empty
/// return value means "no matches" — the shell falls back to its default.
pub(crate) fn run(mode: ComposeMode, ctx: &ScopeContext, partial_token: &str) -> Vec<String> {
    let scope_set = scopes::resolve_compose_scopes(ctx, mode);
    let resolution = scopes::file_resolution_context(ctx);
    // Execution correctly rejects a bare sigil, but completion must still
    // enumerate its roots before the user has typed the first payload byte.
    let completion_token = match partial_token {
        "@" => "@_",
        "&" => "&_",
        "^" => "^_",
        token => token,
    };
    let empty_sigil = (completion_token != partial_token).then_some("");
    let Ok(Some(completion)) =
        FileReference::complete_partial_in_context(completion_token, &resolution)
    else {
        return Vec::new();
    };

    let candidates = match completion.entry_form() {
        CompletionEntryForm::Magic
        | CompletionEntryForm::RepositoryRoot
        | CompletionEntryForm::RepositoryScoped => gather_magic(mode, &completion, empty_sigil),
        CompletionEntryForm::ImplicitRelative => {
            gather_implicit(mode, ctx, &scope_set, partial_token, &completion)
        }
    };

    finalize(candidates)
}

/// Route a shared implicit-relative completion expansion into Claudine's
/// existing display policies without re-parsing the authored token.
fn gather_implicit(
    mode: ComposeMode,
    ctx: &ScopeContext,
    scope_set: &scopes::ScopeSet,
    partial_token: &str,
    completion: &PartialCompletion,
) -> Vec<Candidate> {
    let active = completion.active_segment();
    let scope = completion.rendered_prefix();
    if partial_token.is_empty() {
        gather_empty_or_word(mode, ctx, scope_set, active)
    } else if active.is_empty() {
        gather_committed(mode, ctx, scope, active)
    } else if scope.is_empty() {
        gather_empty_or_word(mode, ctx, scope_set, active)
    } else {
        gather_committed(mode, ctx, scope, active)
    }
}

/// Sort candidates by source rank, then lexically, and collapse duplicates
/// by inserted token. Returns strings ready for stdout.
fn finalize(mut candidates: Vec<Candidate>) -> Vec<String> {
    candidates.sort_by(|a, b| {
        a.source_rank
            .cmp(&b.source_rank)
            .then_with(|| a.insert.cmp(&b.insert))
    });
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for c in candidates {
        if seen.insert(c.insert.clone()) {
            out.push(c.insert);
        }
    }
    out
}

/// File name or directory name as a `String`. Returns `None` for entries
/// whose name is not valid UTF-8 (Claudine never produces such paths; the
/// guard is defensive).
fn display_name(path: &Path) -> Option<String> {
    path.file_name().and_then(|n| n.to_str()).map(String::from)
}

/// True when a completion query `active` matches a file `basename`.
///
/// Tries a fuzzy subsequence against the extension-stripped stem (so `pl`
/// matches `plan.md`) **or** against the full basename (so once the user
/// types into the extension, `plan.`, `plan.m`, and `plan.md` still match
/// `plan.md`). An empty query matches everything. Directory candidates do
/// not use this — they match their full leaf name directly.
fn file_name_matches(basename: &str, active: &str) -> bool {
    active.is_empty()
        || fuzzy::fuzzy_match(name_stem(basename), active)
        || fuzzy::fuzzy_match(basename, active)
}

/// Strip `.md` / `.markdown` / `.yaml` / `.yml` (case-insensitive) from a
/// filename, for match-target purposes only. The original name remains in
/// the rendered candidate.
fn name_stem(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    for ext in [".markdown", ".md", ".yaml", ".yml"] {
        if lower.ends_with(ext) {
            return &name[..name.len() - ext.len()];
        }
    }
    name
}

#[cfg(test)]
mod tests;
