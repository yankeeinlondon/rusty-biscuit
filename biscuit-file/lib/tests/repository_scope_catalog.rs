use std::path::PathBuf;

use biscuit_file::{
    FileResolutionContext, PackageAreaFallback, RepositoryScopeCatalog,
    RepositoryScopeCatalogError,
};
use tempfile::TempDir;

fn catalog(
    repository_root: PathBuf,
    package_area_roots: Vec<PathBuf>,
    package_roots: Vec<PathBuf>,
) -> RepositoryScopeCatalog {
    RepositoryScopeCatalog::new(
        repository_root,
        package_area_roots,
        package_roots,
        PackageAreaFallback::FirstComponent,
    )
    .expect("valid repository scope catalog")
}

#[test]
fn constructor_validates_roots_without_filesystem_io_and_deduplicates() {
    let root = TempDir::new().unwrap().path().join("repository-that-does-not-exist");
    let area = root.join("area-that-does-not-exist");
    let package = area.join("package-that-does-not-exist");
    assert!(!root.exists());

    let catalog = catalog(
        root.clone(),
        vec![area.clone(), area.clone()],
        vec![package.clone(), package.clone()],
    );

    assert_eq!(catalog.repository_root(), root);
    assert_eq!(catalog.package_area_roots(), &[area]);
    assert_eq!(catalog.package_roots(), &[package]);
    assert!(!catalog.repository_root().exists());
}

#[test]
fn constructor_rejects_relative_non_normalized_and_outside_roots() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let outside = temp.path().join("outside");

    for error in [
        RepositoryScopeCatalog::new(
            "relative/repo",
            Vec::<PathBuf>::new(),
            Vec::<PathBuf>::new(),
            PackageAreaFallback::None,
        )
        .unwrap_err(),
        RepositoryScopeCatalog::new(
            repo.join("docs/.."),
            Vec::<PathBuf>::new(),
            Vec::<PathBuf>::new(),
            PackageAreaFallback::None,
        )
        .unwrap_err(),
        RepositoryScopeCatalog::new(
            repo.clone(),
            vec![outside.clone()],
            Vec::<PathBuf>::new(),
            PackageAreaFallback::None,
        )
        .unwrap_err(),
        RepositoryScopeCatalog::new(
            repo,
            Vec::<PathBuf>::new(),
            vec![outside],
            PackageAreaFallback::None,
        )
        .unwrap_err(),
    ] {
        assert!(matches!(
            error,
            RepositoryScopeCatalogError::RootNotAbsolute { .. }
                | RepositoryScopeCatalogError::RootNotNormalized { .. }
                | RepositoryScopeCatalogError::RootOutsideRepository { .. }
        ));
    }
}

#[test]
fn scope_selection_is_component_aware_and_most_specific_first() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let area = repo.join("apps");
    let root_package = area.join("web");
    let nested_package = root_package.join("plugins/auth");
    let catalog = catalog(
        repo.clone(),
        vec![area.clone()],
        vec![root_package.clone(), nested_package.clone()],
    );

    let nested = catalog.scope_for(&nested_package.join("src"));
    assert_eq!(nested.package_root(), Some(nested_package.as_path()));
    assert_eq!(nested.package_area_root(), Some(area.as_path()));
    assert_eq!(nested.repository_root(), Some(repo.as_path()));

    let area_only = catalog.scope_for(&area.join("new-scaffold/src"));
    assert_eq!(area_only.package_root(), None);
    assert_eq!(area_only.package_area_root(), Some(area.as_path()));

    let prefix_collision = temp.path().join("repository-copy");
    let outside = catalog.scope_for(&prefix_collision);
    assert_eq!(outside.repository_root(), None);
    assert_eq!(outside.package_area_root(), None);
    assert_eq!(outside.package_root(), None);
}

#[test]
fn first_component_fallback_covers_new_monorepo_areas() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let catalog = catalog(repo.clone(), Vec::new(), Vec::new());

    let scope = catalog.scope_for(&repo.join("new-area/scaffold/src"));
    assert_eq!(scope.package_area_root(), Some(repo.join("new-area").as_path()));

    let root_scope = catalog.scope_for(&repo);
    assert_eq!(root_scope.package_area_root(), None);
}

#[test]
fn repository_root_can_also_be_the_root_package() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let catalog = catalog(repo.clone(), Vec::new(), vec![repo.clone()]);

    let scope = catalog.scope_for(&repo.join("src"));
    assert_eq!(scope.package_root(), Some(repo.as_path()));
    assert_eq!(scope.repository_root(), Some(repo.as_path()));
}

#[test]
fn context_derivations_recompute_or_clear_repository_scopes() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let first_area = repo.join("first");
    let first_package = first_area.join("lib");
    let second_area = repo.join("second");
    let second_package = second_area.join("cli");
    let outside = temp.path().join("outside");
    let catalog = catalog(
        repo.clone(),
        vec![first_area.clone(), second_area.clone()],
        vec![first_package.clone(), second_package.clone()],
    );

    let request = FileResolutionContext::new(first_package.join("src"))
        .with_repository_scope_catalog(catalog);
    assert_eq!(request.package_root(), Some(first_package.as_path()));
    assert_eq!(request.package_area(), Some(first_area.as_path()));

    let second = request.for_source(second_package.join("prompts/task.md"));
    assert_eq!(second.package_root(), Some(second_package.as_path()));
    assert_eq!(second.package_area(), Some(second_area.as_path()));
    assert_eq!(second.repository_root(), Some(repo.as_path()));

    let external = request.for_trusted_external_base(&outside);
    assert_eq!(external.package_root(), None);
    assert_eq!(external.package_area(), None);
    assert_eq!(external.repository_root(), None);
    external.validate().expect("trusted external context clears stale scopes");
}

#[test]
fn replacing_the_catalog_selects_the_second_repository() {
    let temp = TempDir::new().unwrap();
    let first_repo = temp.path().join("first-repo");
    let second_repo = temp.path().join("second-repo");
    let second_package = second_repo.join("tools/runner");
    let first = catalog(first_repo.clone(), Vec::new(), Vec::new());
    let second = catalog(
        second_repo.clone(),
        vec![second_repo.join("tools")],
        vec![second_package.clone()],
    );

    let context = FileResolutionContext::new(first_repo.join("docs"))
        .with_repository_scope_catalog(first)
        .for_trusted_external_base(second_package.join("src"))
        .with_repository_scope_catalog(second);

    assert_eq!(context.repository_root(), Some(second_repo.as_path()));
    assert_eq!(context.package_root(), Some(second_package.as_path()));
}
