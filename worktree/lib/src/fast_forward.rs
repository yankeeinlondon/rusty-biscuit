//! `wt --ff`: fast-forward the local default branch to `origin/<default>`
//! (spec `2026-09-27-list-freshness-ux` §9).
//!
//! Contracts:
//!
//! - Only `refs/heads/<default>` moves, and only forward. Nothing is forced,
//!   no branch is created, and no other ref is written.
//! - Both refs and their ancestry are read immediately before the move, and
//!   the move names what was read: `update-ref` carries the expected old value
//!   (compare-and-swap), and `merge --ff-only` merges the verified SHA rather
//!   than `origin/<default>`, so a concurrent fetch cannot change the target.
//! - A checkout holding the branch moves only through `merge --ff-only
//!   --no-autostash`, which refuses rather than overwrite local changes.
//! - A holder that is no longer on the default branch is re-resolved once;
//!   anything still unsafe is refused, never merged into another branch.
//! - A holder Git marks `prunable` still holds the branch: the move is
//!   refused, never made underneath it, and nothing is repaired. Git run in
//!   such a directory could find a parent repository instead.
//!
//! One race remains outside Git's guarantees: a checkout that switches
//! branches between the `symbolic-ref` check and the merge.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::git::{git_from, git_from_bytes_allow_no_match};
use crate::live_remote::is_valid_branch_name;
use crate::worktree::{WorktreeEntry, parse_worktree_list};

/// Why a fast-forward did not move the branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfRefusal {
    /// `git merge --ff-only` refused because local changes to files the
    /// update touches would be overwritten.
    DirtyCheckout,
    /// The local default branch and `origin/<default>` have diverged.
    Diverged,
    /// `refs/heads/<default>` does not exist (payload: the ref name shown to
    /// users, e.g. `main`).
    MissingLocal(String),
    /// `refs/remotes/origin/<default>` does not exist (payload e.g.
    /// `origin/main`).
    MissingTracking(String),
    /// The branch or its checkout changed while we worked (compare-and-swap
    /// lost, or the holding checkout left the default branch) and it could
    /// not be made safe.
    Changed,
    /// The checkout holding the branch is one Git marks `prunable` (payload:
    /// its path as `git worktree list` spells it).
    UnavailableHolder(PathBuf),
    /// Any other git failure. Git's stderr is deliberately not carried.
    Other,
}

/// The outcome of [`fast_forward_default`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfResult {
    /// The branch moved from `from` to `to` (full SHAs). `checkout` is the
    /// worktree whose files moved, as `git worktree list` spells it; `None`
    /// when no checkout held the branch.
    Moved {
        from: String,
        to: String,
        checkout: Option<PathBuf>,
    },
    /// Already in sync, or the local branch is ahead: nothing to do and
    /// nothing to say.
    UpToDate,
    Refused(FfRefusal),
}

/// Fast-forwards the local default branch `default` of the repository whose
/// main checkout is `main` to `origin/<default>`.
///
/// ## Returns
///
/// [`FfResult::UpToDate`] when the local branch equals or contains the
/// tracking ref, and [`FfResult::Refused`] for everything that is not a clean
/// fast-forward, including a needed move whose holder Git marks `prunable`. An invalid branch name is [`FfRefusal::Other`] with no git
/// mutation.
pub fn fast_forward_default(main: &Path, default: &str) -> FfResult {
    if !is_valid_branch_name(main, default) {
        return FfResult::Refused(FfRefusal::Other);
    }
    // An unavailable holder refuses only a needed move, so a branch already
    // in sync still reads as up to date.
    let holder = match holder_of(main, default) {
        Ok(holder) => Ok(holder),
        Err(FfRefusal::UnavailableHolder(path)) => Err(path),
        Err(refusal) => return FfResult::Refused(refusal),
    };
    match target(main, default) {
        Ok(Some((old, new))) => match holder {
            Ok(holder) => move_branch(main, default, &old, &new, holder),
            Err(path) => FfResult::Refused(FfRefusal::UnavailableHolder(path)),
        },
        Ok(None) => FfResult::UpToDate,
        Err(refusal) => FfResult::Refused(refusal),
    }
}

/// `Some((local, tracking))` when the local branch is strictly behind its
/// tracking ref, `None` when equal or ahead.
fn target(main: &Path, default: &str) -> Result<Option<(String, String)>, FfRefusal> {
    let local = read_ref(main, &format!("refs/heads/{default}"))
        .ok_or_else(|| FfRefusal::MissingLocal(default.to_string()))?;
    let tracking = read_ref(main, &format!("refs/remotes/origin/{default}"))
        .ok_or_else(|| FfRefusal::MissingTracking(format!("origin/{default}")))?;
    if local == tracking {
        return Ok(None);
    }
    // Exit 1 (no common ancestor) reads as an empty base: diverged.
    let base = git_from_bytes_allow_no_match(main, main, &["merge-base", &local, &tracking], None)
        .map_err(|_| FfRefusal::Other)?;
    let base = String::from_utf8_lossy(&base).trim().to_string();
    if base == tracking {
        Ok(None)
    } else if base == local {
        Ok(Some((local, tracking)))
    } else {
        Err(FfRefusal::Diverged)
    }
}

fn read_ref(main: &Path, refname: &str) -> Option<String> {
    git_from(main, main, &["rev-parse", "--verify", "-q", refname])
        .ok()
        .filter(|sha| !sha.is_empty())
}

/// The worktree that has `refs/heads/<default>` checked out, if any. Two
/// holders (possible with `--force`) are refused: moving the branch under
/// one would leave the other's files stale. So is a `prunable` holder.
fn holder_of(main: &Path, default: &str) -> Result<Option<PathBuf>, FfRefusal> {
    let listing = git_from(main, main, &["worktree", "list", "--porcelain"]).map_err(|_| FfRefusal::Other)?;
    let mut holders: Vec<WorktreeEntry> = parse_worktree_list(&listing)
        .into_iter()
        .filter(|entry| entry.branch.as_deref() == Some(default))
        .collect();
    match holders.len() {
        0 => Ok(None),
        1 => {
            let holder = holders.remove(0);
            match holder.prunable {
                Some(_) => Err(FfRefusal::UnavailableHolder(holder.path)),
                None => Ok(Some(holder.path)),
            }
        }
        _ => Err(FfRefusal::Other),
    }
}

fn is_on_branch(main: &Path, checkout: &Path, default: &str) -> bool {
    git_from(main, checkout, &["symbolic-ref", "-q", "HEAD"]).is_ok_and(|head| head == format!("refs/heads/{default}"))
}

/// Moves `refs/heads/<default>` from `old` to `new`, through `holder`'s
/// checkout when it still holds the branch, re-resolving the holder once
/// when it does not.
fn move_branch(main: &Path, default: &str, old: &str, new: &str, holder: Option<PathBuf>) -> FfResult {
    let holder = match holder {
        Some(checkout) if !is_on_branch(main, &checkout, default) => match holder_of(main, default) {
            Ok(Some(checkout)) if is_on_branch(main, &checkout, default) => Some(checkout),
            Ok(None) => None,
            Ok(Some(_)) => return FfResult::Refused(FfRefusal::Changed),
            Err(refusal) => return FfResult::Refused(refusal),
        },
        holder => holder,
    };
    let outcome = match &holder {
        None => update_ref(main, default, old, new),
        Some(checkout) => merge_ff_only(main, checkout, default, old, new),
    };
    match outcome {
        Ok(()) => FfResult::Moved { from: old.to_string(), to: new.to_string(), checkout: holder },
        Err(refusal) => FfResult::Refused(refusal),
    }
}

fn update_ref(main: &Path, default: &str, old: &str, new: &str) -> Result<(), FfRefusal> {
    let refname = format!("refs/heads/{default}");
    match git_from(main, main, &["update-ref", "-m", "wt: fast-forward", &refname, new, old]) {
        Ok(_) => Ok(()),
        Err(_) if read_ref(main, &refname).as_deref() != Some(old) => Err(FfRefusal::Changed),
        Err(_) => Err(FfRefusal::Other),
    }
}

fn merge_ff_only(main: &Path, checkout: &Path, default: &str, old: &str, new: &str) -> Result<(), FfRefusal> {
    let refname = format!("refs/heads/{default}");
    if read_ref(main, &refname).as_deref() != Some(old) {
        return Err(FfRefusal::Changed);
    }
    let args = ["merge", "--ff-only", "--no-autostash", "-q", new];
    #[cfg(any(test, feature = "count-git"))]
    crate::git::recorder::record(&args);
    // A raw command: `git_from` sets no locale, and the refusal is only
    // recognizable from `LC_ALL=C` stderr.
    let output = Command::new("git")
        .current_dir(main)
        .arg("-C")
        .arg(checkout)
        .args(args)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .map_err(|_| FfRefusal::Other)?;
    if !output.status.success() {
        return Err(if String::from_utf8_lossy(&output.stderr).contains("would be overwritten") {
            FfRefusal::DirtyCheckout
        } else if read_ref(main, &refname).as_deref() != Some(old) {
            FfRefusal::Changed
        } else {
            FfRefusal::Other
        });
    }
    if read_ref(main, &refname).as_deref() == Some(new) {
        Ok(())
    } else {
        Err(FfRefusal::Changed)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::remove::test_support::TestRepo;

    /// Every ref and its value, one per line.
    fn refs(repo: &TestRepo) -> String {
        repo.git(&["for-each-ref", "--format=%(refname) %(objectname)"])
    }

    /// `refs` with `refs/heads/<branch>` swapped to `sha`.
    fn refs_with(before: &str, branch: &str, sha: &str) -> String {
        let prefix = format!("refs/heads/{branch} ");
        before
            .lines()
            .map(|line| if line.starts_with(&prefix) { format!("{prefix}{sha}") } else { line.to_string() })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Pushes a commit to origin's `branch` and fetches it into the local
    /// tracking ref; returns the pushed SHA.
    fn advance_origin(repo: &TestRepo, branch: &str, file: &str) -> String {
        let sha = repo.push_commit_to_origin(branch, file);
        repo.git(&["fetch", "-q", "origin"]);
        sha
    }

    /// Pushes a commit to origin's `main` that sets `file` to `contents`,
    /// from a separate clone, and fetches it; returns the pushed SHA.
    fn push_edit(repo: &TestRepo, file: &str, contents: &str) -> String {
        let editor = repo.path().parent().unwrap().join("editor");
        if !editor.exists() {
            let origin = repo.origin_path();
            repo.git(&["clone", "-q", origin.to_str().unwrap(), editor.to_str().unwrap()]);
            repo.git_in(&editor, &["config", "user.email", "editor@example.com"]);
            repo.git_in(&editor, &["config", "user.name", "Editor"]);
            repo.git_in(&editor, &["config", "commit.gpgsign", "false"]);
        }
        fs::write(editor.join(file), contents).unwrap();
        repo.git_in(&editor, &["commit", "-q", "-am", "edit"]);
        repo.git_in(&editor, &["push", "-q", "origin", "HEAD:refs/heads/main"]);
        repo.git(&["fetch", "-q", "origin"]);
        repo.git_in(&editor, &["rev-parse", "HEAD"])
    }

    fn same_dir(a: &Path, b: &Path) -> bool {
        a.canonicalize().unwrap() == b.canonicalize().unwrap()
    }

    #[test]
    fn moves_the_ref_when_no_checkout_holds_the_branch() {
        let repo = TestRepo::with_origin();
        repo.git(&["switch", "-q", "-c", "feature"]);
        let old = repo.sha("main");
        let new = advance_origin(&repo, "main", "remote.txt");
        let before = refs(&repo);

        let result = fast_forward_default(&repo.path(), "main");

        assert_eq!(result, FfResult::Moved { from: old, to: new.clone(), checkout: None });
        assert_eq!(refs(&repo), refs_with(&before, "main", &new));
        assert!(!repo.path().join("remote.txt").exists());
    }

    #[test]
    fn a_concurrent_change_makes_the_compare_and_swap_refuse() {
        let repo = TestRepo::with_origin();
        repo.git(&["switch", "-q", "-c", "feature"]);
        let stale = repo.sha("main");
        let new = advance_origin(&repo, "main", "remote.txt");
        let concurrent = repo.commit("local.txt");
        repo.git(&["branch", "-f", "main", &concurrent]);

        let result = move_branch(&repo.path(), "main", &stale, &new, None);

        assert_eq!(result, FfResult::Refused(FfRefusal::Changed));
        assert_eq!(repo.sha("refs/heads/main"), concurrent);
    }

    #[test]
    fn fast_forwards_a_clean_checkout_holding_the_branch() {
        let repo = TestRepo::with_origin();
        let old = repo.sha("main");
        let new = advance_origin(&repo, "main", "remote.txt");

        let result = fast_forward_default(&repo.path(), "main");

        let FfResult::Moved { from, to, checkout: Some(checkout) } = result else {
            panic!("expected a move through the checkout, got {result:?}");
        };
        assert_eq!((from, to), (old, new.clone()));
        assert!(same_dir(&checkout, &repo.path()));
        assert_eq!(repo.sha("HEAD"), new);
        assert!(repo.path().join("remote.txt").exists());
        assert_eq!(repo.git(&["status", "--porcelain"]), "");
    }

    #[test]
    fn refuses_when_local_changes_touch_files_the_update_changes() {
        let repo = TestRepo::with_origin();
        let old = repo.sha("HEAD");
        push_edit(&repo, "README.md", "remote edit\n");
        let dirty = b"local edit, not committed\n";
        fs::write(repo.path().join("README.md"), dirty).unwrap();
        let before = refs(&repo);

        let result = fast_forward_default(&repo.path(), "main");

        assert_eq!(result, FfResult::Refused(FfRefusal::DirtyCheckout));
        assert_eq!(fs::read(repo.path().join("README.md")).unwrap(), dirty);
        assert_eq!(repo.sha("HEAD"), old);
        assert_eq!(refs(&repo), before);
    }

    #[test]
    fn re_resolves_a_branch_that_moved_to_another_worktree() {
        let repo = TestRepo::with_origin();
        let old = repo.sha("main");
        let new = advance_origin(&repo, "main", "remote.txt");
        repo.git(&["switch", "-q", "-c", "feature"]);
        let other = repo.path().parent().unwrap().join("other");
        repo.git(&["worktree", "add", "-q", other.to_str().unwrap(), "main"]);
        let feature = repo.sha("feature");

        let result = move_branch(&repo.path(), "main", &old, &new, Some(repo.path()));

        let FfResult::Moved { checkout: Some(checkout), .. } = result else {
            panic!("expected a move through the new holder, got {result:?}");
        };
        assert!(same_dir(&checkout, &other));
        assert!(other.join("remote.txt").exists());
        assert!(!repo.path().join("remote.txt").exists());
        assert_eq!(repo.sha("feature"), feature);
        assert_eq!(repo.sha("refs/heads/main"), new);
    }

    #[test]
    fn re_resolves_to_a_ref_update_when_the_old_holder_left_and_none_took_it() {
        let repo = TestRepo::with_origin();
        let old = repo.sha("main");
        let new = advance_origin(&repo, "main", "remote.txt");
        repo.git(&["switch", "-q", "-c", "feature"]);
        let feature = repo.sha("feature");

        let result = move_branch(&repo.path(), "main", &old, &new, Some(repo.path()));

        assert_eq!(result, FfResult::Moved { from: old, to: new.clone(), checkout: None });
        assert!(!repo.path().join("remote.txt").exists());
        assert_eq!(repo.sha("feature"), feature);
        assert_eq!(repo.sha("refs/heads/main"), new);
    }

    #[test]
    fn refuses_a_diverged_branch_without_changing_refs() {
        let repo = TestRepo::with_origin();
        advance_origin(&repo, "main", "remote.txt");
        repo.commit("local.txt");
        let before = refs(&repo);

        assert_eq!(fast_forward_default(&repo.path(), "main"), FfResult::Refused(FfRefusal::Diverged));
        assert_eq!(refs(&repo), before);
    }

    #[test]
    fn reports_up_to_date_when_in_sync() {
        let repo = TestRepo::with_origin();
        let before = refs(&repo);

        assert_eq!(fast_forward_default(&repo.path(), "main"), FfResult::UpToDate);
        assert_eq!(refs(&repo), before);
    }

    #[test]
    fn reports_up_to_date_when_the_local_branch_is_ahead() {
        let repo = TestRepo::with_origin();
        repo.commit("local.txt");
        let before = refs(&repo);

        assert_eq!(fast_forward_default(&repo.path(), "main"), FfResult::UpToDate);
        assert_eq!(refs(&repo), before);
    }

    #[test]
    fn names_a_missing_local_branch_and_creates_none() {
        let repo = TestRepo::with_origin();
        advance_origin(&repo, "main", "remote.txt");
        repo.git(&["switch", "-q", "-c", "feature"]);
        repo.git(&["branch", "-q", "-D", "main"]);
        let before = refs(&repo);

        let result = fast_forward_default(&repo.path(), "main");

        assert_eq!(result, FfResult::Refused(FfRefusal::MissingLocal("main".into())));
        assert!(repo.try_git(&["rev-parse", "--verify", "-q", "refs/heads/main"]).is_err());
        assert_eq!(refs(&repo), before);
    }

    #[test]
    fn names_a_missing_tracking_ref_and_changes_nothing() {
        let repo = TestRepo::with_origin();
        repo.git(&["update-ref", "-d", "refs/remotes/origin/main"]);
        let before = refs(&repo);

        let result = fast_forward_default(&repo.path(), "main");

        assert_eq!(result, FfResult::Refused(FfRefusal::MissingTracking("origin/main".into())));
        assert_eq!(refs(&repo), before);
    }

    #[test]
    fn fast_forwards_a_default_branch_not_named_main() {
        let repo = TestRepo::with_origin();
        repo.git(&["branch", "trunk", "main"]);
        repo.git(&["push", "-q", "-u", "origin", "trunk"]);
        let old = repo.sha("trunk");
        let new = advance_origin(&repo, "trunk", "remote.txt");
        let before = refs(&repo);

        let result = fast_forward_default(&repo.path(), "trunk");

        assert_eq!(result, FfResult::Moved { from: old, to: new.clone(), checkout: None });
        assert_eq!(refs(&repo), refs_with(&before, "trunk", &new));
    }

    /// A linked worktree at `<base>/nested-main` holding `main`, with the base
    /// switched to `feature`, and origin's `main` one commit ahead.
    fn unavailable_holder_fixture(repo: &TestRepo) -> PathBuf {
        repo.git(&["switch", "-q", "-c", "feature"]);
        let holder = repo.path().join("nested-main");
        repo.git(&["worktree", "add", "-q", holder.to_str().unwrap(), "main"]);
        advance_origin(repo, "main", "remote.txt");
        holder
    }

    /// Git calls that could move a ref, repair a link, or run in the holder.
    fn mutating_or_holder_calls(calls: &[Vec<String>]) -> Vec<Vec<String>> {
        calls
            .iter()
            .filter(|args| {
                matches!(args.first().map(String::as_str), Some("merge" | "update-ref" | "symbolic-ref"))
                    || args.iter().any(|arg| arg == "repair" || arg == "prune")
            })
            .cloned()
            .collect()
    }

    fn assert_refused_for(result: FfResult, holder: &Path) {
        let FfResult::Refused(FfRefusal::UnavailableHolder(path)) = result else {
            panic!("expected an unavailable-holder refusal, got {result:?}");
        };
        assert_eq!(path.file_name(), holder.file_name(), "names the holder: {}", path.display());
    }

    #[test]
    fn refuses_a_move_under_an_unlinked_holder_without_repairing_it() {
        let repo = TestRepo::with_origin();
        let holder = unavailable_holder_fixture(&repo);
        fs::remove_file(holder.join(".git")).unwrap();
        let before = refs(&repo);
        let feature_head = repo.sha("HEAD");

        crate::git::recorder::start_recording();
        let result = fast_forward_default(&repo.path(), "main");
        let calls = crate::git::recorder::finish_recording();

        assert_refused_for(result, &holder);
        assert_eq!(refs(&repo), before, "no ref moved");
        assert_eq!(mutating_or_holder_calls(&calls), Vec::<Vec<String>>::new());
        assert!(fs::symlink_metadata(holder.join(".git")).is_err(), "the link was not repaired");
        assert!(!holder.join("remote.txt").exists() && !repo.path().join("remote.txt").exists());
        // The holder sits inside the base: Git run there would have found the
        // base, which must be untouched too.
        assert_eq!(repo.sha("HEAD"), feature_head);
        assert!(repo.git(&["worktree", "list", "--porcelain"]).contains("prunable"));
    }

    #[test]
    fn refuses_a_move_under_a_missing_holder_and_keeps_its_record() {
        let repo = TestRepo::with_origin();
        let holder = unavailable_holder_fixture(&repo);
        fs::remove_dir_all(&holder).unwrap();
        let before = refs(&repo);

        crate::git::recorder::start_recording();
        let result = fast_forward_default(&repo.path(), "main");
        let calls = crate::git::recorder::finish_recording();

        assert_refused_for(result, &holder);
        assert_eq!(refs(&repo), before);
        assert_eq!(mutating_or_holder_calls(&calls), Vec::<Vec<String>>::new());
        assert!(fs::symlink_metadata(&holder).is_err(), "nothing was recreated");
        let listing = repo.git(&["worktree", "list", "--porcelain"]);
        assert!(listing.contains("nested-main") && listing.contains("prunable"), "{listing}");
    }

    #[test]
    fn an_unavailable_holder_of_an_up_to_date_branch_is_not_a_refusal() {
        let repo = TestRepo::with_origin();
        repo.git(&["switch", "-q", "-c", "feature"]);
        let holder = repo.path().join("nested-main");
        repo.git(&["worktree", "add", "-q", holder.to_str().unwrap(), "main"]);
        fs::remove_file(holder.join(".git")).unwrap();
        let before = refs(&repo);

        assert_eq!(fast_forward_default(&repo.path(), "main"), FfResult::UpToDate);
        assert_eq!(refs(&repo), before);
    }

    #[test]
    fn refuses_an_invalid_branch_name_without_running_a_mutation() {
        let repo = TestRepo::with_origin();
        let before = refs(&repo);

        assert_eq!(fast_forward_default(&repo.path(), "bad..name"), FfResult::Refused(FfRefusal::Other));
        assert_eq!(fast_forward_default(&repo.path(), "-main"), FfResult::Refused(FfRefusal::Other));
        assert_eq!(refs(&repo), before);
    }
}
