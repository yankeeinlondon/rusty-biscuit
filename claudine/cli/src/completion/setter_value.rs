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
//!   and `reviews/` subdirectories at repo root, package-area, and
//!   package levels. Any other leading character returns zero candidates
//!   so the shell's default completion takes over.
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
//! Matched files are rendered as paths relative to the repo root (when a
//! repo is detected via `sniff` or a `.git` ancestor) or the current
//! working directory otherwise. This matches the mental model of a doc
//! path typed into a frontmatter override: it should resolve the same
//! way whether the user typed it by hand or selected it from completion.

#![allow(dead_code)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::fuzzy::{self, PartialLen};
use super::scopes::{Scope, ScopeContext};
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
    let base = repo_or_cwd(ctx);

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
            let canonical =
                std::fs::canonicalize(&entry).unwrap_or_else(|_| entry.clone());
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
/// Scopes are emitted in priority order: repo root, package-area root,
/// package root — each crossed with [`SETTER_VALUE_SUBDIRS`]. The
/// walker tolerates missing directories, so non-existent roots are
/// returned as-is and silently ignored at walk time.
///
/// Symlinks are always followed — none of these scopes are agent-skill
/// peer directories where Claudine's linker produces cross-provider
/// duplicates.
fn resolve_setter_scopes(ctx: &ScopeContext) -> Vec<Scope> {
    let mut scopes: Vec<Scope> = Vec::new();

    let repo_root: Option<&Path> = ctx
        .repo_info
        .as_ref()
        .map(|info| info.root.as_path())
        .or(ctx.git_root.as_deref());

    let area_root: Option<PathBuf> = ctx.repo_info.as_ref().and_then(|info| {
        let area = info.package_area_for_dir(&ctx.cwd)?;
        if area == "root" {
            return None;
        }
        Some(info.root.join(area))
    });

    let pkg_root: Option<PathBuf> = ctx
        .repo_info
        .as_ref()
        .and_then(|info| info.package_for_dir(&ctx.cwd).map(|pkg| pkg.path.clone()));

    let push_scopes = |scopes: &mut Vec<Scope>, base: &Path| {
        for sub in SETTER_VALUE_SUBDIRS {
            scopes.push(Scope {
                path: base.join(sub),
                follow_links: true,
            });
        }
    };

    if let Some(root) = repo_root {
        push_scopes(&mut scopes, root);
    }
    if let Some(root) = area_root.as_deref() {
        push_scopes(&mut scopes, root);
    }
    if let Some(root) = pkg_root.as_deref() {
        push_scopes(&mut scopes, root);
    }

    // When cwd is at the repo root, the area_root / pkg_root branches
    // resolve to the same path as the repo branch, so the same
    // subdirectories would be enumerated twice. Dedup on exact path
    // keeps the walker's work bounded and eliminates double-ranked
    // candidates in `finalize()`.
    let mut seen: HashSet<PathBuf> = HashSet::new();
    scopes.retain(|scope| seen.insert(scope.path.clone()));
    scopes
}

/// Pick the repo root (from `sniff` or git fallback) for rendering.
/// Falls back to the cwd when neither is available — the completer is
/// still useful in a standalone directory, the rendered paths just
/// reflect the cwd's view of the filesystem.
fn repo_or_cwd(ctx: &ScopeContext) -> PathBuf {
    if let Some(info) = ctx.repo_info.as_ref() {
        return info.root.clone();
    }
    if let Some(git) = ctx.git_root.as_deref() {
        return git.to_path_buf();
    }
    ctx.cwd.clone()
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
        let root_manifest =
            format!("[workspace]\nresolver = \"2\"\nmembers = [\n{members_list}\n]\n");
        fs::write(root.join("Cargo.toml"), root_manifest).unwrap();
        for member in members {
            let member_dir = root.join(member);
            fs::create_dir_all(member_dir.join("src")).unwrap();
            let name = member.replace('/', "-");
            fs::write(
                member_dir.join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
            )
            .unwrap();
            fs::write(member_dir.join("src").join("lib.rs"), "").unwrap();
        }
    }

    // -- SetterToken::parse ------------------------------------------------

    #[test]
    fn parse_accepts_plain_setter() {
        let got = SetterToken::parse("spec=@s").unwrap();
        assert_eq!(got.name, "spec");
        assert_eq!(got.raw_value, "@s");
    }

    #[test]
    fn parse_accepts_underscore_and_dash() {
        let got = SetterToken::parse("_foo-bar=val").unwrap();
        assert_eq!(got.name, "_foo-bar");
        assert_eq!(got.raw_value, "val");
    }

    #[test]
    fn parse_accepts_empty_value() {
        let got = SetterToken::parse("spec=").unwrap();
        assert_eq!(got.name, "spec");
        assert_eq!(got.raw_value, "");
    }

    #[test]
    fn parse_rejects_missing_equals() {
        assert!(SetterToken::parse("spec").is_none());
    }

    #[test]
    fn parse_rejects_leading_equals() {
        assert!(SetterToken::parse("=value").is_none());
    }

    #[test]
    fn parse_rejects_digit_first() {
        assert!(SetterToken::parse("1spec=val").is_none());
    }

    #[test]
    fn parse_rejects_bad_char_in_name() {
        assert!(SetterToken::parse("sp ec=val").is_none());
        assert!(SetterToken::parse("sp.ec=val").is_none());
    }

    // -- strip_leading_quote -----------------------------------------------

    #[test]
    fn strip_leading_quote_handles_double_quote() {
        let (body, quote) = strip_leading_quote("\"@s");
        assert_eq!(body, "@s");
        assert_eq!(quote, Some('"'));
    }

    #[test]
    fn strip_leading_quote_handles_single_quote() {
        let (body, quote) = strip_leading_quote("'@s");
        assert_eq!(body, "@s");
        assert_eq!(quote, Some('\''));
    }

    #[test]
    fn strip_leading_quote_unquoted_is_passthrough() {
        let (body, quote) = strip_leading_quote("@s");
        assert_eq!(body, "@s");
        assert_eq!(quote, None);
    }

    #[test]
    fn strip_leading_quote_empty_returns_empty() {
        let (body, quote) = strip_leading_quote("");
        assert_eq!(body, "");
        assert_eq!(quote, None);
    }

    // -- strip_file_extension ----------------------------------------------

    #[test]
    fn strip_file_extension_removes_last_extension() {
        assert_eq!(strip_file_extension("plan.md"), "plan");
        assert_eq!(strip_file_extension("plan.tar.gz"), "plan.tar");
        assert_eq!(strip_file_extension("no-ext"), "no-ext");
    }

    // -- run: @-gated trigger ---------------------------------------------

    #[test]
    fn run_returns_empty_for_non_at_value() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let docs = tmp.path().join("docs");
        write(&docs.join("plan.md"), "# p\n");
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(run("spec=bar", &ctx).is_empty());
    }

    #[test]
    fn run_returns_empty_for_empty_value() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let docs = tmp.path().join("docs");
        write(&docs.join("plan.md"), "# p\n");
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(run("spec=", &ctx).is_empty());
    }

    #[test]
    fn run_returns_empty_for_invalid_shape() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(run("not-a-setter", &ctx).is_empty());
        assert!(run("=value", &ctx).is_empty());
    }

    // -- run: repo-level scopes -------------------------------------------

    #[test]
    fn run_surfaces_repo_docs_under_at_sigil() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let docs = tmp.path().join("docs");
        write(&docs.join("spec.md"), "# s\n");
        write(&docs.join("unrelated.md"), "# u\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run("spec=@s", &ctx);
        assert!(
            got.iter().any(|c| c == "spec='docs/spec.md'"),
            "expected spec='docs/spec.md' in {got:?}"
        );
        // `u` is not a subsequence of `s` so unrelated.md should not match.
        assert!(
            !got.iter().any(|c| c.contains("unrelated.md")),
            "unrelated.md should not match on `s` prefix: {got:?}"
        );
    }

    #[test]
    fn run_wraps_value_in_single_quotes_even_when_user_typed_double() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let docs = tmp.path().join("docs");
        write(&docs.join("spec.md"), "# s\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run("spec=\"@s", &ctx);
        assert!(
            got.iter().all(|c| !c.contains('"')),
            "double quote must be normalized to single quote: {got:?}"
        );
        assert!(
            got.iter().any(|c| c == "spec='docs/spec.md'"),
            "expected spec='docs/spec.md' after quote normalization: {got:?}"
        );
    }

    #[test]
    fn run_wraps_value_in_single_quotes_when_user_typed_single() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let docs = tmp.path().join("docs");
        write(&docs.join("spec.md"), "# s\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run("spec='@s", &ctx);
        assert!(
            got.iter().any(|c| c == "spec='docs/spec.md'"),
            "expected spec='docs/spec.md' with single-quoted body: {got:?}"
        );
    }

    // -- run: empty @ returns everything in scope -------------------------

    #[test]
    fn run_empty_at_surfaces_every_file_in_scope() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        write(&tmp.path().join("docs").join("one.md"), "# 1\n");
        write(&tmp.path().join("features").join("two.md"), "# 2\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run("ref=@", &ctx);
        assert!(got.iter().any(|c| c == "ref='docs/one.md'"), "{got:?}");
        assert!(got.iter().any(|c| c == "ref='features/two.md'"), "{got:?}");
    }

    // -- run: scope resolution (repo + area + package) --------------------

    #[test]
    fn run_cwd_at_repo_root_only_repo_scope() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        write(&tmp.path().join("docs").join("plan.md"), "# p\n");
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run("spec=@p", &ctx);
        assert_eq!(got, vec!["spec='docs/plan.md'"]);
    }

    #[test]
    fn run_cwd_inside_package_area_uses_area_scope() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["claudine/lib"]);
        // Area-level feature doc.
        let area_feat = tmp.path().join("claudine").join("features");
        write(&area_feat.join("plan.md"), "# p\n");

        // cwd is the area directory itself.
        let ctx = ScopeContext::discover_from(&tmp.path().join("claudine"));
        let got = run("spec=@p", &ctx);
        assert!(
            got.iter().any(|c| c == "spec='claudine/features/plan.md'"),
            "expected area-scope plan: {got:?}"
        );
    }

    #[test]
    fn run_cwd_inside_package_uses_package_scope() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["claudine/lib"]);
        // Package-level doc.
        let pkg_docs = tmp.path().join("claudine").join("lib").join("docs");
        write(&pkg_docs.join("pkg.md"), "# x\n");

        // cwd is the package directory.
        let ctx =
            ScopeContext::discover_from(&tmp.path().join("claudine").join("lib"));
        let got = run("ref=@pk", &ctx);
        assert!(
            got.iter().any(|c| c == "ref='claudine/lib/docs/pkg.md'"),
            "expected package-scope doc: {got:?}"
        );
    }

    #[test]
    fn run_returns_empty_when_no_matches() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        write(&tmp.path().join("docs").join("unrelated.md"), "# u\n");
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run("spec=@xyz", &ctx);
        assert!(
            got.is_empty(),
            "expected no matches for 'xyz' query: {got:?}"
        );
    }

    #[test]
    fn run_excludes_directories() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        fs::create_dir_all(tmp.path().join("docs").join("planning")).unwrap();
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run("spec=@pla", &ctx);
        // Even with a 3+ char prefix we never emit directories — setter
        // values target files only.
        assert!(
            !got.iter().any(|c| c.contains("planning") && c.ends_with("/'")),
            "directory should not appear in setter-value output: {got:?}"
        );
    }

    #[test]
    fn run_excludes_underscore_prefixed_files() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let docs = tmp.path().join("docs");
        write(&docs.join("_draft.md"), "# d\n");
        write(&docs.join("published.md"), "# p\n");
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = run("spec=@", &ctx);
        assert!(
            !got.iter().any(|c| c.contains("_draft")),
            "underscore-prefixed files must be elided: {got:?}"
        );
        assert!(
            got.iter().any(|c| c == "spec='docs/published.md'"),
            "non-underscore file must surface: {got:?}"
        );
    }

    #[test]
    fn run_multi_scope_orders_repo_before_area() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["claudine/lib"]);
        write(&tmp.path().join("docs").join("plan.md"), "# p\n");
        write(
            &tmp.path().join("claudine").join("docs").join("plan.md"),
            "# p2\n",
        );

        let ctx =
            ScopeContext::discover_from(&tmp.path().join("claudine").join("lib"));
        let got = run("spec=@p", &ctx);
        // Both files should appear; repo-scope first (rank 0), package-
        // area and package-scope after.
        assert!(got.iter().any(|c| c == "spec='docs/plan.md'"), "{got:?}");
        assert!(
            got.iter().any(|c| c == "spec='claudine/docs/plan.md'"),
            "{got:?}"
        );
        let idx_repo = got.iter().position(|c| c == "spec='docs/plan.md'").unwrap();
        let idx_area = got
            .iter()
            .position(|c| c == "spec='claudine/docs/plan.md'")
            .unwrap();
        assert!(
            idx_repo < idx_area,
            "repo scope must sort before area scope: {got:?}"
        );
    }

    // -- scope resolution helpers -----------------------------------------

    #[test]
    fn resolve_setter_scopes_includes_all_four_subdirs() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let ctx = ScopeContext::discover_from(tmp.path());
        let scopes = resolve_setter_scopes(&ctx);
        for sub in SETTER_VALUE_SUBDIRS {
            let expected = tmp.path().join(sub);
            assert!(
                scopes.iter().any(|s| s.path == expected),
                "missing scope {sub}: {scopes:?}"
            );
        }
    }

    #[test]
    fn resolve_setter_scopes_dedups_when_area_equals_repo() {
        // cwd at repo root: package_area_for_dir() returns None, so
        // no area-level scopes are produced. But the dedup logic also
        // protects against the case where they DO coincide.
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let ctx = ScopeContext::discover_from(tmp.path());
        let scopes = resolve_setter_scopes(&ctx);
        // No duplicate paths.
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for scope in &scopes {
            assert!(
                seen.insert(scope.path.clone()),
                "duplicate scope path: {scope:?}"
            );
        }
    }

    #[test]
    fn resolve_setter_scopes_all_follow_links() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let ctx = ScopeContext::discover_from(tmp.path());
        let scopes = resolve_setter_scopes(&ctx);
        for scope in &scopes {
            assert!(
                scope.follow_links,
                "setter scope must follow links: {scope:?}"
            );
        }
    }
}
