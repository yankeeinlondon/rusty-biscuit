//! The executor against a fake host: no processes, no host model, only the
//! seams.

use std::cell::RefCell;
use std::sync::Mutex;

use serde_json::json;

use super::*;
use crate::lifecycle::envelope::DiagnosticSnapshot;
use crate::lifecycle::services::LifecycleService;
use crate::lifecycle::{LifecycleParseOptions, UnknownVerbPolicy, parse_lifecycle_config_with};
use crate::markdown::compose::shell_expansion::template::FilledChain;
use crate::markdown::compose::{ComposeOptions, ComposeRequest};

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
struct Captured(Mutex<Vec<String>>);

impl Captured {
    fn lines(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn push(&self, line: String) {
        self.0.lock().unwrap().push(line);
    }
}

impl LifecycleEffects for Captured {
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

/// Records every command; reaching it at all is what a refusal must prevent.
#[derive(Default)]
struct RecordingShell(Mutex<Vec<String>>);

impl ShellRunner for RecordingShell {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        self.0.lock().unwrap().push(command.to_string());
        Ok(0)
    }
    fn run_structured(&self, _: &FilledChain) -> Result<i32, ShellRunError> {
        Ok(0)
    }
}

/// A host with no globals, an in-memory runtime state, and no loop or `prep`
/// runner; failures it classifies are labeled by variant.
#[derive(Default)]
struct FakeHost {
    committed: RefCell<Map<String, Value>>,
    failures: RefCell<Vec<String>>,
}

impl LifecycleHost for FakeHost {
    fn registry(&self) -> &crate::lifecycle::registry::EventRegistry {
        crate::lifecycle::registry::tests::agent_host()
    }
    fn bindings(&self) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        (Arc::new(BindingView::baseline()), HashMap::new())
    }
    fn name_coercion_keys(&self) -> Vec<String> {
        Vec::new()
    }
    fn resolution_context(
        &self,
        source_path: &Path,
        prepared_context: Option<&ComposeContext>,
        file_resolution_context: &biscuit_file::FileResolutionContext,
        _: Option<&Path>,
    ) -> Result<ResolutionContext, ContextBuildError> {
        let context = prepared_context.cloned().unwrap_or_else(ComposeContext::capture_minimal);
        let options = ComposeOptions::new_with_context(context).with_source_file(source_path);
        ComposeRequest::with_context(options, file_resolution_context.clone())
            .map(|request| request.local_expression_resolution_context())
    }
    fn capture_overlay_context(&self, _: &Path, _: &str) -> ComposeContext {
        ComposeContext::capture_minimal()
    }
    fn persisted_data(&self, value: &Value) -> Value {
        value.clone()
    }
    fn commit_set(
        &self,
        _: &EffectEngine,
        updates: &IndexMap<String, Value>,
        prior_base: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, Box<dyn StdError + Send + Sync>> {
        let mut committed = self.committed.borrow_mut();
        let prior = updates
            .keys()
            .map(|key| (key.clone(), prior_base.get(key).cloned().unwrap_or(Value::Null)))
            .collect();
        committed.extend(updates.iter().map(|(key, value)| (key.clone(), value.clone())));
        Ok(prior)
    }
    fn in_loop(&self) -> bool {
        false
    }
    fn record_break(&self, _: Option<String>, _: u8) {
        unreachable!("the fake host is never in a loop");
    }
    fn prep(&self, _: &PrepRequest<'_>) -> Result<(), PrepError> {
        Err(PrepError::Unavailable)
    }
    fn user_blacklist(&self, _: Option<&Path>) -> crate::markdown::compose::shell_expansion::ShellRuleSet {
        Default::default()
    }
    fn interrupted(&self) -> bool {
        false
    }
    fn is_known_effect(&self, _: &str) -> bool {
        true
    }
    fn failure(&self, failure: LifecycleRunFailure) -> LifecycleErrorInfo {
        let label = format!("{failure:?}");
        let variant = label.split([' ', '{', '(']).next().unwrap_or_default().to_string();
        self.failures.borrow_mut().push(variant.clone());
        LifecycleErrorInfo::from_action_failure_with(&NoCodes, variant, label)
    }
}

fn parse(document: Value) -> LifecycleConfig {
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: crate::lifecycle::registry::tests::agent_host(),
    };
    parse_lifecycle_config_with(&document, Path::new("doc.md"), &options).unwrap()
}

struct Fixture {
    _dir: tempfile::TempDir,
    engine: EffectEngine,
    files: biscuit_file::FileResolutionContext,
    context: ComposeContext,
    host: FakeHost,
    effects: Captured,
    shell: RecordingShell,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let engine = EffectEngine::builder().mutation_root(dir.path()).auto_rehash(false).build();
        let files = biscuit_file::FileResolutionContext::new(dir.path());
        Self {
            _dir: dir,
            engine,
            files,
            context: ComposeContext::capture_minimal(),
            host: FakeHost::default(),
            effects: Captured::default(),
            shell: RecordingShell::default(),
        }
    }

    fn inputs<'a>(&'a self, signal: LifecycleSignal, frontmatter: &'a Map<String, Value>) -> StackInputs<'a> {
        StackInputs {
            signal,
            frontmatter,
            live_frontmatter: None,
            current: None,
            base_dir: None,
            ctx_base_dir: None,
            prepared_context: Some(&self.context),
            file_resolution_context: &self.files,
            effect_engine: &self.engine,
            shell_runner: &self.shell,
            source_path: Path::new("doc.md"),
            host: &self.host,
        }
    }

    fn run(&self, signal: LifecycleSignal, frontmatter: Value, document: Value) -> LifecycleEventOutcome {
        let config = parse(document);
        let fm = frontmatter.as_object().cloned().unwrap_or_default();
        StackExecutor::with_services(self.inputs(signal, &fm), &self.effects, &NoCodes).execute_event(&config)
    }
}

#[test]
fn a_set_value_is_read_by_the_next_action_and_committed_to_the_host() {
    let fixture = Fixture::new();

    let outcome = fixture.run(
        LifecycleSignal::Start,
        json!({}),
        json!({ "start": [{ "set": { "count": 3 } }, { "info": "count={{ count }}" }] }),
    );

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(fixture.effects.lines(), ["info count=3"]);
    assert_eq!(fixture.host.committed.borrow().get("count"), Some(&json!(3)));
}

#[test]
fn a_false_guard_runs_only_the_else_branch() {
    let fixture = Fixture::new();

    fixture.run(
        LifecycleSignal::Success,
        json!({ "flag": "no" }),
        json!({ "success": [{ "when": "flag == 'yes'", "then": [{ "say": "then" }], "else": [{ "say": "else" }] }] }),
    );

    assert_eq!(fixture.effects.lines(), ["speech else"]);
}

#[test]
fn stop_ends_the_event_before_the_next_action() {
    let fixture = Fixture::new();

    let outcome = fixture.run(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "info": "before" }, { "when": "true", "then": ["stop"] }, { "info": "after" }] }),
    );

    assert_eq!(outcome.control, Some(StackControl::Stop));
    assert_eq!(fixture.effects.lines(), ["info before"]);
}

#[test]
fn error_carries_its_evaluated_reason() {
    let fixture = Fixture::new();

    let outcome = fixture.run(
        LifecycleSignal::Success,
        json!({ "why": "manual" }),
        json!({ "success": [{ "error": "{{ why }} failure" }] }),
    );

    assert_eq!(outcome.control, Some(StackControl::Error { reason: Some("manual failure".into()) }));
}

/// The parser refuses an authored `initialize` shell, so the stack is parsed
/// under `start` and run as `initialize`, as a programmatic caller could.
#[test]
fn a_shell_action_at_initialize_is_refused_before_the_runner_sees_it() {
    let fixture = Fixture::new();
    let fm = Map::new();
    let config = parse(json!({ "start": [{ "shell": "echo hi" }] }));

    let outcome = StackExecutor::with_services(fixture.inputs(LifecycleSignal::Initialize, &fm), &fixture.effects, &NoCodes)
        .execute_action_stack(config.stack(LifecycleSignal::Start));

    assert!(fixture.shell.0.lock().unwrap().is_empty(), "no command reached the runner");
    let error = outcome.evaluation_error.expect("an unsuppressible evaluation error");
    assert_eq!(error.msg, ShellRunError::BeforePreflight.to_string());
    assert_eq!(error.property.as_deref(), Some("start[0].shell"));
}

#[test]
fn a_disabled_runner_refuses_a_shell_action_after_start() {
    let fixture = Fixture::new();
    let fm = Map::new();
    let inputs = StackInputs {
        shell_runner: &DisabledShellRunner,
        ..fixture.inputs(LifecycleSignal::Success, &fm)
    };
    let config = parse(json!({ "success": [{ "shell": "echo hi" }] }));

    let outcome = StackExecutor::with_services(inputs, &fixture.effects, &NoCodes).execute_event(&config);

    assert!(outcome.evaluation_error.is_some(), "a prohibition is never a suppressible dispatch error");
}

#[test]
fn break_outside_a_loop_is_classified_by_the_host() {
    let fixture = Fixture::new();

    let outcome = fixture.run(LifecycleSignal::Success, json!({}), json!({ "success": ["break"] }));

    let error = outcome.evaluation_error.expect("break without a loop is an evaluation error");
    assert_eq!(error.reason, LifecycleEvaluationReason::BreakWithoutLoop);
    assert_eq!(*fixture.host.failures.borrow(), ["BreakWithoutLoop"]);
}

#[test]
fn a_missing_service_is_a_typed_error() {
    let fixture = Fixture::new();
    let fm = Map::new();

    let no_effects = StackExecutor::new(fixture.inputs(LifecycleSignal::Start, &fm), LifecycleServices::default());
    let no_catalog = StackExecutor::new(
        fixture.inputs(LifecycleSignal::Start, &fm),
        LifecycleServices {
            effects: Some(&fixture.effects),
            diagnostics: None,
        },
    );

    assert_eq!(no_effects.err(), Some(MissingServiceError { service: LifecycleService::Effects }));
    assert_eq!(no_catalog.err(), Some(MissingServiceError { service: LifecycleService::Diagnostics }));
}

/// `length`'s result for one action operand, evaluated against `frontmatter`.
fn action_length(frontmatter: Value, operand: Expr) -> Value {
    let fixture = Fixture::new();
    let fm = frontmatter.as_object().cloned().unwrap_or_default();
    StackExecutor::with_services(fixture.inputs(LifecycleSignal::Success, &fm), &fixture.effects, &NoCodes)
        .invoke_expression_function("length", &[operand], &fm)
        .expect("length evaluates")
}

#[test]
fn an_expression_function_receives_a_template_operand_rendered_once() {
    // The raw template text is 16 characters; its rendering `n=3` is 3.
    let operand = Expr::StringLiteral("n={{ fm.count }}".to_string());

    assert_eq!(action_length(json!({ "count": 3 }), operand), json!(3));
}

#[test]
fn an_expression_function_receives_a_whole_value_operand_as_typed_data() {
    let items = crate::markdown::compose::expression::parse("fm.items").unwrap();
    assert_eq!(action_length(json!({ "items": ["a", "b"] }), items), json!(2));

    // Data is never scanned again: the value's own braces are counted.
    let note = crate::markdown::compose::expression::parse("fm.note").unwrap();
    assert_eq!(action_length(json!({ "note": "{{ 1 }}" }), note), json!(7));
}
