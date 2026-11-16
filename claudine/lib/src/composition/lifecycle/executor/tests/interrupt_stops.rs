//! A keyboard interrupt that a foreground lifecycle command reports stops the
//! normal work of its event even when `no_error` suppressed the command's
//! failure; cleanup events still run whole.

use super::*;
use darkmatter::lifecycle::executor::LifecycleHost;

/// A command that held the terminal and was interrupted from the keyboard,
/// as the production runner reports it: the run is marked interrupted and the
/// result is `130`.
struct InterruptedShell {
    commands: Mutex<Vec<String>>,
}

impl ShellRunner for InterruptedShell {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        self.commands.lock().unwrap().push(command.to_string());
        crate::interrupt::mark_interrupted();
        Ok(130)
    }

    fn run_structured(&self, chain: &FilledChain) -> Result<i32, ShellRunError> {
        self.run(&chain.segments[0].program)
    }
}

fn run(signal: LifecycleSignal, key: &str) -> (Vec<String>, Vec<Emitted>) {
    crate::interrupt::clear_for_tests();
    let config = parse_lifecycle_config(
        &json!({ key: [{ "shell": { "command": "interactive", "no_error": true } }, { "info": "reached" }] }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = InterruptedShell { commands: Mutex::new(Vec::new()) };
    let recorder = Recorder::default();
    let harness = Harness::default();
    ctx(signal, &fm, None, &engine, &shell, &recorder, &harness, Path::new("t.md")).execute_event(&config);
    crate::interrupt::clear_for_tests();
    (shell.commands.into_inner().unwrap(), recorder.events())
}

#[test]
fn an_interrupted_start_command_stops_the_next_action_despite_no_error() {
    let (commands, events) = run(LifecycleSignal::Start, "start");
    assert_eq!(commands, ["interactive"]);
    assert_eq!(events, Vec::<Emitted>::new(), "an action ran after the keyboard interrupt");
}

#[test]
fn cleanup_events_run_whole_after_an_interrupt() {
    for (signal, key) in [(LifecycleSignal::Cancelled, "cancelled"), (LifecycleSignal::Finalize, "finalize")] {
        let (commands, events) = run(signal, key);
        assert_eq!(commands, ["interactive"], "{key}");
        assert_eq!(events, [Emitted::Info("reached".to_string())], "{key}");
    }
}

#[test]
fn only_normal_work_stops_on_an_interrupt() {
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    for signal in LifecycleSignal::ALL {
        let context = ctx(signal, &fm, None, &engine, &shell, &recorder, &harness, Path::new("t.md"));
        let normal = matches!(signal, LifecycleSignal::Initialize | LifecycleSignal::Start | LifecycleSignal::Loop);
        assert_eq!(context.stops_on_interrupt(), normal, "{signal:?}");
    }
}
