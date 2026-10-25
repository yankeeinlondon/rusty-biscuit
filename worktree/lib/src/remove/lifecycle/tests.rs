//! A confirmed removal against real repositories: one run from "yes" to
//! nothing left, every flowchart edge crashed and finished by a later run
//! without a question, and each kind of target the question can describe.
//!
//! The usage source, mount table, clock, and filesystem faults are injected,
//! so no test sleeps, mounts, or depends on what the host runs.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::*;
use crate::remove::clock::Slept;
use crate::remove::facts::{Caller, Host, Sources, gather};
use crate::remove::faults::{FsOp, NoFaults};
use crate::remove::handoff::{OperationHandoff, canonical};
use crate::remove::recovery::compact::{UnboundTarget, parse};
use sniff::filesystem::query::FileIdentity;
use crate::remove::recovery::lock::LOCKS_DIR;
use crate::remove::recovery::store::compact_fault::Fault;
use crate::remove::recovery::store::{RECORD_FILE, STORE_DIR};
use crate::remove::repair;
use crate::remove::test_support::{ClearUsage, FixedMounts, LiveMounts, ManualClock, MountDuringDeletion, TestRepo, link_dir, replace_with_link};
use crate::worktree::parse_worktree_list;

const BRANCH: &str = "feat/gone";

/// A repository with a linked worktree `gone` on [`BRANCH`] holding a dirty
/// tracked file, an untracked file, and ignored build output.
struct Fixture {
    repo: TestRepo,
    base: PathBuf,
    gone: PathBuf,
}

fn fixture() -> Fixture {
    let repo = TestRepo::new();
    let common = common_git_dir(&repo.path()).unwrap();
    fs::create_dir_all(common.join("info")).unwrap();
    fs::write(common.join("info").join("exclude"), "target/\n").unwrap();
    let gone = repo.add_worktree(BRANCH, "gone", "main");
    fs::write(gone.join("README.md"), "dirty\n").unwrap();
    fs::write(gone.join("notes.txt"), "untracked\n").unwrap();
    for directory in ["target/debug", "target/release"] {
        fs::create_dir_all(gone.join(directory)).unwrap();
        for file in ["a.o", "b.o"] {
            fs::write(gone.join(directory).join(file), "object").unwrap();
        }
    }
    Fixture { base: repo.path(), repo, gone }
}

impl Fixture {
    fn base(&self) -> PathBuf {
        self.base.clone()
    }

    fn listed(&self) -> Vec<WorktreeEntry> {
        parse_worktree_list(&self.repo.git(&["worktree", "list", "--porcelain"]))
    }

    fn entry(&self) -> WorktreeEntry {
        self.listed().into_iter().find(|entry| same_location(&entry.path, &self.gone)).expect("listed")
    }

    /// Whether Git still registers any worktree on [`BRANCH`].
    fn registered(&self) -> bool {
        self.listed().iter().any(|entry| entry.branch.as_deref() == Some(BRANCH))
    }

    /// What the question shows, gathered as `wt remove` does.
    fn facts_with(&self, mounts: &FixedMounts) -> Facts {
        let entry = self.entry();
        let caller = Caller { cwd: self.base(), shell_wrapper: false, shell: None, parent: None };
        gather(&Sources { base: &self.base(), entry: &entry, usage: &ClearUsage, mounts, git: &repair::Git, caller: &caller, host: &Host::default() })
    }

    fn facts(&self) -> Facts {
        self.facts_with(&FixedMounts::default())
    }

    fn tip(&self) -> String {
        self.repo.sha(&format!("refs/heads/{BRANCH}"))
    }

    fn delete_branch(&self, approval: StepApproval) -> ApprovedBranch {
        ApprovedBranch { action: BranchAction::Local { name: BRANCH.into(), expected_tip: self.tip() }, approval }
    }

    fn branch_exists(&self) -> bool {
        self.repo.try_git(&["rev-parse", "--verify", &format!("refs/heads/{BRANCH}")]).is_ok()
    }

    fn store(&self) -> PathBuf {
        common_git_dir(&self.base()).unwrap().join(STORE_DIR)
    }

    /// Everything in the removal store except its locks.
    fn stored(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.store()) else { return Vec::new() };
        let mut names: Vec<String> =
            entries.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).filter(|name| name != LOCKS_DIR).collect();
        names.sort();
        names
    }

    /// The one saved record.
    fn record(&self) -> CompactRecord {
        let [id] = self.stored().try_into().expect("exactly one operation");
        parse(&fs::read(self.store().join(id).join(RECORD_FILE)).unwrap()).unwrap()
    }

    fn pending_id(&self) -> OperationId {
        match resolve(&self.base(), "gone").unwrap() {
            Resolved::Pending(id) => id,
            other => panic!("expected a pending removal, got {other:?}"),
        }
    }

    fn admin_dir(&self) -> PathBuf {
        common_git_dir(&self.base()).unwrap().join("worktrees").join("gone")
    }

    /// Nothing of the worktree is left: no checkout, registration, record,
    /// journal, report, or hidden quarantine.
    fn assert_gone(&self) {
        assert!(absent(&self.gone), "the checkout is gone");
        assert!(!self.registered(), "Git's record is gone");
        assert!(absent(&self.admin_dir()), "the worktree record directory is gone");
        assert_eq!(self.stored(), Vec::<String>::new(), "a finished removal keeps nothing");
        let hidden: Vec<_> = fs::read_dir(self.gone.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".wt-remove-"))
            .collect();
        assert!(hidden.is_empty(), "{hidden:?}");
    }
}

/// Records what the removal told the user.
#[derive(Default)]
struct Notes {
    running: AtomicUsize,
    waits: Mutex<Vec<String>>,
}

impl Notify for Notes {
    fn removal_running(&self) {
        self.running.fetch_add(1, Ordering::SeqCst);
    }

    fn waiting(&self, wait: &Wait) {
        self.waits.lock().unwrap().push(format!("{wait:?}"));
    }
}

/// Every conditional branch is still safe.
struct Safe;

impl Revalidate for Safe {
    fn still_safe(&self, _name: &str, _tip: &str, _remote_deleted: bool) -> Result<(), String> {
        Ok(())
    }
}

fn never() -> bool {
    false
}

/// The injected surroundings of one run.
struct Doubles {
    clock: Box<dyn Clock>,
    faults: Box<dyn FsFaults>,
    mounts: Box<dyn MountInfo>,
    revalidate: Box<dyn Revalidate>,
    notes: Notes,
}

impl Default for Doubles {
    fn default() -> Self {
        Self {
            clock: Box::new(ManualClock::new()),
            faults: Box::new(NoFaults),
            mounts: Box::new(FixedMounts::default()),
            revalidate: Box::new(Safe),
            notes: Notes::default(),
        }
    }
}

impl Doubles {
    fn env(&self) -> Env<'_> {
        Env {
            usage: &ClearUsage,
            mounts: &*self.mounts,
            clock: &*self.clock,
            faults: &*self.faults,
            cancelled: &never,
            revalidate: &*self.revalidate,
            notify: &self.notes,
        }
    }
}

fn request<'a>(f: &'a Fixture, entry: &'a WorktreeEntry, facts: &'a Facts, branches: &'a [ApprovedBranch]) -> Request<'a> {
    Request { base: &f.base, entry, requested_name: "gone", facts, branches }
}

fn finished(removal: Removal) -> Finished {
    match removal {
        Removal::Finished(finished) => *finished,
        Removal::Refused(refusal) => panic!("refused: {refusal}"),
    }
}

fn refused(removal: Removal) -> Refusal {
    match removal {
        Removal::Refused(refusal) => *refusal,
        Removal::Finished(finished) => panic!("finished: {finished:?}"),
    }
}

/// Runs the removal and stops it at `at` as a crash would.
fn crash_at(f: &Fixture, facts: &Facts, branches: &[ApprovedBranch], doubles: &Doubles, at: Boundary) {
    crash_with(f, facts, branches, doubles, at, Faults::default());
}

fn crash_with(f: &Fixture, facts: &Facts, branches: &[ApprovedBranch], doubles: &Doubles, at: Boundary, moves: Faults) {
    let entry = f.entry();
    let mut seams = Seams { hook: Box::new(move |boundary| boundary == at), moves };
    let result = run_with(&request(f, &entry, facts, branches), &doubles.env(), &mut seams);
    assert!(result.is_err(), "the run reached {at:?} and crashed there");
}

/// A file named `name`, anywhere, fails its removal with `kind`: the first
/// `times` attempts, or every one.
struct FailingName {
    name: &'static str,
    kind: io::ErrorKind,
    times: Option<usize>,
    fired: AtomicUsize,
}

impl FailingName {
    fn always(name: &'static str, kind: io::ErrorKind) -> Self {
        Self { name, kind, times: None, fired: AtomicUsize::new(0) }
    }
}

impl FsFaults for FailingName {
    fn inject(&self, op: FsOp, path: &Path) -> Option<io::Error> {
        if op != FsOp::RemoveFile || path.file_name().is_none_or(|name| name != self.name) {
            return None;
        }
        let fired = self.fired.fetch_add(1, Ordering::SeqCst);
        self.times.is_none_or(|times| fired < times).then(|| io::Error::from(self.kind))
    }
}

// --- One run --------------------------------------------------------------

/// The ordinary case: yes deletes the checkout with its dirty, untracked,
/// and ignored files, unregisters it, deletes the approved branch, and
/// leaves nothing behind.
#[test]
fn a_confirmed_checkout_is_deleted_in_one_run_and_leaves_nothing() {
    let f = fixture();
    let facts = f.facts();
    assert!(facts.needs_force(), "the dirty files are a warning");
    let doubles = Doubles::default();
    let entry = f.entry();
    let branches = [f.delete_branch(StepApproval::Explicit)];

    let finished = finished(remove(&request(&f, &entry, &facts, &branches), &doubles.env()));

    assert!(matches!(finished.checkout, CheckoutResult::Removed), "{finished:?}");
    assert_eq!(finished.exit_class(), ExitClass::Success);
    assert!(finished.entries.is_some_and(|entries| entries >= 10), "{:?}", finished.entries);
    assert_eq!(finished.branches, [BranchResult { target: UnboundTarget::Local { name: BRANCH.into() }, outcome: BranchOutcome::Deleted }]);
    assert!(finished.late_changes.is_empty());
    assert!(finished.warnings.is_empty(), "{:?}", finished.warnings);
    assert!(!f.branch_exists());
    f.assert_gone();
    assert!(doubles.notes.waits.lock().unwrap().is_empty());
}

/// R3 / N7: the lock the question showed is removed through Git after the
/// approval is saved, and the worktree is deleted.
#[test]
fn a_locked_worktree_is_unlocked_and_deleted() {
    let f = fixture();
    f.repo.git(&["worktree", "lock", "--reason", "on a USB disk", f.gone.to_str().unwrap()]);
    let facts = f.facts();
    assert!(matches!(facts.warnings.first(), Some(Warning::Locked { reason: Some(reason) }) if reason == "on a USB disk"));
    let doubles = Doubles::default();
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    f.assert_gone();
}

/// A lock the question did not show was never approved: the move refuses,
/// and the worktree and its lock stay as they were.
#[test]
fn a_lock_added_after_the_question_is_never_removed() {
    let f = fixture();
    let facts = f.facts();
    f.repo.git(&["worktree", "lock", "--reason", "mine", f.gone.to_str().unwrap()]);
    let doubles = Doubles::default();
    let entry = f.entry();

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));

    assert!(matches!(&refusal, Refusal::Move(MoveError::Refused(MoveRefusal::Locked { .. }))), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(f.gone.join("notes.txt").is_file());
    assert_eq!(fs::read_to_string(f.admin_dir().join("locked")).unwrap().trim(), "mine");
    assert!(f.stored().is_empty(), "nothing is pending");
}

/// R2: a submodule holding an uncommitted file and a commit on no remote
/// warns about both, and yes deletes it with the worktree, its
/// `modules/<name>` under the worktree's record included. (A clean
/// submodule adding no warning is `facts::tests`'.)
#[test]
fn a_worktree_with_a_submodule_is_deleted_with_its_module_data() {
    let f = fixture();
    let source = TestRepo::new();
    f.repo.git_in(&f.gone, &["-c", "protocol.file.allow=always", "submodule", "add", "-q", source.path().to_str().unwrap(), "sub"]);
    let module = f.admin_dir().join("modules").join("sub");
    assert!(module.is_dir());
    crate::test_support::append_config(
        &module.join("config"),
        &[("user.email", "test@example.com"), ("user.name", "Test User"), ("commit.gpgsign", "false")],
    );
    f.repo.commit_in(&f.gone.join("sub"), "unpushed.txt");
    fs::write(f.gone.join("sub").join("scratch.txt"), "only here").unwrap();
    let facts = f.facts();
    let at_risk: Vec<&Warning> = facts.warnings.iter().filter(|warning| matches!(warning, Warning::SubmoduleAtRisk { .. })).collect();
    let [Warning::SubmoduleAtRisk { path, dirty, unique_commits, .. }] = at_risk.as_slice() else { panic!("{:?}", facts.warnings) };
    assert_eq!((path.as_path(), dirty.as_slice(), *unique_commits), (Path::new("sub"), &[PathBuf::from("scratch.txt")][..], 1));
    let doubles = Doubles::default();
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    assert!(absent(&module), "the submodule's Git data went with the worktree's record");
    f.assert_gone();
    assert!(source.path().join("README.md").is_file(), "the submodule's source repository is untouched");
}

/// R14: a file created after the question is deleted too, and named.
#[test]
fn a_file_created_after_the_question_is_deleted_and_named() {
    let f = fixture();
    let facts = f.facts();
    fs::write(f.gone.join("late.rs"), "written while the question was open").unwrap();
    let doubles = Doubles::default();
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));

    assert_eq!(finished.late_changes, [PathBuf::from("late.rs")]);
    assert_eq!(finished.exit_class(), ExitClass::Success);
    f.assert_gone();
}

/// R14: a tracked file changed and a new file staged while the question was
/// open are deleted and named; a file the question already showed and that
/// changed again is not named twice.
#[test]
fn files_changed_and_staged_while_the_question_was_open_are_deleted_and_named() {
    let f = fixture();
    fs::write(f.gone.join("tracked.rs"), "committed\n").unwrap();
    f.repo.git_in(&f.gone, &["add", "tracked.rs"]);
    f.repo.git_in(&f.gone, &["commit", "-q", "-m", "tracked"]);
    let facts = f.facts();
    fs::write(f.gone.join("tracked.rs"), "changed after the question\n").unwrap();
    fs::write(f.gone.join("staged.rs"), "staged after the question\n").unwrap();
    f.repo.git_in(&f.gone, &["add", "staged.rs"]);
    fs::write(f.gone.join("README.md"), "dirty again\n").unwrap();
    let doubles = Doubles::default();
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));

    let mut named = finished.late_changes.clone();
    named.sort();
    assert_eq!(named, [PathBuf::from("staged.rs"), PathBuf::from("tracked.rs")]);
    assert_eq!(finished.exit_class(), ExitClass::Success);
    f.assert_gone();
}

/// R29: an approval that cannot be saved deletes, unlocks, and moves
/// nothing, and says so.
#[test]
fn an_approval_that_cannot_be_saved_changes_nothing() {
    let f = fixture();
    f.repo.git(&["worktree", "lock", f.gone.to_str().unwrap()]);
    let facts = f.facts();
    let doubles = Doubles::default();
    let entry = f.entry();
    let _fault = crate::remove::recovery::store::compact_fault::inject(|_| Some(Fault::Fail));

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));

    assert!(matches!(refusal, Refusal::NotSaved(_)), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Failure);
    assert!(refusal.to_string().contains("nothing was deleted"), "{refusal}");
    assert!(f.gone.join("target/debug/a.o").is_file());
    assert!(f.admin_dir().join("locked").exists(), "still locked");
    assert!(f.stored().is_empty());
}

/// R29: a permanent failure deletes everything else, is not retried or
/// waited on, ends with one plain sentence, and a later run finishes
/// without a question once the cause is gone.
#[test]
fn a_permanent_failure_deletes_the_rest_and_a_later_run_finishes() {
    let f = fixture();
    let facts = f.facts();
    let failing = FailingName::always("notes.txt", io::ErrorKind::ReadOnlyFilesystem);
    let doubles = Doubles { faults: Box::new(failing), ..Doubles::default() };
    let entry = f.entry();
    let branches = [f.delete_branch(StepApproval::Explicit)];

    let first = finished(remove(&request(&f, &entry, &facts, &branches), &doubles.env()));

    let CheckoutResult::Permanent { failures, .. } = &first.checkout else { panic!("{first:?}") };
    let [failure] = failures.as_slice() else { panic!("{failures:?}") };
    assert!(failure.path.ends_with("notes.txt"));
    assert!(first.checkout.to_string().ends_with("notes.txt could not be deleted: the disk is read-only"), "{}", first.checkout);
    assert_eq!(first.exit_class(), ExitClass::Failure);
    assert!(first.branches.is_empty(), "branch steps wait for the checkout");
    assert!(f.branch_exists());
    let deletion = f.record().deletion_path.as_path().to_path_buf();
    let left: Vec<_> = fs::read_dir(&deletion).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(left, ["notes.txt"], "only the failed entry is left");

    let second = finished(finish_pending(&f.base(), &f.pending_id(), &Doubles::default().env()));

    assert_eq!(second.exit_class(), ExitClass::Success, "{second:?}");
    assert!(!f.branch_exists());
    f.assert_gone();
}

/// R22: a file another program holds is waited on after everything else
/// is deleted, the wait is shown once, and the removal finishes in the same
/// run once it is released.
#[test]
fn a_held_file_is_waited_on_and_the_removal_finishes_in_the_same_run() {
    let f = fixture();
    let facts = f.facts();
    let held = FailingName { times: Some(2), ..FailingName::always("notes.txt", io::ErrorKind::WouldBlock) };
    let doubles = Doubles { faults: Box::new(held), ..Doubles::default() };
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    let waits = doubles.notes.waits.lock().unwrap().clone();
    assert_eq!(waits.len(), 1, "{waits:?}");
    assert!(waits[0].contains("notes.txt"), "{waits:?}");
    f.assert_gone();
}

/// R7: a nested mount is never crossed, and everything else goes; the
/// registration, record, and branch stay until it is unmounted, then a
/// later run finishes without a question. The table is injected, with a
/// same-device bind mount.
#[test]
fn a_nested_mount_is_kept_until_unmounted_and_then_finished() {
    let f = fixture();
    fs::create_dir_all(f.gone.join("data").join("cache")).unwrap();
    fs::write(f.gone.join("data").join("cache").join("blob"), "on the other filesystem").unwrap();
    let real = biscuit_file::canonicalize_simplified(&f.gone).unwrap();
    let mount = Mount { mount_point: real.join("data"), source: "/srv/data".into(), fs_type: "ext4".into(), super_options: String::new() };
    let mounted = FixedMounts(vec![mount]);
    let facts = f.facts_with(&mounted);
    assert!(facts.warnings.iter().any(|warning| matches!(warning, Warning::NestedMounts { .. })));
    let doubles = Doubles { mounts: Box::new(mounted), ..Doubles::default() };
    let entry = f.entry();
    let branches = [f.delete_branch(StepApproval::Explicit)];

    let first = finished(remove(&request(&f, &entry, &facts, &branches), &doubles.env()));

    assert!(matches!(&first.checkout, CheckoutResult::RetainedMounts(mounts) if mounts.len() == 1), "{first:?}");
    assert_eq!(first.exit_class(), ExitClass::Failure);
    assert!(first.checkout.to_string().contains("unmount it"), "{}", first.checkout);
    assert!(f.gone.join("data").join("cache").join("blob").is_file(), "nothing on the mount is deleted");
    assert!(!f.gone.join("notes.txt").exists() && !f.gone.join("target").exists(), "everything else is");
    assert!(f.registered() && f.branch_exists());
    assert_eq!(f.record().deletion_mode, DeletionMode::InPlace, "a nested mount is deleted in place, never moved");

    let second = finished(finish_pending(&f.base(), &f.pending_id(), &Doubles::default().env()));

    assert_eq!(second.exit_class(), ExitClass::Success, "{second:?}");
    assert!(!f.branch_exists());
    f.assert_gone();
}

/// Doubles whose mount table gains a same-device mount at `target/debug`
/// once deletion has begun, before deletion reaches it; the table is
/// returned to unmount it.
fn mounted_during_deletion() -> (Doubles, LiveMounts) {
    let live = LiveMounts::default();
    let publish = MountDuringDeletion::new(&live, Path::new("target").join("debug"));
    (Doubles { faults: Box::new(publish), mounts: Box::new(live.clone()), ..Doubles::default() }, live)
}

/// Asserts that only `target/debug` under `root` was kept, as a mount.
fn assert_kept_mount(result: &Finished, root: &Path) {
    let point = root.join("target").join("debug");
    assert!(
        matches!(&result.checkout, CheckoutResult::RetainedMounts(mounts) if mounts.iter().map(|mount| &mount.mount_point).eq([&point])),
        "{result:?}"
    );
    assert_eq!(result.exit_class(), ExitClass::Failure);
    assert!(point.join("a.o").is_file(), "nothing on the mount is deleted");
    assert!(!root.join("notes.txt").exists() && !root.join("target").join("release").exists(), "everything else is");
}

/// R7, R2-F3.1: a mount made while a moved checkout is deleted is kept with
/// the registration, record, and branch; once it is unmounted, a later run
/// finishes without a question.
#[test]
fn a_mount_made_while_a_moved_checkout_is_deleted_is_kept_until_unmounted() {
    let f = fixture();
    let facts = f.facts();
    let (doubles, live) = mounted_during_deletion();
    let entry = f.entry();
    let branches = [f.delete_branch(StepApproval::Explicit)];

    let first = finished(remove(&request(&f, &entry, &facts, &branches), &doubles.env()));

    let record = f.record();
    assert_eq!(record.deletion_mode, DeletionMode::Moved, "no mount was there to keep it in place");
    assert_kept_mount(&first, record.deletion_path.as_path());
    assert!(f.registered() && f.branch_exists());

    live.unmount_all();
    let second = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

    assert_eq!(second.exit_class(), ExitClass::Success, "{second:?}");
    assert!(!f.branch_exists());
    f.assert_gone();
}

/// A rename failure that deletes in place: no exclusive rename on Unix
/// (drvfs), a held file on Windows.
fn in_place_rename_failure() -> io::Error {
    #[cfg(unix)]
    return io::Error::from_raw_os_error(libc::EINVAL);
    #[cfg(windows)]
    return io::Error::from_raw_os_error(windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION as i32);
}

/// R7, R2-F3.2: a saved in-place removal that resumes while a mount is made
/// keeps the mount, the registration, the branch, and the approval.
#[test]
fn a_mount_made_while_a_saved_in_place_removal_resumes_keeps_the_approval() {
    let f = fixture();
    let facts = f.facts();
    let branches = [f.delete_branch(StepApproval::Explicit)];
    let moves = Faults { rename_instead: Some(in_place_rename_failure), ..Faults::default() };
    crash_with(&f, &facts, &branches, &Doubles::default(), Boundary::MarkedInPlace, moves);
    assert_eq!(f.record().deletion_mode, DeletionMode::InPlace);
    let id = f.pending_id();
    let (doubles, live) = mounted_during_deletion();

    let first = finished(finish_pending(&f.base(), &id, &doubles.env()));

    assert_kept_mount(&first, f.record().deletion_path.as_path());
    assert!(f.registered() && f.branch_exists());
    assert_eq!(f.pending_id(), id, "the approval is kept");

    live.unmount_all();
    let second = finished(finish_pending(&f.base(), &id, &doubles.env()));

    assert_eq!(second.exit_class(), ExitClass::Success, "{second:?}");
    assert!(!f.branch_exists());
    f.assert_gone();
}

/// R7, R2-F3.3: deleting an own link's checkout never crosses a mount made
/// during it; the link and the approval are kept, and a later run finishes.
#[test]
fn a_mount_made_while_an_own_links_checkout_is_deleted_keeps_the_link_and_approval() {
    let f = fixture();
    let real = replace_with_link(&f.gone, "real");
    let facts = f.facts();
    let (doubles, live) = mounted_during_deletion();
    let entry = f.entry();

    let first = finished(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));

    let record = f.record();
    assert!(matches!(record.target, ApprovedTarget::OwnLink { .. }), "{:?}", record.target);
    assert_kept_mount(&first, record.deletion_path.as_path());
    assert!(fs::symlink_metadata(&f.gone).is_ok(), "the link is kept");
    assert!(f.registered());
    let id = f.pending_id();

    live.unmount_all();
    let second = finished(finish_pending(&f.base(), &id, &doubles.env()));

    assert_eq!(second.exit_class(), ExitClass::Success, "{second:?}");
    assert!(absent(&real));
    f.assert_gone();
}

/// R4: a filesystem without a no-replacement rename (drvfs) deletes in
/// place under the same record; nothing is renamed by a check-then-move.
#[cfg(unix)]
#[test]
fn a_filesystem_without_exclusive_rename_deletes_in_place() {
    let f = fixture();
    let facts = f.facts();
    let doubles = Doubles::default();
    let moves = Faults { rename_instead: Some(|| io::Error::from_raw_os_error(libc::EINVAL)), ..Faults::default() };
    crash_with(&f, &facts, &[], &doubles, Boundary::MarkedInPlace, moves);

    let record = f.record();
    assert_eq!(record.deletion_mode, DeletionMode::InPlace);
    assert_eq!(record.deletion_path, record.original);
    assert!(f.gone.join("notes.txt").is_file(), "marked, not yet deleted");

    let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));
    assert_eq!(finished.exit_class(), ExitClass::Success);
    f.assert_gone();
}

/// R15 (Windows): the question's blocker refuses before anything changes.
#[test]
fn the_shell_inside_blocker_refuses_with_exit_4() {
    let f = fixture();
    let mut facts = f.facts();
    facts.blocker = Some(crate::remove::facts::Blocker::ShellInside { shell: Some("pwsh".into()) });
    let entry = f.entry();

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_eq!(refusal.exit_class(), ExitClass::Blocked);
    assert!(f.gone.join("notes.txt").is_file());
}

/// The main checkout is never removed, whatever was answered.
#[test]
fn the_main_checkout_is_never_removed() {
    let f = fixture();
    let facts = f.facts();
    let main = f.listed().into_iter().find(|entry| entry.is_main).unwrap();

    let refusal = refused(remove(&request(&f, &main, &facts, &[]), &Doubles::default().env()));

    assert!(matches!(refusal, Refusal::MainCheckout));
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(f.base().join(".git").exists());
}

// --- Crashes, and the run that finishes ----------------------------------

/// Every edge of the flowchart: a crash there is finished by the next
/// `wt remove` of the same name, with the saved approval, asking nothing.
/// The worktree is locked and has a branch step so that every edge is
/// passed.
#[test]
fn a_crash_at_every_step_is_finished_by_the_next_run() {
    for at in [
        Boundary::Begun,
        Boundary::Unlocked,
        Boundary::Moved,
        Boundary::ContentRemoved,
        Boundary::Unregistered,
        Boundary::BranchAction(0),
        Boundary::BranchesDone,
    ] {
        let f = fixture();
        f.repo.git(&["worktree", "lock", f.gone.to_str().unwrap()]);
        let facts = f.facts();
        let doubles = Doubles::default();
        let branches = [f.delete_branch(StepApproval::Explicit)];
        crash_at(&f, &facts, &branches, &doubles, at);
        assert_eq!(f.stored().len(), 1, "{at:?}: the record survives the crash");

        let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

        assert_eq!(finished.exit_class(), ExitClass::Success, "{at:?}: {finished:?}");
        assert!(!f.branch_exists(), "{at:?}");
        f.assert_gone();
    }
}

/// A built checkout spans several deletion batches. A crash during
/// deletion leaves the placed checkout partly deleted; the next run deletes
/// the rest from the saved approval, asking nothing, and counts only what it
/// deleted itself.
#[test]
fn a_crash_part_way_through_deleting_a_built_checkout_is_finished_by_the_next_run() {
    let f = fixture();
    let build = f.gone.join("target").join("build");
    fs::create_dir_all(&build).unwrap();
    let built = 2 * crate::remove::delete::BATCH_ENTRIES + 100;
    for n in 0..built {
        fs::write(build.join(format!("{n}.o")), "o").unwrap();
    }
    let facts = f.facts();
    let doubles = Doubles::default();
    let branches = [f.delete_branch(StepApproval::Explicit)];
    crash_at(&f, &facts, &branches, &doubles, Boundary::Moved);
    let record = f.record();
    assert_eq!(record.progress, Progress::Placed);
    // What the first batch deleted before the crash.
    let moved_build = record.deletion_path.as_path().join("target").join("build");
    for n in 0..crate::remove::delete::BATCH_ENTRIES {
        fs::remove_file(moved_build.join(format!("{n}.o"))).unwrap();
    }
    let left = fs::read_dir(&moved_build).unwrap().count() as u64;

    let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    let entries = finished.entries.expect("counted");
    assert!(entries > left && entries < built, "only this run's deletions count: {entries}, {left} built files were left");
    assert!(!f.branch_exists());
    f.assert_gone();
}

/// A crash between the rename and Git's record: the checkout is moved
/// back and placed again, without a question.
#[test]
fn a_crash_between_the_rename_and_the_registration_is_finished() {
    let f = fixture();
    let facts = f.facts();
    let doubles = Doubles::default();
    crash_at(&f, &facts, &[], &doubles, Boundary::Begun);
    let record = f.record();
    fs::rename(&f.gone, record.deletion_path.as_path()).unwrap();

    let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    f.assert_gone();
}

/// New content at the original path after the move is never deleted; the
/// moved checkout still is, and the run says what was kept.
#[test]
fn a_recreated_original_is_kept_and_the_moved_checkout_deleted() {
    let f = fixture();
    let facts = f.facts();
    let doubles = Doubles::default();
    crash_at(&f, &facts, &[], &doubles, Boundary::Moved);
    fs::create_dir(&f.gone).unwrap();
    fs::write(f.gone.join("new.txt"), "someone else's").unwrap();

    let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    assert!(finished.original_kept.as_deref().is_some_and(|kept| same_location(kept, &f.gone)), "{finished:?}");
    assert_eq!(fs::read_to_string(f.gone.join("new.txt")).unwrap(), "someone else's");
    assert!(!f.registered());
    assert!(f.stored().is_empty());
}

/// A different directory at the hidden path is never deleted; the record
/// is kept so the next run explains it again.
#[test]
fn a_replaced_quarantine_is_never_deleted() {
    let f = fixture();
    let facts = f.facts();
    let doubles = Doubles::default();
    crash_at(&f, &facts, &[], &doubles, Boundary::Moved);
    let quarantine = f.record().deletion_path.as_path().to_path_buf();
    fs::rename(&quarantine, f.gone.with_file_name("moved-away")).unwrap();
    fs::create_dir(&quarantine).unwrap();
    fs::write(quarantine.join("stranger.txt"), "not the checkout").unwrap();

    let refusal = refused(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

    assert_eq!(refusal.exit_class(), ExitClass::Refused, "{refusal}");
    assert!(refusal.to_string().contains(&quarantine.display().to_string()), "names the path: {refusal}");
    assert!(quarantine.join("stranger.txt").is_file());
    assert_eq!(f.stored().len(), 1);
}

/// In place, a replaced root is outside the approval: it is kept, and the
/// operation closes so a later run asks afresh.
#[cfg(unix)]
#[test]
fn a_replaced_in_place_root_is_kept_and_the_operation_closed() {
    let f = fixture();
    let facts = f.facts();
    let doubles = Doubles::default();
    let moves = Faults { rename_instead: Some(|| io::Error::from_raw_os_error(libc::EINVAL)), ..Faults::default() };
    crash_with(&f, &facts, &[], &doubles, Boundary::MarkedInPlace, moves);
    let id = f.pending_id();
    fs::rename(&f.gone, f.gone.with_file_name("elsewhere")).unwrap();
    fs::create_dir(&f.gone).unwrap();
    fs::write(f.gone.join("stranger.txt"), "not the checkout").unwrap();

    let refusal = refused(finish_pending(&f.base(), &id, &doubles.env()));

    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(f.gone.join("stranger.txt").is_file());
    assert!(f.stored().is_empty());
}

/// A worktree record reused for another worktree is never resumed into.
#[test]
fn a_replaced_worktree_record_is_never_resumed() {
    let f = fixture();
    let facts = f.facts();
    let doubles = Doubles::default();
    crash_at(&f, &facts, &[], &doubles, Boundary::ContentRemoved);
    let id = f.pending_id();
    let admin = f.admin_dir();
    fs::rename(&admin, admin.with_file_name("gone-old")).unwrap();
    fs::create_dir(&admin).unwrap();
    fs::write(admin.join("gitdir"), "elsewhere\n").unwrap();

    let refusal = refused(finish_pending(&f.base(), &id, &doubles.env()));

    assert_eq!(refusal.exit_class(), ExitClass::Refused, "{refusal}");
    assert!(admin.join("gitdir").is_file());
    assert_eq!(f.stored().len(), 1);
}

// --- Branches -------------------------------------------------------------

/// A branch that moved after it was approved is kept and reported; the
/// checkout is deleted all the same and nothing is asked.
#[test]
fn a_branch_that_moved_is_kept_after_the_checkout_is_deleted() {
    let f = fixture();
    let facts = f.facts();
    let branches = [f.delete_branch(StepApproval::Explicit)];
    let moved = f.repo.commit_in(&f.gone, "later.txt");
    let doubles = Doubles::default();
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &branches), &doubles.env()));

    assert!(matches!(&finished.branches[0].outcome, BranchOutcome::Kept(why) if why.contains(&moved)), "{finished:?}");
    assert_eq!(finished.exit_class(), ExitClass::Failure);
    assert_eq!(f.tip(), moved);
    f.assert_gone();
}

/// A branch already gone completes its step.
#[test]
fn an_absent_branch_completes_its_step() {
    let f = fixture();
    let facts = f.facts();
    let branches = [ApprovedBranch {
        action: BranchAction::Local { name: "feat/never".into(), expected_tip: f.tip() },
        approval: StepApproval::Explicit,
    }];
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &branches), &Doubles::default().env()));

    assert_eq!(finished.branches[0].outcome, BranchOutcome::AlreadyAbsent);
    assert_eq!(finished.exit_class(), ExitClass::Success);
}

/// A conditional step is rechecked by a later run: kept when it is no
/// longer safe, deleted when it is.
#[test]
fn a_conditional_branch_is_rechecked_by_a_later_run() {
    for (revalidate, deleted) in [(Box::new(branches::Unverifiable) as Box<dyn Revalidate>, false), (Box::new(Safe), true)] {
        let f = fixture();
        let facts = f.facts();
        let doubles = Doubles { revalidate, ..Doubles::default() };
        let branches = [f.delete_branch(StepApproval::Conditional)];
        crash_at(&f, &facts, &branches, &doubles, Boundary::Unregistered);

        let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

        assert_eq!(finished.branches[0].outcome.is_done(), deleted, "{finished:?}");
        assert_eq!(f.branch_exists(), !deleted);
        assert_eq!(finished.exit_class() == ExitClass::Success, deleted);
        assert!(f.stored().is_empty(), "a kept branch finishes the operation too");
    }
}

// --- Concurrent runs, names, and the handoff ------------------------------

/// A clock that releases a held lock at its first sleep.
struct Releasing {
    held: Mutex<Option<OperationLock>>,
}

impl Clock for Releasing {
    fn sleep(&self, _duration: Duration, _cancelled: &dyn Fn() -> bool) -> Slept {
        self.held.lock().unwrap().take();
        Slept::Elapsed
    }
}

/// R16: a run that finds another removal of the worktree running says so,
/// waits for its lock, and then continues what that run left.
#[test]
fn a_second_run_waits_for_the_running_removal_and_continues_it() {
    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Moved);
    let store = RecoveryStore::open(&common_git_dir(&f.base()).unwrap()).unwrap();
    let held = store.lock(&f.record().repository).unwrap();
    let doubles = Doubles { clock: Box::new(Releasing { held: Mutex::new(Some(held)) }), ..Doubles::default() };

    let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

    assert_eq!(doubles.notes.running.load(Ordering::SeqCst), 1);
    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    f.assert_gone();
}

/// R16: the wait for the lock ends on Ctrl-C with nothing more deleted.
#[test]
fn the_wait_for_a_running_removal_honors_cancellation() {
    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Moved);
    let store = RecoveryStore::open(&common_git_dir(&f.base()).unwrap()).unwrap();
    let _held = store.lock(&f.record().repository).unwrap();
    let doubles = Doubles { clock: Box::new(ManualClock::cancelled_after(2)), ..Doubles::default() };

    let refusal = refused(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

    assert!(matches!(refusal, Refusal::Cancelled), "{refusal:?}");
    assert!(f.record().deletion_path.as_path().join("notes.txt").is_file());
}

/// A fresh run of a worktree whose earlier removal exited finishes that one
/// instead of saving a second approval.
#[test]
fn a_fresh_run_continues_the_removal_an_earlier_run_left() {
    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Begun);
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    f.assert_gone();
}

/// A name that matches a pending removal and a different live worktree is
/// ambiguous; each alone resolves to itself.
#[test]
fn a_name_matching_a_pending_removal_and_another_worktree_is_ambiguous() {
    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Moved);
    assert!(matches!(resolve(&f.base(), "gone").unwrap(), Resolved::Pending(_)));
    assert!(matches!(resolve(&f.base(), BRANCH).unwrap(), Resolved::Pending(_)), "the branch is an alias");

    f.repo.add_worktree("gone", "other", "main");

    match resolve(&f.base(), "gone").unwrap() {
        Resolved::Ambiguous(candidates) => assert_eq!(candidates.len(), 2),
        other => panic!("{other:?}"),
    }
    assert!(matches!(resolve(&f.base(), "other").unwrap(), Resolved::Live(_)));
}

/// A legacy operation an earlier `wt` finished but kept a record of is
/// deleted when a name is resolved, and its name then means only the live
/// worktree.
#[test]
fn a_finished_legacy_operation_is_discarded_and_matches_no_name() {
    use crate::remove::recovery::record::{BranchBinding, IntendedAction, Phase, RECORD_VERSION, RecoveryRecord};

    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Begun);
    let compact = f.record();
    let legacy = RecoveryRecord {
        record_version: RECORD_VERSION,
        operation_id: compact.operation_id.clone(),
        created_at_ms: compact.created_at_ms,
        requested_name: compact.requested_name.clone(),
        aliases: compact.aliases.clone(),
        original: compact.original.clone(),
        quarantine: compact.deletion_path.clone(),
        checkout_identity: compact.target.checkout_identity().unwrap(),
        repository: compact.repository.clone(),
        branch: BranchBinding::Named(BRANCH.into()),
        approved_tip: f.tip(),
        consent_fingerprint: "b".repeat(64),
        discovery: crate::remove::test_support::clear_report(&f.gone),
        intended_actions: vec![IntendedAction::RemoveCheckout],
        completed_phase: Phase::Completed,
    };
    let dir = f.store().join(compact.operation_id.as_str());
    fs::write(dir.join(RECORD_FILE), legacy.to_json()).unwrap();

    assert!(matches!(resolve(&f.base(), "gone").unwrap(), Resolved::Live(_)));
    assert!(f.stored().is_empty(), "{:?}", f.stored());
}

/// `wt list` and `--ff` learn which checkouts a removal is working on, at
/// the original path before the move and at the hidden one after it.
#[test]
fn pending_removals_name_the_checkout_at_either_path() {
    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Begun);
    let entries = crate::worktree::parse_worktree_list(&crate::git::worktree_list_from(&f.base(), &f.base()).unwrap());
    let pending = PendingRemovals::read(&f.base(), &entries);
    assert_eq!(pending.name_for(&f.entry().path), Some("gone"), "unmoved");
    assert_eq!(pending.name_for(&f.base()), None, "the main checkout");

    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Moved);
    let moved = f.record().deletion_path.as_path().to_path_buf();
    let entries = crate::worktree::parse_worktree_list(&crate::git::worktree_list_from(&f.base(), &f.base()).unwrap());
    let listed = entries.iter().find(|entry| same_location(&entry.path, &moved)).expect("Git lists the moved checkout");
    assert_eq!(PendingRemovals::read(&f.base(), &entries).name_for(&listed.path), Some("gone"), "moved");
}

/// R18, R14, R2-F4.1: the first run only saves its approval
/// ([`save_approval`]); the handoff's second run finishes that operation
/// once the caller has left the worktree, without comparing its content,
/// and names the file created after the question but not one it showed.
#[test]
fn the_handoff_second_run_finishes_once_the_caller_left() {
    let f = fixture();
    let facts = f.facts();
    let entry = f.entry();
    let id = save_approval(&request(&f, &entry, &facts, &[]), &Doubles::default().env()).unwrap();
    assert_eq!(f.record().progress, Progress::Approved, "the first run only saves its approval");
    assert!(f.gone.join("notes.txt").is_file());
    let handoff = OperationHandoff::new(id, canonical(&f.base()), canonical(&f.gone), 0, late::shown(facts.inventory.as_ref()));
    fs::write(f.gone.join("after-question.txt"), "content is not compared").unwrap();
    fs::write(f.gone.join("README.md"), "shown, and changed again\n").unwrap();

    let inside = refused(finish_handoff(&f.base(), &handoff, &f.gone.join("target"), &Doubles::default().env()));
    assert_eq!(inside.exit_class(), ExitClass::Refused);
    assert!(f.gone.join("notes.txt").is_file());

    let finished = finished(finish_handoff(&f.base(), &handoff, &f.base(), &Doubles::default().env()));
    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    assert_eq!(finished.late_changes, [PathBuf::from("after-question.txt")]);
    f.assert_gone();
}

/// R14, R2-F4.1, R2-F4.4: with no question's list (a handoff whose question
/// showed no inventory, or an interrupted removal finished later) a file
/// created since is still deleted, and nothing is named.
#[test]
fn without_the_questions_list_late_files_are_deleted_unnamed() {
    let f = fixture();
    let facts = f.facts();
    let entry = f.entry();
    let id = save_approval(&request(&f, &entry, &facts, &[]), &Doubles::default().env()).unwrap();
    fs::write(f.gone.join("after-question.txt"), "late").unwrap();
    let handoff = OperationHandoff::new(id, canonical(&f.base()), canonical(&f.gone), 0, None);

    let handed_off = finished(finish_handoff(&f.base(), &handoff, &f.base(), &Doubles::default().env()));

    assert_eq!(handed_off.exit_class(), ExitClass::Success, "{handed_off:?}");
    assert!(handed_off.late_changes.is_empty(), "{:?}", handed_off.late_changes);
    f.assert_gone();

    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Moved);
    let moved = f.record().deletion_path.as_path().to_path_buf();
    fs::write(moved.join("after-crash.txt"), "late").unwrap();

    let resumed = finished(finish_pending(&f.base(), &f.pending_id(), &Doubles::default().env()));

    assert_eq!(resumed.exit_class(), ExitClass::Success, "{resumed:?}");
    assert!(resumed.late_changes.is_empty(), "{:?}", resumed.late_changes);
    f.assert_gone();
}

/// R14, R2-F4.3: a checkout deleted in place names a file created after the
/// question, as a moved one does.
#[test]
fn a_file_created_after_the_question_is_named_when_deleted_in_place() {
    let f = fixture();
    let facts = f.facts();
    fs::write(f.gone.join("late.rs"), "written while the question was open").unwrap();
    let entry = f.entry();
    let moves = Faults { rename_instead: Some(in_place_rename_failure), ..Faults::default() };
    let passed = Mutex::new(Vec::new());
    let mut seams = Seams {
        hook: Box::new(|boundary| {
            passed.lock().unwrap().push(boundary);
            false
        }),
        moves,
    };

    let finished = finished(run_with(&request(&f, &entry, &facts, &[]), &Doubles::default().env(), &mut seams).unwrap());

    drop(seams);
    let passed = passed.into_inner().unwrap();
    assert!(passed.contains(&Boundary::MarkedInPlace) && !passed.contains(&Boundary::Moved), "{passed:?}");
    assert_eq!(finished.late_changes, [PathBuf::from("late.rs")]);
    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    f.assert_gone();
}

// --- Targets without a checkout directory ---------------------------------

/// R12 / R3: a locked worktree whose directory is gone is unlocked and its
/// record removed; its branch step runs.
#[test]
fn a_missing_locked_worktree_has_its_record_removed() {
    let f = fixture();
    f.repo.git(&["worktree", "lock", f.gone.to_str().unwrap()]);
    fs::remove_dir_all(&f.gone).unwrap();
    let facts = f.facts();
    assert_eq!(facts.target, Target::Missing);
    let branches = [f.delete_branch(StepApproval::Explicit)];
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &branches), &Doubles::default().env()));

    assert!(matches!(finished.checkout, CheckoutResult::AlreadyRemoved), "{finished:?}");
    assert_eq!(finished.exit_class(), ExitClass::Success);
    assert!(!f.branch_exists());
    f.assert_gone();
}

/// R11: a file where the directory should be is deleted with Git's record.
#[test]
fn a_file_at_the_path_is_deleted_with_the_record() {
    let f = fixture();
    fs::remove_dir_all(&f.gone).unwrap();
    fs::write(&f.gone, "not a checkout").unwrap();
    let facts = f.facts();
    assert_eq!(facts.target, Target::NotADirectory);
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    f.assert_gone();
}

/// R11 / R3: a locked worktree whose path holds a file is gathered as the
/// file, not as a checkout: yes unlocks it and deletes the file with Git's
/// record. (Git does not mark a locked entry prunable.)
#[test]
fn a_file_at_a_locked_worktrees_path_is_unlocked_and_deleted() {
    let f = fixture();
    f.repo.git(&["worktree", "lock", "--reason", "on a USB disk", f.gone.to_str().unwrap()]);
    fs::remove_dir_all(&f.gone).unwrap();
    fs::write(&f.gone, "not a checkout").unwrap();
    let facts = f.facts();
    assert_eq!(facts.target, Target::NotADirectory);
    assert!(matches!(facts.warnings.first(), Some(Warning::Locked { reason: Some(reason) }) if reason == "on a USB disk"));
    assert!(facts.warnings.iter().any(|warning| matches!(warning, Warning::PathOccupied { .. })), "{:?}", facts.warnings);
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    f.assert_gone();
}

/// R9: a link to the worktree's own checkout deletes both.
#[test]
fn a_link_to_the_own_checkout_deletes_both() {
    let f = fixture();
    let real = replace_with_link(&f.gone, "real");
    let facts = f.facts();
    assert!(matches!(facts.target, Target::Link { own: true, .. }), "{:?}", facts.target);
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    assert!(absent(&real), "the real checkout is deleted");
    f.assert_gone();
}

/// R14, R2-F4.2: an own link's question lists the real checkout's files,
/// so a file created there after the question is named, and one it showed
/// is not.
#[test]
fn a_file_created_in_an_own_links_checkout_after_the_question_is_named() {
    let f = fixture();
    let real = replace_with_link(&f.gone, "real");
    let facts = f.facts();
    assert!(facts.inventory.as_ref().is_some_and(|inventory| !inventory.dirty.is_empty()), "the real checkout's files are listed");
    fs::write(real.join("after-question.txt"), "late").unwrap();
    fs::write(real.join("README.md"), "shown, and changed again\n").unwrap();
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    assert_eq!(finished.late_changes, [PathBuf::from("after-question.txt")]);
    assert!(absent(&real));
    f.assert_gone();
}

/// R9: a link to anything else deletes only the link and Git's record.
#[test]
fn a_link_to_another_directory_deletes_only_the_link_and_the_record() {
    let f = fixture();
    fs::remove_dir_all(&f.gone).unwrap();
    let unrelated = f.gone.with_file_name("unrelated");
    fs::create_dir(&unrelated).unwrap();
    fs::write(unrelated.join("keep.txt"), "never deleted").unwrap();
    link_dir(&unrelated, &f.gone);
    let facts = f.facts();
    assert!(matches!(facts.target, Target::Link { own: false, .. }), "{:?}", facts.target);
    let entry = f.entry();

    let finished = finished(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_eq!(finished.exit_class(), ExitClass::Success, "{finished:?}");
    assert_eq!(fs::read_to_string(unrelated.join("keep.txt")).unwrap(), "never deleted");
    f.assert_gone();
}

// --- What the question showed is what is removed ---------------------------

/// The identity of whatever is at `path`, without following a link.
fn identity_at(path: &Path) -> FileIdentity {
    rename::identity(path).unwrap()
}

fn assert_changed(refusal: &Refusal, path: &Path) {
    assert!(matches!(refusal, Refusal::Replaced(_)), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    // Git spells a Windows worktree path with forward slashes.
    let sentence = refusal.to_string().replace('\\', "/");
    let named = path.display().to_string().replace('\\', "/");
    assert!(sentence.contains(&named) && sentence.contains("after the question"), "{sentence}");
}

/// Removes the link at `path` (a junction on Windows) and puts a file there.
fn replace_link_with_file(path: &Path) {
    remove_link(path).unwrap();
    fs::write(path, "a file that was never shown").unwrap();
}

/// R2-F1.1, and the handoff consumer: a checkout directory swapped for
/// another after the question, even one carrying the original `.git` link,
/// is not the approved checkout. Neither a removal nor a handoff touches it.
#[test]
fn a_checkout_replaced_after_the_question_is_refused_and_kept() {
    let f = fixture();
    let facts = f.facts();
    let entry = f.entry();
    let aside = f.gone.with_file_name("aside");
    fs::rename(&f.gone, &aside).unwrap();
    fs::create_dir(&f.gone).unwrap();
    fs::copy(aside.join(".git"), f.gone.join(".git")).unwrap();
    fs::write(f.gone.join("stranger.txt"), "never shown").unwrap();
    let doubles = Doubles::default();

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &doubles.env()));
    let handoff = save_approval(&request(&f, &entry, &facts, &[]), &doubles.env()).unwrap_err();

    assert_changed(&refusal, &f.gone);
    assert_changed(&handoff, &f.gone);
    assert!(f.gone.join("stranger.txt").is_file());
    assert!(aside.join("target/debug/a.o").is_file());
    assert!(f.registered());
    assert!(f.stored().is_empty(), "nothing was saved");
}

/// R2-F1.2: a checkout Git cannot read is bound by identity too, so a
/// directory put in its place after the question is kept.
#[test]
fn an_unreadable_checkout_replaced_after_the_question_is_refused_and_kept() {
    let f = fixture();
    fs::write(f.gone.join(".git"), "not a gitdir link\n").unwrap();
    let facts = f.facts();
    let unreadable = |warning: &Warning| matches!(warning, Warning::GitUnreadable { .. } | Warning::Unchecked { .. });
    assert!(facts.warnings.iter().any(unreadable), "{:?}", facts.warnings);
    let entry = f.entry();
    fs::rename(&f.gone, f.gone.with_file_name("aside")).unwrap();
    fs::create_dir(&f.gone).unwrap();
    fs::write(f.gone.join("stranger.txt"), "never shown").unwrap();

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_changed(&refusal, &f.gone);
    assert!(f.gone.join("stranger.txt").is_file());
    assert!(f.registered());
}

/// R2-F1.3: the checkout an own link led to, replaced after the question
/// while the link stays, is kept, and so is the link.
#[test]
fn an_own_links_checkout_replaced_after_the_question_is_kept() {
    let f = fixture();
    let real = replace_with_link(&f.gone, "real");
    let facts = f.facts();
    assert!(matches!(facts.target, Target::Link { own: true, .. }), "{:?}", facts.target);
    let entry = f.entry();
    fs::rename(&real, real.with_file_name("real-old")).unwrap();
    fs::create_dir(&real).unwrap();
    fs::write(real.join("stranger.txt"), "never shown").unwrap();
    let link = identity_at(&f.gone);

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    let Target::Link { resolved: LinkTarget::Resolved(resolved), .. } = &facts.target else { unreachable!() };
    assert_changed(&refusal, resolved);
    assert!(real.join("stranger.txt").is_file());
    assert!(real.with_file_name("real-old").join("notes.txt").is_file());
    assert_eq!(identity_at(&f.gone), link, "the link is kept");
    assert!(f.registered());
}

/// R2-F1.4: a file at the checkout path swapped for another file after the
/// question is kept.
#[test]
fn a_file_replaced_after_the_question_is_kept() {
    let f = fixture();
    fs::remove_dir_all(&f.gone).unwrap();
    fs::write(&f.gone, "shown").unwrap();
    let facts = f.facts();
    assert_eq!(facts.target, Target::NotADirectory);
    let entry = f.entry();
    fs::rename(&f.gone, f.gone.with_file_name("aside")).unwrap();
    fs::write(&f.gone, "never shown").unwrap();

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_changed(&refusal, &f.gone);
    assert_eq!(fs::read_to_string(&f.gone).unwrap(), "never shown");
    assert!(f.registered());
}

/// R2-F1.5: a link to another directory replaced by a file after the
/// question: the file and the directory are both kept.
#[test]
fn an_unrelated_link_replaced_by_a_file_after_the_question_is_kept() {
    let f = fixture();
    fs::remove_dir_all(&f.gone).unwrap();
    let unrelated = f.gone.with_file_name("unrelated");
    fs::create_dir(&unrelated).unwrap();
    fs::write(unrelated.join("keep.txt"), "never deleted").unwrap();
    link_dir(&unrelated, &f.gone);
    let facts = f.facts();
    assert!(matches!(facts.target, Target::Link { own: false, .. }), "{:?}", facts.target);
    let entry = f.entry();
    replace_link_with_file(&f.gone);

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_changed(&refusal, &f.gone);
    assert_eq!(fs::read_to_string(&f.gone).unwrap(), "a file that was never shown");
    assert!(unrelated.join("keep.txt").is_file());
    assert!(f.registered());
}

/// R2-F1.6: an own link replaced by a file after the question: neither the
/// file nor the real checkout the link led to is deleted.
#[test]
fn an_own_link_replaced_by_a_file_after_the_question_deletes_neither() {
    let f = fixture();
    let real = replace_with_link(&f.gone, "real");
    let facts = f.facts();
    assert!(matches!(facts.target, Target::Link { own: true, .. }), "{:?}", facts.target);
    let entry = f.entry();
    replace_link_with_file(&f.gone);

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_changed(&refusal, &f.gone);
    assert!(f.gone.is_file());
    assert!(real.join("target/debug/a.o").is_file(), "the real checkout is kept");
    assert!(f.registered());
}

/// Copies the directory tree at `from` to the new directory `to`.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

/// R2-F1.7: Git's record of the worktree rebuilt at the same path after the
/// question is not the record the question read. It is kept aside outside
/// `worktrees/`, so no second record names the checkout.
#[test]
fn a_worktree_record_rebuilt_after_the_question_is_refused() {
    let f = fixture();
    let facts = f.facts();
    let entry = f.entry();
    let admin = f.admin_dir();
    let aside = common_git_dir(&f.base()).unwrap().join("gone-record-aside");
    fs::rename(&admin, &aside).unwrap();
    copy_tree(&aside, &admin);

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert_changed(&refusal, &admin);
    assert!(f.gone.join("target/debug/a.o").is_file());
    assert!(admin.join("gitdir").is_file());
    assert!(f.registered());
    assert!(f.stored().is_empty());
}

/// Git's record rebuilt while the checkout is being deleted is not the one
/// the approval bound: it is kept, with the operation, for a later run.
#[test]
fn a_worktree_record_rebuilt_during_the_removal_is_kept() {
    let f = fixture();
    let facts = f.facts();
    let entry = f.entry();
    let admin = f.admin_dir();
    let aside = common_git_dir(&f.base()).unwrap().join("gone-record-aside");
    let (from, to) = (admin.clone(), aside.clone());
    let mut seams = Seams {
        hook: Box::new(move |boundary| {
            if boundary == Boundary::ContentRemoved {
                fs::rename(&from, &to).unwrap();
                copy_tree(&to, &from);
            }
            false
        }),
        moves: Faults::default(),
    };

    let removal = run_with(&request(&f, &entry, &facts, &[]), &Doubles::default().env(), &mut seams).unwrap();

    let finished = finished(removal);
    assert!(matches!(&finished.checkout, CheckoutResult::Stopped(sentence) if sentence.contains("no longer this worktree's")), "{finished:?}");
    assert!(admin.join("gitdir").is_file(), "the rebuilt record is kept");
    assert_eq!(f.stored().len(), 1, "the operation is kept");
}

/// R2-F1.9: no content was approved for a missing or vanished checkout, so
/// a directory that appears at its path after the question is kept with
/// Git's record.
#[test]
fn a_directory_appearing_where_the_checkout_was_missing_is_kept() {
    for vanished in [false, true] {
        let f = fixture();
        fs::remove_dir_all(&f.gone).unwrap();
        let mut facts = f.facts();
        assert_eq!(facts.target, Target::Missing);
        if vanished {
            facts.target = Target::Vanished;
        }
        let entry = f.entry();
        fs::create_dir(&f.gone).unwrap();
        fs::write(f.gone.join("new.txt"), "never shown").unwrap();

        let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

        assert!(matches!(&refusal, Refusal::Replaced(sentence) if sentence.contains("appeared after the question")), "{refusal:?}");
        assert!(f.gone.join("new.txt").is_file(), "vanished: {vanished}");
        assert!(f.registered(), "vanished: {vanished}");
    }
}

/// R14, R2-F4.5: a target without a checkout directory has nothing to
/// compare, so even with a question's list nothing is named and nothing
/// blocks.
#[test]
fn a_target_without_a_checkout_names_no_late_change() {
    type Setup = fn(&Fixture) -> Option<Target>;
    let cases: [(&str, Setup); 5] = [
        ("unrelated link", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            let unrelated = f.gone.with_file_name("unrelated");
            fs::create_dir(&unrelated).unwrap();
            fs::write(unrelated.join("new.txt"), "outside the approval").unwrap();
            link_dir(&unrelated, &f.gone);
            None
        }),
        ("file", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            fs::write(&f.gone, "a file").unwrap();
            None
        }),
        ("missing", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            None
        }),
        ("vanished", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            Some(Target::Vanished)
        }),
        ("uninspectable", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            fs::write(&f.gone, "a file").unwrap();
            Some(Target::Uninspectable)
        }),
    ];
    for (case, setup) in cases {
        let f = fixture();
        let injected = setup(&f);
        let mut facts = f.facts();
        if let Some(target) = injected {
            facts.target = target;
        }
        facts.inventory = Some(crate::remove::inventory::Inventory::default());
        let entry = f.entry();

        let finished = finished(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

        assert_eq!(finished.exit_class(), ExitClass::Success, "{case}: {finished:?}");
        assert!(finished.late_changes.is_empty(), "{case}: {:?}", finished.late_changes);
        f.assert_gone();
    }
}

/// A path the question could not identify is not removed at all.
#[test]
fn a_target_the_question_could_not_identify_is_not_removed() {
    let f = fixture();
    fs::remove_dir_all(&f.gone).unwrap();
    fs::write(&f.gone, "unreadable to the question").unwrap();
    let mut facts = f.facts();
    facts.target = Target::Uninspectable;
    facts.identities.entry = None;
    let entry = f.entry();

    let refusal = refused(remove(&request(&f, &entry, &facts, &[]), &Doubles::default().env()));

    assert!(matches!(&refusal, Refusal::Replaced(sentence) if sentence.contains("could not be identified before the question")), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    assert!(f.gone.is_file());
    assert!(f.registered());
    assert!(f.stored().is_empty());
}

// --- Every target is saved before it is changed -----------------------------

/// Makes the removal store unusable: a file where its directory goes.
fn break_store(f: &Fixture) {
    assert!(absent(&f.store()));
    fs::write(f.store(), "not a directory").unwrap();
}

/// R2-F2.1–F2.6: for every kind of target, an approval that cannot be saved
/// unlocks, deletes, and unregisters nothing, and runs no branch step.
#[test]
fn an_unsavable_approval_of_any_target_changes_nothing() {
    type Setup = fn(&Fixture) -> Option<Target>;
    let cases: [(&str, Setup); 6] = [
        ("own link", |f| {
            replace_with_link(&f.gone, "real");
            None
        }),
        ("unrelated link", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            let unrelated = f.gone.with_file_name("unrelated");
            fs::create_dir(&unrelated).unwrap();
            link_dir(&unrelated, &f.gone);
            None
        }),
        ("file", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            fs::write(&f.gone, "a file").unwrap();
            None
        }),
        ("missing", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            None
        }),
        ("vanished", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            Some(Target::Vanished)
        }),
        ("uninspectable", |f| {
            fs::remove_dir_all(&f.gone).unwrap();
            fs::write(&f.gone, "a file").unwrap();
            Some(Target::Uninspectable)
        }),
    ];
    for (case, setup) in cases {
        let f = fixture();
        f.repo.git(&["worktree", "lock", f.gone.to_str().unwrap()]);
        let injected = setup(&f);
        let mut facts = f.facts();
        if let Some(target) = injected {
            facts.target = target;
        }
        let before = rename::identity(&f.gone).ok();
        let real = f.gone.with_file_name("real");
        let entry = f.entry();
        let branches = [f.delete_branch(StepApproval::Explicit)];
        break_store(&f);

        let refusal = refused(remove(&request(&f, &entry, &facts, &branches), &Doubles::default().env()));

        assert!(matches!(refusal, Refusal::NotSaved(_)), "{case}: {refusal:?}");
        assert_eq!(refusal.exit_class(), ExitClass::Failure, "{case}");
        assert_eq!(rename::identity(&f.gone).ok(), before, "{case}: the entry at the path is kept");
        if case == "own link" {
            assert!(real.join("target/debug/a.o").is_file(), "{case}: the real checkout is kept");
        }
        assert!(f.registered(), "{case}: Git's record is kept");
        assert!(f.admin_dir().join("locked").exists(), "{case}: the lock is kept");
        assert!(f.branch_exists(), "{case}: the branch is kept");
        assert!(f.store().is_file(), "{case}: nothing was saved");
    }
}

/// R2-F2.1: Ctrl-C while an own link's checkout is deleted keeps the
/// operation; the name now finds it pending, and the next run finishes it
/// without asking.
#[test]
fn a_cancelled_own_link_removal_is_pending_and_finished_without_a_question() {
    let f = fixture();
    let real = replace_with_link(&f.gone, "real");
    let facts = f.facts();
    let entry = f.entry();
    let doubles = Doubles::default();
    let cancelled = || true;
    let env = Env { cancelled: &cancelled, ..doubles.env() };

    let first = finished(remove(&request(&f, &entry, &facts, &[]), &env));

    assert!(matches!(first.checkout, CheckoutResult::Cancelled { .. }), "{first:?}");
    assert_eq!(first.exit_class(), ExitClass::Failure);
    let record = f.record();
    assert_eq!(
        record.target,
        ApprovedTarget::OwnLink { link: identity_at(&f.gone), checkout: facts.identities.resolved.unwrap() }
    );
    assert!(same_location(record.deletion_path.as_path(), &biscuit_file::canonicalize_simplified(&real).unwrap()));
    let id = f.pending_id();

    let second = finished(finish_pending(&f.base(), &id, &doubles.env()));

    assert_eq!(second.exit_class(), ExitClass::Success, "{second:?}");
    assert!(absent(&real), "the real checkout is deleted");
    f.assert_gone();
}

/// A crash at every step of removing a missing checkout's record, its
/// approved unlock and branch step included, is finished by the next run
/// without a question.
#[test]
fn a_crash_at_every_step_of_a_missing_checkout_is_finished_by_the_next_run() {
    for at in [Boundary::Begun, Boundary::Unlocked, Boundary::Unregistered, Boundary::BranchAction(0)] {
        let f = fixture();
        f.repo.git(&["worktree", "lock", f.gone.to_str().unwrap()]);
        fs::remove_dir_all(&f.gone).unwrap();
        let facts = f.facts();
        let branches = [f.delete_branch(StepApproval::Explicit)];
        let doubles = Doubles::default();
        crash_at(&f, &facts, &branches, &doubles, at);
        assert_eq!(f.record().target, ApprovedTarget::Absent, "{at:?}");

        let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

        assert!(matches!(finished.checkout, CheckoutResult::AlreadyRemoved), "{at:?}: {finished:?}");
        assert_eq!(finished.exit_class(), ExitClass::Success, "{at:?}");
        assert!(!f.branch_exists(), "{at:?}");
        f.assert_gone();
    }
}

/// A crash at every step of removing an own link and its checkout is
/// finished by the next run without a question.
#[test]
fn a_crash_at_every_step_of_an_own_link_is_finished_by_the_next_run() {
    for at in [Boundary::Begun, Boundary::Unlocked, Boundary::ContentRemoved, Boundary::Unregistered] {
        let f = fixture();
        let real = replace_with_link(&f.gone, "real");
        let facts = f.facts();
        let doubles = Doubles::default();
        crash_at(&f, &facts, &[], &doubles, at);

        let finished = finished(finish_pending(&f.base(), &f.pending_id(), &doubles.env()));

        assert_eq!(finished.exit_class(), ExitClass::Success, "{at:?}: {finished:?}");
        assert!(absent(&real), "{at:?}");
        f.assert_gone();
    }
}

/// A file saved for removal and swapped for another before the next run
/// is kept: the operation closes and Git's record stays.
#[test]
fn a_file_replaced_before_a_saved_removal_resumes_is_kept() {
    let f = fixture();
    fs::remove_dir_all(&f.gone).unwrap();
    fs::write(&f.gone, "shown").unwrap();
    let facts = f.facts();
    let doubles = Doubles::default();
    crash_at(&f, &facts, &[], &doubles, Boundary::Unlocked);
    let id = f.pending_id();
    fs::remove_file(&f.gone).unwrap();
    fs::write(&f.gone, "never shown").unwrap();

    let refusal = refused(finish_pending(&f.base(), &id, &doubles.env()));

    assert_changed(&refusal, &f.gone);
    assert_eq!(fs::read_to_string(&f.gone).unwrap(), "never shown");
    assert!(f.registered());
    assert!(f.stored().is_empty(), "the operation is closed");
}

// --- An operation an earlier wt saved -------------------------------------

/// R19: a legacy (version 1) operation with a version 6 report is finished
/// by the next run without a question. Its own branch is rebuilt as a
/// conditional step and deleted once rechecked; its remote step binds no
/// endpoint or head, so it is never run and the run exits 1.
#[test]
fn a_legacy_operation_is_finished_and_its_unbound_remote_step_left_undone() {
    use crate::remove::recovery::record::{BranchBinding, IntendedAction, Phase, RECORD_VERSION, RecoveryRecord};
    use crate::remove::recovery::store::REPORT_FILE;

    let f = fixture();
    let facts = f.facts();
    crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Moved);
    let compact = f.record();
    let legacy = RecoveryRecord {
        record_version: RECORD_VERSION,
        operation_id: compact.operation_id.clone(),
        created_at_ms: compact.created_at_ms,
        requested_name: compact.requested_name.clone(),
        aliases: compact.aliases.clone(),
        original: compact.original.clone(),
        quarantine: compact.deletion_path.clone(),
        checkout_identity: compact.target.checkout_identity().unwrap(),
        repository: compact.repository.clone(),
        branch: BranchBinding::Named(BRANCH.into()),
        approved_tip: f.tip(),
        consent_fingerprint: "b".repeat(64),
        discovery: crate::remove::test_support::clear_report(&f.gone),
        intended_actions: vec![
            IntendedAction::RemoveCheckout,
            IntendedAction::DeleteLocalBranch { name: BRANCH.into() },
            IntendedAction::DeleteRemoteBranch { remote: "origin".into(), name: BRANCH.into() },
        ],
        completed_phase: Phase::Associated,
    };
    let dir = f.store().join(compact.operation_id.as_str());
    // Overwriting keeps the record's owner-only mode.
    fs::write(dir.join(RECORD_FILE), legacy.to_json()).unwrap();
    let report = serde_json::json!({"report_version": 6, "operation_id": compact.operation_id, "outcome": "activity_detected"});
    fs::write(dir.join(REPORT_FILE), serde_json::to_vec(&report).unwrap()).unwrap();
    fs::write(dir.join("journal.jsonl"), b"{\"torn").unwrap();

    let finished = finished(finish_pending(&f.base(), &f.pending_id(), &Doubles::default().env()));

    assert_eq!(finished.branches.len(), 2, "{finished:?}");
    assert_eq!(finished.branches[0].outcome, BranchOutcome::Deleted);
    assert_eq!(finished.branches[1].outcome, BranchOutcome::NotCarriedOver);
    assert_eq!(finished.exit_class(), ExitClass::Failure);
    assert!(!f.branch_exists());
    f.assert_gone();
}

/// A pending removal whose record is corrupt, of an unknown version, or
/// another operation's is explained, never resumed, and never followed by a
/// speculative deletion: the placed checkout and the record stay as they are.
#[test]
fn an_unreadable_record_of_a_pending_removal_is_explained_and_nothing_is_deleted() {
    type Edit = fn(serde_json::Value) -> Vec<u8>;
    let cells: [(&str, Edit, &str); 3] = [
        ("corrupt", |_| b"{\"record_version\": 3,".to_vec(), "malformed"),
        ("unknown version", |mut doc| { doc["record_version"] = serde_json::json!(4); serde_json::to_vec(&doc).unwrap() }, "version 4"),
        ("another operation's", |mut doc| { doc["operation_id"] = serde_json::json!("f".repeat(32)); serde_json::to_vec(&doc).unwrap() }, "operation id"),
    ];
    for (cell, edit, explained) in cells {
        let f = fixture();
        let facts = f.facts();
        crash_at(&f, &facts, &[], &Doubles::default(), Boundary::Moved);
        let id = f.pending_id();
        let deletion = f.record().deletion_path.as_path().to_path_buf();
        let path = f.store().join(id.as_str()).join(RECORD_FILE);
        let bytes = edit(serde_json::from_slice(&fs::read(&path).unwrap()).unwrap());
        fs::write(&path, &bytes).unwrap();

        let Err(ResolveError::Store(error)) = resolve(&f.base(), "gone") else { panic!("{cell}: the saved name is explained") };
        assert!(error.to_string().contains(id.as_str()) && error.to_string().contains(explained), "{cell}: {error}");
        assert!(matches!(resolve(&f.base(), "unrelated"), Err(ResolveError::Store(_))), "{cell}: no name can tell it is not this operation's");
        let refusal = refused(finish_pending(&f.base(), &id, &Doubles::default().env()));
        assert!(refusal.to_string().contains(id.as_str()) && refusal.to_string().contains(explained), "{cell}: {refusal}");
        assert_eq!(refusal.exit_class(), ExitClass::Refused, "{cell}");
        assert!(deletion.join("notes.txt").is_file(), "{cell}: nothing is deleted");
        assert_eq!(fs::read(&path).unwrap(), bytes, "{cell}: the record is kept as it was");
    }
}
