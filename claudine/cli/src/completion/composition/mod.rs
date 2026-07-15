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
//! Magic paths (`@...`) are a filename search: they resolve against the
//! scope priority order and are rendered as `@<basename>` (the `@` is kept;
//! only the filename is inserted), deduped by basename. The committed
//! `@<basename>` is resolved to the closest matching file at launch. A
//! committed directory token (ending in `/`) shortcuts the pipeline to walk
//! only inside that directory.

use std::collections::HashSet;
use std::path::Path;

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
    ///
    /// Production callers extract the active segment inline at the
    /// pipeline branch point; this method exists for the variant
    /// classification regression test.
    #[cfg(test)]
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
