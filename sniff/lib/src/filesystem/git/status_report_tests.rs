//! Focused status preparation through `detect_repo_git_status`.
//!
//! Every fixture is a temporary git2 repository; none reads this checkout.
//! The URLs name GitHub only so links have a browser form; no test touches
//! the network.

use super::*;
use crate::filesystem::detect_repo_git_status;
use crate::performance::{PerformanceCollector, PerformanceReport, counters, with_current_collector};
use git2::{Repository, Signature, Time};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

const ORIGIN: &str = "https://github.com/o/r.git";

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
    repo: Repository,
    clock: i64,
}

impl Fixture {
    /// A repository with `origin` configured and no commits.
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("main");
        let repo = Repository::init(&root).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        repo.remote("origin", ORIGIN).unwrap();
        Self {
            _dir: dir,
            root,
            repo,
            clock: 1_700_000_000,
        }
    }

    fn temp_path(&self, name: &str) -> PathBuf {
        self._dir.path().join(name)
    }

    /// Write `files` into `workdir`, stage everything, and commit on its HEAD.
    fn commit_in(&mut self, workdir: &Path, files: &[(&str, &str)], message: &str) -> String {
        let repo = Repository::open(workdir).unwrap();
        for (path, content) in files {
            let full = workdir.join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, content).unwrap();
        }
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        // Distinct, increasing times keep newest-first history deterministic.
        self.clock += 60;
        let signature = Signature::new("Test", "test@example.com", &Time::new(self.clock, 0)).unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)
            .unwrap()
            .to_string()
    }

    fn commit(&mut self, files: &[(&str, &str)], message: &str) -> String {
        let root = self.root.clone();
        self.commit_in(&root, files, message)
    }

    fn set_remote_tip(&self, branch: &str, sha: &str) {
        self.repo
            .reference(
                &format!("refs/remotes/origin/{branch}"),
                git2::Oid::from_str(sha).unwrap(),
                true,
                "test remote tip",
            )
            .unwrap();
    }

    fn branch(&self, name: &str, sha: &str) {
        let commit = self
            .repo
            .find_commit(git2::Oid::from_str(sha).unwrap())
            .unwrap();
        self.repo.branch(name, &commit, true).unwrap();
    }

    /// A linked worktree `name` on a new branch `name` at the current HEAD,
    /// checked out in a directory named `<name>-dir`.
    fn worktree(&self, name: &str) -> PathBuf {
        let path = self.temp_path(&format!("{name}-dir"));
        self.repo.worktree(name, &path, None).unwrap();
        path
    }
}

fn url(sha: &str) -> String {
    format!("https://github.com/o/r/commit/{sha}")
}

fn linked(sha: &str) -> CommitLink {
    CommitLink {
        remote: Some(true),
        commit_url: Some(url(sha)),
    }
}

fn absent() -> CommitLink {
    CommitLink {
        remote: Some(false),
        commit_url: None,
    }
}

fn options(history_count: usize) -> GitStatusOptions<'static> {
    GitStatusOptions {
        history_count,
        include_commit_links: true,
        ..GitStatusOptions::default()
    }
}

fn prepare(root: &Path, options: &GitStatusOptions<'_>) -> (Result<PreparedGitStatus>, PerformanceReport) {
    let collector = PerformanceCollector::new_shared();
    let result = with_current_collector(Some(collector.clone()), || {
        detect_repo_git_status(root, options)
    });
    (result, collector.snapshot(Duration::ZERO))
}

fn counter(report: &PerformanceReport, name: &str) -> u64 {
    report.counters.get(name).copied().unwrap_or(0)
}

fn shas(git: &GitInfo) -> Vec<&str> {
    git.recent.iter().map(|commit| commit.sha.as_str()).collect()
}

/// The links, which must be keyed by exactly the displayed IDs.
fn links_for<'a>(prepared: &'a PreparedGitStatus, displayed: &[&str]) -> Vec<&'a CommitLink> {
    let mut keys: Vec<&str> = prepared.commit_links.keys().map(String::as_str).collect();
    keys.sort();
    let mut expected = displayed.to_vec();
    expected.sort();
    assert_eq!(keys, expected, "links are keyed by the displayed full IDs");
    displayed
        .iter()
        .map(|sha| &prepared.commit_links[*sha])
        .collect()
}

fn selection_error(result: Result<PreparedGitStatus>) -> String {
    match result {
        Err(SniffError::RepositorySelection(message)) => message,
        other => panic!("expected a selection error, got {other:?}"),
    }
}

/// One pushed and one unpushed commit, with the unpushed one at HEAD.
fn pushed_and_unpushed() -> (Fixture, String, String) {
    let mut fixture = Fixture::new();
    let pushed = fixture.commit(&[("README.md", "one")], "pushed");
    fixture.set_remote_tip("main", &pushed);
    let unpushed = fixture.commit(&[("README.md", "two")], "unpushed");
    (fixture, pushed, unpushed)
}

#[test]
fn default_status_links_the_displayed_history_with_one_discovery_and_ref_walk() {
    let (fixture, pushed, unpushed) = pushed_and_unpushed();

    let (prepared, report) = prepare(&fixture.root, &options(10));
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();

    assert_eq!(shas(git), [unpushed.as_str(), pushed.as_str()]);
    assert_eq!(links_for(&prepared, &[&unpushed, &pushed]), [&absent(), &linked(&pushed)]);
    assert_eq!(counter(&report, counters::GIT_DISCOVERIES), 1, "{:?}", report.counters);
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 1, "{:?}", report.counters);
    for stage in [
        "filesystem.git.initial",
        "filesystem.git.selection",
        "filesystem.git.commit_links",
        "detect.filesystem",
        "detect.total",
    ] {
        assert!(report.stages.contains_key(stage), "{stage}: {:?}", report.stages.keys());
    }
    assert!(
        !report.stages.contains_key("filesystem.git"),
        "the focused parent stage is the sum of its children"
    );
}

#[test]
fn detached_head_still_observes_remote_tips_in_the_one_ref_walk() {
    let (fixture, pushed, unpushed) = pushed_and_unpushed();
    fixture
        .repo
        .set_head_detached(git2::Oid::from_str(&unpushed).unwrap())
        .unwrap();

    let (prepared, report) = prepare(&fixture.root, &options(10));
    let prepared = prepared.unwrap();

    assert_eq!(prepared.git.as_ref().unwrap().current_branch, None);
    assert_eq!(links_for(&prepared, &[&unpushed, &pushed]), [&absent(), &linked(&pushed)]);
    assert_eq!(counter(&report, counters::GIT_DISCOVERIES), 1);
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 1);
}

#[test]
fn disabled_or_zero_count_linking_runs_no_linking_stage() {
    let (fixture, _pushed, _unpushed) = pushed_and_unpushed();

    for options in [
        GitStatusOptions {
            include_commit_links: false,
            ..options(10)
        },
        options(0),
    ] {
        let (prepared, report) = prepare(&fixture.root, &options);
        let prepared = prepared.unwrap();
        assert!(prepared.commit_links.is_empty(), "{options:?}");
        assert!(!report.stages.contains_key("filesystem.git.commit_links"), "{options:?}");
        // The branch list still takes the one snapshot; only linking is skipped.
        assert_eq!(counter(&report, counters::GIT_REF_WALKS), 1, "{options:?}");
    }
}

#[test]
fn a_bare_or_current_branch_keeps_the_selection_and_another_branch_replaces_it() {
    let (fixture, pushed, unpushed) = pushed_and_unpushed();
    let base = pushed.clone();
    fixture.branch("other", &base);
    std::fs::write(fixture.root.join("dirty.txt"), "dirty").unwrap();
    let current = fixture.repo.head().unwrap().shorthand().unwrap().to_string();

    for branch in [None, Some(""), Some(current.as_str())] {
        let (prepared, _) = prepare(
            &fixture.root,
            &GitStatusOptions {
                branch,
                ..options(10)
            },
        );
        let prepared = prepared.unwrap();
        let git = prepared.git.as_ref().unwrap();
        assert_eq!(shas(git), [unpushed.as_str(), pushed.as_str()], "{branch:?}");
        assert_eq!(git.status.as_ref().unwrap().untracked_count, 1, "{branch:?}");
    }

    let (prepared, report) = prepare(
        &fixture.root,
        &GitStatusOptions {
            branch: Some("other"),
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();
    assert_eq!(shas(git), [pushed.as_str()]);
    assert_eq!(links_for(&prepared, &[&pushed]), [&linked(&pushed)]);
    let status = git.status.as_ref().unwrap();
    assert!(!status.is_dirty && status.untracked.is_empty() && git.file_changes.is_empty());
    // The captured tips and decorations serve the branch walk.
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 1);
    assert!(git.recent[0].refs.iter().any(|r| r.name == "other"));

    // A remote-tracking name resolves from the captured tips too.
    let (prepared, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            branch: Some("origin/main"),
            ..options(10)
        },
    );
    assert_eq!(shas(prepared.unwrap().git.as_ref().unwrap()), [pushed.as_str()]);
}

/// Three packages in two areas; each commit touches one package.
fn monorepo() -> (Fixture, [String; 3]) {
    let mut fixture = Fixture::new();
    let manifest = |name: &str| format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
    fixture.commit(
        &[
            ("Cargo.toml", "[workspace]\nmembers = [\"alpha/core\", \"alpha/cli\", \"beta/core\"]\n"),
            ("alpha/core/Cargo.toml", &manifest("alpha-core")),
            ("alpha/core/src/lib.rs", ""),
            ("alpha/cli/Cargo.toml", &manifest("alpha-cli")),
            ("alpha/cli/src/lib.rs", ""),
            ("beta/core/Cargo.toml", &manifest("beta-core")),
            ("beta/core/src/lib.rs", ""),
        ],
        "initial monorepo",
    );
    let alpha_core = fixture.commit(&[("alpha/core/src/lib.rs", "pub fn a() {}")], "alpha core");
    fixture.set_remote_tip("main", &alpha_core);
    let alpha_cli = fixture.commit(&[("alpha/cli/src/lib.rs", "pub fn b() {}")], "alpha cli");
    let beta_core = fixture.commit(&[("beta/core/src/lib.rs", "pub fn c() {}")], "beta core");
    (fixture, [alpha_core, alpha_cli, beta_core])
}

#[test]
fn package_and_area_select_path_history_and_narrow_the_working_tree() {
    let (fixture, [alpha_core, alpha_cli, beta_core]) = monorepo();
    std::fs::write(fixture.root.join("alpha/cli/new.txt"), "x").unwrap();
    std::fs::write(fixture.root.join("beta/core/new.txt"), "y").unwrap();

    let (prepared, report) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("ALPHA-CORE"),
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();
    // The initial commit touched the package too.
    assert_eq!(git.recent.len(), 2);
    assert_eq!(git.recent[0].sha, alpha_core);
    let displayed: Vec<&str> = shas(git);
    let links = links_for(&prepared, &displayed);
    assert_eq!(links[0], &linked(&alpha_core));
    assert!(!prepared.commit_links.contains_key(&alpha_cli) && !prepared.commit_links.contains_key(&beta_core));
    assert_eq!(git.status.as_ref().unwrap().untracked_count, 0);
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 1, "path history reuses the decorations");

    let (prepared, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package_area: Some("alp"),
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();
    assert_eq!(&shas(git)[..2], [alpha_cli.as_str(), alpha_core.as_str()]);
    assert_eq!(git.status.as_ref().unwrap().untracked_count, 1);
    let changed: Vec<PathBuf> = git.file_changes.iter().map(|change| change.path.clone()).collect();
    assert_eq!(changed, [PathBuf::from("alpha/cli/new.txt")]);

    // Package inside the area: the narrower package history.
    let (prepared, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("alpha-cli"),
            package_area: Some("Alpha"),
            ..options(10)
        },
    );
    assert_eq!(prepared.unwrap().git.unwrap().recent[0].sha, alpha_cli);

    let (outside, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("beta-core"),
            package_area: Some("alpha"),
            ..options(10)
        },
    );
    assert_eq!(selection_error(outside), "Package 'beta-core' is in area 'beta', not 'alpha'");
    let (unknown, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("gamma"),
            ..options(10)
        },
    );
    assert_eq!(
        selection_error(unknown),
        "Package 'gamma' not found.\n\nValid package names: alpha-cli, alpha-core, beta-core"
    );
    let (unknown_area, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package_area: Some("gamma"),
            ..options(10)
        },
    );
    assert_eq!(
        selection_error(unknown_area),
        "Package area 'gamma' not found.\n\nValid package areas: alpha, beta"
    );
}

#[test]
fn selectors_are_validated_outside_a_repository() {
    let dir = TempDir::new().unwrap();
    let (result, _) = prepare(
        dir.path(),
        &GitStatusOptions {
            package: Some("anything"),
            ..options(10)
        },
    );
    assert_eq!(selection_error(result), "No packages found in this repository");

    let (result, _) = prepare(dir.path(), &options(10));
    let prepared = result.unwrap();
    assert!(prepared.git.is_none() && prepared.commit_links.is_empty());
}

#[test]
fn zero_history_keeps_every_matching_path_commit_and_links_none() {
    let (fixture, [alpha_core, ..]) = monorepo();

    let (prepared, report) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("alpha-core"),
            ..options(0)
        },
    );
    let prepared = prepared.unwrap();
    // Uncapped, as `PathHistoryOptions::new(0)` always was.
    assert_eq!(prepared.git.as_ref().unwrap().recent.len(), 2);
    assert_eq!(prepared.git.as_ref().unwrap().recent[0].sha, alpha_core);
    assert!(prepared.commit_links.is_empty());
    assert!(!report.stages.contains_key("filesystem.git.commit_links"));
    // No decorations were captured, so the path reader read its own.
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 2, "{:?}", report.counters);
}

#[test]
fn a_selected_worktree_supplies_history_status_and_head_from_shared_refs() {
    let (mut fixture, pushed, unpushed) = pushed_and_unpushed();
    std::fs::write(fixture.root.join("main-only.txt"), "x").unwrap();
    let worktree = fixture.worktree("feature");
    let feature = fixture.commit_in(&worktree, &[("feature.txt", "f")], "feature work");
    std::fs::write(worktree.join("wt-untracked.txt"), "y").unwrap();

    let (prepared, report) = prepare(
        &fixture.root,
        &GitStatusOptions {
            worktree: Some("feature"),
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();

    assert_eq!(shas(git), [feature.as_str(), unpushed.as_str(), pushed.as_str()]);
    assert_eq!(
        links_for(&prepared, &[&feature, &unpushed, &pushed]),
        [&absent(), &absent(), &linked(&pushed)]
    );
    assert_eq!(git.current_branch.as_deref(), Some("feature"));
    assert!(git.repo_root.ends_with("feature-dir"), "{:?}", git.repo_root);
    assert_eq!(git.status.as_ref().unwrap().untracked_count, 1);
    let changed: Vec<PathBuf> = git.file_changes.iter().map(|change| change.path.clone()).collect();
    assert_eq!(changed, [PathBuf::from("wt-untracked.txt")]);
    // `HEAD` is re-marked for the selected worktree.
    let head = &git.recent[0].refs;
    assert!(head.iter().any(|r| r.name == "feature" && r.is_head), "{head:?}");
    let main_tip = &git.recent[1].refs;
    assert!(main_tip.iter().all(|r| !r.is_head), "{main_tip:?}");
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 1, "shared refs: {:?}", report.counters);
    assert_eq!(counter(&report, counters::GIT_DISCOVERIES), 2, "initial plus the selected worktree");

    // The directory basename selects it too; an unknown name lists the others.
    let (by_dir, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            worktree: Some("feature-dir"),
            ..options(10)
        },
    );
    assert_eq!(by_dir.unwrap().git.unwrap().current_branch.as_deref(), Some("feature"));
    let (missing, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            worktree: Some("nope"),
            ..options(10)
        },
    );
    let message = selection_error(missing);
    assert!(message.starts_with("Worktree not found: nope (available: "), "{message}");
}

#[test]
fn worktree_config_supplies_the_selected_worktrees_urls_over_shared_refs() {
    let (fixture, pushed, _unpushed) = pushed_and_unpushed();
    let _worktree = fixture.worktree("feature");
    {
        let mut config = fixture.repo.config().unwrap();
        config.set_str("extensions.worktreeConfig", "true").unwrap();
        config.set_i32("core.repositoryformatversion", 1).unwrap();
    }
    let worktree_git_dir = fixture.repo.path().join("worktrees").join("feature");
    std::fs::write(
        worktree_git_dir.join("config.worktree"),
        "[remote \"origin\"]\n\turl = https://gitlab.com/wt/r.git\n",
    )
    .unwrap();

    let (prepared, report) = prepare(
        &fixture.root,
        &GitStatusOptions {
            worktree: Some("feature"),
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    assert_eq!(
        prepared.commit_links[&pushed].commit_url.as_deref(),
        Some(format!("https://gitlab.com/wt/r/-/commit/{pushed}").as_str())
    );
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 1, "{:?}", report.counters);
}

#[test]
fn a_branch_resolving_through_per_worktree_refs_takes_a_fresh_snapshot() {
    let (fixture, pushed, unpushed) = pushed_and_unpushed();
    let _worktree = fixture.worktree("feature");
    let git_dir = fixture.repo.path().to_path_buf();
    // `pinned` is shared, but its target is private to each worktree.
    std::fs::write(git_dir.join("refs/heads/pinned"), "ref: refs/worktree/pin\n").unwrap();
    std::fs::create_dir_all(git_dir.join("refs/worktree")).unwrap();
    std::fs::write(git_dir.join("refs/worktree/pin"), format!("{unpushed}\n")).unwrap();
    let private = git_dir.join("worktrees/feature/refs/worktree");
    std::fs::create_dir_all(&private).unwrap();
    std::fs::write(private.join("pin"), format!("{pushed}\n")).unwrap();

    let (initial, _) = prepare(&fixture.root, &options(10));
    let initial = initial.unwrap();
    let git = initial.git.as_ref().unwrap();
    assert!(git.recent[0].refs.iter().any(|r| r.name == "pinned"), "{:?}", git.recent);

    let (prepared, report) = prepare(
        &fixture.root,
        &GitStatusOptions {
            worktree: Some("feature"),
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();
    assert_eq!(shas(git), [unpushed.as_str(), pushed.as_str()]);
    // Seen from the selected worktree, `pinned` names the pushed commit.
    assert!(git.recent[1].refs.iter().any(|r| r.name == "pinned"), "{:?}", git.recent);
    assert!(git.recent[0].refs.iter().all(|r| r.name != "pinned"), "{:?}", git.recent);
    assert_eq!(links_for(&prepared, &[&unpushed, &pushed]), [&absent(), &linked(&pushed)]);
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 2, "{:?}", report.counters);
}

#[test]
fn combined_selectors_apply_in_order_and_link_only_the_final_display() {
    let (mut fixture, [alpha_core, alpha_cli, beta_core]) = monorepo();
    fixture.branch("other", &alpha_core);
    let worktree = fixture.worktree("feature");
    let feature = fixture.commit_in(&worktree, &[("beta/core/src/lib.rs", "pub fn d() {}")], "feature");

    let (prepared, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("alpha-core"),
            branch: Some("other"),
            worktree: Some("feature"),
            ..options(2)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();
    assert_eq!(shas(git), [feature.as_str(), beta_core.as_str()]);
    assert_eq!(links_for(&prepared, &[&feature, &beta_core]), [&absent(), &absent()]);
    assert!(!prepared.commit_links.contains_key(&alpha_core));
    assert!(!prepared.commit_links.contains_key(&alpha_cli));

    // An earlier selector's error survives a later selector that would
    // replace its result.
    let (bad_package, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("gamma"),
            worktree: Some("feature"),
            ..options(2)
        },
    );
    assert!(selection_error(bad_package).starts_with("Package 'gamma' not found."));
    std::fs::write(fixture.repo.path().join("refs/heads/broken"), "not a ref\n").unwrap();
    let (bad_branch, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            branch: Some("broken"),
            worktree: Some("feature"),
            ..options(2)
        },
    );
    assert!(matches!(bad_branch, Err(SniffError::Git { .. })), "{bad_branch:?}");
}

#[test]
fn a_missing_snapshot_produces_an_unlinked_report() {
    let (fixture, _pushed, _unpushed) = pushed_and_unpushed();
    let observation = FilesystemObservation::discover(&fixture.root);
    let initial = observation
        .detect_git(&GitRequest::full())
        .unwrap()
        .unwrap();
    let (git, links) =
        finish_status(&observation, Some((initial, None)), None, &options(10)).unwrap();
    assert_eq!(git.unwrap().recent.len(), 2);
    assert!(links.is_empty());
}

/// Publish the local branch's tip as `main` on the bare repository at `bare`.
fn publish(repo: &Repository, bare: &Path) {
    let head = repo.head().unwrap();
    let branch = head.name().unwrap();
    let mut remote = repo.remote_anonymous(bare.to_str().unwrap()).unwrap();
    remote
        .push(&[format!("+{branch}:refs/heads/main").as_str()], None)
        .unwrap();
}

#[test]
fn explicit_refresh_links_against_post_fetch_refs_without_another_ref_walk() {
    let mut fixture = Fixture::new();
    let bare = fixture.temp_path("remote.git");
    Repository::init_bare(&bare).unwrap();
    let first = fixture.commit(&[("README.md", "one")], "first");
    publish(&fixture.repo, &bare);
    fixture.set_remote_tip("main", &first);
    // The remote moves on; the local remote-tracking ref is now stale.
    let second = fixture.commit(&[("README.md", "two")], "second");
    publish(&fixture.repo, &bare);
    // Fetches of the GitHub URL read the local bare repository instead.
    fixture
        .repo
        .config()
        .unwrap()
        .set_str(
            &format!("url.{}.insteadOf", bare.to_string_lossy().replace('\\', "/")),
            ORIGIN,
        )
        .unwrap();

    let (stale, _) = prepare(&fixture.root, &options(10));
    assert_eq!(stale.unwrap().commit_links[&second], absent());

    let (prepared, report) = prepare(
        &fixture.root,
        &GitStatusOptions {
            refresh_remotes: true,
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();
    assert_eq!(shas(git), [second.as_str(), first.as_str()]);
    assert_eq!(links_for(&prepared, &[&second, &first]), [&linked(&second), &linked(&first)]);
    // Deep membership comes from the same post-fetch refs.
    assert_eq!(git.recent[0].remotes.as_deref(), Some(&["origin".to_string()][..]));
    assert_eq!(counter(&report, counters::GIT_REF_WALKS), 1, "{:?}", report.counters);
}

/// A Cargo workspace whose root manifest is also a package (member `"."`),
/// with one remotely contained commit and a nested `area/member` package.
fn root_member_workspace() -> (Fixture, String) {
    let mut fixture = Fixture::new();
    let initial = fixture.commit(
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"root-app\"\nversion = \"0.1.0\"\n\n[workspace]\nmembers = [\".\", \"area/member\"]\n",
            ),
            ("src/lib.rs", "pub fn original() {}\n"),
            ("area/member/Cargo.toml", "[package]\nname = \"member\"\nversion = \"0.1.0\"\n"),
            ("area/member/src/lib.rs", "pub fn original() {}\n"),
        ],
        "initial",
    );
    fixture.set_remote_tip("main", &initial);
    (fixture, initial)
}

fn stage(fixture: &Fixture, path: &str, content: &str) {
    std::fs::write(fixture.root.join(path), content).unwrap();
    let mut index = fixture.repo.index().unwrap();
    index.add_path(Path::new(path)).unwrap();
    index.write().unwrap();
}

fn changes(git: &GitInfo) -> Vec<(String, FileStatus)> {
    let mut changes: Vec<(String, FileStatus)> = git
        .file_changes
        .iter()
        .map(|change| (change.path.to_string_lossy().replace('\\', "/"), change.status))
        .collect();
    changes.sort_by(|a, b| a.0.cmp(&b.0));
    changes
}

/// The root package's empty relative path is the empty prefix, which every
/// repository-relative path matches, so its selection keeps the whole
/// repository's history and working-tree changes, nested packages included.
#[test]
fn a_root_package_selection_keeps_the_repository_history_and_changes() {
    let (fixture, initial) = root_member_workspace();
    stage(&fixture, "src/lib.rs", "pub fn staged() {}\n");
    std::fs::write(fixture.root.join("src/lib.rs"), "pub fn unstaged() {}\n").unwrap();
    std::fs::write(fixture.root.join("src/new.rs"), "// new\n").unwrap();
    std::fs::write(fixture.root.join("area/member/src/lib.rs"), "pub fn unstaged() {}\n").unwrap();

    let (prepared, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("root-app"),
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();
    assert_eq!(shas(git), [initial.as_str()]);
    assert_eq!(links_for(&prepared, &[initial.as_str()]), [&linked(&initial)]);
    assert_eq!(
        changes(git),
        [
            ("area/member/src/lib.rs".to_string(), FileStatus::Modified),
            ("src/lib.rs".to_string(), FileStatus::Both),
            ("src/new.rs".to_string(), FileStatus::Untracked),
        ]
    );
    let status = git.status.as_ref().unwrap();
    assert!(status.is_dirty);
    assert_eq!(
        (status.staged_count, status.unstaged_count, status.untracked_count),
        (1, 2, 1)
    );

    // Control: the nested package keeps only its own changes.
    let (prepared, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("member"),
            ..options(10)
        },
    );
    let prepared = prepared.unwrap();
    let git = prepared.git.as_ref().unwrap();
    assert_eq!(shas(git), [initial.as_str()]);
    assert_eq!(links_for(&prepared, &[initial.as_str()]), [&linked(&initial)]);
    assert_eq!(
        changes(git),
        [("area/member/src/lib.rs".to_string(), FileStatus::Modified)]
    );
    let status = git.status.as_ref().unwrap();
    assert!(status.is_dirty);
    assert_eq!(
        (status.staged_count, status.unstaged_count, status.untracked_count),
        (0, 1, 0)
    );

    // The root package lies in the top-level `""` area, never a named one.
    let (inside_area, _) = prepare(
        &fixture.root,
        &GitStatusOptions {
            package: Some("root-app"),
            package_area: Some("area"),
            ..options(10)
        },
    );
    assert_eq!(selection_error(inside_area), "Package 'root-app' is in area '', not 'area'");
}
