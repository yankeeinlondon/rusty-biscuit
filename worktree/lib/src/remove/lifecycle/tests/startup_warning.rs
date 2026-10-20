//! A slow observation startup: [`Environment::slow_startup`] is told once,
//! before anything is moved or deleted, and the removal goes on.

use super::*;

/// What the hook saw each time it was called: the startup it was told, and
/// whether the checkout was still where the attempt found it.
type Calls = Mutex<Vec<(Duration, bool)>>;

/// Runs `body` with injected observers whose sessions report a startup of
/// `startup` (`None` leaves the injected session untimed), and a hook that
/// records each call with `untouched()` at that moment.
fn with_startup<R>(
    observers: &Observers,
    startup: Option<Duration>,
    calls: &Calls,
    untouched: &dyn Fn() -> bool,
    body: impl FnOnce(&Environment<'_>) -> R,
) -> R {
    let observe = |targets, moved, _interrupted: &dyn Fn() -> bool| {
        observers.start(targets, moved).map(|observation| match startup {
            Some(startup) => observation.started_after(startup),
            None => observation,
        })
    };
    let slow_startup = |startup: Duration| calls.lock().unwrap().push((startup, untouched()));
    body(&Environment { usage: &Usage::clear(), interrupted: &|| false, observe: &observe, slow_startup: &slow_startup })
}

fn first_attempt(fixture: &Fixture, startup: Option<Duration>, calls: &Calls) -> Removal {
    let (base, entry, approval, branches) = (fixture.base(), fixture.entry(), fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &base, entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    let observers = Observers::default();
    let untouched = || fixture.gone.join("README.md").is_file() && !observers.injector().moved();
    with_startup(&observers, startup, calls, &untouched, |env| remove_checkout(&request, env))
}

/// A startup over [`SLOW_STARTUP`] is reported once, with its duration,
/// while the checkout is still at its original path, and does not refuse.
#[test]
fn a_slow_startup_is_reported_once_before_the_move_and_the_removal_completes() {
    let fixture = fixture();
    let calls = Calls::default();

    let removal = first_attempt(&fixture, Some(Duration::from_millis(1500)), &calls);

    assert_removed(&fixture, &finished(removal));
    assert_eq!(*calls.lock().unwrap(), [(Duration::from_millis(1500), true)]);
}

/// Exactly [`SLOW_STARTUP`], a fast startup, and an untimed one are not
/// reported.
#[test]
fn a_startup_of_at_most_one_second_is_not_reported() {
    for startup in [Some(SLOW_STARTUP), Some(Duration::from_millis(5)), None] {
        let fixture = fixture();
        let calls = Calls::default();

        let removal = first_attempt(&fixture, startup, &calls);

        assert_removed(&fixture, &finished(removal));
        assert!(calls.lock().unwrap().is_empty(), "{startup:?}: {:?}", calls.lock().unwrap());
    }
}

/// A resumed attempt's own slow startup is reported once, before it deletes
/// anything more from the quarantine.
#[test]
fn a_resumed_attempts_slow_startup_is_reported_once_before_its_deletion() {
    let fixture = fixture();
    let id = crash_at(&fixture, &Observers::default(), Boundary::Moved);
    let quarantine = fixture.quarantine_of(&id);
    assert!(quarantine.join("README.md").is_file(), "the crashed attempt deleted nothing");
    let (base, fresh, branches) = (fixture.base(), fixture.resume_approval(&id), fixture.delete_branch());
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&fresh), record: None, branches: &branches };
    let calls = Calls::default();
    let untouched = || quarantine.join("README.md").is_file() && quarantine.join("target/debug/a.o").is_file();

    let removal = with_startup(&Observers::default(), Some(Duration::from_millis(2500)), &calls, &untouched, |env| resume(&request, env));

    assert_removed(&fixture, &finished(removal));
    assert_eq!(*calls.lock().unwrap(), [(Duration::from_millis(2500), true)]);
}
