//! macOS-only facts of the FSEvents adapter that the shared contract tests
//! cannot provoke on demand: the stream's own flags fed through `deliver`, a
//! burst large enough to make FSEvents drop events, resolved and differently
//! cased paths, the run-loop thread's end, and what Sniff sees of the session.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::*;
use crate::remove::observation::{Drain, Observation, ObserverOptions, Scope};
use crate::remove::usage::{OwnedHandles, SniffUsage, check_usage};

struct Fixture {
    _root: tempfile::TempDir,
    checkout: PathBuf,
    quarantine: PathBuf,
    admin: PathBuf,
    markers: PathBuf,
    targets: Targets,
}

fn new_fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("parent").join("checkout");
    fs::create_dir_all(checkout.join("sub")).unwrap();
    let admin = root.path().join("admin");
    fs::create_dir(&admin).unwrap();
    let markers = root.path().join("markers");
    fs::create_dir(&markers).unwrap();
    let quarantine = checkout.with_file_name(".wt-remove-checkout-0123456789abcdef");
    let targets = Targets::new(&checkout, &quarantine, &admin).unwrap();
    Fixture { _root: root, checkout, quarantine, admin, markers, targets }
}

fn resolve(fixture: &Fixture) -> Paths {
    Paths::resolve(&fixture.targets, &fixture.markers, false).unwrap()
}

/// An adapter's shared state, as the callback sees it, without a stream.
fn shared(fixture: &Fixture) -> Shared {
    Shared {
        queue: EventQueue::new(fixture.targets.clone(), 64),
        paths: resolve(fixture),
        moved: AtomicBool::new(false),
        admin_released: AtomicBool::new(false),
        marker_seen: AtomicU64::new(0),
    }
}

fn take(shared: &Shared, targets: &Targets) -> Drain {
    let mut sink = Sink { targets, capacity: 64, events: Vec::new(), lost: None };
    shared.queue.take_into(&mut sink);
    Drain { events: sink.events, lost: sink.lost }
}

fn summary(drain: &Drain) -> Vec<(Scope, EventKind, Option<bool>)> {
    drain.events.iter().map(|event| (event.scope.clone(), event.kind, event.directory)).collect()
}

fn tree(path: &str) -> Scope {
    Scope::Tree(PathBuf::from(path))
}

const FILE: u32 = fs_events::kFSEventStreamEventFlagItemIsFile;
const DIR: u32 = fs_events::kFSEventStreamEventFlagItemIsDir;

#[test]
fn every_dropped_or_rescan_flag_is_coverage_loss() {
    let fixture = new_fixture();
    let resolved = resolve(&fixture);
    for flag in [
        fs_events::kFSEventStreamEventFlagMustScanSubDirs,
        fs_events::kFSEventStreamEventFlagUserDropped,
        fs_events::kFSEventStreamEventFlagKernelDropped,
        fs_events::kFSEventStreamEventFlagMustScanSubDirs | fs_events::kFSEventStreamEventFlagUserDropped,
    ] {
        let shared = shared(&fixture);
        shared.deliver(&resolved.original.join("sub"), FILE | fs_events::kFSEventStreamEventFlagItemCreated, SystemTime::now());
        // FSEvents reports a drop on the watched root or on `/`; either is loss.
        shared.deliver(Path::new("/"), flag, SystemTime::now());
        let drain = take(&shared, &fixture.targets);
        assert_eq!(drain.lost, Some(CoverageLost::Overflow), "{flag:#x}");
        assert_eq!(summary(&drain), vec![(tree("sub"), EventKind::Created, Some(false))], "{flag:#x}: earlier events are kept");
    }
}

#[test]
fn a_mount_or_unmount_under_a_watched_path_is_loss_and_elsewhere_ignored() {
    let fixture = new_fixture();
    let resolved = resolve(&fixture);
    for (flag, path, expected) in [
        (fs_events::kFSEventStreamEventFlagUnmount, resolved.original.join("volume"), Some(CoverageLost::WatchLost { root: Root::Tree })),
        (fs_events::kFSEventStreamEventFlagMount, resolved.original.join("volume"), Some(CoverageLost::WatchLost { root: Root::Tree })),
        (fs_events::kFSEventStreamEventFlagUnmount, resolved.admin.join("volume"), Some(CoverageLost::WatchLost { root: Root::Admin })),
        (fs_events::kFSEventStreamEventFlagUnmount, PathBuf::from("/Volumes/elsewhere"), None),
    ] {
        let shared = shared(&fixture);
        shared.deliver(&path, flag, SystemTime::now());
        let drain = take(&shared, &fixture.targets);
        assert_eq!(drain.lost, expected, "{flag:#x} at {}", path.display());
        assert!(drain.events.is_empty());
    }
}

#[test]
fn coalesced_item_flags_are_each_reported() {
    let fixture = new_fixture();
    let resolved = resolve(&fixture);
    let shared = shared(&fixture);
    let created_and_removed = FILE
        | fs_events::kFSEventStreamEventFlagItemCreated
        | fs_events::kFSEventStreamEventFlagItemModified
        | fs_events::kFSEventStreamEventFlagItemRemoved;
    shared.deliver(&resolved.original.join("sub/gone.txt"), created_and_removed, SystemTime::now());
    for flag in [
        fs_events::kFSEventStreamEventFlagItemInodeMetaMod,
        fs_events::kFSEventStreamEventFlagItemFinderInfoMod,
        fs_events::kFSEventStreamEventFlagItemChangeOwner,
        fs_events::kFSEventStreamEventFlagItemXattrMod,
    ] {
        shared.deliver(&resolved.original.join("sub"), DIR | flag, SystemTime::now());
    }
    // No item-type flag: the platform does not say.
    shared.deliver(&resolved.original.join("x"), fs_events::kFSEventStreamEventFlagItemModified, SystemTime::now());
    // No item flag at all (`HistoryDone`, `EventIdsWrapped`): nothing to report.
    shared.deliver(&resolved.original.join("x"), fs_events::kFSEventStreamEventFlagHistoryDone, SystemTime::now());

    let drain = take(&shared, &fixture.targets);
    assert_eq!(
        summary(&drain),
        vec![
            (tree("sub/gone.txt"), EventKind::Created, Some(false)),
            (tree("sub/gone.txt"), EventKind::Modified, Some(false)),
            (tree("sub/gone.txt"), EventKind::Removed, Some(false)),
            (tree("sub"), EventKind::Metadata, Some(true)),
            (tree("sub"), EventKind::Metadata, Some(true)),
            (tree("sub"), EventKind::Metadata, Some(true)),
            (tree("sub"), EventKind::Metadata, Some(true)),
            (tree("x"), EventKind::Modified, None),
        ]
    );
    assert_eq!(drain.lost, None);
}

#[test]
fn a_rename_is_moved_to_where_the_path_exists_and_moved_from_where_it_does_not() {
    let fixture = new_fixture();
    let resolved = resolve(&fixture);
    let shared = shared(&fixture);
    let renamed = DIR | fs_events::kFSEventStreamEventFlagItemRenamed;
    shared.deliver(&resolved.original.join("sub"), renamed, SystemTime::now());
    shared.deliver(&resolved.original.join("missing"), renamed, SystemTime::now());
    let drain = take(&shared, &fixture.targets);
    assert_eq!(
        summary(&drain),
        vec![(tree("sub"), EventKind::MovedTo, Some(true)), (tree("missing"), EventKind::MovedFrom, Some(true))]
    );
}

#[test]
fn paths_are_labeled_by_where_the_tree_is() {
    let fixture = new_fixture();
    let resolved = resolve(&fixture);
    let shared = shared(&fixture);
    let created = FILE | fs_events::kFSEventStreamEventFlagItemCreated;
    let deliver_all = |shared: &Shared| {
        for path in [
            resolved.original.clone(),
            resolved.original.join("a.txt"),
            resolved.quarantine.clone(),
            resolved.quarantine.join("a.txt"),
            resolved.admin.join("index"),
            resolved.admin.join("ORIG_HEAD"),
            resolved.admin.clone(),
            resolved.parent.join("sibling"),
            resolved.parent.join("checkout2/a.txt"),
            resolved.parent.clone(),
        ] {
            shared.deliver(&path, created, SystemTime::now());
        }
    };

    deliver_all(&shared);
    assert_eq!(
        summary(&take(&shared, &fixture.targets)),
        vec![
            (Scope::Original(PathBuf::new()), EventKind::Created, Some(false)),
            (tree("a.txt"), EventKind::Created, Some(false)),
            (Scope::Quarantine(PathBuf::new()), EventKind::Created, Some(false)),
            (Scope::Quarantine(PathBuf::from("a.txt")), EventKind::Created, Some(false)),
            (Scope::Admin("index".into()), EventKind::Created, Some(false)),
        ],
        "before the move"
    );

    shared.moved.store(true, Ordering::SeqCst);
    deliver_all(&shared);
    assert_eq!(
        summary(&take(&shared, &fixture.targets)),
        vec![
            (Scope::Original(PathBuf::new()), EventKind::Created, Some(false)),
            (Scope::Original(PathBuf::from("a.txt")), EventKind::Created, Some(false)),
            (Scope::Quarantine(PathBuf::new()), EventKind::Created, Some(false)),
            (tree("a.txt"), EventKind::Created, Some(false)),
            (Scope::Admin("index".into()), EventKind::Created, Some(false)),
        ],
        "after the move"
    );

    shared.admin_released.store(true, Ordering::SeqCst);
    shared.deliver(&resolved.admin.join("index"), created, SystemTime::now());
    shared.deliver(&resolved.admin, DIR | fs_events::kFSEventStreamEventFlagItemRemoved, SystemTime::now());
    shared.deliver(&resolved.admin.join("volume"), fs_events::kFSEventStreamEventFlagUnmount, SystemTime::now());
    let drain = take(&shared, &fixture.targets);
    assert_eq!((drain.events.len(), drain.lost), (0, None), "a released admin directory reports nothing");
}

#[test]
fn watched_paths_are_resolved_as_fsevents_reports_them() {
    let fixture = new_fixture();
    let resolved = resolve(&fixture);
    // The temporary directory is under `/var`, a link to `/private/var`.
    assert!(resolved.original.starts_with("/private"), "{}", resolved.original.display());
    assert_eq!(resolved.quarantine, resolved.original.with_file_name(".wt-remove-checkout-0123456789abcdef"));

    // On a case-insensitive volume a differently cased name is the same
    // directory; events carry the on-disk name, labels the session's own.
    let upper = fixture.checkout.with_file_name("CHECKOUT");
    if !upper.is_dir() {
        eprintln!("this volume is case-sensitive; the case variant was not exercised");
        return;
    }
    let targets = Targets::new(&upper, &fixture.quarantine, &fixture.admin).unwrap();
    let mut observation = Observation::start(targets, ObserverOptions::default()).unwrap();
    fs::write(fixture.checkout.join("sub").join("new.txt"), b"x").unwrap();
    let mut events: Vec<_> = observation.drain().events.into_iter().map(|event| (event.scope, event.kind)).collect();
    rename_away(&fixture.checkout, &fixture.quarantine);
    observation.moved_to_quarantine();
    events.extend(observation.drain().events.into_iter().map(|event| (event.scope, event.kind)));
    assert!(events.contains(&(tree("sub/new.txt"), EventKind::Created)), "{events:?}");
    assert!(events.contains(&(Scope::Original(PathBuf::new()), EventKind::MovedFrom)), "{events:?}");
    assert!(events.contains(&(Scope::Quarantine(PathBuf::new()), EventKind::MovedTo)), "{events:?}");
}

fn rename_away(from: &Path, to: &Path) {
    crate::remove::quarantine::rename::rename_no_replace(from, to).unwrap();
}

#[test]
fn a_burst_is_delivered_completely_or_reported_as_lost() {
    const FILES: usize = 2_000;
    let fixture = new_fixture();
    let mut observation = Observation::start(fixture.targets.clone(), ObserverOptions::default()).unwrap();
    let sub = fixture.checkout.join("sub");
    for index in 0..FILES {
        fs::write(sub.join(format!("f{index}")), b"x").unwrap();
    }
    let drain = observation.drain();
    let created: std::collections::HashSet<_> = drain
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Created)
        .map(|event| event.scope.clone())
        .collect();
    let all = (0..FILES).all(|index| created.contains(&tree(&format!("sub/f{index}"))));
    match &drain.lost {
        None => assert!(all, "no loss reported, so all {FILES} creations must be there: {} were", created.len()),
        Some(loss) => eprintln!("the burst was reported as lost ({loss}) after {} creations", created.len()),
    }
}

#[test]
fn the_run_loop_thread_ends_with_the_session_even_with_a_full_queue() {
    let fixture = new_fixture();
    let adapter = FsEvents::start(&fixture.targets, ObserverOptions { queue_capacity: 1 }, false).unwrap();
    let alive = Arc::downgrade(&adapter.thread_alive);
    for index in 0..50 {
        fs::write(fixture.checkout.join(format!("f{index}")), b"x").unwrap();
    }
    // Let the stream deliver into the full queue without a drain.
    std::thread::sleep(Duration::from_millis(3 * (LATENCY_SECONDS * 1000.0) as u64));
    drop(adapter);
    assert!(alive.upgrade().is_none(), "the run-loop thread ended before drop returned");
}

#[test]
fn a_setup_failure_names_the_path_and_starts_nothing() {
    let fixture = new_fixture();
    fs::write(fixture.checkout.with_file_name("file"), b"x").unwrap();
    let file = Targets::new(&fixture.checkout.with_file_name("file"), &fixture.quarantine, &fixture.admin).unwrap();
    let missing = Targets::new(&fixture.checkout.with_file_name("missing"), &fixture.quarantine, &fixture.admin).unwrap();
    for (name, targets) in [("a file", file), ("a missing checkout", missing)] {
        let expected = targets.original().to_path_buf();
        match FsEvents::start(&targets, ObserverOptions::default(), false) {
            Err(ObservationError::Setup { path, .. }) => assert_eq!(path, expected, "{name}"),
            Err(other) => panic!("{name}: {other:?}"),
            Ok(_) => panic!("{name}: started"),
        }
    }
}

#[test]
fn sniff_sees_nothing_of_the_session() {
    let fixture = new_fixture();
    let mut observation = Observation::start(fixture.targets.clone(), ObserverOptions::default()).unwrap();
    fs::write(fixture.checkout.join("sub").join("x"), b"x").unwrap();
    let _ = observation.drain();
    assert!(observation.backend.handles().is_empty(), "fseventsd holds the watch; nothing to register");
    let mut own = OwnedHandles::default();
    observation.register_handles(&mut own);

    let clearance = check_usage(&SniffUsage, &fixture.checkout, &own).expect("an unregistered FSEvents session does not refuse");
    assert_eq!(clearance.own_handles_excluded, 0);
}


/// The marker lives on the system volume; a checkout on another volume must
/// still be behind the barrier (fseventsd reads one ordered kernel queue for
/// every volume).
#[test]
fn the_barrier_holds_for_a_checkout_on_another_volume() {
    let elsewhere = tempfile::Builder::new().prefix(".wt-fsevents-volume-").tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let markers = MarkerDirectory::create().unwrap();
    let device = |path: &Path| fs::metadata(path).unwrap().dev();
    if device(elsewhere.path()) == device(&markers.0) {
        eprintln!("the checkout and the marker directory share a volume here; the cross-volume case was not exercised");
        return;
    }
    let checkout = elsewhere.path().join("parent").join("checkout");
    fs::create_dir_all(&checkout).unwrap();
    let admin = elsewhere.path().join("admin");
    fs::create_dir(&admin).unwrap();
    let targets = Targets::new(&checkout, &checkout.with_file_name(".wt-remove-checkout-0123456789abcdef"), &admin).unwrap();
    let mut observation = Observation::start(targets, ObserverOptions::default()).unwrap();
    for round in 0..25 {
        let name = format!("f{round}.txt");
        fs::write(checkout.join(&name), b"x").unwrap();
        let drain = observation.drain();
        assert!(
            drain.events.iter().any(|event| event.scope == tree(&name) && event.kind == EventKind::Created),
            "round {round}: {:?}",
            summary(&drain)
        );
        assert_eq!(drain.lost, None);
    }
}
