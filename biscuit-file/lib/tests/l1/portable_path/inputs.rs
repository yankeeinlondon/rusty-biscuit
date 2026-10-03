//! Reference inputs: intent forms kept as authored, position forms resolved
//! and rewritten, minimal churn, and inputs that cannot be resolved.

use biscuit_file::{
    EnvAnchorProblem, Finding, IntentForms, NotApplicable, PORTABLE_ENV_VARIABLES,
    PortabilityPreference as P, PortablePath, PortablePathError, ResolutionProblem,
};

use super::*;

/// `md clean` on a document at `<repo>/apps/web/docs/guide.md`.
#[test]
fn the_md_clean_table_keeps_intent_and_rewrites_position() {
    let fx = Fixture::new();
    let doc = fx.file("repo/apps/web/docs/guide.md");
    fx.file("repo/foo.md");
    fx.file("repo/apps/web/foo.md");
    let config = fx.dir("config");
    fx.file("config/x.json");
    let tmp = fx.dir("scratch");
    let scratch = fx.file("scratch/x.json");
    let request = fx.ctx(
        &fx.repo,
        &[
            (PORTABLE_ENV_VARIABLES, "CONFIG_DIR"),
            ("CONFIG_DIR", config.to_str().unwrap()),
            ("TMPDIR", tmp.to_str().unwrap()),
        ],
    )
    .with_package_root(fx.repo.join("apps/web"));
    let ctx = request.for_source(&doc);

    for (authored, expected, strategy) in [
        ("^/foo.md", "^/foo.md".to_string(), P::AuthoredIntent(IntentForms::ALL)),
        ("../../../foo.md", "&foo.md".to_string(), P::RepoRoot(None)),
        ("{{CONFIG_DIR}}/x.json", "{{CONFIG_DIR}}/x.json".to_string(), P::AuthoredIntent(IntentForms::ALL)),
        ("{{TMPDIR}}/x.json", absolute_text(&scratch), P::AbsolutePath),
    ] {
        let found = evaluate(PortablePath::from_reference(reference(authored)).with_ctx(&ctx));
        assert_eq!(found.reference().raw(), expected, "{authored}");
        assert_eq!(found.strategy(), &strategy, "{authored}");
        assert!(found.findings().is_empty(), "{authored}: {:?}", found.findings());
    }

    // The position input's attempts start with AuthoredIntent declining it.
    let found = evaluate(PortablePath::from_reference(reference("../../../foo.md")).with_ctx(&ctx));
    assert_eq!(not_applicable(&found.attempts()[0]), &NotApplicable::PositionForm);
}

#[test]
fn a_path_input_is_never_authored_intent() {
    let fx = Fixture::new();
    let target = fx.file("repo/x.md");
    let found = evaluate(PortablePath::from_path(&target).with_ctx(&fx.ctx(&fx.repo, &[])));
    assert_eq!(not_applicable(&found.attempts()[0]), &NotApplicable::NotAReference);
}

#[test]
fn a_kept_reference_still_reports_its_findings() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let config = fx.dir("config");
    fx.file("config/tmp/x.json");
    fx.file("repo/pkg/README.md");
    let declared = [(PORTABLE_ENV_VARIABLES, "CONFIG_DIR")];
    let keep = |authored: &str, env: &[(&str, &str)]| {
        let found = evaluate(PortablePath::from_reference(reference(authored)).with_ctx(&fx.ctx(&cwd, env)));
        assert_eq!(found.reference().raw(), authored, "kept as authored");
        assert_eq!(found.strategy(), &P::AuthoredIntent(IntentForms::ALL));
        found.findings().to_vec()
    };

    // A portable variable is intent by the author's choice, not its value here.
    assert_eq!(
        keep("{{CONFIG_DIR}}/x.json", &declared),
        [Finding::PortableVariableUnusable {
            name: "CONFIG_DIR".into(),
            problem: EnvAnchorProblem::Unset
        }]
    );
    assert_eq!(
        keep("{{CONFIG_DIR}}/x.json", &[declared[0], ("CONFIG_DIR", "../shared")]),
        [
            Finding::PortableVariableUnusable {
                name: "CONFIG_DIR".into(),
                problem: EnvAnchorProblem::NotAbsolute { value: "../shared".into() }
            },
            // `../shared` from `repo/docs` stays in the tree; nothing is there.
            Finding::TargetMissing,
        ]
    );
    // Later placeholders are still checked.
    assert_eq!(
        keep(
            "{{CONFIG_DIR}}/{{TMPDIR}}/x.json",
            &[declared[0], ("CONFIG_DIR", config.to_str().unwrap()), ("TMPDIR", "tmp")]
        ),
        [Finding::NonPortableVariable { name: "TMPDIR".into() }]
    );
    // A sigil covers its whole payload.
    assert_eq!(
        keep("&{{PKG}}/README.md", &[("PKG", "pkg")]),
        [Finding::NonPortableVariable { name: "PKG".into() }]
    );
    // Broken links and directories.
    assert_eq!(keep("&missing.md", &[]), [Finding::TargetMissing]);
    assert_eq!(keep("&pkg", &[]), [Finding::TargetNotFile]);
    // Ruling R11: a context failure on a kept form is a finding.
    let homeless = fx.ctx(&cwd, &[]).without_home_dir();
    let found = evaluate(PortablePath::from_reference(reference("~/x.md")).with_ctx(&homeless));
    assert_eq!(found.reference().raw(), "~/x.md");
    assert_eq!(found.findings(), [Finding::ResolutionFailed(ResolutionProblem::MissingHome)]);
}

#[test]
fn urls_and_recursive_searches_are_kept_or_refused_never_rewritten() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    fx.file("repo/docs/deep/x.md");
    let ctx = fx.ctx(&cwd, &[]);
    for authored in ["https://example.com/x.md", "%x.md", "%@x.md"] {
        let found = evaluate(PortablePath::from_reference(reference(authored)).with_ctx(&ctx));
        assert_eq!(found.reference().raw(), authored);
        assert_eq!(found.strategy(), &P::AuthoredIntent(IntentForms::ALL));
        assert!(found.findings().is_empty(), "{authored}: {:?}", found.findings());

        let without_intent: Vec<_> = P::DEFAULT_STRATEGY[1..].to_vec();
        let error = evaluate_err(
            PortablePath::from_reference(reference(authored))
                .with_ctx(&ctx)
                .with_strategy(without_intent.clone()),
        );
        assert!(matches!(error, PortablePathError::NormalizationUnsupported { .. }), "{error}");
        assert_eq!(error.attempts().len(), without_intent.len());
        assert!(
            error
                .attempts()
                .iter()
                .all(|attempt| not_applicable(attempt) == &NotApplicable::NotRewritable)
        );
    }
}

#[test]
fn intent_after_same_dir_lets_a_plain_link_win() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&cwd, &[]);
    let authored = || PortablePath::from_reference(reference("&docs/x.md")).with_ctx(&ctx);

    assert_eq!(evaluate(authored()).reference().raw(), "&docs/x.md");
    let found = evaluate(authored().with_strategy([
        P::SameDirRelative,
        P::AuthoredIntent(IntentForms::ALL),
        P::RepoRoot(None),
    ]));
    assert_eq!(found.reference().raw(), "./x.md");
    assert_eq!(found.strategy(), &P::SameDirRelative);
    // Absent: normalize everything.
    let found = evaluate(authored().with_strategy([P::RepoRoot(None)]));
    assert_eq!(found.reference().raw(), "&docs/x.md");
    assert_eq!(found.strategy(), &P::RepoRoot(None));
}

#[test]
fn minimal_churn_keeps_a_relative_link_already_in_the_chosen_form() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs/sub");
    fx.file("repo/docs/sub/foo.md");
    fx.file("repo/docs/sub/a/b.md");
    fx.file("repo/docs/other/x.md");
    fx.file("repo/docs/README.md");
    let ctx = fx.ctx(&cwd, &[]);
    for (authored, expected) in [
        ("foo.md", "foo.md"),
        ("./foo.md", "./foo.md"),
        ("a/b.md", "a/b.md"),
        ("../README.md", "../README.md"),
        ("../other/x.md", "../other/x.md"),
        // Not already the chosen form: respelled.
        ("./a/../foo.md", "./foo.md"),
        ("./../sub/foo.md", "./foo.md"),
        // A bare link reached through the repository-root fallback is not a
        // same-directory link.
        ("docs/other/x.md", "../other/x.md"),
    ] {
        let found = evaluate(PortablePath::from_reference(reference(authored)).with_ctx(&ctx));
        assert_eq!(found.reference().raw(), expected, "{authored}");
    }
}

#[test]
fn an_input_without_one_target_is_unresolvable_with_a_typed_finding() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let ctx = fx.ctx(&cwd, &[]);
    let unresolvable = |authored: &str, ctx: &FileResolutionContext| {
        let error = evaluate_err(PortablePath::from_reference(reference(authored)).with_ctx(ctx));
        let PortablePathError::UnresolvableInput {
            reference,
            attempts,
            findings,
        } = &error
        else {
            panic!("{authored}: expected UnresolvableInput, got {error}");
        };
        assert_eq!(reference.raw(), authored);
        // Only strategies actually tried: the target strategies never ran.
        assert_eq!(attempts.len(), 1, "{authored}");
        assert_eq!(not_applicable(&attempts[0]), &NotApplicable::PositionForm);
        findings.clone()
    };

    // Multi-candidate forms with no match have no single target.
    assert_eq!(unresolvable("missing.md", &ctx), [Finding::TargetMissing]);
    let found = evaluate(PortablePath::from_reference(reference("@missing.md")).with_ctx(&ctx));
    assert_eq!(found.findings(), [Finding::TargetMissing], "kept as intent, reported");
    // Boundary denial and a missing anchor are not a missing file.
    assert_eq!(
        unresolvable("../../../outside.md", &ctx),
        [Finding::ResolutionFailed(ResolutionProblem::TreeEscape {
            base_dir: fx.repo.clone(),
            candidate: fx.root.parent().unwrap().join("outside.md"),
        })]
    );
    assert_eq!(
        unresolvable("{{UNSET}}/x.md", &ctx),
        [Finding::ResolutionFailed(ResolutionProblem::MissingEnvironmentVariable {
            name: "UNSET".into()
        })]
    );

    // A single-candidate form names its candidate even when it is missing.
    let found = evaluate(PortablePath::from_reference(reference("../new.md")).with_ctx(&ctx));
    assert_eq!(found.reference().raw(), "../new.md");
    assert_eq!(found.strategy(), &P::ImmediateParentDir);
    assert_eq!(found.findings(), [Finding::TargetMissing]);
}

#[cfg(unix)]
#[test]
fn a_probe_failure_is_never_a_missing_target() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    fx.file("repo/docs/blocker");
    let ctx = fx.ctx(&cwd, &[]);

    let error = evaluate_err(PortablePath::from_reference(reference("./blocker/x.md")).with_ctx(&ctx));
    let findings = error.findings();
    assert!(
        matches!(
            findings,
            [Finding::ResolutionFailed(ResolutionProblem::Probe(probe))]
                if probe.kind == std::io::ErrorKind::NotADirectory && probe.os_code.is_some()
        ),
        "{findings:?}"
    );

    let error = evaluate_err(PortablePath::from_path(cwd.join("blocker/x.md")).with_ctx(&ctx));
    assert!(
        matches!(&error, PortablePathError::ProbeFailed { error, .. } if error.kind == std::io::ErrorKind::NotADirectory),
        "{error}"
    );

    let found = evaluate(PortablePath::from_reference(reference("&docs/blocker/x.md")).with_ctx(&ctx));
    assert!(
        matches!(found.findings(), [Finding::ResolutionFailed(ResolutionProblem::Probe(_))]),
        "a kept form reports the probe failure, not TargetMissing: {:?}",
        found.findings()
    );
}

/// A cleanup caller reads older escaping links with the reader opt-in while
/// keeping the strict default output strategy.
#[test]
fn an_opted_in_reader_replaces_an_escaping_link_without_writing_another() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    fx.file("home/shared/x.md");
    let strict = fx.ctx(&cwd, &[]);
    let authored = "../../home/shared/x.md";

    let error = evaluate_err(PortablePath::from_reference(reference(authored)).with_ctx(&strict));
    assert!(matches!(error, PortablePathError::UnresolvableInput { .. }), "{error}");

    let reader = strict.allow_external_relative();
    let found = evaluate(PortablePath::from_reference(reference(authored)).with_ctx(&reader));
    assert_eq!(found.reference().raw(), "~/shared/x.md");
    assert_eq!(
        not_applicable(attempt_for(found.attempts(), &P::ImmediateParentDir)),
        &NotApplicable::OutsideBaseDir
    );
}
