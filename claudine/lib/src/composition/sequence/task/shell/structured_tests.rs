use super::*;
use darkmatter::markdown::compose::shell_expansion::template::FilledSegment;

fn segment(program: &str, args: &[&str], operator: ChainOperator) -> FilledSegment {
    FilledSegment { program: program.into(), args: args.iter().map(|arg| (*arg).into()).collect(), operator, redirection: RedirectionConfig::default() }
}

struct TextOnly;
impl TaskShellRunner for TextOnly {
    fn run(&self, _: &str, _: Duration, _: Option<&AtomicBool>, _: Option<&Arc<TaskLiveOutput>>) -> Result<ShellCommandOutput, TaskShellError> {
        panic!("structured values must never fall back to shell text")
    }
}

#[test]
fn custom_runners_refuse_structured_commands_without_a_shell_fallback() {
    assert!(matches!(TextOnly.run_structured(&FilledChain { segments: vec![] }, Duration::from_secs(1), None, None), Err(TaskShellError::UnsupportedStructured)));
}

#[test]
fn an_invalid_later_target_refuses_the_whole_chain() {
    let native = std::env::current_exe().unwrap();
    let chain = FilledChain { segments: vec![segment(native.to_str().unwrap(), &["--list"], ChainOperator::None), segment("claudine-no-such-direct-program-489812", &[], ChainOperator::Or)] };
    assert!(matches!(SystemTaskShell::default().run_structured(&chain, Duration::from_secs(2), None, None), Err(TaskShellError::Target { program, .. }) if program == "claudine-no-such-direct-program-489812"));
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn script(dir: &std::path::Path, name: &str, body: &str) -> String {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path.to_str().unwrap().to_owned()
    }

    #[test]
    fn direct_arguments_preserve_empty_words_metacharacters_and_unicode() {
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "argv", "printf '<%s>\\n' \"$@\"");
        let marker = dir.path().join("forbidden");
        let injection = format!("$(touch '{}')", marker.display());
        let values = ["", "two words", "$(touch forbidden)", "a;echo b", "a|b", "'quoted'", "日本語", "line\nbreak", &injection];
        let chain = FilledChain { segments: vec![segment(&probe, &values, ChainOperator::None)] };
        let output = SystemTaskShell::default().run_structured(&chain, Duration::from_secs(2), None, None).unwrap();
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, values.iter().map(|value| format!("<{value}>\n")).collect::<String>());
        assert!(!marker.exists());
    }

    #[test]
    fn a_later_bad_target_prevents_the_first_side_effect() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("started");
        let probe = script(dir.path(), "first", "touch \"$1\"");
        let chain = FilledChain { segments: vec![segment(&probe, &[marker.to_str().unwrap()], ChainOperator::None), segment("claudine-no-such-direct-program-489812", &[], ChainOperator::Or)] };
        assert!(matches!(SystemTaskShell::default().run_structured(&chain, Duration::from_secs(2), None, None), Err(TaskShellError::Target { .. })));
        assert!(!marker.exists());
    }

    #[test]
    fn direct_chains_short_circuit_and_capture_only_segments_that_run() {
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "probe", "printf '%s' \"$1\"; exit \"$2\"");
        let chain = FilledChain { segments: vec![segment(&probe, &["fail", "7"], ChainOperator::None), segment(&probe, &["skipped", "0"], ChainOperator::And), segment(&probe, &["recover", "0"], ChainOperator::Or), segment(&probe, &["done", "0"], ChainOperator::And)] };
        let output = SystemTaskShell::default().run_structured(&chain, Duration::from_secs(2), None, None).unwrap();
        assert_eq!(output.stdout, "failrecoverdone");
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn direct_redirections_preserve_capture_channel_contract() {
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "probe", "printf out; printf err >&2");
        for (stdout, stderr, expected) in [(StdoutTarget::Capture, StderrTarget::Null, "out"), (StdoutTarget::Null, StderrTarget::ToStdout, "err"), (StdoutTarget::ToStderr, StderrTarget::Null, ""), (StdoutTarget::Null, StderrTarget::Null, "")] {
            let mut command = segment(&probe, &[], ChainOperator::None);
            command.redirection = RedirectionConfig { stdout, stderr };
            let output = SystemTaskShell::default().run_structured(&FilledChain { segments: vec![command] }, Duration::from_secs(2), None, None).unwrap();
            assert_eq!(output.stdout, expected);
            assert_eq!(output.exit_code, 0);
        }
    }

    #[derive(Default)]
    struct Sink { data: Mutex<String>, status: Mutex<String> }

    impl crate::render::TaskStreamSink for Sink {
        fn write_frames(&self, frames: &[String]) { self.status.lock().unwrap().push_str(&frames.concat()); }
        fn write_data_frames(&self, frames: &[String]) { self.data.lock().unwrap().push_str(&frames.concat()); }
    }

    #[test]
    fn redirected_stdout_streams_as_status_and_stderr_as_captured_data() {
        use crate::render::{TaskBar, TaskStream};
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "probe", "printf 'stdout-payload\\n'; printf 'stderr-payload\\n' >&2");
        let mut command = segment(&probe, &[], ChainOperator::None);
        command.redirection = RedirectionConfig { stdout: StdoutTarget::ToStderr, stderr: StderrTarget::ToStdout };
        let sink = Arc::new(Sink::default());
        let live = Arc::new(TaskLiveOutput::new(
            TaskStream::new("task", TaskBar::for_index(0), biscuit_terminal::terminal::Terminal::default()),
            Arc::clone(&sink) as Arc<dyn crate::render::TaskStreamSink>,
        ));
        let output = SystemTaskShell::default().run_structured(&FilledChain { segments: vec![command] }, Duration::from_secs(2), None, Some(&live)).unwrap();
        assert_eq!(output.stdout, "stderr-payload\n");
        let data = sink.data.lock().unwrap();
        let status = sink.status.lock().unwrap();
        assert!(data.contains("stderr-payload"));
        assert!(!data.contains("stdout-payload"));
        assert!(status.contains("stdout-payload"));
        assert!(!status.contains("stderr-payload"));
    }

    fn assert_dead(pid: &str) {
        let pid = pid.trim().parse::<i32>().unwrap();
        let stop = Instant::now() + Duration::from_secs(1);
        while unsafe { libc::kill(pid, 0) } == 0 && Instant::now() < stop { std::thread::sleep(Duration::from_millis(5)); }
        assert_ne!(unsafe { libc::kill(pid, 0) }, 0, "descendant {pid} survived");
    }

    #[test]
    fn direct_deadlines_reap_descendants_and_stop_later_segments() {
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "probe", "sleep 300 &\nprintf '%s\\n' \"$!\"; wait");
        let chain = FilledChain { segments: vec![segment(&probe, &[], ChainOperator::None), segment("/bin/echo", &["must-not-run"], ChainOperator::Or)] };
        let output = SystemTaskShell::default().run_structured(&chain, Duration::from_millis(400), None, None).unwrap();
        assert!(output.timed_out);
        assert!(!output.stdout.contains("must-not-run"));
        assert_dead(&output.stdout);
    }

    #[test]
    fn direct_interrupts_reap_a_running_tree() {
        let dir = tempfile::tempdir().unwrap();
        let ready = dir.path().join("ready");
        let probe = script(dir.path(), "probe", "sleep 300 &\nprintf '%s\\n' \"$!\"; touch \"$1\"; wait");
        let chain = FilledChain { segments: vec![segment(&probe, &[ready.to_str().unwrap()], ChainOperator::None)] };
        let flag = Arc::new(AtomicBool::new(false));
        let setter = Arc::clone(&flag);
        let watcher = std::thread::spawn(move || {
            let stop = Instant::now() + Duration::from_secs(2);
            while !ready.exists() && Instant::now() < stop { std::thread::sleep(Duration::from_millis(5)); }
            let observed = ready.exists();
            setter.store(true, Ordering::SeqCst);
            observed
        });
        let output = SystemTaskShell::default().run_structured(&chain, Duration::from_secs(3), Some(&flag), None).unwrap();
        assert!(watcher.join().unwrap());
        assert!(output.interrupted);
        assert!(!output.timed_out);
        assert_dead(&output.stdout);
    }

    #[test]
    fn direct_wait_errors_settle_output_and_reap_the_tree() {
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "probe", "printf ready; sleep 300");
        let chain = FilledChain { segments: vec![segment(&probe, &[], ChainOperator::None)] };
        let result = SystemTaskShell::failing_wait().run_structured(&chain, Duration::from_secs(2), None, None);
        assert!(matches!(result, Err(TaskShellError::Wait(_))));
    }

    #[test]
    fn direct_isolation_errors_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "probe", "sleep 300");
        let chain = FilledChain { segments: vec![segment(&probe, &[], ChainOperator::None)] };
        let result = SystemTaskShell::failing_isolation().run_structured(&chain, Duration::from_secs(2), None, None);
        assert!(matches!(result, Err(TaskShellError::Isolation(_))));
    }

    #[test]
    fn direct_chain_capture_is_bounded_across_segments() {
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "probe", "printf '12345678'");
        let chain = FilledChain { segments: vec![segment(&probe, &[], ChainOperator::None), segment(&probe, &[], ChainOperator::And), segment(&probe, &[], ChainOperator::And)] };
        let output = SystemTaskShell::with_volume_cap(CaptureVolumeCap::new(true, 100, 12)).run_structured(&chain, Duration::from_secs(2), None, None).unwrap();
        assert_eq!(output.stdout, "1234567812345678");
        assert_eq!(output.runaway.unwrap().limit, 12);
    }

    #[test]
    fn direct_runaway_output_uses_the_owned_tree_guard() {
        let dir = tempfile::tempdir().unwrap();
        let probe = script(dir.path(), "probe", "while :; do printf 'xxxxxxxxxxxxxxxx\\n'; done");
        let chain = FilledChain { segments: vec![segment(&probe, &[], ChainOperator::None)] };
        let output = SystemTaskShell::with_volume_cap(CaptureVolumeCap::new(true, 8, 1024)).run_structured(&chain, Duration::from_secs(2), None, None).unwrap();
        assert!(output.runaway.is_some());
        assert!(!output.timed_out);
        assert!(!output.stdout.is_empty());
    }
}
