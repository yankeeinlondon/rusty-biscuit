//! Workspace file discovery.
//!
//! Walks each workspace root with `ignore` (so `.gitignore`/`.ignore` rules
//! are honored for free), keeps files matching the configured include globs
//! and not the exclude globs, and applies the R-8 symlink policy for v1:
//! **symlinks are not followed**, and two discovered paths that canonicalize
//! to the same physical file are reported as duplicates (the caller emits a
//! diagnostic) rather than indexed twice.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::config::WorkspaceConfig;

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

    /// Whether a root-relative path is a discovery match.
    fn matches(&self, relative: &Path) -> bool {
        self.include.is_match(relative) && !self.exclude.is_match(relative)
    }
}

/// The outcome of a discovery pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DiscoveryReport {
    /// Discovered files (deduplicated by physical identity), in walk order.
    pub files: Vec<PathBuf>,
    /// Files skipped because another discovered path is the same physical
    /// file (symlink/hardlink/overlapping-root aliasing) — R-8 gotcha 8.
    pub duplicates: Vec<PathBuf>,
}

/// Discovers indexable files across `roots` under `config`.
pub fn discover_workspace(roots: &[PathBuf], config: &WorkspaceConfig) -> DiscoveryReport {
    let filter = DiscoveryFilter::from_config(config);
    let mut report = DiscoveryReport::default();
    // Physical identity of files already accepted, so an alias is a duplicate.
    let mut seen_physical: HashSet<PathBuf> = HashSet::new();

    for root in roots {
        let effective_root = match &config.root {
            Some(sub) => root.join(sub),
            None => root.clone(),
        };
        // `follow_links(false)` is the whole symlink policy: symlinked dirs are
        // never descended and symlinked files are yielded as symlinks (skipped
        // below), so loop detection is unnecessary for v1.
        let walker = WalkBuilder::new(&effective_root)
            .follow_links(false)
            .hidden(false)
            .build();
        for entry in walker.flatten() {
            let path = entry.path();
            if entry.file_type().is_none_or(|kind| !kind.is_file()) {
                continue;
            }
            if path.is_symlink() {
                continue;
            }
            let relative = path.strip_prefix(&effective_root).unwrap_or(path);
            if !filter.matches(relative) {
                continue;
            }
            let physical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            if seen_physical.insert(physical) {
                report.files.push(path.to_path_buf());
            } else {
                report.duplicates.push(path.to_path_buf());
            }
        }
    }

    report
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
        // A `.git` dir makes `ignore` treat `.gitignore` as authoritative.
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
