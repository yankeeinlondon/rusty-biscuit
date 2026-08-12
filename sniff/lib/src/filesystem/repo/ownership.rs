//! Request-scoped package ownership lookup.
//!
//! Package boundaries are normalized once, then inventory, document, and Git
//! paths are attributed by walking parent components in memory. Keys remain
//! native [`PathBuf`] values; no string-prefix or case-folding rules are
//! introduced.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::detection::{canonicalize_path, normalize_path};
use super::types::Package;

/// Deepest-prefix package lookup for one repository observation.
#[derive(Debug, Clone, Default)]
pub(crate) struct PackageOwnershipIndex {
    root: PathBuf,
    owners: HashMap<PathBuf, usize>,
}

impl PackageOwnershipIndex {
    /// Build from keys already normalized by package discovery.
    ///
    /// `relative` supplies a lexical alias in the observation-root frame. The
    /// resolved key preserves `RepoInfo::package_for_dir`'s historical symlink
    /// behavior, while the alias preserves inventory, document, and Git paths,
    /// which are observed relative to that root.
    pub(crate) fn from_normalized_keys<'a>(
        root: PathBuf,
        entries: impl IntoIterator<Item = (&'a Path, &'a Path, usize)>,
    ) -> Self {
        let mut owners = HashMap::new();
        for (resolved, relative, index) in entries {
            owners.entry(resolved.to_path_buf()).or_insert(index);
            owners
                .entry(normalize_path(&root.join(relative)))
                .or_insert(index);
        }
        Self { root, owners }
    }

    /// Build a request companion from a public package catalog.
    pub(crate) fn from_packages(root: &Path, packages: &[Package]) -> Self {
        let root = canonicalize_path(root);
        let entries: Vec<_> = packages
            .iter()
            .enumerate()
            .map(|(index, package)| {
                (
                    canonicalize_path(&package.path),
                    PathBuf::from(&package.relative),
                    index,
                )
            })
            .collect();
        Self::from_normalized_keys(
            root,
            entries
                .iter()
                .map(|(resolved, relative, index)| (resolved.as_path(), relative.as_path(), *index)),
        )
    }

    /// Build for APIs whose package catalog is already repo-relative.
    pub(crate) fn from_relative_paths(
        root: &Path,
        packages: &[(String, PathBuf)],
    ) -> Self {
        let root = canonicalize_path(root);
        let mut owners = HashMap::new();
        for (index, (_, relative)) in packages.iter().enumerate() {
            owners
                .entry(normalize_path(&root.join(relative)))
                .or_insert(index);
        }
        Self { root, owners }
    }

    /// Look up an already-normalized absolute path.
    pub(crate) fn lookup_normalized(&self, path: &Path) -> Option<usize> {
        let mut current = path;
        loop {
            if let Some(index) = self.owners.get(current) {
                return Some(*index);
            }
            current = current.parent()?;
        }
    }

    /// Look up a path observed relative to the repository root.
    pub(crate) fn lookup_relative(&self, path: &Path) -> Option<usize> {
        let absolute = normalize_path(&self.root.join(path));
        self.lookup_normalized(&absolute)
    }

    /// The canonicalized observation root, for lexical prefix fallbacks that
    /// would otherwise re-canonicalize it per query.
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::{counters, testing};

    fn index() -> PackageOwnershipIndex {
        let root = PathBuf::from("/repo");
        let entries = [
            (PathBuf::from("/repo/crates/pkg-a"), PathBuf::from("crates/pkg-a"), 0),
            (
                PathBuf::from("/repo/crates/pkg-a/nested"),
                PathBuf::from("crates/pkg-a/nested"),
                1,
            ),
            (PathBuf::from("/repo/crates/pkg-a2"), PathBuf::from("crates/pkg-a2"), 2),
        ];
        PackageOwnershipIndex::from_normalized_keys(
            root,
            entries
                .iter()
                .map(|(resolved, relative, index)| (resolved.as_path(), relative.as_path(), *index)),
        )
    }

    #[test]
    fn chooses_deepest_component_prefix_without_matching_siblings() {
        let index = index();
        assert_eq!(
            index.lookup_relative(Path::new("crates/pkg-a/nested/src/lib.rs")),
            Some(1)
        );
        assert_eq!(
            index.lookup_relative(Path::new("crates/pkg-a/src/lib.rs")),
            Some(0)
        );
        assert_eq!(
            index.lookup_relative(Path::new("crates/pkg-a2/src/lib.rs")),
            Some(2)
        );
        assert_eq!(index.lookup_relative(Path::new("crates/pkg-a20/lib.rs")), None);
    }

    #[test]
    fn hot_relative_lookups_reuse_normalized_keys() {
        let index = index();
        let ((), counts) = testing::measure(|| {
            for _ in 0..100 {
                assert_eq!(
                    index.lookup_relative(Path::new("crates/pkg-a/nested/src/lib.rs")),
                    Some(1)
                );
            }
        });
        assert_eq!(counts.get(counters::FS_CANONICALIZATIONS), 0);
    }

    #[cfg(unix)]
    #[test]
    fn preserves_non_utf8_native_components() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let component = OsString::from_vec(vec![b'p', b'k', b'g', 0xff]);
        let relative = PathBuf::from("crates").join(&component);
        let resolved = PathBuf::from("/repo").join(&relative);
        let entries = [(resolved, relative.clone(), 7)];
        let index = PackageOwnershipIndex::from_normalized_keys(
            PathBuf::from("/repo"),
            entries
                .iter()
                .map(|(resolved, relative, owner)| (resolved.as_path(), relative.as_path(), *owner)),
        );

        assert_eq!(index.lookup_relative(&relative.join("src/lib.rs")), Some(7));
    }

    #[cfg(windows)]
    #[test]
    fn preserves_windows_drive_prefix_and_casing() {
        let entries = [(
            PathBuf::from(r"C:\Repo\crates\pkg"),
            PathBuf::from(r"crates\pkg"),
            3,
        )];
        let index = PackageOwnershipIndex::from_normalized_keys(
            PathBuf::from(r"C:\Repo"),
            entries
                .iter()
                .map(|(resolved, relative, owner)| (resolved.as_path(), relative.as_path(), *owner)),
        );

        assert_eq!(index.lookup_relative(Path::new(r"crates\pkg\src\lib.rs")), Some(3));
        assert_eq!(index.lookup_relative(Path::new(r"crates\PKG\src\lib.rs")), None);
    }
}
