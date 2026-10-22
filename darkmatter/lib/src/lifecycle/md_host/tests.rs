//! The `md` profile and driver against fakes: no process, no terminal.

use std::cell::Cell;
use std::path::Path;

use serde_json::{Value, json};

use super::*;
use crate::lifecycle::actions::{LifecycleAction, LifecycleActionKind, for_each_action};
use crate::lifecycle::executor::StackControl;
use crate::lifecycle::registry::{ActionType, ActionTypes, CoreEvent};
use crate::lifecycle::{collect_lifecycle_shell_commands, walk_stack};

fn parse(frontmatter: Value) -> Result<LifecycleConfig, LifecycleError> {
    parse_lifecycle(&frontmatter, Path::new("doc.md"), None)
}

fn parsed(frontmatter: Value) -> LifecycleConfig {
    parse(frontmatter.clone()).unwrap_or_else(|error| panic!("{frontmatter} parses under md: {error}"))
}

fn leaves(config: &LifecycleConfig, signal: LifecycleSignal) -> Vec<&LifecycleAction> {
    let mut leaves = Vec::new();
    for_each_action(config.stack(signal), &mut |action| leaves.push(action));
    leaves
}

fn ignored_verbs(config: &LifecycleConfig, signal: LifecycleSignal) -> Vec<String> {
    leaves(config, signal)
        .into_iter()
        .filter_map(|action| match &action.kind {
            LifecycleActionKind::Ignored(ignored) => Some(ignored.verb.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn another_hosts_verbs_and_unknown_verbs_are_kept_opaque_beside_running_ones() {
    let config = parsed(json!({
        "composed": [
            { "say": "{{ title }}" },
            { "retry": 3 },
            "break",
            { "frobnicate": { "deep": "$(touch marker)" } },
            { "info": "still runs" },
        ],
        "finalize": { "notify": "$(touch marker)", "stdout": "done" },
    }));
    assert_eq!(ignored_verbs(&config, LifecycleSignal::Start), ["say", "retry", "break", "frobnicate"]);
    assert!(matches!(leaves(&config, LifecycleSignal::Start)[4].kind, LifecycleActionKind::Communication(_)));
    assert_eq!(ignored_verbs(&config, LifecycleSignal::Finalize), ["notify"]);
    let mut surfaces = Vec::new();
    for signal in LifecycleSignal::ALL {
        walk_stack(config.stack(signal), &mut |path, _| surfaces.push(path.to_string()));
    }
    assert_eq!(surfaces, ["composed[4].info.text", "finalize.stdout.text"], "an ignored operand has no surface");
    assert!(collect_lifecycle_shell_commands(&config).is_empty(), "an ignored operand hides no shell");
}

#[test]
fn an_ignored_directive_neither_ends_its_list_nor_counts_as_a_second_directive() {
    let config = parsed(json!({
        "initialize": [{ "proxy": "@other.md", "skip": [] }],
        "composed": ["retry", { "info": "after" }],
    }));
    assert_eq!(ignored_verbs(&config, LifecycleSignal::Initialize), ["proxy"]);
    assert_eq!(leaves(&config, LifecycleSignal::Start).len(), 2);
}

#[test]
fn a_provided_verb_stays_strict_and_its_event_rules_hold() {
    for (frontmatter, expected) in [
        (json!({ "composed": [{ "info": { "bogus": "x" } }] }), "`info` requires a `text` parameter"),
        (json!({ "composed": [{ "stop": "now" }] }), "stop"),
        (json!({ "composed": [{ "when": "(", "then": [{ "say": "x" }] }] }), "`when` is not a valid expression"),
        (json!({ "composed": [{ "then": [{ "say": "x" }] }] }), "`then` requires `when`"),
        (json!({ "initialize": [{ "shell": "echo hi" }] }), "initialize"),
        (json!({ "initialize": [{ "when": "false", "then": [{ "set": { "x": "$(echo hi)" } }] }] }), "initialize"),
        (json!({ "composed": ["skip"] }), "skip"),
        (json!({ "cancelled": [{ "error": "x" }] }), "cancelled"),
        (json!({ "cancelled": ["skip"] }), "cancelled"),
        (json!({ "composed": "not a stack" }), "a lifecycle stack must be"),
    ] {
        let error = parse(frontmatter.clone()).expect_err("a malformed or forbidden provided action fails");
        assert!(error.to_string().contains(expected), "{frontmatter}: {error}");
    }
}

#[test]
fn a_misplaced_flow_verb_names_the_md_event() {
    let error = parse(json!({ "composed": ["skip"] })).unwrap_err();
    let LifecycleError::LifecycleActionPlacement { action, event, property, .. } = &error else {
        panic!("expected a placement error, got {error:?}");
    };
    assert_eq!((action.as_str(), event.as_str(), property.as_str()), ("skip", "composed", "composed[0].skip"));
}

#[test]
fn host_only_keys_and_child_style_keys_are_data() {
    let frontmatter = json!({ "start": "x", "success": [{ "say": 1 }], "blocked": 3, "failure": {}, "loop": "y" });
    assert!(!authors_lifecycle(&frontmatter));
    let config = parsed(frontmatter);
    assert!(config.is_empty());
    assert!(authors_lifecycle(&json!({ "finalize": [] })));
}

#[test]
fn the_strict_profile_still_runs_every_verb_and_rejects_unknown_ones() {
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: crate::lifecycle::registry::tests::agent_host(),
    };
    let config = super::super::parse_lifecycle_config_with(
        &json!({ "start": [{ "say": "hi" }, "retry"] }),
        Path::new("doc.md"),
        &options,
    )
    .unwrap();
    assert!(ignored_verbs(&config, LifecycleSignal::Start).is_empty());
    let error = super::super::parse_lifecycle_config_with(
        &json!({ "start": [{ "frobnicate": 1 }] }),
        Path::new("doc.md"),
        &options,
    )
    .unwrap_err();
    assert!(matches!(error, LifecycleError::LifecycleUnknownVerb { .. }), "{error:?}");
}

#[test]
fn initialize_never_admits_shell_whatever_a_host_declares() {
    let mut registry = EventRegistry::new("permissive");
    registry.declare_core_actions(CoreEvent::Initialize, ActionTypes::all()).unwrap();
    let initialize = registry.actions(crate::lifecycle::registry::EventId::core(CoreEvent::Initialize));
    assert!(!initialize.contains(ActionType::Shell));
    assert!(initialize.contains(ActionType::Expression));
    for event in [CoreEvent::Composed, CoreEvent::Failed, CoreEvent::Cancelled, CoreEvent::Finalize] {
        assert!(registry.actions(crate::lifecycle::registry::EventId::core(event)).contains(ActionType::Shell));
    }
}

#[test]
fn the_shell_audit_refuses_a_late_root_and_admits_document_data() {
    let bindings = MdShellAuditBindings;
    for (raw, root) in [("echo {{ err.msg }}", "err"), ("echo {{ timing.total_ms }}", "timing"), ("echo {{ current.os }}", "current")] {
        let findings = bindings.approval_findings(raw);
        let roots: Vec<_> = findings.iter().filter_map(super::super::preflight::unavailable_root).collect();
        assert_eq!(roots, [root], "{raw}");
    }
    assert!(bindings.approval_findings("echo {{ title }} {{ ctx.today }}").is_empty());
}

#[test]
fn lifecycle_sources_name_one_entry_per_chain_segment() {
    let config = parsed(json!({ "composed": [{ "shell": "echo a && printf b" }], "finalize": [{ "say": "$(echo hidden)" }] }));
    let sources = lifecycle_shell_sources(&config, Path::new("doc.md"));
    let normalized: Vec<_> = sources.entries.iter().map(|entry| entry.normalized.as_str()).collect();
    assert_eq!(normalized, ["echo a", "printf b"]);
}

// ── The driver ──────────────────────────────────────────────────────────

fn error_info(message: &str) -> LifecycleErrorInfo {
    LifecycleErrorInfo::from_action_failure_with(&MdDiagnostics, "test", message)
}

/// A scripted host: each event's outcome by name, the preparation and write
/// results, and an interrupt raised while a named step runs.
#[derive(Default)]
struct Scripted {
    outcomes: Vec<(&'static str, LifecycleEventOutcome)>,
    prepare_fails: Option<&'static str>,
    write_fails: bool,
    interrupt_during: Option<&'static str>,
    interrupted: Cell<bool>,
    steps: Vec<String>,
    shells: Vec<(&'static str, bool)>,
    errors: Vec<(&'static str, Option<String>)>,
}

impl Scripted {
    fn step(&mut self, name: &'static str) {
        self.steps.push(name.to_string());
        if self.interrupt_during == Some(name) {
            self.interrupted.set(true);
        }
    }
}

impl MdRunHost for Scripted {
    fn event(&mut self, event: MdEvent) -> LifecycleEventOutcome {
        self.step(event.name);
        self.shells.push((event.name, event.shells.is_ok()));
        self.errors.push((event.name, event.error.as_ref().map(|error| error.msg.clone())));
        self.outcomes.iter().find(|(name, _)| *name == event.name).map(|(_, outcome)| outcome.clone()).unwrap_or_default()
    }

    fn prepare(&mut self) -> Result<ApprovalEpoch, LifecycleErrorInfo> {
        self.step("prepare");
        match self.prepare_fails {
            Some(message) => Err(error_info(message)),
            None => Ok(ApprovalEpoch(1)),
        }
    }

    fn write(&mut self) -> Result<(), LifecycleErrorInfo> {
        self.step("write");
        if self.write_fails { Err(error_info("broken pipe")) } else { Ok(()) }
    }

    fn interrupted(&self) -> bool {
        self.interrupted.get()
    }
}

fn every_event() -> LifecycleConfig {
    parsed(json!({
        "initialize": [{ "info": "i" }],
        "composed": [{ "info": "c" }],
        "failed": [{ "info": "f" }],
        "cancelled": [{ "info": "x" }],
        "finalize": [{ "info": "z" }],
    }))
}

fn without_cancelled() -> LifecycleConfig {
    parsed(json!({ "initialize": [{ "info": "i" }], "composed": [{ "info": "c" }], "finalize": [{ "info": "z" }] }))
}

fn evaluation_error(message: &str) -> LifecycleEventOutcome {
    LifecycleEventOutcome { evaluation_error: Some(error_info(message)), ..Default::default() }
}

fn control(control: StackControl) -> LifecycleEventOutcome {
    LifecycleEventOutcome { control: Some(control), ..Default::default() }
}

#[test]
fn a_successful_run_fires_initialize_composed_write_finalize() {
    let mut host = Scripted::default();
    let end = drive(&mut host, &every_event());
    assert_eq!(host.steps, ["initialize", "prepare", "composed", "write", "finalize"]);
    assert_eq!((end.finish, end.exit_code(), end.written, end.error.clone()), (Finish::Succeeded, 0, true, None));
    assert_eq!(host.shells, [("initialize", false), ("composed", true), ("finalize", true)]);
}

#[test]
fn a_preparation_failure_runs_failed_then_finalize_with_shells_locked() {
    let mut host = Scripted { prepare_fails: Some("missing include"), ..Default::default() };
    let end = drive(&mut host, &every_event());
    assert_eq!(host.steps, ["initialize", "prepare", "failed", "finalize"]);
    assert_eq!((end.finish, end.exit_code(), end.written), (Finish::Failed, 1, false));
    assert_eq!(end.error.map(|error| error.msg), Some("missing include".to_string()));
    assert_eq!(host.shells, [("initialize", false), ("failed", false), ("finalize", false)]);
    assert_eq!(host.errors[1..], [("failed", Some("missing include".into())), ("finalize", Some("missing include".into()))]);
}

#[test]
fn an_initialize_error_runs_failed_then_finalize_and_never_prepares() {
    for outcome in [evaluation_error("bad guard"), control(StackControl::Error { reason: Some("no changelog".into()) })] {
        let mut host = Scripted { outcomes: vec![("initialize", outcome)], ..Default::default() };
        let end = drive(&mut host, &every_event());
        assert_eq!(host.steps, ["initialize", "failed", "finalize"]);
        assert_eq!(end.exit_code(), 1);
        assert!(end.error.is_some());
    }
}

#[test]
fn skip_writes_nothing_runs_no_finalize_and_succeeds() {
    let mut host = Scripted { outcomes: vec![("initialize", control(StackControl::Skip))], ..Default::default() };
    let end = drive(&mut host, &every_event());
    assert_eq!(host.steps, ["initialize"]);
    assert_eq!((end.finish, end.exit_code(), end.written), (Finish::Skipped, 0, false));
}

#[test]
fn a_composed_error_runs_finalize_without_failed_and_writes_nothing() {
    for outcome in [evaluation_error("bad span"), control(StackControl::Error { reason: Some("refused".into()) })] {
        let mut host = Scripted { outcomes: vec![("composed", outcome)], ..Default::default() };
        let end = drive(&mut host, &every_event());
        assert_eq!(host.steps, ["initialize", "prepare", "composed", "finalize"]);
        assert_eq!((end.finish, end.exit_code(), end.written), (Finish::Failed, 1, false));
    }
}

#[test]
fn a_failed_write_runs_finalize_without_failed() {
    let mut host = Scripted { write_fails: true, ..Default::default() };
    let end = drive(&mut host, &every_event());
    assert_eq!(host.steps, ["initialize", "prepare", "composed", "write", "finalize"]);
    assert_eq!((end.finish, end.exit_code(), end.written), (Finish::Failed, 1, false));
    assert_eq!(host.errors.last(), Some(&("finalize", Some("broken pipe".to_string()))));
}

#[test]
fn stop_ends_only_its_stack() {
    for event in ["initialize", "composed"] {
        let mut host = Scripted { outcomes: vec![(event, control(StackControl::Stop))], ..Default::default() };
        let end = drive(&mut host, &every_event());
        assert_eq!(host.steps, ["initialize", "prepare", "composed", "write", "finalize"], "stop in {event}");
        assert_eq!(end.exit_code(), 0);
    }
}

#[test]
fn an_error_in_cleanup_replaces_the_selected_error_without_recursive_cleanup() {
    let mut host = Scripted {
        prepare_fails: Some("denied"),
        outcomes: vec![("failed", evaluation_error("cleanup broke"))],
        ..Default::default()
    };
    let end = drive(&mut host, &every_event());
    assert_eq!(host.steps, ["initialize", "prepare", "failed", "finalize"]);
    assert_eq!(end.error.map(|error| error.msg), Some("cleanup broke".to_string()));

    let mut host = Scripted { outcomes: vec![("finalize", evaluation_error("finalize broke"))], ..Default::default() };
    let end = drive(&mut host, &every_event());
    assert_eq!(host.steps, ["initialize", "prepare", "composed", "write", "finalize"]);
    assert_eq!((end.exit_code(), end.error.map(|error| error.msg)), (1, Some("finalize broke".to_string())));
}

#[test]
fn an_interrupt_runs_cancelled_then_finalize_and_drops_the_interrupted_work() {
    for (during, before) in [
        ("initialize", vec!["initialize"]),
        ("prepare", vec!["initialize", "prepare"]),
        ("composed", vec!["initialize", "prepare", "composed"]),
        ("write", vec!["initialize", "prepare", "composed", "write"]),
    ] {
        let mut host = Scripted { interrupt_during: Some(during), ..Default::default() };
        let end = drive(&mut host, &every_event());
        let mut expected = before.clone();
        expected.extend(["cancelled", "finalize"]);
        assert_eq!(host.steps, expected, "interrupted during {during}");
        assert_eq!((end.finish, end.exit_code()), (Finish::Cancelled, 130), "{during}");
        assert_eq!(end.late_replies, 1, "the interrupted {during} reply is refused as outdated");
        assert_eq!(end.written, during == "write", "{during}: output already written is not rolled back");
        assert_eq!(host.errors.last(), Some(&("finalize", Some("interrupted by the user".to_string()))));
    }
}

#[test]
fn an_interrupt_without_a_cancelled_stack_runs_finalize_alone() {
    let mut host = Scripted { interrupt_during: Some("prepare"), ..Default::default() };
    let end = drive(&mut host, &without_cancelled());
    assert_eq!(host.steps, ["initialize", "prepare", "finalize"]);
    assert_eq!(end.exit_code(), 130);
}

#[test]
fn cancellation_cleanup_before_shells_unlock_cannot_run_a_shell() {
    let mut host = Scripted { interrupt_during: Some("initialize"), ..Default::default() };
    drive(&mut host, &every_event());
    assert_eq!(host.shells, [("initialize", false), ("cancelled", false), ("finalize", false)]);
}

#[test]
fn an_interrupt_during_finalize_lets_it_finish_once() {
    let mut host = Scripted { interrupt_during: Some("finalize"), ..Default::default() };
    let end = drive(&mut host, &every_event());
    assert_eq!(host.steps, ["initialize", "prepare", "composed", "write", "finalize"]);
    assert_eq!(end.late_replies, 0);
}

// ── One event through the executor ──────────────────────────────────────

#[derive(Default)]
struct Lines(std::sync::Mutex<Vec<String>>);

impl Lines {
    fn push(&self, line: String) {
        self.0.lock().unwrap().push(line);
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

impl LifecycleEffects for Lines {
    fn stdout(&self, text: &str) {
        self.push(format!("stdout {text}"));
    }
    fn stderr(&self, _: LifecycleSignal, text: &str) {
        self.push(format!("stderr {text}"));
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
        self.push(format!("notify {title}"));
    }
}

impl CommandOutput for Lines {
    fn stdout(&self, text: &str) {
        self.push(format!("command stdout {text}"));
    }
    fn stderr(&self, text: &str) {
        self.push(format!("command stderr {text}"));
    }
}

/// Evaluates `signal`'s stack of `config` with an interrupt flag of
/// `interrupted`, and returns what it communicated.
fn evaluate_event(config: &LifecycleConfig, signal: LifecycleSignal, interrupted: bool) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let request = crate::markdown::compose::test_request(crate::markdown::compose::ComposeOptions::new());
    let lines = Lines::default();
    let frontmatter = Map::new();
    let live = Mutex::new(Map::new());
    let state = MdRuntimeState::default();
    let approved = ShellPreApprovals::new();
    let blacklist = ShellRuleSet::default();
    let engine = EffectEngine::builder().mutation_root(dir.path()).auto_rehash(false).build();
    let timing = LifecycleTiming::default();
    let probe = move || interrupted;
    let context = MdStackContext {
        request: &request,
        source_path: Path::new("doc.md"),
        frontmatter: &frontmatter,
        live_frontmatter: &live,
        state: &state,
        approved: &approved,
        user_blacklist: &blacklist,
        effect_engine: &engine,
        effects: &lines,
        output: &lines,
        timing: &timing,
        interrupted: &probe,
    };
    let event = MdEvent { signal, name: "test", error: None, shells: Err(ShellRefusal::Locked) };
    evaluate(&context, config, &event);
    lines.take()
}

#[test]
fn each_reached_ignored_action_warns_once_naming_it_and_its_event() {
    let config = parsed(json!({
        "composed": [
            { "say": "{{ title }}" },
            { "when": "true", "then": [{ "retry": 1 }], "else": [{ "notify": "x" }] },
            { "info": "ran" },
        ],
    }));
    assert_eq!(
        evaluate_event(&config, LifecycleSignal::Start, false),
        [
            "warn `md` does not run `say`; skipped it in `composed`",
            "warn `md` does not run `retry`; skipped it in `composed`",
            "info ran",
        ]
    );
}

#[test]
fn an_interrupt_ends_normal_work_but_never_cleanup() {
    let config = parsed(json!({
        "composed": [{ "info": "composed" }],
        "finalize": [{ "info": "finalize-1" }, { "info": "finalize-2" }],
        "cancelled": [{ "info": "cancelled" }],
    }));
    assert!(evaluate_event(&config, LifecycleSignal::Start, true).is_empty());
    assert_eq!(evaluate_event(&config, LifecycleSignal::Finalize, true), ["info finalize-1", "info finalize-2"]);
    assert_eq!(evaluate_event(&config, LifecycleSignal::Cancelled, true), ["info cancelled"]);
}

#[test]
fn a_shell_before_preflight_is_refused_without_running() {
    let config = parsed(json!({ "finalize": [{ "shell": "echo never" }] }));
    let lines = evaluate_event(&config, LifecycleSignal::Finalize, false);
    assert!(lines.iter().all(|line| !line.contains("command stdout")), "{lines:?}");
}
