use std::{collections::HashMap, fs, path::{Path, PathBuf}, sync::Arc};

use biscuit_file::{
    AttemptOutcome, EntryKind, FileResolutionContext, FsShadowProbe, ImplicitBases,
    PortabilityPreference as P, PortablePath, PortableReference, ProbeError, ShadowCache, ShadowProbe,
};

fn write(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "file").unwrap();
}

fn context(repo: &Path, launch: &Path) -> FileResolutionContext {
    fs::create_dir_all(launch).unwrap();
    FileResolutionContext::from_snapshot(launch, None, HashMap::new())
        .with_repository_root(repo)
        .with_implicit_bases(ImplicitBases::Launch)
}

fn spell(path: &Path, ctx: &FileResolutionContext) -> PortablePath {
    PortablePath::from_path(path).with_ctx(ctx).with_strategy(P::VALUE_STRATEGY.iter().cloned())
}

fn assert_round_trip(spelling: &PortableReference, ctx: &FileResolutionContext, kind: EntryKind, target: &Path) {
    let plan = spelling.reference().plan_target(ctx, kind).unwrap();
    assert_eq!(plan.path, target);
}

#[test]
fn spec_directory_and_index_paths_are_short_and_resolve_back() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path().join("repo");
    let ctx = context(&repo, &repo).for_source(repo.join("prompts/prompt.md"));
    for text in ["feat/x/spec.md", "feat/x/review-2.md"] {
        let target = repo.join(text);
        write(&target);
        let value = spell(&target, &ctx).file_reference().unwrap();
        assert_eq!(value.reference().raw(), text);
        assert!(!value.reference().raw().contains("repo"));
        assert_round_trip(&value, &ctx, EntryKind::File, &target);
    }
    let parent = repo.join("feat/x");
    let value = spell(&parent, &ctx).file_reference().unwrap();
    assert_eq!(value.reference().raw(), "feat/x");
    assert_round_trip(&value, &ctx, EntryKind::Directory, &parent);

    let absolute = spell(&repo.join("feat/x/spec.md"), &ctx)
        .with_strategy([P::AbsolutePath]).file_reference().unwrap();
    assert!(Path::new(absolute.reference().raw()).is_absolute());
    assert_round_trip(&absolute, &ctx, EntryKind::File, &repo.join("feat/x/spec.md"));
}

#[test]
fn repository_root_and_foreign_checkout_do_not_get_implicit_spellings() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path().join("repo");
    let ctx = context(&repo, &repo).with_home_dir(fixture.path());
    let root = spell(&repo, &ctx).file_reference().unwrap();
    assert_eq!(root.reference().raw(), "~/repo");
    assert_round_trip(&root, &ctx, EntryKind::Directory, &repo);

    let foreign = repo.join("vendor");
    let target = foreign.join("spec.md");
    write(&target);
    let value = spell(&target, &ctx)
        .with_foreign_checkouts([foreign])
        .file_reference().unwrap();
    assert_eq!(value.reference().raw(), "~/repo/vendor/spec.md");
    assert_round_trip(&value, &ctx, EntryKind::File, &target);
}

#[test]
fn shadow_and_outside_launch_use_environment_home_and_absolute_in_order() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path().join("repo");
    let launch = repo.join("tools");
    let outside = fixture.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let target = repo.join("feat/x.md");
    write(&target);
    write(&launch.join("feat/x.md"));
    for ctx in [
        context(&repo, &launch),
        FileResolutionContext::from_snapshot(&outside, None, HashMap::new())
            .with_implicit_bases(ImplicitBases::Launch),
        context(&outside, &outside),
    ] {
        let ctx = ctx.with_home_dir(&repo);
        let env_ctx = ctx.clone().with_env(HashMap::from([
            ("PORTABLE_ENV_VARIABLES".into(), "WORKSPACE".into()),
            ("WORKSPACE".into(), repo.to_str().unwrap().into()),
        ]));
        for (ctx, preference, text) in [
            (&env_ctx, P::EnvRootedPath, "{{WORKSPACE}}/feat/x.md"),
            (&ctx, P::HomeDir, "~/feat/x.md"),
            (&ctx.clone().without_home_dir(), P::AbsolutePath, ""),
        ] {
            let value = spell(&target, ctx).file_reference().unwrap();
            assert_eq!(value.strategy(), &preference);
            if !text.is_empty() {
                assert_eq!(value.reference().raw(), text);
            }
            assert_round_trip(&value, ctx, EntryKind::File, &target);
        }
    }
}

#[test]
fn a_missing_target_requires_the_same_planned_parent() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let launch = repo.join("tools");
    let ctx = context(repo, &launch).with_home_dir(repo);
    let target = repo.join("feat/x/review-3.md");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    let value = spell(&target, &ctx).file_reference().unwrap();
    assert_eq!(value.reference().raw(), "feat/x/review-3.md");
    assert_round_trip(&value, &ctx, EntryKind::File, &target);

    fs::create_dir_all(launch.join("feat/x")).unwrap();
    for (target, expected) in [
        (target, "~/feat/x/review-3.md"),
        (repo.join("new.md"), "~/new.md"),
        (repo.join("uncreated/deep/new.md"), "~/uncreated/deep/new.md"),
    ] {
        let value = spell(&target, &ctx).file_reference().unwrap();
        assert_eq!(value.reference().raw(), expected);
        assert_round_trip(&value, &ctx, EntryKind::File, &target);
    }
}

#[test]
fn shared_cache_probes_one_first_component_per_nearer_base() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let area = repo.join("area");
    let package = area.join("package");
    let ctx = context(repo, &package.join("tools"))
        .with_package_root(&package).with_package_area(&area);
    let cache = ShadowCache::new(Arc::new(FsShadowProbe));
    fs::create_dir(repo.join("feat")).unwrap();
    for index in 0..500 {
        let relative = format!("feat/review-{index}.md");
        let value = spell(&repo.join(&relative), &ctx)
            .with_shadow_cache(&cache.clone()).file_reference().unwrap();
        assert_eq!(value.reference().raw(), relative);
    }
    assert_eq!(cache.probe_count(), 3);
    fs::create_dir(repo.join("docs")).unwrap();
    let value = spell(&repo.join("docs/new.md"), &ctx)
        .with_shadow_cache(&cache).file_reference().unwrap();
    assert_eq!(value.reference().raw(), "docs/new.md");
    assert_eq!(cache.probe_count(), 6);
}

#[test]
fn a_denied_shadow_check_records_the_probe_and_uses_the_fallback() {
    struct Deny(PathBuf);
    impl ShadowProbe for Deny {
        fn exists(&self, path: &Path) -> Result<bool, ProbeError> {
            assert_eq!(path, self.0);
            Err(ProbeError {
                path: path.into(), kind: std::io::ErrorKind::PermissionDenied,
                os_code: None, message: "fixture denial".into(),
            })
        }
    }

    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let launch = repo.join("tools");
    let ctx = context(repo, &launch).with_home_dir(repo);
    let denied = launch.join("feat");
    let cache = ShadowCache::new(Arc::new(Deny(denied.clone())));
    let target = repo.join("feat/x.md");
    write(&target);
    let value = spell(&target, &ctx).with_shadow_cache(&cache).file_reference().unwrap();
    assert_eq!(value.reference().raw(), "~/feat/x.md");
    assert!(matches!(&value.attempts()[0].outcome, AttemptOutcome::ProbeFailed(error)
        if error.path == denied && error.kind == std::io::ErrorKind::PermissionDenied));
    assert_eq!(cache.probe_count(), 1);
}

#[cfg(windows)]
#[test]
fn absolute_value_names_the_native_windows_target() {
    let fixture = tempfile::tempdir().unwrap();
    let target = fixture.path().join("spec.md");
    write(&target);
    let ctx = FileResolutionContext::from_snapshot(fixture.path(), None, HashMap::new());
    let value = spell(&target, &ctx).file_reference().unwrap();
    assert_eq!(value.strategy(), &P::AbsolutePath);
    assert!(Path::new(value.reference().raw()).is_absolute());
    assert_round_trip(&value, &ctx, EntryKind::File, &target);
}

#[test]
fn deep_shadow_check_can_be_refused_before_probing_the_candidate() {
    struct DenyCandidate(PathBuf);
    impl ShadowProbe for DenyCandidate {
        fn admit(&self, path: &Path) -> Result<(), ProbeError> {
            if path == self.0 {
                return Err(ProbeError {
                    path: path.into(), kind: std::io::ErrorKind::PermissionDenied,
                    os_code: None, message: "fixture denial".into(),
                });
            }
            Ok(())
        }

        fn exists(&self, path: &Path) -> Result<bool, ProbeError> {
            assert_ne!(path, self.0);
            FsShadowProbe.exists(path)
        }
    }

    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let launch = repo.join("tools");
    let ctx = context(repo, &launch).with_home_dir(repo);
    let target = repo.join("feat/x.md");
    let denied = launch.join("feat/x.md");
    write(&target);
    write(&denied);
    let cache = ShadowCache::new(Arc::new(DenyCandidate(denied.clone())));
    let value = spell(&target, &ctx).with_shadow_cache(&cache).file_reference().unwrap();
    assert_eq!(value.reference().raw(), "~/feat/x.md");
    assert!(matches!(&value.attempts()[0].outcome, AttemptOutcome::ProbeFailed(error)
        if error.path == denied));
}

#[test]
#[cfg(unix)]
fn a_symlinked_launch_base_outside_the_tree_cannot_prove_a_short_value() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path().join("repo");
    let outside = fixture.path().join("outside");
    let launch = repo.join("tools");
    let target = repo.join("feat/x.md");
    write(&target);
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, &launch).unwrap();
    let ctx = context(&repo, &launch).with_home_dir(&repo);
    let value = spell(&target, &ctx).file_reference().unwrap();
    assert_eq!(value.reference().raw(), "~/feat/x.md");
    assert_round_trip(&value, &ctx, EntryKind::File, &target);
}
