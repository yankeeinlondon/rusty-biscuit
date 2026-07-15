//! Bounded, `.gitignore`-aware filesystem walker for completion discovery.
//!
//! Phase 2 of the `2026-04-24-improved-shell-completions` feature. The
//! walker is one layer above `ignore::WalkBuilder`: it applies the project-
//! wide skip list, the hidden-underscore convention, the per-scope symlink
//! policy, and a hard candidate budget so a runaway scope cannot dominate
//! a `__complete` run.
//!
//! Callers in later phases (composition, setter-value) feed a [`Scope`] at
//! a time and layer their own extension / frontmatter filters on the
//! returned paths.
//!
//! ## Filtering contract
//!
//! For every directory entry the walker sees:
//!
//! - `.git` / `.git/HEAD`-based ignore files are honored at every depth
//!   (`ignore::WalkBuilder` default behavior when `.git` is present).
//! - Files or directories whose name starts with `_` are elided, including
//!   their children for directories. This matches the project convention
//!   for "don't surface this yet" drafts.
//! - Directories whose name matches [`SKIP_DIRS`] are pruned at every
//!   depth — `target`, `node_modules`, and similar build-output trees are
//!   never worth enumerating during completion.
//! - Symlinks are followed only when the caller sets
//!   [`Scope::follow_links`] to `true`. Agent-skill peer directories set
//!   this to `false` so Claudine-linked skills do not surface once per
//!   provider CLI directory.
//!
//! The hard candidate budget is [`MAX_CANDIDATES`]. The walker stops as
//! soon as the accumulator reaches the limit — callers that need a
//! different cap pass an explicit budget via [`walk_scope_limited`].

use std::path::{Path, PathBuf};

use ignore::{DirEntry, WalkBuilder};

use super::scopes::Scope;

/// Hard cap on the number of paths returned from a single scope walk.
///
/// Tuned for completion latency — 500 candidates is well past what any
/// shell will meaningfully display, and bounding the walker here is the
/// primary defense against the 100ms target blowing out on overgrown
/// `docs/` trees.
pub(crate) const MAX_CANDIDATES: usize = 500;

/// Directory names unconditionally skipped by the walker.
///
/// Every entry here is a build-output, cache, or VCS directory that never
/// contains completion-relevant content. They dominate wall time on any
/// mature project if left alone, so they are pruned at every depth —
/// `.gitignore` alone is not enough because many projects under-specify
/// these trees in their ignore files.
pub(crate) const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".venv",
    "venv",
    "__pycache__",
];

/// Walk a single scope, returning every matching path up to the default
/// candidate cap.
///
/// Non-existent scope roots return an empty vector — completion should
/// silently ignore scopes that haven't been created on disk yet rather
/// than surfacing errors to the shell.
pub(crate) fn walk_scope(scope: &Scope) -> Vec<PathBuf> {
    walk_scope_limited(scope, MAX_CANDIDATES)
}

/// Variant of [`walk_scope`] with an explicit budget.
///
/// Used by tests that need to exercise budget-exhaustion behavior without
/// having to fabricate 500+ files on disk.
pub(crate) fn walk_scope_limited(scope: &Scope, budget: usize) -> Vec<PathBuf> {
    walk_scope_core(scope, budget, |_| true).0
}

/// Walk outcome that distinguishes "exhausted the budget with matches"
/// from a clean result.
///
/// The ENTER autocomplete path needs this to emit the "narrow your query"
/// error rather than silently truncating when the query-matching count
/// exceeds [`MAX_CANDIDATES`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WalkOutcome {
    /// All matching entries fit within the budget.
    Complete(Vec<PathBuf>),
    /// The budget was exceeded while matching entries; the contained
    /// count is the minimum number of matches (it may be larger).
    OverCapacity(usize),
}

impl WalkOutcome {
    /// Unwrap a complete result, panicking if the walk was over capacity.
    #[cfg(test)]
    pub(crate) fn unwrap_complete(self) -> Vec<PathBuf> {
        match self {
            WalkOutcome::Complete(v) => v,
            WalkOutcome::OverCapacity(n) => {
                panic!("expected complete walk, got over capacity ({n})")
            }
        }
    }
}

/// Walk a single scope with a path predicate and explicit budget.
///
/// Only entries whose path contains `predicate` as a substring count
/// toward the budget. The walker still descends into directories that do
/// not themselves match so children remain reachable. When the matching
/// count would exceed the budget, [`WalkOutcome::OverCapacity`] is returned
/// immediately.
///
/// This is the shared core used by both the shell-completion path
/// ([`walk_scope`], [`walk_scope_limited`]) and the ENTER autocomplete
/// path. The predicate lets autocomplete push the `*query*` filter into
/// the walk so the cap counts query-matching files, not raw discoveries.
pub(crate) fn walk_scope_filtered<P>(
    scope: &Scope,
    budget: usize,
    predicate: P,
) -> WalkOutcome
where
    P: Fn(&Path) -> bool,
{
    let (paths, over_capacity) = walk_scope_core(scope, budget, predicate);
    if over_capacity {
        WalkOutcome::OverCapacity(paths.len() + 1)
    } else {
        WalkOutcome::Complete(paths)
    }
}

/// Shared implementation for all scope walks.
///
/// Returns the collected paths (truncated to `budget`) and a flag that is
/// `true` when at least one additional matching entry was discovered past
/// the budget. The shell-completion path ignores the flag and uses the
/// truncated vector; the ENTER autocomplete path promotes the flag to
/// [`WalkOutcome::OverCapacity`].
fn walk_scope_core<P>(
    scope: &Scope,
    budget: usize,
    predicate: P,
) -> (Vec<PathBuf>, bool)
where
    P: Fn(&Path) -> bool,
{
    if !scope.path.is_dir() {
        return (Vec::new(), false);
    }
    if budget == 0 {
        return (Vec::new(), false);
    }

    let _span = tracing::trace_span!(
        target: "claudine::completion",
        "completion::walk_scope_filtered",
        path = %scope.path.display(),
        follow_links = scope.follow_links,
        budget = budget,
    )
    .entered();

    let mut out: Vec<PathBuf> = Vec::new();
    let mut over_capacity = false;

    let walker = WalkBuilder::new(&scope.path)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .follow_links(scope.follow_links)
        .filter_entry(entry_passes_filters)
        .build();

    for result in walker {
        if over_capacity {
            break;
        }
        let Ok(entry) = result else { continue };
        // The walker yields the root itself first; we skip it so callers
        // only see the scope's contents.
        if entry.depth() == 0 {
            continue;
        }
        let path = entry.path();
        if !predicate(path) {
            continue;
        }
        if out.len() >= budget {
            over_capacity = true;
        } else {
            out.push(path.to_path_buf());
        }
    }

    (out, over_capacity)
}

/// Return `true` when the entry should be descended into / emitted.
///
/// Applied to every entry including the scope root. Depth 0 (the scope
/// root) is always retained so the walker can descend; all other entries
/// are tested against the `_`-prefix convention and the curated skip list.
///
/// Exposed so the schema `match(...)` file-completion path can share the
/// exact same `_`-prefix and [`SKIP_DIRS`] exclusion as the scope walker
/// (it walks the repo with its own glob-matching `WalkBuilder` rather than
/// through [`walk_scope`], but the exclusion contract must be identical).
pub(crate) fn entry_passes_filters(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let Some(name) = entry.file_name().to_str() else {
        // Non-UTF-8 names are exceedingly rare; pass them through so the
        // caller's extension check can decide. Filtering them here would
        // mask broken filesystems silently.
        return true;
    };
    if name.starts_with('_') {
        return false;
    }
    if SKIP_DIRS.contains(&name) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests;
