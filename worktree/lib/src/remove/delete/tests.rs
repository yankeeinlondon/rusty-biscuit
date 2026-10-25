//! L1, rung F2: real directory trees in a temp dir; the clock, faults,
//! mount table, and holder lookup are injected, so nothing sleeps, mounts,
//! or needs privileges.

use std::cell::Cell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sniff::filesystem::query::FileIdentity;

use super::*;
use crate::remove::faults::NoFaults;
use crate::remove::quarantine::rename;
use crate::remove::test_support::{FixedMounts, LiveMounts, ManualClock, MountDuringDeletion, ScriptedFaults, ScriptedHolders};

/// What [`Progress`] was told.
#[derive(Default)]
struct Told {
    batches: Vec<u64>,
    waits: Vec<Wait>,
}

impl Progress for Told {
    fn batch(&mut self, deleted: u64) {
        self.batches.push(deleted);
    }

    fn waiting(&mut self, wait: &Wait) {
        self.waits.push(wait.clone());
    }
}

/// A checkout-like tree under a temp dir, beside an `outside` directory that
/// deletion must never touch.
struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    fn new() -> Self {
        let tree = Self { dir: tempfile::tempdir().unwrap() };
        let root = tree.root();
        fs::create_dir_all(root.join("src").join("deep")).unwrap();
        fs::create_dir_all(root.join("target").join("debug")).unwrap();
        fs::write(root.join(".git"), "gitdir: /repo/.git/worktrees/feat\n").unwrap();
        fs::write(root.join("README.md"), "readme").unwrap();
        fs::write(root.join("src").join("lib.rs"), "fn main() {}").unwrap();
        fs::write(root.join("src").join("deep").join("mod.rs"), "").unwrap();
        fs::write(root.join("target").join("debug").join("app"), "binary").unwrap();
        fs::create_dir_all(tree.outside()).unwrap();
        fs::write(tree.outside().join("keep.txt"), "outside").unwrap();
        tree
    }

    fn root(&self) -> PathBuf {
        self.dir.path().join("feat")
    }

    fn outside(&self) -> PathBuf {
        self.dir.path().join("outside")
    }

    fn identity(&self) -> FileIdentity {
        rename::identity(&self.root()).unwrap()
    }

    fn assert_outside_untouched(&self) {
        assert_eq!(fs::read_to_string(self.outside().join("keep.txt")).unwrap(), "outside");
    }
}

/// Runs [`delete_tree`] on `tree`'s root with the given seams.
fn delete(tree: &Tree, clock: &ManualClock, faults: &dyn FsFaults, mounts: &[Mount], holders: &dyn Holders, told: &mut Told) -> Deletion {
    delete_with(tree, clock, faults, &FixedMounts(mounts.to_vec()), holders, told)
}

fn delete_with(tree: &Tree, clock: &ManualClock, faults: &dyn FsFaults, mounts: &dyn MountInfo, holders: &dyn Holders, told: &mut Told) -> Deletion {
    let identity = tree.identity();
    let env = Env { clock, faults, mounts, holders, cancelled: &|| false };
    delete_tree(Target { root: &tree.root(), identity: &identity }, &env, told)
}

fn plain(tree: &Tree) -> (Deletion, ManualClock) {
    let clock = ManualClock::new();
    let deletion = delete(tree, &clock, &NoFaults, &[], &ScriptedHolders::new(), &mut Told::default());
    (deletion, clock)
}

fn absent(path: &Path) -> bool {
    matches!(fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

/// The error a held file gives: a sharing violation on Windows; on Unix,
/// which has none, the error the OS asks to be repeated.
fn temporary_error() -> io::Error {
    #[cfg(windows)]
    return io::Error::from_raw_os_error(windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION as i32);
    #[cfg(unix)]
    return io::Error::from_raw_os_error(libc::EAGAIN);
}

fn read_only_error() -> io::Error {
    #[cfg(windows)]
    return io::Error::from_raw_os_error(windows_sys::Win32::Foundation::ERROR_WRITE_PROTECT as i32);
    #[cfg(unix)]
    return io::Error::from_raw_os_error(libc::EROFS);
}

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn a_whole_checkout_is_deleted_with_its_root_and_nothing_outside_it() {
    let tree = Tree::new();

    let (deletion, clock) = plain(&tree);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert!(absent(&tree.root()));
    assert!(clock.slept().is_empty());
    tree.assert_outside_untouched();
}

/// An interrupted removal reruns the engine on whatever is left, including
/// nothing.
#[test]
fn a_root_already_gone_is_done_and_a_partial_one_is_finished() {
    let tree = Tree::new();
    fs::remove_dir_all(tree.root().join("src")).unwrap();
    assert!(matches!(plain(&tree).0, Deletion::Done));

    assert!(absent(&tree.root()));
    let clock = ManualClock::new();
    let gone = FileIdentity::Unix { device: 0, inode: 0 };
    let env = Env { clock: &clock, faults: &NoFaults, mounts: &FixedMounts::default(), holders: &ScriptedHolders::new(), cancelled: &|| false };
    assert!(matches!(delete_tree(Target { root: &tree.root(), identity: &gone }, &env, &mut Silent), Deletion::Done));
}

/// During deletion in place a replaced root is outside the approval.
#[test]
fn a_root_that_is_another_directory_or_a_link_is_never_deleted() {
    let tree = Tree::new();
    let stranger = rename::identity(&tree.outside()).unwrap();
    let clock = ManualClock::new();
    let env = Env { clock: &clock, faults: &NoFaults, mounts: &FixedMounts::default(), holders: &ScriptedHolders::new(), cancelled: &|| false };

    let deletion = delete_tree(Target { root: &tree.root(), identity: &stranger }, &env, &mut Silent);
    assert!(matches!(deletion, Deletion::RootReplaced), "{deletion:?}");
    assert!(tree.root().join("src").join("lib.rs").is_file(), "nothing deleted");

    let identity = tree.identity();
    let moved = tree.dir.path().join("moved");
    fs::rename(tree.root(), &moved).unwrap();
    crate::remove::test_support::link_dir(&moved, &tree.root());
    let deletion = delete_tree(Target { root: &tree.root(), identity: &identity }, &env, &mut Silent);
    assert!(matches!(deletion, Deletion::RootReplaced), "{deletion:?}");
    assert!(moved.join("src").join("lib.rs").is_file(), "nothing deleted through the link");
}

#[test]
fn links_inside_the_checkout_are_removed_and_their_targets_never_touched() {
    let tree = Tree::new();
    crate::remove::test_support::link_dir(&tree.outside(), &tree.root().join("linked-dir"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(tree.outside().join("keep.txt"), tree.root().join("linked-file")).unwrap();

    let (deletion, _) = plain(&tree);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert!(absent(&tree.root()));
    tree.assert_outside_untouched();
}

/// [`FsFaults`] that, the first time `dir` is about to be opened, moves it
/// away and puts a link to `outside` in its place: the race a handle-based
/// traversal must win.
struct SwapForLink {
    dir: PathBuf,
    outside: PathBuf,
    swapped: Mutex<bool>,
}

impl FsFaults for SwapForLink {
    fn inject(&self, op: FsOp, path: &Path) -> Option<io::Error> {
        let mut swapped = self.swapped.lock().unwrap();
        if op == FsOp::ReadDir && path == self.dir && !*swapped {
            *swapped = true;
            fs::rename(&self.dir, self.dir.with_file_name("swapped-away")).unwrap();
            crate::remove::test_support::link_dir(&self.outside, &self.dir);
        }
        None
    }
}

#[test]
fn a_directory_swapped_for_a_link_during_traversal_exposes_nothing_behind_the_link() {
    let tree = Tree::new();
    let faults = SwapForLink { dir: tree.root().join("src"), outside: tree.outside(), swapped: Mutex::new(false) };

    let deletion = delete(&tree, &ManualClock::new(), &faults, &[], &ScriptedHolders::new(), &mut Told::default());

    assert!(*faults.swapped.lock().unwrap());
    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert!(absent(&tree.root()));
    tree.assert_outside_untouched();
}

/// R5: unlinked without being opened (opening a FIFO with no writer would
/// block this test forever).
#[cfg(unix)]
#[test]
fn fifos_and_sockets_are_unlinked_without_being_opened() {
    let tree = Tree::new();
    let fifo = std::ffi::CString::new(tree.root().join("pipe").into_os_string().into_encoded_bytes()).unwrap();
    // SAFETY: a NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let _listener = std::os::unix::net::UnixListener::bind(tree.root().join("s.sock")).unwrap();

    let (deletion, _) = plain(&tree);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert!(absent(&tree.root()));
}

/// R13: read-only and unlistable directories are made deletable through
/// their own handles; a hard link inside loses only its name, and the
/// outside name keeps its content and permissions.
#[cfg(unix)]
#[test]
fn read_only_directories_are_made_deletable_without_changing_a_hard_links_shared_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let tree = Tree::new();
    let shared = tree.outside().join("shared.bin");
    fs::write(&shared, "shared").unwrap();
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o444)).unwrap();
    let locked = tree.root().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::hard_link(&shared, locked.join("linked.bin")).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let sealed = tree.root().join("sealed");
    fs::create_dir(&sealed).unwrap();
    fs::write(sealed.join("inside"), "x").unwrap();
    fs::set_permissions(&sealed, fs::Permissions::from_mode(0o000)).unwrap();

    let (deletion, _) = plain(&tree);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert!(absent(&tree.root()));
    assert_eq!(fs::read_to_string(&shared).unwrap(), "shared");
    assert_eq!(fs::metadata(&shared).unwrap().permissions().mode() & 0o777, 0o444);
    tree.assert_outside_untouched();
}

/// Windows: the read-only attribute is ignored, never cleared, so another
/// hard link's attributes do not change.
#[cfg(windows)]
#[test]
fn read_only_files_are_deleted_without_changing_a_hard_links_shared_attributes() {
    let tree = Tree::new();
    let shared = tree.outside().join("shared.bin");
    fs::write(&shared, "shared").unwrap();
    fs::hard_link(&shared, tree.root().join("linked.bin")).unwrap();
    let mut read_only = fs::metadata(&shared).unwrap().permissions();
    read_only.set_readonly(true);
    fs::set_permissions(&shared, read_only).unwrap();
    let alone = tree.root().join("alone.txt");
    fs::write(&alone, "x").unwrap();
    let mut alone_read_only = fs::metadata(&alone).unwrap().permissions();
    alone_read_only.set_readonly(true);
    fs::set_permissions(&alone, alone_read_only).unwrap();

    let (deletion, _) = plain(&tree);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert!(absent(&tree.root()));
    assert_eq!(fs::read_to_string(&shared).unwrap(), "shared");
    assert!(fs::metadata(&shared).unwrap().permissions().readonly());
}

/// R29: a permanent refusal is not waited on or retried; everything else is
/// deleted, and the failure reads as one plain sentence.
#[test]
fn a_permanent_failure_deletes_everything_else_and_is_never_retried() {
    let tree = Tree::new();
    let stuck = tree.root().join("target").join("debug").join("app");
    let faults = ScriptedFaults::new().always(FsOp::RemoveFile, &stuck, read_only_error);
    let clock = ManualClock::new();

    let deletion = delete(&tree, &clock, &faults, &[], &ScriptedHolders::new(), &mut Told::default());

    let Deletion::Permanent { failures, retained_mounts } = deletion else { panic!("{deletion:?}") };
    assert!(retained_mounts.is_empty());
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].path, stuck);
    assert_eq!(failures[0].to_string(), format!("{} could not be deleted: the disk is read-only", stuck.display()));
    assert_eq!(faults.fired(FsOp::RemoveFile, &stuck), 1, "never retried");
    assert!(clock.slept().is_empty(), "never waited");
    assert!(stuck.is_file());
    let left: Vec<PathBuf> = walk(&tree.root());
    assert_eq!(left, [tree.root().join("target"), tree.root().join("target").join("debug"), stuck], "only the failed entry and its parents remain");
}

/// A denial that making the directory writable does not fix is permanent;
/// on Windows, where no program holds the entry, too.
#[test]
fn a_denial_nobody_explains_is_permanent_after_one_fix() {
    let tree = Tree::new();
    let denied = tree.root().join("README.md");
    let faults = ScriptedFaults::new().always(FsOp::RemoveFile, &denied, || io::Error::from(io::ErrorKind::PermissionDenied));
    let clock = ManualClock::new();
    let holders = ScriptedHolders::answering(vec![HolderReport::Found(Vec::new())]);

    let deletion = delete(&tree, &clock, &faults, &[], &holders, &mut Told::default());

    let Deletion::Permanent { failures, .. } = deletion else { panic!("{deletion:?}") };
    assert_eq!(failures.iter().map(|failure| failure.path.clone()).collect::<Vec<_>>(), std::slice::from_ref(&denied));
    assert!(failures[0].to_string().ends_with("could not be deleted: permission was denied"), "{}", failures[0]);
    assert!(clock.slept().is_empty());
    let tries = faults.fired(FsOp::RemoveFile, &denied);
    if cfg!(unix) {
        assert_eq!(tries, 2, "once, then once after making the directory writable");
    } else {
        assert_eq!(tries, 1);
        assert_eq!(holders.asked(), [vec![denied]]);
    }
}

/// Windows: a running program's image is refused as access denied; it is
/// waited on only when a program is seen holding it.
#[cfg(windows)]
#[test]
fn a_windows_denial_with_a_named_holder_is_waited_on() {
    let tree = Tree::new();
    let image = tree.root().join("target").join("debug").join("app");
    let faults = ScriptedFaults::new().times(FsOp::RemoveFile, &image, 1, || {
        io::Error::from_raw_os_error(windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED as i32)
    });
    let holder = Holder { pid: 7, name: Some("app.exe".into()), writing: false, paths: vec![image.clone()] };
    let holders = ScriptedHolders::answering(vec![HolderReport::Found(vec![holder])]);
    let clock = ManualClock::new();
    let mut told = Told::default();

    let deletion = delete(&tree, &clock, &faults, &[], &holders, &mut told);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert_eq!(clock.slept(), [ms(50)]);
    assert_eq!(holders.asked(), [vec![image.clone()]], "the denial's query also explains the wait");
    assert_eq!(told.waits.len(), 1);
    assert!(told.waits[0].to_string().contains("app.exe (pid 7) has a file open"), "{}", told.waits[0]);
}

/// [`Holders`] that records what was still in the tree when it was asked.
struct Snapshotting<'a> {
    root: PathBuf,
    seen: &'a Mutex<Vec<Vec<PathBuf>>>,
    report: HolderReport,
}

impl Holders for Snapshotting<'_> {
    fn query(&self, _root: &Path, _blocked: &[PathBuf]) -> HolderReport {
        self.seen.lock().unwrap().push(walk(&self.root));
        self.report.clone()
    }
}

/// R22: everything else is deleted first, the holder is named once, and the
/// held entry is retried with the backoff until it goes.
#[test]
fn a_held_file_is_waited_on_after_everything_else_is_deleted() {
    let tree = Tree::new();
    let held = tree.root().join("target").join("debug").join("app");
    let faults = ScriptedFaults::new().times(FsOp::RemoveFile, &held, 3, temporary_error);
    let seen = Mutex::new(Vec::new());
    let holder = Holder { pid: 4123, name: Some("cargo".into()), writing: false, paths: vec![held.clone()] };
    let holders = Snapshotting { root: tree.root(), seen: &seen, report: HolderReport::Found(vec![holder]) };
    let clock = ManualClock::new();
    let mut told = Told::default();

    let deletion = delete(&tree, &clock, &faults, &[], &holders, &mut told);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert!(absent(&tree.root()));
    assert_eq!(clock.slept(), [ms(50), ms(100), ms(200)]);
    let seen = seen.into_inner().unwrap();
    assert_eq!(seen.len(), 1, "asked once, to explain the wait");
    assert_eq!(seen[0], [tree.root().join("target"), tree.root().join("target").join("debug"), held.clone()], "the rest went first");
    assert_eq!(told.waits.len(), 1, "reported once while nothing changed");
    assert_eq!(told.waits[0].cause, WaitCause::InUse);
    assert_eq!(told.waits[0].paths, [held]);
    assert!(told.waits[0].to_string().contains("cargo (pid 4123) has a file open"));
}

/// A wait is explained again, and reported again, only when what it waits
/// on changes; progress restarts the backoff.
#[test]
fn a_wait_is_reported_again_only_when_its_blocked_entries_change() {
    let tree = Tree::new();
    let quick = tree.root().join("README.md");
    let slow = tree.root().join("src").join("lib.rs");
    let faults = ScriptedFaults::new()
        .times(FsOp::RemoveFile, &quick, 1, temporary_error)
        .times(FsOp::RemoveFile, &slow, 3, temporary_error);
    let holders = ScriptedHolders::new();
    let clock = ManualClock::new();
    let mut told = Told::default();

    let deletion = delete(&tree, &clock, &faults, &[], &holders, &mut told);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    let mut both = vec![quick, slow.clone()];
    both.sort();
    assert_eq!(holders.asked(), [both.clone(), vec![slow.clone()]]);
    assert_eq!(told.waits.iter().map(|wait| wait.paths.clone()).collect::<Vec<_>>(), [both, vec![slow]]);
    assert!(told.waits.iter().all(|wait| wait.holders == HolderReport::Unknown));
    assert_eq!(clock.slept(), [ms(50), ms(50), ms(100)]);
}

#[test]
fn cancelling_a_wait_ends_deletion_and_names_what_it_waited_on() {
    let tree = Tree::new();
    let held = tree.root().join("README.md");
    let faults = ScriptedFaults::new().always(FsOp::RemoveFile, &held, temporary_error);
    let clock = ManualClock::cancelled_after(2);

    let deletion = delete(&tree, &clock, &faults, &[], &ScriptedHolders::new(), &mut Told::default());

    let Deletion::Cancelled { waiting_on } = deletion else { panic!("{deletion:?}") };
    assert_eq!(waiting_on, std::slice::from_ref(&held));
    assert_eq!(clock.slept(), [ms(50), ms(100)]);
    assert_eq!(walk(&tree.root()), [held], "the rest was deleted before the wait");
}

#[test]
fn cancelling_during_traversal_stops_at_the_next_entry() {
    let tree = Tree::new();
    let polls = Cell::new(0);
    let cancelled = || {
        polls.set(polls.get() + 1);
        polls.get() > 2
    };
    let identity = tree.identity();
    let clock = ManualClock::new();
    let env = Env { clock: &clock, faults: &NoFaults, mounts: &FixedMounts::default(), holders: &ScriptedHolders::new(), cancelled: &cancelled };

    let deletion = delete_tree(Target { root: &tree.root(), identity: &identity }, &env, &mut Silent);

    assert!(matches!(&deletion, Deletion::Cancelled { waiting_on } if waiting_on.is_empty()), "{deletion:?}");
    assert!(tree.root().join(".git").is_file(), "stopped before the end");
    assert!(clock.slept().is_empty());
}

/// [`FsFaults`] standing in for a program that writes into `dir` again each
/// time it is about to be removed, `times` times.
struct Refill {
    dir: PathBuf,
    times: Option<usize>,
    written: Mutex<usize>,
}

impl FsFaults for Refill {
    fn inject(&self, op: FsOp, path: &Path) -> Option<io::Error> {
        let mut written = self.written.lock().unwrap();
        if op == FsOp::RemoveDir && path == self.dir && self.times.is_none_or(|times| *written < times) {
            *written += 1;
            fs::write(self.dir.join(format!("log-{written}")), "x").unwrap();
        }
        None
    }
}

/// R24: what reappears is listed and deleted again; within the threshold
/// there is no wait.
#[test]
fn a_directory_that_refills_a_few_times_is_relisted_and_deleted_without_waiting() {
    let tree = Tree::new();
    let out = tree.root().join("target");
    let faults = Refill { dir: out, times: Some(REFILL_PASSES as usize - 1), written: Mutex::new(0) };
    let holders = ScriptedHolders::new();
    let clock = ManualClock::new();

    let deletion = delete(&tree, &clock, &faults, &[], &holders, &mut Told::default());

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    assert!(absent(&tree.root()));
    assert!(clock.slept().is_empty());
    assert!(holders.asked().is_empty());
}

/// R24: a directory that keeps refilling is a wait, explained by the usage
/// query like a held file.
#[test]
fn a_directory_that_keeps_refilling_is_waited_on_and_its_writer_named() {
    let tree = Tree::new();
    let out = tree.root().join("target");
    let faults = Refill { dir: out.clone(), times: None, written: Mutex::new(0) };
    let writer = Holder { pid: 99, name: Some("tail".into()), writing: true, paths: vec![out.join("log-1")] };
    let holders = ScriptedHolders::answering(vec![HolderReport::Found(vec![writer])]);
    let clock = ManualClock::cancelled_after(1);
    let mut told = Told::default();

    let deletion = delete(&tree, &clock, &faults, &[], &holders, &mut told);

    assert!(matches!(&deletion, Deletion::Cancelled { waiting_on } if *waiting_on == [out.clone()]), "{deletion:?}");
    assert_eq!(*faults.written.lock().unwrap(), REFILL_PASSES as usize * 2, "three removals per pass, two passes");
    assert_eq!(told.waits.len(), 1);
    assert_eq!(told.waits[0].cause, WaitCause::Refilling);
    assert!(told.waits[0].to_string().contains("keeps filling up again: tail (pid 99) is writing to it"), "{}", told.waits[0]);
}

/// R7: a mount found in the table (a same-device bind mount included) is
/// kept, with the directories above it; everything else goes.
#[test]
fn a_nested_mount_is_never_crossed_and_everything_else_is_deleted() {
    let tree = Tree::new();
    let point = tree.root().join("target").join("cache");
    fs::create_dir(&point).unwrap();
    fs::write(point.join("on-the-mount"), "mounted data").unwrap();
    let mount = Mount { mount_point: point.clone(), source: "/dev/sdb1".into(), fs_type: String::new(), super_options: String::new() };
    let mounts = [Mount { mount_point: tree.dir.path().to_path_buf(), source: "/dev/sda1".into(), fs_type: String::new(), super_options: String::new() }, mount.clone()];

    let deletion = delete(&tree, &ManualClock::new(), &NoFaults, &mounts, &ScriptedHolders::new(), &mut Told::default());

    let Deletion::RetainedMounts(retained) = deletion else { panic!("{deletion:?}") };
    assert_eq!(retained, [mount]);
    assert_eq!(fs::read_to_string(point.join("on-the-mount")).unwrap(), "mounted data");
    assert_eq!(walk(&tree.root()), [tree.root().join("target"), point.clone(), point.join("on-the-mount")]);
}

/// A mount made after deletion began, before it reaches the mount point, is
/// found by asking the mount information again before entering the
/// directory: a same-device mount has no other sign.
#[test]
fn a_mount_made_during_deletion_is_never_crossed() {
    let tree = Tree::new();
    let live = LiveMounts::default();
    let publish = MountDuringDeletion::new(&live, Path::new("target").join("debug"));

    let deletion = delete_with(&tree, &ManualClock::new(), &publish, &live, &ScriptedHolders::new(), &mut Told::default());

    let point = tree.root().join("target").join("debug");
    let Deletion::RetainedMounts(retained) = deletion else { panic!("{deletion:?}") };
    assert_eq!(retained.iter().map(|mount| (mount.mount_point.clone(), mount.source.as_str())).collect::<Vec<_>>(), [(point.clone(), "/srv/bound")]);
    assert_eq!(walk(&tree.root()), [tree.root().join("target"), point.clone(), point.join("app")]);
}

/// [`FsFaults`] that, at the first removal of `dir`, refills it and mounts
/// on it, so only the check before reopening it can see the mount.
struct MountOnRefill {
    dir: PathBuf,
    mounts: LiveMounts,
    fired: AtomicBool,
}

impl FsFaults for MountOnRefill {
    fn inject(&self, op: FsOp, path: &Path) -> Option<io::Error> {
        if op == FsOp::RemoveDir && path == self.dir && !self.fired.swap(true, Ordering::SeqCst) {
            fs::write(self.dir.join("refilled"), "written during deletion").unwrap();
            self.mounts.publish(Mount { mount_point: self.dir.clone(), source: "/srv/bound".into(), fs_type: String::new(), super_options: String::new() });
        }
        None
    }
}

/// Each reopening of a refilled directory enters it afresh, so it is checked
/// for a mount again first.
#[test]
fn a_directory_mounted_while_it_refills_is_kept() {
    let tree = Tree::new();
    let point = tree.root().join("target").join("debug");
    let live = LiveMounts::default();
    let faults = MountOnRefill { dir: point.clone(), mounts: live.clone(), fired: AtomicBool::new(false) };

    let deletion = delete_with(&tree, &ManualClock::new(), &faults, &live, &ScriptedHolders::new(), &mut Told::default());

    let Deletion::RetainedMounts(retained) = deletion else { panic!("{deletion:?}") };
    assert_eq!(retained.iter().map(|mount| &mount.mount_point).collect::<Vec<_>>(), [&point]);
    assert_eq!(walk(&tree.root()), [tree.root().join("target"), point.clone(), point.join("refilled")]);
}

#[test]
fn a_permanent_failure_and_a_retained_mount_are_both_reported() {
    let tree = Tree::new();
    let point = tree.root().join("src").join("deep");
    let stuck = tree.root().join("README.md");
    let faults = ScriptedFaults::new().always(FsOp::RemoveFile, &stuck, read_only_error);
    let mounts = [Mount { mount_point: point.clone(), source: "tmpfs".into(), fs_type: String::new(), super_options: String::new() }];

    let deletion = delete(&tree, &ManualClock::new(), &faults, &mounts, &ScriptedHolders::new(), &mut Told::default());

    let Deletion::Permanent { failures, retained_mounts } = deletion else { panic!("{deletion:?}") };
    assert_eq!(failures.iter().map(|failure| failure.path.clone()).collect::<Vec<_>>(), [stuck]);
    assert_eq!(retained_mounts.iter().map(|mount| mount.mount_point.clone()).collect::<Vec<_>>(), [point]);
}

/// [`FsFaults`] that records every removal, in order.
#[derive(Default)]
struct Order(Mutex<Vec<PathBuf>>);

impl FsFaults for Order {
    fn inject(&self, op: FsOp, path: &Path) -> Option<io::Error> {
        if matches!(op, FsOp::RemoveFile | FsOp::RemoveDir) {
            self.0.lock().unwrap().push(path.to_path_buf());
        }
        None
    }
}

/// An interrupted deletion keeps a checkout Git recognizes for as long as
/// possible; batches are reported as they complete.
#[test]
fn the_roots_git_file_goes_last_and_batches_are_reported() {
    let tree = Tree::new();
    let many = tree.root().join("many");
    fs::create_dir(&many).unwrap();
    for index in 0..BATCH_ENTRIES {
        fs::write(many.join(index.to_string()), "").unwrap();
    }
    let order = Order::default();
    let mut told = Told::default();

    let deletion = delete(&tree, &ManualClock::new(), &order, &[], &ScriptedHolders::new(), &mut told);

    assert!(matches!(deletion, Deletion::Done), "{deletion:?}");
    let removed = order.0.into_inner().unwrap();
    assert_eq!(removed[removed.len() - 2..], [tree.root().join(".git"), tree.root()]);
    assert_eq!(told.batches, [BATCH_ENTRIES]);
}

/// Every path under `root`, sorted, `root` excluded.
fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries {
            let path = entry.unwrap().path();
            if fs::symlink_metadata(&path).unwrap().is_dir() {
                pending.push(path.clone());
            }
            found.push(path);
        }
    }
    found.sort();
    found
}
