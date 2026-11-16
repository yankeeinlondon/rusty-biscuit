//! The lifecycle command runner: every command owns its tree, its result and
//! a terminal restoration failure combine with a fixed priority, and a
//! foreground command's keyboard interrupt cancels the run.
//!
//! These run without a terminal, so the handoff is inactive; the
//! `FOREGROUND` seam stands in for a command that held the foreground. The
//! real transfer is exercised in a PTY by the CLI's
//! `level1_lifecycle_shell_terminal_pty` tests.

use super::*;
use crate::composition::lifecycle::cancellation::{CancellationCause, CancellationToken};

fn restore_failure() -> Result<(), std::io::Error> {
    Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
}

#[test]
fn a_restoration_failure_fails_only_a_command_that_exited_zero() {
    let error = settle_lifecycle_command(Ok(LifecycleExit::Code(0)), restore_failure()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);

    assert_eq!(settle_lifecycle_command(Ok(LifecycleExit::Code(3)), restore_failure()).unwrap(), 3);
    assert_eq!(settle_lifecycle_command(Ok(LifecycleExit::Cancelled), restore_failure()).unwrap(), 130);
    let wait = std::io::Error::other("wait failed");
    let error = settle_lifecycle_command(Err(wait), restore_failure()).unwrap_err();
    assert_eq!(error.to_string(), "wait failed");
}

#[test]
fn a_foreground_interrupt_marks_the_run_interrupted() {
    crate::interrupt::clear_for_tests();
    assert_eq!(settle_lifecycle_command(Ok(LifecycleExit::ForegroundInterrupt), Ok(())).unwrap(), 130);
    assert!(crate::interrupt::interrupted());
    crate::interrupt::clear_for_tests();
}

#[test]
fn an_established_worker_cancellation_keeps_the_interrupt_unmarked() {
    crate::interrupt::clear_for_tests();
    let token = CancellationToken::default();
    token.cancel(CancellationCause::Interrupt);
    let _scope = token.enter();
    assert_eq!(settle_lifecycle_command(Ok(LifecycleExit::ForegroundInterrupt), restore_failure()).unwrap(), 130);
    assert!(!crate::interrupt::interrupted(), "a worker cause must not become a process-wide interrupt");
}

#[cfg(unix)]
mod unix {
    use super::*;
    use crate::composition::{DocumentShellRunner, ShellRunError, ShellRunner, SystemShellRunner};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::ExitStatusExt;
    use std::path::Path;

    fn exited(code: i32) -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(code << 8)
    }

    fn signalled(signal: i32) -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(signal)
    }

    #[test]
    fn only_a_foreground_command_turns_sigint_or_130_into_an_interrupt() {
        for status in [signalled(libc::SIGINT), exited(130)] {
            assert_eq!(classify_lifecycle_exit(status, true), LifecycleExit::ForegroundInterrupt, "{status:?}");
        }
        assert_eq!(classify_lifecycle_exit(signalled(libc::SIGINT), false), LifecycleExit::Code(-1));
        assert_eq!(classify_lifecycle_exit(exited(130), false), LifecycleExit::Code(130));
        assert_eq!(classify_lifecycle_exit(signalled(libc::SIGTERM), true), LifecycleExit::Code(-1));
        assert_eq!(classify_lifecycle_exit(exited(0), true), LifecycleExit::Code(0));
        assert_eq!(classify_lifecycle_exit(exited(2), true), LifecycleExit::Code(2));
    }

    fn alive(pid: i32) -> bool {
        // SAFETY: signal 0 only checks that the PID exists.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    fn wait_until_dead(pid: i32) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while alive(pid) {
            assert!(Instant::now() < deadline, "process {pid} survived its lifecycle command");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn wait_for_pid(path: &Path) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(text) = std::fs::read_to_string(path)
                && let Ok(pid) = text.trim().parse()
            {
                return pid;
            }
            assert!(Instant::now() < deadline, "{} never appeared", path.display());
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Shell text that leaves a background `sleep` (its PID in `bg`), then
    /// runs `rest`.
    fn leaves_background(bg: &Path, rest: &str) -> String {
        format!("sleep 30 & echo $! > '{}'; {rest}", bg.display())
    }

    /// An executable script, for a structured chain's native program.
    fn script(dir: &Path, name: &str, body: &str) -> String {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path.to_str().unwrap().to_owned()
    }

    fn chain(segments: &[(&str, &[&str], ChainOperator)]) -> FilledChain {
        FilledChain {
            segments: segments
                .iter()
                .map(|(program, args, operator)| darkmatter::markdown::compose::shell_expansion::template::FilledSegment {
                    program: (*program).into(),
                    args: args.iter().map(|arg| (*arg).into()).collect(),
                    operator: *operator,
                    redirection: RedirectionConfig::default(),
                })
                .collect(),
        }
    }

    fn document_runner() -> DocumentShellRunner {
        DocumentShellRunner::new(EnvBaseline::from_entries(std::env::vars()))
    }

    #[test]
    fn a_command_is_not_started_once_cancellation_is_requested() {
        crate::interrupt::clear_for_tests();
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("started");
        crate::interrupt::mark_interrupted();
        let code = SystemShellRunner.run(&format!("touch '{}'", marker.display())).unwrap();
        crate::interrupt::clear_for_tests();
        assert_eq!(code, 130);
        assert!(!marker.exists(), "a cancelled run started a lifecycle command");
    }

    /// Exact bytes through both runners: the exit code is the command's, and
    /// the background process it left is stopped before the runner returns.
    #[test]
    fn exact_bytes_completion_stops_what_the_command_left_running() {
        let dir = tempfile::tempdir().unwrap();
        let runners: [(&str, Box<dyn ShellRunner>); 2] =
            [("system", Box::new(SystemShellRunner)), ("document", Box::new(document_runner()))];
        for (name, runner) in runners {
            for code in [0, 3] {
                let bg = dir.path().join(format!("{name}-{code}"));
                assert_eq!(runner.run(&leaves_background(&bg, &format!("exit {code}"))).unwrap(), code, "{name}");
                wait_until_dead(wait_for_pid(&bg));
            }
        }
    }

    #[test]
    fn a_structured_chain_segment_owns_its_tree() {
        let dir = tempfile::tempdir().unwrap();
        let program = script(dir.path(), "leaves-background", "sleep 30 & echo $! > \"$1\"; exit \"$2\"");
        let runners: [(&str, Box<dyn ShellRunner>); 2] =
            [("system", Box::new(SystemShellRunner)), ("document", Box::new(document_runner()))];
        for (name, runner) in runners {
            let first = dir.path().join(format!("{name}-first"));
            let second = dir.path().join(format!("{name}-second"));
            let filled = chain(&[
                (&program, &[first.to_str().unwrap(), "4"], ChainOperator::None),
                (&program, &[second.to_str().unwrap(), "0"], ChainOperator::Or),
            ]);
            assert_eq!(runner.run_structured(&filled).unwrap(), 0, "{name}");
            wait_until_dead(wait_for_pid(&first));
            wait_until_dead(wait_for_pid(&second));
        }
    }

    #[test]
    fn worker_cancellation_while_the_command_runs_stops_its_tree() {
        crate::interrupt::clear_for_tests();
        let dir = tempfile::tempdir().unwrap();
        let bg = dir.path().join("bg");
        let root = dir.path().join("root");
        let command = leaves_background(&bg, &format!("echo $$ > '{}'; wait", root.display()));
        let token = CancellationToken::default();
        let (code, bg_pid, root_pid) = std::thread::scope(|scope| {
            let worker = token.clone();
            let running = scope.spawn(move || {
                let _scope = worker.enter();
                SystemShellRunner.run(&command)
            });
            let bg_pid = wait_for_pid(&bg);
            let root_pid = wait_for_pid(&root);
            token.cancel(CancellationCause::Interrupt);
            (running.join().unwrap(), bg_pid, root_pid)
        });
        assert_eq!(code.unwrap(), 130);
        wait_until_dead(bg_pid);
        wait_until_dead(root_pid);
        assert!(!crate::interrupt::interrupted(), "a worker cancellation is not a user interrupt");
    }

    #[test]
    fn a_failed_wait_stops_the_whole_tree() {
        let dir = tempfile::tempdir().unwrap();
        let bg = dir.path().join("bg");
        let root = dir.path().join("root");
        test_seams::FAIL_WAIT_AFTER.set(Some(root.clone()));
        let command = leaves_background(&bg, &format!("echo $$ > '{}'; wait", root.display()));
        let error = SystemShellRunner.run(&command).unwrap_err();
        let ShellRunError::Spawn { source, .. } = &error else {
            panic!("{error:?}");
        };
        assert_eq!(source.to_string(), "injected lifecycle wait failure");
        wait_until_dead(wait_for_pid(&bg));
        wait_until_dead(wait_for_pid(&root));
    }

    /// Ordinary exit codes, including `130` and a `SIGINT` end, keep their
    /// meaning when the command never held the foreground.
    #[test]
    fn without_the_foreground_sigint_and_130_are_ordinary_results() {
        crate::interrupt::clear_for_tests();
        assert_eq!(SystemShellRunner.run("kill -INT $$").unwrap(), -1);
        assert_eq!(SystemShellRunner.run("exit 130").unwrap(), 130);
        assert!(!crate::interrupt::interrupted());
    }

    /// A foreground command that ends by `SIGINT` or exits `130` interrupts
    /// the run: both runners return `130`, the run is marked interrupted, and
    /// a structured chain starts no later segment.
    #[test]
    fn a_foreground_keyboard_interrupt_cancels_the_run() {
        let dir = tempfile::tempdir().unwrap();
        test_seams::FOREGROUND.set(true);
        for command in ["kill -INT $$", "exit 130"] {
            for runner in [&SystemShellRunner as &dyn ShellRunner, &document_runner()] {
                crate::interrupt::clear_for_tests();
                assert_eq!(runner.run(command).unwrap(), 130, "{command}");
                assert!(crate::interrupt::interrupted(), "{command}");
            }
        }

        crate::interrupt::clear_for_tests();
        let later = dir.path().join("later");
        let interrupted = script(dir.path(), "interrupted", "kill -INT $$");
        let toucher = script(dir.path(), "touch-later", "touch \"$1\"");
        let filled = chain(&[
            (&interrupted, &[], ChainOperator::None),
            (&toucher, &[later.to_str().unwrap()], ChainOperator::None),
        ]);
        assert_eq!(SystemShellRunner.run_structured(&filled).unwrap(), 130);
        assert!(crate::interrupt::interrupted());
        assert!(!later.exists(), "a chain segment ran after the keyboard interrupt");
        crate::interrupt::clear_for_tests();
        test_seams::FOREGROUND.set(false);
    }
}
