//! The strategy matrix: each preference's meaning, default and reordered
//! strategies, filters, shadowing, missing and non-file targets, and the
//! visible `AbsolutePath` fallback.

use biscuit_file::{
    AttemptOutcome, Finding, IntentForms, NotApplicable, PathPosition, PortabilityPreference as P,
    PortablePath, PortablePathError,
};

use super::*;

// ---- relative preferences ---------------------------------------------------

#[test]
fn each_relative_preference_writes_its_own_route_shape() {
    let fx = Fixture::new();
    let docs = fx.dir("repo/apps/web/docs");
    let ctx = fx.ctx(&docs, &[]);
    let cases = [
        ("repo/apps/web/docs/same.md", "./same.md", P::SameDirRelative),
        ("repo/apps/web/docs/sub/deep/child.md", "./sub/deep/child.md", P::ChildDir),
        ("repo/apps/web/src/lib.md", "../src/lib.md", P::PeerDir),
        ("repo/apps/web/README.md", "../README.md", P::ImmediateParentDir),
        ("repo/apps/api/x.md", "&apps/api/x.md", P::RepoRoot(None)),
        ("repo/foo.md", "&foo.md", P::RepoRoot(None)),
    ];
    for (rel, expected, strategy) in cases {
        let portable = evaluate(PortablePath::from_path(fx.file(rel)).with_ctx(&ctx));
        assert_eq!(portable.reference().raw(), expected, "{rel}");
        assert_eq!(portable.strategy(), &strategy, "{rel}");
        let last = portable.attempts().last().unwrap();
        assert_eq!(last.strategy, strategy, "the last attempt is the match");
        assert_eq!(last.matched().map(|r| r.raw()), Some(expected));
        assert!(portable.findings().is_empty(), "{rel}: {:?}", portable.findings());
    }
}

#[test]
fn a_relative_preference_refuses_other_route_shapes() {
    let fx = Fixture::new();
    let docs = fx.dir("repo/apps/web/docs");
    let ctx = fx.ctx(&docs, &[]);
    let child = fx.file("repo/apps/web/docs/sub/deep/child.md");
    let cases = [
        (P::SameDirRelative, 0, 3),
        (P::PeerDir, 0, 3),
        (P::ImmediateParentDir, 0, 3),
        (P::ParentDir, 0, 3),
    ];
    for (strategy, parent_hops, names) in cases {
        let error = evaluate_err(PortablePath::from_path(&child).with_ctx(&ctx).with_strategy([strategy.clone()]));
        assert!(matches!(error, PortablePathError::NoStrategyMatched { .. }), "{error}");
        assert_eq!(
            not_applicable(&error.attempts()[0]),
            &NotApplicable::RouteShape { parent_hops, names },
            "{strategy}"
        );
    }
}

#[test]
fn parent_dir_is_the_in_tree_catch_all_and_absent_from_the_default() {
    let fx = Fixture::new();
    let cwd = fx.dir("notes/a/b");
    let target = fx.file("notes/x.md");
    let peer = fx.file("notes/a/c/y.md");
    let ctx = fx.bare_ctx(&cwd, &[]).with_base_dir(fx.path("notes"));

    // Ruling R1: no repository and no `ParentDir`, so the deep-parent link
    // falls through to the anchored forms (here, the absolute path).
    let default = evaluate(PortablePath::from_path(&target).with_ctx(&ctx));
    assert_eq!(default.strategy(), &P::AbsolutePath);
    assert_eq!(
        not_applicable(attempt_for(default.attempts(), &P::RepoRoot(None))),
        &NotApplicable::NoRepository
    );

    let with_parent = PortablePath::from_path(&target)
        .with_ctx(&ctx)
        .with_strategy([P::SameDirRelative, P::ParentDir, P::AbsolutePath]);
    let found = evaluate(with_parent);
    assert_eq!(found.reference().raw(), "../../x.md");
    assert_eq!(found.strategy(), &P::ParentDir);
    // The catch-all also covers one hop when nothing earlier claimed it.
    let peer = evaluate(PortablePath::from_path(&peer).with_ctx(&ctx).with_strategy([P::ParentDir]));
    assert_eq!(peer.reference().raw(), "../c/y.md");
}

#[test]
fn a_target_equal_to_cwd_is_dot_slash_with_a_not_file_finding() {
    let fx = Fixture::new();
    let docs = fx.dir("repo/docs");
    let portable = evaluate(PortablePath::from_path(&docs).with_ctx(&fx.ctx(&docs, &[])));
    assert_eq!(portable.reference().raw(), "./");
    assert_eq!(portable.strategy(), &P::SameDirRelative);
    assert_eq!(portable.findings(), [Finding::TargetNotFile]);

    let child_dir = fx.dir("repo/docs/sub");
    let portable = evaluate(PortablePath::from_path(&child_dir).with_ctx(&fx.ctx(&docs, &[])));
    assert_eq!(portable.reference().raw(), "./sub");
    assert_eq!(portable.findings(), [Finding::TargetNotFile]);
}

#[test]
fn a_fallback_tree_treats_any_upward_route_as_external() {
    let fx = Fixture::new();
    let cwd = fx.dir("loose/a");
    let target = fx.file("loose/x.md");
    let ctx = fx.bare_ctx(&cwd, &[]);
    assert!(!ctx.base_dir_is_boundary());

    let default = evaluate(PortablePath::from_path(&target).with_ctx(&ctx));
    assert_eq!(default.strategy(), &P::AbsolutePath);
    assert_eq!(
        not_applicable(attempt_for(default.attempts(), &P::ImmediateParentDir)),
        &NotApplicable::OutsideBaseDir
    );

    let external = evaluate(
        PortablePath::from_path(&target)
            .with_ctx(&ctx)
            .with_strategy([P::ImmediateParentDir, P::ExternalRelativePath]),
    );
    assert_eq!(external.reference().raw(), "../x.md");
    assert_eq!(external.strategy(), &P::ExternalRelativePath);
}

#[test]
fn external_relative_path_is_opt_in_and_verified_with_the_reader_opt_in() {
    let fx = Fixture::new();
    let cwd = fx.dir("tree/docs");
    let inside = fx.file("tree/docs/in.md");
    let outside = fx.file("elsewhere/x.md");
    let ctx = fx.bare_ctx(&cwd, &[]).with_base_dir(fx.path("tree"));
    let strategy = [P::ExternalRelativePath];

    let found = evaluate(PortablePath::from_path(&outside).with_ctx(&ctx).with_strategy(strategy.clone()));
    assert_eq!(found.reference().raw(), "../../elsewhere/x.md");
    // A strict reader rejects the link; only an opted-in reader reaches it.
    assert!(found.reference().resolve_in_context(&ctx).is_err());
    let opted_in = ctx.clone().allow_external_relative();
    let resolved = found.reference().resolve_in_context(&opted_in).unwrap().unwrap();
    assert!(same_file(&resolved, &outside));

    let error = evaluate_err(PortablePath::from_path(&inside).with_ctx(&ctx).with_strategy(strategy));
    assert_eq!(not_applicable(&error.attempts()[0]), &NotApplicable::InsideBaseDir);

    // Not in the default: the default writes an anchored form instead.
    let default = evaluate(PortablePath::from_path(&outside).with_ctx(&ctx));
    assert_eq!(default.strategy(), &P::AbsolutePath);
}

// ---- repository preferences -------------------------------------------------

#[test]
fn repo_root_needs_a_repository_containing_the_target() {
    let fx = Fixture::new();
    let docs = fx.dir("repo/docs/topics");
    let other_checkout = fx.file("repo2/docs/x.md");

    let found = evaluate(PortablePath::from_path(&other_checkout).with_ctx(&fx.ctx(&docs, &[])));
    assert_ne!(found.strategy(), &P::RepoRoot(None), "another checkout is not this `&`");
    assert_eq!(
        not_applicable(attempt_for(found.attempts(), &P::RepoRoot(None))),
        &NotApplicable::OutsideRepository
    );

    // A context with no repository stays non-repository, even inside one.
    let in_repo = fx.file("repo/src/x.md");
    let found = evaluate(PortablePath::from_path(&in_repo).with_ctx(&fx.bare_ctx(&docs, &[])));
    assert_eq!(
        not_applicable(attempt_for(found.attempts(), &P::RepoRoot(None))),
        &NotApplicable::NoRepository
    );
}

#[test]
fn filters_restrict_eligibility_but_never_add_roots() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/apps/web");
    let ctx = fx.ctx(&cwd, &[]);
    let in_docs = fx.file("repo/docs/guide/x.md");
    let elsewhere = fx.file("repo/src/x.md");
    let strategy = [P::RepoRoot(Some("docs".into())), P::AbsolutePath];

    let found = evaluate(PortablePath::from_path(&in_docs).with_ctx(&ctx).with_strategy(strategy.clone()));
    assert_eq!(found.reference().raw(), "&docs/guide/x.md", "the filter is not a new root");

    let found = evaluate(PortablePath::from_path(&elsewhere).with_ctx(&ctx).with_strategy(strategy));
    assert_eq!(found.strategy(), &P::AbsolutePath);
    assert_eq!(
        not_applicable(&found.attempts()[0]),
        &NotApplicable::OutsideFilter {
            filter: fx.repo.join("docs")
        }
    );

    // `docs` is a whole component: `docs-old` is outside it.
    let lookalike = fx.file("repo/docs-old/x.md");
    let error = evaluate_err(
        PortablePath::from_path(&lookalike)
            .with_ctx(&ctx)
            .with_strategy([P::RepoRoot(Some("docs".into()))]),
    );
    assert!(matches!(not_applicable(&error.attempts()[0]), NotApplicable::OutsideFilter { .. }));
}

/// Claudine's adjusted strategy versus the default, for a package prompt and
/// an external prompt.
#[test]
fn a_reordered_strategy_prefers_searched_forms_the_default_never_writes() {
    let fx = Fixture::new();
    let prompts = fx.dir("repo/pkg/prompts");
    let package_prompt = fx.file("repo/pkg/prompts/a.md");
    let user_prompts = fx.dir("home/.claudine/prompts");
    let user_prompt = fx.file("home/.claudine/prompts/b.md");
    let ctx = fx
        .ctx(&prompts, &[])
        .with_package_root(fx.repo.join("pkg"))
        .add_magic_path(&user_prompts, PathPosition::End);
    let claudine = vec![
        P::AuthoredIntent(IntentForms::ALL),
        P::RepoMultiPath(Some("prompts".into())),
        P::MagicPath(Some("~/.claudine/prompts".into())),
        P::SameDirRelative,
        P::ChildDir,
        P::PeerDir,
        P::ImmediateParentDir,
        P::RepoRoot(None),
        P::EnvRootedPath,
        P::HomeDir,
        P::AbsolutePath,
    ];

    let default = evaluate(PortablePath::from_path(&package_prompt).with_ctx(&ctx));
    assert_eq!(default.reference().raw(), "./a.md");
    let reordered = evaluate(PortablePath::from_path(&package_prompt).with_ctx(&ctx).with_strategy(claudine.clone()));
    assert_eq!(reordered.reference().raw(), "^prompts/a.md");
    assert_eq!(reordered.strategy(), &P::RepoMultiPath(Some("prompts".into())));

    let default = evaluate(PortablePath::from_path(&user_prompt).with_ctx(&ctx));
    assert_eq!(default.reference().raw(), "~/.claudine/prompts/b.md");
    assert_eq!(default.strategy(), &P::HomeDir);
    let reordered = evaluate(PortablePath::from_path(&user_prompt).with_ctx(&ctx).with_strategy(claudine));
    assert_eq!(reordered.reference().raw(), "@b.md");
    assert_eq!(reordered.strategy(), &P::MagicPath(Some("~/.claudine/prompts".into())));
    assert_eq!(
        not_applicable(attempt_for(reordered.attempts(), &P::RepoMultiPath(Some("prompts".into())))),
        &NotApplicable::OutsideRepository
    );
}

#[test]
fn a_magic_filter_must_name_a_search_root() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("home/.claudine/prompts/b.md");
    let strategy = [P::MagicPath(Some("~/.claudine/prompts".into())), P::AbsolutePath];

    // Not registered as an `@` root.
    let found = evaluate(PortablePath::from_path(&target).with_ctx(&fx.ctx(&cwd, &[])).with_strategy(strategy.clone()));
    assert_eq!(
        not_applicable(&found.attempts()[0]),
        &NotApplicable::FilterNotASearchRoot {
            filter: fx.home.join(".claudine/prompts")
        }
    );

    // No home directory: the filter cannot be resolved.
    let homeless = fx.ctx(&cwd, &[]).without_home_dir();
    let found = evaluate(PortablePath::from_path(&target).with_ctx(&homeless).with_strategy(strategy));
    assert!(matches!(
        not_applicable(&found.attempts()[0]),
        NotApplicable::FilterUnavailable { problem: biscuit_file::ResolutionProblem::MissingHome, .. }
    ));
}

#[test]
fn a_shadowed_spelling_is_recorded_and_the_next_root_is_tried() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let user_prompts = fx.dir("home/prompts");
    let target = fx.file("home/prompts/x.md");
    let shadow = fx.file("repo/x.md");
    // Chain: repository (local), `home/prompts` (user prepend), home.
    let ctx = fx.ctx(&cwd, &[]).add_magic_path(&user_prompts, PathPosition::Start);

    let found = evaluate(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::MagicPath(None)]));
    assert_eq!(found.reference().raw(), "@prompts/x.md", "found through home, after the shadowed spelling");
    let attempt = &found.attempts()[0];
    assert_eq!(attempt.rejected.len(), 1);
    match &attempt.rejected[0] {
        AttemptOutcome::Shadowed {
            reference,
            resolves_to,
        } => {
            assert_eq!(reference.raw(), "@x.md");
            assert!(same_file(resolves_to, &shadow));
        }
        other => panic!("expected the shorter spelling to be shadowed, got {other}"),
    }

    // With every spelling shadowed, the strategy moves on.
    fx.file("repo/prompts/x.md");
    let found = evaluate(
        PortablePath::from_path(&target)
            .with_ctx(&ctx)
            .with_strategy([P::MagicPath(None), P::HomeDir]),
    );
    assert_eq!(found.reference().raw(), "~/prompts/x.md");
    assert!(matches!(found.attempts()[0].outcome, AttemptOutcome::Shadowed { .. }));
}

#[test]
fn search_forms_need_an_existing_file_while_single_locations_do_not() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs/topics");
    let ctx = fx.ctx(&cwd, &[]).with_package_root(fx.repo.join("docs"));
    let missing = fx.path("repo/docs/new/config.json");
    let strategy = [P::RepoMultiPath(None), P::MagicPath(None), P::RepoRoot(None)];

    let found = evaluate(PortablePath::from_path(&missing).with_ctx(&ctx).with_strategy(strategy.clone()));
    assert_eq!(found.reference().raw(), "&docs/new/config.json");
    assert_eq!(found.findings(), [Finding::TargetMissing]);
    assert_eq!(not_applicable(&found.attempts()[0]), &NotApplicable::TargetMissing);
    assert_eq!(not_applicable(&found.attempts()[1]), &NotApplicable::TargetMissing);

    let directory = fx.dir("repo/docs/assets");
    let found = evaluate(PortablePath::from_path(&directory).with_ctx(&ctx).with_strategy(strategy));
    assert_eq!(found.reference().raw(), "&docs/assets");
    assert_eq!(found.findings(), [Finding::TargetNotFile]);
    assert_eq!(not_applicable(&found.attempts()[0]), &NotApplicable::TargetNotFile);
}

// ---- home and absolute ------------------------------------------------------

#[test]
fn home_comes_from_the_context_and_absolute_is_a_visible_fallback() {
    let fx = Fixture::new();
    let cwd = fx.dir("work");
    let config = fx.path("home/.claudine/config.json");
    let ctx = fx.bare_ctx(&cwd, &[]);

    let found = evaluate(PortablePath::from_path(&config).with_ctx(&ctx));
    assert_eq!(found.reference().raw(), "~/.claudine/config.json");
    assert_eq!(found.strategy(), &P::HomeDir);
    assert_eq!(found.findings(), [Finding::TargetMissing], "about to be created");

    let homeless = ctx.clone().without_home_dir();
    let found = evaluate(PortablePath::from_path(&config).with_ctx(&homeless));
    assert_eq!(found.strategy(), &P::AbsolutePath, "the caller's cue to warn");
    assert_eq!(found.reference().raw(), absolute_text(&config));
    assert_eq!(
        not_applicable(attempt_for(found.attempts(), &P::HomeDir)),
        &NotApplicable::HomeUnavailable
    );

    let without_absolute: Vec<_> = P::DEFAULT_STRATEGY
        .iter()
        .filter(|strategy| **strategy != P::AbsolutePath)
        .cloned()
        .collect();
    let error = evaluate_err(PortablePath::from_path(&config).with_ctx(&homeless).with_strategy(without_absolute));
    let PortablePathError::NoStrategyMatched { target, attempts, .. } = &error else {
        panic!("expected NoStrategyMatched, got {error}");
    };
    assert_eq!(target, &config);
    assert_eq!(attempts.len(), P::DEFAULT_STRATEGY.len() - 1, "one attempt per preference");
    assert_eq!(error.attempts().len(), attempts.len());
    let display = error.to_string();
    let mut lines = display.lines();
    assert!(lines.next().unwrap().starts_with("no portable reference matched"));
    assert_eq!(lines.count(), attempts.len(), "one line per attempt:\n{display}");
}

#[test]
fn an_empty_strategy_matches_nothing() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo");
    let target = fx.file("repo/x.md");
    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&fx.ctx(&cwd, &[])).with_strategy([]));
    assert!(matches!(error, PortablePathError::NoStrategyMatched { .. }), "{error}");
    assert!(error.attempts().is_empty());
}
