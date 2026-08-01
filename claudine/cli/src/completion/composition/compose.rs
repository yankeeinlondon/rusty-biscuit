//! Empty / Word partial handling for the composition completer.
//!
//! [`gather_empty_or_word`] walks every configured scope, applies fuzzy
//! matching to the active segment, and consults [`PartialLen`] to decide
//! whether directories are in play. A second pass walks the repo (or CWD
//! when no repo is detected) to emit directory candidates independent of
//! the high-profile file scope set.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string;

use super::{Candidate, REPO_DIR_WALK_RANK, display_name, file_name_matches};
use crate::completion::frontmatter;
use crate::completion::fuzzy::{self, DirMatchMode, PartialLen};
use crate::completion::scopes::{self, ComposeMode, Scope, ScopeContext, ScopeKind, ScopeSet};
use crate::completion::walker;

/// Empty / Word path: walk every configured scope, apply fuzzy matching to
/// the active segment, and consult [`PartialLen`] to decide whether
/// directories are in play.
///
/// After the high-profile scope pass completes, a second pass walks the
/// repo (or CWD when no repo is detected) to emit directory candidates
/// independent of the high-profile file scope set. This is what makes
/// `compose c` surface directories like `claudine/` even when they are
/// not under `prompts/`. See [`gather_repo_dirs`] for details.
pub(super) fn gather_empty_or_word(
    mode: ComposeMode,
    ctx: &ScopeContext,
    set: &ScopeSet,
    active: &str,
) -> Vec<Candidate> {
    let partial_len = PartialLen::classify(active.chars().count());
    let mut out: Vec<Candidate> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for (rank, scope) in set.iter_scopes().enumerate() {
        let rank = rank.min(u8::MAX as usize) as u8;
        if !scope.path.is_dir() {
            continue;
        }
        let entries = walker::walk_scope(scope);
        for entry_path in entries {
            let render_ctx = WordRenderCtx {
                mode,
                scope,
                entry_path: &entry_path,
                active,
                partial_len,
                rank,
                scope_ctx: ctx,
            };
            render_entry_word(&render_ctx, &mut seen, &mut out);
        }
    }

    let walk_scope = scopes::resolve_repo_dir_walk_root(ctx);
    let render_base = walk_scope.path.clone();
    gather_repo_dirs(
        &walk_scope,
        &render_base,
        active,
        partial_len,
        &mut seen,
        &mut out,
    );

    out
}

/// Second-pass repo-wide directory walk.
///
/// Walks the supplied scope once and emits **directory** candidates only.
/// Files are intentionally skipped — the file pipeline is owned by the
/// caller's first pass. Used by both Word mode (`gather_empty_or_word`,
/// walking the repo / cwd root) and Magic mode (`gather_magic`, walking
/// either the repo / cwd root or the magic-resolved subdirectory).
///
/// Match strategy varies by typed-prefix length (see
/// [`PartialLen::dir_match_mode`]):
///
/// - `DirMatchMode::None` — empty partial, function returns immediately.
/// - `DirMatchMode::Prefix` — case-insensitive starting-substring match
///   against the directory's leaf name.
/// - `DirMatchMode::Fuzzy` — case-insensitive fuzzy subsequence match.
///
/// Rendering is `render_base`-relative so callers can choose the base
/// segment that should appear in the inserted token. Word mode passes
/// the walk root as the render base (so the rendered token is
/// repo-relative); Magic mode with a path-shaped `dir` passes the repo
/// root so `<repo>/prompts/planning/` renders as `prompts/planning/`
/// rather than just `planning/`.
///
/// Dedup is shared with the file pass via the canonical-path `seen` set
/// so a directory that already surfaced under a high-profile scope (e.g.
/// `prompts/` at the repo root) does not appear twice. Candidates are
/// emitted with [`REPO_DIR_WALK_RANK`] so they sort after every
/// high-profile-rooted candidate.
pub(super) fn gather_repo_dirs(
    walk_scope: &Scope,
    render_base: &Path,
    active: &str,
    partial_len: PartialLen,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Candidate>,
) {
    let dir_mode = partial_len.dir_match_mode();
    if dir_mode == DirMatchMode::None {
        return;
    }
    if !walk_scope.path.is_dir() {
        return;
    }
    let entries = walker::walk_scope(walk_scope);
    for entry_path in entries {
        if !entry_path.is_dir() {
            continue;
        }
        // Skip the walk root itself (the walker already strips depth-0
        // entries, but a defensive guard keeps the empty-relative case
        // out of the rendered output).
        let Ok(rel) = entry_path.strip_prefix(render_base) else {
            continue;
        };
        let rel_str = to_portable_string(rel);
        if rel_str.is_empty() {
            continue;
        }

        let Some(leaf) = entry_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let dir_match = match dir_mode {
            DirMatchMode::None => false,
            DirMatchMode::Prefix => fuzzy::prefix_match(leaf, active),
            DirMatchMode::Fuzzy => fuzzy::fuzzy_match(leaf, active),
        };
        if !dir_match {
            continue;
        }

        let canonical = std::fs::canonicalize(&entry_path).unwrap_or_else(|_| entry_path.clone());
        if !seen.insert(canonical) {
            continue;
        }
        let insert = format!("{}/", rel_str.trim_end_matches('/'));
        out.push(Candidate {
            insert,
            source_rank: REPO_DIR_WALK_RANK,
        });
    }
}

/// Bundled inputs to [`render_entry_word`]. Grouping the per-entry knobs
/// keeps clippy happy about argument count and gives callers a stable
/// spot to plug in future rendering policies (score, highlight, etc.).
struct WordRenderCtx<'a> {
    mode: ComposeMode,
    scope: &'a Scope,
    entry_path: &'a Path,
    active: &'a str,
    partial_len: PartialLen,
    rank: u8,
    scope_ctx: &'a ScopeContext,
}

/// Decide whether a walked entry should become a candidate in Word mode.
fn render_entry_word(
    ctx: &WordRenderCtx<'_>,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Candidate>,
) {
    let is_dir = ctx.entry_path.is_dir();

    // Only files from the immediate level or below are emitted; walker
    // already returns everything rooted under the scope. For Empty / Short
    // partials we skip nested directories entirely — drill-down is only
    // offered once the user commits to a subtree.
    if is_dir && !ctx.partial_len.directories_allowed() {
        return;
    }
    if !is_dir && !frontmatter::valid_for_mode(ctx.entry_path, ctx.mode) {
        return;
    }

    let canonical =
        std::fs::canonicalize(ctx.entry_path).unwrap_or_else(|_| ctx.entry_path.to_path_buf());
    if !seen.insert(canonical) {
        return;
    }

    let Some(name_cmp) = display_name(ctx.entry_path) else {
        return;
    };

    if ctx.partial_len.matching_enabled() {
        // Files match the stem OR the full basename (so `plan` and `plan.`
        // both hit `plan.md`); directories match their full leaf name.
        let matched = if is_dir {
            fuzzy::fuzzy_match(&name_cmp, ctx.active)
        } else {
            file_name_matches(&name_cmp, ctx.active)
        };
        if !matched {
            return;
        }
    }

    let Some(insert) = format_relative_insert(ctx.scope, ctx.entry_path, ctx.scope_ctx) else {
        return;
    };
    let insert = if is_dir {
        format!("{}/", insert.trim_end_matches('/'))
    } else {
        insert
    };
    out.push(Candidate {
        insert,
        source_rank: ctx.rank,
    });
}

/// Render the path relative to the scope root.
///
/// Prioritises repository-relative rendering (via [`ScopeContext::repo_info`]
/// or [`ScopeContext::git_root`]) so that candidates from every scope tier
/// emit runnable paths:
///
/// - Repo `prompts/` → `prompts/plan.md`
/// - Repo `.claudine/prompts/` → `.claudine/prompts/plan.md`
/// - Package-area / package prompts → `claudine/prompts/plan.md`
/// - User-global `~/.claudine/prompts/` → `~/.claudine/prompts/plan.md`
/// - Extras (`docs/`) → `docs/plan.md`
///
/// Falls back to scope-leaf-relative rendering only when no anchor root
/// is available.
pub(super) fn format_relative_insert(
    scope: &Scope,
    entry: &Path,
    ctx: &ScopeContext,
) -> Option<String> {
    if scope.kind == ScopeKind::UserClaudinePrompts
        && let Some(insert) = format_home_relative_insert(entry, ctx)
    {
        return Some(insert);
    }

    // 1. Repo-relative rendering (Cargo workspace, monorepo, or plain git).
    if let Some(root) = scopes::effective_repo_root(ctx)
        && let Ok(rel) = entry.strip_prefix(root)
    {
        let rel_str = to_portable_string(rel);
        if !rel_str.is_empty() {
            return Some(rel_str);
        }
    }
    // 2. Home-relative rendering (user-global `~/.claudine/...`).
    if let Some(insert) = format_home_relative_insert(entry, ctx) {
        return Some(insert);
    }
    // 3. Fallback: scope-leaf-relative rendering.
    let leaf = scope.path.file_name()?;
    let rel = entry.strip_prefix(&scope.path).ok()?;
    let rel_str = to_portable_string(rel);
    if rel_str.is_empty() {
        return None;
    }
    Some(format!("{}/{rel_str}", to_portable_string(Path::new(leaf))))
}

fn format_home_relative_insert(entry: &Path, ctx: &ScopeContext) -> Option<String> {
    let home = ctx.home.as_ref()?;
    if !entry.starts_with(home) {
        return None;
    }
    let rel = entry.strip_prefix(home).ok()?;
    let rel_text = to_portable_string(rel);
    if rel_text.is_empty() {
        return None;
    }
    Some(format!("~/{rel_text}"))
}

#[cfg(test)]
mod portable_tests {
    use super::*;

    fn context() -> ScopeContext {
        ScopeContext {
            cwd: PathBuf::from("elsewhere"),
            home: None,
            repo_info: None,
            git_root: None,
        }
    }

    #[test]
    fn relative_insert_portably_renders_windows_shaped_segments() {
        let scope = Scope {
            kind: ScopeKind::RepoDocs,
            path: PathBuf::from("repo").join("docs"),
            follow_links: true,
        };
        let entry = scope.path.join(r"nested\plan.md");

        assert_eq!(
            format_relative_insert(&scope, &entry, &context()),
            Some("docs/nested/plan.md".to_string())
        );
    }

    #[test]
    fn home_insert_portably_renders_windows_shaped_segments() {
        let home = PathBuf::from("home");
        let entry = home.join(r".claudine\prompts\plan.md");
        let mut ctx = context();
        ctx.home = Some(home);

        assert_eq!(
            format_home_relative_insert(&entry, &ctx),
            Some("~/.claudine/prompts/plan.md".to_string())
        );
    }
}
