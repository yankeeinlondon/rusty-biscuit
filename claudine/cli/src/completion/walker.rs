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
fn entry_passes_filters(entry: &DirEntry) -> bool {
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
mod tests {
    use super::*;
    use crate::completion::scopes::ScopeKind;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn init_git(root: &Path) {
        // WalkBuilder uses the presence of a `.git` directory to root its
        // `.gitignore` discovery. An empty dir is enough.
        fs::create_dir_all(root.join(".git")).unwrap();
    }

    fn scope(path: PathBuf, follow_links: bool) -> Scope {
        Scope {
            kind: ScopeKind::RepoPrompts,
            path,
            follow_links,
        }
    }

    #[test]
    fn nonexistent_scope_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");
        let got = walk_scope(&scope(missing, true));
        assert!(got.is_empty());
    }

    #[test]
    fn walks_flat_directory() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        write_file(&root.join("a.md"), "# a");
        write_file(&root.join("b.md"), "# b");

        let got = walk_scope(&scope(root.clone(), true));
        let names: Vec<String> = got
            .iter()
            .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(String::from))
            .collect();
        assert!(
            names.contains(&"a.md".to_string()),
            "missing a.md: {names:?}"
        );
        assert!(
            names.contains(&"b.md".to_string()),
            "missing b.md: {names:?}"
        );
    }

    #[test]
    fn underscore_prefixed_files_are_elided() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        write_file(&root.join("_draft.md"), "# d");
        write_file(&root.join("public.md"), "# p");

        let got = walk_scope(&scope(root, true));
        let names: Vec<String> = got
            .iter()
            .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(String::from))
            .collect();
        assert!(
            !names.contains(&"_draft.md".to_string()),
            "underscore file leaked: {names:?}"
        );
        assert!(names.contains(&"public.md".to_string()));
    }

    #[test]
    fn underscore_prefixed_directories_are_elided_recursively() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        write_file(&root.join("_wip").join("notes.md"), "# n");
        write_file(&root.join("ok").join("kept.md"), "# k");

        let got = walk_scope(&scope(root, true));
        let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
        assert!(
            !rendered.iter().any(|p| p.contains("_wip")),
            "underscore dir descendant leaked: {rendered:?}"
        );
        assert!(
            rendered.iter().any(|p| p.ends_with("kept.md")),
            "ok/kept.md missing: {rendered:?}"
        );
    }

    #[test]
    fn skip_list_honored_at_every_depth() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        write_file(&root.join("target").join("debris.md"), "# d");
        write_file(
            &root.join("inner").join("node_modules").join("pkg.md"),
            "# p",
        );
        write_file(&root.join("real.md"), "# r");

        let got = walk_scope(&scope(root, true));
        let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
        assert!(
            !rendered.iter().any(|p| p.contains("target")),
            "target/ leaked: {rendered:?}"
        );
        assert!(
            !rendered.iter().any(|p| p.contains("node_modules")),
            "node_modules/ leaked: {rendered:?}"
        );
        assert!(
            rendered.iter().any(|p| p.ends_with("real.md")),
            "real.md missing: {rendered:?}"
        );
    }

    #[test]
    fn gitignore_honored_at_every_depth() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path();
        // `.gitignore` rule applied at repo root must be honored inside
        // nested scopes — the walker inherits the git repo's ignore
        // context because `.git` lives at `tmp.path()`.
        write_file(&root.join(".gitignore"), "prompts/ignored/\nskip.md\n");
        write_file(
            &root.join("prompts").join("ignored").join("secret.md"),
            "# s",
        );
        write_file(&root.join("prompts").join("kept.md"), "# k");
        write_file(&root.join("prompts").join("skip.md"), "# s");

        let got = walk_scope(&scope(root.join("prompts"), true));
        let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
        assert!(
            rendered.iter().any(|p| p.ends_with("kept.md")),
            "kept.md missing: {rendered:?}"
        );
        assert!(
            !rendered.iter().any(|p| p.ends_with("secret.md")),
            "gitignored secret.md leaked: {rendered:?}"
        );
        assert!(
            !rendered.iter().any(|p| p.ends_with("skip.md")),
            "gitignored skip.md leaked: {rendered:?}"
        );
    }

    #[test]
    fn candidate_budget_honored() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        for i in 0..20 {
            write_file(&root.join(format!("f{i}.md")), "# x");
        }
        let got = walk_scope_limited(&scope(root, true), 5);
        assert_eq!(got.len(), 5, "budget not honored: {got:?}");
    }

    #[test]
    fn zero_budget_returns_empty() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        write_file(&root.join("a.md"), "# a");
        let got = walk_scope_limited(&scope(root, true), 0);
        assert!(got.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed_when_follow_links_is_false() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let real_dir = tmp.path().join("real");
        write_file(&real_dir.join("hidden-behind-link.md"), "# h");

        let scope_root = tmp.path().join("skills-scope");
        fs::create_dir_all(&scope_root).unwrap();
        write_file(&scope_root.join("direct.md"), "# d");
        symlink(&real_dir, scope_root.join("link-to-real")).unwrap();

        let got = walk_scope(&scope(scope_root, false));
        let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
        assert!(
            rendered.iter().any(|p| p.ends_with("direct.md")),
            "direct file missing: {rendered:?}"
        );
        assert!(
            !rendered
                .iter()
                .any(|p| p.ends_with("hidden-behind-link.md")),
            "symlink followed despite follow_links=false: {rendered:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_followed_when_follow_links_is_true() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let real_dir = tmp.path().join("real");
        write_file(&real_dir.join("behind-link.md"), "# h");

        let scope_root = tmp.path().join("generic-scope");
        fs::create_dir_all(&scope_root).unwrap();
        symlink(&real_dir, scope_root.join("link-to-real")).unwrap();

        let got = walk_scope(&scope(scope_root, true));
        let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
        assert!(
            rendered.iter().any(|p| p.ends_with("behind-link.md")),
            "symlink not followed despite follow_links=true: {rendered:?}"
        );
    }

    #[test]
    fn git_directory_is_never_descended() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path();
        write_file(&root.join(".git").join("hooks").join("README.md"), "# h");
        write_file(&root.join("real.md"), "# r");

        let got = walk_scope(&scope(root.to_path_buf(), true));
        let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
        assert!(
            !rendered.iter().any(|p| p.contains(".git")),
            ".git/ leaked: {rendered:?}"
        );
    }

    #[test]
    fn filtered_walk_counts_matches_not_raw_discoveries() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        write_file(&root.join("plan.md"), "# p");
        write_file(&root.join("notes.md"), "# n");
        write_file(&root.join("other.md"), "# o");

        let got = walk_scope_filtered(&scope(root, true), MAX_CANDIDATES, |p| {
            p.to_str()
                .map(|s| s.to_ascii_lowercase().contains("plan"))
                .unwrap_or(false)
        });
        let paths = got.unwrap_complete();
        assert_eq!(paths.len(), 1, "only plan.md matches: {paths:?}");
        assert!(
            paths[0].to_str().unwrap().contains("plan.md"),
            "expected plan.md: {paths:?}"
        );
    }

    #[test]
    fn filtered_walk_over_capacity_reports_more_than_cap() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        for i in 0..20 {
            write_file(&root.join(format!("plan{i}.md")), "# x");
        }
        write_file(&root.join("other.md"), "# o");

        let got = walk_scope_filtered(&scope(root, true), 5, |p| {
            p.to_str()
                .map(|s| s.to_ascii_lowercase().contains("plan"))
                .unwrap_or(false)
        });
        assert!(
            matches!(got, WalkOutcome::OverCapacity(n) if n > 5),
            "expected over-capacity with at least 6 matches, got: {got:?}"
        );
    }

    #[test]
    fn filtered_walk_zero_budget_returns_empty() {
        let tmp = TempDir::new().unwrap();
        init_git(tmp.path());
        let root = tmp.path().join("prompts");
        write_file(&root.join("plan.md"), "# p");

        let got = walk_scope_filtered(&scope(root, true), 0, |p| {
            p.to_str()
                .map(|s| s.to_ascii_lowercase().contains("plan"))
                .unwrap_or(false)
        });
        assert_eq!(got.unwrap_complete().len(), 0);
    }

    #[test]
    fn filtered_walk_nonexistent_scope_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");
        let got = walk_scope_filtered(&scope(missing, true), MAX_CANDIDATES, |_| true);
        assert!(got.unwrap_complete().is_empty());
    }
}
