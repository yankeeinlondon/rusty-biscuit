//! Strategy A interruption matrix: in-process crash (error return) and real
//! process abort (no destructors) at every protocol step.

mod common;

use std::path::Path;
use std::process::Command;

use common::{Seen, classify, leftovers, new, old};
use publication_spike::SpikeError;
use publication_spike::fault::{Faults, Point};
use publication_spike::journal::{Options, Recovery, pending, publish, recover};
use publication_spike::model::{MANIFEST_PATH, read_verified};
use publication_spike::resolve;

const ENTRIES: usize = 10;

fn points() -> Vec<Point> {
    let mut points = vec![Point::BeforeStaging, Point::DuringStaging, Point::StagedNoJournal, Point::BeforeSelection];
    points.extend((1..=ENTRIES).map(Point::MidReplacement));
    points.extend([Point::BeforeManifest, Point::AfterSelection, Point::BeforeCleanup]);
    points
}

/// (pre-recovery read, recovery outcome, post-recovery read)
fn expected(point: Point) -> (fn(&Seen) -> bool, Recovery, Seen) {
    fn old(s: &Seen) -> bool { *s == Seen::Old }
    fn newer(s: &Seen) -> bool { *s == Seen::New }
    fn refused(s: &Seen) -> bool { matches!(s, Seen::Refused(_)) }
    match point {
        Point::BeforeStaging | Point::DuringStaging | Point::StagedNoJournal => (old, Recovery::Clean, Seen::Old),
        Point::BeforeSelection => (old, Recovery::RolledBack, Seen::Old),
        Point::MidReplacement(_) | Point::BeforeManifest => (refused, Recovery::RolledBack, Seen::Old),
        Point::AfterSelection => (newer, Recovery::RolledForward, Seen::New),
        Point::BeforeCleanup => (newer, Recovery::Clean, Seen::New),
    }
}

fn published_old() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    publish(dir.path(), &old(), Options::default()).unwrap();
    assert_eq!(classify(read_verified(dir.path())), Seen::Old);
    dir
}

fn check_after_crash(root: &Path, point: Point) -> String {
    let (pre_ok, want_recovery, want_post) = expected(point);
    let pre = classify(read_verified(root));
    assert!(pre_ok(&pre), "{point:?}: pre-recovery read {pre:?}");
    assert!(!matches!(pre, Seen::Other(_)));
    // A publisher must refuse while a journal is pending.
    if pending(root) {
        assert!(matches!(publish(root, &new(), Options::default()), Err(SpikeError::RecoveryRequired)));
    }
    let outcome = recover(root, Options::default()).unwrap();
    assert_eq!(outcome, want_recovery, "{point:?}");
    let post = classify(read_verified(root));
    assert_eq!(post, want_post, "{point:?}");
    assert!(leftovers(root).is_empty(), "{point:?}: leftovers {:?}", leftovers(root));
    let review = resolve(root, "messenger/docs/research/reviews/new.json");
    assert_eq!(review.exists(), post == Seen::New, "{point:?}: review artifact presence");
    // The protocol stays usable: a follow-up publication succeeds.
    publish(root, &new(), Options::default()).unwrap();
    assert_eq!(classify(read_verified(root)), Seen::New);
    let pre_label = match pre { Seen::Refused(_) => "Refused".to_string(), other => format!("{other:?}") };
    format!("{:<18} pre={pre_label:<8} recovery={outcome:?} post={post:?}", point.label())
}

#[test]
fn in_process_crash_at_every_point() {
    let mut rows = Vec::new();
    for point in points() {
        let dir = published_old();
        let err = publish(dir.path(), &new(), Options { faults: Faults::error_at(point), ..Default::default() });
        assert!(matches!(err, Err(SpikeError::Crash(p)) if p == point), "{point:?}: {err:?}");
        rows.push(check_after_crash(dir.path(), point));
    }
    eprintln!("EVIDENCE in-process\n{}", rows.join("\n"));
}

/// Child half of `process_abort_at_every_point`; inert unless the env is set.
#[test]
fn child_entry() {
    let (Ok(root), Ok(point)) = (std::env::var("PUB_SPIKE_ROOT"), std::env::var("PUB_SPIKE_POINT")) else {
        return;
    };
    let point = Point::parse(&point).expect("point");
    let _ = publish(Path::new(&root), &new(), Options { faults: Faults::abort_at(point), ..Default::default() });
    panic!("fault {point:?} did not abort");
}

#[test]
fn process_abort_at_every_point() {
    let exe = std::env::current_exe().unwrap();
    let mut rows = Vec::new();
    for point in points() {
        let dir = published_old();
        let status = Command::new(&exe)
            .args(["child_entry", "--exact", "--nocapture", "--test-threads=1"])
            .env("PUB_SPIKE_ROOT", dir.path())
            .env("PUB_SPIKE_POINT", point.label())
            .output()
            .unwrap()
            .status;
        assert!(!status.success(), "{point:?}: child should have aborted");
        // The OS released the aborted child's lock; recovery can proceed.
        rows.push(format!("{}  child_status={status}", check_after_crash(dir.path(), point)));
    }
    eprintln!("EVIDENCE process-abort\n{}", rows.join("\n"));
}

#[test]
fn crash_during_rollback_is_recovered_by_rerunning() {
    let dir = published_old();
    let root = dir.path();
    let _ = publish(root, &new(), Options { faults: Faults::error_at(Point::MidReplacement(6)), ..Default::default() });
    let again = recover(root, Options { faults: Faults::error_at(Point::MidReplacement(3)), ..Default::default() });
    assert!(matches!(again, Err(SpikeError::Crash(_))));
    assert!(matches!(classify(read_verified(root)), Seen::Refused(_)));
    assert_eq!(recover(root, Options::default()).unwrap(), Recovery::RolledBack);
    assert_eq!(classify(read_verified(root)), Seen::Old);
}

#[test]
fn crash_during_roll_forward_recovery_is_recovered_by_rerunning() {
    let dir = published_old();
    let root = dir.path();
    let _ = publish(root, &new(), Options { faults: Faults::error_at(Point::AfterSelection), ..Default::default() });
    // Simulate lost renames (e.g. power loss reordering) after the manifest landed.
    let skill = resolve(root, ".claude/skills/messenger/platform-metadata.md");
    std::fs::write(&skill, &old().artifacts[".claude/skills/messenger/platform-metadata.md"]).unwrap();
    assert!(matches!(classify(read_verified(root)), Seen::Refused(_)));
    assert_eq!(recover(root, Options::default()).unwrap(), Recovery::RolledForward);
    assert_eq!(classify(read_verified(root)), Seen::New);
}

#[test]
fn interrupted_initial_publication_leaves_no_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let _ = publish(root, &new(), Options { faults: Faults::error_at(Point::MidReplacement(4)), ..Default::default() });
    assert_eq!(classify(read_verified(root)), Seen::NoSnapshot);
    assert_eq!(recover(root, Options::default()).unwrap(), Recovery::RolledBack);
    assert_eq!(classify(read_verified(root)), Seen::NoSnapshot);
    for path in new().artifacts.keys() {
        assert!(!resolve(root, path).exists(), "{path} left behind");
    }
}

#[test]
fn republishing_identical_snapshot_is_byte_identical_and_touches_nothing() {
    let dir = published_old();
    let root = dir.path();
    let before = std::fs::read(resolve(root, MANIFEST_PATH)).unwrap();
    let report = publish(root, &old(), Options::default()).unwrap();
    assert_eq!(report.replaced, 0);
    assert_eq!(std::fs::read(resolve(root, MANIFEST_PATH)).unwrap(), before);
    let report = publish(root, &new(), Options::default()).unwrap();
    assert_eq!(report.unchanged, 1, "signal.md carried over unchanged");
}

#[test]
fn manual_edit_or_partial_checkout_is_refused() {
    let dir = published_old();
    let root = dir.path();
    std::fs::write(resolve(root, "messenger/docs/research/platforms/catalog.json"), b"{}\n").unwrap();
    assert!(matches!(read_verified(root), Err(SpikeError::Inconsistent { .. })));
    // No journal: recovery cannot guess; the fix is regenerate or `git checkout`.
    assert_eq!(recover(root, Options::default()).unwrap(), Recovery::Clean);
    assert!(matches!(read_verified(root), Err(SpikeError::Inconsistent { .. })));
}

#[test]
fn second_publisher_is_locked_out() {
    let dir = published_old();
    let root = dir.path();
    let _held = publication_spike::fsutil::PublicationLock::try_acquire(&resolve(
        root,
        "messenger/.research-state/publication/lock",
    ))
    .unwrap();
    assert!(matches!(publish(root, &new(), Options::default()), Err(SpikeError::Locked)));
}
