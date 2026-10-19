//! The observer contract. The injected observer's tests cover the shared
//! session rules: classification, bounds, typed loss, and release. The native
//! tests run the same contract against this platform's adapter with real
//! files, and on a platform without one assert that `start` says so.

use std::fs;
use std::path::{Path, PathBuf};

use sniff::filesystem::query::EvidenceKind;

use super::*;
use crate::remove::quarantine::rename::rename_no_replace;

/// `<root>/parent/checkout/sub/file.txt`, a sibling, and
/// `<root>/admin/{HEAD,gitdir,index}`.
struct Layout {
    _root: tempfile::TempDir,
    parent: PathBuf,
    checkout: PathBuf,
    quarantine: PathBuf,
    admin: PathBuf,
}

impl Layout {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("parent");
        let checkout = parent.join("checkout");
        fs::create_dir_all(checkout.join("sub")).unwrap();
        fs::write(checkout.join("sub").join("file.txt"), b"payload").unwrap();
        fs::create_dir(parent.join("sibling")).unwrap();
        let admin = root.path().join("admin");
        fs::create_dir(&admin).unwrap();
        for file in ["HEAD", "gitdir", "index"] {
            fs::write(admin.join(file), b"x").unwrap();
        }
        let quarantine = parent.join(".wt-remove-checkout-0123456789abcdef");
        Self { _root: root, parent, checkout, quarantine, admin }
    }

    fn targets(&self) -> Targets {
        Targets::new(&self.checkout, &self.quarantine, &self.admin).unwrap()
    }
}

fn tree(path: &str) -> Scope {
    Scope::Tree(PathBuf::from(path))
}

fn summary(drain: &Drain) -> Vec<(Scope, EventKind)> {
    drain.events.iter().map(|event| (event.scope.clone(), event.kind)).collect()
}

fn has(drain: &Drain, scope: &Scope, kind: EventKind) -> bool {
    drain.events.iter().any(|event| &event.scope == scope && event.kind == kind)
}

#[test]
fn targets_must_be_distinct_named_siblings() {
    let admin = Path::new("/repo/.git/worktrees/x");
    assert!(Targets::new(Path::new("/w/x"), Path::new("/w/.q"), admin).is_some());
    assert!(Targets::new(Path::new("/w/x"), Path::new("/elsewhere/.q"), admin).is_none(), "not a sibling");
    assert!(Targets::new(Path::new("/w/x"), Path::new("/w/x"), admin).is_none(), "same name");
    assert!(Targets::new(Path::new("/"), Path::new("/.q"), admin).is_none(), "no name");
}

#[test]
fn admin_files_are_the_identity_and_index_files() {
    for name in ["HEAD", "gitdir", "index", "index.lock", "locked", "sharedindex.0123abcd"] {
        assert!(is_admin_file(OsStr::new(name)), "{name}");
    }
    for name in ["ORIG_HEAD", "HEAD.lock", "logs", "commondir", "index.bak", "sharedindex"] {
        assert!(!is_admin_file(OsStr::new(name)), "{name}");
    }
}

// ---- injected observer ----

#[test]
fn injected_events_are_classified_and_unrelated_ones_dropped() {
    let layout = Layout::new();
    let (mut observation, injector) = Observation::injected(layout.targets(), ObserverOptions::default());
    injector.event(Root::Tree, "sub/new.txt", EventKind::Created);
    injector.event(Root::Tree, "", EventKind::Metadata);
    injector.event(Root::Parent, "checkout", EventKind::Created);
    injector.event(Root::Parent, "checkout/late.txt", EventKind::Created);
    injector.event(Root::Parent, ".wt-remove-checkout-0123456789abcdef", EventKind::MovedTo);
    injector.event(Root::Parent, "sibling", EventKind::Modified);
    injector.event(Root::Parent, "", EventKind::Metadata);
    injector.event(Root::Admin, "index", EventKind::Modified);
    injector.event(Root::Admin, "sharedindex.ab", EventKind::Created);
    injector.event(Root::Admin, "logs", EventKind::Modified);
    injector.event(Root::Admin, "", EventKind::Metadata);

    let drain = observation.drain();

    assert_eq!(
        summary(&drain),
        vec![
            (tree("sub/new.txt"), EventKind::Created),
            (tree(""), EventKind::Metadata),
            (Scope::Original(PathBuf::new()), EventKind::Created),
            (Scope::Original(PathBuf::from("late.txt")), EventKind::Created),
            (Scope::Quarantine(PathBuf::new()), EventKind::MovedTo),
            (Scope::Admin("index".into()), EventKind::Modified),
            (Scope::Admin("sharedindex.ab".into()), EventKind::Created),
        ]
    );
    assert_eq!(drain.lost, None);
    assert!(observation.drain().events.is_empty(), "a drain hands events over once");
}

#[test]
fn delayed_events_arrive_only_once_delivered() {
    let layout = Layout::new();
    let (mut observation, injector) = Observation::injected(layout.targets(), ObserverOptions::default());
    injector.delayed(Root::Tree, "slow.txt", EventKind::Created);
    injector.event(Root::Tree, "fast.txt", EventKind::Created);

    assert_eq!(summary(&observation.drain()), vec![(tree("fast.txt"), EventKind::Created)]);
    injector.deliver_delayed();
    assert_eq!(summary(&observation.drain()), vec![(tree("slow.txt"), EventKind::Created)]);
}

#[test]
fn coverage_loss_is_typed_and_stays_lost() {
    type Lose = fn(&injected::Injector);
    let cases: [(&str, Lose, CoverageLost); 4] = [
        ("overflow", |injector| injector.overflow(), CoverageLost::Overflow),
        ("parent", |injector| injector.lose_watch(Root::Parent), CoverageLost::WatchLost { root: Root::Parent }),
        ("tree", |injector| injector.lose_watch(Root::Tree), CoverageLost::WatchLost { root: Root::Tree }),
        ("admin", |injector| injector.lose_watch(Root::Admin), CoverageLost::WatchLost { root: Root::Admin }),
    ];
    for (name, lose, expected) in cases {
        let layout = Layout::new();
        let (mut observation, injector) = Observation::injected(layout.targets(), ObserverOptions::default());
        injector.event(Root::Tree, "before.txt", EventKind::Created);
        lose(&injector);

        let drain = observation.drain();
        assert_eq!(summary(&drain), vec![(tree("before.txt"), EventKind::Created)], "{name}: events before the loss are kept");
        assert_eq!(drain.lost.as_ref(), Some(&expected), "{name}");

        // Nothing later restores coverage, and the first loss is the one kept.
        injector.overflow();
        let later = observation.drain();
        assert!(later.events.is_empty(), "{name}");
        assert_eq!(later.lost, Some(expected), "{name}");
    }
}

#[test]
fn a_full_queue_is_coverage_loss_keeping_what_fit() {
    let layout = Layout::new();
    let (mut observation, injector) = Observation::injected(layout.targets(), ObserverOptions { queue_capacity: 3 });
    // Dropped siblings do not count against the bound.
    injector.event(Root::Parent, "sibling", EventKind::Created);
    for index in 0..5 {
        injector.event(Root::Tree, format!("f{index}"), EventKind::Created);
    }

    let drain = observation.drain();

    assert_eq!(drain.events.len(), 3);
    assert_eq!(drain.lost, Some(CoverageLost::QueueFull { capacity: 3 }));
    assert_eq!(observation.drain().lost, Some(CoverageLost::QueueFull { capacity: 3 }));
}

#[test]
fn exactly_the_capacity_is_not_a_loss() {
    let layout = Layout::new();
    let (mut observation, injector) = Observation::injected(layout.targets(), ObserverOptions { queue_capacity: 3 });
    for index in 0..3 {
        injector.event(Root::Tree, format!("f{index}"), EventKind::Created);
    }
    let drain = observation.drain();
    assert_eq!((drain.events.len(), drain.lost), (3, None));
}

#[test]
fn a_released_admin_directory_is_neither_reported_nor_lost() {
    let layout = Layout::new();
    let (mut observation, injector) = Observation::injected(layout.targets(), ObserverOptions::default());
    injector.event(Root::Admin, "index", EventKind::Modified);
    observation.release_admin();
    injector.event(Root::Admin, "index", EventKind::Removed);
    injector.lose_watch(Root::Admin);

    let drain = observation.drain();

    assert!(injector.admin_released());
    assert_eq!(summary(&drain), vec![(Scope::Admin("index".into()), EventKind::Modified)]);
    assert_eq!(drain.lost, None);
}

#[test]
fn the_move_is_relabeled_and_handles_registered_and_released() {
    let layout = Layout::new();
    let (mut observation, injector) = Observation::injected(layout.targets(), ObserverOptions::default());
    injector.hold_handle(EvidenceKind::WatchRegistration, 7);
    assert_eq!(observation.tree_location(), layout.checkout);

    observation.moved_to_quarantine();
    assert!(injector.moved());
    assert_eq!(observation.tree_location(), layout.quarantine);

    let mut own = OwnedHandles::for_process(42);
    observation.register_handles(&mut own);
    assert!(own.is_registered(EvidenceKind::WatchRegistration, 7));

    drop(observation);
    assert!(injector.ended(), "dropping the session ends it");
}

// ---- the native adapter ----

const NATIVE_ADAPTER: bool = cfg!(any(target_os = "linux", target_os = "macos", windows));

/// `ReadDirectoryChangesW` reports an attribute change as the same
/// `FILE_ACTION_MODIFIED` as a write.
const METADATA_IS_DISTINCT: bool = !cfg!(windows);

/// `ReadDirectoryChangesW` reports a move between two directories of a
/// watched subtree as a removal and a creation; only a rename within one
/// directory is a rename.
const MOVE_ACROSS_DIRECTORIES_IS_A_RENAME: bool = !cfg!(windows);

/// FSEvents reports a directory's removal or rename in place of changes below
/// it still waiting in the same batch, so the caller drains first.
const COALESCES_BELOW_DIRECTORY: bool = cfg!(target_os = "macos");

/// Drains where the platform coalesces, so the next directory operation does
/// not hide earlier changes below it; keeps what it drained.
fn settle(observation: &mut Observation, kept: &mut Drain) {
    if COALESCES_BELOW_DIRECTORY {
        let drain = observation.drain();
        kept.events.extend(drain.events);
        kept.lost = kept.lost.take().or(drain.lost);
    }
}

/// Whether every change is in the very next drain. FSEvents promises no
/// ordering, so on macOS a change can arrive in a later drain instead (see the
/// `fsevents` adapter); it must still arrive, and no loss may be reported.
const DRAIN_IS_A_BARRIER: bool = !cfg!(target_os = "macos");

/// Drains made after the first, while waiting for a late FSEvents event.
const LATE_DRAINS: usize = 20;

/// The next drain where the drain is a barrier; elsewhere the drains up to the
/// one where `done` holds, kept together.
fn drain_until(observation: &mut Observation, done: impl Fn(&Drain) -> bool) -> Drain {
    let mut kept = observation.drain();
    if DRAIN_IS_A_BARRIER {
        return kept;
    }
    for _ in 0..LATE_DRAINS {
        if done(&kept) || kept.lost.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        kept = then(kept, observation.drain());
    }
    kept
}

fn then(mut kept: Drain, drain: Drain) -> Drain {
    kept.events.extend(drain.events);
    kept.lost = kept.lost.or(drain.lost);
    kept
}

/// Starts the native adapter, or proves this platform has none yet.
fn native(layout: &Layout, options: ObserverOptions) -> Option<Observation> {
    match Observation::start(layout.targets(), options) {
        Ok(observation) if NATIVE_ADAPTER => Some(observation),
        Err(ObservationError::Unsupported { reason }) if !NATIVE_ADAPTER => {
            eprintln!("no native adapter on this platform yet ({reason}); the contract was not exercised");
            None
        }
        Ok(_) => panic!("an adapter started where none is expected"),
        Err(error) => panic!("observation did not start: {error}"),
    }
}

#[test]
fn native_drain_delivers_every_change_kind() {
    let layout = Layout::new();
    let Some(mut observation) = native(&layout, ObserverOptions::default()) else { return };
    let sub = layout.checkout.join("sub");

    // No sleep before a drain: where the drain is a barrier each change must
    // be in the very next one. A macOS drain waits about 11 ms for fseventsd,
    // so it runs fewer rounds.
    let rounds = if cfg!(target_os = "macos") { 25 } else { 50 };
    for round in 0..rounds {
        let name = format!("sub/f{round}.txt");
        fs::write(layout.checkout.join(&name), b"x").unwrap();
        let drain = drain_until(&mut observation, |drain| has(drain, &tree(&name), EventKind::Created));
        assert!(has(&drain, &tree(&name), EventKind::Created), "round {round}: {:?}", summary(&drain));
        assert_eq!(drain.lost, None, "round {round}");
    }

    fs::write(sub.join("file.txt"), b"changed").unwrap();
    let drain = drain_until(&mut observation, |drain| has(drain, &tree("sub/file.txt"), EventKind::Modified));
    assert!(has(&drain, &tree("sub/file.txt"), EventKind::Modified), "{:?}", summary(&drain));

    let mut permissions = fs::metadata(sub.join("file.txt")).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(sub.join("file.txt"), permissions).unwrap();
    let metadata = if METADATA_IS_DISTINCT { EventKind::Metadata } else { EventKind::Modified };
    let drain = drain_until(&mut observation, |drain| has(drain, &tree("sub/file.txt"), metadata));
    assert!(has(&drain, &tree("sub/file.txt"), metadata), "{:?}", summary(&drain));

    fs::rename(sub.join("f0.txt"), layout.checkout.join("moved.txt")).unwrap();
    let (from, to) = if MOVE_ACROSS_DIRECTORIES_IS_A_RENAME {
        (EventKind::MovedFrom, EventKind::MovedTo)
    } else {
        (EventKind::Removed, EventKind::Created)
    };
    let drain = drain_until(&mut observation, |drain| {
        has(drain, &tree("sub/f0.txt"), from) && has(drain, &tree("moved.txt"), to)
    });
    assert!(has(&drain, &tree("sub/f0.txt"), from), "{:?}", summary(&drain));
    assert!(has(&drain, &tree("moved.txt"), to), "{:?}", summary(&drain));

    fs::rename(sub.join("f2.txt"), sub.join("renamed.txt")).unwrap();
    let drain = drain_until(&mut observation, |drain| {
        has(drain, &tree("sub/f2.txt"), EventKind::MovedFrom) && has(drain, &tree("sub/renamed.txt"), EventKind::MovedTo)
    });
    assert!(has(&drain, &tree("sub/f2.txt"), EventKind::MovedFrom), "{:?}", summary(&drain));
    assert!(has(&drain, &tree("sub/renamed.txt"), EventKind::MovedTo), "{:?}", summary(&drain));

    fs::create_dir(sub.join("newdir")).unwrap();
    let drain = drain_until(&mut observation, |drain| drain.events.iter().any(|event| event.scope == tree("sub/newdir")));
    let created = drain.events.iter().find(|event| event.scope == tree("sub/newdir")).expect("directory creation");
    assert_eq!((created.kind, created.directory), (EventKind::Created, Some(true)));

    fs::remove_file(sub.join("f1.txt")).unwrap();
    let drain = drain_until(&mut observation, |drain| has(drain, &tree("sub/f1.txt"), EventKind::Removed));
    assert!(has(&drain, &tree("sub/f1.txt"), EventKind::Removed), "{:?}", summary(&drain));
    assert_eq!(drain.lost, None);
}

#[test]
fn native_coverage_follows_the_move_and_sees_recreation() {
    let layout = Layout::new();
    let Some(mut observation) = native(&layout, ObserverOptions::default()) else { return };
    fs::write(layout.checkout.join("sub").join("before.txt"), b"x").unwrap();
    let mut kept = Drain { events: Vec::new(), lost: None };
    settle(&mut observation, &mut kept);

    rename_no_replace(&layout.checkout, &layout.quarantine).unwrap();
    observation.moved_to_quarantine();
    fs::write(layout.quarantine.join("sub").join("after.txt"), b"x").unwrap();
    fs::create_dir(&layout.checkout).unwrap();
    fs::write(layout.checkout.join("recreated.txt"), b"x").unwrap();

    let drain = then(kept, observation.drain());
    let events = summary(&drain);
    assert_eq!(drain.lost, None, "{events:?}");
    assert!(has(&drain, &tree("sub/before.txt"), EventKind::Created), "{events:?}");
    assert!(has(&drain, &Scope::Original(PathBuf::new()), EventKind::MovedFrom), "{events:?}");
    assert!(has(&drain, &Scope::Quarantine(PathBuf::new()), EventKind::MovedTo), "{events:?}");
    assert!(has(&drain, &tree("sub/after.txt"), EventKind::Created), "the tree is still covered: {events:?}");
    assert!(has(&drain, &Scope::Original(PathBuf::new()), EventKind::Created), "recreation: {events:?}");
    assert_eq!(observation.tree_location(), layout.quarantine);
}

#[test]
fn native_siblings_are_ignored_and_admin_files_reported() {
    let layout = Layout::new();
    let Some(mut observation) = native(&layout, ObserverOptions::default()) else { return };
    fs::write(layout.parent.join("sibling").join("busy.txt"), b"x").unwrap();
    fs::write(layout.parent.join("other.txt"), b"x").unwrap();
    fs::write(layout.admin.join("ORIG_HEAD"), b"x").unwrap();
    fs::create_dir(layout.admin.join("logs")).unwrap();

    fs::write(layout.admin.join("index.lock"), b"x").unwrap();
    fs::rename(layout.admin.join("index.lock"), layout.admin.join("index")).unwrap();
    fs::write(layout.admin.join("sharedindex.0123"), b"x").unwrap();
    fs::write(layout.admin.join("HEAD"), b"y").unwrap();
    fs::write(layout.admin.join("gitdir"), b"y").unwrap();
    fs::write(layout.admin.join("locked"), b"").unwrap();

    let drain = observation.drain();
    let events = summary(&drain);
    let scopes: Vec<&Scope> = drain.events.iter().map(|event| &event.scope).collect();
    assert!(scopes.iter().all(|scope| matches!(scope, Scope::Admin(_))), "{events:?}");
    // ReFS reports a rename onto an existing file as the source's removal and
    // the target's modification.
    let replaced = |name: &str, kind: EventKind, refs_kind: EventKind| {
        has(&drain, &Scope::Admin(name.into()), kind) || (cfg!(windows) && has(&drain, &Scope::Admin(name.into()), refs_kind))
    };
    assert!(replaced("index.lock", EventKind::MovedFrom, EventKind::Removed), "{events:?}");
    assert!(replaced("index", EventKind::MovedTo, EventKind::Modified), "{events:?}");
    for (name, kind) in [
        ("index.lock", EventKind::Created),
        ("sharedindex.0123", EventKind::Created),
        ("HEAD", EventKind::Modified),
        ("gitdir", EventKind::Modified),
        ("locked", EventKind::Created),
    ] {
        assert!(has(&drain, &Scope::Admin(name.into()), kind), "{name} {kind:?}: {events:?}");
    }
    assert_eq!(drain.lost, None);
}

#[test]
fn native_intentional_deletion_is_not_coverage_loss() {
    let layout = Layout::new();
    let Some(mut observation) = native(&layout, ObserverOptions::default()) else { return };
    rename_no_replace(&layout.checkout, &layout.quarantine).unwrap();
    observation.moved_to_quarantine();

    let mut kept = Drain { events: Vec::new(), lost: None };
    fs::remove_file(layout.quarantine.join("sub").join("file.txt")).unwrap();
    settle(&mut observation, &mut kept);
    fs::remove_dir(layout.quarantine.join("sub")).unwrap();
    settle(&mut observation, &mut kept);
    fs::remove_dir(&layout.quarantine).unwrap();
    let drain = then(kept, observation.drain());
    assert_eq!(drain.lost, None, "{:?}", summary(&drain));
    assert!(has(&drain, &tree("sub/file.txt"), EventKind::Removed));
    assert!(has(&drain, &tree("sub"), EventKind::Removed));
    assert!(has(&drain, &Scope::Quarantine(PathBuf::new()), EventKind::Removed));

    observation.release_admin();
    fs::remove_dir_all(&layout.admin).unwrap();
    let drain = observation.drain();
    assert_eq!(drain.lost, None, "{:?}", summary(&drain));
    assert!(drain.events.is_empty(), "a released admin directory reports nothing: {:?}", summary(&drain));

    // The parent watch still works.
    fs::create_dir(&layout.checkout).unwrap();
    assert!(has(&observation.drain(), &Scope::Original(PathBuf::new()), EventKind::Created));
}

#[test]
fn native_a_change_below_a_directory_is_reported_or_the_directory_event_stands_for_it() {
    let layout = Layout::new();
    let Some(mut observation) = native(&layout, ObserverOptions::default()) else { return };
    // No drain between the change and the directory operation: on macOS the
    // child's event may be coalesced away, but never without the
    // directory's own.
    fs::write(layout.checkout.join("sub").join("new.txt"), b"x").unwrap();
    rename_no_replace(&layout.checkout, &layout.quarantine).unwrap();
    observation.moved_to_quarantine();
    let drain = observation.drain();
    let events = summary(&drain);
    assert!(has(&drain, &Scope::Original(PathBuf::new()), EventKind::MovedFrom), "{events:?}");
    assert!(COALESCES_BELOW_DIRECTORY || has(&drain, &tree("sub/new.txt"), EventKind::Created), "{events:?}");

    fs::write(layout.quarantine.join("sub").join("file.txt"), b"changed").unwrap();
    fs::remove_file(layout.quarantine.join("sub").join("file.txt")).unwrap();
    fs::remove_file(layout.quarantine.join("sub").join("new.txt")).unwrap();
    fs::remove_dir(layout.quarantine.join("sub")).unwrap();
    let drain = observation.drain();
    let events = summary(&drain);
    assert!(has(&drain, &tree("sub"), EventKind::Removed), "{events:?}");
    assert!(COALESCES_BELOW_DIRECTORY || has(&drain, &tree("sub/file.txt"), EventKind::Modified), "{events:?}");
    assert_eq!(drain.lost, None, "{events:?}");
}

#[test]
fn native_losing_a_watched_directory_is_coverage_loss() {
    let layout = Layout::new();
    let Some(mut observation) = native(&layout, ObserverOptions::default()) else { return };
    fs::remove_dir_all(&layout.admin).unwrap();
    let drain = observation.drain();
    assert_eq!(drain.lost, Some(CoverageLost::WatchLost { root: Root::Admin }), "{:?}", summary(&drain));
    fs::write(layout.checkout.join("later.txt"), b"x").unwrap();
    assert_eq!(observation.drain().lost, Some(CoverageLost::WatchLost { root: Root::Admin }), "loss is sticky");

    let layout = Layout::new();
    let Some(mut observation) = native(&layout, ObserverOptions::default()) else { return };
    fs::rename(&layout.parent, layout.parent.with_file_name("parent-moved")).unwrap();
    assert_eq!(observation.drain().lost, Some(CoverageLost::WatchLost { root: Root::Parent }));
}

#[test]
fn native_a_full_queue_is_coverage_loss() {
    let layout = Layout::new();
    let Some(mut observation) = native(&layout, ObserverOptions { queue_capacity: 4 }) else { return };
    for index in 0..10 {
        fs::write(layout.checkout.join(format!("f{index}")), b"x").unwrap();
    }
    let drain = observation.drain();
    assert_eq!(drain.events.len(), 4);
    assert_eq!(drain.lost, Some(CoverageLost::QueueFull { capacity: 4 }));
}

#[test]
fn native_setup_failure_is_an_error_before_anything_changes() {
    let layout = Layout::new();
    if native(&layout, ObserverOptions::default()).is_none() {
        return;
    }

    let missing_admin = Targets::new(&layout.checkout, &layout.quarantine, &layout.parent.join("no-admin")).unwrap();
    let error = Observation::start(missing_admin, ObserverOptions::default()).unwrap_err();
    assert!(matches!(&error, ObservationError::Setup { path, .. } if path.ends_with("no-admin")), "{error:?}");

    let moved_checkout = layout.parent.join("real-checkout");
    fs::rename(&layout.checkout, &moved_checkout).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&moved_checkout, &layout.checkout).unwrap();
    let error = Observation::start(layout.targets(), ObserverOptions::default()).unwrap_err();
    assert!(matches!(&error, ObservationError::Setup { path, .. } if path == &layout.checkout), "a link is not watched through: {error:?}");
    assert!(moved_checkout.join("sub").join("file.txt").is_file(), "nothing changed");
}

#[test]
fn unconfirmed_delivery_says_confirmation_failed_not_that_events_dropped() {
    let waited = std::time::Duration::from_secs(30);
    let stalled = CoverageLost::DeliveryStalled { waited, watch: Some(Root::Admin) }.to_string();
    assert_eq!(stalled, "could not confirm delivery of earlier changes on the Admin watch: no progress for 30s");
    let unnamed = CoverageLost::DeliveryStalled { waited, watch: None }.to_string();
    assert_eq!(unnamed, "could not confirm delivery of earlier changes: no progress for 30s");
    let ceiling = CoverageLost::DrainCeiling { waited: std::time::Duration::from_secs(300), watch: Some(Root::Parent) }.to_string();
    assert_eq!(ceiling, "could not confirm delivery of earlier changes on the Parent watch within 300s");
    for text in [stalled, unnamed, ceiling] {
        assert!(!text.contains("dropped"), "{text}");
    }
}
