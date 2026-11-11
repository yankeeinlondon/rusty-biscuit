//! `ci_push::run`: what it validates before discovery, what it refuses before
//! review, and which pushes need no review at all.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use gix::hash::Kind;
use rstest::rstest;

use crate::ci_push::{
    BlockReason, Diagnostic, HostServices, PushMode, PushOutcome, PushRequest, PushServices,
    SkipReason, run,
};

const A: &str = "1111111111111111111111111111111111111111";
const B: &str = "2222222222222222222222222222222222222222";
const ZERO: &str = "0000000000000000000000000000000000000000";

/// Counts every boundary `run` reaches.
#[derive(Default)]
struct Recorder {
    discoveries: Vec<PathBuf>,
    fail_discovery: bool,
}

impl PushServices for Recorder {
    fn object_hash(&mut self, cwd: &Path) -> anyhow::Result<Kind> {
        self.discoveries.push(cwd.to_owned());
        if self.fail_discovery {
            anyhow::bail!("{} is not inside a Git repository", cwd.display());
        }
        Ok(Kind::Sha1)
    }
}

fn request(mode: Option<&str>, input: &str) -> PushRequest {
    PushRequest {
        cwd: PathBuf::from("checkout"),
        remote_name: "origin".into(),
        remote_url: None,
        input: input.as_bytes().to_vec(),
        mode: mode.map(OsString::from),
        selection: None,
    }
}

fn decide(request: PushRequest) -> (PushOutcome, Recorder) {
    let mut recorder = Recorder::default();
    let outcome = run(request, &mut recorder).expect("no boundary failed");
    (outcome, recorder)
}

#[rstest]
#[case::unset(None, PushMode::Auto)]
#[case::empty(Some(""), PushMode::Auto)]
#[case::auto(Some("auto"), PushMode::Auto)]
#[case::strict(Some("strict"), PushMode::Strict)]
#[case::warn(Some("warn"), PushMode::Warn)]
#[case::scope_only(Some("scope-only"), PushMode::ScopeOnly)]
fn every_mode_spelling_is_read(#[case] value: Option<&str>, #[case] mode: PushMode) {
    let (outcome, _) = decide(request(value, ""));
    assert_eq!(outcome.mode, Some(mode));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.diagnostics, [Diagnostic::NothingToReview]);
}

#[test]
fn off_is_a_deprecated_alias_of_scope_only() {
    let (outcome, _) = decide(request(Some("off"), ""));
    assert_eq!(outcome.mode, Some(PushMode::ScopeOnly));
    assert_eq!(outcome.diagnostics, [Diagnostic::ModeDeprecated, Diagnostic::NothingToReview]);
    assert_eq!(
        outcome.diagnostics[0].to_string(),
        "RUSTY_BISCUIT_PRE_PUSH=off is deprecated; it now means scope-only."
    );
}

#[rstest]
#[case::unknown("sometimes")]
#[case::padded(" strict")]
#[case::capitalized("Strict")]
fn an_invalid_mode_blocks_before_discovery(#[case] value: &str) {
    let input = format!("refs/heads/a {A} refs/heads/a {ZERO}\n");
    let (outcome, recorder) = decide(request(Some(value), &input));
    assert_eq!(outcome.exit_code, 1);
    assert_eq!(outcome.mode, None);
    assert!(outcome.updates.is_empty());
    assert!(recorder.discoveries.is_empty(), "discovery ran for an invalid mode");
    let [Diagnostic::Blocked(BlockReason::InvalidMode(error))] = &outcome.diagnostics[..] else {
        panic!("{:?}", outcome.diagnostics);
    };
    assert_eq!(error.value, value);
    let message = outcome.diagnostics[0].to_string();
    assert!(message.contains("expected auto, scope-only, warn, or strict"), "{message}");
    assert!(message.contains("off is a deprecated alias of scope-only"), "{message}");
}

#[rstest]
#[case::unset(None)]
#[case::empty(Some(""))]
#[case::whitespace(Some(" \t "))]
#[case::names(Some("repo-deps  sniff"))]
fn a_readable_selection_never_blocks(#[case] selection: Option<&str>) {
    let mut push = request(None, "");
    push.selection = selection.map(OsString::from);
    let (outcome, recorder) = decide(push);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(recorder.discoveries.len(), 1);
}

#[cfg(unix)]
#[test]
fn an_unreadable_selection_blocks_before_discovery() {
    use std::os::unix::ffi::OsStringExt;

    let mut push = request(Some("strict"), "");
    push.selection = Some(OsString::from_vec(b"repo-\xffdeps".to_vec()));
    let (outcome, recorder) = decide(push);
    assert_eq!(outcome.exit_code, 1);
    assert!(matches!(outcome.diagnostics[..], [Diagnostic::Blocked(BlockReason::InvalidSelection(_))]));
    assert!(recorder.discoveries.is_empty());
}

#[test]
fn a_malformed_line_blocks_the_whole_push_before_review() {
    // The first update is valid and reviewable; the malformed second line
    // must still stop the push before any part of it is reviewed.
    let input = format!("refs/heads/a {A} refs/heads/a {ZERO}\nrefs/heads/b {B} refs/heads/b\n");
    let (outcome, recorder) = decide(request(Some("warn"), &input));
    assert_eq!(outcome.exit_code, 1, "warn never passes a review failure");
    assert!(outcome.updates.is_empty());
    assert_eq!(recorder.discoveries, [PathBuf::from("checkout")]);
    let [Diagnostic::Blocked(BlockReason::MalformedInput(error))] = &outcome.diagnostics[..] else {
        panic!("{:?}", outcome.diagnostics);
    };
    assert_eq!(error.line, 2);
    let message = outcome.diagnostics[0].to_string();
    assert!(message.contains("ref update line 2 has 3 fields"), "{message}");
    assert!(message.contains("nothing was reviewed, published, or run"), "{message}");
}

#[test]
fn a_push_of_only_deletions_and_tags_names_each_skip_and_passes() {
    let input = format!(
        "(delete) {ZERO} refs/heads/old {A}\nrefs/tags/v1 {B} refs/tags/v1 {ZERO}\n"
    );
    let (outcome, _) = decide(request(Some("strict"), &input));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.updates.len(), 2);
    let skipped: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            Diagnostic::UpdateSkipped { update, reason } => Some((update.remote_ref.as_str(), *reason)),
            _ => None,
        })
        .collect();
    assert_eq!(
        skipped,
        [("refs/heads/old", SkipReason::Deletion), ("refs/tags/v1", SkipReason::NotABranch)]
    );
    assert_eq!(outcome.diagnostics.last(), Some(&Diagnostic::NothingToReview));
    assert_eq!(
        outcome.diagnostics[1].to_string(),
        "Not reviewing refs/tags/v1 -> refs/tags/v1: only a branch update triggers a run."
    );
}

#[test]
fn a_branch_update_is_never_passed_unreviewed() {
    // Until review is wired in, a reviewable push blocks rather than passing
    // without the checks the shell hook performs.
    let input = format!("refs/tags/v1 {B} refs/tags/v1 {ZERO}\nrefs/heads/a {A} refs/heads/a {ZERO}\n");
    let (outcome, _) = decide(request(Some("scope-only"), &input));
    assert_eq!(outcome.exit_code, 1);
    assert!(matches!(outcome.diagnostics[0], Diagnostic::UpdateSkipped { .. }));
    assert_eq!(outcome.diagnostics.last(), Some(&Diagnostic::Blocked(BlockReason::ReviewUnavailable)));
}

#[test]
fn a_discovery_failure_is_an_error() {
    let mut recorder = Recorder { fail_discovery: true, ..Recorder::default() };
    let error = run(request(None, ""), &mut recorder).unwrap_err();
    assert!(error.to_string().contains("is not inside a Git repository"));
}

#[test]
fn the_host_reads_a_repository_object_format_and_decides_a_tag_push() {
    let temp = tempfile::tempdir().unwrap();
    git2::Repository::init(temp.path()).unwrap();
    let mut host = HostServices;
    assert_eq!(host.object_hash(temp.path()).unwrap(), Kind::Sha1);

    let mut push = request(Some("strict"), &format!("refs/tags/v1 {A} refs/tags/v1 {ZERO}\r\n"));
    push.cwd = temp.path().to_owned();
    let outcome = run(push, &mut host).unwrap();
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.diagnostics.last(), Some(&Diagnostic::NothingToReview));
}
