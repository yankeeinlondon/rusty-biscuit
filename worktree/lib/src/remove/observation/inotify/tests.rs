//! Linux-only facts of the inotify adapter that the shared contract tests
//! cannot provoke on demand: the kernel's own signals fed through
//! `translate`, the instance limit, the kernel queue overflow, and what Sniff
//! sees of the session's watches.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::*;
use crate::remove::observation::{Drain, ObserverOptions, Observation, Scope};
use crate::remove::usage::{OwnedHandles, SniffUsage, UsageRefusal, check_usage};

struct Fixture {
    _root: tempfile::TempDir,
    checkout: PathBuf,
    targets: Targets,
}

fn new_fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("parent").join("checkout");
    fs::create_dir_all(checkout.join("sub")).unwrap();
    let admin = root.path().join("admin");
    fs::create_dir(&admin).unwrap();
    let quarantine = checkout.with_file_name(".wt-remove-checkout-0123456789abcdef");
    let targets = Targets::new(&checkout, &quarantine, &admin).unwrap();
    Fixture { _root: root, checkout, targets }
}

/// Feeds one synthetic event for `root`'s watch through `translate`.
fn feed(adapter: &mut Inotify, targets: &Targets, root: Root, path: &str, mask: EventMask, name: Option<&str>) -> Drain {
    let id = adapter
        .watches
        .iter()
        .find(|(_, watch)| watch.root == root && watch.path == Path::new(path))
        .map(|(id, _)| *id)
        .expect("a watch for the root");
    feed_id(adapter, targets, id, mask, name)
}

fn feed_id(adapter: &mut Inotify, targets: &Targets, id: i32, mask: EventMask, name: Option<&str>) -> Drain {
    let mut sink = Sink { targets, capacity: usize::MAX, events: Vec::new(), lost: None };
    adapter.translate(id, mask, name.map(OsStr::new), SystemTime::now(), &mut sink);
    Drain { events: sink.events, lost: sink.lost, interrupted: false }
}

#[test]
fn a_kernel_overflow_is_coverage_loss() {
    let fixture = new_fixture();
    let mut adapter = Inotify::start(&fixture.targets, fixture.targets.original()).unwrap();
    // The kernel reports an overflow with watch descriptor -1.
    let drain = feed_id(&mut adapter, &fixture.targets, -1, EventMask::Q_OVERFLOW, None);
    assert_eq!(drain.lost, Some(CoverageLost::Overflow));
}

#[test]
fn an_ignored_watch_is_loss_unless_its_directory_was_deleted_or_released() {
    for root in [Root::Parent, Root::Tree, Root::Admin] {
        let fixture = new_fixture();
        let mut adapter = Inotify::start(&fixture.targets, fixture.targets.original()).unwrap();
        let drain = feed(&mut adapter, &fixture.targets, root, "", EventMask::IGNORED, None);
        assert_eq!(drain.lost, Some(CoverageLost::WatchLost { root }), "{root:?}: unexplained IN_IGNORED");
    }

    // A tree directory's own deletion explains its IN_IGNORED.
    let fixture = new_fixture();
    let mut adapter = Inotify::start(&fixture.targets, fixture.targets.original()).unwrap();
    let deleted = feed(&mut adapter, &fixture.targets, Root::Tree, "sub", EventMask::DELETE_SELF, None);
    let ignored = feed(&mut adapter, &fixture.targets, Root::Tree, "sub", EventMask::IGNORED, None);
    assert_eq!((deleted.lost, deleted.events.len()), (None, 0), "the parent directory reports the deletion");
    assert_eq!(ignored.lost, None);

    // The parent and the admin directory must outlive the session.
    for root in [Root::Parent, Root::Admin] {
        let fixture = new_fixture();
        let mut adapter = Inotify::start(&fixture.targets, fixture.targets.original()).unwrap();
        let drain = feed(&mut adapter, &fixture.targets, root, "", EventMask::DELETE_SELF, None);
        assert_eq!(drain.lost, Some(CoverageLost::WatchLost { root }), "{root:?}");
    }

    // A released admin directory reports nothing, and its end is expected.
    let fixture = new_fixture();
    let mut adapter = Inotify::start(&fixture.targets, fixture.targets.original()).unwrap();
    let id = adapter.watches.iter().find(|(_, watch)| watch.root == Root::Admin).map(|(id, _)| *id).unwrap();
    adapter.release_admin(&|| false);
    for mask in [EventMask::MODIFY, EventMask::DELETE_SELF, EventMask::IGNORED] {
        let drain = feed_id(&mut adapter, &fixture.targets, id, mask, Some("index"));
        assert_eq!((drain.lost, drain.events.len()), (None, 0), "{mask:?}");
    }
}

#[test]
fn an_unmount_or_a_moved_parent_or_admin_directory_is_loss() {
    for (root, mask) in [
        (Root::Tree, EventMask::UNMOUNT),
        (Root::Parent, EventMask::UNMOUNT),
        (Root::Parent, EventMask::MOVE_SELF),
        (Root::Admin, EventMask::MOVE_SELF),
    ] {
        let fixture = new_fixture();
        let mut adapter = Inotify::start(&fixture.targets, fixture.targets.original()).unwrap();
        let drain = feed(&mut adapter, &fixture.targets, root, "", mask, None);
        assert_eq!(drain.lost, Some(CoverageLost::WatchLost { root }), "{root:?} {mask:?}");
    }

    // A moved tree directory is reported by the directory it left.
    let fixture = new_fixture();
    let mut adapter = Inotify::start(&fixture.targets, fixture.targets.original()).unwrap();
    let drain = feed(&mut adapter, &fixture.targets, Root::Tree, "sub", EventMask::MOVE_SELF, None);
    assert_eq!((drain.lost, drain.events.len()), (None, 0));
}

#[test]
fn event_masks_map_to_kinds_under_the_watch_path() {
    let fixture = new_fixture();
    let mut adapter = Inotify::start(&fixture.targets, fixture.targets.original()).unwrap();
    for (mask, kind) in [
        (EventMask::CREATE, EventKind::Created),
        (EventMask::DELETE, EventKind::Removed),
        (EventMask::MODIFY, EventKind::Modified),
        (EventMask::CLOSE_WRITE, EventKind::Modified),
        (EventMask::ATTRIB, EventKind::Metadata),
        (EventMask::MOVED_FROM, EventKind::MovedFrom),
        (EventMask::MOVED_TO, EventKind::MovedTo),
    ] {
        let drain = feed(&mut adapter, &fixture.targets, Root::Tree, "sub", mask, Some("f"));
        assert_eq!(drain.events.len(), 1, "{mask:?}");
        let event = &drain.events[0];
        assert_eq!((&event.scope, event.kind, event.directory), (&Scope::Tree("sub/f".into()), kind, Some(false)), "{mask:?}");
    }
    let drain = feed(&mut adapter, &fixture.targets, Root::Tree, "sub", EventMask::CREATE | EventMask::ISDIR, Some("d"));
    assert_eq!(drain.events[0].directory, Some(true));
    let drain = feed(&mut adapter, &fixture.targets, Root::Tree, "sub", EventMask::ATTRIB, None);
    assert_eq!(drain.events[0].scope, Scope::Tree("sub".into()), "an event on the watched directory itself");
}

#[test]
fn the_kernel_queue_overflowing_between_drains_is_coverage_loss() {
    let limit: usize = fs::read_to_string("/proc/sys/fs/inotify/max_queued_events").unwrap().trim().parse().unwrap();
    if limit > 32_768 {
        eprintln!("skipping: max_queued_events is {limit}; overflowing it would take too long here");
        return;
    }
    let fixture = new_fixture();
    let mut observation = Observation::start(fixture.targets.clone(), ObserverOptions { queue_capacity: usize::MAX, ..ObserverOptions::default() }, &|| false).unwrap();
    // Each file queues IN_CREATE, IN_MODIFY, and IN_CLOSE_WRITE.
    for index in 0..limit / 3 + 100 {
        fs::write(fixture.checkout.join(format!("f{index}")), b"x").unwrap();
    }
    assert_eq!(observation.drain(&|| false).lost, Some(CoverageLost::Overflow));
}

#[test]
fn the_instance_limit_is_a_setup_failure() {
    let fixture = new_fixture();
    // Exhausts this process's descriptors (nextest runs each test in its own
    // process), which fails `inotify_init1` with the same EMFILE as the
    // per-user instance limit.
    let mut original = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut original) }, 0);
    let lowered = libc::rlimit { rlim_cur: original.rlim_cur.min(256), rlim_max: original.rlim_max };
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &lowered) }, 0);
    let mut held = Vec::new();
    let exhausted = loop {
        match fs::File::open("/dev/null") {
            Ok(file) => held.push(file),
            Err(error) => break error,
        }
    };
    let result = Observation::start(fixture.targets.clone(), ObserverOptions::default(), &|| false);
    drop(held);
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &original) }, 0);

    assert_eq!(exhausted.raw_os_error(), Some(libc::EMFILE));
    match result {
        Err(ObservationError::Setup { source, .. }) => assert_eq!(source.raw_os_error(), Some(libc::EMFILE)),
        other => panic!("expected a setup failure, got {other:?}"),
    }
}

fn thread_count() -> usize {
    fs::read_dir("/proc/self/task").unwrap().count()
}

/// Sniff sees the session's watches as this process's evidence; registering
/// them is what keeps the decisive query from refusing the removal, and
/// dropping the session leaves neither a watch nor a thread.
#[test]
fn sniff_sees_the_watches_until_the_session_ends() {
    let fixture = new_fixture();
    let threads = thread_count();
    let mut observation = Observation::start(fixture.targets.clone(), ObserverOptions::default(), &|| false).unwrap();
    let _ = observation.drain(&|| false);
    assert_eq!(thread_count(), threads, "the adapter starts no thread");

    match check_usage(&SniffUsage, &fixture.checkout, &OwnedHandles::default()) {
        Err(UsageRefusal::InUse(in_use)) => {
            let own = in_use.holders.iter().find(|holder| holder.pid == std::process::id()).expect("this process holds the watches");
            assert!(own.evidence.iter().all(|evidence| evidence.kind == EvidenceKind::WatchRegistration), "{own:?}");
        }
        other => panic!("unregistered watches must refuse: {other:?}"),
    }

    let mut own = OwnedHandles::default();
    observation.register_handles(&mut own);
    let clearance = check_usage(&SniffUsage, &fixture.checkout, &own).expect("registered watches are excluded");
    assert_eq!(clearance.own_handles_excluded, 2, "the checkout and its subdirectory");

    drop(observation);
    let clearance = check_usage(&SniffUsage, &fixture.checkout, &OwnedHandles::default()).expect("no watch survives the session");
    assert_eq!(clearance.own_handles_excluded, 0);
    assert_eq!(thread_count(), threads);
}
