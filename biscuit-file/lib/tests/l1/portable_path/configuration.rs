//! Builders, captured state, repository discovery, and the errors raised
//! before any preference runs.

use biscuit_file::{
    ConfigurationProblem, FilterProblem, InvalidTarget, PortabilityPreference as P, PortablePath,
    PortablePathError,
};

use super::*;

fn configuration_problem(error: PortablePathError) -> ConfigurationProblem {
    assert!(error.attempts().is_empty(), "no preference ran: {error}");
    match error {
        PortablePathError::InvalidConfiguration(problem) => problem,
        other => panic!("expected InvalidConfiguration, got {other}"),
    }
}

#[test]
fn a_context_cannot_be_combined_with_directory_builders_in_either_order() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&cwd, &[]);

    for portable in [
        PortablePath::from_path(&target).with_ctx(&ctx).with_cwd(&cwd),
        PortablePath::from_path(&target).with_cwd(&cwd).with_ctx(&ctx),
    ] {
        assert_eq!(configuration_problem(evaluate_err(portable)), ConfigurationProblem::ContextWithCwd);
    }
    for portable in [
        PortablePath::from_path(&target).with_ctx(&ctx).with_base_dir(&fx.repo),
        PortablePath::from_path(&target).with_base_dir(&fx.repo).with_ctx(&ctx),
    ] {
        assert_eq!(configuration_problem(evaluate_err(portable)), ConfigurationProblem::ContextWithBaseDir);
    }
    // Strategy and portable-name builders remain available with a context.
    let found = evaluate(
        PortablePath::from_path(&target)
            .with_ctx(&ctx)
            .with_strategy([P::SameDirRelative])
            .with_portable_env(["NOTES"]),
    );
    assert_eq!(found.reference().raw(), "./x.md");
}

#[test]
fn directories_and_path_targets_must_be_absolute() {
    let fx = Fixture::new();
    let target = fx.file("repo/x.md");
    assert_eq!(
        configuration_problem(evaluate_err(PortablePath::from_path(&target).with_cwd("relative/dir"))),
        ConfigurationProblem::RelativeDirectory {
            path: PathBuf::from("relative/dir")
        }
    );
    assert_eq!(
        configuration_problem(evaluate_err(
            PortablePath::from_path(&target).with_cwd(&fx.repo).with_base_dir("tree")
        )),
        ConfigurationProblem::RelativeDirectory { path: PathBuf::from("tree") }
    );

    #[cfg(not(windows))]
    let foreign = r"C:\docs\x.md";
    #[cfg(windows)]
    let foreign = "/docs/x.md";
    for (input, reason) in [("docs/x.md", InvalidTarget::Relative), (foreign, InvalidTarget::ForeignAbsolute)] {
        let error = evaluate_err(PortablePath::from_path(input).with_ctx(&fx.ctx(&fx.repo, &[])));
        assert!(
            matches!(&error, PortablePathError::InvalidTarget { target, reason: actual }
                if target == Path::new(input) && *actual == reason),
            "{input}: {error}"
        );
        assert!(error.attempts().is_empty());
    }
}

#[test]
fn an_invalid_context_fails_before_any_preference_runs() {
    let fx = Fixture::new();
    let elsewhere = fx.dir("elsewhere");
    let target = fx.file("repo/x.md");
    let ctx = fx.ctx(&elsewhere, &[]);
    assert_eq!(
        configuration_problem(evaluate_err(PortablePath::from_path(&target).with_ctx(&ctx))),
        ConfigurationProblem::CwdOutsideBaseDir {
            base_dir: fx.repo.clone(),
            cwd: elsewhere.clone(),
        },
        "even though AbsolutePath could otherwise succeed"
    );

    let tree = fx.dir("tree");
    assert_eq!(
        configuration_problem(evaluate_err(
            PortablePath::from_path(&target).with_cwd(&elsewhere).with_base_dir(&tree)
        )),
        ConfigurationProblem::CwdOutsideBaseDir {
            base_dir: tree,
            cwd: elsewhere,
        }
    );
}

#[test]
fn without_a_context_the_repository_is_discovered_from_cwd() {
    let fx = Fixture::new();
    let checkout = fx.dir("checkout");
    gix::init(&checkout).unwrap();
    let topics = fx.dir("checkout/docs/topics");
    let source = fx.file("checkout/src/x.md");

    let found = evaluate(PortablePath::from_path(&source).with_cwd(&topics));
    assert_eq!(found.reference().raw(), "&src/x.md");
    assert_eq!(found.strategy(), &P::RepoRoot(None));

    let found = evaluate(PortablePath::from_path(&source).with_cwd(&topics).with_base_dir(&checkout));
    assert_eq!(found.reference().raw(), "&src/x.md", "a base_dir equal to the repository root");

    let error = evaluate_err(
        PortablePath::from_path(&source)
            .with_cwd(&topics)
            .with_base_dir(checkout.join("docs")),
    );
    assert!(
        matches!(
            configuration_problem(error),
            ConfigurationProblem::BaseDirNotRepositoryRoot { .. }
        ),
        "inside a repository, base_dir must be its root"
    );
}

#[test]
fn invalid_filter_syntax_is_a_configuration_error() {
    let fx = Fixture::new();
    let target = fx.file("repo/x.md");
    let ctx = fx.ctx(&fx.repo, &[]);
    for (strategy, problem) in [
        (P::RepoRoot(Some(String::new())), FilterProblem::Empty),
        (P::RepoRoot(Some("../docs".into())), FilterProblem::NotARelativeSubdirectory),
        (P::RepoMultiPath(Some(absolute_text(&fx.repo))), FilterProblem::NotARelativeSubdirectory),
        (P::RepoMultiPath(Some("a/../..".into())), FilterProblem::NotARelativeSubdirectory),
        (P::MagicPath(Some(String::new())), FilterProblem::Empty),
    ] {
        let error = evaluate_err(
            PortablePath::from_path(&target)
                .with_ctx(&ctx)
                .with_strategy([strategy.clone(), P::AbsolutePath]),
        );
        assert_eq!(
            configuration_problem(error),
            ConfigurationProblem::InvalidFilter { strategy, problem }
        );
    }
    for filter in ["%@prompts", "https://example.com/prompts", "{{lower}}/prompts"] {
        let strategy = P::MagicPath(Some(filter.into()));
        let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([strategy]));
        assert!(
            matches!(
                configuration_problem(error),
                ConfigurationProblem::InvalidFilter {
                    problem: FilterProblem::InvalidReference { .. },
                    ..
                }
            ),
            "{filter}"
        );
    }
}
