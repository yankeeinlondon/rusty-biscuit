//! Validation covers every captured directory and survives derivation.
//!
//! A replacement launch `@` scope carries its own request directory, which
//! must be absolute like every other anchor. Deriving a document context
//! keeps the validation failure of the context it came from, even when the
//! derivation clears the explicit tree root and, without a catalog, the
//! repository and package anchors (trusted external), or reselects those
//! anchors from a scope catalog.
//!
//! Every context is checked through the same public projections: both
//! `PortablePath` writers and every context-aware `FileReference` reader.
//! Contexts with relative directories are built with `from_snapshot`
//! directly, so no test creates a directory under the process working
//! directory.

use biscuit_file::{
    CandidatePlanOrder, ConfigurationProblem, ContextAnchor, DetailedOutcome, FileReferenceError,
    LaunchMagicScope, PackageAreaFallback, PathPosition, PortabilityPreference as P,
    RepositoryScopeCatalog,
};

use super::*;

/// The configuration failure a context must report.
#[derive(Debug, Clone, PartialEq)]
enum Expected {
    Relative(ContextAnchor, &'static str),
    Conflict { base_dir: PathBuf, repository_root: PathBuf },
}

impl Expected {
    fn matches(&self, error: &FileReferenceError) -> bool {
        match (self, error) {
            (Self::Relative(anchor, path), FileReferenceError::RelativeContextDirectory { anchor: a, path: p }) => {
                a == anchor && p == Path::new(path)
            }
            (
                Self::Conflict { base_dir, repository_root },
                FileReferenceError::BaseDirNotRepositoryRoot { base_dir: b, repository_root: r },
            ) => b == base_dir && r == repository_root,
            _ => false,
        }
    }

    fn problem(&self) -> ConfigurationProblem {
        match self {
            Self::Relative(_, path) => ConfigurationProblem::RelativeDirectory { path: PathBuf::from(path) },
            Self::Conflict { base_dir, repository_root } => ConfigurationProblem::BaseDirNotRepositoryRoot {
                base_dir: base_dir.clone(),
                repository_root: repository_root.clone(),
            },
        }
    }
}

fn assert_error(case: &str, projection: &str, expected: &Expected, result: Result<impl std::fmt::Debug, FileReferenceError>) {
    match result {
        Err(error) if expected.matches(&error) => {}
        other => panic!("{case}: {projection}: expected {expected:?}, got {other:?}"),
    }
}

/// Assert `ctx` is rejected with `expected` by every public projection,
/// before any strategy runs or any candidate is probed.
fn assert_rejected(case: &str, fx: &Fixture, ctx: &FileResolutionContext, expected: &Expected) {
    let target = fx.path("repo/docs/x.md");
    assert_error(case, "validate", expected, ctx.validate());

    for (projection, portable) in [
        ("from_path", PortablePath::from_path(&target)),
        ("from_reference", PortablePath::from_reference(reference(&absolute_text(&target)))),
    ] {
        let error = evaluate_err(portable.with_ctx(ctx).with_strategy([P::AbsolutePath]));
        assert!(error.attempts().is_empty(), "{case}: {projection}: no preference ran: {error}");
        match error {
            PortablePathError::InvalidConfiguration(problem) => {
                assert_eq!(problem, expected.problem(), "{case}: {projection}");
            }
            other => panic!("{case}: {projection}: expected InvalidConfiguration, got {other}"),
        }
    }

    let absolute = reference(&absolute_text(&target));
    assert_error(case, "resolve_in_context", expected, absolute.resolve_in_context(ctx));
    let detailed = absolute.resolve_detailed(ctx);
    assert!(matches!(detailed.outcome(), DetailedOutcome::Failed(_)), "{case}: resolve_detailed: {:?}", detailed.outcome());
    assert!(
        detailed.error().is_some_and(|error| expected.matches(error)),
        "{case}: resolve_detailed: expected {expected:?}, got {:?}",
        detailed.error()
    );
    assert_error(case, "candidate_plan", expected, absolute.candidate_plan(ctx));
    for order in [CandidatePlanOrder::Resolution, CandidatePlanOrder::AuthoringBaseFirst] {
        assert_error(case, "candidate_plan_with_order", expected, absolute.candidate_plan_with_order(ctx, order));
    }
    for token in ["x", "@x"] {
        assert_error(case, "complete_partial_in_context", expected, FileReference::complete_partial_in_context(token, ctx));
    }
    for raw in ["@x.md", "%@x.md"] {
        assert_error(case, raw, expected, reference(raw).resolve_in_context(ctx));
    }
}

/// Assert `ctx` validates and both writers and the reader reach the target.
fn assert_accepted(case: &str, fx: &Fixture, ctx: &FileResolutionContext) {
    let target = fx.path("repo/docs/x.md");
    if let Err(error) = ctx.validate() {
        panic!("{case}: expected a valid context, got {error}");
    }
    for portable in [
        PortablePath::from_path(&target),
        PortablePath::from_reference(reference(&absolute_text(&target))),
    ] {
        let found = evaluate(portable.with_ctx(ctx).with_strategy([P::AbsolutePath]));
        assert_eq!(found.reference().raw(), absolute_text(&target), "{case}");
    }
    let resolved = reference(&absolute_text(&target)).resolve_in_context(ctx).unwrap().unwrap();
    assert!(same_file(&resolved, &target), "{case}: {}", resolved.display());
}

fn snapshot(fx: &Fixture, cwd: impl Into<PathBuf>) -> FileResolutionContext {
    FileResolutionContext::from_snapshot(cwd, Some(fx.home.clone()), HashMap::new())
}

/// The target, a document beside it, and a document outside the repository.
fn setup() -> Fixture {
    let fx = Fixture::new();
    fx.file("repo/docs/x.md");
    fx.file("repo/docs/y.md");
    fx.file("other/x.md");
    fx
}

/// A launch scope whose own anchors are all absolute.
fn valid_launch_scope(fx: &Fixture) -> LaunchMagicScope {
    fx.ctx(&fx.path("repo/docs"), &[]).launch_magic_scope().clone()
}

/// Every derivation wrapper to an in-repository document, and the trusted
/// wrappers (plus the ordinary ones, whose escape is reported after the
/// retained failure) to an external document.
fn derivations(fx: &Fixture, origin: &FileResolutionContext) -> Vec<(String, FileResolutionContext)> {
    let opening = reference("x.md");
    let mut derived = Vec::new();
    for (place, file) in [("in-tree", fx.path("repo/docs/y.md")), ("external", fx.path("other/x.md"))] {
        let dir = file.parent().unwrap().to_path_buf();
        derived.extend([
            (format!("for_cwd {place}"), origin.for_cwd(&dir)),
            (format!("for_source {place}"), origin.for_source(&file)),
            (format!("for_source_reference {place}"), origin.for_source_reference(&opening, &file)),
            (format!("for_trusted_external_cwd {place}"), origin.for_trusted_external_cwd(&dir)),
            (format!("for_trusted_external_source {place}"), origin.for_trusted_external_source(&file)),
            (
                format!("for_trusted_external_source_reference {place}"),
                origin.for_trusted_external_source_reference(&opening, &file),
            ),
        ]);
    }
    derived
}

fn catalog(fx: &Fixture) -> RepositoryScopeCatalog {
    RepositoryScopeCatalog::new(
        &fx.repo,
        vec![fx.path("repo/docs")],
        vec![fx.path("repo/docs")],
        PackageAreaFallback::None,
    )
    .unwrap()
}

// --- Replacement launch scope ----------------------------------------------

#[test]
fn a_replacement_launch_scope_is_validated_like_every_other_anchor() {
    let fx = setup();
    let docs = fx.path("repo/docs");
    let base = fx.ctx(&docs, &[]);
    let cases = [
        ("relative launch request directory", snapshot(&fx, "relative"), Expected::Relative(ContextAnchor::RequestDirectory, "relative")),
        (
            "relative launch repository root",
            snapshot(&fx, &docs).with_repository_root("relative"),
            Expected::Relative(ContextAnchor::RepositoryRoot, "relative"),
        ),
        (
            "relative launch package root",
            fx.ctx(&docs, &[]).with_package_root("pkg"),
            Expected::Relative(ContextAnchor::PackageRoot, "pkg"),
        ),
        (
            "relative launch package area",
            fx.ctx(&docs, &[]).with_package_area("area"),
            Expected::Relative(ContextAnchor::PackageArea, "area"),
        ),
    ];
    for (case, launch, expected) in cases {
        let ctx = base.clone().with_launch_magic_scope(launch.launch_magic_scope().clone());
        assert_rejected(case, &fx, &ctx, &expected);
    }
    assert_accepted("valid replacement launch scope", &fx, &base.with_launch_magic_scope(valid_launch_scope(&fx)));
}

// --- Invalid explicit tree root --------------------------------------------

#[test]
fn every_derivation_keeps_an_invalid_explicit_root() {
    let fx = setup();
    let docs = fx.path("repo/docs");
    let cases = [
        ("relative explicit root", fx.ctx(&docs, &[]).with_base_dir("relative"), Expected::Relative(ContextAnchor::BaseDir, "relative")),
        (
            "explicit root conflicting with the repository root",
            fx.ctx(&docs, &[]).with_base_dir(&docs),
            Expected::Conflict { base_dir: docs.clone(), repository_root: fx.repo.clone() },
        ),
    ];
    for (case, origin, expected) in cases {
        assert_rejected(&format!("{case}: origin"), &fx, &origin, &expected);
        for (wrapper, derived) in derivations(&fx, &origin) {
            assert_rejected(&format!("{case}: {wrapper}"), &fx, &derived, &expected);
        }
    }
}

// --- Invalid repository and package anchors replaced by derivation ---------

#[test]
fn every_derivation_keeps_a_relative_repository_or_package_anchor() {
    let fx = setup();
    let docs = fx.path("repo/docs");
    type Edit = fn(FileResolutionContext) -> FileResolutionContext;
    // A trusted external derivation without a catalog clears all three
    // anchors, and catalog reselection overwrites them, so only the retained
    // failure still reports them.
    let edits: [(&str, Edit, Expected); 3] = [
        (
            "repository root",
            |ctx| ctx.with_repository_root("relative"),
            Expected::Relative(ContextAnchor::RepositoryRoot, "relative"),
        ),
        ("package root", |ctx| ctx.with_package_root("pkg"), Expected::Relative(ContextAnchor::PackageRoot, "pkg")),
        ("package area", |ctx| ctx.with_package_area("area"), Expected::Relative(ContextAnchor::PackageArea, "area")),
    ];
    for (anchor, edit, expected) in edits {
        // The relative anchor also reaches the launch scope; replacing that
        // scope leaves the current anchor as the only invalid setting.
        let origins = [
            ("catalog", fx.bare_ctx(&docs, &[]).with_repository_scope_catalog(catalog(&fx))),
            ("no catalog", fx.ctx(&docs, &[])),
        ];
        for (source, origin) in origins {
            let origin = edit(origin).with_launch_magic_scope(valid_launch_scope(&fx));
            let case = format!("relative {anchor}, {source}");
            assert_rejected(&format!("{case}: origin"), &fx, &origin, &expected);
            for (wrapper, derived) in derivations(&fx, &origin) {
                assert_rejected(&format!("{case}: {wrapper}"), &fx, &derived, &expected);
            }
        }
    }
}

// --- Windows rooted paths without a drive ----------------------------------

/// `\repo` has a root but no drive, so it is relative to the current drive
/// and must be rejected like any other relative directory.
#[cfg(windows)]
#[test]
fn every_derivation_keeps_a_drive_relative_rooted_explicit_root() {
    let fx = setup();
    let origin = fx.ctx(&fx.path("repo/docs"), &[]).with_base_dir(r"\repo");
    let expected = Expected::Relative(ContextAnchor::BaseDir, r"\repo");
    assert_rejected("rooted explicit root: origin", &fx, &origin, &expected);
    for (wrapper, derived) in derivations(&fx, &origin) {
        assert_rejected(&format!("rooted explicit root: {wrapper}"), &fx, &derived, &expected);
    }
}

// --- Builders and derivation order -----------------------------------------

#[test]
fn a_builder_before_derivation_corrects_a_setting_but_one_after_does_not() {
    let fx = setup();
    let docs = fx.path("repo/docs");
    let corrected = fx.ctx(&docs, &[]).with_base_dir("relative").with_base_dir(&fx.repo);
    for (wrapper, derived) in derivations(&fx, &corrected) {
        if wrapper.ends_with("in-tree") || wrapper.contains("trusted") {
            assert_accepted(&wrapper, &fx, &derived);
        }
    }

    // The retained failure describes the originating request, which no
    // later builder on the derived context can change.
    let derived = fx
        .ctx(&docs, &[])
        .with_base_dir("relative")
        .for_trusted_external_cwd(fx.path("other"))
        .with_base_dir(fx.path("other"));
    assert_rejected("builder after derivation", &fx, &derived, &Expected::Relative(ContextAnchor::BaseDir, "relative"));
    // A further derivation keeps it too.
    let nested = derived.for_trusted_external_cwd(&docs);
    assert_rejected("nested derivation", &fx, &nested, &Expected::Relative(ContextAnchor::BaseDir, "relative"));
}

// --- Clean controls --------------------------------------------------------

#[test]
fn valid_contexts_stay_valid_through_every_accepted_derivation() {
    let fx = setup();
    let docs = fx.path("repo/docs");
    let origins = [
        ("unedited", fx.ctx(&docs, &[])),
        ("catalog", fx.bare_ctx(&docs, &[]).with_repository_scope_catalog(catalog(&fx))),
        (
            "relative environment, magic, and vault values",
            fx.ctx(&docs, &[("ROOT", "docs"), ("VAULT", "vault")])
                .add_magic_path("prompts", PathPosition::Start)
                .add_vault("vault"),
        ),
    ];
    for (case, origin) in origins {
        assert_accepted(&format!("{case}: origin"), &fx, &origin);
        for (wrapper, derived) in derivations(&fx, &origin) {
            // An ordinary derivation to an external document is an escape,
            // reported as one; only accepted destinations are controls here.
            if wrapper.ends_with("in-tree") || wrapper.contains("trusted") {
                assert_accepted(&format!("{case}: {wrapper}"), &fx, &derived);
            }
        }
    }
}

// --- Process-directory independence ----------------------------------------

mod process_directory {
    //! These tests change the process working directory, so they run
    //! serially.

    use std::env;

    use serial_test::serial;

    use super::*;

    /// Restores the original working directory on drop.
    struct CwdGuard {
        original: PathBuf,
    }

    impl CwdGuard {
        fn set(dir: &Path) -> Self {
            let original = env::current_dir().expect("current_dir");
            env::set_current_dir(dir).expect("set_current_dir");
            Self { original }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

    #[test]
    #[serial]
    fn a_relative_launch_directory_is_rejected_from_any_process_directory() {
        let fx = setup();
        let first = fx.file("first/relative/x.md").parent().unwrap().parent().unwrap().to_path_buf();
        let second = fx.dir("second");
        let ctx = fx
            .ctx(&fx.path("repo/docs"), &[])
            .with_launch_magic_scope(snapshot(&fx, "relative").launch_magic_scope().clone());
        let expected = Expected::Relative(ContextAnchor::RequestDirectory, "relative");
        for dir in [&first, &second] {
            let _guard = CwdGuard::set(dir);
            assert_error(&dir.display().to_string(), "@x.md", &expected, reference("@x.md").resolve_in_context(&ctx));
        }
    }
}
