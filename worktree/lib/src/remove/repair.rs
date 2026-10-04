//! Restoring the link of a worktree whose directory remains but whose `.git`
//! file is gone, judged only by what Git reads afterward.
//!
//! `git worktree repair` can exit nonzero after fully restoring the link, and
//! run from the base checkout it also restores every other worktree's broken
//! link, so its exit status and output are diagnostics only. Success is the
//! four postconditions of [`Postcondition`], all checked through Git from the
//! base checkout.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::availability::{self, Availability};
use crate::error::WorktreeError;
use crate::git::{git_from, git_from_output};
use crate::worktree::{WorktreeEntry, parse_worktree_list};

use super::admin_entry::{
    AdminEntry, AssociationError, admin_entry_in, common_git_dir, read_back_reference, same_location,
};

/// The Git calls repair and its verification make, so tests can substitute
/// any of them. [`Git`] runs the real ones.
pub trait RepairGit {
    /// `git -C <base> worktree repair <target>`.
    fn repair(&self, base: &Path, target: &Path) -> RepairAttempt;
    /// `git -C <dir> rev-parse --path-format=absolute <flag>`, run from `base`.
    fn rev_parse_path(&self, base: &Path, dir: &Path, flag: &str) -> Result<PathBuf, WorktreeError>;
    /// `git -C <base> worktree list --porcelain`.
    fn worktree_list(&self, base: &Path) -> Result<String, WorktreeError>;
}

/// The real Git calls of [`RepairGit`].
pub struct Git;

impl RepairGit for Git {
    fn repair(&self, base: &Path, target: &Path) -> RepairAttempt {
        match git_from_output(base, base, &[OsStr::new("worktree"), OsStr::new("repair"), target.as_os_str()]) {
            Ok(output) => RepairAttempt {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
                spawn_error: None,
            },
            Err(error) => RepairAttempt { spawn_error: Some(error.to_string()), ..RepairAttempt::default() },
        }
    }

    fn rev_parse_path(&self, base: &Path, dir: &Path, flag: &str) -> Result<PathBuf, WorktreeError> {
        git_from(base, dir, &["rev-parse", "--path-format=absolute", flag]).map(PathBuf::from)
    }

    fn worktree_list(&self, base: &Path) -> Result<String, WorktreeError> {
        git_from(base, base, &["worktree", "list", "--porcelain"])
    }
}

/// What running `git worktree repair` produced. Never evidence of success.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairAttempt {
    /// `None` when Git did not run or was ended by a signal.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// Why Git could not be started or waited on.
    pub spawn_error: Option<String>,
}

impl RepairAttempt {
    /// Git's output and any spawn failure, one item per line, for display.
    pub fn diagnostics(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(error) = &self.spawn_error {
            lines.push(format!("git worktree repair could not run: {error}"));
        }
        lines.extend(self.stderr.lines().chain(self.stdout.lines()).filter(|line| !line.trim().is_empty()).map(str::to_string));
        lines
    }
}

/// A postcondition that did not hold after the repair attempt.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Postcondition {
    #[error("the checkout's .git leads to {}, not its worktree record {}", .found.display(), .expected.display())]
    GitDir { expected: PathBuf, found: PathBuf },
    #[error("the checkout belongs to the repository at {}, not this one", .found.display())]
    CommonDir { found: PathBuf },
    #[error("Git couldn't resolve the checkout's .git: {0}")]
    Unresolvable(String),
    #[error("the worktree record's back-reference is unusable: {0}")]
    BackReference(String),
    #[error("Git found the checkout at {}, not at the worktree itself", .found.display())]
    TopLevel { found: PathBuf },
    #[error("Git's worktree list {0}")]
    Listing(String),
}

/// A verified repair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairReport {
    /// The record the restored link was verified to lead to.
    pub admin: AdminEntry,
    pub attempt: RepairAttempt,
}

/// Why the link was not restored. Only [`RepairRefusal::Unverified`] follows
/// an attempt, which may have changed Git metadata for this or other
/// worktrees.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepairRefusal {
    /// The target is no longer a directory with a missing `.git`.
    #[error("the worktree is no longer a directory with a missing .git file")]
    NotUnlinked(Availability),
    #[error(transparent)]
    Association(AssociationError),
    #[error("Git couldn't restore a verified link: {}", display_failures(.failures))]
    Unverified { failures: Vec<Postcondition>, attempt: RepairAttempt },
}

impl RepairRefusal {
    /// Whether `git worktree repair` ran (or was attempted) before refusing.
    pub fn repair_attempted(&self) -> bool {
        matches!(self, RepairRefusal::Unverified { .. })
    }
}

fn display_failures(failures: &[Postcondition]) -> String {
    failures.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")
}

/// Restores the link of `entry`, an unlinked worktree, and verifies it.
///
/// Refuses without running repair unless `entry` is still
/// [`Availability::Unlinked`] and exactly one worktree record points to it.
/// Otherwise runs repair from `base` and returns a report only when every
/// [`Postcondition`] holds, whatever Git's exit status. Nothing is rolled
/// back on refusal.
pub fn repair_unlinked(base: &Path, entry: &WorktreeEntry, git: &dyn RepairGit) -> Result<RepairReport, RepairRefusal> {
    let availability = availability::classify(entry);
    if availability != Availability::Unlinked {
        return Err(RepairRefusal::NotUnlinked(availability));
    }
    let common = common_git_dir(base)
        .map_err(|error| RepairRefusal::Association(AssociationError::CommonDirUnknown(error.to_string())))?;
    let admin = admin_entry_in(&common, &entry.path).map_err(RepairRefusal::Association)?;

    let attempt = git.repair(base, &entry.path);
    let failures = postcondition_failures(base, entry, &common, &admin, git);
    if failures.is_empty() {
        Ok(RepairReport { admin, attempt })
    } else {
        Err(RepairRefusal::Unverified { failures, attempt })
    }
}

/// Every postcondition that does not hold, in a fixed order; empty when the
/// link is verified.
fn postcondition_failures(base: &Path, entry: &WorktreeEntry, common: &Path, admin: &AdminEntry, git: &dyn RepairGit) -> Vec<Postcondition> {
    let target = &entry.path;
    let mut failures = Vec::new();

    match git.rev_parse_path(base, target, "--git-dir") {
        Ok(found) if same_location(&found, &admin.dir) => {}
        Ok(found) => failures.push(Postcondition::GitDir { expected: admin.dir.clone(), found }),
        Err(error) => failures.push(Postcondition::Unresolvable(error.to_string())),
    }
    match git.rev_parse_path(base, target, "--git-common-dir") {
        Ok(found) if same_location(&found, common) => {}
        Ok(found) => failures.push(Postcondition::CommonDir { found }),
        Err(error) => failures.push(Postcondition::Unresolvable(error.to_string())),
    }
    match read_back_reference(&admin.dir) {
        Ok(named) if same_location(&named, &target.join(".git")) => {}
        Ok(named) => failures.push(Postcondition::BackReference(format!("it names {}", named.display()))),
        Err(error) => failures.push(Postcondition::BackReference(error.to_string())),
    }
    match git.rev_parse_path(base, target, "--show-toplevel") {
        Ok(found) if same_location(&found, target) => {}
        Ok(found) => failures.push(Postcondition::TopLevel { found }),
        Err(error) => failures.push(Postcondition::Unresolvable(error.to_string())),
    }
    if let Some(problem) = listing_problem(base, entry, git) {
        failures.push(Postcondition::Listing(problem));
    }
    failures
}

/// Why a fresh listing does not show `entry` as the same, readable worktree.
fn listing_problem(base: &Path, entry: &WorktreeEntry, git: &dyn RepairGit) -> Option<String> {
    let listing = match git.worktree_list(base) {
        Ok(listing) => listing,
        Err(error) => return Some(format!("could not be read: {error}")),
    };
    let entries = parse_worktree_list(&listing);
    let found: Vec<&WorktreeEntry> = entries.iter().filter(|listed| same_location(&listed.path, &entry.path)).collect();
    let [listed] = found.as_slice() else {
        return Some(format!("shows this path {} times", found.len()));
    };
    if listed.prunable.is_some() {
        return Some("still marks it prunable".into());
    }
    identity_change(entry, listed)
}

/// How `now` differs from `before`, by branch, or by HEAD when detached.
/// `None` when it is the same worktree.
pub(crate) fn identity_change(before: &WorktreeEntry, now: &WorktreeEntry) -> Option<String> {
    match (&before.branch, &now.branch) {
        (Some(was), Some(is)) if was == is => None,
        (None, None) if before.head_sha == now.head_sha => None,
        _ => Some(format!(
            "shows {} where {} was checked out",
            describe(now),
            describe(before)
        )),
    }
}

fn describe(entry: &WorktreeEntry) -> String {
    match (&entry.branch, &entry.head_sha) {
        (Some(branch), _) => format!("branch {branch}"),
        (None, Some(head)) => format!("detached HEAD {}", &head[..head.len().min(12)]),
        (None, None) => "no HEAD".into(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;

    use super::*;
    use crate::remove::test_support::TestRepo;

    type ScriptedRepair<'a> = Box<dyn Fn(&Path, &Path) -> RepairAttempt + 'a>;
    /// `Some` replaces the answer for that `rev-parse` flag.
    type ScriptedRevParse<'a> = Box<dyn Fn(&str) -> Option<Result<PathBuf, WorktreeError>> + 'a>;

    /// Real Git, except for whichever calls a test replaces.
    #[derive(Default)]
    struct Scripted<'a> {
        repair: Option<ScriptedRepair<'a>>,
        rev_parse: Option<ScriptedRevParse<'a>>,
        repairs: Cell<usize>,
    }

    impl RepairGit for Scripted<'_> {
        fn repair(&self, base: &Path, target: &Path) -> RepairAttempt {
            self.repairs.set(self.repairs.get() + 1);
            match &self.repair {
                Some(repair) => repair(base, target),
                None => Git.repair(base, target),
            }
        }

        fn rev_parse_path(&self, base: &Path, dir: &Path, flag: &str) -> Result<PathBuf, WorktreeError> {
            match self.rev_parse.as_ref().and_then(|rev_parse| rev_parse(flag)) {
                Some(result) => result,
                None => Git.rev_parse_path(base, dir, flag),
            }
        }

        fn worktree_list(&self, base: &Path) -> Result<String, WorktreeError> {
            Git.worktree_list(base)
        }
    }

    fn listed(repo: &TestRepo, path: &Path) -> WorktreeEntry {
        let entries = parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]));
        entries.into_iter().find(|entry| same_location(&entry.path, path)).expect("listed")
    }

    /// A linked worktree on `feat/x` with a tracked edit, then its `.git` file
    /// deleted: the `lhg-before` shape, attached.
    fn unlinked(repo: &TestRepo) -> (PathBuf, WorktreeEntry) {
        let checkout = repo.add_worktree("feat/x", "x", "main");
        fs::write(checkout.join("README.md"), "edited\n").unwrap();
        fs::remove_file(checkout.join(".git")).unwrap();
        let entry = listed(repo, &checkout);
        assert!(entry.prunable.is_some(), "Git marks it prunable");
        (checkout, entry)
    }

    fn admin_of(repo: &TestRepo, name: &str) -> PathBuf {
        common_git_dir(&repo.path()).unwrap().join("worktrees").join(name)
    }

    fn attempt(exit_code: i32) -> RepairAttempt {
        RepairAttempt { exit_code: Some(exit_code), ..RepairAttempt::default() }
    }

    fn failures(result: Result<RepairReport, RepairRefusal>) -> (Vec<Postcondition>, RepairAttempt) {
        match result {
            Err(RepairRefusal::Unverified { failures, attempt }) => (failures, attempt),
            other => panic!("expected an unverified repair, got {other:?}"),
        }
    }

    #[test]
    fn git_repair_of_an_unlinked_branch_is_verified_by_its_postconditions() {
        let repo = TestRepo::new();
        let (checkout, entry) = unlinked(&repo);

        let report = repair_unlinked(&repo.path(), &entry, &Git).expect("verified");

        assert!(same_location(&report.admin.dir, &admin_of(&repo, "x")));
        assert_eq!(repo.git_in(&checkout, &["status", "--porcelain"]), "M README.md", "the edit survives and is readable");
        let after = listed(&repo, &checkout);
        assert_eq!((after.prunable, after.branch.as_deref()), (None, Some("feat/x")));
    }

    #[test]
    fn git_repair_of_an_unlinked_detached_worktree_is_verified() {
        let repo = TestRepo::new();
        let checkout = repo.add_worktree("feat/d", "d", "main");
        repo.git_in(&checkout, &["checkout", "-q", "--detach"]);
        fs::remove_file(checkout.join(".git")).unwrap();
        let entry = listed(&repo, &checkout);
        assert_eq!(entry.branch, None);

        repair_unlinked(&repo.path(), &entry, &Git).expect("verified");
        assert_eq!(listed(&repo, &checkout).head_sha, entry.head_sha);
    }

    #[test]
    fn a_nonzero_exit_with_valid_postconditions_is_success() {
        let repo = TestRepo::new();
        let (_checkout, entry) = unlinked(&repo);
        let git = Scripted {
            repair: Some(Box::new(|base, target| RepairAttempt { exit_code: Some(1), stderr: "error: unable to locate repository".into(), ..Git.repair(base, target) })),
            ..Scripted::default()
        };

        let report = repair_unlinked(&repo.path(), &entry, &git).expect("exit status is not the verdict");
        assert_eq!(report.attempt.exit_code, Some(1));
        assert_eq!(report.attempt.diagnostics(), ["error: unable to locate repository"]);
    }

    #[test]
    fn a_zero_exit_without_a_change_refuses_and_touches_nothing() {
        let repo = TestRepo::new();
        let (checkout, entry) = unlinked(&repo);
        let git = Scripted { repair: Some(Box::new(|_, _| attempt(0))), ..Scripted::default() };

        let (failures, attempt) = failures(repair_unlinked(&repo.path(), &entry, &git));

        assert_eq!(attempt.exit_code, Some(0));
        assert!(failures.iter().any(|failure| matches!(failure, Postcondition::Listing(text) if text.contains("prunable"))), "{failures:?}");
        assert!(!checkout.join(".git").exists());
        assert_eq!(fs::read_to_string(checkout.join("README.md")).unwrap(), "edited\n");
        assert!(admin_of(&repo, "x").is_dir(), "record kept");
        repo.git(&["rev-parse", "--verify", "refs/heads/feat/x"]);
    }

    #[test]
    fn a_spawn_failure_is_named_and_never_claimed_as_repaired() {
        let repo = TestRepo::new();
        let (_checkout, entry) = unlinked(&repo);
        let git = Scripted {
            repair: Some(Box::new(|_, _| RepairAttempt { spawn_error: Some("No such file or directory".into()), ..RepairAttempt::default() })),
            ..Scripted::default()
        };

        let (failures, attempt) = failures(repair_unlinked(&repo.path(), &entry, &git));
        assert!(!failures.is_empty());
        assert_eq!(attempt.diagnostics(), ["git worktree repair could not run: No such file or directory"]);
    }

    #[test]
    fn a_link_to_another_record_in_the_same_repository_refuses() {
        let repo = TestRepo::new();
        let (checkout, entry) = unlinked(&repo);
        repo.add_worktree("feat/y", "y", "main");
        let other = admin_of(&repo, "y");
        let git = Scripted {
            repair: Some(Box::new(|_, target| {
                fs::write(target.join(".git"), format!("gitdir: {}\n", other.display())).unwrap();
                attempt(0)
            })),
            ..Scripted::default()
        };

        let (failures, _) = failures(repair_unlinked(&repo.path(), &entry, &git));
        assert!(failures.iter().any(|failure| matches!(failure, Postcondition::GitDir { .. })), "{failures:?}");
        assert!(checkout.join("README.md").exists());
    }

    #[test]
    fn a_link_into_a_foreign_repository_refuses() {
        let repo = TestRepo::new();
        let (_checkout, entry) = unlinked(&repo);
        let foreign = TestRepo::new();
        foreign.add_worktree("feat/x", "x", "main");
        let foreign_admin = admin_of(&foreign, "x");
        let git = Scripted {
            repair: Some(Box::new(|_, target| {
                fs::write(target.join(".git"), format!("gitdir: {}\n", foreign_admin.display())).unwrap();
                attempt(0)
            })),
            ..Scripted::default()
        };

        let (failures, _) = failures(repair_unlinked(&repo.path(), &entry, &git));
        assert!(failures.iter().any(|failure| matches!(failure, Postcondition::GitDir { .. })), "{failures:?}");
        assert!(failures.iter().any(|failure| matches!(failure, Postcondition::CommonDir { .. })), "{failures:?}");
    }

    /// Nested in the base, a checkout without `.git` is found by Git as part
    /// of the base: the top level is the base, never the target.
    #[test]
    fn discovering_the_parent_repository_is_not_success() {
        let repo = TestRepo::new();
        let nested = repo.path().join("inner").join("wt");
        repo.git(&["worktree", "add", "-q", "-b", "feat/n", nested.to_str().unwrap(), "main"]);
        fs::remove_file(nested.join(".git")).unwrap();
        let entry = listed(&repo, &nested);
        let git = Scripted { repair: Some(Box::new(|_, _| attempt(0))), ..Scripted::default() };

        let (failures, _) = failures(repair_unlinked(&repo.path(), &entry, &git));
        assert!(
            failures.iter().any(|failure| matches!(failure, Postcondition::TopLevel { found } if same_location(found, &repo.path()))),
            "{failures:?}"
        );
    }

    /// The top-level check alone refuses: every other postcondition holds.
    #[test]
    fn a_wrong_top_level_alone_refuses() {
        let repo = TestRepo::new();
        let (_checkout, entry) = unlinked(&repo);
        let base = repo.path();
        let git = Scripted {
            rev_parse: Some(Box::new(|flag| (flag == "--show-toplevel").then(|| Ok(base.clone())))),
            ..Scripted::default()
        };

        let (failures, _) = failures(repair_unlinked(&repo.path(), &entry, &git));
        assert!(matches!(failures.as_slice(), [Postcondition::TopLevel { .. }]), "{failures:?}");
    }

    #[test]
    fn a_changed_identity_refuses() {
        let repo = TestRepo::new();
        let (_checkout, entry) = unlinked(&repo);
        repo.git(&["branch", "other", "main"]);
        let git = Scripted {
            repair: Some(Box::new(|base, target| {
                let attempt = Git.repair(base, target);
                repo.git_in(target, &["checkout", "-q", "-f", "other"]);
                attempt
            })),
            ..Scripted::default()
        };

        let (failures, _) = failures(repair_unlinked(&repo.path(), &entry, &git));
        assert!(
            matches!(failures.as_slice(), [Postcondition::Listing(text)] if text.contains("branch other") && text.contains("branch feat/x")),
            "{failures:?}"
        );
    }

    #[test]
    fn no_repair_runs_unless_the_target_is_unlinked() {
        let repo = TestRepo::new();
        let healthy = repo.add_worktree("feat/h", "h", "main");
        let missing = repo.add_worktree("feat/m", "m", "main");
        fs::remove_dir_all(&missing).unwrap();
        let git = Scripted::default();

        for (path, expected) in [(&healthy, Availability::Healthy), (&missing, Availability::Missing)] {
            let entry = listed(&repo, path);
            assert_eq!(repair_unlinked(&repo.path(), &entry, &git), Err(RepairRefusal::NotUnlinked(expected)));
        }
        assert_eq!(git.repairs.get(), 0);
    }

    #[test]
    fn an_ambiguous_record_refuses_before_repair() {
        let repo = TestRepo::new();
        let (checkout, entry) = unlinked(&repo);
        let copy = admin_of(&repo, "x-copy");
        fs::create_dir(&copy).unwrap();
        fs::copy(admin_of(&repo, "x").join("gitdir"), copy.join("gitdir")).unwrap();
        let git = Scripted::default();

        let result = repair_unlinked(&repo.path(), &entry, &git);
        assert!(matches!(result, Err(RepairRefusal::Association(AssociationError::Ambiguous(ref dirs))) if dirs.len() == 2), "{result:?}");
        assert!(!result.unwrap_err().repair_attempted());
        assert_eq!(git.repairs.get(), 0);
        assert!(!checkout.join(".git").exists());
    }
}
