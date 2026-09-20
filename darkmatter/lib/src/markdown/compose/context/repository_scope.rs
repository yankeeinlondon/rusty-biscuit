//! Projection from retained repository observations into file-reference scopes
//! and package-name lookups.

use std::path::{Path, PathBuf};

use biscuit_file::{
    PackageAreaFallback, RepositoryScopeCatalog, RepositoryScopeCatalogError,
};
use sniff::filesystem::repo::RepoInfo;

/// Project a retained Sniff repository observation into pure file-reference scope data.
///
/// `repository_root` is the request's spelling of the observed repository root.
/// Package and area roots are rebuilt from repository-relative identities so
/// equivalent canonical or symlinked root spellings do not get mixed.
pub fn repository_scope_catalog(
    repo: &RepoInfo,
    repository_root: &Path,
) -> Result<RepositoryScopeCatalog, RepositoryScopeCatalogError> {
    let packages = repo.packages.as_deref().unwrap_or_default();
    let package_roots = packages
        .iter()
        .map(|package| repository_root.join(&package.relative))
        .collect();
    let package_area_roots = packages
        .iter()
        .filter_map(|package| package_area_root(repository_root, &package.package_area))
        .collect();
    let fallback = if repo.is_monorepo {
        PackageAreaFallback::FirstComponent
    } else {
        PackageAreaFallback::None
    };

    RepositoryScopeCatalog::new(
        repository_root,
        package_area_roots,
        package_roots,
        fallback,
    )
}

/// One request's repository as a capture observed it: the root the discovery
/// found and the topology walked under it (decision D3).
///
/// Retained beside the projected `ctx.*` values so the file-resolution scope
/// catalog and the lazy `current.*` repository facts can be projected from the
/// same observation instead of discovering the repository again.
#[derive(Debug, Clone, Default)]
pub(crate) struct RepositoryObservation {
    pub(crate) root: Option<PathBuf>,
    pub(crate) info: Option<RepoInfo>,
}

impl RepositoryObservation {
    /// The file-reference scope catalog this observation projects to, or
    /// `None` when no repository was found.
    pub(crate) fn scope_catalog(&self) -> Option<RepositoryScopeCatalog> {
        let root = self.root.as_deref()?;
        match self.info.as_ref() {
            Some(repo) => repository_scope_catalog(repo, root).ok(),
            None => RepositoryScopeCatalog::new(
                root,
                Vec::new(),
                Vec::new(),
                PackageAreaFallback::None,
            )
            .ok(),
        }
    }

    /// Whether `path` lies inside the observed repository root, by path
    /// components and the root's own spelling.
    pub(crate) fn contains(&self, path: &Path) -> bool {
        self.root.as_deref().is_some_and(|root| path.starts_with(root))
    }
}

/// Package and package-area names keyed by their roots under the captured
/// repository root.
#[derive(Debug)]
pub(crate) struct PackageLookup {
    /// `Ok(None)` outside a repository or monorepo, where every lookup misses.
    /// `Err` retains the rendered projection failure for every later lookup.
    catalog: Result<Option<RepositoryScopeCatalog>, String>,
    package_names: Vec<(PathBuf, String)>,
    area_names: Vec<(PathBuf, String)>,
}

/// The package and package area containing one path; `""` means none (R26).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PackageMatch {
    pub(crate) package: String,
    pub(crate) area: String,
}

impl PackageLookup {
    pub(crate) fn new(root: Option<&Path>, repo: Option<&RepoInfo>) -> Self {
        let (Some(root), Some(repo)) = (root, repo.filter(|repo| repo.is_monorepo)) else {
            return Self { catalog: Ok(None), package_names: Vec::new(), area_names: Vec::new() };
        };
        let packages = repo.packages.as_deref().unwrap_or_default();
        let package_names = packages
            .iter()
            .map(|package| (root.join(&package.relative), package.name.clone()))
            .collect();
        let area_names = packages
            .iter()
            .filter(|package| !package.package_area.is_empty())
            .map(|package| (root.join(&package.package_area), package.package_area.clone()))
            .collect();
        // The shared projection supplies root spelling and validation; the
        // first-component area fallback it applies to file references is
        // replaced because a lookup names only observed areas.
        let catalog = repository_scope_catalog(repo, root)
            .and_then(|projected| {
                RepositoryScopeCatalog::new(
                    projected.repository_root(),
                    projected.package_area_roots().to_vec(),
                    projected.package_roots().to_vec(),
                    PackageAreaFallback::None,
                )
            })
            .map(Some)
            .map_err(|error| error.to_string());
        Self { catalog, package_names, area_names }
    }

    /// Selects the deepest observed package and package area containing `path`
    /// independently, by path components.
    ///
    /// `request_root` is the file-resolution snapshot's spelling of the same
    /// repository; a path under it is rebased onto the observed root so equal
    /// repositories reached through different spellings still match.
    ///
    /// ## Errors
    ///
    /// Returns the catalog validation error when the captured topology could
    /// not be projected (for example, a package root outside the repository).
    pub(crate) fn lookup(
        &self,
        path: &Path,
        request_root: Option<&Path>,
    ) -> Result<PackageMatch, String> {
        let Some(catalog) = self.catalog.as_ref().map_err(Clone::clone)? else {
            return Ok(PackageMatch::default());
        };
        let root = catalog.repository_root();
        let rebased = request_root
            .filter(|request_root| *request_root != root)
            .and_then(|request_root| path.strip_prefix(request_root).ok())
            .map(|relative| root.join(relative));
        let scope = catalog.scope_for(rebased.as_deref().unwrap_or(path));
        let name = |names: &[(PathBuf, String)], selected: Option<&Path>| {
            selected
                .and_then(|selected| names.iter().find(|(root, _)| root == selected))
                .map(|(_, name)| name.clone())
                .unwrap_or_default()
        };
        Ok(PackageMatch {
            package: name(&self.package_names, scope.package_root()),
            area: name(&self.area_names, scope.package_area_root()),
        })
    }
}

fn package_area_root(repository_root: &Path, area: &str) -> Option<PathBuf> {
    if area.is_empty() {
        None
    } else {
        Some(repository_root.join(area))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::filesystem::repo::Package;

    fn package(observed_root: &Path, name: &str, relative: &str, area: &str) -> Package {
        Package {
            name: name.to_owned(),
            relative: relative.to_owned(),
            package_area: area.to_owned(),
            path: observed_root.join(relative),
            ..Package::default()
        }
    }

    fn monorepo(observed_root: &Path) -> RepoInfo {
        RepoInfo {
            is_monorepo: true,
            root: observed_root.to_path_buf(),
            packages: Some(vec![
                package(observed_root, "root-tool", "tools", ""),
                package(observed_root, "area-lib", "area/lib", "area"),
                package(observed_root, "nested", "area/lib/nested", "area"),
            ]),
            ..RepoInfo::default()
        }
    }

    #[test]
    fn projection_preserves_root_area_scaffold_and_nested_package_scopes() {
        let temp = tempfile::tempdir().expect("temp directory");
        let observed_root = temp.path().join("observed-repository");
        let request_root = temp.path().join("request-spelling");
        let catalog = repository_scope_catalog(&monorepo(&observed_root), &request_root)
            .expect("valid projected catalog");

        let root = catalog.scope_for(&request_root);
        assert_eq!(root.repository_root(), Some(request_root.as_path()));
        assert_eq!(root.package_root(), None);
        assert_eq!(root.package_area_root(), None);

        let known_area = catalog.scope_for(&request_root.join("area/new-file.md"));
        assert_eq!(known_area.package_area_root(), Some(request_root.join("area").as_path()));
        assert_eq!(known_area.package_root(), None);

        let scaffolded = catalog.scope_for(&request_root.join("scaffold/new/src"));
        assert_eq!(
            scaffolded.package_area_root(),
            Some(request_root.join("scaffold").as_path())
        );
        assert_eq!(scaffolded.package_root(), None);

        let root_package = catalog.scope_for(&request_root.join("tools/src"));
        assert_eq!(root_package.package_area_root(), None);
        assert_eq!(root_package.package_root(), Some(request_root.join("tools").as_path()));

        let nested = catalog.scope_for(&request_root.join("area/lib/nested/src"));
        assert_eq!(nested.package_area_root(), Some(request_root.join("area").as_path()));
        assert_eq!(
            nested.package_root(),
            Some(request_root.join("area/lib/nested").as_path())
        );
    }

    #[test]
    fn projection_rebuilds_foreign_spelled_package_paths_under_request_root() {
        let temp = tempfile::tempdir().expect("temp directory");
        let observed_root = temp.path().join("canonical-repository");
        let request_root = temp.path().join("symlink-spelling");
        let catalog = repository_scope_catalog(&monorepo(&observed_root), &request_root)
            .expect("valid projected catalog");

        assert!(
            catalog
                .package_roots()
                .iter()
                .all(|root| root.starts_with(&request_root))
        );
        assert!(
            catalog
                .package_roots()
                .iter()
                .all(|root| !root.starts_with(&observed_root))
        );
    }

    #[test]
    fn projection_keeps_repository_snapshots_independent() {
        let temp = tempfile::tempdir().expect("temp directory");
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        let first_catalog = repository_scope_catalog(&monorepo(&first), &first)
            .expect("first catalog");
        let second_catalog = repository_scope_catalog(&monorepo(&second), &second)
            .expect("second catalog");

        assert!(first_catalog.scope_for(&second.join("area/lib")).repository_root().is_none());
        assert_eq!(
            second_catalog
                .scope_for(&second.join("area/lib"))
                .repository_root(),
            Some(second.as_path())
        );
    }

    #[test]
    fn non_monorepo_projection_has_no_scaffolded_area_fallback() {
        let temp = tempfile::tempdir().expect("temp directory");
        let root = temp.path().join("repository");
        let repo = RepoInfo {
            root: root.clone(),
            ..RepoInfo::default()
        };
        let catalog = repository_scope_catalog(&repo, &root).expect("valid catalog");

        let scope = catalog.scope_for(&root.join("src"));
        assert_eq!(scope.repository_root(), Some(root.as_path()));
        assert_eq!(scope.package_area_root(), None);
        assert_eq!(scope.package_root(), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn projection_preserves_var_symlink_spelling_on_macos() {
        let temp = tempfile::tempdir().expect("temp directory");
        let request_root = temp.path();
        let observed_root = request_root.canonicalize().expect("canonical temp path");
        let catalog = repository_scope_catalog(&monorepo(&observed_root), request_root)
            .expect("valid projected catalog");

        assert_eq!(catalog.repository_root(), request_root);
        assert!(
            catalog
                .package_roots()
                .iter()
                .all(|root| root.starts_with(request_root))
        );
        let nested = catalog.scope_for(&request_root.join("area/lib/nested/src"));
        assert_eq!(
            nested.package_root(),
            Some(request_root.join("area/lib/nested").as_path())
        );
    }

    #[test]
    fn resolver_inventory_has_one_projection_adapter_and_no_discovery_fallbacks() {
        let source_root = biscuit_test_harness::manifest_dir!().join("src");
        let mut stack = vec![source_root.clone()];
        let mut adapter_files = Vec::new();
        while let Some(directory) = stack.pop() {
            for entry in std::fs::read_dir(&directory).expect("read source directory") {
                let path = entry.expect("source entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path).expect("read Rust source");
                if source.contains("RepoInfo") && source.contains("RepositoryScopeCatalog") {
                    adapter_files.push(path.clone());
                }
            }
        }
        assert_eq!(
            adapter_files,
            vec![source_root.join("markdown/compose/context/repository_scope.rs")]
        );

        for relative in [
            "markdown/reference/mod.rs",
            "markdown/compose/transclusion/resolver.rs",
            "markdown/compose/link_resolve.rs",
            "markdown/compose/link_normalization.rs",
            "markdown/compose/schema_validation.rs",
            "markdown/schemas/resolve.rs",
            "markdown/schemas/rewrite.rs",
            "markdown/schemas/detect.rs",
            "markdown/schemas/format.rs",
            "markdown/compose/expression/path_projection.rs",
            "markdown/compose/expression/functions/mod.rs",
        ] {
            let source = std::fs::read_to_string(source_root.join(relative))
                .unwrap_or_else(|error| panic!("read {relative}: {error}"));
            assert!(
                !source.contains("find_git_root_from"),
                "resolver `{relative}` must consume the request snapshot"
            );
        }
    }
}

#[cfg(test)]
mod lookup_tests {
    use sniff::filesystem::repo::Package;

    use super::*;

    fn package(root: &Path, name: &str, relative: &str, area: &str) -> Package {
        Package {
            name: name.to_owned(),
            relative: relative.to_owned(),
            package_area: area.to_owned(),
            path: root.join(relative),
            ..Package::default()
        }
    }

    fn monorepo(root: &Path) -> RepoInfo {
        RepoInfo {
            is_monorepo: true,
            root: root.to_path_buf(),
            packages: Some(vec![
                package(root, "tool", "tool", ""),
                package(root, "foo-lib", "foo/lib", "foo"),
                package(root, "foo-nested", "foo/lib/nested", "foo"),
                package(root, "rendezvous", "claudine/rendezvous/lib", "claudine/rendezvous"),
                package(root, "claudine", "claudine/lib", "claudine"),
            ]),
            ..RepoInfo::default()
        }
    }

    fn lookup(root: &Path, relative: &str) -> PackageMatch {
        PackageLookup::new(Some(root), Some(&monorepo(root)))
            .lookup(&root.join(relative), None)
            .expect("valid catalog")
    }

    fn matched(package: &str, area: &str) -> PackageMatch {
        PackageMatch { package: package.into(), area: area.into() }
    }

    #[test]
    fn deepest_package_and_area_are_selected_independently() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        assert_eq!(lookup(root, "foo/lib/src/lib.rs"), matched("foo-lib", "foo"));
        assert_eq!(lookup(root, "foo/lib/nested/src/x.rs"), matched("foo-nested", "foo"));
        assert_eq!(lookup(root, "tool/src/main.rs"), matched("tool", ""));
        assert_eq!(lookup(root, "foo/docs/readme.md"), matched("", "foo"));
        assert_eq!(
            lookup(root, "claudine/rendezvous/lib/src/lib.rs"),
            matched("rendezvous", "claudine/rendezvous")
        );
        assert_eq!(lookup(root, "claudine/rendezvous/notes.md"), matched("", "claudine/rendezvous"));
        assert_eq!(lookup(root, "claudine/lib/src/lib.rs"), matched("claudine", "claudine"));
    }

    /// Components, not string prefixes: `foobar` is not inside `foo`, and an
    /// unobserved top-level directory gets no inferred area.
    #[test]
    fn sibling_prefixes_root_files_and_unobserved_directories_miss() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        assert_eq!(lookup(root, "foobar/lib/src/lib.rs"), PackageMatch::default());
        assert_eq!(lookup(root, "foo/libx/src.rs"), matched("", "foo"));
        assert_eq!(lookup(root, "README.md"), PackageMatch::default());
        assert_eq!(lookup(root, "scaffold/new/src"), PackageMatch::default());
        assert_eq!(lookup(root, ""), PackageMatch::default());
        assert_eq!(lookup(root, "foo/lib/../../tool/x"), matched("tool", ""));
    }

    #[test]
    fn outside_repository_and_non_monorepo_miss() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        let outside = temp.path().join("elsewhere/foo/lib/src/lib.rs");
        let lookup_table = PackageLookup::new(Some(&root), Some(&monorepo(&root)));
        assert_eq!(lookup_table.lookup(&outside, None).unwrap(), PackageMatch::default());

        let mut plain = monorepo(&root);
        plain.is_monorepo = false;
        let plain = PackageLookup::new(Some(&root), Some(&plain));
        assert_eq!(plain.lookup(&root.join("foo/lib/src/lib.rs"), None).unwrap(), PackageMatch::default());

        let unobserved = PackageLookup::new(None, None);
        assert_eq!(unobserved.lookup(&root.join("foo/lib"), None).unwrap(), PackageMatch::default());
    }

    #[test]
    fn request_root_spelling_is_rebased_onto_the_observed_root() {
        let temp = tempfile::tempdir().unwrap();
        let observed = temp.path().join("canonical");
        let request = temp.path().join("symlink");
        let table = PackageLookup::new(Some(&observed), Some(&monorepo(&observed)));
        assert_eq!(
            table.lookup(&request.join("foo/lib/src/lib.rs"), Some(&request)).unwrap(),
            matched("foo-lib", "foo")
        );
        assert_eq!(table.lookup(&request.join("foo/lib/src/lib.rs"), None).unwrap(), PackageMatch::default());
    }

    #[cfg(windows)]
    #[test]
    fn windows_drive_letters_and_separators_match_by_component() {
        let root = Path::new(r"C:\work\repo");
        let table = PackageLookup::new(Some(root), Some(&monorepo(root)));
        assert_eq!(
            table.lookup(Path::new(r"c:\work\repo\foo\lib\src\lib.rs"), None).unwrap(),
            matched("foo-lib", "foo")
        );
        assert_eq!(
            table.lookup(Path::new("C:/work/repo/foo/lib/src/lib.rs"), None).unwrap(),
            matched("foo-lib", "foo")
        );
        assert_eq!(
            table.lookup(Path::new(r"D:\work\repo\foo\lib\src\lib.rs"), None).unwrap(),
            PackageMatch::default()
        );
    }
}
