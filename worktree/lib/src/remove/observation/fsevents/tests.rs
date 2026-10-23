//! macOS-only facts of the FSEvents adapter that the shared contract tests
//! cannot provoke on demand: the stream's own flags fed through `deliver`, a
//! burst large enough to make FSEvents drop events, resolved and differently
//! cased paths, the run-loop thread's end, what Sniff sees of the session,
//! and the wait policy against a stream whose callback is held.

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
        delivery: Delivery::new([()]),
        held: AtomicBool::new(false),
    }
}

fn take(shared: &Shared, targets: &Targets) -> Drain {
    let mut sink = Sink { targets, capacity: 64, events: Vec::new(), lost: None };
    shared.queue.take_into(&mut sink);
    Drain { events: sink.events, lost: sink.lost, interrupted: false }
}

fn summary(drain: &Drain) -> Vec<(Scope, EventKind, Option<bool>)> {
    drain.events.iter().map(|event| (event.scope.clone(), event.kind, event.directory)).collect()
}

fn tree(path: &str) -> Scope {
    Scope::Tree(PathBuf::from(path))
}

const FILE: u32 = fs_events::kFSEventStreamEventFlagItemIsFile;
const DIR: u32 = fs_events::kFSEventStreamEventFlagItemIsDir;

/// Each loss flag alone, and several at once, is loss that keeps the raw
/// mask (bits without a name too) and names every flag, in declaration order.
#[test]
fn every_dropped_or_rescan_flag_is_loss_with_its_raw_mask_and_names() {
    use FsEventsLossFlag::{KernelDropped, MustScanSubDirs, UserDropped};
    let fixture = new_fixture();
    let resolved = resolve(&fixture);
    let must_scan = fs_events::kFSEventStreamEventFlagMustScanSubDirs;
    let user = fs_events::kFSEventStreamEventFlagUserDropped;
    let kernel = fs_events::kFSEventStreamEventFlagKernelDropped;
    // A flag bit no loss flag names (`kFSEventStreamEventFlagRootChanged`).
    let unnamed = fs_events::kFSEventStreamEventFlagRootChanged;
    let cases = [
        (must_scan, vec![MustScanSubDirs]),
        (user, vec![UserDropped]),
        (kernel, vec![KernelDropped]),
        (must_scan | user | kernel | unnamed, vec![KernelDropped, UserDropped, MustScanSubDirs]),
    ];
    for (raw, flags) in cases {
        let shared = shared(&fixture);
        shared.deliver(&resolved.original.join("sub"), FILE | fs_events::kFSEventStreamEventFlagItemCreated, SystemTime::now());
        // FSEvents reports a drop on the watched root or on `/`; either is loss.
        shared.deliver(Path::new("/"), raw, SystemTime::now());
        // Only the first loss is kept.
        shared.deliver(Path::new("/"), kernel, SystemTime::now());
        let drain = take(&shared, &fixture.targets);
        assert_eq!(drain.lost, Some(CoverageLost::FsEventsDropped { raw, flags }), "{raw:#x}");
        assert_eq!(summary(&drain), vec![(tree("sub"), EventKind::Created, Some(false))], "{raw:#x}: earlier events are kept");
    }
    let combined = CoverageLost::FsEventsDropped { raw: 0x27, flags: vec![KernelDropped, UserDropped, MustScanSubDirs] };
    assert_eq!(combined.to_string(), "FSEvents could not report every change (KernelDropped, UserDropped, MustScanSubDirs; flags 0x27)");
}

/// The payload reaches the report's diagnostics with every flag, including
/// `MustScanSubDirs` alone, and reads back unchanged.
#[test]
fn a_loss_payload_survives_into_the_serialized_diagnostics() {
    let fixture = new_fixture();
    for raw in [fs_events::kFSEventStreamEventFlagMustScanSubDirs, fs_events::kFSEventStreamEventFlagKernelDropped | fs_events::kFSEventStreamEventFlagUserDropped] {
        let shared = Arc::new(shared(&fixture));
        let delivered = Arc::clone(&shared);
        let mut observation = Observation::with_backend(fixture.targets.clone(), ObserverOptions::default(), Box::new(Delivered(delivered)));
        shared.deliver(Path::new("/"), raw, SystemTime::now());
        let lost = observation.drain(&|| false).lost.expect("lost");

        let diagnostics = observation.diagnostics();
        let loss = diagnostics.loss.as_ref().expect("recorded");
        assert_eq!(loss.reason, lost);
        assert_eq!(loss.stage, crate::remove::observation::ObservationStage::PreMove);
        let json = serde_json::to_value(&diagnostics).unwrap();
        assert_eq!(json["backend"], "fs_events");
        assert_eq!(json["loss"]["reason"]["kind"], "fs_events_dropped");
        assert_eq!(json["loss"]["reason"]["raw"], raw);
        let names: Vec<FsEventsLossFlag> = serde_json::from_value(json["loss"]["reason"]["flags"].clone()).unwrap();
        assert_eq!(Some(names), match &lost { CoverageLost::FsEventsDropped { flags, .. } => Some(flags.clone()), _ => None });
        assert_eq!(serde_json::from_value::<crate::remove::observation::ObservationDiagnostics>(json).unwrap(), diagnostics);
    }
}

/// A backend that only hands over what `deliver` queued.
struct Delivered(Arc<Shared>);

impl Backend for Delivered {
    fn drain(&mut self, sink: &mut Sink<'_>, _interrupted: &dyn Fn() -> bool) -> Waited {
        self.0.queue.take_into(sink);
        Waited::Done
    }

    fn release_admin(&mut self, _interrupted: &dyn Fn() -> bool) -> Waited {
        Waited::Done
    }

    fn handles(&self) -> Vec<(EvidenceKind, u64)> {
        Vec::new()
    }

    fn backend(&self) -> ObservationBackend {
        ObservationBackend::FsEvents
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
    let mut observation = Observation::start(targets, ObserverOptions::default(), &|| false).unwrap();
    fs::write(fixture.checkout.join("sub").join("new.txt"), b"x").unwrap();
    let mut events: Vec<_> = observation.drain(&|| false).events.into_iter().map(|event| (event.scope, event.kind)).collect();
    rename_away(&fixture.checkout, &fixture.quarantine);
    observation.moved_to_quarantine(&|| false);
    events.extend(observation.drain(&|| false).events.into_iter().map(|event| (event.scope, event.kind)));
    assert!(events.contains(&(tree("sub/new.txt"), EventKind::Created)), "{events:?}");
    assert!(events.contains(&(Scope::Original(PathBuf::new()), EventKind::MovedFrom)), "{events:?}");
    assert!(events.contains(&(Scope::Quarantine(PathBuf::new()), EventKind::MovedTo)), "{events:?}");
}

fn rename_away(from: &Path, to: &Path) {
    crate::remove::quarantine::rename::rename_no_replace(from, to).unwrap();
}

/// FSEvents promises no ordering, so a drain can return before a few of a
/// burst's events (see the adapter's module docs). None may be lost: each
/// creation is delivered by some drain, or a loss is reported.
#[test]
fn a_burst_is_delivered_eventually_or_reported_as_lost() {
    const FILES: usize = 2_000;
    const DRAINS: usize = 20;
    let fixture = new_fixture();
    let mut observation = Observation::start(fixture.targets.clone(), ObserverOptions::default(), &|| false).unwrap();
    let sub = fixture.checkout.join("sub");
    for index in 0..FILES {
        fs::write(sub.join(format!("f{index}")), b"x").unwrap();
    }
    let mut created = std::collections::HashSet::new();
    for _ in 0..DRAINS {
        let drain = observation.drain(&|| false);
        created.extend(drain.events.iter().filter(|event| event.kind == EventKind::Created).map(|event| event.scope.clone()));
        if let Some(loss) = &drain.lost {
            eprintln!("the burst was reported as lost ({loss}) after {} creations", created.len());
            return;
        }
        if (0..FILES).all(|index| created.contains(&tree(&format!("sub/f{index}")))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("no loss reported, yet {DRAINS} drains delivered only {} of {FILES} creations", created.len());
}

#[test]
fn the_run_loop_thread_ends_with_the_session_even_with_a_full_queue() {
    let fixture = new_fixture();
    let adapter = FsEvents::start(&fixture.targets, ObserverOptions { queue_capacity: 1, ..ObserverOptions::default() }, false, &|| false).unwrap();
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
        match FsEvents::start(&targets, ObserverOptions::default(), false, &|| false) {
            Err(ObservationError::Setup { path, .. }) => assert_eq!(path, expected, "{name}"),
            Err(other) => panic!("{name}: {other:?}"),
            Ok(_) => panic!("{name}: started"),
        }
    }
}

#[test]
fn sniff_sees_nothing_of_the_session() {
    let fixture = new_fixture();
    let mut observation = Observation::start(fixture.targets.clone(), ObserverOptions::default(), &|| false).unwrap();
    fs::write(fixture.checkout.join("sub").join("x"), b"x").unwrap();
    let _ = observation.drain(&|| false);
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
    let elsewhere = tempfile::Builder::new().prefix(".wt-fsevents-volume-").tempdir_in(crate::test_support::package_dir()).unwrap();
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
    let mut observation = Observation::start(targets, ObserverOptions::default(), &|| false).unwrap();
    for round in 0..25 {
        let name = format!("f{round}.txt");
        fs::write(checkout.join(&name), b"x").unwrap();
        let drain = observation.drain(&|| false);
        assert!(
            drain.events.iter().any(|event| event.scope == tree(&name) && event.kind == EventKind::Created),
            "round {round}: {:?}",
            summary(&drain)
        );
        assert_eq!(drain.lost, None);
    }
}

// ---- the wait policy against a real stream; `held` delays the callback as a
// slow fseventsd would ----

fn take_after(adapter: &mut FsEvents, targets: &Targets, interrupted: &dyn Fn() -> bool) -> (Waited, Drain) {
    let mut sink = Sink { targets, capacity: usize::MAX, events: Vec::new(), lost: None };
    let waited = adapter.drain(&mut sink, interrupted);
    (waited, Drain { events: sink.events, lost: sink.lost, interrupted: waited == Waited::Interrupted })
}

/// No delivery for the stall limit is `DeliveryStalled`, with no watch named
/// (one stream covers every place), and no claim that events were dropped.
#[test]
fn a_held_stream_stalls_the_drain_at_the_session_policy() {
    let fixture = new_fixture();
    let mut adapter = FsEvents::start(&fixture.targets, ObserverOptions::default(), false, &|| false).unwrap();
    let stall = Duration::from_millis(300);
    adapter.policy = WaitPolicy { stall, ceiling: Duration::from_secs(60) };
    adapter.shared.held.store(true, Ordering::SeqCst);
    let started = Instant::now();

    let (waited, drain) = take_after(&mut adapter, &fixture.targets, &|| false);

    assert_eq!(waited, Waited::Done);
    match drain.lost {
        Some(CoverageLost::DeliveryStalled { waited, watch: None }) => assert!(waited >= stall, "{waited:?}"),
        other => panic!("expected a stall, got {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(30), "the stall, not the ceiling, ended it");
}

/// Events that keep arriving are progress, but the ceiling spans every
/// marker round of one drain, so they cannot hold it open. The stream is
/// held, so only the feeder delivers: for each marker the drain writes, it
/// delivers a busy event and only then the marker, through the callback's
/// own `deliver`. Every round therefore sees an event however the threads
/// are scheduled, and none is quiet.
#[test]
fn the_ceiling_spans_every_marker_round_of_a_busy_drain() {
    let fixture = new_fixture();
    let options = ObserverOptions { queue_capacity: usize::MAX, ..ObserverOptions::default() };
    let mut adapter = FsEvents::start(&fixture.targets, options, false, &|| false).unwrap();
    adapter.shared.held.store(true, Ordering::SeqCst);
    let ceiling = Duration::from_secs(1);
    adapter.policy = WaitPolicy { stall: Duration::from_secs(20), ceiling };
    let stop = Arc::new(AtomicBool::new(false));
    let feeder = {
        let (stop, shared) = (Arc::clone(&stop), Arc::clone(&adapter.shared));
        let mut next = adapter.marker + 1;
        let path = shared.paths.original.join("sub").join("busy");
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let marker = shared.paths.markers.join(next.to_string());
                if !marker.exists() {
                    std::thread::sleep(Duration::from_micros(100));
                    continue;
                }
                shared.deliver(&path, FILE | fs_events::kFSEventStreamEventFlagItemModified, SystemTime::now());
                shared.deliver(&marker, FILE | fs_events::kFSEventStreamEventFlagItemCreated, SystemTime::now());
                next += 1;
            }
        })
    };
    let started = Instant::now();

    let (_, drain) = take_after(&mut adapter, &fixture.targets, &|| false);
    stop.store(true, Ordering::SeqCst);
    feeder.join().unwrap();

    match drain.lost {
        Some(CoverageLost::DrainCeiling { waited, watch: None }) => assert!(waited >= ceiling, "{waited:?}"),
        other => panic!("expected the ceiling, got {other:?} after {} events", drain.events.len()),
    }
    assert!(!drain.events.is_empty(), "the events that kept the drain busy are kept");
    assert!(started.elapsed() < Duration::from_secs(20), "the ceiling, not the stall, ended it");
}

/// The interrupt ends a wait at once, records no loss, and a held release
/// keeps the admin directory watched.
#[test]
fn an_interrupt_ends_a_held_wait_without_loss() {
    let fixture = new_fixture();
    let mut adapter = FsEvents::start(&fixture.targets, ObserverOptions::default(), false, &|| false).unwrap();
    adapter.shared.held.store(true, Ordering::SeqCst);
    let started = Instant::now();
    let soon = || started.elapsed() >= Duration::from_millis(100);

    let (waited, drain) = take_after(&mut adapter, &fixture.targets, &soon);

    assert_eq!((waited, drain.lost), (Waited::Interrupted, None));
    assert!(started.elapsed() < Duration::from_secs(5), "{:?}: far below the 30 s stall", started.elapsed());
    assert_eq!(adapter.release_admin(&|| true), Waited::Interrupted);
    assert!(!adapter.shared.admin_released.load(Ordering::SeqCst), "an interrupted release keeps the watch");
    assert_eq!(adapter.moved_to_quarantine(&|| true), Waited::Interrupted);
    assert!(adapter.shared.moved.load(Ordering::SeqCst), "the move happened; only its labeling wait was cut short");
}

/// Startup waits under the same policy: a stream that never delivers its
/// first marker fails with diagnostics naming the stall at the startup stage,
/// and an interrupt during that wait is `Interrupted`, not an environment
/// limit.
#[test]
fn startup_waits_under_the_policy_and_stops_for_an_interrupt() {
    let fixture = new_fixture();
    let options = ObserverOptions { wait: WaitPolicy { stall: Duration::from_millis(300), ceiling: Duration::from_secs(60) }, ..ObserverOptions::default() };
    match FsEvents::start_held(&fixture.targets, options, false, &|| false, true) {
        Err(error @ ObservationError::NotDelivering(_)) => {
            assert!(error.to_string().contains("could not confirm delivery of earlier changes: no progress for"), "{error}");
            let diagnostics = error.diagnostics().expect("carried");
            assert_eq!(diagnostics.backend, ObservationBackend::FsEvents);
            assert_eq!((diagnostics.last_round_trip_ms, diagnostics.longest_round_trip_ms), (None, None), "no marker was confirmed");
            let loss = diagnostics.loss.as_ref().expect("the stall");
            assert_eq!(loss.stage, crate::remove::observation::ObservationStage::Startup);
            assert!(matches!(loss.reason, CoverageLost::DeliveryStalled { watch: None, .. }), "{:?}", loss.reason);
        }
        Err(other) => panic!("expected a startup loss, got {other}"),
        Ok(_) => panic!("started without a delivered marker"),
    }
    let started = Instant::now();
    match FsEvents::start_held(&fixture.targets, ObserverOptions::default(), false, &|| true, true) {
        Err(ObservationError::Interrupted) => {}
        Err(other) => panic!("expected an interrupt, got {other}"),
        Ok(_) => panic!("started without a delivered marker"),
    }
    assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
}

/// A session started on a real stream reports how long starting took and
/// its confirmed marker round trips; a later drain updates them.
#[test]
fn a_started_session_reports_its_startup_and_marker_round_trips() {
    let fixture = new_fixture();
    let mut observation = Observation::start(fixture.targets.clone(), ObserverOptions::default(), &|| false).unwrap();

    let started = observation.diagnostics();
    assert_eq!(started.backend, ObservationBackend::FsEvents);
    assert!(started.startup_ms.is_some());
    let (Some(last), Some(longest)) = (started.last_round_trip_ms, started.longest_round_trip_ms) else {
        panic!("startup confirmed a marker: {started:?}")
    };
    assert!(last <= longest && longest <= started.startup_ms.unwrap(), "{started:?}");
    assert_eq!(started.loss, None);

    assert_eq!(observation.drain(&|| false).lost, None);
    let drained = observation.diagnostics();
    assert!(drained.longest_round_trip_ms >= started.longest_round_trip_ms, "{drained:?}");
    assert_eq!(drained.startup_ms, started.startup_ms, "startup is timed once");
}
