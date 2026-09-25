//! Would deleting a branch lose work? The branch's tier depends on where its
//! last commit can be found; every earlier commit is part of that commit's
//! history, so wherever the tip is found, all of them are.
//!
//! - **Safe**: the tip is on the default branch (local or `origin/<default>`),
//!   or it is the exact source head of an open or merged PR on origin.
//! - **Pretty safe**: the tip is on another local branch, a tag, or an
//!   `origin/*` branch whose live head was verified.
//! - **Not safe**: nowhere else. **Unknown** (a git failure) is treated as
//!   Not safe.
//!
//! See item 3 of `2026-09-24-ux-improvements` for the full rules.

use std::path::Path;
use std::time::Duration;

use crate::default_target::{DefaultTarget, select_default_target};
use crate::git::git_from;

use super::live_remote::RemoteHeads;

/// The ruled PR-lookup deadline for `wt remove` (Decision 22).
pub const PR_DEADLINE: Duration = Duration::from_secs(2);

/// Where the branch's work was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    /// The default branch or its origin copy, by short name.
    DefaultBranch(String),
    PullRequest {
        number: u64,
        merged: bool,
        url: Option<String>,
    },
    LocalBranch(String),
    Tag(String),
    /// An `origin/*` ref whose live head was verified, as `origin/<name>`.
    RemoteBranch(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tier {
    Safe(Evidence),
    PrettySafe(Evidence),
    NotSafe,
    /// Safety could not be established; treated as Not safe.
    Unknown(String),
}

impl Tier {
    /// Safe or Pretty safe: the branch may be deleted without asking.
    pub fn allows_deletion(&self) -> bool {
        matches!(self, Tier::Safe(_) | Tier::PrettySafe(_))
    }

    pub fn evidence(&self) -> Option<&Evidence> {
        match self {
            Tier::Safe(evidence) | Tier::PrettySafe(evidence) => Some(evidence),
            Tier::NotSafe | Tier::Unknown(_) => None,
        }
    }
}

/// A commit named in a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub short_sha: String,
    pub subject: String,
}

/// A PR as far as the safety tiers need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub url: Option<String>,
    pub open: bool,
    /// The PR's source repository (`owner/repo`).
    pub source_repo: String,
    /// The source head exactly as the provider reported it; possibly a prefix.
    pub head_sha: Option<String>,
    pub target_branch: Option<String>,
}

/// The answer to "which PR was opened from this branch?".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrLookup {
    Found(PullRequest),
    NoneFound,
    /// Offline, no credentials, unsupported host, or the deadline passed.
    Unavailable(String),
}

/// Looks up the open or merged PR for one branch of origin.
///
/// A trait so tests can script provider answers.
pub trait PrSource: Sync {
    fn lookup(&self, branch: &str) -> PrLookup;
}

/// [`PrSource`] through sniff's blocking provider lookup, for the repository
/// behind `remote_url`.
#[derive(Debug, Clone)]
pub struct SniffPrSource {
    pub remote_url: String,
    pub deadline: Duration,
}

impl SniffPrSource {
    /// The source for the repository's `origin`, or `None` without one.
    pub fn for_origin(base: &Path) -> Option<Self> {
        let remote_url = git_from(base, base, &["remote", "get-url", "origin"]).ok()?;
        Some(Self {
            remote_url,
            deadline: PR_DEADLINE,
        })
    }

    /// `owner/repo` of origin, the repository a same-repo PR comes from.
    pub fn source_repo(&self) -> Option<String> {
        sniff::filesystem::git::repository_link(&self.remote_url).map(|link| link.owner_repo)
    }
}

impl PrSource for SniffPrSource {
    fn lookup(&self, branch: &str) -> PrLookup {
        use sniff::remote::blocking::{PrState, pull_request_for_branch};
        let Some(source_repo) = self.source_repo() else {
            return PrLookup::Unavailable("origin's URL names no repository".to_string());
        };
        match pull_request_for_branch(&self.remote_url, &source_repo, branch, self.deadline) {
            Ok(Some(evidence)) => PrLookup::Found(PullRequest {
                number: evidence.number,
                url: evidence.html_url,
                open: evidence.state == PrState::Open,
                source_repo: evidence.source_repo,
                head_sha: evidence.source_head_sha,
                target_branch: evidence.target_branch,
            }),
            Ok(None) => PrLookup::NoneFound,
            Err(reason) => PrLookup::Unavailable(reason.to_string()),
        }
    }
}

/// A [`PrSource`] that never has an answer, for callers without a remote.
pub struct NoPrSource;

impl PrSource for NoPrSource {
    fn lookup(&self, _branch: &str) -> PrLookup {
        PrLookup::Unavailable("no origin remote".to_string())
    }
}

/// The branch's copy on origin, as of the last fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginCopy {
    /// `origin/<name>`.
    pub reference: String,
    /// Local commits the copy lacks.
    pub missing_local: usize,
    /// Commits on the copy that the local branch lacks.
    pub extra_remote: usize,
}

/// Everything the report says about one branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSafety {
    pub tier: Tier,
    /// Evidence that was considered and rejected, in plain words (a remote
    /// copy that could not be verified, a PR whose head moved on).
    pub notes: Vec<String>,
    /// Commits on the branch that no other local branch, `origin/*` ref, or
    /// tag contains; `Err` when git could not list them.
    pub lost_commits: Result<Vec<Commit>, String>,
    pub target: Option<DefaultTarget>,
    /// `(ahead, behind)` against [`BranchSafety::target`].
    pub ahead_behind: Option<(usize, usize)>,
    pub origin_copy: Option<OriginCopy>,
    pub pr: PrLookup,
}

/// What to assess.
#[derive(Debug, Clone)]
pub struct SafetyInput<'a> {
    /// The main checkout; every git call runs against it.
    pub base: &'a Path,
    pub branch: &'a str,
    /// The branch's full tip SHA.
    pub tip: &'a str,
    pub default_branch: &'a str,
    /// The branch's name on origin (its upstream there, else its own name),
    /// or `None` without an origin remote.
    pub remote_branch: Option<&'a str>,
    /// `--force-remote`: leave out the branch's own origin copy and its open
    /// PR, since both are about to be deleted.
    pub force_remote: bool,
    /// `owner/repo` of origin; a PR from another repository never counts.
    pub source_repo: Option<&'a str>,
}

/// Classifies the branch and gathers the facts the report shows.
///
/// The PR lookup (network) runs alongside the git work.
pub fn assess(input: &SafetyInput<'_>, prs: &dyn PrSource, heads: &dyn RemoteHeads) -> BranchSafety {
    std::thread::scope(|scope| {
        let pr_handle = scope.spawn(|| prs.lookup(input.branch));
        let target = select_default_target(input.base, input.default_branch);
        let ahead_behind = target
            .as_ref()
            .and_then(|target| ahead_behind(input.base, &target.sha, input.tip));
        let origin_copy = input
            .remote_branch
            .and_then(|name| origin_copy(input.base, name, input.tip));
        let lost_commits = lost_commits(input);
        let pr = pr_handle
            .join()
            .unwrap_or_else(|_| PrLookup::Unavailable("PR lookup panicked".to_string()));
        let mut notes = Vec::new();
        let tier = classify(input, &pr, heads, &mut notes);
        BranchSafety {
            tier,
            notes,
            lost_commits,
            target,
            ahead_behind,
            origin_copy,
            pr,
        }
    })
}

fn classify(
    input: &SafetyInput<'_>,
    pr: &PrLookup,
    heads: &dyn RemoteHeads,
    notes: &mut Vec<String>,
) -> Tier {
    let containing = match git_from(
        input.base,
        input.base,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "--contains",
            input.tip,
            "refs/heads",
            "refs/remotes/origin",
            "refs/tags",
        ],
    ) {
        Ok(out) => out,
        Err(error) => return Tier::Unknown(format!("could not search refs: {error}")),
    };
    let own_local = format!("refs/heads/{}", input.branch);
    let own_remote = input
        .remote_branch
        .map(|name| format!("refs/remotes/origin/{name}"));
    let default_local = format!("refs/heads/{}", input.default_branch);
    let default_remote = format!("refs/remotes/origin/{}", input.default_branch);
    let refs: Vec<&str> = containing
        .lines()
        .filter(|name| *name != own_local && *name != "refs/remotes/origin/HEAD")
        .collect();

    // Pass 1 -- Safe.
    for default in [&default_local, &default_remote] {
        if refs.contains(&default.as_str()) {
            return Tier::Safe(Evidence::DefaultBranch(short_ref(default)));
        }
    }
    if let Some(evidence) = pr_evidence(input, pr, notes) {
        return Tier::Safe(evidence);
    }

    // Pass 2 -- Pretty safe. Local refs and tags need no network.
    if let Some(branch) = refs.iter().find_map(|name| name.strip_prefix("refs/heads/")) {
        return Tier::PrettySafe(Evidence::LocalBranch(branch.to_string()));
    }
    if let Some(tag) = refs.iter().find_map(|name| name.strip_prefix("refs/tags/")) {
        return Tier::PrettySafe(Evidence::Tag(tag.to_string()));
    }

    // Remote-tracking refs are last-fetch observations; each must pass the
    // live check. The branch's own copy goes first, being the likeliest.
    let mut remote_refs: Vec<&str> = refs
        .iter()
        .copied()
        .filter(|name| name.starts_with("refs/remotes/origin/"))
        .filter(|name| !(input.force_remote && Some(*name) == own_remote.as_deref()))
        .collect();
    remote_refs.sort_by_key(|name| Some(*name) != own_remote.as_deref());
    for full_ref in remote_refs {
        let name = &full_ref["refs/remotes/origin/".len()..];
        let tracking = match git_from(input.base, input.base, &["rev-parse", full_ref]) {
            Ok(sha) => sha,
            Err(_) => continue,
        };
        match heads.live_head(name) {
            Ok(Some(live)) if live == input.tip || live == tracking => {
                return Tier::PrettySafe(Evidence::RemoteBranch(format!("origin/{name}")));
            }
            Ok(Some(_)) => notes.push(format!(
                "origin/{name} has moved since your last fetch, so it could not be verified"
            )),
            Ok(None) => notes.push(format!("origin/{name} no longer exists on origin")),
            Err(reason) => {
                notes.push(format!(
                    "origin/{name} could not be verified: {reason}"
                ));
                // Every other remote ref would hit the same wall.
                break;
            }
        }
    }
    Tier::NotSafe
}

/// A PR counts only when it is open (and not about to be closed by
/// `--force-remote`) or merged, comes from origin's own repository, and its
/// recorded source head is exactly the local tip.
fn pr_evidence(input: &SafetyInput<'_>, pr: &PrLookup, notes: &mut Vec<String>) -> Option<Evidence> {
    let PrLookup::Found(pr) = pr else {
        return None;
    };
    if pr.open && input.force_remote {
        return None;
    }
    let same_repo = input
        .source_repo
        .is_some_and(|repo| repo.eq_ignore_ascii_case(&pr.source_repo));
    if !same_repo {
        notes.push(format!(
            "PR #{} comes from {}, not this repository",
            pr.number, pr.source_repo
        ));
        return None;
    }
    let matches = pr
        .head_sha
        .as_deref()
        .is_some_and(|head| head_matches_tip(input.base, head, input.tip));
    if !matches {
        let state = if pr.open { "open" } else { "merged" };
        notes.push(format!(
            "{state} PR #{} does not include your latest commit",
            pr.number
        ));
        return None;
    }
    Some(Evidence::PullRequest {
        number: pr.number,
        merged: !pr.open,
        url: pr.url.clone(),
    })
}

/// Decision 27: a full-length head must equal the tip. A shorter one counts
/// only when it is at least 7 lowercase hex characters, the tip starts with
/// it, and it names exactly one object in the local repository.
pub fn head_matches_tip(base: &Path, head: &str, tip: &str) -> bool {
    if head.len() >= tip.len() {
        return head == tip;
    }
    let is_lower_hex = head.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if head.len() < 7 || !is_lower_hex || !tip.starts_with(head) {
        return false;
    }
    git_from(base, base, &["rev-parse", &format!("--disambiguate={head}")])
        .is_ok_and(|out| out.lines().count() == 1)
}

fn short_ref(full_ref: &str) -> String {
    full_ref
        .strip_prefix("refs/heads/")
        .or_else(|| full_ref.strip_prefix("refs/remotes/"))
        .unwrap_or(full_ref)
        .to_string()
}

/// `(ahead, behind)` of `tip` against `target`.
fn ahead_behind(base: &Path, target: &str, tip: &str) -> Option<(usize, usize)> {
    let range = format!("{target}...{tip}");
    let out = git_from(base, base, &["rev-list", "--left-right", "--count", &range]).ok()?;
    let mut parts = out.split_whitespace().map(|n| n.parse::<usize>().ok());
    let behind = parts.next()??;
    let ahead = parts.next()??;
    Some((ahead, behind))
}

fn origin_copy(base: &Path, name: &str, tip: &str) -> Option<OriginCopy> {
    let reference = format!("origin/{name}");
    let sha = git_from(
        base,
        base,
        &["rev-parse", "--verify", "--quiet", &format!("refs/remotes/{reference}")],
    )
    .ok()?;
    let (missing_local, extra_remote) = ahead_behind(base, &sha, tip)?;
    Some(OriginCopy {
        reference,
        missing_local,
        extra_remote,
    })
}

/// Commits on the branch that no other local branch, `origin/*` ref, or tag
/// contains; with `--force-remote`, the branch's own origin copy does not
/// count either.
fn lost_commits(input: &SafetyInput<'_>) -> Result<Vec<Commit>, String> {
    let exclude_local = format!("--exclude={}", input.branch);
    let exclude_remote = input
        .remote_branch
        .filter(|_| input.force_remote)
        .map(|name| format!("--exclude=origin/{name}"));
    let mut args = vec![
        "log",
        "--format=%h%x1f%s",
        input.tip,
        "--not",
        &exclude_local,
        "--branches",
    ];
    if let Some(exclude) = exclude_remote.as_deref() {
        args.push(exclude);
    }
    args.extend(["--remotes=origin", "--tags"]);
    let out = git_from(input.base, input.base, &args).map_err(|e| e.to_string())?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let (short_sha, subject) = line.split_once('\u{1f}')?;
            Some(Commit {
                short_sha: short_sha.to_string(),
                subject: subject.to_string(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::remove::live_remote::{LIVE_CHECK_DEADLINE, LsRemote};
    use crate::remove::test_support::TestRepo;

    struct StubPr(PrLookup);
    impl PrSource for StubPr {
        fn lookup(&self, _branch: &str) -> PrLookup {
            self.0.clone()
        }
    }

    /// Answers every live check with the same result and counts the calls.
    struct StubHeads {
        answer: Result<Option<String>, String>,
        calls: Mutex<Vec<String>>,
    }
    impl StubHeads {
        fn new(answer: Result<Option<String>, String>) -> Self {
            Self {
                answer,
                calls: Mutex::new(Vec::new()),
            }
        }
    }
    impl RemoteHeads for StubHeads {
        fn live_head(&self, branch: &str) -> Result<Option<String>, String> {
            self.calls.lock().unwrap().push(branch.to_string());
            self.answer.clone()
        }
    }

    const REPO: &str = "acme/widgets";

    fn pr(open: bool, head: Option<&str>, source_repo: &str) -> PrLookup {
        PrLookup::Found(PullRequest {
            number: 99,
            url: Some("https://github.com/acme/widgets/pull/99".into()),
            open,
            source_repo: source_repo.into(),
            head_sha: head.map(str::to_string),
            target_branch: Some("main".into()),
        })
    }

    /// A worktree branch `feat/x` with one commit of its own.
    fn feature(repo: &TestRepo) -> String {
        let wt = repo.add_worktree("feat/x", "feat-x", "main");
        repo.commit_in(&wt, "feature.txt")
    }

    fn assess_with(
        repo: &TestRepo,
        tip: &str,
        force_remote: bool,
        prs: PrLookup,
        heads: &dyn RemoteHeads,
    ) -> BranchSafety {
        let base = repo.path();
        let input = SafetyInput {
            base: &base,
            branch: "feat/x",
            tip,
            default_branch: "main",
            remote_branch: Some("feat/x"),
            force_remote,
            source_repo: Some(REPO),
        };
        assess(&input, &StubPr(prs), heads)
    }

    fn live(repo: &TestRepo) -> LsRemote<'static> {
        LsRemote {
            base: Box::leak(Box::new(repo.path())),
            deadline: LIVE_CHECK_DEADLINE,
        }
    }

    #[test]
    fn merged_into_local_main_is_safe() {
        let repo = TestRepo::new();
        let tip = feature(&repo);
        repo.git(&["merge", "-q", "--no-ff", "-m", "merge", "feat/x"]);
        let heads = StubHeads::new(Err("offline".into()));
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &heads);
        assert_eq!(safety.tier, Tier::Safe(Evidence::DefaultBranch("main".into())));
        assert_eq!(safety.lost_commits, Ok(Vec::new()));
        assert!(heads.calls.lock().unwrap().is_empty(), "no network needed");
    }

    #[test]
    fn merged_into_origin_main_only_is_safe_without_a_live_check() {
        let repo = TestRepo::with_origin();
        let tip = feature(&repo);
        // Someone merged it on origin; local main is behind.
        repo.git(&["push", "-q", "origin", "feat/x:main"]);
        repo.git(&["fetch", "-q", "origin"]);
        let heads = StubHeads::new(Err("offline".into()));
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &heads);
        assert_eq!(safety.tier, Tier::Safe(Evidence::DefaultBranch("origin/main".into())));
        assert!(heads.calls.lock().unwrap().is_empty());
        let target = safety.target.unwrap();
        assert_eq!(target.reference, "origin/main");
        assert_eq!(safety.ahead_behind, Some((0, 0)));
    }

    #[test]
    fn a_pr_counts_only_for_the_same_repository_and_exact_tip() {
        let repo = TestRepo::new();
        let tip = feature(&repo);
        let heads = StubHeads::new(Ok(None));
        let expected_merged = Tier::Safe(Evidence::PullRequest {
            number: 99,
            merged: true,
            url: Some("https://github.com/acme/widgets/pull/99".into()),
        });

        // Merged PR, exact tip (a squash merge left no ancestry).
        let safety = assess_with(&repo, &tip, false, pr(false, Some(&tip), REPO), &heads);
        assert_eq!(safety.tier, expected_merged);

        // Open PR, exact tip; the repository name compares case-insensitively.
        let safety = assess_with(&repo, &tip, false, pr(true, Some(&tip), "Acme/Widgets"), &heads);
        assert!(matches!(
            safety.tier,
            Tier::Safe(Evidence::PullRequest { merged: false, .. })
        ));

        // A fork's PR from a same-named branch never counts.
        let safety = assess_with(&repo, &tip, false, pr(true, Some(&tip), "mallory/widgets"), &heads);
        assert_eq!(safety.tier, Tier::NotSafe);
        assert!(safety.notes.iter().any(|n| n.contains("mallory/widgets")));

        // Commits made after the merge: the recorded head is an ancestor, not the tip.
        let parent = repo.sha("main");
        let safety = assess_with(&repo, &tip, false, pr(false, Some(&parent), REPO), &heads);
        assert_eq!(safety.tier, Tier::NotSafe);
        assert!(safety.notes.iter().any(|n| n.contains("latest commit")));

        // A provider that sent no head grants nothing.
        let safety = assess_with(&repo, &tip, false, pr(false, None, REPO), &heads);
        assert_eq!(safety.tier, Tier::NotSafe);

        // An abbreviated head (Bitbucket's 12 characters) that is a unique prefix.
        let safety = assess_with(&repo, &tip, false, pr(false, Some(&tip[..12]), REPO), &heads);
        assert_eq!(safety.tier, expected_merged);

        // Too short, or not lowercase hex.
        for head in [&tip[..6], &tip[..12].to_uppercase()] {
            let safety = assess_with(&repo, &tip, false, pr(false, Some(head), REPO), &heads);
            assert_eq!(safety.tier, Tier::NotSafe, "{head}");
        }
    }

    #[test]
    fn an_unavailable_pr_answer_falls_through_to_lower_tiers() {
        let repo = TestRepo::new();
        let tip = feature(&repo);
        repo.git(&["tag", "v1", &tip]);
        let heads = StubHeads::new(Ok(None));
        let safety = assess_with(&repo, &tip, false, PrLookup::Unavailable("offline".into()), &heads);
        assert_eq!(safety.tier, Tier::PrettySafe(Evidence::Tag("v1".into())));
    }

    #[test]
    fn force_remote_ignores_the_open_pr_but_not_a_merged_one() {
        let repo = TestRepo::new();
        let tip = feature(&repo);
        let heads = StubHeads::new(Ok(None));
        let open = assess_with(&repo, &tip, true, pr(true, Some(&tip), REPO), &heads);
        assert_eq!(open.tier, Tier::NotSafe);
        let merged = assess_with(&repo, &tip, true, pr(false, Some(&tip), REPO), &heads);
        assert!(matches!(merged.tier, Tier::Safe(Evidence::PullRequest { merged: true, .. })));
    }

    #[test]
    fn another_local_branch_or_tag_is_pretty_safe_offline() {
        let repo = TestRepo::new();
        let tip = feature(&repo);
        repo.git(&["branch", "feat/theme", &tip]);
        let heads = StubHeads::new(Err("offline".into()));
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &heads);
        assert_eq!(safety.tier, Tier::PrettySafe(Evidence::LocalBranch("feat/theme".into())));
        assert_eq!(safety.lost_commits, Ok(Vec::new()));
        assert_eq!(safety.ahead_behind, Some((1, 0)));
    }

    #[test]
    fn own_origin_copy_is_pretty_safe_after_the_live_check() {
        let repo = TestRepo::with_origin();
        let tip = feature(&repo);
        repo.git(&["push", "-q", "origin", "feat/x"]);
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &live(&repo));
        assert_eq!(safety.tier, Tier::PrettySafe(Evidence::RemoteBranch("origin/feat/x".into())));
        assert_eq!(
            safety.origin_copy,
            Some(OriginCopy {
                reference: "origin/feat/x".into(),
                missing_local: 0,
                extra_remote: 0
            })
        );
    }

    #[test]
    fn a_remote_copy_that_moved_on_with_the_tip_in_it_still_qualifies_only_if_tracked() {
        let repo = TestRepo::with_origin();
        let tip = feature(&repo);
        repo.git(&["push", "-q", "origin", "feat/x"]);
        // Someone pushed on top; our tracking ref is stale.
        repo.push_commit_to_origin("feat/x", "theirs.txt");
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &live(&repo));
        assert_eq!(safety.tier, Tier::NotSafe, "live SHA equals neither tip nor tracking SHA");
        assert!(safety.notes.iter().any(|n| n.contains("moved since your last fetch")));

        // After a fetch the tracking ref equals the live head and contains the tip.
        repo.git(&["fetch", "-q", "origin"]);
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &live(&repo));
        assert_eq!(safety.tier, Tier::PrettySafe(Evidence::RemoteBranch("origin/feat/x".into())));
    }

    #[test]
    fn a_stale_or_deleted_origin_ref_never_justifies_deletion() {
        let repo = TestRepo::with_origin();
        let tip = feature(&repo);
        repo.git(&["push", "-q", "origin", "feat/x"]);
        // Deleted on origin; the tracking ref survives until a prune.
        repo.git_in(&repo.origin_path(), &["branch", "-D", "feat/x"]);
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &live(&repo));
        assert_eq!(safety.tier, Tier::NotSafe);
        assert!(safety.notes.iter().any(|n| n.contains("no longer exists")));
    }

    #[test]
    fn an_unreachable_remote_keeps_the_branch() {
        let repo = TestRepo::with_origin();
        let tip = feature(&repo);
        repo.git(&["push", "-q", "origin", "feat/x"]);
        let heads = StubHeads::new(Err("origin did not answer within 3 s".into()));
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &heads);
        assert_eq!(safety.tier, Tier::NotSafe);
        assert!(safety.notes.iter().any(|n| n.contains("could not be verified")));
    }

    #[test]
    fn force_remote_excludes_the_own_origin_copy_from_tier_and_lost_commits() {
        let repo = TestRepo::with_origin();
        let tip = feature(&repo);
        repo.git(&["push", "-q", "origin", "feat/x"]);
        let heads = StubHeads::new(Ok(Some(tip.clone())));
        let safety = assess_with(&repo, &tip, true, PrLookup::NoneFound, &heads);
        assert_eq!(safety.tier, Tier::NotSafe);
        assert!(heads.calls.lock().unwrap().is_empty());
        assert_eq!(safety.lost_commits.unwrap().len(), 1);

        let without = assess_with(&repo, &tip, false, PrLookup::NoneFound, &heads);
        assert_eq!(without.lost_commits, Ok(Vec::new()));
    }

    #[test]
    fn another_origin_branch_is_verified_live() {
        let repo = TestRepo::with_origin();
        let tip = feature(&repo);
        repo.git(&["push", "-q", "origin", "feat/x:refs/heads/backup"]);
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &live(&repo));
        assert_eq!(safety.tier, Tier::PrettySafe(Evidence::RemoteBranch("origin/backup".into())));
    }

    #[test]
    fn unique_commits_are_not_safe_and_listed() {
        let repo = TestRepo::new();
        let wt = repo.add_worktree("feat/x", "feat-x", "main");
        repo.commit_in(&wt, "one.txt");
        let tip = repo.commit_in(&wt, "two.txt");
        let heads = StubHeads::new(Ok(None));
        let safety = assess_with(&repo, &tip, false, PrLookup::NoneFound, &heads);
        assert_eq!(safety.tier, Tier::NotSafe);
        let subjects: Vec<_> = safety.lost_commits.unwrap().into_iter().map(|c| c.subject).collect();
        assert_eq!(subjects, ["add two.txt", "add one.txt"]);
    }

    /// Item 4's original bug: the branch is merged into HEAD (local `main`)
    /// but not into its upstream. `git branch -d` refused it; the tiers say
    /// Safe, so it is deleted.
    #[test]
    fn merged_into_head_but_not_its_upstream_is_safe() {
        let repo = TestRepo::with_origin();
        feature(&repo);
        repo.git(&["push", "-q", "-u", "origin", "feat/x"]);
        let wt = repo.path().parent().unwrap().join("wts").join("feat-x");
        let newer = repo.commit_in(&wt, "after-push.txt");
        repo.git(&["merge", "-q", "--no-ff", "-m", "merge", "feat/x"]);
        let heads = StubHeads::new(Err("offline".into()));
        let safety = assess_with(&repo, &newer, false, PrLookup::NoneFound, &heads);
        assert_eq!(safety.tier, Tier::Safe(Evidence::DefaultBranch("main".into())));
        assert_eq!(safety.lost_commits, Ok(Vec::new()));
    }

    #[test]
    fn a_ref_search_failure_is_unknown() {
        let repo = TestRepo::new();
        let heads = StubHeads::new(Ok(None));
        let bogus = "0123456789abcdef0123456789abcdef01234567";
        let safety = assess_with(&repo, bogus, false, PrLookup::NoneFound, &heads);
        assert!(matches!(safety.tier, Tier::Unknown(_)), "{:?}", safety.tier);
        assert!(!safety.tier.allows_deletion());
    }
}
