//! Builds the [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
//! tree directly from discovered `::file-links` entries.
//!
//! Discovery already walked the filesystem to find the matched files. Rebuilding
//! the tree from that result — rather than letting `FileSystem` walk the
//! directory a second time — keeps the operation to a single traversal per
//! directive and guarantees the rendered tree agrees with the discovered
//! allowlist even if the filesystem changes between phases.

use std::collections::BTreeMap;
use std::path::Path;

use biscuit_terminal::components::filesystem::{GitignoreMatcher, TreeNode};

use super::types::FileLinksRender;

/// Builds the projected [`TreeNode`] forest for a discovered file set.
///
/// The hierarchy is reconstructed from [`FileLinksRender::included_paths`] (each
/// relative to the component root). `is_ignored` is resolved through the shared
/// [`GitignoreMatcher`]; `is_symlink` through a per-entry symlink probe. The
/// sort order — directories first, then case-insensitive by name — mirrors
/// `FileSystem`'s own walk so the rendered tree is identical to the walked one.
pub(crate) fn build_included_tree(render: &FileLinksRender) -> Vec<TreeNode> {
    let matcher = GitignoreMatcher::for_root(&render.component_root);
    let mut root = DirBuilder::default();
    for rel in &render.included_paths {
        root.insert(rel);
    }
    root.into_nodes(&render.component_root, Path::new(""), &matcher)
}

/// A directory under construction: child directories keyed by name, plus the
/// file names directly inside it.
#[derive(Default)]
struct DirBuilder {
    dirs: BTreeMap<String, DirBuilder>,
    files: Vec<String>,
}

impl DirBuilder {
    /// Inserts a file path (relative to the component root) into the tree,
    /// creating intermediate directories as needed.
    fn insert(&mut self, rel: &Path) {
        let names: Vec<String> = rel
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        self.insert_names(&names);
    }

    fn insert_names(&mut self, names: &[String]) {
        match names {
            [] => {}
            [file] => {
                if !self.files.iter().any(|existing| existing == file) {
                    self.files.push(file.clone());
                }
            }
            [dir, rest @ ..] => {
                self.dirs.entry(dir.clone()).or_default().insert_names(rest);
            }
        }
    }

    /// Converts the builder into sorted [`TreeNode`]s. `rel_so_far` is the path
    /// of this directory relative to `component_root`, used to resolve the
    /// absolute path each gitignore/symlink probe needs.
    fn into_nodes(
        self,
        component_root: &Path,
        rel_so_far: &Path,
        matcher: &GitignoreMatcher,
    ) -> Vec<TreeNode> {
        let mut nodes = Vec::with_capacity(self.dirs.len() + self.files.len());

        // Directories first, sorted case-insensitively (the `BTreeMap` order is
        // case-sensitive, so re-sort to match `FileSystem`'s walk).
        let mut dirs: Vec<(String, DirBuilder)> = self.dirs.into_iter().collect();
        dirs.sort_by_key(|(name, _)| name.to_lowercase());
        for (name, builder) in dirs {
            let rel = rel_so_far.join(&name);
            let abs = component_root.join(&rel);
            let children = builder.into_nodes(component_root, &rel, matcher);
            nodes.push(TreeNode::Dir {
                name,
                children,
                is_ignored: matcher.is_ignored(&abs, true),
                is_symlink: is_symlink(&abs),
                has_error: false,
                at_depth_limit: false,
                metrics: None,
            });
        }

        let mut files = self.files;
        files.sort_by_key(|name| name.to_lowercase());
        for name in files {
            let abs = component_root.join(rel_so_far.join(&name));
            nodes.push(TreeNode::File {
                is_ignored: matcher.is_ignored(&abs, false),
                is_symlink: is_symlink(&abs),
                metrics: None,
                name,
            });
        }

        nodes
    }
}

/// Whether `abs` is a symbolic link (without following it). Missing or
/// unreadable entries are reported as non-symlinks.
fn is_symlink(abs: &Path) -> bool {
    std::fs::symlink_metadata(abs)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn render_for(dir: &Path, included: &[&str]) -> FileLinksRender {
        FileLinksRender {
            component_root: dir.to_path_buf(),
            included_paths: included.iter().map(PathBuf::from).collect(),
            dimmed_prefix: String::new(),
            target_name: "root".to_string(),
            uses_repo_icon: false,
        }
    }

    #[test]
    fn builds_hierarchy_dirs_before_files_sorted() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("z.md"), "").unwrap();
        std::fs::write(dir.path().join("b.md"), "").unwrap();
        std::fs::write(dir.path().join("a.md"), "").unwrap();

        let tree = build_included_tree(&render_for(dir.path(), &["sub/z.md", "b.md", "a.md"]));

        // Directories sort before files; names case-insensitively ascending.
        assert_eq!(tree.len(), 3);
        assert!(matches!(&tree[0], TreeNode::Dir { name, .. } if name == "sub"));
        assert!(matches!(&tree[1], TreeNode::File { name, .. } if name == "a.md"));
        assert!(matches!(&tree[2], TreeNode::File { name, .. } if name == "b.md"));
        if let TreeNode::Dir { children, .. } = &tree[0] {
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].name(), "z.md");
        } else {
            panic!("expected dir node");
        }
    }

    #[test]
    fn marks_gitignored_entries_without_walking() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "ignored.md\n").unwrap();
        std::fs::write(dir.path().join("kept.md"), "").unwrap();
        std::fs::write(dir.path().join("ignored.md"), "").unwrap();
        // A sibling file the discovery did NOT include must not appear, proving
        // the tree comes from the allowlist rather than a directory scan.
        std::fs::write(dir.path().join("stray.md"), "").unwrap();

        let tree = build_included_tree(&render_for(dir.path(), &["kept.md", "ignored.md"]));

        let names: Vec<&str> = tree.iter().map(TreeNode::name).collect();
        assert_eq!(names, vec!["ignored.md", "kept.md"]);
        assert!(!names.contains(&"stray.md"), "non-included file leaked in");
        for node in &tree {
            match node.name() {
                "ignored.md" => assert!(node.is_ignored(), "gitignored file must be flagged"),
                "kept.md" => assert!(!node.is_ignored(), "tracked file must not be flagged"),
                other => panic!("unexpected node {other}"),
            }
        }
    }
}
