//! An excess `..` at a filesystem root names the root itself (`/..` is `/`),
//! so a path or anchor spelled through one must behave exactly like its plain
//! spelling in resolution, context selection, and generation — the same rule
//! `PathIdentity` applies when comparing.

use std::ffi::OsString;
use std::path::Component;

use biscuit_file::{
    BaseDirOrigin, PackageAreaFallback, PathPosition, PortabilityPreference as P,
    RepositoryScopeCatalog,
};

use super::*;

/// `path` respelled with a `..` straight after its root: `/../a` on Unix,
/// `C:\..\a` on Windows. Both name the same file as `path`.
fn via_root_parent(path: &Path) -> PathBuf {
    let mut text = OsString::new();
    let mut rest = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => text.push(component.as_os_str()),
            other => rest.push(other.as_os_str().to_os_string()),
        }
    }
    text.push("..");
    for name in rest {
        text.push(std::path::MAIN_SEPARATOR_STR);
        text.push(name);
    }
    let respelled = PathBuf::from(text);
    assert_ne!(respelled, path, "the respelling must differ in text");
    respelled
}

fn text(path: &Path) -> String {
    path.to_str().expect("fixture paths are UTF-8").to_string()
}

#[test]
fn absolute_path_strategy_accepts_a_target_spelled_through_the_root_parent() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&cwd, &[]);

    let found = evaluate(
        PortablePath::from_path(via_root_parent(&target))
            .with_ctx(&ctx)
            .with_strategy([P::AbsolutePath]),
    );

    assert_eq!(found.strategy(), &P::AbsolutePath);
    let resolved = found.reference().resolve_in_context(&ctx).unwrap().unwrap();
    assert!(resolved.is_absolute(), "{}", resolved.display());
    assert!(same_file(&resolved, &target));
}

#[test]
fn an_absolute_reference_through_the_root_parent_resolves_to_the_absolute_target() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&cwd, &[]);
    let respelled = reference(&absolute_text(&via_root_parent(&target)));

    assert_eq!(respelled.resolve_in_context(&ctx).unwrap(), Some(target.clone()));
    assert_eq!(respelled.resolve_detailed(&ctx).matched_path(), Some(target.as_path()));
}

#[test]
fn from_reference_generates_the_same_link_as_the_plain_spelling() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&cwd, &[]);

    let plain = evaluate(PortablePath::from_reference(reference(&absolute_text(&target))).with_ctx(&ctx));
    let respelled = evaluate(
        PortablePath::from_reference(reference(&absolute_text(&via_root_parent(&target)))).with_ctx(&ctx),
    );

    assert_eq!(plain.reference().raw(), "./x.md");
    assert_eq!(respelled.reference().raw(), "./x.md");
}

#[test]
fn context_containment_accepts_root_parent_spellings_of_cwd_and_roots() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let respelled_cwd = via_root_parent(&cwd);
    let respelled_repo = via_root_parent(&fx.repo);

    let changed_cwd = fx.bare_ctx(&respelled_cwd, &[]).with_base_dir(&fx.repo);
    changed_cwd.validate().expect("a contained cwd spelled through the root parent");

    let changed_tree_root = fx.bare_ctx(&cwd, &[]).with_base_dir(&respelled_repo);
    changed_tree_root.validate().expect("an explicit tree root spelled through the root parent");

    let changed_repository = fx.bare_ctx(&cwd, &[]).with_repository_root(&respelled_repo);
    changed_repository.validate().expect("a repository root spelled through the root parent");
}

#[test]
fn a_vault_root_spelled_through_the_root_parent_is_selected() {
    let fx = Fixture::new();
    let cwd = fx.dir("vault/notes");
    let vault = fx.path("vault");

    let ctx = fx.bare_ctx(&cwd, &[]).add_vault(via_root_parent(&vault));

    assert_eq!(ctx.base_dir_origin(), &BaseDirOrigin::Vault);
}

#[test]
fn home_dir_strategy_uses_a_home_anchor_spelled_through_the_root_parent() {
    let fx = Fixture::new();
    let cwd = fx.dir("work");
    let target = fx.file("home/x.md");
    let ctx = fx.bare_ctx(&cwd, &[]).with_home_dir(via_root_parent(&fx.home));

    let found = evaluate(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::HomeDir]));

    assert_eq!(found.reference().raw(), "~/x.md");
}

#[test]
fn env_rooted_strategy_uses_a_value_spelled_through_the_root_parent() {
    let fx = Fixture::new();
    let cwd = fx.dir("work");
    let target = fx.file("home/x.md");
    let root = text(&via_root_parent(&fx.home));
    let ctx = fx.bare_ctx(&cwd, &[("ROOT", &root)]);

    let found = evaluate(
        PortablePath::from_path(&target)
            .with_ctx(&ctx)
            .with_portable_env(["ROOT"])
            .with_strategy([P::EnvRootedPath]),
    );

    assert_eq!(found.reference().raw(), "{{ROOT}}/x.md");
}

#[test]
fn opening_anchors_spelled_through_the_root_parent_select_the_tree() {
    let fx = Fixture::new();
    let source = fx.file("home/x.md");
    let respelled_home = via_root_parent(&fx.home);

    let home_launch = fx.bare_ctx(&fx.root, &[]).with_home_dir(&respelled_home);
    let home_child = home_launch.for_source_reference(&reference("~/x.md"), &source);
    assert_eq!(home_child.base_dir_origin(), &BaseDirOrigin::Home);

    let env_launch = fx.bare_ctx(&fx.root, &[("ROOT", &text(&respelled_home))]);
    let env_child = env_launch.for_source_reference(&reference("{{ROOT}}/x.md"), &source);
    assert_eq!(
        env_child.base_dir_origin(),
        &BaseDirOrigin::Environment {
            name: "ROOT".to_string()
        }
    );
}

#[test]
fn magic_path_strategy_uses_a_configured_root_spelled_through_the_root_parent() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let prompts = fx.dir("prompts");
    let target = fx.file("prompts/x.md");
    let ctx = fx
        .ctx(&cwd, &[])
        .without_home_dir()
        .add_magic_path(via_root_parent(&prompts), PathPosition::Start);

    for root in ctx.magic_search_roots() {
        assert!(root.path().is_absolute(), "{}", root.path().display());
    }
    let found = evaluate(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::MagicPath(None)]));

    assert_eq!(found.reference().raw(), "@x.md");
}

#[test]
fn catalog_scope_selection_accepts_a_source_dir_spelled_through_the_root_parent() {
    let fx = Fixture::new();
    let area = fx.dir("repo/area");
    let catalog = RepositoryScopeCatalog::new(
        fx.repo.clone(),
        vec![area.clone()],
        Vec::new(),
        PackageAreaFallback::FirstComponent,
    )
    .unwrap();

    let scope = catalog.scope_for(&via_root_parent(&area.join("docs")));

    assert_eq!(scope.repository_root(), Some(fx.repo.as_path()));
    assert_eq!(scope.package_area_root(), Some(area.as_path()));
}

#[test]
fn resolve_relative_accepts_a_base_spelled_through_the_root_parent() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");

    let relative = reference(&absolute_text(&target))
        .resolve_relative(Some(&via_root_parent(&cwd)))
        .unwrap();

    assert_eq!(relative, Some(PathBuf::from("x.md")));
}

#[test]
fn recursive_resolution_keeps_an_absolute_target_spelled_through_the_root_parent() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&cwd, &[]);
    let recursive = reference(&format!("%{}", absolute_text(&via_root_parent(&target))));

    assert_eq!(recursive.resolve_in_context(&ctx).unwrap(), Some(target));
}

/// More `..` hops than a drive root has directories still stop at the drive
/// root, and the result keeps its drive.
#[cfg(windows)]
#[test]
fn excess_parents_at_a_drive_root_stay_on_the_drive() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&cwd, &[]);
    let respelled = via_root_parent(&via_root_parent(&target));
    assert!(text(&respelled).contains(r":\..\..\"), "{}", respelled.display());

    let resolved = reference(&absolute_text(&respelled)).resolve_in_context(&ctx).unwrap();

    assert_eq!(resolved, Some(target));
}
