//! Schema-aware completion for composition `name=value` setters.
//!
//! Phase 4 of the `2026-05-15-schemas` feature. When the cursor sits in a
//! setter slot of `claudine compose`, `claudine inline-compose`, or
//! `claudine sequence` AND a positional prompt-file argument is already
//! committed, this module consults the prompt's `$schema` declaration via
//! Darkmatter to produce:
//!
//! - **Property names** (before `=`) — required properties first, then
//!   optional, in declaration order. Already-supplied names are filtered out.
//! - **Property values** (after `=`) — enum members for `enum` properties,
//!   filesystem paths constrained by `match(...)` globs for `file` properties.
//!
//! All entry points are pure functions over schema state plus filesystem
//! reads — no shell execution, no provider launches. When the schema cannot
//! be resolved or the file_arg cannot be loaded, every function returns an
//! empty `Vec`, signalling to the caller that the shell's native fallback
//! should take over.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{
    CompletionKind, Constraint, DarkmatterSchemas, EffectiveSchema, PropertyAtom, PropertyDef,
    SchemaShape, SimplifiedSchema, completion as dm_completion,
};

use super::fuzzy;
use super::scopes::{self, ScopeContext};

/// Resolve `file_arg` to a loaded [`EffectiveSchema`].
///
/// Returns `None` when:
/// - the file cannot be located on disk,
/// - the file is not valid Markdown,
/// - the document has no `$schema`,
/// - the schema cannot be parsed or compiled.
///
/// All failure modes are silent: completion is best-effort and falls back to
/// shell-native behavior on any error.
pub(crate) fn load_effective_schema(file_arg: &str, ctx: &ScopeContext) -> Option<EffectiveSchema> {
    let path = resolve_prompt_path(file_arg, ctx)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let markdown: Markdown = Markdown::from(text).with_source(ComposeSource::infer_from_path(&path));
    DarkmatterSchemas::new().effective_for(&markdown).ok()?
}

/// Resolve a setter-token file_arg to an absolute path on disk.
///
/// Tries the following interpretations in order:
/// 1. As-is when absolute.
/// 2. Relative to cwd.
/// 3. Relative to the effective repo root (when one is detected).
///
/// `@`-prefixed magic paths are not expanded — the file_arg here is the
/// value the user has already committed in argv, which the shell completion
/// engine would have rewritten to a concrete path on selection.
fn resolve_prompt_path(file_arg: &str, ctx: &ScopeContext) -> Option<PathBuf> {
    if file_arg.is_empty() {
        return None;
    }
    let trimmed = file_arg.trim_matches(['"', '\'']);
    let raw = Path::new(trimmed);
    if raw.is_absolute() && raw.is_file() {
        return Some(raw.to_path_buf());
    }
    let from_cwd = ctx.cwd.join(raw);
    if from_cwd.is_file() {
        return Some(from_cwd);
    }
    if let Some(root) = scopes::effective_repo_root(ctx) {
        let from_repo = root.join(raw);
        if from_repo.is_file() {
            return Some(from_repo);
        }
    }
    None
}

/// Property name candidates for the cursor partial.
///
/// Returns names in priority order: required properties first (in declaration
/// order), then optional properties (in declaration order). `supplied` is the
/// set of property names already passed on the command line — those are
/// filtered out so the user is never offered to re-set the same key.
///
/// Each candidate is rendered with a trailing `=` so accepting it leaves the
/// cursor positioned to start typing the value.
pub(crate) fn property_names(
    effective: &EffectiveSchema,
    partial: &str,
    supplied: &HashSet<String>,
) -> Vec<String> {
    let Some(shape) = single_shape(effective) else {
        return Vec::new();
    };

    let mut required: Vec<String> = Vec::new();
    let mut optional: Vec<String> = Vec::new();
    for (name, def) in &shape.properties {
        if supplied.contains(name) {
            continue;
        }
        if !name_matches(name, partial) {
            continue;
        }
        if is_required(def) {
            required.push(format!("{name}="));
        } else {
            optional.push(format!("{name}="));
        }
    }

    required.extend(optional);
    required
}

/// Value candidates for `property=<partial>` when the property is in the
/// schema's completable set.
///
/// - `enum` → enum members matching `value_partial` (prefix-insensitive when
///   `value_partial` is non-empty; all members for an empty partial).
/// - `file` → filesystem paths matching the property's `match(...)` globs.
///   Walks the filesystem starting from the effective repo root (or cwd when
///   no repo). Empty `match` patterns return no candidates — fall back to
///   shell-native file completion.
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
            file_candidates(property, patterns, value_partial, ctx)
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

#[cfg(test)]
fn ordered_completable_suggestions(
    effective: &EffectiveSchema,
) -> Vec<darkmatter::markdown::schemas::CompletionSuggestion> {
    dm_completion::completable_properties(effective)
        .into_iter()
        .filter_map(|name| dm_completion::for_property(effective, &name))
        .collect()
}

fn name_matches(name: &str, partial: &str) -> bool {
    if partial.is_empty() {
        return true;
    }
    // Use fuzzy subsequence matching to match the rest of the engine's
    // partial-classification semantics. Short partials behave as starting
    // substrings under `fuzzy_match` (which lowercases both sides).
    fuzzy::fuzzy_match(name, partial)
}

fn single_shape(effective: &EffectiveSchema) -> Option<&SchemaShape> {
    match effective.simplified.as_ref()? {
        SimplifiedSchema::Single(shape) => Some(shape),
        SimplifiedSchema::Union(_) => None,
    }
}

fn is_required(def: &PropertyDef) -> bool {
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(atom) => vec![atom],
        PropertyDef::Union(items) => items.iter().collect(),
    };
    atoms.iter().any(|atom| {
        atom.constraints
            .iter()
            .any(|c| matches!(c, Constraint::Required))
    })
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
    ctx: &ScopeContext,
) -> Vec<String> {
    // Empty `match()` means "any file" — without a pattern the result set
    // would be unbounded, so we return nothing and let the existing
    // `@`-gated path or shell-native file completion handle it.
    if patterns.is_empty() {
        return Vec::new();
    }
    let base: PathBuf = scopes::effective_repo_root(ctx)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ctx.cwd.clone());
    let trimmed = value_partial.trim_matches(['"', '\'']);
    let active = trimmed.trim_start_matches('@');

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let walker = ignore::WalkBuilder::new(&base)
        .hidden(true)
        .git_ignore(true)
        .require_git(false)
        .build();
    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(rel) = path.strip_prefix(&base) else {
            continue;
        };
        let Some(rel_str) = rel.to_str() else {
            continue;
        };
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !matches_any_glob(file_name, patterns) {
            continue;
        }
        if !active.is_empty() && !fuzzy::fuzzy_match(file_name, active) {
            continue;
        }
        let rendered = format!("{property}='{rel_str}'");
        if seen.insert(rendered.clone()) {
            out.push(rendered);
        }
    }
    out.sort();
    out
}

/// Match `name` against a list of `match(...)` patterns, honouring leading
/// `!` as negation. A negated pattern matching the name eliminates it from
/// the result set; an un-negated pattern matching the name accepts it.
///
/// Mirrors Darkmatter's `darkmatter-file` keyword semantics.
fn matches_any_glob(name: &str, patterns: &[String]) -> bool {
    let mut accepted = false;
    for pat in patterns {
        if let Some(neg) = pat.strip_prefix('!') {
            if glob_match(neg, name) {
                return false;
            }
        } else if glob_match(pat, name) {
            accepted = true;
        }
    }
    accepted
}

/// Minimal glob matcher covering `*` and `?` wildcards against a file name.
/// Patterns are expected to be filename-only (no path separators), matching
/// Darkmatter's `file(match='*.md')` convention.
fn glob_match(pattern: &str, name: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), name.as_bytes())
}

fn glob_match_inner(pat: &[u8], name: &[u8]) -> bool {
    let (mut i, mut j, mut star, mut mark) = (0usize, 0usize, None, 0usize);
    while j < name.len() {
        if i < pat.len() && (pat[i] == b'?' || pat[i].eq_ignore_ascii_case(&name[j])) {
            i += 1;
            j += 1;
        } else if i < pat.len() && pat[i] == b'*' {
            star = Some(i);
            mark = j;
            i += 1;
        } else if let Some(s) = star {
            i = s + 1;
            mark += 1;
            j = mark;
        } else {
            return false;
        }
    }
    while i < pat.len() && pat[i] == b'*' {
        i += 1;
    }
    i == pat.len()
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

    fn seed_repo(root: &Path) {
        fs::create_dir_all(root.join(".git")).unwrap();
    }

    fn effective_from_doc(doc: &str) -> EffectiveSchema {
        let md: Markdown = doc.into();
        DarkmatterSchemas::new()
            .effective_for(&md)
            .unwrap()
            .expect("effective schema")
    }

    #[test]
    fn property_names_required_first_then_optional() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  status: 'enum(draft, published; required)'\n",
            "  description: string\n",
            "  count: number\n",
            "---\nbody\n",
        ));
        let got = property_names(&effective, "", &HashSet::new());
        // Required properties (status, title) appear before optional
        // properties (count, description). Within each group the order
        // reflects the IndexMap iteration produced by Darkmatter's
        // JSON-Schema-driven storage, which is alphabetical when the
        // `$schema` was authored as inline YAML.
        let pos = |needle: &str| got.iter().position(|c| c == needle).unwrap();
        assert!(pos("status=") < pos("description="));
        assert!(pos("status=") < pos("count="));
        assert!(pos("title=") < pos("description="));
        assert!(pos("title=") < pos("count="));
        assert_eq!(got.len(), 4);
    }

    #[test]
    fn property_names_filters_supplied() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  description: string\n",
            "---\nbody\n",
        ));
        let mut supplied = HashSet::new();
        supplied.insert("title".to_string());
        let got = property_names(&effective, "", &supplied);
        assert_eq!(got, vec!["description="]);
    }

    #[test]
    fn property_names_fuzzy_matches_partial() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  description: string\n",
            "---\nbody\n",
        ));
        let got = property_names(&effective, "des", &HashSet::new());
        assert_eq!(got, vec!["description="]);
    }

    #[test]
    fn property_value_returns_enum_members() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  status: 'enum(draft, published, archived; required)'\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "status", "", &ctx);
        assert!(got.contains(&"status='draft'".to_string()));
        assert!(got.contains(&"status='published'".to_string()));
        assert!(got.contains(&"status='archived'".to_string()));
    }

    #[test]
    fn property_value_filters_enum_by_partial() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  status: 'enum(draft, published, archived; required)'\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "status", "pub", &ctx);
        assert_eq!(got, vec!["status='published'".to_string()]);
    }

    #[test]
    fn property_value_returns_files_for_match_pattern() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  cover: \"file(match('*.png'))\"\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("assets").join("cover.png"), "");
        write(&tmp.path().join("assets").join("other.jpg"), "");
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "cover", "", &ctx);
        assert!(
            got.iter().any(|c| c.ends_with("cover.png'")),
            "expected cover.png in candidates: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.contains("other.jpg")),
            "non-matching extension must be filtered: {got:?}"
        );
    }

    #[test]
    fn property_value_returns_empty_for_string_property() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "title", "", &ctx);
        assert!(got.is_empty());
    }

    #[test]
    fn property_value_returns_empty_for_file_without_match() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  cover: file\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("a.txt"), "");
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "cover", "", &ctx);
        assert!(got.is_empty(), "no `match` => no candidates: {got:?}");
    }

    #[test]
    fn property_value_hint_returns_format_for_url() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  homepage: url\n",
            "---\nbody\n",
        ));
        let hint = property_value_hint(&effective, "homepage");
        assert!(hint.unwrap_or("").contains("URL"));
    }

    #[test]
    fn load_effective_schema_resolves_cwd_relative_path() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let doc_path = tmp.path().join("prompt.md");
        write(
            &doc_path,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        );
        let ctx = ScopeContext::discover_from(tmp.path());
        let effective = load_effective_schema("prompt.md", &ctx).expect("schema loads");
        let suggestions = ordered_completable_suggestions(&effective);
        // `title` is `string` so it's NOT a completable type.
        assert!(suggestions.is_empty(), "string is not completable");
    }

    #[test]
    fn load_effective_schema_returns_none_for_missing_file() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(load_effective_schema("does-not-exist.md", &ctx).is_none());
    }

    #[test]
    fn load_effective_schema_returns_none_when_no_schema() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("p.md"), "---\ntitle: hi\n---\n");
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(load_effective_schema("p.md", &ctx).is_none());
    }

    #[test]
    fn load_effective_schema_strips_surrounding_quotes() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(
            &tmp.path().join("p.md"),
            "---\n$schema:\n  status: 'enum(a, b)'\n---\n",
        );
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(load_effective_schema("'p.md'", &ctx).is_some());
        assert!(load_effective_schema("\"p.md\"", &ctx).is_some());
    }

    #[test]
    fn glob_match_handles_star_and_question() {
        assert!(glob_match("*.png", "cover.png"));
        assert!(glob_match("*.png", "PHOTO.PNG"));
        assert!(!glob_match("*.png", "cover.jpg"));
        assert!(glob_match("img.?", "img.a"));
        assert!(!glob_match("img.?", "img.abc"));
        assert!(glob_match("*", "anything.txt"));
    }

    #[test]
    fn matches_any_glob_honors_negation() {
        let patterns = vec!["*.md".to_string(), "!_*.md".to_string()];
        assert!(matches_any_glob("plan.md", &patterns));
        assert!(!matches_any_glob("_draft.md", &patterns));
        assert!(!matches_any_glob("notes.txt", &patterns));
    }
}
