//! The guard's accounting against a fake host: one terminal outcome, one
//! `finalize`, the shell lock, the success downgrade, and the drop safety net.

use std::cell::RefCell;
use std::error::Error as StdError;
use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::json;

use super::*;
use crate::lifecycle::envelope::DiagnosticSnapshot;
use crate::lifecycle::{LifecycleParseOptions, UnknownVerbPolicy, parse_lifecycle_config_with};

struct NoCodes;

impl DiagnosticCatalog for NoCodes {
    fn select(&self, _: &(dyn StdError + 'static)) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_code(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_error_kind(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
}

#[derive(Default)]
struct Recorded(Mutex<Vec<String>>);

impl Recorded {
    fn push(&self, line: String) {
        self.0.lock().unwrap().push(line);
    }
    fn lines(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl LifecycleEffects for Recorded {
    fn stdout(&self, text: &str) {
        self.push(format!("stdout {text}"));
    }
    fn stderr(&self, signal: LifecycleSignal, text: &str) {
        self.push(format!("stderr {} {text}", signal.property_name()));
    }
    fn info(&self, text: &str) {
        self.push(format!("info {text}"));
    }
    fn warn(&self, text: &str) {
        self.push(format!("warn {text}"));
    }
    fn success(&self, text: &str) {
        self.push(format!("success {text}"));
    }
    fn message(&self, text: &str) {
        self.push(format!("message {text}"));
    }
    fn speech(&self, text: &str) {
        self.push(format!("speech {text}"));
    }
    fn effect(&self, name: &str) {
        self.push(format!("effect {name}"));
    }
    fn notification(&self, title: &str) {
        self.push(format!("notification {title}"));
    }
}

struct FakeHost<'a> {
    effects: &'a Recorded,
    interrupted: bool,
}

impl LifecycleRunHost for FakeHost<'_> {
    type TerminalScope = ();

    fn source_path(&self) -> &Path {
        Path::new("doc.md")
    }
    fn prepared_context(&self) -> Option<&ComposeContext> {
        None
    }
    fn effects(&self) -> &dyn LifecycleEffects {
        self.effects
    }
    fn diagnostics(&self) -> &dyn DiagnosticCatalog {
        &NoCodes
    }
    fn interrupted(&self) -> bool {
        self.interrupted
    }
    fn enter_terminal_event(&self) {}
    fn failure(&self, failure: LifecycleRunFailure) -> LifecycleErrorInfo {
        LifecycleErrorInfo::from_action_failure_with(&NoCodes, "failure", format!("{failure:?}"))
    }
}

/// Records each event the guard runs and whether its shells were locked.
#[derive(Default)]
struct FakeStack {
    runs: RefCell<Vec<(LifecycleSignal, bool)>>,
}

impl GuardedEventStack for FakeStack {
    fn run_guarded(&self, event: GuardedEvent<'_>) -> LifecycleEventOutcome {
        let locked = event
            .shell_runner
            .is_some_and(|runner| runner.ensure_permitted().is_err());
        self.runs.borrow_mut().push((event.signal, locked));
        LifecycleEventOutcome::default()
    }
}

impl FakeStack {
    fn runs(&self) -> Vec<(LifecycleSignal, bool)> {
        self.runs.borrow().clone()
    }
}

fn config(frontmatter: serde_json::Value) -> LifecycleConfig {
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
    };
    parse_lifecycle_config_with(&frontmatter, &PathBuf::from("doc.md"), &options).unwrap()
}

fn guard<'a>(config: &'a LifecycleConfig, effects: &'a Recorded) -> LifecycleRunGuard<'a, FakeHost<'a>> {
    LifecycleRunGuard::with_host(config, FakeHost { effects, interrupted: false })
}

#[test]
fn the_terminal_slot_holds_the_first_outcome_only() {
    let config = LifecycleConfig::default();
    let effects = Recorded::default();
    let stack = FakeStack::default();
    let mut guard = guard(&config, &effects);

    guard.execute_event(LifecycleSignal::Start, &stack);
    guard.execute_event(LifecycleSignal::Success, &stack);
    guard.execute_event(LifecycleSignal::Failure, &stack);
    guard.emit_terminal(LifecycleSignal::Blocked);

    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    let signals: Vec<_> = stack.runs().into_iter().map(|(signal, _)| signal).collect();
    assert_eq!(signals, [LifecycleSignal::Start, LifecycleSignal::Success]);
}

#[test]
fn finalize_runs_once_and_only_after_a_terminal_event() {
    let config = LifecycleConfig::default();
    let effects = Recorded::default();
    let stack = FakeStack::default();
    let mut guard = guard(&config, &effects);

    assert!(!guard.record_event_emission(LifecycleSignal::Finalize), "no terminal event yet");
    guard.execute_event(LifecycleSignal::Failure, &stack);
    guard.execute_event(LifecycleSignal::Finalize, &stack);
    guard.execute_event(LifecycleSignal::Finalize, &stack);

    assert!(guard.finalize_emitted());
    let finalizes = stack.runs().iter().filter(|(signal, _)| *signal == LifecycleSignal::Finalize).count();
    assert_eq!(finalizes, 1);
}

#[test]
fn shells_are_refused_before_start_and_relocked_after_a_proxy_reset() {
    let config = LifecycleConfig::default();
    let effects = Recorded::default();
    let stack = FakeStack::default();
    let mut guard = guard(&config, &effects);

    guard.execute_event(LifecycleSignal::Initialize, &stack);
    guard.execute_event(LifecycleSignal::Start, &stack);
    guard.execute_event(LifecycleSignal::Loop, &stack);
    guard.reset_for_proxy();
    guard.execute_event(LifecycleSignal::Initialize, &stack);
    guard.execute_event(LifecycleSignal::Blocked, &stack);

    assert_eq!(
        stack.runs(),
        [
            (LifecycleSignal::Initialize, true),
            (LifecycleSignal::Start, false),
            (LifecycleSignal::Loop, false),
            (LifecycleSignal::Initialize, true),
            (LifecycleSignal::Blocked, true),
        ]
    );
}

#[test]
fn an_explicit_error_downgrades_a_recorded_success_to_failure_once() {
    let config = LifecycleConfig::default();
    let effects = Recorded::default();
    let stack = FakeStack::default();
    let mut guard = guard(&config, &effects);

    guard.execute_event(LifecycleSignal::Success, &stack);
    assert!(guard.redesignate_terminal_to_failure());
    assert!(!guard.redesignate_terminal_to_failure(), "failure is authoritative");

    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(!guard.record_event_emission(LifecycleSignal::Failure), "the slot stays taken");
    assert!(guard.record_event_emission(LifecycleSignal::Finalize), "finalize still fires");
}

#[test]
fn dropping_a_started_run_emits_the_safety_net_terminal_stderr() {
    let config = config(json!({
        "blocked": [{ "stderr": "blocked" }, { "message": "sent" }],
        "failure": [{ "stderr": "failed" }],
    }));
    let effects = Recorded::default();
    {
        let mut guard = guard(&config, &effects);
        guard.emit_start_once();
    }
    {
        let mut guard =
            LifecycleRunGuard::with_host(&config, FakeHost { effects: &effects, interrupted: true });
        guard.emit_start_once();
        guard.mark_provider_launched();
    }
    {
        let mut guard = guard(&config, &effects);
        guard.emit_start_once();
        guard.defuse();
    }

    assert_eq!(effects.lines(), ["stderr blocked blocked", "message sent", "stderr failure failed"]);
}

#[test]
fn cancellation_cleanup_reports_a_recovery_instead_of_running_it() {
    let config = LifecycleConfig::default();
    let effects = Recorded::default();
    let guard = guard(&config, &effects);
    let outcome = LifecycleEventOutcome {
        control: Some(StackControl::Retry {
            max_attempts: 1,
            backoff: crate::lifecycle::actions::RetryBackoff::Fixed,
            delay: "0s".to_string(),
            overlay: Default::default(),
        }),
        ..LifecycleEventOutcome::default()
    };

    let normal = guard.cleanup_warnings(EvaluationMode::NormalRecovery, &outcome);
    let cancelled = guard.cleanup_warnings(EvaluationMode::CancellationCleanup, &outcome);

    assert!(normal.is_empty());
    assert_eq!(cancelled.len(), 1);
    assert!(cancelled[0].msg.contains("CancellationRecovery"), "{}", cancelled[0].msg);
}
