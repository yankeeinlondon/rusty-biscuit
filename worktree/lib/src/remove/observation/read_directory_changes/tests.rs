//! Windows-only facts of the `ReadDirectoryChangesW` adapter that the shared
//! contract tests cannot provoke on demand: a system buffer overflow (which
//! `notify` hid in S2), completion codes, the notification buffer format,
//! labels, the delivery thread's end, the admin handle's release, what
//! Sniff sees of the session, and the wait policy against held handles.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::remove::observation::{Drain, Observation, Scope};
use crate::remove::usage::{OwnedHandles, SniffUsage, check_usage};

struct Fixture {
    _root: tempfile::TempDir,
    parent: PathBuf,
    checkout: PathBuf,
    quarantine: PathBuf,
    admin: PathBuf,
    targets: Targets,
}

fn new_fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("parent");
    let checkout = parent.join("checkout");
    fs::create_dir_all(checkout.join("sub")).unwrap();
    let admin = root.path().join("admin");
    fs::create_dir(&admin).unwrap();
    let quarantine = parent.join(".wt-remove-checkout-0123456789abcdef");
    let targets = Targets::new(&checkout, &quarantine, &admin).unwrap();
    Fixture { _root: root, parent, checkout, quarantine, admin, targets }
}

fn start(fixture: &Fixture) -> ReadDirectoryChanges {
    ReadDirectoryChanges::start(&fixture.targets, ObserverOptions::default(), false, &|| false).unwrap()
}

fn drain(adapter: &mut ReadDirectoryChanges, targets: &Targets) -> Drain {
    let mut sink = Sink { targets, capacity: usize::MAX, events: Vec::new(), lost: None };
    adapter.drain(&mut sink, &|| false);
    Drain { events: sink.events, lost: sink.lost, interrupted: false }
}

fn summary(drain: &Drain) -> Vec<(Scope, EventKind, Option<bool>)> {
    drain.events.iter().map(|event| (event.scope.clone(), event.kind, event.directory)).collect()
}

/// One notification entry as the system lays it out in `format`.
fn entry(format: Format, action: u32, attributes: u32, name: &str, last: bool) -> Vec<u8> {
    let (attributes_at, length_at, name_at) = format.offsets();
    let units: Vec<u16> = name.encode_utf16().collect();
    let length = units.len() * 2;
    let size = (name_at + length).next_multiple_of(8);
    let mut bytes = vec![0; size];
    let mut put = |at: usize, value: u32| bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    put(0, if last { 0 } else { size as u32 });
    put(4, action);
    if let Some(at) = attributes_at {
        put(at, attributes);
    }
    put(length_at, length as u32);
    for (index, unit) in units.iter().enumerate() {
        bytes[name_at + 2 * index..name_at + 2 * index + 2].copy_from_slice(&unit.to_le_bytes());
    }
    bytes
}

#[test]
fn notification_buffers_are_parsed_entry_by_entry() {
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE;
    for (format, carried) in [(Format::Extended, true), (Format::Plain, false)] {
        let attributes = |value: u32| carried.then_some(value);
        let mut buffer = entry(format, FILE_ACTION_ADDED, FILE_ATTRIBUTE_ARCHIVE, r"checkout\sub\ü.txt", false);
        buffer.extend(entry(format, FILE_ACTION_RENAMED_OLD_NAME, FILE_ATTRIBUTE_DIRECTORY, "a", false));
        buffer.extend(entry(format, FILE_ACTION_RENAMED_NEW_NAME, FILE_ATTRIBUTE_DIRECTORY, "b", true));
        assert_eq!(
            parse(&buffer, format).unwrap(),
            vec![
                (FILE_ACTION_ADDED, attributes(FILE_ATTRIBUTE_ARCHIVE), OsString::from(r"checkout\sub\ü.txt")),
                (FILE_ACTION_RENAMED_OLD_NAME, attributes(FILE_ATTRIBUTE_DIRECTORY), OsString::from("a")),
                (FILE_ACTION_RENAMED_NEW_NAME, attributes(FILE_ATTRIBUTE_DIRECTORY), OsString::from("b")),
            ],
            "{format:?}"
        );

        // An entry pointing past the bytes read, a name running past them, and
        // a truncated header are errors, not a shorter list.
        let (_, length_at, name_at) = format.offsets();
        let mut past = entry(format, FILE_ACTION_ADDED, 0, "a", false);
        past[0..4].copy_from_slice(&256_u32.to_le_bytes());
        assert!(parse(&past, format).is_err(), "{format:?}");
        let mut long_name = entry(format, FILE_ACTION_ADDED, 0, "a", true);
        long_name[length_at..length_at + 4].copy_from_slice(&200_u32.to_le_bytes());
        assert!(parse(&long_name, format).is_err(), "{format:?}");
        assert!(parse(&vec![0; name_at - 4], format).is_err(), "{format:?}");
    }
}

#[test]
fn completions_map_to_events_overflow_or_an_ended_watch() {
    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_NETNAME_DELETED, ERROR_OPERATION_ABORTED};
    assert_eq!(completion(Ok(24)), Completion::Notifications(24));
    // A zero-byte completion is the system's overflow signal.
    assert_eq!(completion(Ok(0)), Completion::Overflow);
    assert_eq!(completion(Err(ERROR_NOTIFY_ENUM_DIR)), Completion::Overflow);
    // The session never reads its own cancel as a completion, so an abort
    // is someone else's doing.
    for code in [ERROR_ACCESS_DENIED, ERROR_NETNAME_DELETED, ERROR_OPERATION_ABORTED, ERROR_INVALID_PARAMETER] {
        assert_eq!(completion(Err(code)), Completion::Ended, "{code}");
    }
}

#[test]
fn actions_map_to_kinds_and_modified_is_never_metadata() {
    use windows_sys::Win32::Storage::FileSystem::FILE_ACTION_MODIFIED;
    assert_eq!(kind(FILE_ACTION_ADDED), EventKind::Created);
    assert_eq!(kind(FILE_ACTION_REMOVED), EventKind::Removed);
    assert_eq!(kind(FILE_ACTION_MODIFIED), EventKind::Modified);
    assert_eq!(kind(FILE_ACTION_RENAMED_OLD_NAME), EventKind::MovedFrom);
    assert_eq!(kind(FILE_ACTION_RENAMED_NEW_NAME), EventKind::MovedTo);
    assert_eq!(kind(99), EventKind::Modified, "an unknown action is still a change");
}

#[test]
fn paths_are_labeled_by_where_the_tree_is() {
    let fixture = new_fixture();
    let adapter = start(&fixture);
    let shared = &adapter.shared;
    let at = |path: &str| shared.locate(Root::Parent, Path::new(path));

    assert_eq!(at(r"checkout\sub\x"), Some((Root::Tree, PathBuf::from(r"sub\x"))));
    assert_eq!(at("checkout"), Some((Root::Parent, PathBuf::from("checkout"))));
    assert_eq!(at(r"CHECKOUT\x"), Some((Root::Tree, PathBuf::from("x"))), "names match in any case");
    assert_eq!(at(".wt-remove-checkout-0123456789abcdef"), Some((Root::Parent, PathBuf::from(".wt-remove-checkout-0123456789abcdef"))));
    assert_eq!(at(r"sibling\x"), None);
    assert_eq!(at(r"checkout2\x"), None);
    assert_eq!(shared.locate(Root::Admin, Path::new("index")), Some((Root::Admin, PathBuf::from("index"))));

    shared.moved.store(true, Ordering::SeqCst);
    assert_eq!(at(r".wt-remove-checkout-0123456789abcdef\sub\x"), Some((Root::Tree, PathBuf::from(r"sub\x"))));
    assert_eq!(at(r"Checkout\recreated.txt"), Some((Root::Parent, PathBuf::from(r"checkout\recreated.txt"))), "reported under the session's name");
}

#[test]
fn markers_are_never_reported_and_leave_nothing_behind() {
    let fixture = new_fixture();
    let mut adapter = start(&fixture);
    fs::write(fixture.checkout.join("a.txt"), b"x").unwrap();
    let drain = drain(&mut adapter, &fixture.targets);
    assert_eq!(drain.lost, None);
    assert!(drain.events.iter().any(|event| event.scope == Scope::Tree("a.txt".into())), "{:?}", summary(&drain));
    let leftovers = |directory: &Path| {
        fs::read_dir(directory).unwrap().map(|entry| entry.unwrap().file_name()).filter(|name| name.to_string_lossy().starts_with(".wt-observe-")).count()
    };
    assert_eq!((leftovers(&fixture.parent), leftovers(&fixture.admin)), (0, 0));
}

/// The case S2 showed `notify` hiding: a burst while the thread is not
/// reading overflows the system's buffer, and that is `Overflow`, not a
/// shorter list of events.
#[test]
fn a_system_buffer_overflow_is_coverage_loss() {
    let fixture = new_fixture();
    let mut adapter = ReadDirectoryChanges::start_with_buffer(&fixture.targets, ObserverOptions::default(), 4096, false, &|| false, false).unwrap();
    let gate = adapter.shared.gate.lock().unwrap();
    for index in 0..500 {
        fs::write(fixture.checkout.join(format!("burst-file-{index:04}.txt")), b"x").unwrap();
    }
    drop(gate);
    let drain = drain(&mut adapter, &fixture.targets);
    assert_eq!(drain.lost, Some(CoverageLost::Overflow), "{} events", drain.events.len());
}

/// With the production buffer and the thread reading, a burst arrives
/// complete or is reported lost; never silently partial. 1,000 files take
/// about 1 s to write here; the overflow itself is provoked above.
#[test]
fn a_burst_is_delivered_completely_or_reported_as_lost() {
    let fixture = new_fixture();
    let mut adapter = start(&fixture);
    let count = 1_000;
    for index in 0..count {
        fs::write(fixture.checkout.join(format!("f{index}")), b"x").unwrap();
    }
    let drain = drain(&mut adapter, &fixture.targets);
    let created = drain.events.iter().filter(|event| event.kind == EventKind::Created && matches!(&event.scope, Scope::Tree(_))).count();
    assert!(created == count || drain.lost.is_some(), "{created} of {count} without a loss");
}

#[test]
fn the_delivery_thread_ends_with_the_session() {
    let fixture = new_fixture();
    let adapter = start(&fixture);
    let alive = Arc::downgrade(&adapter.thread_alive);
    drop(adapter);
    assert!(alive.upgrade().is_none(), "the thread was joined");
}

/// Answers the plan's question: does an open watch handle leave a deleted
/// directory delete-pending? The answer is recorded in the `os` skill; the
/// release closes the handle first either way.
#[test]
fn a_watched_directory_deleted_before_release_and_one_released_first() {
    let fixture = new_fixture();
    let mut adapter = start(&fixture);
    fs::remove_dir(&fixture.admin).expect("a watched directory can be deleted");
    let gone = fs::symlink_metadata(&fixture.admin).is_err();
    let recreated = fs::create_dir(&fixture.admin).is_ok();
    eprintln!("watched directory deleted: name gone at once = {gone}, recreated while watched = {recreated}");
    assert_eq!(drain(&mut adapter, &fixture.targets).lost, Some(CoverageLost::WatchLost { root: Root::Admin }));

    let fixture = new_fixture();
    let mut adapter = start(&fixture);
    adapter.release_admin(&|| false);
    fs::remove_dir(&fixture.admin).unwrap();
    assert!(fs::symlink_metadata(&fixture.admin).is_err(), "no handle keeps a released directory's name");
    fs::create_dir(&fixture.admin).expect("the name is free again");
    let drain = drain(&mut adapter, &fixture.targets);
    assert_eq!(drain.lost, None, "{:?}", summary(&drain));
}

/// The type of an entry comes from the notification: a directory and a file
/// removed before their creation is read are still reported as a directory
/// and a file. The adapter used to open each entry to learn its type, and that
/// handle, inside the tree, made a move made at the same moment fail with
/// `ERROR_ACCESS_DENIED` (3 in 300 moves under a parallel test run).
#[test]
fn an_entry_type_is_known_without_opening_the_entry() {
    let fixture = new_fixture();
    let mut adapter = start(&fixture);
    let gate = adapter.shared.gate.lock().unwrap();
    fs::create_dir(fixture.checkout.join("sub").join("d")).unwrap();
    fs::write(fixture.checkout.join("sub").join("f"), b"x").unwrap();
    fs::remove_dir(fixture.checkout.join("sub").join("d")).unwrap();
    fs::remove_file(fixture.checkout.join("sub").join("f")).unwrap();
    drop(gate);
    let drain = drain(&mut adapter, &fixture.targets);
    let created = |path: &str| {
        drain.events.iter().find(|event| event.scope == Scope::Tree(path.into()) && event.kind == EventKind::Created).map(|event| event.directory)
    };
    assert_eq!((created(r"sub\d"), created(r"sub\f")), (Some(Some(true)), Some(Some(false))), "{:?}", summary(&drain));
    assert_eq!(drain.lost, None);
}

#[test]
fn the_watch_does_not_block_the_move_or_the_deletion() {
    let fixture = new_fixture();
    let mut adapter = start(&fixture);
    fs::write(fixture.checkout.join("sub").join("x"), b"x").unwrap();
    crate::remove::quarantine::rename::rename_no_replace(&fixture.checkout, &fixture.quarantine).unwrap();
    adapter.moved_to_quarantine(&|| false);
    fs::remove_dir_all(&fixture.quarantine).unwrap();
    assert!(fs::symlink_metadata(&fixture.quarantine).is_err());
    let drain = drain(&mut adapter, &fixture.targets);
    assert_eq!(drain.lost, None, "{:?}", summary(&drain));
    assert!(drain.events.iter().any(|event| event.scope == Scope::Quarantine(PathBuf::new()) && event.kind == EventKind::Removed), "{:?}", summary(&drain));
}

#[test]
fn sniff_sees_nothing_of_the_session() {
    let fixture = new_fixture();
    let mut observation = Observation::start(fixture.targets.clone(), ObserverOptions::default(), &|| false).unwrap();
    fs::write(fixture.checkout.join("sub").join("x"), b"x").unwrap();
    let _ = observation.drain(&|| false);
    let mut own = OwnedHandles::default();
    observation.register_handles(&mut own);
    let clearance = check_usage(&SniffUsage, &fixture.checkout, &own, None).expect("the parent and admin handles are outside the checkout");
    assert_eq!(clearance.own_handles_excluded, 0);
}

#[test]
fn a_link_or_a_missing_directory_is_a_setup_failure() {
    let fixture = new_fixture();
    let missing = Targets::new(&fixture.checkout, &fixture.quarantine, &fixture.parent.join("no-admin")).unwrap();
    let error = ReadDirectoryChanges::start(&missing, ObserverOptions::default(), false, &|| false).err().expect("no admin directory");
    assert!(matches!(&error, ObservationError::Setup { path, .. } if path.ends_with("no-admin")), "{error:?}");

    let file = fixture.parent.join("file");
    fs::write(&file, b"x").unwrap();
    let not_a_directory = Targets::new(&file, &fixture.quarantine, &fixture.admin).unwrap();
    let error = ReadDirectoryChanges::start(&not_a_directory, ObserverOptions::default(), false, &|| false).err().expect("a file");
    assert!(matches!(&error, ObservationError::Setup { path, .. } if path == &file), "{error:?}");

    let link = fixture.parent.join("link");
    match std::os::windows::fs::symlink_dir(&fixture.checkout, &link) {
        Ok(()) => {
            let through_link = Targets::new(&link, &fixture.quarantine, &fixture.admin).unwrap();
            let error = ReadDirectoryChanges::start(&through_link, ObserverOptions::default(), false, &|| false).err().expect("a link");
            assert!(matches!(&error, ObservationError::Setup { path, .. } if path == &link), "{error:?}");
        }
        Err(error) => eprintln!("creating a directory link failed ({error}); the link case was not exercised"),
    }
}

/// The file system's name for the volume holding `path` ("NTFS", "ReFS").
fn file_system(path: &Path) -> String {
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
    let root = path.ancestors().last().unwrap().join("");
    let root = wide(&std::path::absolute(&root).unwrap());
    let mut name = [0_u16; 64];
    // SAFETY: `root` is NUL-terminated and `name` holds `name.len()` units.
    let read = unsafe {
        GetVolumeInformationW(root.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), name.as_mut_ptr(), name.len() as u32)
    };
    assert_ne!(read, 0, "{}", io::Error::last_os_error());
    String::from_utf16_lossy(&name[..name.iter().position(|unit| *unit == 0).unwrap()])
}

/// Measured on a ReFS Dev Drive (`build-win-native`'s `B:`), where NTFS
/// reports every case: a directory deleted by `std::fs::remove_dir_all`
/// (POSIX delete semantics) raises no notification, empty or not, while a
/// file deleted the same way and a directory deleted by `RemoveDirectoryW`
/// do. The barrier still holds. The deletion engine's per-entry recheck, not
/// a notification, is what finds such a directory gone. Runs where the
/// package's own directory is on ReFS.
#[test]
fn on_refs_a_directory_removed_with_posix_semantics_is_not_reported() {
    let system = file_system(&crate::test_support::package_dir());
    if system != "ReFS" {
        eprintln!("the package is on {system}, not ReFS; the ReFS case was not exercised");
        return;
    }
    let root = tempfile::Builder::new().prefix(".wt-refs-").tempdir_in(crate::test_support::package_dir()).unwrap();
    let checkout = root.path().join("parent").join("checkout");
    for name in ["plain", "posix-empty", "posix-full"] {
        fs::create_dir_all(checkout.join(name)).unwrap();
    }
    fs::write(checkout.join("posix-full").join("f"), b"x").unwrap();
    let admin = root.path().join("admin");
    fs::create_dir(&admin).unwrap();
    let targets = Targets::new(&checkout, &checkout.with_file_name(".q"), &admin).unwrap();
    let mut adapter = ReadDirectoryChanges::start(&targets, ObserverOptions::default(), false, &|| false).unwrap();
    let removed = |drain: &Drain, path: &str| drain.events.iter().any(|event| event.scope == Scope::Tree(path.into()) && event.kind == EventKind::Removed);

    fs::remove_dir(checkout.join("plain")).unwrap();
    assert!(removed(&drain(&mut adapter, &targets), "plain"));

    fs::remove_dir_all(checkout.join("posix-empty")).unwrap();
    let empty = drain(&mut adapter, &targets);
    assert!(empty.events.is_empty(), "{:?}", summary(&empty));

    fs::remove_dir_all(checkout.join("posix-full")).unwrap();
    let full = drain(&mut adapter, &targets);
    assert!(removed(&full, r"posix-full\f") && !removed(&full, "posix-full"), "{:?}", summary(&full));
    assert_eq!(full.lost, None);
}

// ---- the wait policy against real handles; `held` keeps the delivery thread
// from handling completions, as a stalled watch would ----

fn take_after(adapter: &mut ReadDirectoryChanges, targets: &Targets, interrupted: &dyn Fn() -> bool) -> (Waited, Drain) {
    let mut sink = Sink { targets, capacity: usize::MAX, events: Vec::new(), lost: None };
    let waited = adapter.drain(&mut sink, interrupted);
    (waited, Drain { events: sink.events, lost: sink.lost, interrupted: waited == Waited::Interrupted })
}

/// No delivery on a handle for the stall limit is `DeliveryStalled` naming
/// that handle's watch.
#[test]
fn a_held_watch_stalls_the_drain_and_names_the_watch() {
    let fixture = new_fixture();
    let mut adapter = start(&fixture);
    let stall = std::time::Duration::from_millis(300);
    adapter.policy = WaitPolicy { stall, ceiling: std::time::Duration::from_secs(60) };
    adapter.shared.held.store(true, Ordering::SeqCst);

    let (waited, drain) = take_after(&mut adapter, &fixture.targets, &|| false);

    assert_eq!(waited, Waited::Done);
    match drain.lost {
        Some(CoverageLost::DeliveryStalled { waited, watch: Some(Root::Parent | Root::Admin) }) => assert!(waited >= stall, "{waited:?}"),
        other => panic!("expected a stall on a named watch, got {other:?}"),
    }
}

/// The interrupt ends a wait at once and records no loss; an interrupted
/// release keeps the admin handle open, and a later one closes it.
#[test]
fn an_interrupt_ends_a_held_wait_without_loss() {
    let fixture = new_fixture();
    let mut adapter = start(&fixture);
    adapter.shared.held.store(true, Ordering::SeqCst);
    let started = Instant::now();
    let soon = || started.elapsed() >= std::time::Duration::from_millis(100);

    let (waited, drain) = take_after(&mut adapter, &fixture.targets, &soon);

    assert_eq!((waited, drain.lost), (Waited::Interrupted, None));
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}: far below the 30 s stall", started.elapsed());
    assert_eq!(adapter.release_admin(&|| true), Waited::Interrupted);
    assert!(!adapter.admin_released, "an interrupted release keeps the handle");

    adapter.shared.held.store(false, Ordering::SeqCst);
    assert_eq!(adapter.release_admin(&|| false), Waited::Done);
    assert!(adapter.admin_released);
}

/// Startup waits under the same policy: a watch that never delivers its first
/// marker fails with startup diagnostics naming the stalled watch, and an
/// interrupt during that wait is `Interrupted`, not an environment limit.
#[test]
fn startup_waits_under_the_policy_and_stops_for_an_interrupt() {
    let fixture = new_fixture();
    let policy = WaitPolicy { stall: std::time::Duration::from_millis(300), ceiling: std::time::Duration::from_secs(60) };
    let options = ObserverOptions { wait: policy, ..ObserverOptions::default() };
    match ReadDirectoryChanges::start_with_buffer(&fixture.targets, options, LARGE_BUFFER_BYTES, false, &|| false, true) {
        Err(error @ ObservationError::NotDelivering(_)) => {
            assert!(error.to_string().contains("could not confirm delivery of earlier changes on the"), "{error}");
            let diagnostics = error.diagnostics().expect("carried");
            assert_eq!(diagnostics.backend, ObservationBackend::ReadDirectoryChanges);
            let loss = diagnostics.loss.as_ref().expect("the stall");
            assert_eq!(loss.stage, crate::remove::observation::ObservationStage::Startup);
            assert!(matches!(loss.reason, CoverageLost::DeliveryStalled { watch: Some(_), .. }), "{:?}", loss.reason);
        }
        Err(other) => panic!("expected a startup loss, got {other}"),
        Ok(_) => panic!("started without a delivered marker"),
    }
    let started = Instant::now();
    match ReadDirectoryChanges::start_with_buffer(&fixture.targets, ObserverOptions::default(), LARGE_BUFFER_BYTES, false, &|| true, true) {
        Err(ObservationError::Interrupted) => {}
        Err(other) => panic!("expected an interrupt, got {other}"),
        Ok(_) => panic!("started without a delivered marker"),
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
    let leftovers = |directory: &Path| {
        fs::read_dir(directory).unwrap().map(|entry| entry.unwrap().file_name()).filter(|name| name.to_string_lossy().starts_with(".wt-observe-")).count()
    };
    assert_eq!((leftovers(&fixture.parent), leftovers(&fixture.admin)), (0, 0), "no marker is left behind");
}
