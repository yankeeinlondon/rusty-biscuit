//! Cross-platform bookkeeping tests for the interrupt coordinator.
//!
//! Split from `coordinator.rs` to keep the module within the package area's
//! inline-test size budget (`claudine-cli/tests/test_placement.rs`).

use super::*;

fn registry() -> InterruptRegistry<&'static str> {
    InterruptRegistry::new()
}

#[test]
fn first_press_is_graceful_and_second_is_forceful() {
    let reg = registry();
    let token = reg.register_child("child-a");

    let first = reg.record_press();
    assert_eq!(first.count, 1);
    assert_eq!(first.targets.len(), 1);
    assert_eq!(first.targets[0].token, token);
    assert_eq!(first.targets[0].action, PressAction::Graceful);

    let second = reg.record_press();
    assert_eq!(second.count, 2);
    assert_eq!(second.targets[0].action, PressAction::Force);
}

#[test]
fn press_fans_out_to_every_registered_child() {
    let reg = registry();
    reg.register_child("child-a");
    reg.register_child("child-b");
    reg.register_child("child-c");

    let outcome = reg.record_press();
    let names: Vec<&str> = outcome.targets.iter().map(|t| t.child).collect();
    assert_eq!(names, vec!["child-a", "child-b", "child-c"]);
    assert!(
        outcome
            .targets
            .iter()
            .all(|t| t.action == PressAction::Graceful)
    );
}

/// Regression for the defect this coordinator replaces: escalation state
/// used to be process-global, so whichever sibling reached the forceful
/// rung first suppressed every other sibling's kill.
#[test]
fn a_siblings_escalation_does_not_suppress_another_childs_ladder() {
    let reg = registry();
    reg.register_child("early");
    reg.record_press();

    reg.register_child("late");
    let second = reg.record_press();

    let by_name: Vec<(&str, PressAction)> = second
        .targets
        .iter()
        .map(|t| (t.child, t.action))
        .collect();
    assert_eq!(
        by_name,
        vec![("early", PressAction::Force), ("late", PressAction::Graceful)]
    );
}

#[test]
fn deregistered_children_are_not_targeted() {
    let reg = registry();
    let gone = reg.register_child("child-a");
    reg.register_child("child-b");
    reg.deregister_child(gone);

    let outcome = reg.record_press();
    assert_eq!(outcome.targets.len(), 1);
    assert_eq!(outcome.targets[0].child, "child-b");
    assert_eq!(reg.child_presses(gone), 0);
    assert_eq!(reg.registered_children(), 1);
}

#[test]
fn child_presses_tracks_only_that_child() {
    let reg = registry();
    let a = reg.register_child("child-a");
    reg.record_press();
    let b = reg.register_child("child-b");
    reg.record_press();

    assert_eq!(reg.child_presses(a), 2);
    assert_eq!(reg.child_presses(b), 1);
}

#[test]
fn press_sets_every_registered_flag() {
    let reg = registry();
    let first = Arc::new(AtomicBool::new(false));
    let second = Arc::new(AtomicBool::new(false));
    reg.register_flag(&first);
    reg.register_flag(&second);

    reg.record_press();

    assert!(first.load(Ordering::SeqCst));
    assert!(second.load(Ordering::SeqCst));
}

#[test]
fn deregistered_flag_is_not_set() {
    let reg = registry();
    let flag = Arc::new(AtomicBool::new(false));
    let token = reg.register_flag(&flag);
    reg.deregister_flag(token);

    reg.record_press();

    assert!(!flag.load(Ordering::SeqCst));
    assert_eq!(reg.registered_flags(), 0);
}

#[test]
fn dropped_flag_registration_is_pruned_instead_of_panicking() {
    let reg = registry();
    {
        let flag = Arc::new(AtomicBool::new(false));
        reg.register_flag(&flag);
    }
    assert_eq!(reg.registered_flags(), 1);

    reg.record_press();

    assert_eq!(reg.registered_flags(), 0);
}

/// A press with nothing registered still counts, so a later child does not
/// inherit a stale rung and the feedback wording stays monotonic.
#[test]
fn press_count_advances_with_no_children_registered() {
    let reg = registry();
    assert_eq!(reg.record_press().count, 1);
    assert_eq!(reg.record_press().count, 2);

    let token = reg.register_child("late");
    let third = reg.record_press();
    assert_eq!(third.count, 3);
    assert_eq!(third.targets[0].action, PressAction::Graceful);
    assert_eq!(reg.child_presses(token), 1);
}

#[test]
fn press_count_saturates_rather_than_wrapping() {
    let reg = registry();
    for _ in 0..300 {
        reg.record_press();
    }
    assert_eq!(reg.record_press().count, u8::MAX);
}

#[test]
fn refcount_installs_on_the_first_holder_and_removes_on_the_last() {
    let refcount = InstallRefcount::new();
    assert_eq!(refcount.acquire(), RefcountTransition::Install);
    assert_eq!(refcount.acquire(), RefcountTransition::Unchanged);
    assert_eq!(refcount.acquire(), RefcountTransition::Unchanged);

    assert_eq!(refcount.release(), RefcountTransition::Unchanged);
    assert_eq!(refcount.release(), RefcountTransition::Unchanged);
    assert_eq!(refcount.release(), RefcountTransition::Remove);
}

#[test]
fn refcount_reinstalls_after_a_full_release() {
    let refcount = InstallRefcount::new();
    refcount.acquire();
    refcount.release();
    assert_eq!(refcount.acquire(), RefcountTransition::Install);
}

#[test]
fn unbalanced_release_does_not_report_a_second_removal() {
    let refcount = InstallRefcount::new();
    assert_eq!(refcount.release(), RefcountTransition::Unchanged);
    refcount.acquire();
    assert_eq!(refcount.release(), RefcountTransition::Remove);
    assert_eq!(refcount.release(), RefcountTransition::Unchanged);
}

/// Whatever the interleaving, concurrent holders must produce exactly one
/// install edge and exactly one remove edge.
#[test]
fn concurrent_holders_yield_one_install_and_one_remove() {
    static REFCOUNT: InstallRefcount = InstallRefcount::new();
    let installs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let removes = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // One long-lived holder spans every worker's lifetime, so the count
    // never returns to zero mid-run and the edges cannot be double-counted.
    assert_eq!(REFCOUNT.acquire(), RefcountTransition::Install);
    installs.fetch_add(1, Ordering::SeqCst);

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let installs = Arc::clone(&installs);
            let removes = Arc::clone(&removes);
            std::thread::spawn(move || {
                if REFCOUNT.acquire() == RefcountTransition::Install {
                    installs.fetch_add(1, Ordering::SeqCst);
                }
                if REFCOUNT.release() == RefcountTransition::Remove {
                    removes.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(REFCOUNT.release(), RefcountTransition::Remove);
    removes.fetch_add(1, Ordering::SeqCst);
    assert_eq!(installs.load(Ordering::SeqCst), 1);
    assert_eq!(removes.load(Ordering::SeqCst), 1);
}

/// Stand-in for `SetConsoleCtrlHandler`, recording the edges it is told
/// about so a host without a Win32 console can still assert them.
struct RecordingHandler;

static RECORDING_REFCOUNT: InstallRefcount = InstallRefcount::new();
static INSTALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static REMOVES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// The statics above are process-wide, exactly like the handler they model,
/// so the tests that read them must not interleave.
static RECORDING_LOCK: Mutex<()> = Mutex::new(());

impl ProcessHandler for RecordingHandler {
    fn refcount() -> &'static InstallRefcount {
        &RECORDING_REFCOUNT
    }

    fn install() {
        INSTALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn remove() {
        REMOVES.fetch_add(1, Ordering::SeqCst);
    }
}

/// ## Returns
///
/// `(installs, removes)` observed since the counters were reset.
fn recording_edges() -> (usize, usize) {
    (
        INSTALLS.load(Ordering::SeqCst),
        REMOVES.load(Ordering::SeqCst),
    )
}

fn reset_recording() -> MutexGuard<'static, ()> {
    let guard = RECORDING_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    INSTALLS.store(0, Ordering::SeqCst);
    REMOVES.store(0, Ordering::SeqCst);
    guard
}

#[test]
fn a_lone_handler_guard_installs_once_and_removes_on_drop() {
    let _lock = reset_recording();

    let guard = HandlerGuard::<RecordingHandler>::acquire();
    assert_eq!(recording_edges(), (1, 0));

    drop(guard);
    assert_eq!(recording_edges(), (1, 1));
}

/// The compose path's Windows shape: a compose-scoped registration and a
/// sequence-scoped one can be live at once, and whichever drops first must
/// leave the shared console handler installed for the other.
#[test]
fn dropping_a_compose_guard_leaves_a_live_sequence_registration_installed() {
    let _lock = reset_recording();

    let compose = HandlerGuard::<RecordingHandler>::acquire();
    let sequence = HandlerGuard::<RecordingHandler>::acquire();
    assert_eq!(recording_edges(), (1, 0), "second holder must not reinstall");

    drop(compose);
    assert_eq!(
        recording_edges(),
        (1, 0),
        "handler was torn down while the sequence registration was still live"
    );

    drop(sequence);
    assert_eq!(recording_edges(), (1, 1));
}

/// Concurrent registration and fan-out is the parallel-group shape: the
/// registry must serialize both without losing a child.
#[test]
fn concurrent_registration_and_presses_stay_consistent() {
    let reg = Arc::new(InterruptRegistry::<usize>::new());
    let mut handles = Vec::new();
    for id in 0..8usize {
        let reg = Arc::clone(&reg);
        handles.push(std::thread::spawn(move || {
            let token = reg.register_child(id);
            reg.record_press();
            token
        }));
    }
    let tokens: Vec<ChildToken> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    assert_eq!(reg.registered_children(), 8);
    assert!(tokens.iter().all(|t| reg.child_presses(*t) >= 1));
    for token in tokens {
        reg.deregister_child(token);
    }
    assert_eq!(reg.registered_children(), 0);
}
