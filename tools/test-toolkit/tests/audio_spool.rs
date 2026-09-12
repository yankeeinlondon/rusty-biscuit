//! Regressions for [`test_toolkit::LockedAudioSpool`].
//!
//! The property under test is the one a green audio test cannot otherwise
//! promise: when the fixture releases worker ownership, no runnable record is
//! left behind — including while the test is unwinding, and including when
//! another actor is mutating the queue at the same time.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use fs4::fs_std::FileExt;
use test_toolkit::LockedAudioSpool;

/// Longest the queue holder waits for the fixture-owning scope to unwind
/// before it commits its record and releases `queue.lock`.
///
/// Only the passing path pays this. Cleanup that takes `queue.lock` cannot
/// finish while the holder owns it, so the wait runs to the deadline; cleanup
/// that skips the lock finishes immediately and the holder proceeds at once.
const QUEUE_HOLD_BUDGET: Duration = Duration::from_millis(250);

/// Cadence for the two flag waits below. Both conditions are set by another
/// thread, so this polls rather than spins.
const POLL: Duration = Duration::from_millis(1);

fn wait_for(flag: &AtomicBool, deadline: Option<Instant>) {
    while !flag.load(Ordering::SeqCst) {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return;
        }
        std::thread::sleep(POLL);
    }
}

fn assert_worker_lock_is_free(root: &Path) {
    let worker = LockedAudioSpool::open_lock(&LockedAudioSpool::worker_lock_path(root))
        .expect("worker lock should open");
    assert!(
        worker
            .try_lock_exclusive()
            .expect("worker lock should probe"),
        "the fixture must release worker ownership only after cleanup"
    );
}

fn assert_worker_lock_is_held(root: &Path) {
    let worker = LockedAudioSpool::open_lock(&LockedAudioSpool::worker_lock_path(root))
        .expect("worker lock should open");
    assert!(
        !worker
            .try_lock_exclusive()
            .expect("worker lock should probe"),
        "worker ownership was released over a record cleanup could not remove"
    );
}

#[test]
fn publication_fixture_clears_jobs_before_unlocking_on_unwind() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("spool");
    let pending = root.join("fixture.pending.json");

    let result = std::panic::catch_unwind(|| {
        let _spool = LockedAudioSpool::new(&root);
        fs::write(&pending, b"inert fixture job").unwrap();
        panic!("simulated assertion failure");
    });

    assert!(result.is_err());
    assert!(!pending.exists());
    assert_worker_lock_is_free(&root);
}

#[test]
fn publication_fixture_clears_a_record_committed_under_the_queue_lock() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("spool");
    let pending = root.join("1.pending.json");
    fs::create_dir_all(&root).unwrap();

    // Mirrors `playa::detached::enqueue_state_at`, which owns `queue.lock`
    // across its commit. `queue_held` reports that ownership. `owner_finished`
    // reports that the fixture-owning scope has fully unwound, which can only
    // precede the commit if cleanup skipped the queue lock — so the holder
    // waiting on it is what makes the broken ordering fail deterministically
    // rather than occasionally.
    let queue_held = Arc::new(AtomicBool::new(false));
    let owner_finished = Arc::new(AtomicBool::new(false));

    let publisher = std::thread::spawn({
        let queue_path = LockedAudioSpool::queue_lock_path(&root);
        let pending = pending.clone();
        let queue_held = Arc::clone(&queue_held);
        let owner_finished = Arc::clone(&owner_finished);
        move || {
            let queue = LockedAudioSpool::open_lock(&queue_path).expect("queue lock should open");
            queue
                .lock_exclusive()
                .expect("publisher should own the queue");
            queue_held.store(true, Ordering::SeqCst);

            wait_for(&owner_finished, Some(Instant::now() + QUEUE_HOLD_BUDGET));
            fs::write(&pending, b"record committed under the queue lock").unwrap();

            FileExt::unlock(&queue).expect("publisher should release the queue");
        }
    });
    wait_for(&queue_held, None);

    let result = std::panic::catch_unwind(|| {
        let _spool = LockedAudioSpool::new(&root);
        panic!("simulated assertion failure");
    });

    owner_finished.store(true, Ordering::SeqCst);
    publisher.join().expect("publisher should not panic");

    assert!(result.is_err());
    assert!(
        !pending.exists(),
        "a record committed under the queue lock survived fixture cleanup"
    );
    assert_worker_lock_is_free(&root);
}

/// Longest either side of the handoff waits for the other. Neither pays it on
/// the passing path; it only bounds a thread whose partner never arrives.
const HANDOFF_BUDGET: Duration = Duration::from_secs(10);

/// The other side of the same property: a publisher that arrives *after* the
/// cleanup scan rather than before it must not be able to commit while the
/// fixture still owns `worker.lock`.
///
/// The interval at risk is two adjacent unlock calls wide, so a thread waiting
/// on `queue.lock` would observe the wrong order only occasionally. The
/// fixture's handoff observer runs inside the critical section, which makes the
/// observation deterministic.
#[test]
fn cleanup_holds_the_queue_lock_across_the_release_of_worker_ownership() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("spool");
    let pending = root.join("2.pending.json");

    let scan_finished = Arc::new(AtomicBool::new(false));
    let probe_finished = Arc::new(AtomicBool::new(false));
    let committed = Arc::new(AtomicBool::new(false));

    let publisher = std::thread::spawn({
        let root = root.clone();
        let pending = pending.clone();
        let scan_finished = Arc::clone(&scan_finished);
        let probe_finished = Arc::clone(&probe_finished);
        let committed = Arc::clone(&committed);
        move || {
            wait_for(&scan_finished, Some(Instant::now() + HANDOFF_BUDGET));

            let queue = LockedAudioSpool::open_lock(&LockedAudioSpool::queue_lock_path(&root))
                .expect("queue lock should open");
            if queue.try_lock_exclusive().expect("queue lock should probe") {
                fs::write(&pending, b"record committed after the cleanup scan").unwrap();
                committed.store(true, Ordering::SeqCst);
                FileExt::unlock(&queue).expect("publisher should release the queue");
            }
            probe_finished.store(true, Ordering::SeqCst);
        }
    });

    let mut spool = LockedAudioSpool::new(&root);
    spool.observe_handoff({
        let scan_finished = Arc::clone(&scan_finished);
        let probe_finished = Arc::clone(&probe_finished);
        move || {
            scan_finished.store(true, Ordering::SeqCst);
            wait_for(&probe_finished, Some(Instant::now() + HANDOFF_BUDGET));
        }
    });
    drop(spool);

    publisher.join().expect("publisher should not panic");

    assert!(
        !committed.load(Ordering::SeqCst),
        "a publisher took queue ownership between the cleanup scan and the worker release"
    );
    assert!(
        !pending.exists(),
        "a record committed after the cleanup scan survived the handoff"
    );
    assert_worker_lock_is_free(&root);
}

/// A cleanup failure must not hand the spool to the next scheduler.
///
/// A directory named like a record makes `remove_file` fail while leaving the
/// root and both lock files intact, which is the only failure shape that keeps
/// the interesting state observable afterwards.
#[test]
fn a_failed_cleanup_retains_worker_ownership() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("spool");

    let result = std::panic::catch_unwind(|| {
        let _spool = LockedAudioSpool::new(&root);
        fs::create_dir(root.join("unremovable.pending.json")).unwrap();
    });
    assert!(
        result.is_err(),
        "a cleanup failure must fail the owning test"
    );

    assert_worker_lock_is_held(&root);
}

/// The same failure while the thread is already unwinding must not raise a
/// second panic. That would abort the process, so this test reaching its
/// assertions at all is the proof; what it then checks is that the original
/// failure is the one that propagated and that ownership was still retained.
#[test]
fn a_failed_cleanup_while_unwinding_reports_without_a_second_panic() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("spool");

    let result = std::panic::catch_unwind(|| {
        let _spool = LockedAudioSpool::new(&root);
        fs::create_dir(root.join("unremovable.pending.json")).unwrap();
        panic!("simulated assertion failure");
    });

    let payload = result.expect_err("the original failure should propagate");
    assert_eq!(
        payload.downcast_ref::<&str>().copied(),
        Some("simulated assertion failure"),
        "the cleanup failure must not replace the failure that started the unwind"
    );
    assert_worker_lock_is_held(&root);
}
