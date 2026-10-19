//! Criterion 7: a terminal interrupt settles both kinds of group leg before
//! their enclosing containers clean up. tmux never raises a window.

#[cfg(unix)]
#[test]
fn level2_ctrl_c_cancels_group_descendants_and_finalizes_innermost_first() {
    use std::fs;
    use std::time::{Duration, Instant};
    use biscuit_test_harness::{TerminalHarness, tmux::TmuxHarness};
    use test_toolkit::{Backend, Level, require_level};
    use crate::common::{CliProcessFixture, sh_quote, wait_for_exit_marker, write_executable};
    use crate::common::signal::ShellBarrier;

    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let fixture = CliProcessFixture::named("container-ctrl-c");
    fs::create_dir_all(fixture.home().join(".claudine")).unwrap();
    fs::write(fixture.home().join(".claudine/config.json"), "{}").unwrap();
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");
    let opted = ShellBarrier::new(fixture.cwd(), "opted");
    let plain = ShellBarrier::new(fixture.cwd(), "plain");
    for (name, barrier) in [("opted", &opted), ("plain", &plain)] {
        write_executable(&fixture.cwd().join(format!("{name}.sh")), &format!(
            "#!/bin/sh\n{}\nprintf '%s\\n' completed > {name}.completed\n",
            barrier.block_snippet(),
        ));
    }
    let document = r#"---
cancelled: [{ append_line: [events.log, root-cancelled] }]
finalize:
  - append_line: [events.log, root-finalize]
  - retry: { max_attempts: 1 }
sequence:
  - name: bundle
    group:
      name: bundle
      cancelled: [{ append_line: [events.log, group-cancelled] }]
      finalize: [{ append_line: [events.log, group-finalize] }]
      tasks:
        - name: opted
          cancelled: [{ append_line: [events.log, opted-cancelled] }]
          finalize: [{ append_line: [events.log, opted-finalize] }]
          sequence:
            - name: opted-child
              shell: ./opted.sh
        - name: plain
          finalize: [{ append_line: [events.log, plain-finalize] }]
          sequence:
            - name: plain-child
              shell: ./plain.sh
  - name: forbidden-later
    shell: echo forbidden > later-started
---
"#;
    let source = fixture.cwd().join("sequence.md");
    fs::write(&source, document).unwrap();
    let mut command = fixture.command_std();
    command.args(["sequence", "--goose", "--yolo", "--silent"]).arg(&source);
    let environment: Vec<(String, String)> = command.get_envs().filter_map(|(key, value)| {
        value.map(|value| (key.to_string_lossy().into_owned(), value.to_string_lossy().into_owned()))
    }).collect();
    let environment_refs: Vec<(&str, &str)> = environment.iter()
        .map(|(key, value)| (key.as_str(), value.as_str())).collect();
    let invocation = std::iter::once(command.get_program()).chain(command.get_args())
        .map(|argument| sh_quote(&argument.to_string_lossy())).collect::<Vec<_>>().join(" ");
    let mut pane = TmuxHarness::new();
    pane.spawn_shell().expect("spawn headless tmux pane");
    pane.resize(180, 60).unwrap();
    let script = format!("cd {} && exec {invocation}", sh_quote(&fixture.cwd().to_string_lossy()));
    pane.send_command_with_env(&format!(
        "/bin/sh -c {}; printf '\\ncontainer_rc:%s\\n' \"$?\"",
        sh_quote(&script),
    ), &environment_refs).unwrap();

    let deadline = Instant::now() + Duration::from_secs(30);
    while !opted.reached_path().exists() || !plain.reached_path().exists() {
        assert!(Instant::now() < deadline, "both group legs must start: {}", pane.capture().unwrap().plain);
        std::thread::sleep(Duration::from_millis(20));
    }
    let pids: Vec<i32> = [&opted, &plain].iter().map(|barrier| {
        fs::read_to_string(barrier.reached_path()).unwrap().trim().parse().unwrap()
    }).collect();
    pane.send_key("C-c").expect("terminal foreground-group interrupt");
    let (frame, status) = wait_for_exit_marker(&mut pane, "container_rc", Duration::from_secs(20));
    assert_eq!(status, "130", "{}", frame.plain);
    assert!(frame.plain.contains("cannot recover a cancelled"), "{}", frame.plain);
    for pid in pids {
        // Signal zero observes termination without changing the process.
        assert_ne!(unsafe { libc::kill(pid, 0) }, 0, "descendant {pid} survived wrapper exit");
    }
    for forbidden in ["opted.completed", "plain.completed", "later-started"] {
        assert!(!fixture.cwd().join(forbidden).exists(), "{forbidden} must not run");
    }
    let events = fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    let events: Vec<&str> = events.lines().collect();
    let position = |event| {
        assert_eq!(events.iter().filter(|line| **line == event).count(), 1, "{events:?}");
        events.iter().position(|line| *line == event).unwrap()
    };
    assert!(position("opted-cancelled") < position("opted-finalize"));
    assert!(position("opted-finalize") < position("group-cancelled"));
    assert!(position("plain-finalize") < position("group-cancelled"));
    assert!(position("group-cancelled") < position("group-finalize"));
    assert!(position("group-finalize") < position("root-cancelled"));
    assert!(position("root-cancelled") < position("root-finalize"));
    assert_eq!(fs::read_to_string(&source).unwrap(), document);
    assert!(!fixture.audio_spool().exists());
}

#[cfg(windows)]
#[test]
fn level2_ctrl_c_cancels_group_descendants_and_finalizes_innermost_first() {
    use test_toolkit::{Backend, Level, require_level};
    eprintln!("skipping criterion 7: this tmux foreground-group fixture needs Unix; native Windows needs a non-focusing console-control harness");
    require_level!(Level::L2, false, Backend::Tmux);
}
