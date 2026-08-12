//! Value-candidate matching for schema `name=value` setters.
//!
//! Resolves the value slot after `=`: enum members for `enum` properties and
//! filesystem paths constrained by `match(...)` globs for `file` properties.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string;
use globset::{Glob, GlobSet, GlobSetBuilder};

use darkmatter::markdown::schemas::{completion as dm_completion, CompletionKind, EffectiveSchema};

use crate::completion::default_glob;
use crate::completion::scopes::{self, ScopeContext};
use crate::completion::walker;

/// Value candidates for `property=<partial>` when the property is in the
/// schema's completable set.
///
/// - `enum` → enum members matching `value_partial` (prefix-insensitive when
///   `value_partial` is non-empty; all members for an empty partial).
/// - `file` → filesystem paths matching the property's `match(...)` globs.
///   Walks the filesystem starting from the invoking `cwd` (the launch area;
///   see [`scopes::property_value_root`]). Empty `match` patterns return no
///   candidates — fall back to shell-native file completion.
///
/// Each candidate is rendered as the **full** `name='value'` token so the
/// shell can replace the entire setter under the cursor (matches the
/// existing `setter_value` contract).
///
/// Returns an empty `Vec` for completion shapes that have no value
/// candidates (string, number, url/email/date hints) — those defer to the
/// caller's fallback completer.
pub(crate) fn property_value(
    effective: &EffectiveSchema,
    property: &str,
    value_partial: &str,
    ctx: &ScopeContext,
) -> Vec<String> {
    let Some(suggestion) = dm_completion::for_property(effective, property) else {
        return Vec::new();
    };
    match &suggestion.kind {
        CompletionKind::Enum { members } => {
            enum_candidates(property, members, value_partial)
        }
        CompletionKind::File { patterns } => {
            file_candidates(property, patterns, value_partial, suggestion.is_array, ctx)
        }
        // Hints don't produce concrete candidates — the caller falls back
        // to the existing `@`-gated path or shell-native completion.
        CompletionKind::Hint { .. } => Vec::new(),
    }
}

/// Returns the format hint string for a property when the schema marks it as
/// a hint-only completable type (url, email, date, datetime, time).
///
/// Used by the engine to surface a one-line description through the shell
/// when the completion protocol supports it. The current `__complete`
/// stdout protocol emits one candidate per line with no description channel,
/// so the engine does not yet route through this — exposed for future
/// integration with description-bearing completion protocols.
#[allow(dead_code)]
pub(crate) fn property_value_hint(
    effective: &EffectiveSchema,
    property: &str,
) -> Option<&'static str> {
    let suggestion = dm_completion::for_property(effective, property)?;
    match suggestion.kind {
        CompletionKind::Hint { format } => Some(format),
        _ => None,
    }
}

fn enum_candidates(property: &str, members: &[String], value_partial: &str) -> Vec<String> {
    let trimmed = value_partial.trim_matches(['"', '\'']);
    members
        .iter()
        .filter(|m| trimmed.is_empty() || m.to_ascii_lowercase().starts_with(&trimmed.to_ascii_lowercase()))
        .map(|m| format!("{property}='{m}'"))
        .collect()
}

fn file_candidates(
    property: &str,
    patterns: &[String],
    value_partial: &str,
    is_array: bool,
    ctx: &ScopeContext,
) -> Vec<String> {
    let base: PathBuf = scopes::property_value_root(ctx).to_path_buf();

    let (active, prefix_segments) = if is_array {
        parse_array_file_value(value_partial)
    } else {
        (normalize_partial(value_partial), Vec::new())
    };
    let excluded: HashSet<String> = prefix_segments.iter().cloned().collect();

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    if patterns.is_empty() {
        // Bare `file` / `file[]` falls back to the default markdown glob.
        let candidates = default_glob::default_markdown_candidates_filtered(ctx, |path| {
            active.is_empty() || rel_or_name_matches(path, &base, &active)
        });
        for path in candidates {
            let Ok(rel) = path.strip_prefix(&base) else {
                continue;
            };
            let rel_text = to_portable_string(rel);
            if !excluded.is_empty() && excluded.contains(&rel_text) {
                continue;
            }
            let rendered = if is_array {
                format_array_candidate(property, &prefix_segments, &rel_text)
            } else {
                format!("{property}='{rel_text}'")
            };
            if seen.insert(rendered.clone()) {
                out.push(rendered);
            }
        }
        out.sort();
        return out;
    }

    let Some(matcher) = MatchGlobs::compile(patterns) else {
        return Vec::new();
    };

    let walker = ignore::WalkBuilder::new(&base)
        .hidden(true)
        .git_ignore(true)
        .require_git(false)
        .filter_entry(walker::entry_passes_filters)
        .build();
    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(rel) = path.strip_prefix(&base) else {
            continue;
        };
        let rel_text = to_portable_string(rel);
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&rel_text);
        if !matcher.is_match(&rel_text, file_name) {
            continue;
        }
        // Filter the typed partial against the repo-relative path, not the
        // basename: `match(...)` candidates routinely share a basename
        // (e.g. every `**/*spec*.md` hit is `spec.md`), so only path-fragment
        // matching lets the user narrow by directory.
        if !active.is_empty() && !contains_ci(&rel_text, &active) {
            continue;
        }
        if !excluded.is_empty() && excluded.contains(&rel_text) {
            continue;
        }
        let rendered = if is_array {
            format_array_candidate(property, &prefix_segments, &rel_text)
        } else {
            format!("{property}='{rel_text}'")
        };
        if seen.insert(rendered.clone()) {
            out.push(rendered);
        }
    }
    out.sort();
    out
}

/// Gather absolute file paths for a schema `file`/`file[]` property.
///
/// - `patterns` empty → delegates to [`default_glob::default_markdown_candidates`]
///   (bare `file`/`file[]` fallback).
/// - `patterns` non-empty → walks from the invoking `cwd` (the launch area;
///   see [`scopes::property_value_root`]) and returns every file that
///   satisfies the `match(...)` globs.
///
/// Used by the ENTER-path missing-property chooser. TAB completion uses
/// [`file_candidates`] instead because it needs formatted setter tokens and
/// array-continuation exclusion.
pub(crate) fn file_candidate_paths(patterns: &[String], ctx: &ScopeContext) -> Vec<PathBuf> {
    let base: PathBuf = scopes::property_value_root(ctx).to_path_buf();

    if patterns.is_empty() {
        return default_glob::default_markdown_candidates(ctx);
    }

    let Some(matcher) = MatchGlobs::compile(patterns) else {
        return Vec::new();
    };

    let mut out: Vec<PathBuf> = Vec::new();
    let walker = ignore::WalkBuilder::new(&base)
        .hidden(true)
        .git_ignore(true)
        .require_git(false)
        .filter_entry(walker::entry_passes_filters)
        .build();
    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(rel) = path.strip_prefix(&base) else {
            continue;
        };
        let rel_text = to_portable_string(rel);
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&rel_text);
        if !matcher.is_match(&rel_text, file_name) {
            continue;
        }
        out.push(path.to_path_buf());
    }
    out.sort();
    out
}

/// Case-insensitive substring test.
///
/// The typed partial is applied as a `*active*` filter — it may appear
/// anywhere in the candidate path — matching the ENTER-autocomplete `*query*`
/// contract rather than a fuzzy subsequence.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// Substring-match `active` against `path`'s repo-relative form (falling back
/// to the basename when `path` is not under `base`).
///
/// Used by the bare-`file` default-glob branch so it filters on the same
/// repo-relative path the match-glob branch does — what the user types is a
/// fragment of the path that will be inserted, and shared basenames make
/// basename-only matching useless for `file(match(...))` candidates.
fn rel_or_name_matches(path: &Path, base: &Path, active: &str) -> bool {
    let target = path
        .strip_prefix(base)
        .ok()
        .map(to_portable_string)
        .or_else(|| {
            path.file_name()
                .map(|name| to_portable_string(Path::new(name)))
        });
    target.is_some_and(|t| contains_ci(&t, active))
}

/// Normalize a value partial by stripping surrounding quotes and the `@`
/// sigil so it can be used as a fuzzy match target.
fn normalize_partial(value_partial: &str) -> String {
    value_partial
        .trim_matches(['"', '\''])
        .trim_start_matches('@')
        .to_string()
}

/// Parse a `file[]` value partial into the active fuzzy target and the
/// already-committed prefix segments.
///
/// The input may be quoted or unquoted, open or closed. Outer quotes are
/// stripped for parsing; emitted candidates are always single-quoted. A
/// top-level comma splits the list. Filenames that themselves contain a
/// comma are unsupported and will be mis-split — this matches the spec
/// contract for the exclusion set.
fn parse_array_file_value(value_partial: &str) -> (String, Vec<String>) {
    let (body, _quote) = strip_outer_quotes(value_partial);
    let raw_segments: Vec<&str> = body.split(',').collect();
    if raw_segments.is_empty() {
        return (String::new(), Vec::new());
    }
    let prefix = &raw_segments[..raw_segments.len() - 1];
    let active = raw_segments.last().unwrap_or(&"").trim();
    let prefix_segments = prefix
        .iter()
        .map(|s| {
            s.trim()
                .trim_matches(['"', '\''])
                .trim_start_matches('@')
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();
    (normalize_partial(active), prefix_segments)
}

/// Strip a leading quote and, when present, a matching trailing quote.
/// Returns the unquoted body plus the quote character that was removed.
/// Unclosed quotes are also stripped from the leading edge so that
/// `spec='a.md,b<TAB>` parses correctly.
fn strip_outer_quotes(input: &str) -> (&str, Option<char>) {
    let first = input.chars().next();
    let quote = match first {
        Some('\'') | Some('"') => first,
        _ => return (input, None),
    };
    if input.len() >= 2 && input.ends_with(quote.unwrap()) {
        (&input[1..input.len() - 1], quote)
    } else {
        (&input[1..], quote)
    }
}

/// Render a `file[]` candidate, preserving the already-committed segments
/// and appending the newly selected file. The entire value is wrapped in
/// single quotes so it round-trips through the setter token parser.
fn format_array_candidate(property: &str, prefix_segments: &[String], selected: &str) -> String {
    let mut parts: Vec<String> = prefix_segments.to_vec();
    parts.push(selected.to_string());
    let joined = parts.join(",");
    format!("{property}='{joined}'")
}

/// Compiled positive + negative globset pair for a Darkmatter
/// `file(match(...))` constraint.
///
/// Implements `file(match(...))` glob semantics for completion filtering: a
/// path is accepted iff (a) at least one positive pattern matches AND (b) no
/// negative pattern matches. When the constraint contains only negative
/// patterns, every non-rejected path is accepted (`positive: None`).
///
/// `match(...)` is suggestion metadata only — Darkmatter no longer validates
/// against it — so these globs shape completion candidates, not validation.
///
/// Each pattern is added to the globset twice — once with its raw shape
/// (which may contain path separators like `src/**/*.rs`) and once as a
/// `**/`-anchored variant so filename-only patterns like `*.png` continue
/// to match files in subdirectories: `*.md` accepts `docs/api.md` because the
/// resolved filename `api.md` is one of the views tested.
pub(super) struct MatchGlobs {
    positive: Option<GlobSet>,
    negative: GlobSet,
}

impl MatchGlobs {
    pub(super) fn compile(patterns: &[String]) -> Option<Self> {
        let mut positive = GlobSetBuilder::new();
        let mut negative = GlobSetBuilder::new();
        let mut has_positive = false;
        let mut has_negative = false;
        for raw in patterns {
            let (target, is_negative) = match raw.strip_prefix('!') {
                Some(stripped) => (stripped, true),
                None => (raw.as_str(), false),
            };
            let primary = Glob::new(target).ok()?;
            // Anchor filename-only patterns so they match anywhere in the
            // tree. Skip when the pattern already contains a path separator
            // or is itself a recursive prefix.
            let secondary = if target.contains('/') || target.starts_with("**") {
                None
            } else {
                Glob::new(&format!("**/{target}")).ok()
            };
            if is_negative {
                negative.add(primary);
                if let Some(g) = secondary {
                    negative.add(g);
                }
                has_negative = true;
            } else {
                positive.add(primary);
                if let Some(g) = secondary {
                    positive.add(g);
                }
                has_positive = true;
            }
        }
        let positive = if has_positive {
            Some(positive.build().ok()?)
        } else {
            None
        };
        let negative = if has_negative {
            negative.build().ok()?
        } else {
            GlobSetBuilder::new().build().ok()?
        };
        Some(Self { positive, negative })
    }

    /// Returns true when the relative path is accepted by the constraint.
    ///
    /// Both `rel_path` (e.g. `src/lib.rs`) and `file_name` (e.g. `lib.rs`)
    /// are tested so that patterns can target either view. Negation wins:
    /// a negative match against any view rejects the path even if a
    /// positive pattern would accept it.
    pub(super) fn is_match(&self, rel_path: &str, file_name: &str) -> bool {
        if self.negative.is_match(rel_path) || self.negative.is_match(file_name) {
            return false;
        }
        match &self.positive {
            Some(set) => set.is_match(rel_path) || set.is_match(file_name),
            None => true,
        }
    }
}
