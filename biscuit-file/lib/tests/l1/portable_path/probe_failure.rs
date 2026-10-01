//! A filesystem failure while verifying a preference's candidate ends
//! evaluation with `ProbeFailed`, after recording that preference's attempt
//! and every candidate it rejected before the failure.
//!
//! Each fixture places a regular file where a search root's directory is
//! expected, so the lookup of `{root}/docs/x.md` fails with `ENOTDIR`. Windows
//! reports that probe as "path not found", which the resolver treats as
//! absence, so these tests are Unix-only.

#![cfg(unix)]

use std::io::ErrorKind;
use std::path::Path;

use biscuit_file::{
    Attempt, AttemptOutcome, PathPosition, PortabilityPreference as P, PortablePath, PortablePathError,
    ProbeError,
};

use super::{Fixture, evaluate_err, same_file};

/// The typed probe failure, its attempts, and a check that the error and the
/// last attempt carry the same failure.
fn probe_failure(error: &PortablePathError) -> (&ProbeError, &[Attempt]) {
    let PortablePathError::ProbeFailed { error: probe, attempts, .. } = error else {
        panic!("expected ProbeFailed, got {error}");
    };
    (probe, attempts)
}

fn assert_not_a_directory(probe: &ProbeError, expected: &Path) {
    assert_eq!(probe.kind, ErrorKind::NotADirectory, "{probe:?}");
    assert!(probe.os_code.is_some(), "{probe:?}");
    assert!(same_file(&probe.path, expected), "{probe:?}");
}

fn failed_outcome(attempt: &Attempt) -> &ProbeError {
    match &attempt.outcome {
        AttemptOutcome::ProbeFailed(probe) => probe,
        other => panic!("{}: expected a probe failure, got {other}", attempt.strategy),
    }
}

#[test]
fn magic_search_through_a_file_root_records_the_failed_attempt() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let blocker = fx.file("repo/blocker");
    let ctx = fx.ctx(&cwd, &[]).add_magic_path(&blocker, PathPosition::Start);

    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::MagicPath(None)]));

    let (probe, attempts) = probe_failure(&error);
    assert_not_a_directory(probe, &blocker.join("docs").join("x.md"));
    assert_eq!(attempts.len(), 1, "{error}");
    assert_eq!(attempts[0].strategy, P::MagicPath(None));
    assert_eq!(failed_outcome(&attempts[0]), probe);
    assert!(error.to_string().contains("MagicPath"), "Display lists the attempt: {error}");
}

#[test]
fn repository_search_through_a_file_package_root_records_the_failed_attempt() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let blocker = fx.file("repo/blocker");
    let ctx = fx.ctx(&cwd, &[]).with_package_root(&blocker);

    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::RepoMultiPath(None)]));

    let (probe, attempts) = probe_failure(&error);
    assert_not_a_directory(probe, &blocker.join("docs").join("x.md"));
    assert_eq!(attempts.len(), 1, "{error}");
    assert_eq!(attempts[0].strategy, P::RepoMultiPath(None));
    assert_eq!(failed_outcome(&attempts[0]), probe);
}

#[test]
fn rejections_before_a_probe_failure_stay_in_the_same_attempt() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let user_prompts = fx.dir("home/prompts");
    let target = fx.file("home/prompts/x.md");
    let shadow = fx.file("repo/x.md");
    // `@x.md` (from `home/prompts`) lands on the repository's `x.md`; the
    // next spelling, `@prompts/x.md` (from home), first probes through the
    // regular file `repo/prompts`.
    let blocker = fx.file("repo/prompts");
    let ctx = fx.ctx(&cwd, &[]).add_magic_path(&user_prompts, PathPosition::Start);

    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::MagicPath(None)]));

    let (probe, attempts) = probe_failure(&error);
    assert_not_a_directory(probe, &blocker.join("x.md"));
    let [attempt] = attempts else {
        panic!("expected one attempt: {error}");
    };
    assert_eq!(failed_outcome(attempt), probe);
    match attempt.rejected.as_slice() {
        [AttemptOutcome::Shadowed { reference, resolves_to }] => {
            assert_eq!(reference.raw(), "@x.md");
            assert!(same_file(resolves_to, &shadow));
        }
        other => panic!("expected the shadowed spelling before the failure, got {other:?}"),
    }
}

#[test]
fn earlier_preferences_precede_the_failed_one() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/other");
    let target = fx.file("repo/docs/x.md");
    let blocker = fx.file("repo/blocker");
    let ctx = fx.ctx(&cwd, &[]).add_magic_path(&blocker, PathPosition::Start);
    let strategy = [P::SameDirRelative, P::ChildDir, P::HomeDir, P::MagicPath(None), P::RepoRoot(None)];

    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy(strategy));

    let (probe, attempts) = probe_failure(&error);
    assert_not_a_directory(probe, &blocker.join("docs").join("x.md"));
    let tried: Vec<&P> = attempts.iter().map(|attempt| &attempt.strategy).collect();
    assert_eq!(tried, [&P::SameDirRelative, &P::ChildDir, &P::HomeDir, &P::MagicPath(None)]);
    for earlier in &attempts[..3] {
        assert!(
            matches!(earlier.outcome, AttemptOutcome::NotApplicable(_)),
            "{earlier}"
        );
    }
    assert_eq!(failed_outcome(&attempts[3]), probe);
}

#[test]
fn a_target_probe_failure_is_not_a_failed_preference() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let blocker = fx.file("repo/docs/blocker");
    let ctx = fx.ctx(&cwd, &[]);
    let target = blocker.join("x.md");

    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&ctx).with_strategy([P::MagicPath(None)]));
    let (probe, attempts) = probe_failure(&error);
    assert_not_a_directory(probe, &target);
    assert!(attempts.is_empty(), "no preference ran: {error}");

    // The default strategy decides `AuthoredIntent` without a target first;
    // no attempt reports the target's own probe failure.
    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&ctx));
    let (probe, attempts) = probe_failure(&error);
    assert_not_a_directory(probe, &target);
    assert!(
        attempts
            .iter()
            .all(|attempt| !matches!(attempt.outcome, AttemptOutcome::ProbeFailed(_))),
        "{error}"
    );
}
