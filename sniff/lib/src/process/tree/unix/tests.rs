use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::*;
use crate::process::tree::ProcessTree;

/// Everything the fake host was asked to do, shared with the test so it can
/// still be read after the owner (and its drop) are gone.
#[derive(Default)]
struct FakeState {
    now: Option<Instant>,
    refreshes: usize,
    table: Vec<Observed>,
    pid_kills: Vec<(u32, i32)>,
    group_kills: Vec<(i32, i32)>,
    /// Answers for successive `kill_group` calls; `ESRCH` once exhausted.
    group_answers: VecDeque<io::Result<()>>,
}

#[derive(Clone)]
struct FakeHost(Rc<RefCell<FakeState>>);

impl FakeHost {
    fn new(table: Vec<Observed>) -> Self {
        Self(Rc::new(RefCell::new(FakeState {
            now: Some(Instant::now()),
            table,
            ..FakeState::default()
        })))
    }

    fn advance(&self, by: Duration) {
        let mut state = self.0.borrow_mut();
        state.now = state.now.map(|now| now + by);
    }

    fn set_table(&self, table: Vec<Observed>) {
        self.0.borrow_mut().table = table;
    }

    fn answer_group_kills(&self, answers: impl IntoIterator<Item = io::Result<()>>) {
        self.0.borrow_mut().group_answers.extend(answers);
    }

    fn refreshes(&self) -> usize {
        self.0.borrow().refreshes
    }

    fn pid_kills(&self) -> Vec<(u32, i32)> {
        self.0.borrow().pid_kills.clone()
    }

    fn group_kills(&self) -> usize {
        self.0.borrow().group_kills.len()
    }
}

impl Host for FakeHost {
    fn now(&self) -> Instant {
        self.0.borrow().now.expect("fake clock is set")
    }

    fn refresh(&mut self) {
        self.0.borrow_mut().refreshes += 1;
    }

    fn snapshot(&self) -> Vec<Observed> {
        self.0.borrow().table.clone()
    }

    fn kill_pid(&mut self, pid: u32, signal: i32) -> io::Result<()> {
        self.0.borrow_mut().pid_kills.push((pid, signal));
        Ok(())
    }

    fn kill_group(&mut self, pgid: i32, signal: i32) -> io::Result<()> {
        let mut state = self.0.borrow_mut();
        state.group_kills.push((pgid, signal));
        state
            .group_answers
            .pop_front()
            .unwrap_or_else(|| Err(io::Error::from_raw_os_error(libc::ESRCH)))
    }
}

const ROOT: u32 = 100;

fn entry(pid: u32, parent: Option<u32>, start_time: u64) -> Observed {
    Observed {
        pid,
        parent,
        start_time,
    }
}

/// The root (start 1) with one child, 200 (start 2), which has a child, 300.
fn root_with_two_generations() -> Vec<Observed> {
    vec![
        entry(ROOT, Some(1), 1),
        entry(200, Some(ROOT), 2),
        entry(300, Some(200), 3),
    ]
}

fn owner(host: &FakeHost) -> Core<FakeHost> {
    Core::with_host(host.clone(), ROOT)
}

fn killed_pids(host: &FakeHost) -> Vec<u32> {
    let mut pids: Vec<u32> = host.pid_kills().into_iter().map(|(pid, _)| pid).collect();
    pids.sort_unstable();
    pids
}

// ---- sample scheduling -------------------------------------------------

#[test]
fn no_scan_at_attach() {
    let host = FakeHost::new(root_with_two_generations());
    let core = owner(&host);
    assert_eq!(host.refreshes(), 0);
    drop(core);
}

#[test]
fn first_sample_waits_one_interval() {
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);

    core.sample();
    host.advance(DESCENDANT_SAMPLE_INTERVAL - Duration::from_millis(1));
    core.sample();
    assert_eq!(host.refreshes(), 0, "no sample is due before one interval");
    assert!(core.observed.is_empty());

    host.advance(Duration::from_millis(1));
    core.sample();
    assert_eq!(host.refreshes(), 1);
    assert_eq!(core.observed.len(), 2, "both descendants are recorded");
    assert_eq!(core.root_start, Some(1), "the root's identity is learned");
    core.armed = false;
}

#[test]
fn samples_are_no_more_frequent_than_the_interval() {
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);

    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();
    host.advance(Duration::from_millis(50));
    core.sample();
    host.advance(DESCENDANT_SAMPLE_INTERVAL - Duration::from_millis(51));
    core.sample();
    assert_eq!(host.refreshes(), 1);

    host.advance(Duration::from_millis(1));
    core.sample();
    assert_eq!(host.refreshes(), 2);
    core.armed = false;
}

// ---- cleanup scan policy ----------------------------------------------

#[test]
fn cleanup_scan_policy() {
    // A short, successful command: nothing observed, no scan.
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    core.terminate(CleanupScan::WhenObserved).unwrap();
    assert_eq!(host.refreshes(), 0);
    assert_eq!(host.group_kills(), 1, "the group is still signaled");

    // An interrupted command: one scan even though nothing was observed.
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    core.terminate(CleanupScan::Always).unwrap();
    assert_eq!(host.refreshes(), 1);

    // A command whose descendants were observed: one scan at cleanup.
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();
    core.terminate(CleanupScan::WhenObserved).unwrap();
    assert_eq!(host.refreshes(), 2, "one sample plus one cleanup scan");
    assert_eq!(killed_pids(&host), vec![200, 300]);
}

// ---- identity checks ---------------------------------------------------

#[test]
fn a_validated_root_lends_its_descendants_to_cleanup() {
    let host = FakeHost::new(vec![entry(ROOT, Some(1), 1)]);
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();

    // A descendant started after the last sample is still found through the
    // root, whose identity matches.
    host.set_table(root_with_two_generations());
    core.terminate(CleanupScan::Always).unwrap();
    assert_eq!(killed_pids(&host), vec![200, 300]);
    assert!(host.pid_kills().iter().all(|(_, signal)| *signal == libc::SIGKILL));
}

#[test]
fn an_observed_descendant_is_killed_after_reparenting() {
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();

    // The root exited and was reaped; 200 called `setsid` and was reparented
    // to init, keeping its child.
    host.set_table(vec![entry(200, Some(1), 2), entry(300, Some(200), 3)]);
    core.terminate(CleanupScan::WhenObserved).unwrap();
    assert_eq!(killed_pids(&host), vec![200, 300]);
}

#[test]
fn recycled_observed_pid_is_not_a_traversal_root_or_target() {
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();
    assert!(core.observed.contains_key(&200));

    // 200 and 300 exited and the root was reaped; PID 200 now names an
    // unrelated process (start 99) that has a child of its own.
    let recycled = vec![entry(200, Some(1), 99), entry(400, Some(200), 5)];
    host.set_table(recycled);

    // A later sample forgets the recycled PID instead of walking from it.
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();
    assert!(core.observed.is_empty(), "observed: {:?}", core.observed);

    // Cleanup signals neither the recycled PID nor its child.
    core.terminate(CleanupScan::Always).unwrap();
    assert!(host.pid_kills().is_empty(), "kills: {:?}", host.pid_kills());
}

#[test]
fn recycled_observed_pid_is_skipped_at_cleanup_without_another_sample() {
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();

    host.set_table(vec![entry(200, Some(1), 99), entry(400, Some(200), 5)]);
    core.terminate(CleanupScan::WhenObserved).unwrap();
    assert!(host.pid_kills().is_empty(), "kills: {:?}", host.pid_kills());
}

#[test]
fn root_never_sampled_is_not_a_traversal_root() {
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    core.terminate(CleanupScan::Always).unwrap();
    assert!(
        host.pid_kills().is_empty(),
        "an unverified root must not lend its children: {:?}",
        host.pid_kills()
    );
    assert_eq!(host.group_kills(), 1, "the group is still signaled");
}

#[test]
fn root_with_changed_start_is_not_a_traversal_root() {
    let host = FakeHost::new(vec![entry(ROOT, Some(1), 1)]);
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();

    // The root was reaped and its PID reused by a process with children.
    host.set_table(vec![entry(ROOT, Some(1), 7), entry(200, Some(ROOT), 8)]);
    core.terminate(CleanupScan::Always).unwrap();
    assert!(host.pid_kills().is_empty(), "kills: {:?}", host.pid_kills());
}

// ---- graceful shutdown scans ------------------------------------------

/// The table after the root exited from `SIGTERM` and was reaped, and the
/// helper (200, start 2), which ignored it, was reparented: to init, or to a
/// parent the table does not name.
fn helper_reparented(parent: Option<u32>, start_time: u64) -> Vec<Observed> {
    vec![entry(200, parent, start_time)]
}

#[test]
fn graceful_scan_retains_a_helper_reparented_before_the_hard_stop() {
    for parent in [Some(1), None] {
        let host = FakeHost::new(vec![entry(ROOT, Some(1), 1)]);
        let mut core = owner(&host);
        host.advance(DESCENDANT_SAMPLE_INTERVAL);
        core.sample();

        // The helper started after the last sample; only the shutdown scan
        // links it to the tree.
        host.set_table(vec![entry(ROOT, Some(1), 1), entry(200, Some(ROOT), 2)]);
        core.request_exit(true);
        assert_eq!(host.pid_kills(), vec![(200, libc::SIGTERM)]);

        host.set_table(helper_reparented(parent, 2));
        core.terminate(CleanupScan::Always).unwrap();
        assert_eq!(
            host.pid_kills(),
            vec![(200, libc::SIGTERM), (200, libc::SIGKILL)],
            "parent {parent:?}"
        );
    }
}

#[test]
fn graceful_scan_does_not_kill_a_recycled_helper_pid() {
    let host = FakeHost::new(vec![entry(ROOT, Some(1), 1)]);
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();

    host.set_table(vec![entry(ROOT, Some(1), 1), entry(200, Some(ROOT), 2)]);
    core.request_exit(true);

    // The helper exited and PID 200 now names an unrelated process.
    host.set_table(helper_reparented(Some(1), 99));
    core.terminate(CleanupScan::Always).unwrap();
    assert_eq!(host.pid_kills(), vec![(200, libc::SIGTERM)]);
}

#[test]
fn graceful_scan_learns_an_unreaped_root_that_was_never_sampled() {
    let host = FakeHost::new(vec![entry(ROOT, Some(1), 1), entry(200, Some(ROOT), 2)]);
    let mut core = owner(&host);

    core.request_exit(true);
    assert_eq!(core.root_start, Some(1));
    assert_eq!(host.pid_kills(), vec![(200, libc::SIGTERM)]);

    host.set_table(helper_reparented(Some(1), 2));
    core.terminate(CleanupScan::Always).unwrap();
    assert_eq!(host.pid_kills(), vec![(200, libc::SIGTERM), (200, libc::SIGKILL)]);
}

#[test]
fn graceful_scan_does_not_trust_a_root_it_cannot_prove_unreaped() {
    let host = FakeHost::new(vec![entry(ROOT, Some(1), 1), entry(200, Some(ROOT), 2)]);
    let mut core = owner(&host);

    core.request_exit(false);
    assert_eq!(core.root_start, None);
    host.set_table(helper_reparented(Some(1), 2));
    core.terminate(CleanupScan::Always).unwrap();
    assert!(host.pid_kills().is_empty(), "kills: {:?}", host.pid_kills());
}

#[test]
fn failed_cleanup_keeps_scanned_identities_for_the_drop() {
    let host = FakeHost::new(vec![entry(ROOT, Some(1), 1)]);
    host.answer_group_kills([Err(io::Error::from_raw_os_error(libc::EACCES))]);
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();

    host.set_table(vec![entry(ROOT, Some(1), 1), entry(200, Some(ROOT), 2)]);
    core.terminate(CleanupScan::Always).unwrap_err();
    assert!(core.observed.contains_key(&200));

    host.set_table(helper_reparented(Some(1), 2));
    drop(core);
    assert_eq!(host.pid_kills(), vec![(200, libc::SIGKILL), (200, libc::SIGKILL)]);
}

// ---- disarming, failure, release --------------------------------------

#[test]
fn successful_hard_cleanup_disarms() {
    let host = FakeHost::new(root_with_two_generations());
    host.answer_group_kills([Ok(())]);
    let mut core = owner(&host);
    core.terminate(CleanupScan::Always).unwrap();
    let after_first = (host.group_kills(), host.refreshes(), host.pid_kills().len());

    core.terminate(CleanupScan::Always).unwrap();
    drop(core);
    assert_eq!(
        (host.group_kills(), host.refreshes(), host.pid_kills().len()),
        after_first,
        "a disarmed owner signals nothing again, explicitly or on drop"
    );
}

#[test]
fn failed_cleanup_keeps_the_drop_backstop() {
    let host = FakeHost::new(root_with_two_generations());
    host.answer_group_kills([Err(io::Error::from_raw_os_error(libc::EACCES))]);
    let mut core = owner(&host);
    let error = core
        .terminate(CleanupScan::WhenObserved)
        .expect_err("EACCES is a cleanup failure");
    assert_eq!(error.raw_os_error(), Some(libc::EACCES));
    assert_eq!(host.group_kills(), 1);

    drop(core);
    assert_eq!(host.group_kills(), 2, "the drop retries the group kill");
}

#[test]
fn an_armed_drop_hard_stops_the_tree() {
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();
    drop(core);
    assert_eq!(killed_pids(&host), vec![200, 300]);
    assert_eq!(host.group_kills(), 1);
}

#[test]
fn release_never_signals() {
    let host = FakeHost::new(root_with_two_generations());
    let mut core = owner(&host);
    host.advance(DESCENDANT_SAMPLE_INTERVAL);
    core.sample();
    let refreshes = host.refreshes();

    core.release().expect("Unix release never fails");
    assert!(host.pid_kills().is_empty());
    assert_eq!(host.group_kills(), 0);
    assert_eq!(host.refreshes(), refreshes, "release scans nothing");
}

// ---- group kill classification and repetition --------------------------

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[test]
fn group_kill_repeats_off_linux() {
    let host = FakeHost::new(Vec::new());
    host.answer_group_kills([
        Ok(()),
        Ok(()),
        Err(io::Error::from_raw_os_error(libc::ESRCH)),
    ]);
    let mut core = owner(&host);
    core.terminate(CleanupScan::WhenObserved).unwrap();
    assert_eq!(host.group_kills(), 3, "kill repeats until the group is empty");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn group_kill_is_not_repeated_on_linux() {
    let host = FakeHost::new(Vec::new());
    host.answer_group_kills([Ok(()), Ok(())]);
    let mut core = owner(&host);
    core.terminate(CleanupScan::WhenObserved).unwrap();
    assert_eq!(host.group_kills(), 1);
}

#[test]
fn an_absent_group_is_success() {
    let host = FakeHost::new(Vec::new());
    let mut core = owner(&host);
    core.terminate(CleanupScan::WhenObserved)
        .expect("ESRCH means already gone");
    assert!(!core.armed);
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[test]
fn a_zombie_only_group_answering_eperm_is_success_off_linux() {
    let host = FakeHost::new(Vec::new());
    host.answer_group_kills([Err(io::Error::from_raw_os_error(libc::EPERM))]);
    let mut core = owner(&host);
    core.terminate(CleanupScan::WhenObserved)
        .expect("macOS answers EPERM for a group of zombies");
    assert!(!core.armed);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn eperm_from_the_group_is_an_error_on_linux() {
    let host = FakeHost::new(Vec::new());
    host.answer_group_kills([Err(io::Error::from_raw_os_error(libc::EPERM))]);
    let mut core = owner(&host);
    let error = core.terminate(CleanupScan::WhenObserved).unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EPERM));
    assert!(core.armed);
    core.armed = false;
}

// ---- root verification -------------------------------------------------

#[test]
fn verify_root_rejects_invalid_ids() {
    let never = |_: i32| -> io::Result<i32> { panic!("not consulted for an invalid ID") };
    for pid in [0, -1, i32::MIN] {
        let error = verify_root(pid, never).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput, "pid {pid}");
    }

    let denied = verify_root(42, |_| Err(io::Error::from_raw_os_error(libc::EPERM))).unwrap_err();
    assert_eq!(denied.raw_os_error(), Some(libc::EPERM));
}

#[test]
fn verify_root_accepts_its_own_group_and_an_exited_child() {
    verify_root(42, Ok::<i32, io::Error>).expect("a group leader is accepted");
    verify_root(42, |_| Err(io::Error::from_raw_os_error(libc::ESRCH)))
        .expect("an exited, unreaped child is accepted");
}

#[test]
fn verify_root_rejects_a_foreign_group() {
    let error = verify_root(42, |_| Ok(7)).unwrap_err();
    assert!(
        error.to_string().contains("child 42 is in foreign process group 7"),
        "{error}"
    );
}

// ---- real processes ----------------------------------------------------

/// SIGKILLs and reaps a spawned fixture on every exit path.
struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn get(&mut self) -> &mut Child {
        self.0.as_mut().expect("child is present")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// SIGKILLs a PID reported by a fixture on every exit path.
struct PidGuard(Option<libc::pid_t>);

impl Drop for PidGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            // SAFETY: the PID was reported by a fixture this test spawned.
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
    }
}

fn spawn(script: &str, config: Option<TreeConfig>) -> ChildGuard {
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    if let Some(config) = config {
        config.configure(&mut command);
    }
    ChildGuard(Some(command.spawn().expect("sh should spawn")))
}

/// Waits up to 10 s for a fixture to publish its PID.
fn wait_for_pid(path: &std::path::Path) -> libc::pid_t {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(pid) = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| text.trim().parse().ok())
        {
            return pid;
        }
        assert!(Instant::now() < deadline, "fixture never published its PID");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Waits up to `limit` for `pid` to stop existing (a zombie still exists).
fn gone_within(pid: libc::pid_t, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        // SAFETY: signal zero only checks existence and permission.
        if unsafe { libc::kill(pid, 0) } != 0
            && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn attach_accepts_a_child_that_already_exited() {
    let mut child = spawn("true", Some(TreeConfig::process_group()));
    let pid = libc::id_t::try_from(child.get().id()).unwrap();
    // Wait for the exit without reaping, so the child is an unreaped zombie.
    // SAFETY: `info` is a valid out-parameter; `WNOWAIT` leaves the child
    // waitable for `Child::wait` below.
    let waited = unsafe {
        let mut info: libc::siginfo_t = std::mem::zeroed();
        libc::waitid(libc::P_PID, pid, &mut info, libc::WEXITED | libc::WNOWAIT)
    };
    assert_eq!(waited, 0, "waitid: {}", io::Error::last_os_error());

    let tree = ProcessTree::attach(child.get()).expect("an exited, unreaped child is owned");
    assert!(child.get().wait().unwrap().success());
    drop(tree);
}

#[test]
fn attach_rejects_a_live_root_in_a_foreign_group() {
    let mut child = spawn("exec sleep 30", None);
    let error = ProcessTree::attach(child.get()).expect_err("the runner's group is foreign");
    assert!(error.to_string().contains("foreign process group"), "{error}");
}

#[test]
fn session_mode_makes_the_child_a_session_leader() {
    let mut child = spawn("exec sleep 30", Some(TreeConfig::session()));
    let pid = libc::pid_t::try_from(child.get().id()).unwrap();
    // SAFETY: `getsid`/`getpgid` have no memory-safety preconditions.
    let (sid, pgid) = unsafe { (libc::getsid(pid), libc::getpgid(pid)) };
    assert_eq!(sid, pid, "the child leads a new session");
    assert_eq!(pgid, pid, "and a new process group");
    ProcessTree::attach(child.get())
        .expect("a session leader leads its group")
        .release()
        .unwrap();
}

#[test]
fn group_mode_does_not_start_a_session() {
    let mut child = spawn("exec sleep 30", Some(TreeConfig::process_group()));
    let pid = libc::pid_t::try_from(child.get().id()).unwrap();
    // SAFETY: as above.
    let (sid, pgid) = unsafe { (libc::getsid(pid), libc::getpgid(pid)) };
    assert_ne!(sid, pid, "the child keeps the caller's session");
    assert_eq!(pgid, pid, "the child leads a new process group");
}

#[test]
fn a_caller_pre_exec_hook_runs_after_group_setup() {
    let mut command = Command::new("true");
    TreeConfig::process_group().configure(&mut command);
    // SAFETY: the hook calls only async-signal-safe functions.
    unsafe {
        command.pre_exec(|| {
            if libc::getpgrp() == libc::getpid() {
                Ok(())
            } else {
                Err(io::Error::other("hook ran before the group was created"))
            }
        });
    }
    let mut child = ChildGuard(Some(
        command
            .spawn()
            .expect("the hook sees the child already leading its group"),
    ));
    assert!(child.get().wait().unwrap().success());
}

#[test]
fn immediate_terminate_kills_a_group_member_that_left_the_root_alone() {
    let scratch = tempfile::tempdir().unwrap();
    let pid_file = scratch.path().join("member");
    let script = format!(
        "sleep 60 & echo $! > '{}'; while :; do sleep 1; done",
        pid_file.display()
    );
    let mut child = spawn(&script, Some(TreeConfig::process_group()));
    let mut tree = ProcessTree::attach(child.get()).unwrap();
    let member = PidGuard(Some(wait_for_pid(&pid_file)));

    tree.terminate(CleanupScan::Always).unwrap();
    let status = child.get().wait().unwrap();
    assert!(!status.success(), "the root was killed");
    assert!(
        gone_within(member.0.unwrap(), Duration::from_secs(3)),
        "the backgrounded group member survived"
    );
}

#[test]
fn graceful_cleanup_when_the_root_exits_but_a_descendant_ignores_sigterm() {
    let scratch = tempfile::tempdir().unwrap();
    let pid_file = scratch.path().join("stubborn");
    let script = format!(
        "trap 'exit 0' TERM; \
         sh -c 'trap \"\" TERM; echo $$ > \"$1\"; while :; do sleep 1; done' inner '{}' & \
         while :; do sleep 1; done",
        pid_file.display()
    );
    let mut child = spawn(&script, Some(TreeConfig::process_group()));
    let mut tree = ProcessTree::attach(child.get()).unwrap();
    let stubborn = PidGuard(Some(wait_for_pid(&pid_file)));
    let root = libc::pid_t::try_from(child.get().id()).unwrap();

    let termination = tree.terminate_gracefully(Duration::from_millis(300), child.get());

    termination.cleanup.expect("the sweep succeeds");
    let status = termination.status.expect("the root is waited for");
    assert!(status.success(), "the root obeyed SIGTERM: {status:?}");
    // SAFETY: WNOHANG only reports; ECHILD proves the root was reaped.
    let rc = unsafe { libc::waitpid(root, std::ptr::null_mut(), libc::WNOHANG) };
    assert_eq!(rc, -1, "the root was already reaped");
    assert!(
        gone_within(stubborn.0.unwrap(), Duration::from_secs(3)),
        "the descendant ignoring SIGTERM survived the sweep"
    );
    child.0 = None;
}

#[test]
fn release_leaves_a_member_running() {
    let scratch = tempfile::tempdir().unwrap();
    let pid_file = scratch.path().join("survivor");
    let script = format!("sleep 60 & echo $! > '{}'; exit 0", pid_file.display());
    let mut child = spawn(&script, Some(TreeConfig::process_group()));
    let tree = ProcessTree::attach(child.get()).unwrap();
    let survivor = PidGuard(Some(wait_for_pid(&pid_file)));
    assert!(child.get().wait().unwrap().success());

    tree.release().unwrap();
    assert!(
        !gone_within(survivor.0.unwrap(), Duration::from_millis(200)),
        "a released tree's member keeps running"
    );
}

/// Names the helper fixture's PID file; read by the fixture below.
const HELPER_PID_FILE: &str = "SNIFF_TREE_HELPER_PID_FILE";

/// Starts its own session, ignores `SIGTERM`, then publishes its PID.
#[test]
#[ignore = "subprocess fixture invoked by the process-tree tests"]
fn detached_helper_ignoring_sigterm() {
    let pid_file = std::path::PathBuf::from(
        std::env::var_os(HELPER_PID_FILE).expect("the test provides a PID file"),
    );
    // SAFETY: this fixture is a fresh child of `sh`, not a group leader, and
    // `signal` with `SIG_IGN` installs no handler code.
    unsafe {
        assert_ne!(libc::setsid(), -1, "setsid: {}", io::Error::last_os_error());
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
    // Renamed into place so the reader never parses a partial PID.
    let partial = pid_file.with_extension("partial");
    std::fs::write(&partial, std::process::id().to_string()).unwrap();
    std::fs::rename(&partial, &pid_file).unwrap();
    std::thread::sleep(Duration::from_secs(30));
}

/// Spawns a root that waits for `go`, then starts
/// [`detached_helper_ignoring_sigterm`] and keeps running until signaled.
fn spawn_root_of_detached_helper(go: &std::path::Path, pid_file: &std::path::Path) -> ChildGuard {
    let mut command = Command::new("sh");
    command
        .args([
            "-c",
            "while [ ! -e \"$1\" ]; do sleep 0.01; done; \
             \"$2\" \"$3\" --exact --ignored --nocapture & \
             while :; do sleep 1; done",
            "root",
        ])
        .arg(go)
        .arg(std::env::current_exe().expect("the test executable resolves"))
        .arg("process::tree::unix::tests::detached_helper_ignoring_sigterm")
        .env(HELPER_PID_FILE, pid_file)
        // The helper outlives the root briefly; it must not hold the test's
        // output pipes open.
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    TreeConfig::process_group().configure(&mut command);
    ChildGuard(Some(command.spawn().expect("sh should spawn")))
}

/// Graceful cleanup must hard-stop a detached helper that ignores `SIGTERM`,
/// whether or not a sample ran before it started.
fn assert_graceful_cleanup_stops_detached_helper(sample_first: bool) {
    let scratch = tempfile::tempdir().unwrap();
    let go = scratch.path().join("go");
    let pid_file = scratch.path().join("helper");
    if !sample_first {
        std::fs::write(&go, "").unwrap();
    }
    let mut child = spawn_root_of_detached_helper(&go, &pid_file);
    let mut tree = ProcessTree::attach(child.get()).unwrap();
    if sample_first {
        // The sampler's own schedule: the first sample is due one interval
        // after attachment. It records the root before the helper exists.
        std::thread::sleep(DESCENDANT_SAMPLE_INTERVAL);
        tree.sample();
        std::fs::write(&go, "").unwrap();
    }
    let helper = PidGuard(Some(wait_for_pid(&pid_file)));

    let termination = tree.terminate_gracefully(Duration::from_secs(2), child.get());
    child.0 = None;

    termination.cleanup.expect("the sweep succeeds");
    termination.status.expect("the root is waited for");
    assert!(
        gone_within(helper.0.unwrap(), Duration::from_secs(3)),
        "the detached helper ignoring SIGTERM survived graceful cleanup"
    );
}

#[test]
fn graceful_cleanup_stops_a_detached_helper_first_seen_by_its_scan() {
    assert_graceful_cleanup_stops_detached_helper(false);
}

#[test]
fn graceful_cleanup_stops_a_detached_helper_started_after_the_last_sample() {
    assert_graceful_cleanup_stops_detached_helper(true);
}
