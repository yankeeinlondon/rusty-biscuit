//! Magic (`@...`) path handling for the composition completer.
//!
//! Magic paths resolve against the scope priority order (see
//! [`ScopeSet::iter_magic_scopes`]). The first scope whose walked tree
//! yields a matching file wins the **file tier**; subsequent file tiers
//! are shadowed. Directories surface independently from the repo-wide
//! directory walk — they mirror Word-mode directory behaviour and are
//! not affected by the file-tier shadowing rule.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::compose::{format_relative_insert, gather_repo_dirs};
use super::{Candidate, name_stem};
use crate::completion::frontmatter;
use crate::completion::fuzzy::{self, DirMatchMode, PartialLen};
use crate::completion::scopes::{self, ComposeMode, Scope, ScopeContext, ScopeSet};
use crate::completion::walker;

/// Entry point for magic (`@...`) resolution. See the module docs for
/// the shadow rule, and spec §5.5 for the full source-tier → insert-form
/// table.
///
/// `dir` carries the path portion of a path-shaped magic partial (e.g.
/// `@prompts/plan` → `dir = "prompts"`). When non-empty, the walk root is
/// constrained via [`resolve_magic_walk_root`] so the typed prefix
/// narrows the search before fuzzy matching; the same constraint applies
/// to the directory pass.
pub(super) fn gather_magic(
    mode: ComposeMode,
    ctx: &ScopeContext,
    set: &ScopeSet,
    dir: &str,
    active: &str,
) -> Vec<Candidate> {
    let partial_len = PartialLen::classify(active.chars().count());

    let (mut out, mut seen) = gather_magic_files(mode, ctx, set, dir, active, partial_len);
    gather_magic_dirs(ctx, set, dir, active, partial_len, &mut seen, &mut out);
    out
}

/// First-tier file gathering for magic resolution.
///
/// Implements the spec §5.5 first-hit-wins shadow rule: walks each scope
/// in magic priority order and returns the first scope's matching files.
/// Lower-priority scopes never contribute file candidates once a higher-
/// priority scope has produced a hit. The returned `seen` set carries the
/// canonical paths of every emitted candidate so the directory pass can
/// dedup against them without re-emitting the same path twice.
fn gather_magic_files(
    mode: ComposeMode,
    ctx: &ScopeContext,
    set: &ScopeSet,
    dir: &str,
    active: &str,
    partial_len: PartialLen,
) -> (Vec<Candidate>, HashSet<PathBuf>) {
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
        let entries = walker::walk_scope(&scoped);
        let mut scope_candidates: Vec<Candidate> = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for entry_path in entries {
            // Directories are owned by the repo-wide walk (see
            // `gather_magic_dirs`) — skip here to keep the file tier free
            // of directory candidates and avoid double-emission.
            if entry_path.is_dir() {
                continue;
            }
            if !frontmatter::valid_for_mode(&entry_path, mode) {
                continue;
            }

            let canonical =
                std::fs::canonicalize(&entry_path).unwrap_or_else(|_| entry_path.clone());
            if seen.contains(&canonical) {
                continue;
            }
            let Some(target) = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            let match_target = name_stem(&target);
            if partial_len.matching_enabled() && !fuzzy::fuzzy_match(match_target, active) {
                continue;
            }
            let Some(insert) = render_magic_insert(scope, &entry_path, ctx) else {
                continue;
            };
            seen.insert(canonical);
            scope_candidates.push(Candidate {
                insert,
                source_rank: rank,
            });
        }
        if !scope_candidates.is_empty() {
            return (scope_candidates, seen);
        }
    }
    (Vec::new(), HashSet::new())
}

/// Magic-mode directory parity walk (review-3 finding 4).
///
/// Mirrors the Word-mode directory-walk surface into Magic mode so
/// `@pl<TAB>` and `pl<TAB>` agree on directory candidates at Short and
/// Long prefix lengths. Empty (`@<TAB>`) remains directory-free.
///
/// Walk root selection:
///
/// - When `dir` is empty, walks [`scopes::resolve_repo_dir_walk_root`]
///   exactly as Word mode does — surfacing `planning/`, `docs/`, etc.
///   at the repo root.
/// - When `dir` is non-empty, walks the magic-resolved subdirectory of
///   the highest-priority scope whose joined walk root resolves on
///   disk. Match target is the leaf segment (`active`), rendered as
///   repo-relative so `<repo>/prompts/planning/` becomes
///   `prompts/planning/`.
///
/// Independent of the file-tier shadowing rule from
/// [`gather_magic_files`]: directories surface even when the winning
/// file tier was the user-global scope (or no scope produced files at
/// all). The file tier's `seen` set is threaded in so a directory that
/// already surfaced as a file candidate is not double-emitted.
fn gather_magic_dirs(
    ctx: &ScopeContext,
    set: &ScopeSet,
    dir: &str,
    active: &str,
    partial_len: PartialLen,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Candidate>,
) {
    if partial_len.dir_match_mode() == DirMatchMode::None {
        return;
    }
    let render_base = scopes::effective_repo_root(ctx)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ctx.cwd.clone());

    if dir.is_empty() {
        let walk_scope = scopes::resolve_repo_dir_walk_root(ctx);
        gather_repo_dirs(&walk_scope, &render_base, active, partial_len, seen, out);
        return;
    }

    // Path-shaped magic: walk the magic-resolved subdirectory of the
    // highest-priority scope whose joined walk root exists on disk. Render
    // base is the repo / cwd root so the inserted token retains the typed
    // prefix (e.g. `prompts/planning/` rather than `planning/`).
    for scope in set.iter_magic_scopes() {
        let Some(walk_root) = resolve_magic_walk_root(&scope.path, dir) else {
            continue;
        };
        if !walk_root.is_dir() {
            continue;
        }
        let scoped = Scope {
            kind: scope.kind,
            path: walk_root,
            follow_links: scope.follow_links,
        };
        gather_repo_dirs(&scoped, &render_base, active, partial_len, seen, out);
        return;
    }
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

/// Render the inserted token for a magic-path match.
///
/// Delegates to [`format_relative_insert`] since both magic and non-magic
/// paths now share the same repo-relative / git-root-relative /
/// home-relative rendering logic.
fn render_magic_insert(scope: &Scope, entry: &Path, ctx: &ScopeContext) -> Option<String> {
    format_relative_insert(scope, entry, ctx)
}
