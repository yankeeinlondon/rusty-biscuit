//! Setter-value completer for composition subcommands.
//!
//! Phase 4 of the `2026-04-24-improved-shell-completions` feature. This
//! module owns the value slot of a `name=value` setter when the cursor
//! sits on a setter-shaped token inside a `compose`, `inline-compose`, or
//! `sequence` invocation.
//!
//! ## Contract (spec §5.4)
//!
//! - Setter names match `^[A-Za-z_][A-Za-z0-9_-]*` (enforced upstream by
//!   [`super::engine::classify_completion_target`]). This module
//!   re-parses the token so it can split name from value without relying
//!   on the classifier's internal state.
//! - The value is classified by its first non-quote character. `@`
//!   triggers file completion against the `docs/`, `features/`, `fixes/`,
//!   and `reviews/` subdirectories under the invoking `cwd` (the launch
//!   area). Any other leading character returns zero candidates so the
//!   shell's default completion takes over.
//! - Leading `"` and `'` quotes are stripped for classification; the
//!   emitted candidate always wraps the value in `'...'`. A user-typed
//!   opening `"` is effectively normalized to `'` (spec §5.4).
//! - The inserted candidate replaces the entire setter token:
//!   `spec='docs/plan.md'`, not just the value. Shell completion rewrites
//!   the whole word under the cursor, so the name must travel with the
//!   value to avoid dropping the `spec=` prefix on selection.
//!
//! ## Rendering
//!
//! Matched files are rendered as paths relative to the invoking `cwd` (the
//! launch area). This matches the mental model of a doc path typed into a
//! frontmatter override: a frontmatter file reference resolves at runtime
//! against the launch area, so completion offers exactly those `cwd`-relative
//! paths the runtime resolver will accept — whether the user typed them by
//! hand or selected them from completion.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::fuzzy::{self, PartialLen};
use super::scopes::{self, Scope, ScopeContext, ScopeKind};
use super::walker;

/// Subdirectories resolved for setter-value `@` completion.
///
/// Mirrors the repo convention: `docs/` for documentation, `features/`
/// and `fixes/` for planning artefacts, `reviews/` for code-review
/// outputs. These are the typical targets a composition frontmatter
/// setter points at when the author wants to inject a document by
/// reference instead of inlining it.
const SETTER_VALUE_SUBDIRS: &[&str] = &["docs", "features", "fixes", "reviews"];

/// Parsed `name=value` token.
///
/// Kept private — the module's public contract is the [`run`] function,
/// which takes the raw cursor token and returns rendered candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SetterToken<'a> {
    name: &'a str,
    raw_value: &'a str,
}

impl<'a> SetterToken<'a> {
    /// Parse a `name=value` token. Returns `None` when the shape does not
    /// match `^[A-Za-z_][A-Za-z0-9_-]*=.*` — the classifier is expected
    /// to have already enforced this, but re-checking keeps the module
    /// self-contained and test-friendly.
    fn parse(token: &'a str) -> Option<Self> {
        let eq_pos = token.find('=')?;
        if eq_pos == 0 {
            return None;
        }
        let bytes = token.as_bytes();
        if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
            return None;
        }
        if !bytes[1..eq_pos]
            .iter()
            .all(|&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return None;
        }
        Some(Self {
            name: &token[..eq_pos],
            raw_value: &token[eq_pos + 1..],
        })
    }
}

/// Run the setter-value completer against a cursor token.
///
/// Returns the candidate strings (one full `name='value'` replacement
/// per line). An empty return value means "no matches" — the shell falls
/// back to its default completion.
pub(crate) fn run(token: &str, ctx: &ScopeContext) -> Vec<String> {
    let Some(parsed) = SetterToken::parse(token) else {
        return Vec::new();
    };
    let (stripped_value, _quote) = strip_leading_quote(parsed.raw_value);
    // Classify by the first non-quote character. Only `@...` routes to
    // file completion; anything else yields zero candidates.
    let mut chars = stripped_value.chars();
    let first = chars.next();
    if first != Some('@') {
        return Vec::new();
    }
    let active = chars.as_str();
    let relatives = gather_value_candidates(active, ctx);
    relatives
        .into_iter()
        .map(|rel| format!("{}='{}'", parsed.name, rel))
        .collect()
}

/// Strip a single leading `"` or `'` from a value. Returns the remaining
/// body and the quote character that was stripped, if any. Used only to
/// classify the value body — the emitted candidate is always wrapped in
/// `'...'` regardless of the user's original quote choice.
fn strip_leading_quote(value: &str) -> (&str, Option<char>) {
    let mut chars = value.chars();
    if let Some(c) = chars.next()
        && (c == '"' || c == '\'')
    {
        return (chars.as_str(), Some(c));
    }
    (value, None)
}

/// Walk every setter-value scope and collect matching files rendered as
/// paths relative to the repo root (or cwd when no repo is detected).
fn gather_value_candidates(active: &str, ctx: &ScopeContext) -> Vec<String> {
    let partial_len = PartialLen::classify(active.chars().count());
    let scopes = resolve_setter_scopes(ctx);
    let base = scopes::property_value_root(ctx).to_path_buf();

    let mut out: Vec<(u8, String)> = Vec::new();
    let mut seen_entries: HashSet<PathBuf> = HashSet::new();

    for (rank, scope) in scopes.iter().enumerate() {
        let rank = rank.min(u8::MAX as usize) as u8;
        if !scope.path.is_dir() {
            continue;
        }
        for entry in walker::walk_scope(scope) {
            if entry.is_dir() {
                // Setter values target files only. Directories would not
                // round-trip back through the composition frontmatter
                // override slot anyway.
                continue;
            }
            // Gate on Markdown extension before any other work. The setter-
            // value slot is contractually a Markdown-document reference
            // (spec §5.4, review-1 finding 1); non-Markdown files must
            // never surface regardless of partial_len classification.
            if !has_markdown_extension(&entry) {
                continue;
            }
            let canonical = std::fs::canonicalize(&entry).unwrap_or_else(|_| entry.clone());
            if !seen_entries.insert(canonical) {
                continue;
            }
            let Some(name) = entry.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if partial_len.matching_enabled()
                && !fuzzy::fuzzy_match(strip_file_extension(name), active)
            {
                continue;
            }
            let Some(rel) = format_relative(&base, &entry) else {
                continue;
            };
            out.push((rank, rel));
        }
    }

    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut seen_str: HashSet<String> = HashSet::new();
    let mut result: Vec<String> = Vec::with_capacity(out.len());
    for (_, s) in out {
        if seen_str.insert(s.clone()) {
            result.push(s);
        }
    }
    result
}

/// Resolve the ordered scope list for setter-value `@` completion.
///
/// Each [`SETTER_VALUE_SUBDIRS`] entry is joined under the invoking `cwd`
/// (the launch area; see [`scopes::property_value_root`]) — the same anchor
/// the runtime read-side resolver uses for frontmatter file references, so a
/// selected candidate resolves at launch exactly as it was offered. The
/// walker tolerates missing directories, so non-existent subdirs are returned
/// as-is and silently ignored at walk time.
///
/// Symlinks are always followed — none of these scopes are agent-skill
/// peer directories where Claudine's linker produces cross-provider
/// duplicates.
fn resolve_setter_scopes(ctx: &ScopeContext) -> Vec<Scope> {
    let base = scopes::property_value_root(ctx);
    SETTER_VALUE_SUBDIRS
        .iter()
        .map(|sub| Scope {
            kind: ScopeKind::RepoDocs,
            path: base.join(sub),
            follow_links: true,
        })
        .collect()
}

/// Render `entry` as a path relative to `base`. Returns `None` when
/// `entry` is not inside `base` — this should not occur in practice
/// because every scope we walk is rooted under `base`, but keeping the
/// guard avoids rendering absolute paths on unusual filesystem layouts.
fn format_relative(base: &Path, entry: &Path) -> Option<String> {
    entry
        .strip_prefix(base)
        .ok()
        .and_then(|r| r.to_str().map(str::to_string))
}

/// Strip the last extension from a filename (for match-target purposes
/// only). The original name remains in the rendered candidate.
fn strip_file_extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(idx) if idx > 0 => &name[..idx],
        _ => name,
    }
}

/// Accept `.md` and `.markdown` extensions case-insensitively.
///
/// Mirrors [`super::frontmatter::has_markdown_extension`] but is
/// duplicated here so `setter_value` does not need to depend on
/// `frontmatter` for a single small predicate. The canonical list is
/// `md` and `markdown` (spec §5.4).
fn has_markdown_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;
