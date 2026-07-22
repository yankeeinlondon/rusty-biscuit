//! Polyglot build-system detection (Bazel, Pants, Buck2).
//!
//! Unlike the workspace authorities that parse a root manifest's member list,
//! these standards declare membership through the `LeafMarkers` model: a package
//! is any directory containing a per-directory build file. The walk **is** the
//! membership model — there is no root list to expand.
//!
//! Each detector is gated on its project-root marker (Bazel `WORKSPACE` /
//! `MODULE.bazel`, Pants `pants.toml`, Buck2 `.buckconfig`) so a stray `BUILD`
//! or `BUCK` file in an unrelated tree never triggers a false positive. Bazel
//! additionally segments the tree at nested `WORKSPACE` markers
//! (`NestingPolicy::IgnoresNested`): a nested workspace and its subtree become
//! a separate layer, excluded from the parent's package list.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::Result;
use crate::filesystem::file_types::should_skip_directory_name;
use crate::performance;
use crate::performance::counters;

use super::detection::probe_exists;
use super::seed::PackageSeed;
use super::standard::{MonorepoStandard, PackageProvenance};
use super::topology::DetectorOutcome;


/// Bazel project-root markers. Any one of these promotes a directory to a
/// (parent or nested) workspace root.
const BAZEL_ROOT_MARKERS: [&str; 3] = ["WORKSPACE", "WORKSPACE.bazel", "MODULE.bazel"];
/// Bazel per-directory build files that mark a package boundary.
const BAZEL_LEAF_FILES: [&str; 2] = ["BUILD", "BUILD.bazel"];
/// Pants per-directory build files (`BUILD.pants` is canonical; plain `BUILD`
/// is also accepted since Pants projects commonly use it).
const PANTS_LEAF_FILES: [&str; 2] = ["BUILD.pants", "BUILD"];
/// Buck2 per-directory build files that mark a package boundary.
const BUCK2_LEAF_FILES: [&str; 2] = ["BUCK", "TARGETS"];

/// One leaf-marker workspace: a root and the package directories it owns.
struct LeafWorkspace {
    root: PathBuf,
    seeds: Vec<PackageSeed>,
}

/// Detect a Bazel workspace forest rooted at `root`.
///
/// Returns one outcome per workspace root. The parent workspace excludes any
/// subtree owned by a nested `WORKSPACE` / `MODULE.bazel`, which is reported as
/// its own outcome.
pub(super) fn detect_bazel_workspace(root: &Path) -> Result<Vec<DetectorOutcome>> {
    if !has_any_marker(root, &BAZEL_ROOT_MARKERS) {
        return Ok(Vec::new());
    }
    let workspaces = walk_leaf_workspaces(
        root,
        &BAZEL_LEAF_FILES,
        &BAZEL_ROOT_MARKERS,
        MonorepoStandard::Bazel,
    );
    Ok(into_outcomes(workspaces, MonorepoStandard::Bazel))
}

/// Detect a Pants workspace rooted at `pants.toml`.
pub(super) fn detect_pants_workspace(root: &Path) -> Result<Vec<DetectorOutcome>> {
    if !probe_exists(&root.join("pants.toml")) {
        return Ok(Vec::new());
    }
    let workspaces = walk_leaf_workspaces(root, &PANTS_LEAF_FILES, &[], MonorepoStandard::Pants);
    Ok(into_outcomes(workspaces, MonorepoStandard::Pants))
}

/// Detect a Buck2 workspace rooted at `.buckconfig`.
pub(super) fn detect_buck2_workspace(root: &Path) -> Result<Vec<DetectorOutcome>> {
    if !probe_exists(&root.join(".buckconfig")) {
        return Ok(Vec::new());
    }
    let workspaces = walk_leaf_workspaces(root, &BUCK2_LEAF_FILES, &[], MonorepoStandard::Buck2);
    Ok(into_outcomes(workspaces, MonorepoStandard::Buck2))
}

/// Whether `dir` contains any of `markers` directly.
fn has_any_marker(dir: &Path, markers: &[&str]) -> bool {
    markers.iter().any(|name| probe_exists(&dir.join(name)))
}

/// Convert resolved workspaces into detector outcomes, dropping empty ones.
fn into_outcomes(
    workspaces: Vec<LeafWorkspace>,
    standard: MonorepoStandard,
) -> Vec<DetectorOutcome> {
    workspaces
        .into_iter()
        .filter(|ws| !ws.seeds.is_empty())
        .map(|ws| DetectorOutcome {
            standard,
            root: ws.root,
            seeds: ws.seeds,
        })
        .collect()
}

/// Walk `root` once and group package directories (those containing a leaf file)
/// by the workspace that owns them.
///
/// A directory's owning workspace is the deepest `nested_root` marker on the
/// path from `root` to the directory (inclusive); `root` itself is always a
/// workspace. When `nested_roots` is empty there is no segmentation and every
/// package belongs to `root`. Package paths are built relative to the supplied
/// `root` so both [`DetectorOutcome`] packages and the flat `RepoInfo.packages`
/// list use repo-root-relative framing.
fn walk_leaf_workspaces(
    root: &Path,
    leaf_files: &[&str],
    nested_roots: &[&str],
    standard: MonorepoStandard,
) -> Vec<LeafWorkspace> {
    let dirs = walk_dirs(root);

    // Workspace roots, longest-path-first so the first prefix match is the
    // deepest (most specific) owner.
    let mut workspace_roots: Vec<PathBuf> = vec![root.to_path_buf()];
    if !nested_roots.is_empty() {
        for dir in &dirs {
            if dir != root && has_any_marker(dir, nested_roots) {
                workspace_roots.push(dir.clone());
            }
        }
    }
    workspace_roots.sort_by_key(|path| std::cmp::Reverse(path.components().count()));

    let mut by_root: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for dir in &dirs {
        if !has_any_marker(dir, leaf_files) {
            continue;
        }
        let owner = workspace_roots
            .iter()
            .find(|ws_root| dir.starts_with(ws_root))
            .cloned()
            .unwrap_or_else(|| root.to_path_buf());
        by_root.entry(owner).or_default().push(dir.clone());
    }

    by_root
        .into_iter()
        .map(|(ws_root, leaf_dirs)| {
            let seeds = leaf_dirs
                .into_iter()
                .map(|dir| {
                    PackageSeed::new(&dir, root, standard, PackageProvenance::LeafMarkers)
                })
                .collect();
            LeafWorkspace {
                root: ws_root,
                seeds,
            }
        })
        .collect()
}

/// Collect every non-skipped directory under `root` (including `root` itself),
/// honoring `.gitignore` and skipping `node_modules` / `target` / `dist` /
/// `build` exactly as the manifest-index walk does.
fn walk_dirs(root: &Path) -> Vec<PathBuf> {
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| {
            if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                return true;
            }
            !entry
                .file_name()
                .to_str()
                .is_some_and(should_skip_directory_name)
        })
        .build();

    walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_dir()))
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, "").unwrap();
    }

    #[test]
    fn bazel_segments_nested_workspace_into_its_own_layer() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("WORKSPACE"));
        touch(&root.join("a/BUILD"));
        touch(&root.join("b/BUILD.bazel"));
        touch(&root.join("nested/WORKSPACE"));
        touch(&root.join("nested/BUILD"));
        touch(&root.join("nested/sub/BUILD"));

        let outcomes = detect_bazel_workspace(root).unwrap();
        assert_eq!(outcomes.len(), 2, "parent + nested workspace");

        let parent = outcomes
            .iter()
            .find(|o| o.root == root)
            .expect("parent workspace");
        let mut parent_rels: Vec<String> =
            parent.seeds.iter().map(|s| s.relative.clone()).collect();
        parent_rels.sort();
        assert_eq!(parent_rels, vec!["a".to_string(), "b".to_string()]);

        let nested = outcomes
            .iter()
            .find(|o| o.root == root.join("nested"))
            .expect("nested workspace");
        let mut nested_rels: Vec<String> =
            nested.seeds.iter().map(|s| s.relative.clone()).collect();
        nested_rels.sort();
        // The nested subtree is owned by the nested workspace, never the parent.
        // With repo-root-relative framing the nested workspace root and its sub
        // package both carry the `nested/` prefix.
        assert_eq!(
            nested_rels,
            vec!["nested".to_string(), "nested/sub".to_string()]
        );
    }

    #[test]
    fn bazel_without_build_files_yields_no_layer() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("WORKSPACE"));
        assert!(detect_bazel_workspace(root).unwrap().is_empty());
    }

    #[test]
    fn bazel_requires_a_root_marker() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("a/BUILD"));
        // No WORKSPACE/MODULE.bazel at root: not a Bazel workspace.
        assert!(detect_bazel_workspace(root).unwrap().is_empty());
    }

    #[test]
    fn pants_discovers_leaf_build_files_under_one_root() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("pants.toml"));
        touch(&root.join("src/app/BUILD.pants"));
        touch(&root.join("src/lib/BUILD"));

        let outcomes = detect_pants_workspace(root).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].standard, MonorepoStandard::Pants);
        assert_eq!(outcomes[0].seeds.len(), 2);
    }

    #[test]
    fn buck2_discovers_buck_and_targets_leaves() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join(".buckconfig"));
        touch(&root.join("app/BUCK"));
        touch(&root.join("lib/TARGETS"));

        let outcomes = detect_buck2_workspace(root).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].seeds.len(), 2);
    }

    #[test]
    fn buck2_requires_buckconfig() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("app/BUCK"));
        assert!(detect_buck2_workspace(root).unwrap().is_empty());
    }
}
