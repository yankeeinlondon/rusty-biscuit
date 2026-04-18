//! File-reference token classification and candidate discovery.
//!
//! Phase 2 of the `2026-04-17-file-completion` feature implements:
//!
//! - token classification (`SetterPartial`, `Bare`, `DotRelative`,
//!   `DotDotRelative`, `Magic`, `Package`, `Unsupported`) using the spec's
//!   strict setter regex `^[A-Za-z_][A-Za-z0-9_]*=`;
//! - repo-context discovery via `sniff::filesystem::repo::detect_repo_structure`
//!   plus `RepoInfo::package_area_for_dir`;
//! - scope-specific candidate discovery (bare landing menu, `./` / `../`,
//!   `@`, `!`);
//! - the bounded walker with depth, candidate, file-size, and skip-list
//!   caps plus deterministic source-rank deduplication.
//!
//! The validators that filter candidates by mode-specific contracts (markdown
//! extension for `compose`, `prompt:` frontmatter for `inline-compose`,
//! `sequence:` frontmatter for `sequence`) land in Phase 3 and live in
//! [`super::validate`]. Phase 2 emits every walked entry; Phase 3 wraps the
//! emission with a per-mode validator.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use sniff::filesystem::repo::{RepoInfo, detect_repo_structure};

/// Maximum recursion depth for `@` / `!` walks. Children of the walk root sit
/// at depth 1; a value of 4 means we surface entries up to four directories
/// deep below the root.
pub(crate) const MAX_RECURSION_DEPTH: usize = 4;

/// Cap on raw entries collected by the walker before deduplication. The
/// post-dedup candidate list is always at most this size.
pub(crate) const MAX_CANDIDATES: usize = 500;

/// Maximum file size, in bytes, for any file the validator layer is willing
/// to open. Larger files are skipped without reading their bodies. Declared
/// here so all walker-related limits live in one place; consumed by
/// [`super::validate`].
pub(crate) const MAX_FRONTMATTER_BYTES: u64 = 1024 * 1024;

/// Directory names the walker must never descend into. Treated as a curated
/// performance and relevance safeguard, not a security boundary.
pub(crate) const SKIP_DIR_NAMES: &[&str] = &[
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

/// Classification of the user's current partial token.
///
/// Determined entirely from the partial in isolation — no inspection of the
/// surrounding command line, the repo context, or earlier positionals. Each
/// class drives a distinct discovery path in [`discover_candidates`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenClass {
    /// `^[A-Za-z_][A-Za-z0-9_]*=` — a `key=value` setter; suppresses
    /// completion entirely.
    SetterPartial,
    /// Empty or anything that is not classified as one of the other forms.
    /// Drives the curated landing menu union.
    Bare,
    /// Begins with `./` — list immediate cwd children.
    DotRelative,
    /// Begins with `../` — list immediate parent-of-cwd children.
    DotDotRelative,
    /// Begins with `@` — recursive walk of repo root and `~/.claudine/{prompts,sequences}`.
    Magic,
    /// Begins with `!` — recursive walk of the current package-area root.
    Package,
    /// Prefixes that v1 deliberately does not complete (`vault:`, absolute
    /// `/`, `%`, `{{…}}`).
    Unsupported,
}

/// A single candidate produced by the walker, before deduplication.
#[derive(Debug, Clone)]
pub(crate) struct FileCompletionEntry {
    /// The exact token the shell will insert when this candidate is chosen.
    /// Directories carry a trailing `/`.
    pub value: String,
    /// Absolute filesystem path for the validator layer to inspect.
    pub resolved_path: PathBuf,
    /// True when the resolved path is a directory. Used by the Phase 3
    /// validator dispatch to short-circuit directory entries (which always
    /// pass through — one `<TAB>` should keep descending).
    pub is_dir: bool,
    /// Source rank for deduplication; lower wins on equal `value`.
    /// 0 = cwd-local bare, 1 = package-area, 2 = repo-wide magic,
    /// 3 = home-scoped magic.
    pub source_rank: u8,
}

/// Lightweight repo awareness for completion. Built once per completion
/// request via [`Self::discover`] and threaded through the discovery
/// helpers so they all see the same view of the filesystem.
///
/// `repo` and `current_package_area` are populated for completeness even
/// though Phase 2 reads only the derived `repo_root` and
/// `current_package_area_root`. They give Phase 3 + later iterations cheap
/// access to the full `RepoInfo` (e.g. listing all package areas for the
/// future cross-area landing menu) without re-walking the repo.
#[derive(Debug, Clone)]
pub(crate) struct CompletionRepoContext {
    pub cwd: PathBuf,
    pub repo_root: Option<PathBuf>,
    #[allow(dead_code)]
    pub repo: Option<RepoInfo>,
    #[allow(dead_code)]
    pub current_package_area: Option<String>,
    pub current_package_area_root: Option<PathBuf>,
}

impl CompletionRepoContext {
    /// Discover repo context using the process's current working directory.
    /// Falls back to `"."` when the CWD cannot be read.
    pub(crate) fn discover() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::for_cwd(cwd)
    }

    /// Discover repo context for an explicit `cwd`. The test-friendly entry
    /// point so unit tests can swap in fixture trees without touching the
    /// process's working directory.
    pub(crate) fn for_cwd(cwd: PathBuf) -> Self {
        let repo = detect_repo_structure(&cwd).ok().flatten();
        let repo_root = repo.as_ref().map(|r| r.root.clone());
        let current_package_area = repo
            .as_ref()
            .and_then(|info| info.package_area_for_dir(&cwd).map(str::to_string));
        let current_package_area_root = match (&repo_root, &current_package_area) {
            (Some(root), Some(area)) if area != "root" => Some(root.join(area)),
            (Some(root), Some(_)) => Some(root.clone()),
            _ => None,
        };
        Self {
            cwd,
            repo_root,
            repo,
            current_package_area,
            current_package_area_root,
        }
    }
}

/// Classify the user's current partial token.
///
/// Pure function over the partial only — no filesystem or repo lookup. The
/// classifier never returns more than one class; the order of checks below
/// matches the precedence listed in the tech design.
pub(crate) fn classify_token(partial: &str) -> TokenClass {
    if matches_setter(partial) {
        return TokenClass::SetterPartial;
    }
    if partial.starts_with('@') {
        return TokenClass::Magic;
    }
    if partial.starts_with('!') {
        return TokenClass::Package;
    }
    if partial.starts_with("./") {
        return TokenClass::DotRelative;
    }
    if partial.starts_with("../") {
        return TokenClass::DotDotRelative;
    }
    if is_unsupported_prefix(partial) {
        return TokenClass::Unsupported;
    }
    TokenClass::Bare
}

/// Top-level discovery dispatcher. Returns deduplicated, source-rank-sorted
/// entries ready to be filtered by the mode-specific validator layer.
pub(crate) fn discover_candidates(
    partial: &str,
    ctx: &CompletionRepoContext,
) -> Vec<FileCompletionEntry> {
    let raw = match classify_token(partial) {
        TokenClass::SetterPartial | TokenClass::Unsupported => return Vec::new(),
        TokenClass::Bare => discover_bare(partial, ctx),
        TokenClass::DotRelative => discover_dot_relative(partial, ctx, "./"),
        TokenClass::DotDotRelative => discover_dot_relative(partial, ctx, "../"),
        TokenClass::Magic => discover_magic(partial, ctx),
        TokenClass::Package => discover_package(partial, ctx),
    };
    dedup_and_sort(raw)
}

/// Implementation of the spec's strict setter regex
/// `^[A-Za-z_][A-Za-z0-9_]*=` without pulling in the regex crate. The
/// completion path is hot enough that a hand-rolled byte scan is preferable
/// to a compiled regex.
fn matches_setter(token: &str) -> bool {
    let bytes = token.as_bytes();
    let Some(eq_pos) = bytes.iter().position(|&b| b == b'=') else {
        return false;
    };
    if eq_pos == 0 {
        return false;
    }
    let head = &bytes[..eq_pos];
    if !(head[0].is_ascii_alphabetic() || head[0] == b'_') {
        return false;
    }
    head[1..]
        .iter()
        .all(|&c| c.is_ascii_alphanumeric() || c == b'_')
}

/// Prefixes the spec calls out as explicitly unsupported in v1.
fn is_unsupported_prefix(partial: &str) -> bool {
    partial.starts_with("vault:")
        || partial.starts_with('/')
        || partial.starts_with('%')
        || partial.starts_with("{{")
}

fn discover_bare(partial: &str, ctx: &CompletionRepoContext) -> Vec<FileCompletionEntry> {
    let mut out = Vec::new();
    let mut budget = MAX_CANDIDATES;

    // 1. cwd immediate children → bare values, rank 0
    list_immediate(&ctx.cwd, &mut |path, is_dir, name| {
        let value = render_value("", name, is_dir);
        if value.starts_with(partial) {
            out.push(FileCompletionEntry {
                value,
                resolved_path: path.to_path_buf(),
                is_dir,
                source_rank: 0,
            });
        }
    });

    // 2. repo-area landing entries, rank 1
    if let Some(area_root) = ctx.current_package_area_root.as_ref() {
        for (sub, prefix) in [("prompts", "!prompts/"), ("sequences", "!sequences/")] {
            let dir = area_root.join(sub);
            list_immediate(&dir, &mut |path, is_dir, name| {
                let value = render_value(prefix, name, is_dir);
                if value.starts_with(partial) {
                    out.push(FileCompletionEntry {
                        value,
                        resolved_path: path.to_path_buf(),
                        is_dir,
                        source_rank: 1,
                    });
                }
            });
        }
    }

    // 3. repo root immediate children + repo prompts/sequences depth-1, rank 2
    if let Some(repo_root) = ctx.repo_root.as_ref() {
        list_immediate(repo_root, &mut |path, is_dir, name| {
            let value = render_value("@", name, is_dir);
            if value.starts_with(partial) {
                out.push(FileCompletionEntry {
                    value,
                    resolved_path: path.to_path_buf(),
                    is_dir,
                    source_rank: 2,
                });
            }
        });

        for (sub, prefix) in [("prompts", "@prompts/"), ("sequences", "@sequences/")] {
            let dir = repo_root.join(sub);
            list_immediate(&dir, &mut |path, is_dir, name| {
                let value = render_value(prefix, name, is_dir);
                if value.starts_with(partial) {
                    out.push(FileCompletionEntry {
                        value,
                        resolved_path: path.to_path_buf(),
                        is_dir,
                        source_rank: 2,
                    });
                }
            });
        }
    }

    // The bare path runs only depth-1 listings, so the global budget is
    // unlikely to bite, but we still cap defensively for huge cwds.
    if out.len() > budget {
        out.truncate(budget);
    }
    let _ = &mut budget;
    out
}

fn discover_dot_relative(
    partial: &str,
    ctx: &CompletionRepoContext,
    leading: &str,
) -> Vec<FileCompletionEntry> {
    let after_lead = &partial[leading.len()..];
    let (base_rel, file_prefix) = if after_lead.is_empty() || after_lead.ends_with('/') {
        (Path::new(after_lead.trim_end_matches('/')), "")
    } else {
        let p = Path::new(after_lead);
        let parent = p.parent().unwrap_or_else(|| Path::new(""));
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        (parent, name)
    };

    let parent_dir = match leading {
        "../" => ctx
            .cwd
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| ctx.cwd.clone()),
        _ => ctx.cwd.clone(),
    };
    let search_dir = parent_dir.join(base_rel);

    // Reconstruct the prefix so the emitted value preserves what the user
    // already typed. e.g. partial "./foo/bar" → display_prefix "./foo/".
    let mut display_prefix = String::from(leading);
    if !base_rel.as_os_str().is_empty() {
        if let Some(s) = base_rel.to_str() {
            display_prefix.push_str(s);
            display_prefix.push('/');
        } else {
            return Vec::new();
        }
    }

    let mut out = Vec::new();
    list_immediate(&search_dir, &mut |path, is_dir, name| {
        if !name.starts_with(file_prefix) {
            return;
        }
        let value = render_value(&display_prefix, name, is_dir);
        out.push(FileCompletionEntry {
            value,
            resolved_path: path.to_path_buf(),
            is_dir,
            source_rank: 0,
        });
    });
    out
}

fn discover_magic(partial: &str, ctx: &CompletionRepoContext) -> Vec<FileCompletionEntry> {
    let mut out = Vec::new();
    let mut budget = MAX_CANDIDATES;

    if let Some(repo_root) = ctx.repo_root.as_ref() {
        for (path, is_dir, _depth) in collect_walk(repo_root, MAX_RECURSION_DEPTH, &mut budget) {
            let Ok(rel) = path.strip_prefix(repo_root) else {
                continue;
            };
            let Some(rel_str) = relative_to_value(rel) else {
                continue;
            };
            if rel_str.is_empty() {
                continue;
            }
            let value = render_value("@", &rel_str, is_dir);
            if value.starts_with(partial) {
                out.push(FileCompletionEntry {
                    value,
                    resolved_path: path,
                    is_dir,
                    source_rank: 2,
                });
            }
        }
    }

    if let Some(home) = dirs::home_dir() {
        for (sub, prefix) in [("prompts", "@prompts/"), ("sequences", "@sequences/")] {
            let root = home.join(".claudine").join(sub);
            for (path, is_dir, _depth) in collect_walk(&root, MAX_RECURSION_DEPTH, &mut budget) {
                let Ok(rel) = path.strip_prefix(&root) else {
                    continue;
                };
                let Some(rel_str) = relative_to_value(rel) else {
                    continue;
                };
                if rel_str.is_empty() {
                    continue;
                }
                let value = render_value(prefix, &rel_str, is_dir);
                if value.starts_with(partial) {
                    out.push(FileCompletionEntry {
                        value,
                        resolved_path: path,
                        is_dir,
                        source_rank: 3,
                    });
                }
            }
        }
    }

    out
}

fn discover_package(partial: &str, ctx: &CompletionRepoContext) -> Vec<FileCompletionEntry> {
    let Some(area_root) = ctx.current_package_area_root.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut budget = MAX_CANDIDATES;
    for (path, is_dir, _depth) in collect_walk(area_root, MAX_RECURSION_DEPTH, &mut budget) {
        let Ok(rel) = path.strip_prefix(area_root) else {
            continue;
        };
        let Some(rel_str) = relative_to_value(rel) else {
            continue;
        };
        if rel_str.is_empty() {
            continue;
        }
        let value = render_value("!", &rel_str, is_dir);
        if value.starts_with(partial) {
            out.push(FileCompletionEntry {
                value,
                resolved_path: path,
                is_dir,
                source_rank: 1,
            });
        }
    }
    out
}

/// Walk `dir` non-recursively and invoke `visit(path, is_dir, name)` for each
/// child entry. Symlinks are dropped, the directory skip list is applied,
/// and unreadable directories are silently ignored.
fn list_immediate<F>(dir: &Path, visit: &mut F)
where
    F: FnMut(&Path, bool, &str),
{
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if file_type.is_dir() {
            if SKIP_DIR_NAMES.contains(&name) {
                continue;
            }
            if is_claudine_shadow(&path, name) {
                continue;
            }
            visit(&path, true, name);
        } else if file_type.is_file() {
            visit(&path, false, name);
        }
    }
}

/// Recursive walker bounded by `max_depth` and the shared `budget`. Children
/// of `root` are emitted at depth 1; the deepest entries returned have
/// depth `max_depth`. The walker never follows symlinks (which is what
/// keeps it safe from cycles) and stops as soon as `*budget` reaches 0.
fn collect_walk(
    root: &Path,
    max_depth: usize,
    budget: &mut usize,
) -> Vec<(PathBuf, bool, usize)> {
    let mut out = Vec::new();
    if root.is_dir() {
        recurse_walk(root, 1, max_depth, budget, &mut out);
    }
    out
}

fn recurse_walk(
    dir: &Path,
    child_depth: usize,
    max_depth: usize,
    budget: &mut usize,
    out: &mut Vec<(PathBuf, bool, usize)>,
) {
    if *budget == 0 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if file_type.is_dir() {
            if SKIP_DIR_NAMES.contains(&name) {
                continue;
            }
            if is_claudine_shadow(&path, name) {
                continue;
            }
            out.push((path.clone(), true, child_depth));
            *budget -= 1;
            if child_depth < max_depth {
                recurse_walk(&path, child_depth + 1, max_depth, budget, out);
            }
        } else if file_type.is_file() {
            out.push((path, false, child_depth));
            *budget -= 1;
        }
    }
}

/// True when `path` is a `.shadow` directory living directly under a
/// `.claudine` parent. The skip list cannot blanket every `.shadow` because
/// that name is not Claudine-specific; only the Claudine shadow tree should
/// be filtered.
fn is_claudine_shadow(path: &Path, name: &str) -> bool {
    if name != ".shadow" {
        return false;
    }
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        == Some(".claudine")
}

/// Turn `<prefix><name>` into the exact token the shell should insert,
/// appending `/` when the entry is a directory so subsequent `<TAB>`s
/// continue inside the directory.
fn render_value(prefix: &str, name: &str, is_dir: bool) -> String {
    let mut out = String::with_capacity(prefix.len() + name.len() + 1);
    out.push_str(prefix);
    out.push_str(name);
    if is_dir {
        out.push('/');
    }
    out
}

/// Convert a relative path to its `/`-joined string form, returning `None` if
/// any component is non-`Normal` (`..`, prefix, root) or non-UTF-8.
fn relative_to_value(rel: &Path) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(s) => parts.push(s.to_str()?),
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

/// Dedup by emitted `value` keeping the lowest `source_rank`, then return
/// entries sorted by `(source_rank, value)` so the shell sees a stable,
/// locality-preferring order.
fn dedup_and_sort(mut entries: Vec<FileCompletionEntry>) -> Vec<FileCompletionEntry> {
    entries.sort_by(|a, b| {
        a.source_rank
            .cmp(&b.source_rank)
            .then_with(|| a.value.cmp(&b.value))
    });
    let mut seen: HashSet<String> = HashSet::new();
    entries.retain(|entry| seen.insert(entry.value.clone()));
    if entries.len() > MAX_CANDIDATES {
        entries.truncate(MAX_CANDIDATES);
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    use tempfile::TempDir;

    // -- token classification ----------------------------------------------

    #[test]
    fn classify_recognises_setter_partials() {
        assert_eq!(classify_token("topic="), TokenClass::SetterPartial);
        assert_eq!(classify_token("topic=value"), TokenClass::SetterPartial);
        assert_eq!(classify_token("_internal="), TokenClass::SetterPartial);
        assert_eq!(classify_token("KEY_2="), TokenClass::SetterPartial);
    }

    #[test]
    fn classify_rejects_setter_lookalikes_with_invalid_chars() {
        // Hyphens are valid for the runtime parser but not for the spec's
        // strict completion regex.
        assert_eq!(classify_token("my-key="), TokenClass::Bare);
        // Dotted keys do not match either.
        assert_eq!(classify_token("foo.bar="), TokenClass::Bare);
        // Leading digit fails the identifier shape.
        assert_eq!(classify_token("1foo="), TokenClass::Bare);
        // Empty key with a leading `=` is not a setter.
        assert_eq!(classify_token("="), TokenClass::Bare);
    }

    #[test]
    fn classify_recognises_sigil_prefixes() {
        assert_eq!(classify_token("@"), TokenClass::Magic);
        assert_eq!(classify_token("@prompts/foo"), TokenClass::Magic);
        assert_eq!(classify_token("!"), TokenClass::Package);
        assert_eq!(classify_token("!src/lib.rs"), TokenClass::Package);
        assert_eq!(classify_token("./"), TokenClass::DotRelative);
        assert_eq!(classify_token("./foo"), TokenClass::DotRelative);
        assert_eq!(classify_token("../"), TokenClass::DotDotRelative);
        assert_eq!(classify_token("../foo"), TokenClass::DotDotRelative);
    }

    #[test]
    fn classify_recognises_unsupported_prefixes() {
        assert_eq!(classify_token("vault:foo"), TokenClass::Unsupported);
        assert_eq!(classify_token("/abs/path"), TokenClass::Unsupported);
        assert_eq!(classify_token("%recursive"), TokenClass::Unsupported);
        assert_eq!(classify_token("{{HOME}}/foo"), TokenClass::Unsupported);
    }

    #[test]
    fn classify_defaults_to_bare() {
        assert_eq!(classify_token(""), TokenClass::Bare);
        assert_eq!(classify_token("foo"), TokenClass::Bare);
        assert_eq!(classify_token("."), TokenClass::Bare);
        assert_eq!(classify_token(".."), TokenClass::Bare);
    }

    // -- discovery ---------------------------------------------------------

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn ctx_with_cwd(cwd: PathBuf) -> CompletionRepoContext {
        // Tests build their own filesystem fixtures and don't want
        // `detect_repo_structure` to walk a real repo, so we stub the
        // optional repo fields manually.
        CompletionRepoContext {
            cwd,
            repo_root: None,
            repo: None,
            current_package_area: None,
            current_package_area_root: None,
        }
    }

    fn ctx_with_repo(cwd: PathBuf, repo_root: PathBuf) -> CompletionRepoContext {
        CompletionRepoContext {
            cwd,
            repo_root: Some(repo_root),
            repo: None,
            current_package_area: None,
            current_package_area_root: None,
        }
    }

    fn ctx_with_area(
        cwd: PathBuf,
        repo_root: PathBuf,
        area_root: PathBuf,
    ) -> CompletionRepoContext {
        CompletionRepoContext {
            cwd,
            repo_root: Some(repo_root),
            repo: None,
            current_package_area: Some("test-area".into()),
            current_package_area_root: Some(area_root),
        }
    }

    #[test]
    fn setter_partial_returns_zero_candidates() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("a.md"), "");
        let ctx = ctx_with_cwd(tmp.path().to_path_buf());
        assert!(discover_candidates("topic=", &ctx).is_empty());
        assert!(discover_candidates("_internal=", &ctx).is_empty());
    }

    #[test]
    fn unsupported_prefix_returns_zero_candidates() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("a.md"), "");
        let ctx = ctx_with_cwd(tmp.path().to_path_buf());
        assert!(discover_candidates("vault:foo", &ctx).is_empty());
        assert!(discover_candidates("/abs/path", &ctx).is_empty());
        assert!(discover_candidates("%recursive", &ctx).is_empty());
        assert!(discover_candidates("{{HOME}}", &ctx).is_empty());
    }

    #[test]
    fn bare_landing_menu_lists_cwd_children_with_trailing_slash_for_dirs() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("a.md"), "");
        write_file(&tmp.path().join("b.md"), "");
        fs::create_dir(tmp.path().join("child-dir")).unwrap();
        let ctx = ctx_with_cwd(tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert!(values.contains(&"a.md".to_string()));
        assert!(values.contains(&"b.md".to_string()));
        assert!(values.contains(&"child-dir/".to_string()));
    }

    #[test]
    fn bare_landing_menu_filters_by_partial_prefix() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("apple.md"), "");
        write_file(&tmp.path().join("banana.md"), "");
        let ctx = ctx_with_cwd(tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("ap", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert_eq!(values, vec!["apple.md".to_string()]);
    }

    #[test]
    fn dot_relative_lists_cwd_children_with_dot_slash_prefix() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("apple.md"), "");
        write_file(&tmp.path().join("banana.md"), "");
        let ctx = ctx_with_cwd(tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("./", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert!(values.contains(&"./apple.md".to_string()));
        assert!(values.contains(&"./banana.md".to_string()));
    }

    #[test]
    fn dot_relative_with_subdir_path_lists_subdir_children() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("sub").join("inner.md"), "");
        let ctx = ctx_with_cwd(tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("./sub/", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert_eq!(values, vec!["./sub/inner.md".to_string()]);
    }

    #[test]
    fn dot_dot_relative_lists_parent_children() {
        let tmp = TempDir::new().unwrap();
        let inner = tmp.path().join("inner");
        fs::create_dir(&inner).unwrap();
        write_file(&tmp.path().join("sibling.md"), "");
        let ctx = ctx_with_cwd(inner);

        let values: Vec<String> = discover_candidates("../", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert!(values.contains(&"../sibling.md".to_string()));
    }

    #[test]
    fn magic_emits_repo_relative_value_with_at_prefix() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("prompts").join("review.md"), "");
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("@", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert!(values.contains(&"@prompts/review.md".to_string()));
    }

    #[test]
    fn magic_filters_by_partial_inside_value() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("prompts").join("review.md"), "");
        write_file(&tmp.path().join("docs").join("guide.md"), "");
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("@prompts/", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        // Only @prompts/* entries pass the prefix filter.
        assert!(values.iter().all(|v| v.starts_with("@prompts/")));
        assert!(values.contains(&"@prompts/review.md".to_string()));
    }

    #[test]
    fn package_emits_area_relative_value_with_bang_prefix() {
        let tmp = TempDir::new().unwrap();
        let area = tmp.path().join("test-area");
        write_file(&area.join("src").join("lib.rs"), "");
        let ctx = ctx_with_area(area.clone(), tmp.path().to_path_buf(), area);

        let values: Vec<String> = discover_candidates("!", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert!(values.contains(&"!src/lib.rs".to_string()));
    }

    #[test]
    fn package_returns_empty_when_no_area_resolved() {
        let tmp = TempDir::new().unwrap();
        write_file(&tmp.path().join("a.md"), "");
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());
        // No `current_package_area_root` is set, so `!` cannot resolve.
        assert!(discover_candidates("!", &ctx).is_empty());
    }

    #[test]
    fn skip_list_is_honoured_during_walk() {
        let tmp = TempDir::new().unwrap();
        // Should be skipped entirely.
        write_file(&tmp.path().join(".git").join("config"), "");
        write_file(&tmp.path().join("target").join("doc.md"), "");
        write_file(&tmp.path().join("node_modules").join("pkg.md"), "");
        // Should be visible.
        write_file(&tmp.path().join("docs").join("real.md"), "");
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("@", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert!(values.contains(&"@docs/real.md".to_string()));
        assert!(!values.iter().any(|v| v.starts_with("@.git/")));
        assert!(!values.iter().any(|v| v.starts_with("@target/")));
        assert!(!values.iter().any(|v| v.starts_with("@node_modules/")));
    }

    #[test]
    fn claudine_shadow_is_skipped_only_under_claudine_dir() {
        let tmp = TempDir::new().unwrap();
        // .claudine/.shadow → skipped
        write_file(
            &tmp.path().join(".claudine").join(".shadow").join("a.md"),
            "",
        );
        // top-level .shadow → kept
        write_file(&tmp.path().join(".shadow").join("b.md"), "");
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("@", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert!(!values.iter().any(|v| v.starts_with("@.claudine/.shadow/")));
        assert!(values.contains(&"@.shadow/b.md".to_string()));
    }

    #[test]
    fn depth_cap_stops_recursion() {
        let tmp = TempDir::new().unwrap();
        // Children of the walk root sit at depth 1, so with
        // MAX_RECURSION_DEPTH=4 the deepest visible entry is at depth 4
        // (`d1/d2/d3/ok.md`). A file one level deeper must not appear.
        write_file(
            &tmp.path().join("d1").join("d2").join("d3").join("ok.md"),
            "",
        );
        write_file(
            &tmp.path()
                .join("d1")
                .join("d2")
                .join("d3")
                .join("d4")
                .join("too-deep.md"),
            "",
        );
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("@", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        assert!(values.contains(&"@d1/d2/d3/ok.md".to_string()));
        assert!(!values.iter().any(|v| v.contains("too-deep.md")));
    }

    #[test]
    fn candidate_cap_truncates_results() {
        let tmp = TempDir::new().unwrap();
        // Build > MAX_CANDIDATES files in a single dir; the budget cuts the
        // walker off before the full list is collected.
        let dir = tmp.path().join("big");
        fs::create_dir(&dir).unwrap();
        for i in 0..(MAX_CANDIDATES + 50) {
            write_file(&dir.join(format!("file-{i:04}.md")), "");
        }
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());

        let entries = discover_candidates("@", &ctx);
        assert!(entries.len() <= MAX_CANDIDATES);
    }

    #[test]
    fn symlinked_directory_is_not_recursed_into() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real");
        fs::create_dir(&real).unwrap();
        write_file(&real.join("real-file.md"), "");
        let link = tmp.path().join("link-to-real");
        symlink(&real, &link).unwrap();
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());

        let values: Vec<String> = discover_candidates("@", &ctx)
            .into_iter()
            .map(|e| e.value)
            .collect();
        // Real directory is walked normally.
        assert!(values.contains(&"@real/real-file.md".to_string()));
        // Symlink is not followed, so its contents do not appear.
        assert!(!values.iter().any(|v| v.starts_with("@link-to-real/")));
        // The symlink entry itself is dropped (file_type().is_symlink()).
        assert!(!values.iter().any(|v| v == "@link-to-real/" || v == "@link-to-real"));
    }

    #[test]
    fn symlink_cycle_does_not_hang() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        symlink(&b, a.join("to-b")).unwrap();
        symlink(&a, b.join("to-a")).unwrap();
        write_file(&a.join("doc.md"), "");
        let ctx = ctx_with_repo(tmp.path().to_path_buf(), tmp.path().to_path_buf());

        // The walker drops every symlink, so this returns quickly with no
        // recursion through the cycle.
        let entries = discover_candidates("@", &ctx);
        assert!(entries.iter().any(|e| e.value == "@a/doc.md"));
    }

    #[test]
    fn dedup_keeps_lowest_source_rank() {
        let entries = vec![
            FileCompletionEntry {
                value: "@prompts/dup.md".into(),
                resolved_path: PathBuf::from("/home/user/.claudine/prompts/dup.md"),
                is_dir: false,
                source_rank: 3,
            },
            FileCompletionEntry {
                value: "@prompts/dup.md".into(),
                resolved_path: PathBuf::from("/repo/prompts/dup.md"),
                is_dir: false,
                source_rank: 2,
            },
        ];
        let deduped = dedup_and_sort(entries);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].source_rank, 2);
        assert_eq!(deduped[0].resolved_path, PathBuf::from("/repo/prompts/dup.md"));
    }

    #[test]
    fn dedup_orders_by_rank_then_value() {
        let entries = vec![
            FileCompletionEntry {
                value: "z.md".into(),
                resolved_path: PathBuf::from("z"),
                is_dir: false,
                source_rank: 0,
            },
            FileCompletionEntry {
                value: "@a.md".into(),
                resolved_path: PathBuf::from("a"),
                is_dir: false,
                source_rank: 2,
            },
            FileCompletionEntry {
                value: "!area.md".into(),
                resolved_path: PathBuf::from("area"),
                is_dir: false,
                source_rank: 1,
            },
        ];
        let deduped = dedup_and_sort(entries);
        let values: Vec<&str> = deduped.iter().map(|e| e.value.as_str()).collect();
        assert_eq!(values, vec!["z.md", "!area.md", "@a.md"]);
    }
}
