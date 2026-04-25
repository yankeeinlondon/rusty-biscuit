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
//! 1. Classify the partial under the cursor into [`PartialKind`].
//! 2. Resolve [`ScopeSet`] for the mode (see [`scopes`]).
//! 3. Walk each scope under a per-partial-kind strategy
//!    ([`walker::walk_scope`]).
//! 4. Filter each file by mode contract ([`frontmatter::valid_for_mode`]).
//! 5. Dedup across scopes by canonical path.
//! 6. Sort by source rank then candidate text.
//! 7. Render tokens.
//!
//! Magic paths (`@...`) resolve against the scope priority order and are
//! rendered as relative paths — the `@` is a search sigil, not part of the
//! inserted value. A committed directory token (ending in `/`) shortcuts
//! the pipeline to walk only inside that directory.

#![allow(dead_code)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::frontmatter;
use super::fuzzy::{self, DirMatchMode, PartialLen};
use super::scopes::{self, ComposeMode, Scope, ScopeContext, ScopeSet};
use super::walker;

/// Source rank for candidates emitted by the repo-wide directory walk.
///
/// Set well above the highest scope-iteration rank so high-profile-rooted
/// candidates always sort first. Phase 3 of review-plan-1 introduces the
/// repo-wide walk to surface directories outside the curated scope set
/// (spec §5.3); these candidates are still useful but are intentionally
/// de-prioritized so the existing prompt scopes win on dedup ties.
const REPO_DIR_WALK_RANK: u8 = 10;

/// Classification of the token under the cursor in a composition positional
/// slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PartialKind {
    /// Empty partial — the user has typed `claudine compose <TAB>`.
    Empty,
    /// `@...` magic path — search sigil; resolved to a relative inserted
    /// token on selection.
    ///
    /// `dir` is the path portion before the last `/` (scope-relative);
    /// `active` is the fuzzy-match segment after the last `/`. For a bare
    /// `@plan`, `dir` is empty and `active` is `"plan"`; for
    /// `@prompts/plan`, `dir` is `"prompts"` and `active` is `"plan"`.
    Magic { dir: String, active: String },
    /// A committed directory token — ends in `/`. Walking is confined to
    /// that directory relative to cwd (or repo root).
    CommittedDir(String),
    /// A word partial with no `/` and no leading `@`. The active segment
    /// length drives fuzzy matching and directory visibility.
    Word(String),
    /// A path-shaped partial with a `/` inside it but **not** ending in
    /// `/` — e.g. `prompts/pl`. The committed portion before the last
    /// `/` is the walk root; everything after is the active segment.
    PartialPath { dir: String, active: String },
}

impl PartialKind {
    /// Derive a partial kind from the raw token the shell forwarded.
    pub(crate) fn classify(token: &str) -> Self {
        if token.is_empty() {
            return Self::Empty;
        }
        if let Some(rest) = token.strip_prefix('@') {
            if let Some((dir, active)) = rest.rsplit_once('/') {
                return Self::Magic {
                    dir: dir.to_string(),
                    active: active.to_string(),
                };
            }
            return Self::Magic {
                dir: String::new(),
                active: rest.to_string(),
            };
        }
        if token.ends_with('/') {
            return Self::CommittedDir(token.to_string());
        }
        if let Some((dir, active)) = token.rsplit_once('/') {
            return Self::PartialPath {
                dir: dir.to_string(),
                active: active.to_string(),
            };
        }
        Self::Word(token.to_string())
    }

    /// The "active segment" — the piece that drives fuzzy matching and
    /// directory visibility gating. Empty for `Empty` / `CommittedDir`.
    /// For `Magic`, returns the segment after the last `/` (or the whole
    /// token if there is no `/`).
    pub(crate) fn active_segment(&self) -> &str {
        match self {
            Self::Empty | Self::CommittedDir(_) => "",
            Self::Magic { active, .. } => active,
            Self::Word(s) => s,
            Self::PartialPath { active, .. } => active,
        }
    }
}

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
    let kind = PartialKind::classify(partial_token);
    let scope_set = scopes::resolve_compose_scopes(ctx, mode);

    let candidates = match &kind {
        PartialKind::Empty => gather_empty_or_word(mode, ctx, &scope_set, ""),
        PartialKind::Word(active) => gather_empty_or_word(mode, ctx, &scope_set, active),
        PartialKind::Magic { dir, active } => gather_magic(mode, ctx, &scope_set, dir, active),
        PartialKind::CommittedDir(dir) => gather_committed(mode, ctx, dir, ""),
        PartialKind::PartialPath { dir, active } => gather_committed(mode, ctx, dir, active),
    };

    finalize(candidates)
}

/// Empty / Word path: walk every configured scope, apply fuzzy matching to
/// the active segment, and consult [`PartialLen`] to decide whether
/// directories are in play.
///
/// After the high-profile scope pass completes, a second pass walks the
/// repo (or CWD when no repo is detected) to emit directory candidates
/// independent of the high-profile file scope set. This is what makes
/// `compose c` surface directories like `claudine/` even when they are
/// not under `prompts/`. See [`gather_repo_dirs`] for details.
fn gather_empty_or_word(
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
                scope_root: &scope.path,
                entry_path: &entry_path,
                active,
                partial_len,
                rank,
            };
            render_entry_word(&render_ctx, &mut seen, &mut out);
        }
    }

    gather_repo_dirs(ctx, active, partial_len, &mut seen, &mut out);

    out
}

/// Second-pass repo-wide directory walk.
///
/// Independent of the high-profile scope set: walks
/// [`scopes::resolve_repo_dir_walk_root`] once and emits **directory**
/// candidates only. Files are intentionally skipped — the file pipeline
/// is owned by the high-profile-scope pass that runs first.
///
/// Match strategy varies by typed-prefix length (see
/// [`PartialLen::dir_match_mode`]):
///
/// - `DirMatchMode::None` — empty partial, function returns immediately.
/// - `DirMatchMode::Prefix` — case-insensitive starting-substring match
///   against the directory's leaf name.
/// - `DirMatchMode::Fuzzy` — case-insensitive fuzzy subsequence match.
///
/// Dedup is shared with the first pass via the canonical-path `seen` set
/// so a directory that already surfaced under a high-profile scope (e.g.
/// `prompts/` at the repo root) does not appear twice. Candidates are
/// emitted with [`REPO_DIR_WALK_RANK`] so they sort after every
/// high-profile-rooted candidate.
fn gather_repo_dirs(
    ctx: &ScopeContext,
    active: &str,
    partial_len: PartialLen,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Candidate>,
) {
    let dir_mode = partial_len.dir_match_mode();
    if dir_mode == DirMatchMode::None {
        return;
    }
    let walk_scope = scopes::resolve_repo_dir_walk_root(ctx);
    if !walk_scope.path.is_dir() {
        return;
    }
    let walk_root = walk_scope.path.clone();
    let entries = walker::walk_scope(&walk_scope);
    for entry_path in entries {
        if !entry_path.is_dir() {
            continue;
        }
        // Skip the walk root itself (the walker already strips depth-0
        // entries, but a defensive guard keeps the empty-relative case
        // out of the rendered output).
        let Ok(rel) = entry_path.strip_prefix(&walk_root) else {
            continue;
        };
        let Some(rel_str) = rel.to_str() else {
            continue;
        };
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
    scope_root: &'a Path,
    entry_path: &'a Path,
    active: &'a str,
    partial_len: PartialLen,
    rank: u8,
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
        // Match against the stem for files and the directory name for
        // directories. The stem is a more natural target — users type
        // `plan` intending `plan.md`, not `plan.md` literally.
        let target = if is_dir {
            name_cmp.as_str()
        } else {
            name_stem(&name_cmp)
        };
        if !fuzzy::fuzzy_match(target, ctx.active) {
            return;
        }
    }

    let Some(insert) = format_relative_insert(ctx.scope_root, ctx.entry_path) else {
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

/// Render the path relative to the scope root. Scope roots already carry
/// the `prompts/` suffix, so we rejoin with that leaf to keep the rendered
/// candidate anchored at a recognizable root (`prompts/plan.md`, not
/// `plan.md`). For scopes whose leaf is not `prompts` (e.g. extras like
/// `docs/`), we use the leaf of the scope root directly.
fn format_relative_insert(scope_root: &Path, entry: &Path) -> Option<String> {
    let leaf = scope_root.file_name()?.to_str()?;
    let rel = entry.strip_prefix(scope_root).ok()?;
    let rel_str = rel.to_str()?;
    if rel_str.is_empty() {
        return None;
    }
    Some(format!("{leaf}/{rel_str}"))
}

/// Magic (`@...`): resolve against the magic scope priority order (see
/// [`ScopeSet::iter_magic_scopes`]). The first scope whose walked tree
/// yields a matching file wins, and the rendered token is the
/// scope-relative path (stripping the `@`). Spec §5.5 table:
///
/// | Source tier | Inserted token |
/// |---|---|
/// | Repo / pkg-area / pkg / repo.claudine / extras | `prompts/plan.md` (or `docs/plan.md`, `.claudine/prompts/plan.md`) |
/// | User global | `~/.claudine/prompts/plan.md` |
///
/// The `dir` argument carries the path portion of a path-shaped magic
/// partial (e.g. `@prompts/plan` → `dir = "prompts"`). When non-empty, the
/// walk root is constrained via [`resolve_magic_walk_root`] so the typed
/// path prefix narrows the search before fuzzy matching the final
/// segment.
fn gather_magic(
    mode: ComposeMode,
    ctx: &ScopeContext,
    set: &ScopeSet,
    dir: &str,
    active: &str,
) -> Vec<Candidate> {
    let partial_len = PartialLen::classify(active.chars().count());
    let mut out: Vec<Candidate> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for (rank, scope) in set.iter_magic_scopes().enumerate() {
        let Some(walk_root) = resolve_magic_walk_root(&scope.path, dir) else {
            continue;
        };
        if !walk_root.is_dir() {
            continue;
        }
        let rank = rank.min(u8::MAX as usize) as u8;
        let scoped = Scope {
            path: walk_root,
            follow_links: scope.follow_links,
        };
        let entries = walker::walk_scope(&scoped);
        for entry_path in entries {
            let is_dir = entry_path.is_dir();
            if is_dir && !partial_len.directories_allowed() {
                continue;
            }
            if !is_dir && !frontmatter::valid_for_mode(&entry_path, mode) {
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
            let match_target = if is_dir {
                target.as_str()
            } else {
                name_stem(&target)
            };
            if partial_len.matching_enabled() && !fuzzy::fuzzy_match(match_target, active) {
                continue;
            }
            let Some(insert) = render_magic_insert(&scope.path, &entry_path, ctx) else {
                continue;
            };
            let insert = if is_dir {
                format!("{}/", insert.trim_end_matches('/'))
            } else {
                insert
            };
            seen.insert(canonical);
            out.push(Candidate {
                insert,
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
fn resolve_magic_walk_root(scope_root: &Path, dir: &str) -> Option<PathBuf> {
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
/// - Entries under a repo-scoped prompt root emit `prompts/<rest>` (repo
///   root) or `.claudine/prompts/<rest>` (repo `.claudine` root).
/// - Entries under the user-global scope emit `~/.claudine/prompts/<rest>`.
/// - Extras (e.g. `docs/`) emit `<leaf>/<rest>`.
fn render_magic_insert(scope_root: &Path, entry: &Path, ctx: &ScopeContext) -> Option<String> {
    if let Some(home) = ctx.home.as_ref()
        && entry.starts_with(home)
    {
        let rel = entry.strip_prefix(home).ok()?.to_str()?;
        return Some(format!("~/{rel}"));
    }
    if let Some(info) = ctx.repo_info.as_ref() {
        let rel = entry.strip_prefix(&info.root).ok()?.to_str()?;
        return Some(rel.to_string());
    }
    // Fallback: scope-relative rendering.
    format_relative_insert(scope_root, entry)
}

/// CommittedDir / PartialPath: walk only inside the committed directory
/// relative to the repo root (or cwd when no repo). High-profile scopes
/// are intentionally **not** consulted — the spec flips semantics once the
/// user commits to a subtree.
fn gather_committed(
    mode: ComposeMode,
    ctx: &ScopeContext,
    dir: &str,
    active: &str,
) -> Vec<Candidate> {
    let partial_len = PartialLen::classify(active.chars().count());

    let base = ctx
        .repo_info
        .as_ref()
        .map(|info| info.root.clone())
        .unwrap_or_else(|| ctx.cwd.clone());
    let walk_root = base.join(dir);
    if !walk_root.is_dir() {
        return Vec::new();
    }
    let scope = Scope {
        path: walk_root.clone(),
        follow_links: true,
    };
    let entries = walker::walk_scope(&scope);
    let mut out: Vec<Candidate> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for entry_path in entries {
        let is_dir = entry_path.is_dir();
        if is_dir && !partial_len.directories_allowed() {
            continue;
        }
        if !is_dir && !frontmatter::valid_for_mode(&entry_path, mode) {
            continue;
        }

        let canonical =
            std::fs::canonicalize(&entry_path).unwrap_or_else(|_| entry_path.clone());
        if !seen.insert(canonical) {
            continue;
        }
        let Some(name) = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let match_target = if is_dir { name.as_str() } else { name_stem(&name) };
        if partial_len.matching_enabled() && !fuzzy::fuzzy_match(match_target, active) {
            continue;
        }
        let rel = match entry_path.strip_prefix(&walk_root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let Some(rel_str) = rel.to_str() else { continue };
        let insert = if dir.is_empty() {
            rel_str.to_string()
        } else {
            format!("{}/{}", dir.trim_end_matches('/'), rel_str)
        };
        let insert = if is_dir {
            format!("{}/", insert.trim_end_matches('/'))
        } else {
            insert
        };
        out.push(Candidate {
            insert,
            source_rank: 0,
        });
    }
    out
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
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn seed_cargo_workspace(root: &Path, members: &[&str]) {
        fs::create_dir_all(root.join(".git")).unwrap();
        let members_list = members
            .iter()
            .map(|m| format!("    \"{m}\""))
            .collect::<Vec<_>>()
            .join(",\n");
        let root_manifest = format!(
            "[workspace]\nresolver = \"2\"\nmembers = [\n{members_list}\n]\n"
        );
        fs::write(root.join("Cargo.toml"), root_manifest).unwrap();
        for member in members {
            let member_dir = root.join(member);
            fs::create_dir_all(member_dir.join("src")).unwrap();
            let name = member.replace('/', "-");
            fs::write(
                member_dir.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
                ),
            )
            .unwrap();
            fs::write(member_dir.join("src").join("lib.rs"), "").unwrap();
        }
    }

    // -- PartialKind::classify --------------------------------------------

    #[test]
    fn classify_empty() {
        assert_eq!(PartialKind::classify(""), PartialKind::Empty);
    }

    #[test]
    fn classify_magic_strips_at_sigil() {
        assert_eq!(
            PartialKind::classify("@plan"),
            PartialKind::Magic {
                dir: String::new(),
                active: "plan".to_string(),
            }
        );
        assert_eq!(
            PartialKind::classify("@"),
            PartialKind::Magic {
                dir: String::new(),
                active: String::new(),
            }
        );
    }

    #[test]
    fn classify_magic_path_shaped() {
        assert_eq!(
            PartialKind::classify("@prompts/plan"),
            PartialKind::Magic {
                dir: "prompts".to_string(),
                active: "plan".to_string(),
            }
        );
        assert_eq!(
            PartialKind::classify("@a/b/c"),
            PartialKind::Magic {
                dir: "a/b".to_string(),
                active: "c".to_string(),
            }
        );
        assert_eq!(
            PartialKind::classify("@/x"),
            PartialKind::Magic {
                dir: String::new(),
                active: "x".to_string(),
            }
        );
        assert_eq!(
            PartialKind::classify("@plan"),
            PartialKind::Magic {
                dir: String::new(),
                active: "plan".to_string(),
            }
        );
    }

    #[test]
    fn classify_committed_dir_with_trailing_slash() {
        assert_eq!(
            PartialKind::classify("prompts/"),
            PartialKind::CommittedDir("prompts/".to_string())
        );
        assert_eq!(
            PartialKind::classify("docs/guides/"),
            PartialKind::CommittedDir("docs/guides/".to_string())
        );
    }

    #[test]
    fn classify_partial_path_between_slashes() {
        assert_eq!(
            PartialKind::classify("prompts/pl"),
            PartialKind::PartialPath {
                dir: "prompts".to_string(),
                active: "pl".to_string(),
            }
        );
    }

    #[test]
    fn classify_word_has_no_slash() {
        assert_eq!(
            PartialKind::classify("plan"),
            PartialKind::Word("plan".to_string())
        );
    }

    #[test]
    fn active_segment_varies_by_kind() {
        assert_eq!(PartialKind::Empty.active_segment(), "");
        assert_eq!(
            PartialKind::CommittedDir("x/".to_string()).active_segment(),
            ""
        );
        assert_eq!(
            PartialKind::Magic {
                dir: String::new(),
                active: "pl".to_string(),
            }
            .active_segment(),
            "pl"
        );
        assert_eq!(
            PartialKind::Magic {
                dir: "prompts".to_string(),
                active: "pl".to_string(),
            }
            .active_segment(),
            "pl"
        );
        assert_eq!(
            PartialKind::Word("pl".to_string()).active_segment(),
            "pl"
        );
        assert_eq!(
            PartialKind::PartialPath {
                dir: "prompts".to_string(),
                active: "pl".to_string(),
            }
            .active_segment(),
            "pl"
        );
    }

    // -- name_stem --------------------------------------------------------

    #[test]
    fn name_stem_strips_markdown_extensions() {
        assert_eq!(name_stem("plan.md"), "plan");
        assert_eq!(name_stem("plan.MD"), "plan");
        assert_eq!(name_stem("readme.markdown"), "readme");
        assert_eq!(name_stem("no-ext"), "no-ext");
    }

    #[test]
    fn name_stem_strips_yaml_extensions() {
        assert_eq!(name_stem("steps.yaml"), "steps");
        assert_eq!(name_stem("steps.yml"), "steps");
    }

    // -- end-to-end (compose) ---------------------------------------------

    #[test]
    fn compose_empty_partial_surfaces_files_without_prompt() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\nBody\n");
        write(&prompts.join("inline.md"), "---\nprompt: Write\n---\nBody\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "");
        assert!(
            got.iter().any(|c| c.ends_with("plan.md")),
            "compose must surface plan.md: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.ends_with("inline.md")),
            "compose must NOT surface inline.md (has prompt key): {got:?}"
        );
    }

    #[test]
    fn compose_empty_partial_does_not_surface_directories() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");
        fs::create_dir_all(prompts.join("subdir")).unwrap();

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "");
        // Empty partial → no directories in output.
        assert!(
            !got.iter().any(|c| c.ends_with("subdir/")),
            "empty partial must not emit directories: {got:?}"
        );
    }

    #[test]
    fn compose_short_prefix_fuzzy_matches_filenames_and_dirs() {
        // Spec §5.3 (review-1 finding #3): short (1–2 char) prefixes
        // surface files via fuzzy matching in high-profile scopes AND
        // directories via prefix matching from the repo-wide walk.
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");
        write(&prompts.join("notes.md"), "---\ntitle: Y\n---\n");
        // Directory at the repo root surfaces via the repo-wide walk.
        fs::create_dir_all(tmp.path().join("planning")).unwrap();

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "pl");
        assert!(
            got.iter().any(|c| c.ends_with("plan.md")),
            "short prefix `pl` must match plan.md: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.ends_with("notes.md")),
            "short prefix `pl` must not match notes.md: {got:?}"
        );
        assert!(
            got.iter().any(|c| c == "planning/"),
            "short prefix `pl` must surface matching repo-wide dir: {got:?}"
        );
    }

    #[test]
    fn compose_long_prefix_surfaces_directories() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        fs::create_dir_all(prompts.join("planning")).unwrap();
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "pla");
        assert!(
            got.iter().any(|c| c.ends_with("plan.md")),
            "long prefix must still match files: {got:?}"
        );
        assert!(
            got.iter().any(|c| c == "prompts/planning/"),
            "long prefix must surface directories with trailing slash: {got:?}"
        );
    }

    #[test]
    fn compose_committed_dir_walks_only_inside() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("outer.md"), "---\ntitle: O\n---\n");
        write(&prompts.join("planning").join("deep.md"), "---\ntitle: D\n---\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "prompts/planning/");
        assert!(
            got.iter().any(|c| c == "prompts/planning/deep.md"),
            "committed dir must surface inside content: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.ends_with("outer.md")),
            "committed dir must not surface outer content: {got:?}"
        );
    }

    #[test]
    fn compose_magic_path_resolves_relative() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "@plan");
        assert!(
            got.iter().any(|c| c == "prompts/plan.md"),
            "magic path must render without @: {got:?}"
        );
    }

    #[test]
    fn compose_magic_path_shaped_resolves_scope_relative() {
        // `@prompts/plan` must resolve against the repo-scope `prompts/`
        // root and emit `prompts/plan.md`.
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "@prompts/plan");
        assert!(
            got.iter().any(|c| c == "prompts/plan.md"),
            "path-shaped magic must resolve scope-relative: {got:?}"
        );
    }

    #[test]
    fn compose_magic_nested_path_shaped_resolves() {
        // `@prompts/drafts/plan` must resolve against
        // `<repo>/prompts/drafts/` and emit `prompts/drafts/plan.md`.
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(
            &prompts.join("drafts").join("plan.md"),
            "---\ntitle: X\n---\n",
        );

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "@prompts/drafts/plan");
        assert!(
            got.iter().any(|c| c == "prompts/drafts/plan.md"),
            "nested path-shaped magic must resolve: {got:?}"
        );
    }

    #[test]
    fn compose_magic_path_shaped_misses_when_dir_absent() {
        // Seed only `prompts/plan.md`; query `@prompts/drafts/plan`.
        // `<repo>/prompts/drafts/` does not exist, so the walk-root
        // `is_dir()` check fails and zero candidates are emitted.
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "@prompts/drafts/plan");
        assert!(
            !got.iter().any(|c| c.ends_with("plan.md")),
            "missing dir join must yield no candidates: {got:?}"
        );
    }

    // -- resolve_magic_walk_root ------------------------------------------

    #[test]
    fn magic_walk_root_empty_dir_is_scope_root() {
        let scope = Path::new("/r/prompts");
        assert_eq!(
            resolve_magic_walk_root(scope, ""),
            Some(PathBuf::from("/r/prompts"))
        );
    }

    #[test]
    fn magic_walk_root_dir_matches_scope_leaf_exactly() {
        // `@prompts/plan` against `scope = /r/prompts` with
        // `dir = "prompts"` should resolve to `/r/prompts`, not
        // `/r/prompts/prompts`.
        let scope = Path::new("/r/prompts");
        assert_eq!(
            resolve_magic_walk_root(scope, "prompts"),
            Some(PathBuf::from("/r/prompts"))
        );
    }

    #[test]
    fn magic_walk_root_dir_extends_past_scope_leaf() {
        // `dir = "prompts/drafts"` peels `prompts` off so the join
        // becomes `/r/prompts/drafts`, not `/r/prompts/prompts/drafts`.
        let scope = Path::new("/r/prompts");
        assert_eq!(
            resolve_magic_walk_root(scope, "prompts/drafts"),
            Some(PathBuf::from("/r/prompts/drafts"))
        );
    }

    #[test]
    fn magic_walk_root_dir_without_leaf_match_joins_raw() {
        // `dir = "drafts"` does not start with the scope leaf; join
        // raw → `/r/prompts/drafts`.
        let scope = Path::new("/r/prompts");
        assert_eq!(
            resolve_magic_walk_root(scope, "drafts"),
            Some(PathBuf::from("/r/prompts/drafts"))
        );
    }

    #[test]
    fn magic_walk_root_dir_against_non_matching_scope() {
        // `dir = "prompts/plan"` against a scope whose leaf is `docs`
        // joins raw because `"prompts/plan"` does not start with
        // `"docs"`. The caller's `is_dir()` check will then reject an
        // unlikely path.
        let scope = Path::new("/r/docs");
        assert_eq!(
            resolve_magic_walk_root(scope, "prompts/plan"),
            Some(PathBuf::from("/r/docs/prompts/plan"))
        );
    }

    // -- end-to-end (inline-compose) --------------------------------------

    #[test]
    fn inline_compose_surfaces_files_with_prompt_key() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plain.md"), "---\ntitle: X\n---\n");
        write(&prompts.join("inline.md"), "---\nprompt: Write\n---\nBody\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::InlineCompose, &ctx, "");
        assert!(
            got.iter().any(|c| c.ends_with("inline.md")),
            "inline-compose must surface inline.md: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.ends_with("plain.md")),
            "inline-compose must not surface plain.md: {got:?}"
        );
    }

    #[test]
    fn inline_compose_includes_docs_extra() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let docs = tmp.path().join("docs");
        write(&docs.join("spec.md"), "---\nprompt: Describe\n---\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::InlineCompose, &ctx, "");
        assert!(
            got.iter().any(|c| c.ends_with("spec.md")),
            "inline-compose must include docs/ extras: {got:?}"
        );
    }

    // -- end-to-end (sequence) --------------------------------------------

    #[test]
    fn sequence_surfaces_markdown_with_sequence_key() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(
            &prompts.join("steps.md"),
            "---\nsequence:\n  - a\n  - b\n---\n",
        );
        write(&prompts.join("nope.md"), "---\ntitle: X\n---\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Sequence, &ctx, "");
        assert!(
            got.iter().any(|c| c.ends_with("steps.md")),
            "sequence must surface steps.md: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.ends_with("nope.md")),
            "sequence must not surface nope.md (no sequence key): {got:?}"
        );
    }

    #[test]
    fn sequence_surfaces_yaml_files() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("steps.yaml"), "sequence:\n  - one\n  - two\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Sequence, &ctx, "");
        assert!(
            got.iter().any(|c| c.ends_with("steps.yaml")),
            "sequence must surface YAML files: {got:?}"
        );
    }

    // -- prefix progression ------------------------------------------------

    #[test]
    fn empty_prefix_does_not_include_any_directory() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        fs::create_dir_all(prompts.join("planning")).unwrap();
        write(&prompts.join("planning").join("a.md"), "---\ntitle: A\n---\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "");
        assert!(
            !got.iter().any(|c| c.ends_with('/')),
            "no directory candidate for empty prefix: {got:?}"
        );
    }

    #[test]
    fn three_plus_chars_includes_directories() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        fs::create_dir_all(prompts.join("planning")).unwrap();

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "pla");
        assert!(
            got.iter().any(|c| c.ends_with('/')),
            "3+ char prefix must include directories: {got:?}"
        );
    }

    // -- repo-wide directory walk (review-1 finding #2 + #3) --------------

    #[test]
    fn compose_one_char_prefix_surfaces_matching_repo_dir() {
        // Spec §5.3: 1-char prefix must surface matching directories
        // from the repo-wide walk (case-insensitive prefix match).
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        fs::create_dir_all(tmp.path().join("claudine")).unwrap();

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "c");
        assert!(
            got.iter().any(|c| c == "claudine/"),
            "1-char prefix must surface `claudine/` from repo root: {got:?}"
        );
    }

    #[test]
    fn compose_two_char_prefix_surfaces_matching_repo_dir() {
        // Spec §5.3: 2-char prefix surfaces matching directories.
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        fs::create_dir_all(tmp.path().join("docs")).unwrap();

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "do");
        assert!(
            got.iter().any(|c| c == "docs/"),
            "2-char prefix must surface `docs/` from repo root: {got:?}"
        );
    }

    #[test]
    fn compose_short_prefix_directory_match_is_starting_substring() {
        // Spec §5.3: short prefixes use prefix matching, not fuzzy. So
        // `do` matches `docs/` but NOT `widgets/` (no 'd' at start).
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        fs::create_dir_all(tmp.path().join("docs")).unwrap();
        fs::create_dir_all(tmp.path().join("widgets")).unwrap();

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "do");
        assert!(
            got.iter().any(|c| c == "docs/"),
            "starting-substring prefix must hit `docs/`: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c == "widgets/"),
            "starting-substring prefix must miss `widgets/`: {got:?}"
        );
    }

    #[test]
    fn compose_long_prefix_directory_match_is_fuzzy() {
        // Spec §5.3: 3+ char prefixes use fuzzy subsequence matching.
        // `fbb` is a subsequence of `foo-bar-baz` but not a prefix.
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        fs::create_dir_all(tmp.path().join("foo-bar-baz")).unwrap();

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "fbb");
        assert!(
            got.iter().any(|c| c == "foo-bar-baz/"),
            "long prefix must fuzzy-match `foo-bar-baz/` against `fbb`: {got:?}"
        );
    }

    #[test]
    fn compose_repo_dir_walk_skips_high_profile_roots_once() {
        // A directory that the high-profile scope walker also surfaces
        // (e.g. `prompts/planning/` at Long prefix) must dedup across
        // both passes — appearing exactly once in output.
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        fs::create_dir_all(prompts.join("planning")).unwrap();

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run(ComposeMode::Compose, &ctx, "pla");
        let count = got.iter().filter(|c| c.contains("planning")).count();
        assert_eq!(
            count, 1,
            "`prompts/planning/` must dedup across both passes: {got:?}"
        );
    }

    // -- dedup + sort ------------------------------------------------------

    #[test]
    fn finalize_dedups_and_sorts_by_rank() {
        let cs = vec![
            Candidate {
                insert: "prompts/b.md".into(),
                source_rank: 1,
            },
            Candidate {
                insert: "prompts/a.md".into(),
                source_rank: 0,
            },
            Candidate {
                insert: "prompts/a.md".into(),
                source_rank: 1,
            },
        ];
        let got = finalize(cs);
        assert_eq!(got, vec!["prompts/a.md", "prompts/b.md"]);
    }
}
