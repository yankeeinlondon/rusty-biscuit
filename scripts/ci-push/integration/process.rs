//! `ChildCommand` against real children: argv, stdin, output draining, the
//! sanitized environment, and ownership of the whole tree at a deadline or a
//! cancellation.

use std::path::Path;
use std::time::{Duration, Instant};

use repo_deps::ci_push::process::{
    CancelSignal, Cancellation, ChildCommand, Completed, Termination,
};
use rstest::rstest;
use serde_json::{json, Value};

use crate::fixture::{pid_of, process_gone, read_pid, same_path, wait_until, Fixture};

/// Bounds every supervised run, so a regression fails instead of hanging.
const RUN_LIMIT: Duration = Duration::from_secs(4);
/// How long an orphaned, killed descendant may take to be reaped by init.
const REAP_LIMIT: Duration = Duration::from_secs(3);

fn run(
    fixture: &Fixture,
    name: &str,
    configure: impl FnOnce(ChildCommand) -> ChildCommand,
) -> Completed {
    configure(
        ChildCommand::new(fixture.tool(name))
            .current_dir(fixture.work())
            .cancellation(Cancellation::new())
            .deadline(RUN_LIMIT),
    )
    .run()
    .expect("the fixture runs")
}

#[test]
fn arguments_reach_the_child_as_separate_native_values() {
    let fixture = Fixture::new(&["gh"], json!({ "gh": {} }));
    let args = [
        "with space",
        "ünïcödé ☃",
        "",
        "quote\"d",
        r"back\slash\",
        "--flag=a b",
    ];

    let completed = run(&fixture, "gh", |command| command.args(args));

    assert!(completed.success(), "{completed:?}");
    let invocation = fixture.only_invocation();
    assert_eq!(invocation["argv"], json!(args));
    assert!(same_path(
        Path::new(invocation["cwd"].as_str().unwrap()),
        &fixture.work()
    ));
}

#[test]
fn stdin_is_null_so_the_child_reads_end_of_file() {
    let fixture = Fixture::new(&["just"], json!({ "just": { "read_stdin": true } }));

    let completed = run(&fixture, "just", |command| command);

    assert!(completed.success(), "{completed:?}");
    assert_eq!(fixture.only_invocation()["stdin"], json!(""));
}

#[test]
fn large_stdout_and_stderr_are_drained_concurrently() {
    const MIB: usize = 1024 * 1024;
    let fixture = Fixture::new(
        &["cargo"],
        json!({ "cargo": { "stdout_fill": MIB, "stderr_fill": MIB } }),
    );

    let completed = run(&fixture, "cargo", |command| command);

    assert!(completed.success(), "{:?}", completed.termination);
    assert_eq!(completed.stdout.len(), MIB);
    assert_eq!(completed.stderr.len(), MIB);
    assert!(completed.stdout.iter().all(|byte| *byte == b'o'));
    assert!(completed.stderr.iter().all(|byte| *byte == b'e'));
    assert!(!completed.incomplete);
}

#[test]
fn a_capture_limit_keeps_the_head_and_still_drains_the_rest() {
    let fixture = Fixture::new(
        &["cargo"],
        json!({ "cargo": { "stdout_fill": 512 * 1024, "stderr": "short" } }),
    );

    let completed = run(&fixture, "cargo", |command| command.capture_limit(1000));

    assert!(completed.success(), "{completed:?}");
    assert_eq!(completed.stdout.len(), 1000);
    assert_eq!(completed.stderr, b"short");
    assert!(completed.incomplete);
}

#[test]
fn inherited_output_is_not_captured() {
    let fixture = Fixture::new(&["just"], json!({ "just": { "stdout": "gate output\n" } }));

    let completed = run(&fixture, "just", ChildCommand::inherit_output);

    assert!(completed.success(), "{completed:?}");
    assert!(completed.stdout.is_empty() && completed.stderr.is_empty());
}

const PINNING: [&str; 6] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_COMMON_DIR",
    "GIT_PREFIX",
];

fn recorded_env_fixture() -> Fixture {
    let mut names: Vec<&str> = PINNING.to_vec();
    names.extend([
        "BISCUIT_CI_PLAN_IN",
        "GIT_TERMINAL_PROMPT",
        "PYTHONDONTWRITEBYTECODE",
        "PYTHONUTF8",
        "GIT_CONFIG_GLOBAL",
        "CI_PUSH_UNRELATED",
    ]);
    Fixture::new(&["git"], json!({ "git": { "record_env": names } }))
}

fn with_pinned_environment(mut command: ChildCommand) -> ChildCommand {
    for name in PINNING {
        command = command.env(name, "/pinned");
    }
    command
        .env("BISCUIT_CI_PLAN_IN", "/inherited/plan.json")
        .env("GIT_CONFIG_GLOBAL", "/kept/gitconfig")
        .env("CI_PUSH_UNRELATED", "kept")
        .env("GIT_TERMINAL_PROMPT", "1")
        .env("PYTHONUTF8", "0")
}

#[test]
fn a_repository_independent_child_loses_only_the_pinning_variables() {
    let fixture = recorded_env_fixture();

    let completed = run(&fixture, "git", |command| {
        with_pinned_environment(command)
            .repository_independent()
            .clear_plan_input()
    });

    assert!(completed.success(), "{completed:?}");
    let env = &fixture.only_invocation()["env"];
    for name in PINNING {
        assert_eq!(env[name], Value::Null, "{name} must be removed");
    }
    assert_eq!(env["BISCUIT_CI_PLAN_IN"], Value::Null);
    assert_eq!(env["GIT_CONFIG_GLOBAL"], json!("/kept/gitconfig"));
    assert_eq!(env["CI_PUSH_UNRELATED"], json!("kept"));
    assert_eq!(env["GIT_TERMINAL_PROMPT"], json!("0"));
    assert_eq!(env["PYTHONDONTWRITEBYTECODE"], json!("1"));
    assert_eq!(env["PYTHONUTF8"], json!("1"));
}

#[test]
fn pinning_variables_and_the_plan_input_stay_unless_removal_is_asked_for() {
    let fixture = recorded_env_fixture();

    let completed = run(&fixture, "git", with_pinned_environment);

    assert!(completed.success(), "{completed:?}");
    let env = &fixture.only_invocation()["env"];
    for name in PINNING {
        assert_eq!(env[name], json!("/pinned"), "{name}");
    }
    assert_eq!(env["BISCUIT_CI_PLAN_IN"], json!("/inherited/plan.json"));
    assert_eq!(
        env["GIT_TERMINAL_PROMPT"],
        json!("0"),
        "fixed variables always win"
    );
}

/// A fixture child that starts a hanging grandchild (which writes its pid to
/// the returned path once running) and then hangs itself.
fn tree_fixture() -> (Fixture, std::path::PathBuf) {
    let fixture = Fixture::new(&["just"], json!({}));
    let pid_file = fixture.work().join("grandchild.pid");
    fixture.configure(json!({
        "just": { "stdout": "started\n", "spawn_grandchild": pid_file, "hang": true }
    }));
    (fixture, pid_file)
}

/// The tree is gone: the direct child was reaped before `run` returned, and
/// the grandchild was killed.
fn assert_tree_gone(fixture: &Fixture, pid_file: &Path) {
    let child = pid_of(&fixture.only_invocation());
    if cfg!(unix) {
        // Unreaped, it would still answer as a zombie.
        assert!(process_gone(child), "direct child {child} was not reaped");
    } else {
        wait_until(REAP_LIMIT, "the direct child to exit", || {
            process_gone(child)
        });
    }
    let grandchild = read_pid(pid_file);
    wait_until(REAP_LIMIT, "the grandchild to be killed", || {
        process_gone(grandchild)
    });
}

#[test]
fn a_deadline_kills_the_whole_tree_and_keeps_the_output_read_so_far() {
    let (fixture, pid_file) = tree_fixture();
    let started = Instant::now();

    let completed = run(&fixture, "just", |command| {
        command.deadline(Duration::from_secs(1))
    });

    assert_eq!(completed.termination, Termination::TimedOut);
    assert!(completed.interrupted());
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    assert!(
        pid_file.exists(),
        "the grandchild never became ready, so the tree was not proven"
    );
    assert_eq!(completed.stdout, b"started\n");
    assert_tree_gone(&fixture, &pid_file);
}

#[test]
fn cancellation_kills_the_whole_tree_and_maps_to_the_signal_exit_code() {
    let (fixture, pid_file) = tree_fixture();
    let token = Cancellation::new();

    let completed = std::thread::scope(|scope| {
        scope.spawn(|| {
            wait_until(RUN_LIMIT, "the grandchild to start", || pid_file.exists());
            token.cancel(CancelSignal::Interrupt);
        });
        run(&fixture, "just", |command| {
            command.cancellation(token.clone())
        })
    });

    assert_eq!(
        completed.termination,
        Termination::Cancelled(CancelSignal::Interrupt)
    );
    assert_eq!(CancelSignal::Interrupt.exit_code(), 130);
    assert!(completed.interrupted());
    assert_eq!(completed.stdout, b"started\n");
    assert_tree_gone(&fixture, &pid_file);
}

/// A descendant that left the owned process group cannot be killed, and may
/// hold the pipe forever; the supervisor stops waiting for it.
#[cfg(unix)]
#[test]
fn a_pipe_held_outside_the_owned_tree_is_abandoned_after_a_grace() {
    let fixture = Fixture::new(&["gh"], json!({}));
    let pid_file = fixture.work().join("escaped.pid");
    fixture.configure(json!({ "gh": { "stdout": "listed\n", "escape_grandchild": pid_file } }));
    let started = Instant::now();

    let completed = run(&fixture, "gh", |command| command);

    let elapsed = started.elapsed();
    wait_until(RUN_LIMIT, "the escaped grandchild to start", || {
        pid_file.exists()
    });
    let escaped = read_pid(&pid_file);
    // SAFETY: the pid is the fixture grandchild this test caused.
    unsafe {
        libc::kill(escaped as libc::pid_t, libc::SIGKILL);
    }
    assert!(completed.success(), "{completed:?}");
    assert_eq!(completed.stdout, b"listed\n");
    assert!(
        completed.incomplete,
        "the held pipe must mark the capture incomplete"
    );
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    wait_until(REAP_LIMIT, "the escaped grandchild to be killed", || {
        process_gone(escaped)
    });
}

#[rstest]
#[case(130, true)]
#[case(128, true)]
#[case(3, false)]
fn a_child_exit_of_128_or_more_is_an_interruption(#[case] code: u8, #[case] interrupted: bool) {
    let fixture = Fixture::new(&["just"], json!({ "just": { "exit": code } }));

    let completed = run(&fixture, "just", |command| command);

    let Termination::Exited(status) = completed.termination else {
        panic!("{completed:?}");
    };
    assert_eq!(status.code(), Some(i32::from(code)));
    assert_eq!(completed.interrupted(), interrupted);
    assert!(!completed.success());
}

/// A real signal, sent to a fixture child (never to the test process) that
/// supervises its own hanging grandchild through `ChildCommand` with the
/// production handlers installed.
#[cfg(unix)]
#[rstest]
#[case(libc::SIGINT, 130)]
#[case(libc::SIGTERM, 143)]
#[case(libc::SIGHUP, 129)]
fn a_handled_signal_ends_the_supervised_tree_with_the_signal_exit_code(
    #[case] signal: libc::c_int,
    #[case] expected: i32,
) {
    let fixture = Fixture::new(&["just"], json!({}));
    let pid_file = fixture.work().join("grandchild.pid");
    fixture.configure(json!({ "supervisor": { "supervise": pid_file } }));

    let completed = std::thread::scope(|scope| {
        scope.spawn(|| {
            wait_until(RUN_LIMIT, "the supervised grandchild to start", || {
                pid_file.exists()
            });
            let supervisor = pid_of(&fixture.only_invocation());
            // SAFETY: the pid is the fixture child this test started.
            assert_eq!(unsafe { libc::kill(supervisor as libc::pid_t, signal) }, 0);
        });
        run(&fixture, "just", |command| {
            command.args(["--scenario", "supervisor"])
        })
    });

    let Termination::Exited(status) = completed.termination else {
        panic!("{completed:?}");
    };
    assert_eq!(status.code(), Some(expected), "{completed:?}");
    assert!(completed.interrupted());
    let grandchild = read_pid(&pid_file);
    wait_until(REAP_LIMIT, "the supervised grandchild to be killed", || {
        process_gone(grandchild)
    });
}
