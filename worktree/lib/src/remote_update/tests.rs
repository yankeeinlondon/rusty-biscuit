//! [`run_attempt`] against a real local bare `origin` and a `pusher` clone
//! ([`TestRepo`]), with the provider API scripted, Git real unless a test
//! scripts it, and a test clock.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use super::*;
use crate::remote_head::{CredentialEvidence, StoreState, read_store};
use crate::remove::test_support::TestRepo;

const ID: &str = "0123456789abcdef0123456789abcdef";
const NOW: u64 = 1_790_000_000;
const OLD_SHA: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// Unix and monotonic time, advanced only by the test.
struct Clock {
    unix: Cell<u64>,
    monotonic: Cell<Duration>,
}

impl Clock {
    fn new() -> Self {
        Self { unix: Cell::new(NOW), monotonic: Cell::new(Duration::ZERO) }
    }

    fn advance(&self, by: Duration) {
        self.unix.set(self.unix.get() + by.as_secs());
        self.monotonic.set(self.monotonic.get() + by);
    }
}

type Hook<'a> = Box<dyn Fn() + 'a>;

/// A scripted provider API: each request takes the next answer, runs the next
/// hook, and advances the clock by `takes`. A request past the script fails
/// the test.
struct Api<'a> {
    answers: RefCell<VecDeque<Result<String, PrUnavailable>>>,
    hooks: RefCell<VecDeque<Hook<'a>>>,
    /// What every successful request reports it was sent with.
    credentials: CredentialEvidence,
    takes: Duration,
    clock: &'a Clock,
    calls: Cell<usize>,
}

impl<'a> Api<'a> {
    fn new(clock: &'a Clock, answers: impl IntoIterator<Item = Result<String, PrUnavailable>>) -> Self {
        Self {
            answers: RefCell::new(answers.into_iter().collect()),
            hooks: RefCell::new(VecDeque::new()),
            credentials: CredentialEvidence::Anonymous,
            takes: Duration::ZERO,
            clock,
            calls: Cell::new(0),
        }
    }

    fn unsupported(clock: &'a Clock) -> Self {
        let unsupported = || Err(PrUnavailable::Unsupported { message: "a local path".into() });
        Self::new(clock, [unsupported(), unsupported()])
    }

    fn during(self, hook: impl Fn() + 'a) -> Self {
        self.hooks.borrow_mut().push_back(Box::new(hook));
        self
    }
}

impl BranchHeadSource for Api<'_> {
    fn branch_head(&self, _origin: &str, _branch: &str, deadline: Duration) -> Result<ApiHead, PrUnavailable> {
        assert_eq!(deadline, REMOTE_HEAD_REFRESH_DEADLINE, "the API call starts the budget");
        self.calls.set(self.calls.get() + 1);
        if let Some(hook) = self.hooks.borrow_mut().pop_front() {
            hook();
        }
        self.clock.advance(self.takes);
        let answer = self.answers.borrow_mut().pop_front().expect("an unscripted API request");
        answer.map(|sha| ApiHead { sha, credentials: self.credentials.clone() })
    }
}

/// Real Git in the repository, unless the test scripts an answer; it records
/// every request's deadline.
struct Git<'a> {
    main: PathBuf,
    live: RefCell<VecDeque<Result<Option<String>, GitFailure>>>,
    fetches: RefCell<VecDeque<Result<(), GitFailure>>>,
    before_fetch: RefCell<Option<Hook<'a>>>,
    live_deadlines: RefCell<Vec<Duration>>,
    fetch_deadlines: RefCell<Vec<Duration>>,
}

impl<'a> Git<'a> {
    fn real(main: &Path) -> Self {
        Self {
            main: main.to_path_buf(),
            live: RefCell::default(),
            fetches: RefCell::default(),
            before_fetch: RefCell::default(),
            live_deadlines: RefCell::default(),
            fetch_deadlines: RefCell::default(),
        }
    }

    fn live(self, answer: Result<Option<String>, GitFailure>) -> Self {
        self.live.borrow_mut().push_back(answer);
        self
    }

    fn fetch_fails(self, failure: GitFailure) -> Self {
        self.fetches.borrow_mut().push_back(Err(failure));
        self
    }

    fn before_fetch(self, hook: impl Fn() + 'a) -> Self {
        *self.before_fetch.borrow_mut() = Some(Box::new(hook));
        self
    }

    fn live_calls(&self) -> usize {
        self.live_deadlines.borrow().len()
    }

    fn fetch_calls(&self) -> usize {
        self.fetch_deadlines.borrow().len()
    }
}

impl GitRemote for Git<'_> {
    fn live_head(&self, branch: &str, deadline: Duration) -> Result<Option<String>, GitFailure> {
        self.live_deadlines.borrow_mut().push(deadline);
        match self.live.borrow_mut().pop_front() {
            Some(answer) => answer,
            None => GitTransport { main: &self.main }.live_head(branch, deadline),
        }
    }

    fn fetch(&self, branch: &str, deadline: Duration) -> Result<(), GitFailure> {
        self.fetch_deadlines.borrow_mut().push(deadline);
        if let Some(hook) = self.before_fetch.borrow_mut().take() {
            hook();
        }
        match self.fetches.borrow_mut().pop_front() {
            Some(answer) => answer,
            None => GitTransport { main: &self.main }.fetch(branch, deadline),
        }
    }
}

fn store(repo: &TestRepo) -> PathBuf {
    repo.cache_path().join("abc.remote-head.json")
}

fn digest(repo: &TestRepo) -> String {
    origin_digest(&origin_url(&repo.path()).expect("an origin"))
}

fn run_with(repo: &TestRepo, api: &dyn BranchHeadSource, git: &dyn GitRemote, clock: &Clock, ignore_api: bool) -> AttemptEnd {
    let main = repo.path();
    let store = store(repo);
    let now = || clock.unix.get();
    let monotonic = || clock.monotonic.get();
    run_attempt(
        AttemptRequest { store: &store, main: &main, id: ID, ignore_api },
        &Seams { api, git, now: &now, monotonic: &monotonic },
    )
}

fn run(repo: &TestRepo, api: &dyn BranchHeadSource, git: &dyn GitRemote, clock: &Clock) -> AttemptEnd {
    run_with(repo, api, git, clock, false)
}

fn stored(repo: &TestRepo) -> StoreState {
    read_store(&store(repo))
}

fn answer(repo: &TestRepo, sha: Option<&str>, checked_at: u64, source: AnswerSource) -> Answer {
    Answer { origin_digest: digest(repo), branch: "main".into(), sha: sha.map(str::to_string), checked_at, source }
}

/// Publishes an earlier answer, as a previous attempt would have.
fn seed_old_answer(repo: &TestRepo) -> Answer {
    let old = answer(repo, Some(OLD_SHA), NOW - 3600, AnswerSource::Git);
    publish_answer(&store(repo), &old).unwrap();
    old
}

fn finished(state: &StoreState) -> (Phase, Option<Outcome>, Option<ApiNote>) {
    let attempt = state.attempt.clone().expect("an attempt record");
    assert_eq!(attempt.id, ID);
    (attempt.phase, attempt.outcome, attempt.api)
}

fn fetch_head_exists(repo: &TestRepo) -> bool {
    repo.path().join(".git").join("FETCH_HEAD").exists()
}

#[test]
fn no_variance_is_in_sync_with_no_fetch() {
    let repo = TestRepo::with_origin();
    let clock = Clock::new();
    let tip = repo.sha("origin/main");
    let api = Api::new(&clock, [Ok(tip.clone())]);
    let git = Git::real(&repo.path());

    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::InSync));

    assert_eq!(git.fetch_calls() + git.live_calls(), 0, "the API answered; no Git request");
    let state = stored(&repo);
    assert_eq!(state.answer, Some(answer(&repo, Some(&tip), NOW, AnswerSource::Api)));
    assert_eq!(finished(&state), (Phase::Checking, Some(Outcome::InSync), None));
    assert_eq!(state.attempt.unwrap().started_at, NOW);
}

#[test]
fn variance_fetches_and_publishes_the_fetched_tip() {
    let repo = TestRepo::with_origin();
    let before = repo.sha("origin/main");
    let pushed = repo.push_commit_to_origin("main", "upstream.txt");
    let clock = Clock::new();
    // The check takes 1 s, so the fetch starts later than it.
    let mut api = Api::new(&clock, [Ok(pushed.clone())]);
    api.takes = Duration::from_secs(1);
    let git = Git::real(&repo.path());

    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::Fetched));

    assert_ne!(before, pushed);
    assert_eq!(repo.sha("origin/main"), pushed, "the tracking ref moved to the remote head");
    assert_eq!(repo.sha("main"), before, "the local branch did not move");
    assert!(!fetch_head_exists(&repo), "no FETCH_HEAD");
    assert_eq!(*git.fetch_deadlines.borrow(), [FETCH_DEADLINE]);
    let state = stored(&repo);
    assert_eq!(state.answer, Some(answer(&repo, Some(&pushed), NOW + 1, AnswerSource::Fetch)));
    assert_eq!(finished(&state), (Phase::Fetching, Some(Outcome::Fetched), None));
}

#[test]
fn a_remote_move_between_check_and_fetch_is_reported_from_the_fetched_tip() {
    let repo = TestRepo::with_origin();
    let checked = repo.push_commit_to_origin("main", "first.txt");
    let clock = Clock::new();
    let api = Api::new(&clock, [Ok(checked.clone())]);
    let later = RefCell::new(None);
    let git = Git::real(&repo.path()).before_fetch(|| {
        *later.borrow_mut() = Some(repo.push_commit_to_origin("main", "second.txt"));
    });

    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::Fetched));

    let later = later.borrow().clone().expect("pushed during the attempt");
    assert_ne!(later, checked);
    assert_eq!(repo.sha("origin/main"), later);
    assert_eq!(stored(&repo).answer.unwrap().sha.as_deref(), Some(later.as_str()), "not the earlier check");
}

#[test]
fn a_fetch_timeout_keeps_the_new_answer_and_the_tracking_ref() {
    let repo = TestRepo::with_origin();
    seed_old_answer(&repo);
    let before = repo.sha("origin/main");
    let pushed = repo.push_commit_to_origin("main", "upstream.txt");
    let clock = Clock::new();
    let api = Api::new(&clock, [Ok(pushed.clone())]);
    let git = Git::real(&repo.path()).fetch_fails(GitFailure::Timeout);

    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::FetchFailed { reason: FetchFailure::Timeout })
    );

    assert_eq!(repo.sha("origin/main"), before, "the tracking ref is untouched");
    let state = stored(&repo);
    assert_eq!(state.answer, Some(answer(&repo, Some(&pushed), NOW, AnswerSource::Api)), "the new check stays");
    assert_eq!(finished(&state).0, Phase::Fetching);

    // Any other fetch failure is `other`, whatever git's text said.
    let git = Git::real(&repo.path()).fetch_fails(GitFailure::Credentials);
    let api = Api::new(&clock, [Ok(pushed)]);
    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::FetchFailed { reason: FetchFailure::Other })
    );
}

#[test]
fn a_check_that_uses_the_whole_budget_fails_as_a_timeout_and_keeps_the_old_answer() {
    let repo = TestRepo::with_origin();
    let old = seed_old_answer(&repo);
    let clock = Clock::new();
    let mut api = Api::new(&clock, [Err(PrUnavailable::Timeout { deadline: REMOTE_HEAD_REFRESH_DEADLINE })]);
    api.takes = REMOTE_HEAD_REFRESH_DEADLINE;
    let git = Git::real(&repo.path());

    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::CheckFailed { reason: CheckFailure::Timeout })
    );
    assert_eq!(git.live_calls(), 0, "no budget was left for the fallback");
    assert_eq!(stored(&repo).answer, Some(old.clone()));

    // An `ls-remote` that reaches its deadline is a timeout too.
    let api = Api::unsupported(&clock);
    let git = Git::real(&repo.path()).live(Err(GitFailure::Timeout));
    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::CheckFailed { reason: CheckFailure::Timeout })
    );
    assert_eq!(stored(&repo).answer, Some(old));
}

#[test]
fn the_api_call_and_the_fallback_share_one_budget() {
    let repo = TestRepo::with_origin();
    let clock = Clock::new();
    let mut api = Api::new(&clock, [Err(PrUnavailable::Network { message: "reset".into() })]);
    api.takes = Duration::from_secs(7);
    let git = Git::real(&repo.path());

    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::InSync));

    assert_eq!(*git.live_deadlines.borrow(), [Duration::from_secs(3)], "10 s less the API's 7 s");
    let state = stored(&repo);
    assert_eq!(state.answer.clone().unwrap().source, AnswerSource::Git);
    assert_eq!(finished(&state), (Phase::CheckingFallback { reason: FallbackReason::Other }, Some(Outcome::InSync), None));
}

#[test]
fn an_unsupported_remote_is_checked_by_ls_remote_in_phase_checking() {
    // A local bare origin: sniff itself refuses it without a request.
    let repo = TestRepo::with_origin();
    let clock = Clock::new();
    let git = Git::real(&repo.path());

    assert_eq!(run(&repo, &SniffBranchHeads, &git, &clock), AttemptEnd::Finished(Outcome::InSync));

    assert_eq!(git.live_calls(), 1);
    let state = stored(&repo);
    let tip = repo.sha("origin/main");
    assert_eq!(state.answer, Some(answer(&repo, Some(&tip), NOW, AnswerSource::Git)));
    assert_eq!(finished(&state), (Phase::Checking, Some(Outcome::InSync), None), "no fallback, no note");
}

#[test]
fn each_fallback_reason_and_condition_is_recorded() {
    let key = || "GITHUB_TOKEN".to_string();
    // The 404's key is the one its own request sent, as sniff reports it.
    let not_found = |key| PrUnavailable::NotFoundOrNotPermitted { message: "404".into(), key };
    let note = |condition, key: Option<String>| Some(ApiNote { condition, key, fallback_answered: true });
    let cases: Vec<(PrUnavailable, FallbackReason, Option<ApiNote>)> = vec![
        (
            PrUnavailable::CredentialsRequired { key: None },
            FallbackReason::NoKey,
            note(ApiCondition::CredentialsRequired, None),
        ),
        (not_found(None), FallbackReason::NotVisible, note(ApiCondition::NotFoundOrNotPermitted, None)),
        (
            not_found(Some("SNIFF_GITHUB_GIT_2E_EXAMPLE_TOKEN".into())),
            FallbackReason::Other,
            note(ApiCondition::NotFoundOrNotPermitted, Some("SNIFF_GITHUB_GIT_2E_EXAMPLE_TOKEN".into())),
        ),
        (
            PrUnavailable::RateLimited { authenticated: false, key: None },
            FallbackReason::RateLimited,
            note(ApiCondition::RateLimited { authenticated: false }, None),
        ),
        (
            PrUnavailable::RateLimited { authenticated: true, key: Some(key()) },
            FallbackReason::RateLimited,
            note(ApiCondition::RateLimited { authenticated: true }, Some(key())),
        ),
        (
            PrUnavailable::CredentialsRejected { key: key() },
            FallbackReason::Rejected,
            note(ApiCondition::CredentialsRejected, Some(key())),
        ),
        (
            PrUnavailable::CredentialsInsufficient { key: key() },
            FallbackReason::Rejected,
            note(ApiCondition::CredentialsInsufficient, Some(key())),
        ),
        (PrUnavailable::Timeout { deadline: Duration::from_secs(1) }, FallbackReason::Other, None),
        (PrUnavailable::Network { message: "reset".into() }, FallbackReason::Other, None),
        (PrUnavailable::Other { message: "odd".into() }, FallbackReason::Other, None),
    ];
    for (error, reason, expected_note) in cases {
        let repo = TestRepo::with_origin();
        let clock = Clock::new();
        let label = format!("{error:?}");
        // The phase is written before the fallback's request.
        let seen = RefCell::new(None);
        let api = Api::new(&clock, [Err(error)]);
        let git = Git::real(&repo.path());
        let store_path = store(&repo);
        let wrapped = WatchPhase { git: &git, store: &store_path, seen: &seen };

        assert_eq!(run(&repo, &api, &wrapped, &clock), AttemptEnd::Finished(Outcome::InSync), "{label}");

        assert_eq!(*seen.borrow(), Some(Phase::CheckingFallback { reason }), "{label}");
        let state = stored(&repo);
        assert_eq!(state.answer.clone().unwrap().source, AnswerSource::Git, "{label}");
        assert_eq!(
            finished(&stored(&repo)),
            (Phase::CheckingFallback { reason }, Some(Outcome::InSync), expected_note),
            "{label}"
        );
    }
}

/// Records the stored phase when `ls-remote` starts.
struct WatchPhase<'a> {
    git: &'a dyn GitRemote,
    store: &'a Path,
    seen: &'a RefCell<Option<Phase>>,
}

impl GitRemote for WatchPhase<'_> {
    fn live_head(&self, branch: &str, deadline: Duration) -> Result<Option<String>, GitFailure> {
        *self.seen.borrow_mut() = read_store(self.store).attempt.map(|attempt| attempt.phase);
        self.git.live_head(branch, deadline)
    }

    fn fetch(&self, branch: &str, deadline: Duration) -> Result<(), GitFailure> {
        self.git.fetch(branch, deadline)
    }
}

#[test]
fn an_ignored_repository_makes_no_provider_request() {
    let repo = TestRepo::with_origin();
    let pushed = repo.push_commit_to_origin("main", "upstream.txt");
    let clock = Clock::new();
    let api = Api::new(&clock, []);
    let git = Git::real(&repo.path());

    assert_eq!(run_with(&repo, &api, &git, &clock, true), AttemptEnd::Finished(Outcome::Fetched));

    assert_eq!(api.calls.get(), 0);
    assert_eq!(git.live_calls(), 1);
    assert_eq!(repo.sha("origin/main"), pushed);
    assert_eq!(finished(&stored(&repo)), (Phase::Fetching, Some(Outcome::Fetched), None));
}

#[test]
fn only_ls_remote_proves_absence() {
    let repo = TestRepo::with_origin();
    let tracking = repo.sha("origin/main");
    repo.git_in(&repo.origin_path(), &["update-ref", "-d", "refs/heads/main"]);
    let clock = Clock::new();
    let api = Api::new(&clock, [Err(PrUnavailable::NotFoundOrNotPermitted { message: "404".into(), key: None })]);
    let git = Git::real(&repo.path());

    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::Absent));

    assert_eq!(git.fetch_calls(), 0);
    assert_eq!(repo.sha("origin/main"), tracking, "an absence keeps the tracking ref");
    assert_eq!(stored(&repo).answer, Some(answer(&repo, None, NOW, AnswerSource::Git)));

    // A 404 whose fallback fails too is a failed check, never an absence.
    let repo = TestRepo::with_origin();
    let old = seed_old_answer(&repo);
    let api = Api::new(&clock, [Err(PrUnavailable::NotFoundOrNotPermitted { message: "404".into(), key: None })]);
    let git = Git::real(&repo.path()).live(Err(GitFailure::Other));
    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::CheckFailed { reason: CheckFailure::Other })
    );
    let state = stored(&repo);
    assert_eq!(state.answer, Some(old));
    let note = finished(&state).2.expect("the 404 is noted");
    assert!(!note.fallback_answered);

    // A credentials failure from Git is named as one.
    let api = Api::unsupported(&clock);
    let git = Git::real(&repo.path()).live(Err(GitFailure::Credentials));
    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::CheckFailed { reason: CheckFailure::Credentials })
    );
}

#[test]
fn an_origin_or_default_branch_change_during_the_check_publishes_nothing() {
    let repo = TestRepo::with_origin();
    let old = seed_old_answer(&repo);
    let clock = Clock::new();
    let api = Api::new(&clock, [Ok(OLD_SHA.replace('8', "7"))])
        .during(|| drop(repo.git(&["remote", "set-url", "origin", "https://heads.example.invalid/o/new.git"])));

    assert_eq!(
        run(&repo, &api, &Git::real(&repo.path()), &clock),
        AttemptEnd::Finished(Outcome::Unavailable { reason: UnavailableReason::OriginChanged })
    );
    assert_eq!(stored(&repo).answer, Some(old.clone()));

    let repo = TestRepo::with_origin();
    repo.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    let old = seed_old_answer(&repo);
    let api = Api::new(&clock, [Ok(OLD_SHA.replace('8', "7"))])
        .during(|| drop(repo.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"])));
    assert_eq!(
        run(&repo, &api, &Git::real(&repo.path()), &clock),
        AttemptEnd::Finished(Outcome::Unavailable { reason: UnavailableReason::BranchChanged })
    );
    assert_eq!(stored(&repo).answer, Some(old));
}

#[test]
fn an_origin_change_during_the_fetch_keeps_the_check_and_ends_unavailable() {
    let repo = TestRepo::with_origin();
    let pushed = repo.push_commit_to_origin("main", "upstream.txt");
    let clock = Clock::new();
    let api = Api::new(&clock, [Ok(pushed.clone())]);
    let git = Git::real(&repo.path())
        .before_fetch(|| drop(repo.git(&["remote", "set-url", "origin", "https://heads.example.invalid/o/new.git"])))
        .fetch_fails(GitFailure::Other);

    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::Unavailable { reason: UnavailableReason::OriginChanged })
    );
    assert_eq!(stored(&repo).answer.unwrap().sha, Some(pushed), "published before the fetch");
}

#[test]
fn a_concurrent_fetch_that_reached_the_current_remote_head_counts_as_fetched() {
    let repo = TestRepo::with_origin();
    let pushed = repo.push_commit_to_origin("main", "upstream.txt");
    let clock = Clock::new();
    // The check, then the recheck after the failed fetch.
    let api = Api::new(&clock, [Ok(pushed.clone()), Ok(pushed.clone())]);
    let git = Git::real(&repo.path())
        .before_fetch(|| {
            // Another process's fetch wins the ref lock and lands the head.
            repo.git(&["fetch", "-q", "origin"]);
            clock.advance(Duration::from_secs(4));
        })
        .fetch_fails(GitFailure::Other);

    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::Fetched));

    assert_eq!(api.calls.get(), 2);
    assert_eq!(stored(&repo).answer, Some(answer(&repo, Some(&pushed), NOW + 4, AnswerSource::Api)), "the recheck");
}

#[test]
fn a_concurrent_fetch_of_an_older_head_is_not_labeled_current() {
    let repo = TestRepo::with_origin();
    let checked = repo.push_commit_to_origin("main", "first.txt");
    let clock = Clock::new();
    let newer = RefCell::new(String::new());
    let git = Git::real(&repo.path())
        .before_fetch(|| {
            // Another fetch lands the SHA this attempt checked, and then the
            // remote moves on.
            repo.git(&["fetch", "-q", "origin"]);
            *newer.borrow_mut() = repo.push_commit_to_origin("main", "second.txt");
        })
        .fetch_fails(GitFailure::Other);
    let recheck = RefCell::new(None);
    let api = RecheckApi { first: checked.clone(), newer: &newer, recheck: &recheck };

    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::FetchFailed { reason: FetchFailure::Other })
    );

    assert_eq!(recheck.borrow().as_deref(), Some(newer.borrow().as_str()), "the recheck saw the newer head");
    assert_eq!(repo.sha("origin/main"), checked);
    assert_eq!(stored(&repo).answer, Some(answer(&repo, Some(&checked), NOW, AnswerSource::Api)), "the first check");
}

/// Answers `first`, then whatever `newer` holds by the time of the recheck.
struct RecheckApi<'a> {
    first: String,
    newer: &'a RefCell<String>,
    recheck: &'a RefCell<Option<String>>,
}

impl BranchHeadSource for RecheckApi<'_> {
    fn branch_head(&self, _origin: &str, _branch: &str, _deadline: Duration) -> Result<ApiHead, PrUnavailable> {
        let newer = self.newer.borrow().clone();
        let head = |sha| Ok(ApiHead { sha, credentials: CredentialEvidence::Anonymous });
        if newer.is_empty() {
            return head(self.first.clone());
        }
        *self.recheck.borrow_mut() = Some(newer.clone());
        head(newer)
    }
}

#[test]
fn the_attempt_is_recorded_before_any_request() {
    let repo = TestRepo::with_origin();
    let clock = Clock::new();
    let seen = RefCell::new(None);
    let tip = repo.sha("origin/main");
    let api = Api::new(&clock, [Ok(tip)]).during(|| *seen.borrow_mut() = stored(&repo).attempt);

    run(&repo, &api, &Git::real(&repo.path()), &clock);

    let attempt = seen.borrow().clone().expect("recorded before the request");
    assert_eq!(attempt, Attempt::begin(ID.into(), digest(&repo), "main".into(), NOW));
}

#[test]
fn a_contended_lock_writes_nothing_and_asks_nothing() {
    let repo = TestRepo::with_origin();
    fs::create_dir_all(repo.cache_path()).unwrap();
    let held = try_lock_sidecar(&remote_head_lock_path(&store(&repo))).unwrap().expect("the test holds the lock");
    let clock = Clock::new();
    let api = Api::new(&clock, []);
    let git = Git::real(&repo.path());

    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Contended);

    assert_eq!(api.calls.get() + git.live_calls() + git.fetch_calls(), 0);
    assert!(!store(&repo).exists());
    drop(held);
    assert_eq!(AttemptEnd::Contended.head_status(), HeadStatus::AdoptedElsewhere);

    // Once released, the next attempt runs, and the sidecar is never unlinked.
    let api = Api::unsupported(&clock);
    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::InSync));
    assert!(remote_head_lock_path(&store(&repo)).exists());
}

#[test]
fn a_lock_that_cannot_be_taken_records_nothing() {
    let repo = TestRepo::with_origin();
    fs::create_dir_all(remote_head_lock_path(&store(&repo))).unwrap();
    let clock = Clock::new();
    let api = Api::new(&clock, []);

    assert_eq!(run(&repo, &api, &Git::real(&repo.path()), &clock), AttemptEnd::NotStarted);
    assert!(!store(&repo).exists());
    assert_eq!(api.calls.get(), 0);
}

#[test]
fn without_an_origin_or_a_default_branch_nothing_is_recorded() {
    let clock = Clock::new();
    let api = Api::new(&clock, []);

    let repo = TestRepo::new();
    assert_eq!(run(&repo, &api, &Git::real(&repo.path()), &clock), AttemptEnd::NotStarted);
    assert!(!store(&repo).exists());

    let repo = TestRepo::with_origin();
    repo.git(&["branch", "-m", "main", "trunk"]);
    assert_eq!(run(&repo, &api, &Git::real(&repo.path()), &clock), AttemptEnd::NotStarted);
    assert!(!store(&repo).exists());
    assert_eq!(api.calls.get(), 0);
}

#[test]
fn an_invalid_default_branch_name_fails_the_check_without_a_request() {
    let repo = TestRepo::with_origin();
    // A ref name git accepts, but not as a branch name.
    repo.git(&["update-ref", "refs/remotes/origin/-x", "main"]);
    repo.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/-x"]);
    let clock = Clock::new();
    let api = Api::new(&clock, []);
    let git = Git::real(&repo.path());

    assert_eq!(
        run(&repo, &api, &git, &clock),
        AttemptEnd::Finished(Outcome::CheckFailed { reason: CheckFailure::Other })
    );
    assert_eq!(api.calls.get() + git.live_calls() + git.fetch_calls(), 0);
    assert_eq!(stored(&repo).answer, None);
}

#[test]
fn a_store_that_cannot_be_written_ends_the_attempt_before_any_request() {
    let repo = TestRepo::with_origin();
    fs::create_dir_all(store(&repo)).unwrap();
    let clock = Clock::new();
    let api = Api::new(&clock, []);

    assert_eq!(run(&repo, &api, &Git::real(&repo.path()), &clock), AttemptEnd::WriteFailed);
    assert_eq!(api.calls.get(), 0);
    assert_eq!(AttemptEnd::WriteFailed.head_status(), HeadStatus::Failed);
}

#[test]
fn a_non_main_default_branch_is_checked_and_fetched() {
    let repo = TestRepo::with_origin();
    repo.git(&["push", "-q", "origin", "main:refs/heads/trunk"]);
    repo.git(&["fetch", "-q", "origin"]);
    repo.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"]);
    let main_before = repo.sha("origin/main");
    let pushed = repo.push_commit_to_origin("trunk", "trunk.txt");
    repo.push_commit_to_origin("main", "main.txt");
    let clock = Clock::new();
    let git = Git::real(&repo.path());

    assert_eq!(run(&repo, &SniffBranchHeads, &git, &clock), AttemptEnd::Finished(Outcome::Fetched));

    assert_eq!(repo.sha("origin/trunk"), pushed);
    assert_eq!(repo.sha("origin/main"), main_before, "only the default branch's ref");
    assert_eq!(stored(&repo).answer.unwrap().branch, "trunk");
}

#[test]
fn the_store_records_only_a_digest_of_the_origin() {
    let repo = TestRepo::with_origin();
    let secret = "https://user:hunter2@heads.example.invalid/o/r.git";
    repo.git(&["remote", "set-url", "origin", secret]);
    let clock = Clock::new();
    let tip = repo.sha("origin/main");
    let api = Api::new(&clock, [Ok(tip)]);

    assert_eq!(run(&repo, &api, &Git::real(&repo.path()), &clock), AttemptEnd::Finished(Outcome::InSync));

    let bytes = fs::read_to_string(store(&repo)).unwrap();
    assert!(!bytes.contains("hunter2") && !bytes.contains("example.invalid"), "{bytes}");
    assert!(bytes.contains(&origin_digest(secret)));
}

#[test]
fn successive_attempts_replace_the_answer_read_write_read() {
    let repo = TestRepo::with_origin();
    let clock = Clock::new();
    let git = Git::real(&repo.path());
    assert_eq!(run(&repo, &SniffBranchHeads, &git, &clock), AttemptEnd::Finished(Outcome::InSync));
    let first = stored(&repo).answer.unwrap();

    clock.advance(Duration::from_secs(30));
    let pushed = repo.push_commit_to_origin("main", "upstream.txt");
    // A second attempt with a fresh answer still asks: listing has no
    // freshness skip for the head (Rule 7).
    assert_eq!(run(&repo, &SniffBranchHeads, &git, &clock), AttemptEnd::Finished(Outcome::Fetched));
    let second = stored(&repo).answer.unwrap();

    assert_eq!(first.sha.as_deref(), Some(repo.sha("main").as_str()));
    assert_eq!(second, answer(&repo, Some(&pushed), NOW + 30, AnswerSource::Fetch));
    assert_eq!(AttemptEnd::Finished(Outcome::Fetched).head_status(), HeadStatus::Ok);
    assert_eq!(AttemptEnd::Finished(Outcome::CheckFailed { reason: CheckFailure::Other }).head_status(), HeadStatus::Failed);
}

#[test]
fn an_unauthorized_origin_fails_the_check_fast_as_credentials_and_keeps_the_answer() {
    use crate::live_remote::tests::{FAST, Loopback, http_origin};

    let server = Loopback::unauthorized();
    let repo = http_origin(&server);
    let old = seed_old_answer(&repo);
    let clock = Clock::new();
    let git = Git::real(&repo.path());

    let started = std::time::Instant::now();
    let end = run(&repo, &SniffBranchHeads, &git, &clock);

    assert_eq!(end, AttemptEnd::Finished(Outcome::CheckFailed { reason: CheckFailure::Credentials }));
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    assert!(server.accepted() >= 1, "git reached the origin");
    assert_eq!(stored(&repo).answer, Some(old));
}

fn credentials(repo: &TestRepo) -> CredentialEvidence {
    stored(repo).attempt.expect("an attempt record").credentials
}

#[test]
fn an_api_answer_records_its_credentials_before_the_answer_and_through_the_fetch() {
    let repo = TestRepo::with_origin();
    let pushed = repo.push_commit_to_origin("main", "upstream.txt");
    let clock = Clock::new();
    let mut api = Api::new(&clock, [Ok(pushed.clone())]);
    api.credentials = CredentialEvidence::Keyed { variables: vec!["SNIFF_GITHUB_GIT_2E_EXAMPLE_TOKEN".into()] };
    let during_fetch = RefCell::new(None);
    let git = Git::real(&repo.path()).before_fetch(|| *during_fetch.borrow_mut() = stored(&repo).attempt);

    assert_eq!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::Fetched));

    let fetching = during_fetch.borrow().clone().expect("the attempt while fetching");
    assert_eq!(fetching.phase, Phase::Fetching);
    assert_eq!(fetching.credentials, api.credentials, "already recorded while fetching");
    let state = stored(&repo);
    assert_eq!(state.answer.unwrap().source, AnswerSource::Fetch, "a `source: fetch` answer ...");
    assert_eq!(state.attempt.unwrap().credentials, api.credentials, "... keeps the API result's credentials");
}

#[test]
fn an_anonymous_api_answer_survives_a_fetch_failure_or_timeout() {
    for failure in [GitFailure::Timeout, GitFailure::Other] {
        let repo = TestRepo::with_origin();
        let pushed = repo.push_commit_to_origin("main", "upstream.txt");
        let clock = Clock::new();
        let api = Api::new(&clock, [Ok(pushed)]);
        let git = Git::real(&repo.path()).fetch_fails(failure);

        assert!(matches!(run(&repo, &api, &git, &clock), AttemptEnd::Finished(Outcome::FetchFailed { .. })));

        assert_eq!(credentials(&repo), CredentialEvidence::Anonymous, "{failure:?}");
    }
}

#[test]
fn a_check_the_api_did_not_answer_has_unknown_credentials() {
    // The fallback answered, the provider was unsupported, the repository is
    // ignored: no successful API request, so nothing to claim.
    let repo = TestRepo::with_origin();
    let clock = Clock::new();
    let api = Api::new(&clock, [Err(PrUnavailable::CredentialsRequired { key: None })]);
    assert_eq!(run(&repo, &api, &Git::real(&repo.path()), &clock), AttemptEnd::Finished(Outcome::InSync));
    assert_eq!(credentials(&repo), CredentialEvidence::Unknown, "fallback");

    let api = Api::unsupported(&clock);
    assert_eq!(run(&repo, &api, &Git::real(&repo.path()), &clock), AttemptEnd::Finished(Outcome::InSync));
    assert_eq!(credentials(&repo), CredentialEvidence::Unknown, "unsupported");

    let api = Api::new(&clock, []);
    assert_eq!(run_with(&repo, &api, &Git::real(&repo.path()), &clock, true), AttemptEnd::Finished(Outcome::InSync));
    assert_eq!(credentials(&repo), CredentialEvidence::Unknown, "ignored");
}
