use std::time::Instant;

use ::windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
use ::windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessHandleCount, OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    TerminateProcess, WaitForSingleObject,
};

use super::*;
use crate::process::tree::{ProcessTree, TreeConfig};

const SPAWNING_CHILD: &str = "process::tree::windows::tests::child_spawns_reporting_descendant";
const REPORTING_DESCENDANT: &str = "process::tree::windows::tests::reporting_descendant";
const SLEEPING_CHILD: &str = "process::tree::windows::tests::child_sleeps";
const PID_FILE: &str = "SNIFF_TREE_DESCENDANT_PID_FILE";

const ATTACH_STEPS: [Step; 5] = [
    Step::CreateJob,
    Step::SetKillOnClose,
    Step::Assign,
    Step::FindThread,
    Step::Resume,
];

fn fixture(name: &str) -> Command {
    let mut command =
        Command::new(std::env::current_exe().expect("current test executable should resolve"));
    command.args([name, "--exact", "--ignored", "--nocapture"]);
    command
}

#[test]
#[ignore = "subprocess fixture invoked by the process-tree tests"]
fn child_sleeps() {
    std::thread::sleep(Duration::from_secs(30));
}

/// Starts a descendant at once, before doing anything else, then sleeps.
#[test]
#[ignore = "subprocess fixture invoked by the process-tree tests"]
#[allow(clippy::zombie_processes)]
fn child_spawns_reporting_descendant() {
    fixture(REPORTING_DESCENDANT)
        .spawn()
        .expect("descendant should spawn");
    std::thread::sleep(Duration::from_secs(30));
}

#[test]
#[ignore = "subprocess fixture invoked by the process-tree tests"]
fn reporting_descendant() {
    let pid_file = std::env::var_os(PID_FILE).expect("the test provides a PID file");
    std::fs::write(pid_file, std::process::id().to_string()).expect("PID should be written");
    std::thread::sleep(Duration::from_secs(30));
}

/// Kills and reaps a spawned fixture on every exit path.
struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// An owned handle to a fixture's descendant, terminated on every exit path.
struct ProcessGuard(Owned<HANDLE>);

impl ProcessGuard {
    fn open(pid: u32) -> Self {
        // SAFETY: the handle is owned at once.
        let handle = unsafe {
            Owned::new(
                OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid)
                    .expect("descendant should be openable"),
            )
        };
        Self(handle)
    }

    fn exits_within(&self, limit: Duration) -> bool {
        let millis = u32::try_from(limit.as_millis()).unwrap_or(u32::MAX);
        // SAFETY: the handle is owned and live.
        unsafe { WaitForSingleObject(*self.0, millis) == WAIT_OBJECT_0 }
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        // SAFETY: the handle is owned and live; an exited process is a no-op.
        let _ = unsafe { TerminateProcess(*self.0, 1) };
    }
}

fn spawn_configured(mut command: Command) -> ChildGuard {
    TreeConfig::process_group().configure(&mut command);
    ChildGuard(command.spawn().expect("fixture should spawn"))
}

fn wait_for_pid(path: &std::path::Path) -> u32 {
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

fn handle_count() -> u32 {
    let mut count = 0;
    // SAFETY: the pseudo-handle needs no closing; `count` is a valid out-param.
    unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) }
        .expect("handle count should be readable");
    count
}

#[test]
fn attach_assigns_before_resuming() {
    hooks::reset();
    let mut child = spawn_configured(fixture(SLEEPING_CHILD));
    let tree = ProcessTree::attach(&child.0).expect("attach should succeed");
    assert_eq!(hooks::take_log(), ATTACH_STEPS);
    drop(tree);
    let _ = child.0.wait();
}

/// One injected failure per attach step: attach fails, no later step runs
/// (so nothing is resumed without assignment), and the caller's kill and wait
/// leave no suspended child behind.
fn attach_fails_at(failing: Step) {
    hooks::reset();
    let mut child = spawn_configured(fixture(SLEEPING_CHILD));
    hooks::inject(failing, io::Error::other("injected"));

    let error = ProcessTree::attach(&child.0).expect_err("an injected failure fails attach");
    assert_eq!(error.to_string(), "injected");

    let log = hooks::take_log();
    let position = ATTACH_STEPS.iter().position(|step| *step == failing).unwrap();
    assert_eq!(log, ATTACH_STEPS[..=position], "the log stops at the failing step");

    let _ = child.0.kill();
    let status = child.0.wait().expect("the direct child is reaped");
    assert!(!status.success(), "the suspended child never ran to completion");
}

#[test]
fn attach_fails_cleanly_when_the_job_cannot_be_created() {
    attach_fails_at(Step::CreateJob);
}

#[test]
fn attach_fails_cleanly_when_kill_on_close_cannot_be_set() {
    attach_fails_at(Step::SetKillOnClose);
}

#[test]
fn attach_fails_cleanly_when_assignment_fails() {
    attach_fails_at(Step::Assign);
}

#[test]
fn attach_fails_cleanly_when_the_thread_is_not_found() {
    attach_fails_at(Step::FindThread);
}

#[test]
fn attach_fails_cleanly_when_resume_fails() {
    attach_fails_at(Step::Resume);
}

#[test]
fn failed_attach_does_not_leak_handles() {
    hooks::reset();
    let before = handle_count();
    for _ in 0..25 {
        let mut child = spawn_configured(fixture(SLEEPING_CHILD));
        hooks::inject(Step::Resume, io::Error::other("injected"));
        assert!(ProcessTree::attach(&child.0).is_err());
        let _ = child.0.kill();
        let _ = child.0.wait();
    }
    let after = handle_count();
    assert!(
        after < before + 10,
        "handle count grew from {before} to {after}"
    );
}

#[test]
fn a_descendant_created_immediately_is_in_the_job() {
    let scratch = tempfile::tempdir().unwrap();
    let pid_file = scratch.path().join("descendant");
    let mut command = fixture(SPAWNING_CHILD);
    command.env(PID_FILE, &pid_file);
    let mut child = spawn_configured(command);
    let mut tree = ProcessTree::attach(&child.0).unwrap();
    let descendant = ProcessGuard::open(wait_for_pid(&pid_file));

    tree.terminate(CleanupScan::Always).unwrap();
    let _ = child.0.wait();
    assert!(
        descendant.exits_within(Duration::from_secs(5)),
        "the descendant escaped the Job"
    );
}

#[test]
fn release_leaves_a_member_running() {
    let scratch = tempfile::tempdir().unwrap();
    let pid_file = scratch.path().join("descendant");
    let mut command = fixture(SPAWNING_CHILD);
    command.env(PID_FILE, &pid_file);
    let child = spawn_configured(command);
    let tree = ProcessTree::attach(&child.0).unwrap();
    let descendant = ProcessGuard::open(wait_for_pid(&pid_file));

    tree.release().expect("kill-on-close is cleared");
    drop(child);
    assert!(
        !descendant.exits_within(Duration::from_millis(200)),
        "a released Job's member keeps running"
    );
}

#[test]
fn release_failure_keeps_cleanup_armed() {
    hooks::reset();
    let scratch = tempfile::tempdir().unwrap();
    let pid_file = scratch.path().join("descendant");
    let mut command = fixture(SPAWNING_CHILD);
    command.env(PID_FILE, &pid_file);
    let mut child = spawn_configured(command);
    let tree = ProcessTree::attach(&child.0).unwrap();
    let descendant = ProcessGuard::open(wait_for_pid(&pid_file));

    hooks::inject(Step::ClearKillOnClose, io::Error::other("injected"));
    let error = tree.release().expect_err("an injected failure fails release");
    assert_eq!(error.to_string(), "injected");
    assert!(
        descendant.exits_within(Duration::from_secs(5)),
        "the owner dropped after a failed release still terminates the Job"
    );
    let _ = child.0.wait();
}

#[test]
fn sampling_does_nothing() {
    hooks::reset();
    let mut child = spawn_configured(fixture(SLEEPING_CHILD));
    let mut tree = ProcessTree::attach(&child.0).unwrap();
    let _ = hooks::take_log();
    tree.sample();
    assert!(hooks::take_log().is_empty(), "sampling performs no step");
    tree.terminate(CleanupScan::Always).unwrap();
    assert_eq!(hooks::take_log(), [Step::Terminate]);
    tree.terminate(CleanupScan::Always).unwrap();
    assert!(hooks::take_log().is_empty(), "a disarmed owner does nothing");
    let _ = child.0.wait();
}

#[test]
fn graceful_termination_stops_the_job_and_reaps_the_root() {
    let mut child = spawn_configured(fixture(SLEEPING_CHILD));
    let mut tree = ProcessTree::attach(&child.0).unwrap();
    let termination = tree.terminate_gracefully(Duration::from_secs(10), &mut child.0);
    termination.cleanup.expect("the Job is terminated");
    let status = termination.status.expect("the root is waited for");
    assert!(!status.success());
}
