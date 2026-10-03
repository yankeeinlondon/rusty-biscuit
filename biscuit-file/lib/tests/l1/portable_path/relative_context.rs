//! A context directory or tree anchor that is relative fails validation
//! before any preference runs, however the context was built or derived.
//!
//! Every writer here uses only `AbsolutePath`, which would otherwise succeed
//! for the absolute target. Contexts with relative directories are built with
//! `from_snapshot` directly, never through `Fixture::bare_ctx`, so no test
//! creates a directory under the process working directory.

use biscuit_file::{
    ConfigurationProblem, ContextAnchor, FileReferenceError, PathPosition,
    PortabilityPreference as P, PortablePath, PortablePathError,
};

use super::*;

/// The relative directory the absolute-only writer rejects for `ctx`.
fn writer_rejects(target: &Path, ctx: &FileResolutionContext) -> PathBuf {
    let error = evaluate_err(
        PortablePath::from_path(target)
            .with_ctx(ctx)
            .with_strategy([P::AbsolutePath]),
    );
    assert!(error.attempts().is_empty(), "no preference ran: {error}");
    match error {
        PortablePathError::InvalidConfiguration(ConfigurationProblem::RelativeDirectory { path }) => path,
        other => panic!("expected InvalidConfiguration(RelativeDirectory), got {other}"),
    }
}

/// The anchor and path `validate` reports, and that the writer reports the
/// same path.
fn rejected(target: &Path, ctx: &FileResolutionContext) -> (ContextAnchor, PathBuf) {
    let (anchor, path) = match ctx.validate() {
        Err(FileReferenceError::RelativeContextDirectory { anchor, path }) => (anchor, path),
        other => panic!("expected RelativeContextDirectory, got {other:?}"),
    };
    assert_eq!(writer_rejects(target, ctx), path);
    (anchor, path)
}

fn snapshot(fx: &Fixture, cwd: impl Into<PathBuf>) -> FileResolutionContext {
    FileResolutionContext::from_snapshot(cwd, Some(fx.home.clone()), HashMap::new())
}

#[test]
fn from_snapshot_with_a_relative_cwd_is_rejected() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    assert_eq!(
        rejected(&target, &snapshot(&fx, "relative")),
        (ContextAnchor::RequestDirectory, PathBuf::from("relative"))
    );
}

#[test]
fn new_with_a_relative_cwd_is_rejected() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    let ctx = FileResolutionContext::new("relative").with_home_dir(&fx.home);
    assert_eq!(
        rejected(&target, &ctx),
        (ContextAnchor::RequestDirectory, PathBuf::from("relative"))
    );
}

#[test]
fn an_explicit_tree_with_relative_directories_is_rejected() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    // Containment holds for the relative pair, so only the absolute check
    // can reject it.
    let paired = snapshot(&fx, "relative/docs").with_base_dir("relative");
    assert_eq!(
        rejected(&target, &paired),
        (ContextAnchor::RequestDirectory, PathBuf::from("relative/docs"))
    );
    let root_only = snapshot(&fx, fx.dir("repo/docs")).with_base_dir("relative");
    assert_eq!(
        rejected(&target, &root_only),
        (ContextAnchor::BaseDir, PathBuf::from("relative"))
    );
}

#[test]
fn a_repository_tree_with_relative_directories_is_rejected() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    let paired = snapshot(&fx, "relative/docs").with_repository_root("relative");
    assert_eq!(
        rejected(&target, &paired),
        (ContextAnchor::RequestDirectory, PathBuf::from("relative/docs"))
    );
    let root_only = snapshot(&fx, fx.dir("repo/docs")).with_repository_root("relative");
    assert_eq!(
        rejected(&target, &root_only),
        (ContextAnchor::RepositoryRoot, PathBuf::from("relative"))
    );
}

#[test]
fn relative_package_anchors_are_rejected() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    let base = fx.ctx(&fx.dir("repo/docs"), &[]);
    assert_eq!(
        rejected(&target, &base.clone().with_package_root("pkg")),
        (ContextAnchor::PackageRoot, PathBuf::from("pkg"))
    );
    assert_eq!(
        rejected(&target, &base.with_package_area("area")),
        (ContextAnchor::PackageArea, PathBuf::from("area"))
    );
}

#[test]
fn a_relative_captured_home_is_rejected() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    let ctx = FileResolutionContext::from_snapshot(fx.dir("repo/docs"), Some(PathBuf::from("home")), HashMap::new())
        .with_repository_root(&fx.repo);
    assert_eq!(rejected(&target, &ctx), (ContextAnchor::HomeDir, PathBuf::from("home")));
}

#[test]
fn normal_derivations_to_a_relative_directory_are_rejected() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    let opening = reference("./relative/x.md");
    // A fallback tree selects a new tree for the derived `cwd`, and a
    // repository tree keeps its own; both must reject a relative `cwd`.
    for origin in [snapshot(&fx, fx.dir("other")), fx.ctx(&fx.dir("repo/docs"), &[])] {
        origin.validate().unwrap();
        for derived in [
            origin.for_cwd("relative"),
            origin.for_source("relative/x.md"),
            origin.for_source_reference(&opening, "relative/x.md"),
        ] {
            assert_eq!(
                rejected(&target, &derived),
                (ContextAnchor::WorkingDirectory, PathBuf::from("relative"))
            );
        }
    }
}

#[test]
fn trusted_derivations_to_a_relative_directory_are_rejected() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    let origin = fx.ctx(&fx.dir("repo/docs"), &[]);
    origin.validate().unwrap();
    let opening = reference("./relative/x.md");
    for derived in [
        origin.for_trusted_external_cwd("relative"),
        origin.for_trusted_external_source("relative/x.md"),
        origin.for_trusted_external_source_reference(&opening, "relative/x.md"),
    ] {
        assert!(derived.is_trusted_external_authoring_cwd());
        assert_eq!(
            rejected(&target, &derived),
            (ContextAnchor::WorkingDirectory, PathBuf::from("relative"))
        );
    }
}

#[test]
fn a_trusted_derivation_does_not_hide_a_relative_launch_repository() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    // Without a scope catalog the trusted derivation drops the current
    // repository anchor, but the launch `@` scope still carries it.
    let origin = snapshot(&fx, fx.dir("repo/docs")).with_repository_root("relative");
    let derived = origin.for_trusted_external_cwd(fx.dir("other"));
    assert_eq!(derived.repository_root(), None);
    assert_eq!(
        rejected(&target, &derived),
        (ContextAnchor::RepositoryRoot, PathBuf::from("relative"))
    );
}

#[test]
fn resolver_entry_points_reject_a_relative_context() {
    let fx = Fixture::new();
    fx.file("repo/docs/x.md");
    let ctx = snapshot(&fx, "relative");
    let x = reference("x.md");
    assert!(matches!(
        x.resolve_in_context(&ctx),
        Err(FileReferenceError::RelativeContextDirectory { .. })
    ));
    assert!(matches!(
        x.resolve_detailed(&ctx).error(),
        Some(FileReferenceError::RelativeContextDirectory { .. })
    ));
    assert!(matches!(
        x.candidate_plan(&ctx),
        Err(FileReferenceError::RelativeContextDirectory { .. })
    ));
    assert!(matches!(
        biscuit_file::FileReference::complete_partial_in_context("x", &ctx),
        Err(FileReferenceError::RelativeContextDirectory { .. })
    ));
}

// --- Controls: relative values with their own contracts --------------------

#[test]
fn relative_environment_values_remain_supported() {
    let fx = Fixture::new();
    let target = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&fx.repo, &[("ROOT", "docs"), ("VAULT", "vault")]);
    ctx.validate().unwrap();
    let found = evaluate(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::AbsolutePath]));
    assert_eq!(found.reference().raw(), absolute_text(&target));
    let resolved = reference("{{ROOT}}/x.md").resolve_in_context(&ctx).unwrap().unwrap();
    assert!(same_file(&resolved, &target), "{}", resolved.display());
}

#[test]
fn relative_configured_magic_and_vault_roots_remain_supported() {
    let fx = Fixture::new();
    let docs = fx.dir("repo/docs");
    let target = fx.file("repo/docs/prompts/x.md");
    let ctx = fx
        .ctx(&docs, &[])
        .add_magic_path("prompts", PathPosition::Start)
        .add_vault("vault");
    ctx.validate().unwrap();
    let found = evaluate(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::AbsolutePath]));
    assert_eq!(found.reference().raw(), absolute_text(&target));
    // The relative magic root follows the captured request directory.
    let resolved = reference("@x.md").resolve_in_context(&ctx).unwrap().unwrap();
    assert!(same_file(&resolved, &target), "{}", resolved.display());
}
