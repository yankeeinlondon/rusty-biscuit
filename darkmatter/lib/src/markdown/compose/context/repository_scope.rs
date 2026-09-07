//! Projection from retained repository observations into file-reference scopes.

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

fn package_area_root(repository_root: &Path, area: &str) -> Option<PathBuf> {
    if area == "root" || area.is_empty() {
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
                package(observed_root, "root-tool", "tools", "root"),
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
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
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
