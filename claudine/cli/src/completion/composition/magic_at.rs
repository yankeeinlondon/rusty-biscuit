//! Magic (`@...`) path handling for the composition completer.
//!
//! Magic mode is a **filename search**: the `@` sigil means "find a prompt
//! by name across the scope priority order" (see
//! [`ScopeSet::iter_magic_scopes`]) and the completion keeps the `@`,
//! inserting just `@<basename>`. The committed `@<basename>` is resolved to
//! a concrete file at launch — the runtime resolver searches the same
//! prompt-scope directories and the closest one wins (see
//! `claudine::composition::resolve`). Candidates are deduped by basename so
//! the same logical filename appears once even when it exists in several
//! scopes. Directories are intentionally NOT surfaced under `@` — directory
//! drilling is a Word-mode (non-`@`) behaviour; magic mode stays clutter-free.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{Candidate, file_name_matches};
use crate::completion::frontmatter;
use crate::completion::fuzzy::PartialLen;
use crate::completion::scopes::{ComposeMode, Scope, ScopeContext, ScopeSet};
use crate::completion::walker;

/// Entry point for magic (`@...`) resolution.
///
/// Walks every magic scope in priority order and emits one `@<basename>`
/// candidate per distinct prompt filename matching `active`. The first
/// (closest) scope to contribute a given basename owns its `source_rank`;
/// later scopes with the same basename are deduped away.
///
/// `dir` carries the path portion of a path-shaped magic partial (e.g.
/// `@prompts/plan` → `dir = "prompts"`). When non-empty, the walk root is
/// constrained via [`resolve_magic_walk_root`] so the typed prefix narrows
/// the search; the rendered candidate is still filename-only.
pub(super) fn gather_magic(
    mode: ComposeMode,
    _ctx: &ScopeContext,
    set: &ScopeSet,
    dir: &str,
    active: &str,
) -> Vec<Candidate> {
    let partial_len = PartialLen::classify(active.chars().count());

    let mut out: Vec<Candidate> = Vec::new();
    // Deduplicate by lowercased basename so a filename present in several
    // scopes surfaces once; the closest scope (iterated first) keeps it.
    let mut seen_basenames: HashSet<String> = HashSet::new();

    for (rank, scope) in set.iter_magic_scopes().enumerate() {
        let Some(walk_root) = resolve_magic_walk_root(&scope.path, dir) else {
            continue;
        };
        if !walk_root.is_dir() {
            continue;
        }
        let rank = rank.min(u8::MAX as usize) as u8;
        let scoped = Scope {
            kind: scope.kind,
            path: walk_root,
            follow_links: scope.follow_links,
        };
        for entry_path in walker::walk_scope(&scoped) {
            if entry_path.is_dir() {
                continue;
            }
            if !frontmatter::valid_for_mode(&entry_path, mode) {
                continue;
            }
            let Some(basename) = entry_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if partial_len.matching_enabled() && !file_name_matches(basename, active) {
                continue;
            }
            if !seen_basenames.insert(basename.to_ascii_lowercase()) {
                continue;
            }
            out.push(Candidate {
                insert: format!("@{basename}"),
                source_rank: rank,
            });
        }
    }
    out
}

/// Resolve the walk root for a magic-path partial against a single scope.
///
/// When `dir` is empty, the walk root is the scope root itself. When `dir`
/// is non-empty, it joins to the scope root — but with a crucial special
/// case: if `dir` starts with the scope's leaf name (e.g. `dir = "prompts"`
/// against `scope = /repo/prompts`), peel the leaf off before joining so
/// `@prompts/plan` resolves against `/repo/prompts/` rather than
/// `/repo/prompts/prompts/`.
///
/// ## Returns
///
/// `Some(path)` with the resolved walk root path (which may or may not
/// exist on disk — the caller must `is_dir()` check). `None` only when the
/// scope root has no final component to extract (defensive — Claudine
/// never emits such scopes in practice).
pub(super) fn resolve_magic_walk_root(scope_root: &Path, dir: &str) -> Option<PathBuf> {
    if dir.is_empty() {
        return Some(scope_root.to_path_buf());
    }
    // Peel off a matching scope-suffix so `@prompts/plan` resolves
    // against `<scope>/plan/` rather than `<scope>/prompts/plan/` when
    // the scope already ends in `/prompts`. Handles multi-segment
    // suffixes too (e.g. `@.claudine/prompts/plan` against a scope
    // ending in `.claudine/prompts`).
    let dir_segments: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    if dir_segments.is_empty() {
        return Some(scope_root.to_path_buf());
    }
    // Find the longest prefix of `dir` that matches a trailing suffix of
    // `scope_root`. For each candidate length k (from the longest down),
    // compare the last k components of `scope_root` against the first k
    // components of `dir_segments`. The first match wins; peel that many
    // segments off and join the rest.
    let scope_components: Vec<&str> = scope_root
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    for k in (1..=dir_segments.len().min(scope_components.len())).rev() {
        let scope_tail = &scope_components[scope_components.len() - k..];
        if scope_tail == &dir_segments[..k] {
            let remainder = dir_segments[k..].join("/");
            if remainder.is_empty() {
                return Some(scope_root.to_path_buf());
            }
            return Some(scope_root.join(remainder));
        }
    }
    Some(scope_root.join(dir))
}
