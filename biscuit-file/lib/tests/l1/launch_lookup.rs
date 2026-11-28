use std::{collections::HashMap, fs, path::Path};

use biscuit_file::{
    EntryKind, FileReference, FileReferenceError, FileResolutionContext, GlobReference,
    ImplicitBases, RootProvenance,
};

fn reference(text: &str) -> FileReference {
    FileReference::new(text).unwrap()
}

fn context(repo: &Path, launch: &Path) -> FileResolutionContext {
    fs::create_dir_all(launch).unwrap();
    FileResolutionContext::from_snapshot(launch, None, HashMap::new())
        .with_repository_root(repo)
        .with_implicit_bases(ImplicitBases::Launch)
}

fn write(root: &Path, name: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, name).unwrap();
}

#[test]
fn bases_deduplicate_in_launch_order() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let area = repo.join("area");
    let package = area.join("package");
    let launch = package.join("nested");
    let ctx = context(repo, &launch)
        .with_package_area(&area)
        .with_package_root(&package);
    assert_eq!(ctx.launch_scope().bases(), [&launch, &package, &area, repo]);
    let ctx = context(repo, &package)
        .with_package_area(&area)
        .with_package_root(&package);
    assert_eq!(ctx.launch_scope().bases(), [&package, &area, repo]);
    assert_eq!(context(repo, repo).launch_scope().bases(), [repo]);
    assert_eq!(context(repo, &area).launch_scope().bases(), [&area, repo]);
}

#[test]
fn outside_repository_uses_only_launch_directory() {
    let fixture = tempfile::tempdir().unwrap();
    let launch = fixture.path().join("launch");
    let author = fixture.path().join("author");
    write(&launch, "spec.md");
    write(&author, "spec.md");
    let ctx = FileResolutionContext::from_snapshot(&launch, None, HashMap::new())
        .with_package_area(fixture.path())
        .with_implicit_bases(ImplicitBases::Launch)
        .for_cwd(&author);
    assert_eq!(ctx.launch_scope().bases(), [launch.as_path()]);
    assert_eq!(reference("spec.md").resolve_in_context(&ctx).unwrap(), Some(launch.join("spec.md")));
}

#[test]
fn launch_package_area_and_repository_shadows_are_tried_in_order() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let area = repo.join("area");
    let package = area.join("package");
    let launch = package.join("nested");
    let author = repo.join("prompts");
    let ctx = context(repo, &launch)
        .with_package_area(&area)
        .with_package_root(&package)
        .for_source(author.join("prompt.md"));
    for base in [repo, &area, &package, &launch, &author] {
        write(base, "feat/x/spec.md");
    }
    for base in [&launch, &package, &area, repo] {
        let expected = base.join("feat/x/spec.md");
        assert_eq!(reference("feat/x/spec.md").resolve_in_context(&ctx).unwrap(), Some(expected.clone()));
        fs::remove_file(expected).unwrap();
    }
    let authoring = ctx.with_implicit_bases(ImplicitBases::Authoring);
    assert_eq!(reference("feat/x/spec.md").resolve_in_context(&authoring).unwrap(), Some(author.join("feat/x/spec.md")));
}

#[test]
fn completion_glob_and_recursive_search_share_launch_roots() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let area = repo.join("area");
    let launch = area.join("launch");
    let ctx = context(repo, &launch).with_package_area(&area)
        .for_cwd(repo.join("prompt"));
    write(&launch, "feat/x/spec.md");
    write(&area, "feat/x/spec.md");
    let glob = GlobReference::new(["feat/x/*.md"]).unwrap();
    let expected = [&launch, &area, repo].map(|base| base.join("feat/x"));
    assert_eq!(FileReference::complete_partial_in_context("feat/x/sp", &ctx).unwrap().unwrap().roots(), expected);
    assert_eq!(glob.take_first(&ctx).unwrap(), Some(launch.join("feat/x/spec.md")));
    assert_eq!(reference("%feat/x/spec.md").resolve_in_context(&ctx).unwrap(), Some(launch.join("feat/x/spec.md")));
}

#[test]
fn launch_and_source_keep_their_own_tree_boundaries() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path().join("repo");
    let launch = repo.join("launch");
    let external = fixture.path().join("external");
    write(&launch, "spec.md");
    write(&external, "spec.md");
    write(fixture.path(), "outside.md");
    let launch_context = context(&repo, &launch);
    let ctx = launch_context.for_trusted_external_cwd(&external)
        .with_repository_root(&external)
        .with_launch_scope(launch_context.launch_scope().clone());
    assert_eq!(ctx.implicit_bases(), ImplicitBases::Launch);
    assert_eq!(reference("spec.md").resolve_in_context(&ctx).unwrap(), Some(launch.join("spec.md")));
    assert_eq!(reference("./spec.md").resolve_in_context(&ctx).unwrap(), Some(external.join("spec.md")));
    assert!(matches!(
        reference("../outside.md").resolve_in_context(&ctx),
        Err(FileReferenceError::RelativeTreeEscape { base_dir, .. }) if base_dir == external
    ));
    assert!(matches!(
        reference("folder/../../outside.md").resolve_in_context(&ctx),
        Err(FileReferenceError::RelativeTreeEscape { base_dir, .. }) if base_dir == repo
    ));
}

#[test]
#[cfg(unix)]
fn a_launch_symlink_escape_stops_before_the_repository_match() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path().join("repo");
    let launch = repo.join("launch");
    let outside = fixture.path().join("outside");
    write(&outside, "spec.md");
    write(&repo, "feat/x/spec.md");
    fs::create_dir_all(launch.join("feat")).unwrap();
    std::os::unix::fs::symlink(&outside, launch.join("feat/x")).unwrap();
    let ctx = context(&repo, &launch);
    assert!(matches!(
        reference("feat/x/spec.md").resolve_in_context(&ctx),
        Err(FileReferenceError::RelativeTreeEscape { base_dir, .. }) if base_dir == repo
    ));
}

#[test]
fn launch_lookup_preserves_host_miss_and_io_semantics() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let launch = repo.join("launch");
    write(repo, "feat/x/spec.md");
    let ctx = context(repo, &launch);
    let spec = reference("feat/x/spec.md");
    assert_eq!(spec.resolve_in_context(&ctx).unwrap(), Some(repo.join("feat/x/spec.md")));
    write(&launch, "feat");
    #[cfg(unix)]
    assert!(matches!(
        spec.resolve_in_context(&ctx),
        Err(FileReferenceError::Io { path, .. }) if path.starts_with(&launch)
    ));
    // Win32 reports an absent path where Unix reports ENOTDIR.
    #[cfg(windows)]
    assert_eq!(spec.resolve_in_context(&ctx).unwrap(), Some(repo.join("feat/x/spec.md")));
}

#[test]
fn directory_probe_skips_a_nearer_file_and_reports_the_matching_base() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let launch = repo.join("launch");
    write(&launch, "feat");
    write(repo, "feat/x/spec.md");
    let ctx = context(repo, &launch);
    let input = reference("feat");
    assert_eq!(input.resolve_detailed(&ctx).matched_path(), Some(launch.join("feat").as_path()));
    assert_eq!(input.resolve_detailed_as(&ctx, EntryKind::Directory).matched_path(), Some(repo.join("feat").as_path()));
    let plan = input.plan_target(&ctx, EntryKind::Directory).unwrap();
    assert_eq!(plan.path, repo.join("feat"));
    assert_eq!(plan.provenance, RootProvenance::Repository);
    assert!(plan.existed);
}

#[test]
fn plan_uses_the_first_existing_parent_or_the_launch_directory() {
    let fixture = tempfile::tempdir().unwrap();
    let repo = fixture.path();
    let area = repo.join("darkmatter");
    write(repo, "darkmatter/features/x/spec.md");
    let ctx = context(repo, &area).with_package_area(&area);
    for (input, expected) in [
        ("darkmatter/features/x/review-3.md", "darkmatter/features/x/review-3.md"),
        ("features/new.md", "darkmatter/features/new.md"),
        ("missing/new.md", "darkmatter/missing/new.md"),
        ("./darkmatter/features/x/review-3.md", "darkmatter/darkmatter/features/x/review-3.md"),
    ] {
        let plan = reference(input).plan_target(&ctx, EntryKind::File).unwrap();
        assert_eq!(plan.path, repo.join(expected));
        assert!(!plan.existed);
    }
    assert!(matches!(reference("%spec.md").plan_target(&ctx, EntryKind::File),
        Err(FileReferenceError::InvalidSyntax(_))));
    let plan = reference("{{INPUT}}")
        .plan_target(&ctx.with_env(HashMap::from([("INPUT".into(), "darkmatter/features/x/spec.md".into())])), EntryKind::File)
        .unwrap();
    assert_eq!(plan.path, repo.join("darkmatter/features/x/spec.md"));
    assert!(plan.existed);
}
