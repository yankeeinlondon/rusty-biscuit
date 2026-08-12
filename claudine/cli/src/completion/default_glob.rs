//! Default-glob file candidate gatherer for bare `file`/`file[]` schema
//! properties and frontmatter `file`/`file[]` choosers.
//!
//! Phase 1 of the `2026-06-14-auto-complete` feature. When a schema property
//! is typed `file`/`file[]` with no `match(...)` patterns, or when a
//! frontmatter `file`/`file[]` value is collected interactively, this module
//! supplies the fallback candidate set.
//!
//! The walk obeys the same cross-cutting rules as the shell-completion
//! walker:
//!
//! - `.gitignore` / `.git/info/exclude` / global gitignore are honored.
//! - Files and directories whose name starts with `_` are elided.
//! - Curated build/cache directories are pruned.
//! - Every prompt directory surfaced by [`ScopeSet`] is excluded so
//!   compose-associated prompts never leak into generic `file` values.
//! - Results are capped at [`MAX_CANDIDATES`].

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ignore::{DirEntry, WalkBuilder};

use super::scopes::{self, ComposeMode, ScopeContext, ScopeKind};
use super::walker::{self, MAX_CANDIDATES};

/// Gather markdown candidates from the default glob.
///
/// Walks the invoking `cwd` (the launch area; see
/// [`scopes::property_value_root`]), honoring git ignore files and excluding
/// prompt directories. Returns absolute paths; callers relativize as needed.
pub(crate) fn default_markdown_candidates(ctx: &ScopeContext) -> Vec<PathBuf> {
    default_markdown_candidates_filtered(ctx, |_| true)
}

/// Filtered variant that only returns paths matching `predicate`.
///
/// The predicate is applied after the default filters, so git-ignored,
/// `_`-prefixed, build-directory, and prompt-directory entries are never
/// seen. The [`MAX_CANDIDATES`] cap counts only predicate-matching files.
pub(crate) fn default_markdown_candidates_filtered<P>(
    ctx: &ScopeContext,
    predicate: P,
) -> Vec<PathBuf>
where
    P: Fn(&Path) -> bool,
{
    let base = scopes::property_value_root(ctx).to_path_buf();
    let excluded = prompt_dirs(ctx);
    let excluded_for_filter = excluded.clone();

    let mut out: Vec<PathBuf> = Vec::new();
    let walker = WalkBuilder::new(&base)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(move |entry| passes_default_filters(entry, &excluded_for_filter))
        .build();

    for result in walker {
        if out.len() >= MAX_CANDIDATES {
            break;
        }
        let Ok(entry) = result else { continue };
        if entry.depth() == 0 {
            continue;
        }
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if !has_markdown_extension(path) {
            continue;
        }
        if !predicate(path) {
            continue;
        }
        out.push(path.to_path_buf());
    }

    out
}

/// Return every prompt directory surfaced by the current [`ScopeSet`].
///
/// These directories are pruned from the default glob so generic `file`
/// values do not accidentally reference composition prompts.
fn prompt_dirs(ctx: &ScopeContext) -> HashSet<PathBuf> {
    let scope_set = scopes::resolve_compose_scopes(ctx, ComposeMode::Compose);
    scope_set
        .iter_scopes()
        .filter(|scope| {
            matches!(
                scope.kind,
                ScopeKind::RepoPrompts
                    | ScopeKind::PackageAreaPrompts
                    | ScopeKind::PackagePrompts
                    | ScopeKind::RepoClaudinePrompts
                    | ScopeKind::UserClaudinePrompts
            )
        })
        .map(|scope| scope.path.clone())
        .collect()
}

/// Entry filter used by the default-glob walker.
///
/// Shares the `_`-prefix and curated-skip-list exclusion with the scope
/// walker via [`walker::entry_passes_filters`], then adds the default-glob-only
/// rule that an entry must not sit inside an excluded prompt directory.
fn passes_default_filters(entry: &DirEntry, excluded: &HashSet<PathBuf>) -> bool {
    if !walker::entry_passes_filters(entry) {
        return false;
    }
    let path = entry.path();
    !excluded.iter().any(|excluded_dir| path.starts_with(excluded_dir))
}

/// Accept `.md` and `.markdown` extensions case-insensitively.
fn has_markdown_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
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

    fn seed_cargo_workspace(root: &Path, members: &[&str]) {
        seed_repo(root);
        let members_list = members
            .iter()
            .map(|m| format!("    \"{m}\""))
            .collect::<Vec<_>>()
            .join(",\n");
        let manifest = format!("[workspace]\nresolver = \"2\"\nmembers = [\n{members_list}\n]\n");
        fs::write(root.join("Cargo.toml"), manifest).unwrap();
        for member in members {
            let dir = root.join(member).join("src");
            fs::create_dir_all(&dir).unwrap();
            let name = member.replace('/', "-");
            fs::write(
                root.join(member).join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
            )
            .unwrap();
            fs::write(dir.join("lib.rs"), "").unwrap();
        }
    }

    #[test]
    fn surfaces_markdown_at_repo_root() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("readme.md"), "# Readme\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = default_markdown_candidates(&ctx);
        assert!(got.iter().any(|p| p.ends_with("readme.md")), "got: {got:?}");
    }

    #[test]
    fn excludes_prompt_directories() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        write(&tmp.path().join("prompts").join("plan.md"), "# Plan\n");
        write(&tmp.path().join("docs").join("spec.md"), "# Spec\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = default_markdown_candidates(&ctx);
        assert!(
            !got.iter().any(|p| p.components().any(|c| c.as_os_str() == "prompts")),
            "prompt dir must be excluded: {got:?}"
        );
        assert!(got.iter().any(|p| p.ends_with("spec.md")), "docs/spec.md must surface: {got:?}");
    }

    #[test]
    fn excludes_underscore_prefixed_files_and_dirs() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("_draft.md"), "# Draft\n");
        write(&tmp.path().join("_wip").join("notes.md"), "# Notes\n");
        write(&tmp.path().join("public.md"), "# Public\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = default_markdown_candidates(&ctx);
        assert!(
            !got.iter().any(|p| p.file_name().is_some_and(|n| n.to_str() == Some("_draft.md"))),
            "underscore file leaked: {got:?}"
        );
        assert!(
            !got.iter().any(|p| p.components().any(|c| c.as_os_str() == "_wip")),
            "underscore dir leaked: {got:?}"
        );
        assert!(got.iter().any(|p| p.ends_with("public.md")), "public.md missing: {got:?}");
    }

    #[test]
    fn honors_gitignore() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        fs::write(tmp.path().join(".gitignore"), "ignored/\n").unwrap();
        write(&tmp.path().join("kept.md"), "# Kept\n");
        write(&tmp.path().join("ignored").join("secret.md"), "# Secret\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = default_markdown_candidates(&ctx);
        assert!(got.iter().any(|p| p.ends_with("kept.md")), "kept.md missing: {got:?}");
        assert!(
            !got.iter().any(|p| p.components().any(|c| c.as_os_str() == "ignored")),
            "gitignored dir leaked: {got:?}"
        );
    }

    #[test]
    fn filtered_variant_counts_matches_not_discoveries() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("plan.md"), "# Plan\n");
        write(&tmp.path().join("notes.md"), "# Notes\n");
        write(&tmp.path().join("other.md"), "# Other\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = default_markdown_candidates_filtered(&ctx, |p| {
            p.to_str()
                .map(|s| s.to_ascii_lowercase().contains("plan"))
                .unwrap_or(false)
        });
        assert_eq!(got.len(), 1, "only plan.md matches: {got:?}");
        assert!(got[0].ends_with("plan.md"));
    }
}
