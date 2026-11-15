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
    /// `None` is a host that cannot run `edit`, the trait's default.
    editor: Option<FakeEditor>,
    side_effects_attempted: std::cell::Cell<usize>,
    interrupted: bool,
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
        ComposeRequest::local_expression_context(options, file_resolution_context)
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
        prior_base: &SharedFrontmatter,
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
    fn shell_exclusions(
        &self,
    ) -> Result<crate::markdown::compose::shell_expansion::ShellRuleSet, crate::config::load::ConfigError> {
        Ok(Default::default())
    }
    fn interrupted(&self) -> bool {
        self.interrupted
    }
    fn side_effect_attempted(&self) {
        self.side_effects_attempted.set(self.side_effects_attempted.get() + 1);
    }
    fn editor(&self) -> Option<&dyn crate::editor::EditorService> {
        self.editor.as_ref().map(|editor| editor as &dyn crate::editor::EditorService)
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
    parse_completed(document, None)
}

/// `document` parsed as a host parses a schema-completed value, with
/// `authored` as its authored source (`None`: `document` is authored).
fn parse_completed(document: Value, authored: Option<&Value>) -> LifecycleConfig {
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        authored_values: authored,
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

    fn inputs<'a>(&'a self, signal: LifecycleSignal, frontmatter: &'a SharedFrontmatter) -> StackInputs<'a> {
        StackInputs {
            signal,
            frontmatter,
            live_frontmatter: None,
            current: None,
            base_dir: None,
            ctx_base_dir: None,
            prepared_context: Some(&self.context),
            file_resolution_context: &self.files,
            config_snapshot: None,
            effect_engine: &self.engine,
            shell_runner: &self.shell,
            source_path: Path::new("doc.md"),
            host: &self.host,
        }
    }

    fn run(&self, signal: LifecycleSignal, frontmatter: Value, document: Value) -> LifecycleEventOutcome {
        let config = parse(document);
        let fm: SharedFrontmatter = frontmatter.as_object().cloned().unwrap_or_default().into();
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
    let fm = SharedFrontmatter::default();
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
    let fm = SharedFrontmatter::default();
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
    let fm = SharedFrontmatter::default();

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
    let fm: SharedFrontmatter = frontmatter.as_object().cloned().unwrap_or_default().into();
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

#[test]
fn private_live_cells_over_one_seed_share_its_definitions_and_keep_writes_apart() {
    let fixture = Fixture::new();
    let steps: Vec<Value> = (0..50).map(|index| json!({ "id": format!("s{index}"), "shell": "echo" })).collect();
    let fm = json!({ "title": "T", "sequence": steps }).as_object().cloned().unwrap();
    let seed = SharedFrontmatter::from(fm.clone());
    let left = Mutex::new(seed.clone());
    let right = Mutex::new(seed.clone());
    let config = parse(json!({ "start": [
        { "set": { "phase": "left" } },
        { "info": "phase={{ phase }} steps={{ length(sequence) }}" },
    ] }));

    let inputs = StackInputs { live_frontmatter: Some(&left), ..fixture.inputs(LifecycleSignal::Start, &seed) };
    let outcome = StackExecutor::with_services(inputs, &fixture.effects, &NoCodes).execute_event(&config);

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(fixture.effects.lines(), ["info phase=left steps=50"]);
    let left = left.into_inner().unwrap();
    assert_eq!(left.get("phase"), Some(&json!("left")));
    assert!(left.shares_base_with(&seed), "the write must not copy the shared definitions");
    assert_eq!(left.owned_len(), 1);
    let right = right.into_inner().unwrap();
    assert_eq!(right.get("phase"), None);
    assert_eq!(right, seed);
}

/// `function`'s result for `operands`, evaluated against `frontmatter` in a
/// fixture whose private directory holds no data files.
fn action_call(function: &str, frontmatter: Value, operands: Vec<Expr>) -> Result<Value, LifecycleExprError> {
    let fixture = Fixture::new();
    let fm: SharedFrontmatter = frontmatter.as_object().cloned().unwrap_or_default().into();
    let source = fixture._dir.path().join("doc.md");
    let inputs = StackInputs { source_path: &source, ..fixture.inputs(LifecycleSignal::Success, &fm) };
    StackExecutor::with_services(inputs, &fixture.effects, &NoCodes).invoke_expression_function(function, &operands, &fm)
}

fn operand(source: &str) -> Expr {
    crate::markdown::compose::expression::parse(source).unwrap()
}

const MISSING_DATA: &str = "load_data('absent.yaml', 'string[]')";

#[test]
fn try_action_returns_the_selected_value_not_a_boolean() {
    let fm = json!({ "items": ["a", "b"], "count": 3 });
    // A template operand routes the call through the operand path.
    let template = Expr::StringLiteral("n={{ fm.count }}".to_string());
    assert_eq!(action_call("try", fm.clone(), vec![operand("fm.items"), template.clone()]).unwrap(), json!(["a", "b"]));
    assert_eq!(action_call("try", fm.clone(), vec![operand("0"), template]).unwrap(), json!(0), "a falsy success");
    // Without a template operand the shared evaluator runs it, alike.
    assert_eq!(action_call("try", fm, vec![operand(MISSING_DATA), operand("['x']")]).unwrap(), json!(["x"]));
}

/// A template operand sends the call down the value-only path, which cannot
/// run a callback function: the rejection is a contract violation.
#[test]
fn callback_function_with_template_operand_is_a_contract_violation() {
    let template = Expr::StringLiteral("{{ fm.count }}".to_string());
    let error = action_call("map", json!({ "items": [1], "count": 3 }), vec![operand("fm.items"), template]).unwrap_err();
    let LifecycleExprError::Evaluate(error) = error else { panic!("an evaluation error: {error:?}") };
    assert!(matches!(*error, ExpressionError::ContractViolation { ref function, .. } if function == "map"), "{error:?}");
}

#[test]
fn try_template_operand_fallback_is_interpolated() {
    let template = Expr::StringLiteral("n={{ fm.count }}".to_string());
    let value = action_call("try", json!({ "count": 3 }), vec![operand(MISSING_DATA), template]).unwrap();
    assert_eq!(value, json!("n=3"));
}

#[test]
fn unused_template_operand_is_not_interpolated() {
    // Interpolating this template fails, so a success proves it never ran.
    let broken = Expr::StringLiteral("{{ no_such_function() }}".to_string());
    assert!(action_call("length", json!({}), vec![broken.clone()]).is_err(), "control: interpolating it fails");
    let value = action_call("try", json!({ "items": ["a"] }), vec![operand("fm.items"), broken]).unwrap();
    assert_eq!(value, json!(["a"]));
}

#[test]
fn template_operand_compose_failure_is_not_recovered() {
    // The read inside the template fails as subtree composition, not as an
    // expression failure of `try`'s own operand, so it propagates.
    let template = Expr::StringLiteral(format!("{{{{ {MISSING_DATA} }}}}"));
    let error = action_call("try", json!({}), vec![template, operand("'fallback'")]).unwrap_err();
    assert!(matches!(error, LifecycleExprError::Compose(_)), "{error:?}");
}

// ── edit ─────────────────────────────────────────────────────────────────

/// What a [`FakeEditor`] saw for one request.
#[derive(Debug, Clone, PartialEq)]
struct SeenEdit {
    target: PathBuf,
    mutation_root: PathBuf,
    create_new: bool,
    gui: Option<bool>,
    cancelled_at_start: bool,
}

/// An editor service that records each request and answers with a canned
/// outcome; `cancel` cancels the worker's token while "open".
#[derive(Default)]
struct FakeEditor {
    seen: Mutex<Vec<SeenEdit>>,
    attempted: bool,
    fail: Option<fn(&Path) -> crate::editor::EditError>,
    cancel: Option<cancellation::CancellationToken>,
}

impl crate::editor::EditorService for FakeEditor {
    fn edit(&self, request: &crate::editor::EditRequest<'_>) -> crate::editor::EditOutcome {
        self.seen.lock().unwrap().push(SeenEdit {
            target: request.target.to_path_buf(),
            mutation_root: request.mutation_root.to_path_buf(),
            create_new: request.create_new,
            gui: request.gui,
            cancelled_at_start: (request.cancelled)(),
        });
        if let Some(token) = &self.cancel {
            token.cancel(cancellation::CancellationCause::Interrupt);
        }
        crate::editor::EditOutcome {
            attempted: self.attempted,
            result: match self.fail {
                Some(fail) => Err(fail(request.target)),
                None => Ok(()),
            },
        }
    }
}

impl Fixture {
    fn with_editor(editor: FakeEditor) -> Self {
        let mut fixture = Self::new();
        fixture.host.editor = Some(editor);
        fixture
    }

    /// [`Self::run`] with the document inside the fixture's directory, so
    /// its references resolve there.
    fn run_edit(&self, signal: LifecycleSignal, frontmatter: Value, document: Value) -> LifecycleEventOutcome {
        let config = parse(document);
        let fm: SharedFrontmatter = frontmatter.as_object().cloned().unwrap_or_default().into();
        let source = self.root().join("doc.md");
        let inputs = StackInputs { source_path: &source, ..self.inputs(signal, &fm) };
        StackExecutor::with_services(inputs, &self.effects, &NoCodes).execute_event(&config)
    }

    /// [`Self::run_edit`] over `completed`, parsed with `authored` as its
    /// authored source.
    fn run_completed_edit(&self, signal: LifecycleSignal, completed: Value, authored: &Value) -> LifecycleEventOutcome {
        let config = parse_completed(completed, Some(authored));
        let fm = SharedFrontmatter::default();
        let source = self.root().join("doc.md");
        let inputs = StackInputs { source_path: &source, ..self.inputs(signal, &fm) };
        StackExecutor::with_services(inputs, &self.effects, &NoCodes).execute_event(&config)
    }

    fn root(&self) -> &Path {
        self._dir.path()
    }

    fn seen(&self) -> Vec<SeenEdit> {
        self.host.editor.as_ref().map(|editor| editor.seen.lock().unwrap().clone()).unwrap_or_default()
    }
}

fn cancelled_at(target: &Path) -> crate::editor::EditError {
    crate::editor::EditorError::Cancelled { path: target.to_path_buf() }.into()
}

fn non_zero_exit(target: &Path) -> crate::editor::EditError {
    crate::editor::EditorError::NonZeroExit { code: 3, editor: "fake".into(), path: target.to_path_buf() }.into()
}

#[test]
fn edit_opens_the_interpolated_absolute_path_with_its_flags_then_continues() {
    let fixture = Fixture::with_editor(FakeEditor { attempted: true, ..Default::default() });
    let target = fixture.root().join("features/new/spec.md");

    let outcome = fixture.run_edit(
        LifecycleSignal::Initialize,
        json!({ "fqn": target.to_string_lossy() }),
        json!({ "initialize": [
            { "edit": { "file": "{{ fqn }}", "create_new": true, "gui": false } },
            { "info": "ready at {{ fqn }}" },
        ] }),
    );

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        fixture.seen(),
        [SeenEdit {
            target: target.clone(),
            mutation_root: fixture.root().to_path_buf(),
            create_new: true,
            gui: Some(false),
            cancelled_at_start: false,
        }]
    );
    assert_eq!(fixture.effects.lines(), [format!("info ready at {}", target.display())]);
    assert_eq!(fixture.host.side_effects_attempted.get(), 1, "an attempted edit forgets reused reads");
}

#[test]
fn a_missing_bare_relative_target_is_created_under_the_mutation_root() {
    let fixture = Fixture::with_editor(FakeEditor::default());

    let outcome = fixture.run_edit(LifecycleSignal::Success, json!({}), json!({ "success": [{ "edit": "notes/today.md" }] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    let seen = fixture.seen();
    assert_eq!(seen[0].target, fixture.root().join("notes/today.md"));
    assert_eq!((seen[0].create_new, seen[0].gui), (false, None));
    assert_eq!(fixture.host.side_effects_attempted.get(), 0, "a refusal before preparation reuses reads");
}

#[test]
fn a_host_without_an_editor_fails_the_action_as_unsupported_and_no_error_continues() {
    let fixture = Fixture::new();

    let outcome = fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": "a.md" }, { "info": "after" }] }),
    );

    let error = outcome.action_error.expect("a dispatch failure");
    assert_eq!(error.property.as_deref(), Some("success[0].edit"));
    assert!(error.msg.contains("error: Unsupported"), "{}", error.msg);
    assert!(error.msg.contains(&fixture.root().join("a.md").display().to_string()), "names the target: {}", error.msg);
    assert_eq!(*fixture.host.failures.borrow(), ["EditFailed"]);
    assert!(fixture.effects.lines().is_empty(), "no continuation after an unsuppressed failure");

    let fixture = Fixture::new();
    let outcome = fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": { "file": "a.md", "no_error": true } }, { "info": "after" }] }),
    );
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(fixture.effects.lines(), ["info after"]);
}

#[test]
fn an_editor_failure_is_a_located_dispatch_failure_that_no_error_suppresses() {
    let fixture = Fixture::with_editor(FakeEditor { attempted: true, fail: Some(non_zero_exit), ..Default::default() });

    let outcome = fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "when": "true", "then": [{ "edit": "a.md" }, { "info": "after" }] }] }),
    );

    let error = outcome.action_error.expect("a dispatch failure");
    assert_eq!(error.property.as_deref(), Some("success[0].then[0].edit"));
    assert!(error.msg.contains("EditFailed"), "{}", error.msg);
    assert!(fixture.effects.lines().is_empty());
    assert_eq!(fixture.host.side_effects_attempted.get(), 1, "a failed attempt still forgets reused reads");

    let fixture = Fixture::with_editor(FakeEditor { attempted: true, fail: Some(non_zero_exit), ..Default::default() });
    let outcome = fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": { "file": "a.md", "no_error": true } }, { "info": "after" }] }),
    );
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(fixture.effects.lines(), ["info after"]);
    assert_eq!(fixture.host.side_effects_attempted.get(), 1);
}

#[test]
fn a_non_string_or_blank_value_is_an_unsuppressible_evaluation_failure() {
    for (value, expected) in [
        (json!(5), "got number"),
        (json!(true), "got boolean"),
        (json!(null), "got null"),
        (json!(["a.md"]), "got array"),
        (json!("   "), "non-blank"),
        (json!(""), "non-blank"),
    ] {
        let fixture = Fixture::with_editor(FakeEditor::default());
        let outcome = fixture.run_edit(
            LifecycleSignal::Success,
            json!({ "fqn": value }),
            json!({ "success": [{ "edit": { "file": "{{ fqn }}", "no_error": true } }, { "info": "after" }] }),
        );

        let error = outcome.evaluation_error.unwrap_or_else(|| panic!("{value}: an evaluation failure"));
        assert!(error.msg.contains(expected), "{value}: {}", error.msg);
        assert_eq!(error.property.as_deref(), Some("success[0].edit"), "{value}");
        assert!(fixture.seen().is_empty(), "{value}: the editor never ran");
        assert!(fixture.effects.lines().is_empty(), "{value}: `no_error` does not continue");
    }
}

#[test]
fn a_url_is_refused_without_reaching_the_editor() {
    let fixture = Fixture::with_editor(FakeEditor::default());

    let outcome = fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": "https://example.invalid/spec.md" }] }),
    );

    let error = outcome.action_error.expect("a dispatch failure");
    assert!(error.msg.contains("UrlRefused"), "{}", error.msg);
    assert!(fixture.seen().is_empty());
}

#[test]
fn an_edit_cancelled_while_open_never_continues_even_with_no_error() {
    let token = cancellation::CancellationToken::default();
    let _scope = token.enter();
    let fixture = Fixture::with_editor(FakeEditor {
        attempted: true,
        fail: Some(cancelled_at),
        ..Default::default()
    });

    let outcome = fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": { "file": "a.md", "no_error": true } }, { "info": "after" }] }),
    );

    let error = outcome.evaluation_error.expect("cancellation is an evaluation failure");
    assert_eq!(error.msg, "the run was cancelled while an `edit` was open");
    assert!(fixture.effects.lines().is_empty());
    assert_eq!(fixture.host.side_effects_attempted.get(), 1);
}

#[test]
fn a_cancellation_during_a_successful_editor_exit_still_stops_the_stack() {
    let token = cancellation::CancellationToken::default();
    let _scope = token.enter();
    let fixture = Fixture::with_editor(FakeEditor { cancel: Some(token.clone()), ..Default::default() });

    let outcome = fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": { "file": "a.md", "no_error": true } }, { "info": "after" }] }),
    );

    assert_eq!(outcome.evaluation_error.expect("cancelled").msg, "the run was cancelled while an `edit` was open");
    assert!(fixture.effects.lines().is_empty());
}

/// Terminal events run in a cleanup scope. A healthy run still edits there;
/// after a cancellation (a worker cause or the host's interrupt) the edit is
/// refused before the editor is reached.
#[test]
fn cleanup_events_edit_on_a_healthy_run_and_refuse_after_a_cancellation() {
    let token = cancellation::CancellationToken::default();
    let _scope = token.enter();
    let _cleanup = cancellation::CleanupScope::enter();
    let document = json!({ "finalize": [{ "edit": { "file": "a.md", "no_error": true } }, { "info": "after" }] });

    let healthy = Fixture::with_editor(FakeEditor::default());
    assert_eq!(healthy.run_edit(LifecycleSignal::Finalize, json!({}), document.clone()), LifecycleEventOutcome::default());
    assert_eq!(healthy.seen().len(), 1);
    assert!(!healthy.seen()[0].cancelled_at_start);

    let mut interrupted = Fixture::with_editor(FakeEditor::default());
    interrupted.host.interrupted = true;
    let outcome = interrupted.run_edit(LifecycleSignal::Finalize, json!({}), document.clone());
    assert_eq!(
        outcome.evaluation_error.expect("refused").msg,
        "the run was cancelled, so `edit` did not open an editor"
    );
    assert!(interrupted.seen().is_empty());

    token.cancel(cancellation::CancellationCause::Interrupt);
    let cancelled = Fixture::with_editor(FakeEditor::default());
    let outcome = cancelled.run_edit(LifecycleSignal::Finalize, json!({}), document);
    assert!(outcome.evaluation_error.is_some());
    assert!(cancelled.seen().is_empty());
    assert!(cancelled.effects.lines().is_empty());
}

#[test]
fn a_compact_dictionary_runs_communication_before_edit() {
    let fixture = Fixture::with_editor(FakeEditor { fail: Some(non_zero_exit), ..Default::default() });

    let outcome = fixture.run_edit(
        LifecycleSignal::Start,
        json!({}),
        json!({ "start": [
            { "info": "first" },
            { "edit": "a.md", "warn": "same item" },
            { "info": "last" },
        ] }),
    );

    assert!(outcome.action_error.is_some(), "the failing edit ends the stack");
    assert_eq!(fixture.effects.lines(), ["info first", "warn same item"]);
    assert_eq!(fixture.seen().len(), 1);
}

#[test]
fn a_number_or_boolean_target_opens_the_file_its_text_names() {
    for (operand, name) in [(json!(5), "5"), (json!(true), "true"), (json!(2.5), "2.5")] {
        for short in [operand.clone(), json!([operand.clone()])] {
            let fixture = Fixture::with_editor(FakeEditor::default());
            let outcome = fixture.run_edit(LifecycleSignal::Success, json!({}), json!({ "success": [{ "edit": short.clone() }] }));
            assert_eq!(outcome, LifecycleEventOutcome::default(), "{short}");
            assert_eq!(fixture.seen()[0].target, fixture.root().join(name), "{short}");
        }

        let fixture = Fixture::with_editor(FakeEditor::default());
        fixture.run_edit(LifecycleSignal::Success, json!({}), json!({ "success": [{ "edit": { "file": operand.clone() } }] }));
        assert_eq!(fixture.seen()[0].target, fixture.root().join(name), "file: {operand}");
    }
}

#[test]
fn converted_and_null_flags_reach_the_editor_as_booleans_and_defaults() {
    let fixture = Fixture::with_editor(FakeEditor::default());
    fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": { "file": "a.md", "create_new": "yes", "gui": "no" } }] }),
    );
    assert_eq!((fixture.seen()[0].create_new, fixture.seen()[0].gui), (true, Some(false)));

    let fixture = Fixture::with_editor(FakeEditor::default());
    fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": { "file": "a.md", "create_new": null, "gui": null } }] }),
    );
    assert_eq!((fixture.seen()[0].create_new, fixture.seen()[0].gui), (false, None));
}

#[test]
fn a_completed_edit_runs_once_and_only_its_authored_flag_suppresses_failure() {
    // The schema filled `no_error: false` beside and inside the long form,
    // and `create_new: false` inside it; nothing was authored.
    let authored = json!({ "success": [{ "edit": { "file": "a.md" } }, { "info": "after" }] });
    let completed = json!({ "success": [
        { "edit": { "file": "a.md", "create_new": false, "no_error": false }, "no_error": false },
        { "info": "after", "no_error": false },
    ] });
    let fixture = Fixture::with_editor(FakeEditor { attempted: true, fail: Some(non_zero_exit), ..Default::default() });
    let outcome = fixture.run_completed_edit(LifecycleSignal::Success, completed, &authored);
    assert!(outcome.action_error.is_some(), "a filled `false` suppresses nothing");
    assert_eq!(fixture.seen().len(), 1);
    assert!(fixture.effects.lines().is_empty());

    // An authored `"yes"` beside the action, converted by the schema, wins
    // over the filled flag inside it.
    let authored = json!({ "success": [{ "edit": { "file": "a.md" }, "no_error": "yes" }, { "info": "after" }] });
    let completed = json!({ "success": [
        { "edit": { "file": "a.md", "create_new": false, "no_error": false }, "no_error": true },
        { "info": "after", "no_error": false },
    ] });
    let fixture = Fixture::with_editor(FakeEditor { attempted: true, fail: Some(non_zero_exit), ..Default::default() });
    let outcome = fixture.run_completed_edit(LifecycleSignal::Success, completed, &authored);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(fixture.effects.lines(), ["info after"]);
}

#[test]
fn a_completed_edit_cancelled_while_open_never_continues_even_with_an_authored_flag() {
    let token = cancellation::CancellationToken::default();
    let _scope = token.enter();
    let fixture = Fixture::with_editor(FakeEditor { attempted: true, fail: Some(cancelled_at), ..Default::default() });
    let authored = json!({ "success": [{ "edit": { "file": "a.md", "no_error": "yes" } }, { "info": "after" }] });
    let completed = json!({ "success": [
        { "edit": { "file": "a.md", "create_new": false, "no_error": true }, "no_error": false },
        { "info": "after", "no_error": false },
    ] });

    let outcome = fixture.run_completed_edit(LifecycleSignal::Success, completed, &authored);

    let error = outcome.evaluation_error.expect("cancellation is an evaluation failure");
    assert_eq!(error.msg, "the run was cancelled while an `edit` was open");
    assert!(fixture.effects.lines().is_empty());
}
