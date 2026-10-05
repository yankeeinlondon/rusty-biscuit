//! Value-candidate matching for schema `name=value` setters.
//!
//! Resolves the value slot after `=`: enum members for `enum` properties and
//! filesystem paths constrained by `match(...)` globs for `file` properties.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use biscuit_file::{
    FileReference, FileResolutionContext, PathIdentity, PortabilityPreference, PortablePath,
    canonicalize_simplified, to_portable_string,
};

use darkmatter::markdown::schemas::{completion as dm_completion, CompletionKind, EffectiveSchema};

use crate::completion::default_glob;
use crate::completion::scopes::{self, ScopeContext};
use crate::completion::walker;

/// Value candidates for `property=<partial>` when the property is in the
/// schema's completable set.
///
/// - `enum` → enum members matching `value_partial` (prefix-insensitive when
///   `value_partial` is non-empty; all members for an empty partial).
/// - `file` → files the property's `match(...)` globs admit, walked from the
///   globs' roots in precedence order (see [`match_glob_files`]) and spelled
///   as [`candidate_value`] spells them. A bare `file` with no `match`
///   patterns falls back to the default Markdown glob under the launch
///   directory.
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

    let Ok(globs) = MatchGlobs::new(patterns) else {
        return Vec::new();
    };
    let Ok(resolution) = scopes::file_resolution_context(ctx) else {
        return Vec::new();
    };
    for path in match_glob_files(&globs, &resolution) {
        let Some(value) = candidate_value(&path, &resolution) else {
            continue;
        };
        // The typed partial filters the spelling that will be inserted, so a
        // directory fragment narrows candidates that share a basename (every
        // `**/*spec*.md` hit is `spec.md`).
        if !active.is_empty() && !contains_ci(&value, &active) {
            continue;
        }
        if !excluded.is_empty() && excluded.contains(&value) {
            continue;
        }
        let rendered = if is_array {
            format_array_candidate(property, &prefix_segments, &value)
        } else {
            format!("{property}='{value}'")
        };
        if seen.insert(rendered.clone()) {
            out.push(rendered);
        }
    }
    out
}

/// Gather absolute file paths for a schema `file`/`file[]` property.
///
/// - `patterns` empty → [`default_glob::default_markdown_candidates`], sorted
///   (bare `file`/`file[]` fallback).
/// - `patterns` non-empty → every file [`match_glob_files`] finds, in native
///   order.
///
/// Used by the ENTER-path missing-property chooser. TAB completion uses
/// [`file_candidates`] instead because it needs formatted setter tokens and
/// array-continuation exclusion.
pub(crate) fn file_candidate_paths(patterns: &[String], ctx: &ScopeContext) -> Vec<PathBuf> {
    if patterns.is_empty() {
        let mut paths = default_glob::default_markdown_candidates(ctx);
        paths.sort();
        return paths;
    }
    let Ok(globs) = MatchGlobs::new(patterns) else {
        return Vec::new();
    };
    let Ok(resolution) = scopes::file_resolution_context(ctx) else {
        return Vec::new();
    };
    match_glob_files(&globs, &resolution)
}

/// Every file `globs` admit, for a suggestion list.
///
/// Walks each of the globs' roots in precedence order with the completion
/// filters (hidden, gitignored, `_`-prefixed, [`walker::SKIP_DIRS`]), which
/// shape suggestions only; validation still admits a filtered file a user
/// types. Each file is examined once, in the first root's walk that reaches
/// it, which is the most local root that contains it, and judged by
/// [`MatchGlobs::lists_file`] (nearest-root judgment), so a later root never
/// re-offers a file an earlier root's pattern excluded. A file symlink whose
/// target leaves the tree is omitted without a warning: a suggestion list is
/// not a place for warnings.
///
/// The result is in native order: root precedence, then fewest components
/// below the root, then component-wise, as `GlobReference::list_files`
/// orders its matches.
fn match_glob_files(globs: &MatchGlobs, resolution: &FileResolutionContext) -> Vec<PathBuf> {
    let mut examined: HashSet<PathIdentity> = HashSet::new();
    let mut out: Vec<PathBuf> = Vec::new();
    for root in globs.roots(resolution) {
        let Ok(canonical_root) = canonicalize_simplified(&root) else {
            continue;
        };
        let mut pass: Vec<(Vec<OsString>, PathBuf)> = Vec::new();
        let walker = ignore::WalkBuilder::new(&root)
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
            let Ok(below_root) = path.strip_prefix(&root) else {
                continue;
            };
            // Directory symlinks are not followed, so the canonical root plus
            // the walked names is the file's identity; a file symlink keeps
            // its own name.
            if !examined.insert(PathIdentity::new(&canonical_root.join(below_root))) {
                continue;
            }
            if !globs.lists_file(path, resolution) {
                continue;
            }
            let names = below_root.iter().map(OsString::from).collect();
            pass.push((names, path.to_path_buf()));
        }
        pass.sort_by(|(left, _), (right, _)| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));
        out.extend(pass.into_iter().map(|(_, path)| path));
    }
    out
}

/// The forms a candidate outside the launch directory may take, most
/// preferred first. `{{VAR}}` is left out: a placeholder reads oddly in a
/// shell argument.
const CANDIDATE_STRATEGY: [PortabilityPreference; 7] = [
    PortabilityPreference::SameDirRelative,
    PortabilityPreference::ChildDir,
    PortabilityPreference::ImmediateParentDir,
    PortabilityPreference::PeerDir,
    PortabilityPreference::RepoRoot(None),
    PortabilityPreference::HomeDir,
    PortabilityPreference::AbsolutePath,
];

/// The value a completion inserts for `path`, verified to resolve back to it
/// in `resolution`; `None` when no form does.
///
/// A file under the launch directory keeps its bare launch-relative spelling
/// (`fixes/x/spec.md`, never `./fixes/x/spec.md`): `PortablePath` keeps a
/// relative input that already spells the route from `cwd`. Any other file
/// takes the first [`CANDIDATE_STRATEGY`] form that verifies, so a spelling a
/// closer root would shadow falls through to a later form.
///
/// `path` must be spelled as the walk found it, under a root of
/// `resolution`: `PortablePath` compares spellings, so a canonical
/// `/private/var/…` target would share no route with a `/var/…` launch
/// directory.
fn candidate_value(path: &Path, resolution: &FileResolutionContext) -> Option<String> {
    let launch_relative = PathIdentity::new(path)
        .relative_from(&PathIdentity::new(resolution.cwd()))
        .filter(|route| route.parent_hops() == 0 && !route.forward().is_empty())
        .and_then(|route| FileReference::new(&to_portable_string(&route.to_path_buf())).ok());
    let portable = match launch_relative {
        Some(reference) => PortablePath::from_reference(reference).with_ctx(resolution),
        None => PortablePath::from_path(path).with_ctx(resolution),
    };
    portable
        .with_strategy(CANDIDATE_STRATEGY)
        .file_reference()
        .ok()
        .map(|portable| portable.reference().raw().to_string())
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

/// Substring-match `active` against `path` relative to `base`, the launch
/// directory (falling back to the basename when `path` is not under `base`).
///
/// Used by the bare-`file` default-glob branch so that, like the match-glob
/// branch, it filters on the path that will be inserted: what the user types
/// is a fragment of that path, and shared basenames make basename-only
/// matching useless.
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

/// Compiled `file(match(...))` globs. Darkmatter owns the comparison so the
/// candidates offered here are exactly the files a root-union arm that
/// contests the glob accepts.
pub(super) use darkmatter::markdown::schemas::file_match::FileMatchGlobs as MatchGlobs;
