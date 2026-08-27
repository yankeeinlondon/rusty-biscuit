use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{
    CompletionEntryForm, FileReference, FileReferenceError, FileResolutionContext,
    PackageAreaFallback, PathPosition, RepositoryScopeCatalog, RootProvenance,
};
use tempfile::TempDir;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn scoped_context(
    repo: &Path,
    base: &Path,
    area: &Path,
    package: &Path,
    home: &Path,
) -> FileResolutionContext {
    let catalog = RepositoryScopeCatalog::new(
        repo,
        vec![area.to_path_buf()],
        vec![package.to_path_buf()],
        PackageAreaFallback::FirstComponent,
    )
    .unwrap();
    FileResolutionContext::new(base)
        .with_repository_scope_catalog(catalog)
        .with_home_dir(home)
}

fn paths(plan: &[biscuit_file::ResolutionCandidate]) -> Vec<PathBuf> {
    plan.iter().map(|candidate| candidate.path().to_path_buf()).collect()
}

#[test]
fn implicit_relative_is_cwd_first_and_first_match_wins() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let base = repo.join("docs");
    write(&repo.join("shared.md"), "repository");
    write(&base.join("shared.md"), "composition cwd");
    let ctx = FileResolutionContext::new(&base).with_repository_root(&repo);

    let reference = FileReference::new("shared.md").unwrap();
    let plan = reference.candidate_plan(&ctx).unwrap();
    assert_eq!(paths(&plan), vec![base.join("shared.md"), repo.join("shared.md")]);
    assert_eq!(plan[0].provenance(), RootProvenance::Source);
    assert_eq!(
        reference.resolve_in_context(&ctx).unwrap(),
        Some(base.join("shared.md"))
    );
}

#[test]
fn repository_root_reference_has_one_candidate_match_and_miss() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let base = repo.join("package/src");
    write(&repo.join("README.md"), "root");
    let ctx = FileResolutionContext::new(&base).with_repository_root(&repo);

    let matched = FileReference::new("&README.md").unwrap();
    let plan = matched.candidate_plan(&ctx).unwrap();
    assert_eq!(paths(&plan), vec![repo.join("README.md")]);
    assert_eq!(plan[0].provenance(), RootProvenance::Repository);
    assert_eq!(matched.resolve_in_context(&ctx).unwrap(), Some(repo.join("README.md")));

    assert_eq!(
        FileReference::new("&missing.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap(),
        None
    );
}

#[test]
fn repository_scoped_reference_orders_package_area_and_repository() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let area = repo.join("claudine");
    let package = area.join("lib");
    let base = package.join("src");
    let home = temp.path().join("home");
    write(&package.join("config/value.md"), "package");
    write(&area.join("config/value.md"), "area");
    write(&repo.join("config/value.md"), "repository");
    let ctx = scoped_context(&repo, &base, &area, &package, &home);

    let reference = FileReference::new("^config/value.md").unwrap();
    let plan = reference.candidate_plan(&ctx).unwrap();
    assert_eq!(
        paths(&plan),
        vec![
            package.join("config/value.md"),
            area.join("config/value.md"),
            repo.join("config/value.md"),
        ]
    );
    assert_eq!(plan[0].provenance(), RootProvenance::PackageRoot);
    assert_eq!(plan[1].provenance(), RootProvenance::PackageArea);
    assert_eq!(reference.resolve_in_context(&ctx).unwrap(), Some(package.join("config/value.md")));

    fs::remove_file(package.join("config/value.md")).unwrap();
    assert_eq!(reference.resolve_in_context(&ctx).unwrap(), Some(area.join("config/value.md")));

    assert_eq!(
        FileReference::new("^config/missing.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap(),
        None
    );
}

#[test]
fn magic_intrinsic_chain_is_between_registered_roots() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let area = repo.join("claudine");
    let package = area.join("lib");
    let base = package.join("src");
    let home = temp.path().join("home");
    let prepend = temp.path().join("prepend");
    let append = temp.path().join("append");
    let ctx = scoped_context(&repo, &base, &area, &package, &home)
        .add_magic_path(&prepend, PathPosition::Start)
        .add_magic_path(&append, PathPosition::End);

    let plan = FileReference::new("@config.md").unwrap().candidate_plan(&ctx).unwrap();
    assert_eq!(
        paths(&plan),
        vec![
            prepend.join("config.md"),
            package.join("config.md"),
            area.join("config.md"),
            repo.join("config.md"),
            home.join("config.md"),
            append.join("config.md"),
        ]
    );
}

#[test]
fn repository_sigils_outside_a_repository_are_typed_errors() {
    let outside = TempDir::new().unwrap();
    let ctx = FileResolutionContext::new(outside.path());

    for (raw, sigil) in [("&README.md", '&'), ("^README.md", '^')] {
        let error = FileReference::new(raw)
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap_err();
        assert!(matches!(
            error,
            FileReferenceError::OutsideRepository {
                sigil: actual,
                reference_cwd
            } if actual == sigil && reference_cwd == outside.path()
        ));
    }
}

#[test]
fn repository_sigils_reject_lexical_escapes_for_direct_recursive_and_completion() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir_all(repo.join("docs")).unwrap();
    let ctx = FileResolutionContext::new(repo.join("docs")).with_repository_root(&repo);

    for raw in ["&../outside.md", "^../outside.md", "%&../outside.md", "%^../outside.md"] {
        let error = FileReference::new(raw)
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap_err();
        assert!(matches!(error, FileReferenceError::RepositoryEscape { reference, .. } if reference == raw));
    }

    for token in ["&../out", "^../out"] {
        assert!(matches!(
            FileReference::complete_partial_in_context(token, &ctx),
            Err(FileReferenceError::RepositoryEscape { .. })
        ));
    }
}

#[cfg(unix)]
#[test]
fn canonical_containment_accepts_internal_links_and_rejects_external_targets() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let outside = temp.path().join("outside");
    write(&repo.join("actual/internal.md"), "inside");
    write(&outside.join("external.md"), "outside");
    symlink(repo.join("actual/internal.md"), repo.join("internal-link.md")).unwrap();
    symlink(outside.join("external.md"), repo.join("external-link.md")).unwrap();
    symlink(&outside, repo.join("external-dir")).unwrap();
    let ctx = FileResolutionContext::new(&repo).with_repository_root(&repo);

    assert_eq!(
        FileReference::new("&internal-link.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap(),
        Some(repo.join("internal-link.md"))
    );
    assert!(matches!(
        FileReference::new("&external-link.md")
            .unwrap()
            .resolve_in_context(&ctx),
        Err(FileReferenceError::RepositoryEscape { .. })
    ));
    assert!(matches!(
        FileReference::new("&external-dir/missing/child.md")
            .unwrap()
            .validate_repository_candidate(
                &repo.join("external-dir/missing/child.md"),
                &repo,
            ),
        Err(FileReferenceError::RepositoryEscape { .. })
    ));
    assert!(matches!(
        FileReference::complete_partial_in_context("&external-dir/mi", &ctx),
        Err(FileReferenceError::RepositoryEscape { .. })
    ));
    assert!(matches!(
        FileReference::new("%^external-dir/missing.md")
            .unwrap()
            .resolve_in_context(&ctx),
        Err(FileReferenceError::RepositoryEscape { .. })
    ));
}

#[test]
fn completion_roots_match_execution_order_for_supported_forms() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let area = repo.join("claudine");
    let package = area.join("lib");
    let base = package.join("src");
    let home = temp.path().join("home");
    fs::create_dir_all(&base).unwrap();
    let ctx = scoped_context(&repo, &base, &area, &package, &home);

    let cases = [
        ("docs/pl", "docs/plan.md", CompletionEntryForm::ImplicitRelative),
        ("@docs/pl", "@docs/plan.md", CompletionEntryForm::Magic),
        ("&docs/pl", "&docs/plan.md", CompletionEntryForm::RepositoryRoot),
        ("^docs/pl", "^docs/plan.md", CompletionEntryForm::RepositoryScoped),
    ];
    for (token, executable, form) in cases {
        let completion = FileReference::complete_partial_in_context(token, &ctx)
            .unwrap()
            .unwrap();
        let plan = FileReference::new(executable).unwrap().candidate_plan(&ctx).unwrap();
        let expected: Vec<PathBuf> = plan
            .iter()
            .map(|candidate| candidate.path().parent().unwrap().to_path_buf())
            .collect();
        assert_eq!(completion.entry_form(), form);
        assert_eq!(completion.roots(), expected);
    }
}

#[test]
fn recursive_completion_stays_unsupported_but_malformed_payloads_still_error() {
    let temp = TempDir::new().unwrap();
    let ctx = FileResolutionContext::new(temp.path()).with_repository_root(temp.path());

    for token in ["%@docs/pl", "%&docs/pl", "%^docs/pl"] {
        assert!(
            FileReference::complete_partial_in_context(token, &ctx)
                .unwrap()
                .is_none()
        );
    }
    for token in ["%@//etc/passwd", "%&//etc/passwd", "%^//etc/passwd"] {
        assert!(matches!(
            FileReference::complete_partial_in_context(token, &ctx),
            Err(FileReferenceError::InvalidSyntax(_))
        ));
    }
}

#[cfg(windows)]
#[test]
fn repository_containment_rejects_an_external_junction() {
    use std::process::Command;

    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let outside = temp.path().join("outside");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let junction = repo.join("external-junction");
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside)
        .status()
        .unwrap();
    assert!(status.success());
    let ctx = FileResolutionContext::new(&repo).with_repository_root(&repo);

    assert!(matches!(
        FileReference::new("&external-junction/missing.md")
            .unwrap()
            .validate_repository_candidate(&junction.join("missing.md"), &repo),
        Err(FileReferenceError::RepositoryEscape { .. })
    ));
}
