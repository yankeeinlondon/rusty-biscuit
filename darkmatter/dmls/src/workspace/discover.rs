//! Workspace file discovery: the in-memory file list.
//!
//! One walk per effective workspace root (`workspace.root` applied) lists
//! paths and reads no file contents:
//!
//! - **Markdown**: files matching `workspace.include` and not
//!   `workspace.exclude`;
//! - **YAML**: `*.yaml`/`*.yml` files not matching `workspace.exclude`
//!   (`include` does not apply: its defaults are Markdown-only).
//!
//! The walk honors `.gitignore`, `.ignore`, `.git/info/exclude`, and the
//! global excludes file, also outside a git repository
//! (`require_git(false)`). Hidden directories are walked, because folders
//! such as `.claude/` hold real Markdown, but every directory named `.git` is
//! skipped (`filter_entry`). **Symlinks are not followed**: a symlinked
//! directory is never descended and a symlinked file is never listed. Two
//! listed paths that canonicalize to the same file (overlapping or aliased
//! workspace roots) are reported as duplicates (the caller logs each) rather
//! than listed twice; hard links canonicalize apart and are not detected.
//!
//! A walk error does not stop the walk; it marks the list incomplete
//! ([`DiscoveryReport::is_complete`]), so no caller can mistake absence from
//! the list for absence from disk.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::config::WorkspaceConfig;

/// What kind of file-list entry a path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListedKind {
    /// A Markdown document (a graph node).
    Markdown,
    /// A YAML file (listed, never a graph node).
    Yaml,
}

/// Compiled include/exclude glob matchers derived from a [`WorkspaceConfig`].
#[derive(Debug, Clone)]
pub struct DiscoveryFilter {
    include: GlobSet,
    exclude: GlobSet,
}

impl DiscoveryFilter {
    /// Compiles the filter from workspace config. Invalid globs are skipped
    /// (logged), so a typo in one pattern cannot disable discovery entirely.
    pub fn from_config(config: &WorkspaceConfig) -> Self {
        Self {
            include: build_glob_set(&config.include),
            exclude: build_glob_set(&config.exclude),
        }
    }

    /// How a root-relative path is listed, or `None` when it is not.
    pub fn classify(&self, relative: &Path) -> Option<ListedKind> {
        if self.exclude.is_match(relative) {
            return None;
        }
        if self.include.is_match(relative) {
            return Some(ListedKind::Markdown);
        }
        is_yaml(relative).then_some(ListedKind::Yaml)
    }
}

/// Whether `path` has a `.yaml` or `.yml` extension.
fn is_yaml(path: &Path) -> bool {
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml" | "yml"))
}

/// The outcome of a discovery pass: the workspace file list.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DiscoveryReport {
    /// Listed Markdown files (deduplicated by physical identity), in walk
    /// order.
    pub files: Vec<PathBuf>,
    /// Listed YAML files, in walk order.
    pub yaml: Vec<PathBuf>,
    /// Files skipped because another listed path canonicalizes to the same
    /// file (overlapping or aliased workspace roots; hard links are not
    /// detected).
    pub duplicates: Vec<PathBuf>,
    /// Walk errors (an unreadable directory, a vanished root). Any error
    /// makes the list incomplete.
    pub errors: Vec<String>,
}

impl DiscoveryReport {
    /// Whether every directory under every root was listed.
    pub fn is_complete(&self) -> bool {
        self.errors.is_empty()
    }
}

/// The effective roots discovery walks: each workspace folder with
/// `workspace.root` applied.
pub fn effective_roots(roots: &[PathBuf], config: &WorkspaceConfig) -> Vec<PathBuf> {
    roots
        .iter()
        .map(|root| match &config.root {
            Some(sub) => root.join(sub),
            None => root.clone(),
        })
        .collect()
}

/// The walker every discovery decision uses, so the initial walk and a
/// single-path check agree on ignore rules.
fn walker(root: &Path, max_depth: Option<usize>) -> ignore::Walk {
    // `follow_links(false)` is the whole symlink policy: symlinked dirs are
    // never descended and symlinked files are yielded as symlinks (skipped by
    // the caller), so loop detection is unnecessary.
    WalkBuilder::new(root)
        .follow_links(false)
        .hidden(false)
        .require_git(false)
        .max_depth(max_depth)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build()
}

/// Lists the Markdown and YAML files across `roots` under `config`.
pub fn discover_workspace(roots: &[PathBuf], config: &WorkspaceConfig) -> DiscoveryReport {
    let filter = DiscoveryFilter::from_config(config);
    let mut report = DiscoveryReport::default();
    // Physical identity of files already accepted, so an alias is a duplicate.
    let mut seen_physical: HashSet<PathBuf> = HashSet::new();

    for effective_root in effective_roots(roots, config) {
        for entry in walker(&effective_root, None) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    report.errors.push(error.to_string());
                    continue;
                }
            };
            let path = entry.path();
            if entry.path_is_symlink() || entry.file_type().is_none_or(|kind| !kind.is_file()) {
                continue;
            }
            let relative = path.strip_prefix(&effective_root).unwrap_or(path);
            let Some(kind) = filter.classify(relative) else {
                continue;
            };
            let physical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            if !seen_physical.insert(physical) {
                report.duplicates.push(path.to_path_buf());
                continue;
            }
            match kind {
                ListedKind::Markdown => report.files.push(path.to_path_buf()),
                ListedKind::Yaml => report.yaml.push(path.to_path_buf()),
            }
        }
    }

    report
}

/// How `path` would be listed by [`discover_workspace`], judged without a
/// full walk: the path must sit under an effective root, match the filter,
/// lie outside every `.git` directory, and survive the same ignore rules
/// (one directory listing of its parent).
///
/// ## Returns
///
/// `None` when the walk would not list `path`, including when it does not
/// exist as a regular file.
pub fn listed_kind(roots: &[PathBuf], config: &WorkspaceConfig, path: &Path) -> Option<ListedKind> {
    let filter = DiscoveryFilter::from_config(config);
    let effective_root = effective_roots(roots, config)
        .into_iter()
        .find(|root| path.starts_with(root))?;
    let relative = path.strip_prefix(&effective_root).ok()?;
    if relative.components().any(|component| component.as_os_str() == ".git") {
        return None;
    }
    let kind = filter.classify(relative)?;
    let parent = path.parent()?;
    // Ignore rules for an ancestor directory hide the whole parent; walking
    // from the root to it would re-list the tree, so check the directories
    // between the root and the parent one level at a time.
    let mut directory = effective_root.clone();
    let remaining = parent.strip_prefix(&effective_root).ok()?.components();
    for next in remaining {
        let child = directory.join(next);
        if !walker(&directory, Some(1)).flatten().any(|entry| entry.path() == child) {
            return None;
        }
        directory = child;
    }
    walker(parent, Some(1))
        .flatten()
        .any(|entry| entry.path() == path && entry.file_type().is_some_and(|kind| kind.is_file()))
        .then_some(kind)
}

/// Builds a `GlobSet` from patterns, skipping (and logging) invalid ones.
fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(error) => tracing::warn!("ignoring invalid glob {pattern:?}: {error}"),
        }
    }
    builder.build().unwrap_or_else(|error| {
        tracing::warn!("failed to compile glob set: {error}");
        GlobSet::empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn test_discovers_markdown_and_respects_excludes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "a.md", "# A");
        write(root, "docs/b.markdown", "# B");
        write(root, "notes.txt", "plain");
        write(root, "target/gen.md", "# generated");

        let config = WorkspaceConfig {
            exclude: vec!["target/**".to_string()],
            ..WorkspaceConfig::default()
        };
        let report = discover_workspace(&[root.to_path_buf()], &config);
        let names: Vec<String> = report
            .files
            .iter()
            .map(|p| biscuit_file::to_portable_string(p.strip_prefix(root).unwrap()))
            .collect();
        assert!(names.contains(&"a.md".to_string()));
        assert!(names.contains(&"docs/b.markdown".to_string()));
        assert!(!names.iter().any(|n| n.ends_with("notes.txt")));
        assert!(!names.iter().any(|n| n.contains("target/")));
    }

    #[test]
    fn test_gitignore_is_honored() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, ".gitignore", "ignored.md\n");
        write(root, "kept.md", "# kept");
        write(root, "ignored.md", "# ignored");
        std::fs::create_dir_all(root.join(".git")).unwrap();

        let report = discover_workspace(&[root.to_path_buf()], &WorkspaceConfig::default());
        let names: Vec<String> = report
            .files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"kept.md".to_string()));
        assert!(!names.contains(&"ignored.md".to_string()));
    }

    #[test]
    fn test_config_root_subdirectory() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "outside.md", "# out");
        write(root, "wiki/inside.md", "# in");
        let config = WorkspaceConfig {
            root: Some(PathBuf::from("wiki")),
            ..WorkspaceConfig::default()
        };
        let report = discover_workspace(&[root.to_path_buf()], &config);
        assert_eq!(report.files.len(), 1);
        assert!(report.files[0].ends_with("inside.md"));
    }

    fn relative_names(root: &Path, paths: &[PathBuf]) -> Vec<String> {
        let mut names: Vec<String> = paths
            .iter()
            .map(|path| biscuit_file::to_portable_string(path.strip_prefix(root).unwrap()))
            .collect();
        names.sort();
        names
    }

    /// Markdown, YAML, a gitignored `node_modules/`, a hidden folder, a
    /// symlink, and a `.git/` directory holding `.md` files.
    fn mixed_workspace(root: &Path) {
        write(root, ".gitignore", "node_modules/\n");
        write(root, "README.md", "# R");
        write(root, ".claude/skill.md", "# hidden but real");
        write(root, "docs/guide.markdown", "# G");
        write(root, "schemas/kind.yaml", "kind: schema");
        write(root, "config.yml", "a: 1");
        write(root, "notes.txt", "plain");
        write(root, "node_modules/pkg/README.md", "# vendored");
        write(root, "node_modules/pkg/schema.yaml", "a: 1");
        write(root, ".git/HEAD", "ref: refs/heads/main");
        write(root, ".git/notes/stray.md", "# never listed");
        write(root, ".git/hooks/config.yaml", "a: 1");
    }

    #[test]
    fn test_lists_markdown_and_yaml_skipping_ignored_git_and_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        mixed_workspace(root);
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("README.md"), root.join("alias.md")).unwrap();
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_file(root.join("README.md"), root.join("alias.md"));

        let before = crate::work_counters::work_counts();
        let report = discover_workspace(&[root.to_path_buf()], &WorkspaceConfig::default());
        assert_eq!(crate::work_counters::work_counts().since(before).reads, 0);

        assert_eq!(
            relative_names(root, &report.files),
            vec![".claude/skill.md", "README.md", "docs/guide.markdown"]
        );
        assert_eq!(relative_names(root, &report.yaml), vec!["config.yml", "schemas/kind.yaml"]);
        assert!(report.duplicates.is_empty());
        assert!(report.is_complete(), "{:?}", report.errors);
    }

    #[test]
    fn test_exclude_applies_to_yaml_and_include_does_not() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        mixed_workspace(root);
        let config = WorkspaceConfig {
            include: Vec::new(),
            exclude: vec!["schemas/**".to_string()],
            ..WorkspaceConfig::default()
        };
        let report = discover_workspace(&[root.to_path_buf()], &config);
        assert!(report.files.is_empty());
        assert_eq!(relative_names(root, &report.yaml), vec!["config.yml"]);
    }

    #[test]
    fn test_overlapping_roots_report_each_duplicate() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "a.md", "# A");
        write(root, "sub/b.md", "# B");
        let report = discover_workspace(
            &[root.to_path_buf(), root.join("sub")],
            &WorkspaceConfig::default(),
        );
        assert_eq!(relative_names(root, &report.files), vec!["a.md", "sub/b.md"]);
        assert_eq!(relative_names(root, &report.duplicates), vec!["sub/b.md"]);
    }

    #[test]
    fn test_gitignore_applies_outside_a_repository() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, ".gitignore", "node_modules/\n");
        write(root, "kept.md", "# kept");
        write(root, "node_modules/pkg/README.md", "# vendored");
        assert!(!root.join(".git").exists());
        let report = discover_workspace(&[root.to_path_buf()], &WorkspaceConfig::default());
        assert_eq!(relative_names(root, &report.files), vec!["kept.md"]);
    }

    #[test]
    fn test_gitignore_applies_in_a_linked_worktree() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        // A linked worktree's `.git` is a file naming the common git dir.
        write(root, ".git", "gitdir: /elsewhere/.git/worktrees/feature\n");
        write(root, ".gitignore", "node_modules/\nbuild.md\n");
        write(root, "kept.md", "# kept");
        write(root, "build.md", "# ignored");
        write(root, "node_modules/pkg/README.md", "# vendored");
        let report = discover_workspace(&[root.to_path_buf()], &WorkspaceConfig::default());
        assert_eq!(relative_names(root, &report.files), vec!["kept.md"]);
    }

    #[test]
    fn test_an_unreadable_root_marks_the_list_incomplete() {
        let temp = tempfile::tempdir().unwrap();
        let report = discover_workspace(
            &[temp.path().join("vanished")],
            &WorkspaceConfig::default(),
        );
        assert!(report.files.is_empty());
        assert!(!report.is_complete());
    }

    #[test]
    fn test_listed_kind_agrees_with_the_walk() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        mixed_workspace(root);
        let roots = [root.to_path_buf()];
        let config = WorkspaceConfig::default();
        let kind = |rel: &str| listed_kind(&roots, &config, &root.join(rel));
        assert_eq!(kind("README.md"), Some(ListedKind::Markdown));
        assert_eq!(kind(".claude/skill.md"), Some(ListedKind::Markdown));
        assert_eq!(kind("schemas/kind.yaml"), Some(ListedKind::Yaml));
        assert_eq!(kind("node_modules/pkg/README.md"), None);
        assert_eq!(kind("node_modules/pkg/schema.yaml"), None);
        assert_eq!(kind(".git/notes/stray.md"), None);
        assert_eq!(kind("notes.txt"), None);
        assert_eq!(kind("absent.md"), None);
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), "far.md", "# far");
        assert_eq!(listed_kind(&roots, &config, &outside.path().join("far.md")), None);
    }

    #[test]
    fn test_symlinked_file_is_not_followed() {
        #[cfg(unix)]
        {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            write(root, "real.md", "# real");
            std::os::unix::fs::symlink(root.join("real.md"), root.join("alias.md")).unwrap();
            let report = discover_workspace(&[root.to_path_buf()], &WorkspaceConfig::default());
            // The symlink is skipped; only the real file is indexed once.
            assert_eq!(report.files.len(), 1);
            assert!(report.files[0].ends_with("real.md"));
        }
    }
}
