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
    /// The display of each classified `edit` failure's error.
    edit_errors: RefCell<Vec<String>>,
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
        if let LifecycleRunFailure::EditFailed { error, .. } = &failure {
            self.edit_errors.borrow_mut().push(error.to_string());
        }
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
    warning: Option<String>,
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
            warning: self.warning.clone(),
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
fn an_edit_warning_reaches_the_host_once_and_the_stack_continues() {
    let warning = "Windows could not keep the editor running after the edit; it was closed: Access is denied.";
    let fixture = Fixture::with_editor(FakeEditor {
        attempted: true,
        warning: Some(warning.into()),
        ..Default::default()
    });

    let outcome = fixture.run_edit(
        LifecycleSignal::Success,
        json!({}),
        json!({ "success": [{ "edit": "a.md" }, { "info": "after" }] }),
    );

    assert_eq!(outcome, LifecycleEventOutcome::default(), "a warning does not fail the edit");
    assert_eq!(fixture.effects.lines(), [format!("warn {warning}"), "info after".to_string()]);
    assert_eq!(fixture.host.side_effects_attempted.get(), 1);
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
    // The fake host labels a failure with its `Debug` form, which escapes a
    // Windows path's backslashes.
    let target = format!("{:?}", fixture.root().join("a.md").display().to_string());
    assert!(error.msg.contains(target.trim_matches('"')), "names the target: {}", error.msg);
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

// ── mutation targets ─────────────────────────────────────────────────────
//
// A mutation target is selected by the file-reference contract (an existing
// file by lookup, a missing one at its first candidate) and then checked
// against the engine's mutation root, which is only the boundary.

/// A workspace laid out as `<tmp>/ws/prompts/doc.md`, with `<tmp>/ws` as the
/// repository root, the launch directory, and the mutation root, and
/// `<tmp>/outside` beyond the boundary.
struct Workspace {
    fixture: Fixture,
    ws: PathBuf,
    outside: PathBuf,
    doc: PathBuf,
}

const DOC_TEXT: &str = "---\ncount: 1\n---\nBody\n";

impl Workspace {
    fn new() -> Self {
        Self::with_editor(None)
    }

    fn with_editor(editor: Option<FakeEditor>) -> Self {
        let mut fixture = Fixture::new();
        let ws = fixture.root().join("ws");
        let outside = fixture.root().join("outside");
        let doc = ws.join("prompts/doc.md");
        std::fs::create_dir_all(doc.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(&doc, DOC_TEXT).unwrap();
        fixture.engine = EffectEngine::builder().mutation_root(&ws).auto_rehash(false).build();
        fixture.files = biscuit_file::FileResolutionContext::new(&ws).with_repository_root(&ws);
        fixture.host.editor = editor;
        Self { fixture, ws, outside, doc }
    }

    /// Run `document`'s `success` stack as authored by `prompts/doc.md`, with
    /// the document's frontmatter as live state.
    fn run(&self, document: Value) -> LifecycleEventOutcome {
        let config = parse(document);
        let fm: SharedFrontmatter = json!({ "count": 1, "patch": { "count": 2 } }).as_object().cloned().unwrap().into();
        let inputs = StackInputs { source_path: &self.doc, ..self.fixture.inputs(LifecycleSignal::Success, &fm) };
        StackExecutor::with_services(inputs, &self.fixture.effects, &NoCodes).execute_event(&config)
    }

    fn info(&self) -> Vec<String> {
        self.fixture.effects.lines()
    }

    /// Every path under the fixture's temp directory, relative to it.
    fn tree(&self) -> Vec<String> {
        fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                out.push(path.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"));
                if path.is_dir() {
                    walk(&path, base, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(self.fixture.root(), self.fixture.root(), &mut out);
        out.sort();
        out
    }
}

impl Workspace {
    /// The failure message of `outcome`: an `edit` failure's error display
    /// (the fake host's classification carries its debug label), else the action error.
    fn failure_message(&self, outcome: &LifecycleEventOutcome) -> String {
        match self.fixture.host.edit_errors.borrow().last() {
            Some(error) if outcome.action_error.as_ref().is_some_and(|info| info.msg.starts_with("EditFailed")) => {
                error.clone()
            }
            _ => action_message(outcome),
        }
    }
}

fn action_message(outcome: &LifecycleEventOutcome) -> String {
    outcome.action_error.as_ref().map(|error| error.msg.clone()).unwrap_or_default()
}

#[test]
fn a_missing_bare_relative_target_is_created_beside_the_document() {
    let fixture = Fixture::with_editor(FakeEditor::default());

    let outcome = fixture.run_edit(LifecycleSignal::Success, json!({}), json!({ "success": [{ "edit": "notes/today.md" }] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    let seen = fixture.seen();
    assert_eq!(seen[0].target, fixture.root().join("notes/today.md"));
    assert_eq!((seen[0].create_new, seen[0].gui), (false, None));
    assert_eq!(fixture.host.side_effects_attempted.get(), 0, "a refusal before preparation reuses reads");
}

#[test]
fn a_missing_bare_relative_target_in_a_subdirectory_document_lands_beside_it_not_at_the_root() {
    let workspace = Workspace::with_editor(Some(FakeEditor::default()));

    let outcome = workspace.run(json!({ "success": [
        { "edit": "notes.md" },
        { "append_line": ["log.txt", "one"] },
    ] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    let seen = workspace.fixture.seen();
    assert_eq!(seen[0].target, workspace.ws.join("prompts/notes.md"));
    assert_eq!(seen[0].mutation_root, workspace.ws, "the boundary is the root, never the base");
    assert_eq!(std::fs::read_to_string(workspace.ws.join("prompts/log.txt")).unwrap(), "one\n");
    assert!(!workspace.ws.join("log.txt").exists(), "never root-joined");
}

#[test]
fn an_existing_root_level_file_is_the_contracts_second_candidate_while_dot_slash_pins_the_document_dir() {
    let workspace = Workspace::with_editor(Some(FakeEditor::default()));
    std::fs::write(workspace.ws.join("notes.md"), "root\n").unwrap();

    let outcome = workspace.run(json!({ "success": [
        { "edit": "notes.md" },
        { "append_line": ["notes.md", "appended"] },
        { "ensure_file": ["./notes.md", "pinned\n"] },
    ] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(workspace.fixture.seen()[0].target, workspace.ws.join("notes.md"));
    assert_eq!(std::fs::read_to_string(workspace.ws.join("notes.md")).unwrap(), "root\nappended\n");
    assert_eq!(std::fs::read_to_string(workspace.ws.join("prompts/notes.md")).unwrap(), "pinned\n");
}

#[test]
fn a_file_created_beside_the_document_is_found_there_by_the_next_reference() {
    let workspace = Workspace::with_editor(Some(FakeEditor::default()));
    std::fs::write(workspace.ws.join("notes.md"), "root\n").unwrap();

    // `./notes.md` created beside the document now outranks the root file
    // for the bare spelling too.
    let outcome = workspace.run(json!({ "success": [
        { "ensure_file": "./notes.md" },
        { "append_line": ["notes.md", "beside"] },
        { "edit": "notes.md" },
    ] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(std::fs::read_to_string(workspace.ws.join("prompts/notes.md")).unwrap(), "beside\n");
    assert_eq!(std::fs::read_to_string(workspace.ws.join("notes.md")).unwrap(), "root\n");
    assert_eq!(workspace.fixture.seen()[0].target, workspace.ws.join("prompts/notes.md"));
}

#[test]
fn a_nested_missing_target_creates_its_parents_beside_the_document() {
    let workspace = Workspace::new();

    let outcome = workspace.run(json!({ "success": [
        { "ensure_file": ["specs/new/a.md", "seed"] },
        { "ensure_dir": "./logs/today" },
    ] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(std::fs::read_to_string(workspace.ws.join("prompts/specs/new/a.md")).unwrap(), "seed");
    assert!(workspace.ws.join("prompts/logs/today").is_dir());
    assert!(!workspace.ws.join("specs").exists() && !workspace.ws.join("logs").exists());
}

#[test]
fn an_unmatched_recursive_reference_is_refused_and_creates_nothing() {
    let workspace = Workspace::with_editor(Some(FakeEditor::default()));
    let before = workspace.tree();

    let ensure = workspace.run(json!({ "success": [{ "ensure_file": "%missing.md" }] }));
    let edit = workspace.run(json!({ "success": [{ "edit": "%missing.md" }] }));

    let message = action_message(&ensure);
    assert!(message.contains("recursive") && message.contains("%missing.md"), "{message}");
    assert!(action_message(&edit).contains("NoLocalTarget"), "{}", action_message(&edit));
    assert!(workspace.fixture.seen().is_empty(), "no editor for an unresolved target");
    assert_eq!(workspace.tree(), before);
}

#[test]
fn a_url_target_is_refused_as_not_local_without_writing() {
    let workspace = Workspace::new();
    let before = workspace.tree();

    let outcome = workspace.run(json!({ "success": [{ "append_line": ["https://example.invalid/log.txt", "x"] }] }));

    // `RemoteNotLocal` is decided before any probe or fetch; the test has
    // no network, so reaching a fetch would surface as a different error.
    let message = action_message(&outcome);
    assert!(message.contains("example.invalid"), "{message}");
    assert!(!message.contains("outside the mutation root"), "{message}");
    assert_eq!(workspace.tree(), before);
}

#[test]
fn a_target_outside_the_root_is_refused_naming_target_and_root_before_any_parent_exists() {
    let workspace = Workspace::with_editor(Some(FakeEditor::default()));
    let before = workspace.tree();
    let target = workspace.outside.join("deep").join("new").join("file.md");

    let ensure = workspace.run(json!({ "success": [{ "ensure_file": target.to_string_lossy() }] }));
    let edit = workspace.run(json!({ "success": [{ "edit": target.to_string_lossy() }] }));
    // A relative escape is already refused by the reference's own tree rule.
    let escape = workspace.run(json!({ "success": [{ "ensure_file": "../../outside/escape.md" }] }));

    assert!(action_message(&escape).contains("leaves file tree"), "{}", action_message(&escape));
    for outcome in [&ensure, &edit] {
        let message = workspace.failure_message(outcome);
        assert!(message.contains("outside the mutation root"), "{message}");
        assert!(message.contains(&*workspace.ws.to_string_lossy()), "names the root: {message}");
    }
    assert!(action_message(&ensure).contains(&*target.to_string_lossy()), "names the target: {}", action_message(&ensure));
    assert!(workspace.fixture.seen().is_empty(), "the editor is never invoked");
    assert_eq!(workspace.tree(), before, "no file or parent was created");
}

#[test]
fn an_in_root_parent_reference_and_a_sibling_text_prefix_are_judged_by_components() {
    let workspace = Workspace::new();
    let sibling = workspace.fixture.root().join("ws-x");

    let inside = workspace.run(json!({ "success": [{ "ensure_file": "../shared/state.md" }] }));
    let prefix = workspace.run(json!({ "success": [{ "ensure_file": sibling.join("f.md").to_string_lossy() }] }));

    assert_eq!(inside, LifecycleEventOutcome::default());
    assert!(workspace.ws.join("shared/state.md").is_file());
    assert!(action_message(&prefix).contains("outside the mutation root"), "{}", action_message(&prefix));
    assert!(!sibling.exists());
}

#[test]
fn a_frontmatter_write_to_the_active_document_is_visible_to_the_next_action() {
    let workspace = Workspace::new();

    let outcome = workspace.run(json!({ "success": [
        { "set_frontmatter": ["doc.md", "count", 7] },
        { "info": "count={{ count }}" },
    ] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(workspace.info(), ["info count=7"]);
    assert!(std::fs::read_to_string(&workspace.doc).unwrap().contains("count: 7"));
}

#[test]
fn a_frontmatter_write_to_another_file_leaves_the_active_documents_state_alone() {
    let workspace = Workspace::new();
    let other = workspace.ws.join("prompts/other.md");
    std::fs::write(&other, DOC_TEXT).unwrap();

    let outcome = workspace.run(json!({ "success": [
        { "set_frontmatter": ["other.md", "count", 9] },
        { "info": "count={{ count }}" },
    ] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(workspace.info(), ["info count=1"]);
    assert!(std::fs::read_to_string(&other).unwrap().contains("count: 9"));
    assert_eq!(std::fs::read_to_string(&workspace.doc).unwrap(), DOC_TEXT);
}

#[test]
fn the_active_document_is_recognized_through_an_equivalent_spelling() {
    let workspace = Workspace::new();
    let spelled = workspace.ws.join("prompts/../prompts/./doc.md");

    let outcome = workspace.run(json!({ "success": [
        { "increment_frontmatter": ["../prompts/doc.md", "count"] },
        { "increment_frontmatter": [spelled.to_string_lossy(), "count"] },
        { "info": "count={{ count }}" },
    ] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(workspace.info(), ["info count=3"]);
    assert!(std::fs::read_to_string(&workspace.doc).unwrap().contains("count: 3"));
}

/// One action per file-writing verb, by the operand it writes.
fn writer_action(verb: &str, file: &str) -> Value {
    match verb {
        "set_frontmatter" => json!({ verb: [file, "count", 2] }),
        "merge_frontmatter" => json!({ verb: [file, "{{ patch }}"] }),
        "delete_frontmatter" | "increment_frontmatter" | "decrement_frontmatter" => json!({ verb: [file, "count"] }),
        "append_frontmatter" | "prepend_frontmatter" => json!({ verb: [file, "tags", "t"] }),
        "ensure_file" | "ensure_dir" | "edit" => json!({ verb: file }),
        "ensure_file_with_content" => json!({ "ensure_file": [file, "seed"] }),
        "append_line" => json!({ verb: [file, "line"] }),
        "append_jsonl" => json!({ verb: [file, "{{ patch }}"] }),
        other => unreachable!("{other}"),
    }
}

const WRITERS: [&str; 13] = [
    "set_frontmatter",
    "merge_frontmatter",
    "delete_frontmatter",
    "increment_frontmatter",
    "decrement_frontmatter",
    "append_frontmatter",
    "prepend_frontmatter",
    "ensure_file",
    "ensure_file_with_content",
    "ensure_dir",
    "append_line",
    "append_jsonl",
    "edit",
];

#[test]
fn the_writer_table_covers_every_frontmatter_verb() {
    for verb in FRONTMATTER_VERBS {
        assert!(WRITERS.contains(&verb), "{verb} has no row");
    }
}

#[test]
fn every_writer_acts_on_the_target_beside_the_document() {
    for verb in WRITERS {
        let workspace = Workspace::with_editor(Some(FakeEditor::default()));
        let file = if FRONTMATTER_VERBS.contains(&verb) { "target.md" } else { "new/target.md" };
        let target = workspace.ws.join("prompts").join(file);
        if FRONTMATTER_VERBS.contains(&verb) {
            std::fs::write(&target, "---\ncount: 1\ntags: []\n---\n").unwrap();
        }

        let outcome = workspace.run(json!({ "success": [writer_action(verb, file)] }));

        assert_eq!(outcome, LifecycleEventOutcome::default(), "{verb}");
        if verb == "edit" {
            assert_eq!(workspace.fixture.seen()[0].target, target, "{verb}");
        } else {
            assert!(target.exists(), "{verb} acted on {}", target.display());
        }
        assert!(!workspace.ws.join(file).exists(), "{verb} never root-joins");
    }
}

#[test]
fn every_writer_refuses_an_outside_target_and_touches_nothing() {
    for verb in WRITERS {
        let workspace = Workspace::with_editor(Some(FakeEditor::default()));
        let target = workspace.outside.join("sub/target.md");
        if FRONTMATTER_VERBS.contains(&verb) {
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(&target, "---\ncount: 1\ntags: []\n---\n").unwrap();
        }
        let before = workspace.tree();
        let target_before = std::fs::read_to_string(&target).ok();

        let outcome = workspace.run(json!({ "success": [writer_action(verb, &target.to_string_lossy())] }));

        let message = workspace.failure_message(&outcome);
        assert!(message.contains("outside the mutation root"), "{verb}: {message}");
        assert_eq!(workspace.tree(), before, "{verb} created nothing");
        assert_eq!(std::fs::read_to_string(&target).ok(), target_before, "{verb} wrote nothing");
        assert!(workspace.fixture.seen().is_empty(), "{verb} invoked no editor");
        assert_eq!(workspace.info(), Vec::<String>::new());
    }
}

#[test]
fn a_refused_target_honors_no_error_and_the_stack_continues() {
    let workspace = Workspace::new();
    let target = workspace.outside.join("x.md");

    let outcome = workspace.run(json!({ "success": [
        { "ensure_file": target.to_string_lossy(), "no_error": true },
        { "info": "continued" },
    ] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(workspace.info(), ["info continued"]);
    assert!(!target.exists());
}

#[cfg(unix)]
#[test]
fn a_symlink_inside_the_root_cannot_redirect_a_write_outside_it() {
    let workspace = Workspace::new();
    std::os::unix::fs::symlink(&workspace.outside, workspace.ws.join("prompts/link")).unwrap();

    // Spelled absolute, so the boundary check (not the reference's relative
    // tree rule) is what refuses it.
    let through_link = workspace.ws.join("prompts/link/escape.md");
    let outcome = workspace.run(json!({ "success": [{ "ensure_file": through_link.to_string_lossy() }] }));

    assert!(action_message(&outcome).contains("outside the mutation root"), "{}", action_message(&outcome));
    assert!(!workspace.outside.join("escape.md").exists());
}

#[cfg(windows)]
#[test]
fn a_junction_or_directory_symlink_inside_the_root_cannot_redirect_a_write_outside_it() {
    let workspace = Workspace::new();
    // A junction needs no privilege; `cmd` reads a `/` inside an argument as
    // a switch, so both paths are re-spelled natively.
    let junction = workspace.ws.join("prompts/junction");
    let native = |path: &Path| path.components().collect::<PathBuf>();
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(native(&junction))
        .arg(native(&workspace.outside))
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "mklink /J failed");
    let symlink = workspace.ws.join("prompts/symlink");
    std::os::windows::fs::symlink_dir(&workspace.outside, &symlink)
        .expect("creating a directory symlink needs Developer Mode or SeCreateSymbolicLinkPrivilege");

    for link in [junction, symlink] {
        let through_link = link.join("escape.md");
        let outcome = workspace.run(json!({ "success": [{ "ensure_file": through_link.to_string_lossy() }] }));

        assert!(action_message(&outcome).contains("outside the mutation root"), "{}", action_message(&outcome));
        assert!(!workspace.outside.join("escape.md").exists(), "{}", link.display());
    }
}

impl Workspace {
    /// This workspace with its resolution context passed through `configure`.
    fn configure_files(
        mut self,
        configure: impl FnOnce(biscuit_file::FileResolutionContext) -> biscuit_file::FileResolutionContext,
    ) -> Self {
        self.fixture.files = configure(self.fixture.files.clone());
        self
    }

    /// This workspace with `root` as the engine's mutation root instead of `ws`.
    fn with_mutation_root(mut self, root: &Path) -> Self {
        self.fixture.engine = EffectEngine::builder().mutation_root(root).auto_rehash(false).build();
        self
    }

    /// `path` relative to the fixture's temp directory, for messages.
    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(self.fixture.root()).unwrap_or(path).to_string_lossy().replace('\\', "/")
    }

    /// Run each `(action, expected)` row as its own `success` stack, in order,
    /// asserting the row succeeds (`None`) or fails with a message containing
    /// the expected text.
    fn run_rows(&self, rows: &[(Value, Option<&str>)]) {
        for (action, expected) in rows {
            let outcome = self.run(json!({ "success": [action] }));
            match expected {
                None => assert_eq!(outcome, LifecycleEventOutcome::default(), "{action}: {}", self.failure_message(&outcome)),
                Some(text) => {
                    let message = self.failure_message(&outcome);
                    assert!(message.contains(text), "{action}: expected `{text}` in {message}");
                }
            }
        }
    }
}

const OUTSIDE_ROOT: &str = "outside the mutation root";

#[test]
fn the_specs_example_table_selects_each_target_by_the_file_reference_contract() {
    let workspace = Workspace::with_editor(Some(FakeEditor::default()));
    let home = workspace.outside.clone();
    let workspace = workspace.configure_files(|files| files.with_home_dir(home));
    let auth = workspace.ws.join("specs/auth.md");
    std::fs::create_dir_all(auth.parent().unwrap()).unwrap();
    std::fs::write(&auth, "# Auth\nexisting\n").unwrap();

    workspace.run_rows(&[
        (json!({ "edit": "specs/auth.md" }), None),
        (json!({ "ensure_file": ["specs/auth.md", "# Auth"] }), None),
        (json!({ "append_line": ["specs/auth.md", "reviewed"] }), None),
        (json!({ "ensure_file": ["specs/new.md", "# New"] }), None),
        (json!({ "edit": "~/notes.md" }), Some(OUTSIDE_ROOT)),
    ]);

    let seen: Vec<PathBuf> = workspace.fixture.seen().into_iter().map(|edit| edit.target).collect();
    assert_eq!(seen, std::slice::from_ref(&auth), "the second candidate opens; the refused `~` never reaches the editor");
    assert_eq!(std::fs::read_to_string(&auth).unwrap(), "# Auth\nexisting\nreviewed\n", "unchanged by ensure_file, then appended");
    assert_eq!(std::fs::read_to_string(workspace.ws.join("prompts/specs/new.md")).unwrap(), "# New");
    assert!(!workspace.ws.join("specs/new.md").exists(), "a missing target is the first candidate");
    assert!(!workspace.outside.join("notes.md").exists());
}

#[test]
fn prefixed_targets_inside_the_root_resolve_by_their_prefix_and_write() {
    let workspace = Workspace::new();
    let (ws, outside) = (workspace.ws.clone(), workspace.outside.clone());
    let package = ws.join("pkg");
    let vault = ws.join("vault");
    for dir in [&package, &vault, &ws.join("specs")] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(ws.join("specs/shared.md"), "repo\n").unwrap();
    std::fs::write(vault.join("kept.md"), "vault\n").unwrap();
    let workspace = workspace.configure_files(|files| {
        files.with_package_root(&package).add_vault(&vault).with_home_dir(outside)
    });

    workspace.run_rows(&[
        // Existing targets: each prefix's lookup result.
        (json!({ "append_line": ["&specs/shared.md", "amp"] }), None),
        (json!({ "append_line": ["^specs/shared.md", "caret"] }), None),
        (json!({ "append_line": ["@specs/shared.md", "magic"] }), None),
        (json!({ "append_line": ["vault:kept.md", "vault"] }), None),
        // Missing targets: each prefix's first candidate.
        (json!({ "ensure_file": ["&specs/amp-new.md", "amp"] }), None),
        (json!({ "ensure_file": ["^caret-new.md", "caret"] }), None),
        (json!({ "ensure_file": ["@magic-new.md", "magic"] }), None),
        (json!({ "ensure_file": ["vault:notes/vault-new.md", "vault"] }), None),
    ]);

    assert_eq!(std::fs::read_to_string(ws.join("specs/shared.md")).unwrap(), "repo\namp\ncaret\nmagic\n");
    assert_eq!(std::fs::read_to_string(vault.join("kept.md")).unwrap(), "vault\nvault\n");
    for (created, prefix) in [
        (ws.join("specs/amp-new.md"), "&: the repository root"),
        (package.join("caret-new.md"), "^: the package root before the repository"),
        (package.join("magic-new.md"), "@: the launch package root first"),
        (vault.join("notes/vault-new.md"), "vault: the first vault root"),
    ] {
        assert!(created.is_file(), "{prefix}: {}", workspace.relative(&created));
    }
    assert!(!ws.join("prompts/specs").exists(), "no prefix falls back beside the document");
}

#[test]
fn prefixed_targets_resolving_outside_the_root_are_refused_and_create_nothing() {
    let workspace = Workspace::with_editor(Some(FakeEditor::default()));
    let outside = workspace.outside.clone();
    let (vault, convention) = (outside.join("vault"), outside.join("conventions"));
    for dir in [&vault, &convention] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(vault.join("kept.md"), "vault\n").unwrap();
    std::fs::write(outside.join("home-only.md"), "home\n").unwrap();
    std::fs::write(convention.join("convention-only.md"), "convention\n").unwrap();
    let workspace = workspace.configure_files(|files| {
        files
            .add_vault(&vault)
            .with_home_dir(&outside)
            .add_magic_path(&convention, biscuit_file::PathPosition::End)
    });
    let before = workspace.tree();

    workspace.run_rows(&[
        // Existing targets the prefix finds outside the root.
        (json!({ "append_line": ["vault:kept.md", "x"] }), Some(OUTSIDE_ROOT)),
        (json!({ "append_line": ["@home-only.md", "x"] }), Some(OUTSIDE_ROOT)),
        (json!({ "append_line": ["@convention-only.md", "x"] }), Some(OUTSIDE_ROOT)),
        (json!({ "edit": "vault:kept.md" }), Some(OUTSIDE_ROOT)),
        // Missing targets whose first candidate lies outside the root.
        (json!({ "ensure_file": ["vault:notes/new.md", "x"] }), Some(OUTSIDE_ROOT)),
        (json!({ "ensure_file": ["~/new/home.md", "x"] }), Some(OUTSIDE_ROOT)),
    ]);

    assert_eq!(workspace.tree(), before, "no file or parent was created");
    assert_eq!(std::fs::read_to_string(vault.join("kept.md")).unwrap(), "vault\n");
    assert_eq!(std::fs::read_to_string(outside.join("home-only.md")).unwrap(), "home\n");
    assert_eq!(std::fs::read_to_string(convention.join("convention-only.md")).unwrap(), "convention\n");
    assert!(workspace.fixture.seen().is_empty(), "the editor is never invoked");
}

/// `&` and `^` refuse targets outside their repository on their own, so an
/// outside-the-root case needs a mutation root narrower than the document's
/// repository; in production that is a shared prompt whose repository is not
/// the caller's.
#[test]
fn repository_prefixes_are_not_write_grants_when_the_root_is_narrower_than_the_repository() {
    let workspace = Workspace::new();
    let narrow = workspace.ws.join("prompts");
    let workspace = workspace.with_mutation_root(&narrow);
    let shared = workspace.ws.join("specs/shared.md");
    std::fs::create_dir_all(shared.parent().unwrap()).unwrap();
    std::fs::write(&shared, "repo\n").unwrap();
    let before = workspace.tree();

    workspace.run_rows(&[
        (json!({ "append_line": ["&specs/shared.md", "x"] }), Some(OUTSIDE_ROOT)),
        (json!({ "append_line": ["^specs/shared.md", "x"] }), Some(OUTSIDE_ROOT)),
        (json!({ "ensure_file": ["&specs/new.md", "x"] }), Some(OUTSIDE_ROOT)),
        (json!({ "ensure_file": ["^new.md", "x"] }), Some(OUTSIDE_ROOT)),
    ]);

    assert_eq!(workspace.tree(), before, "no file or parent was created");
    assert_eq!(std::fs::read_to_string(&shared).unwrap(), "repo\n");
    // The same root still admits a target beside the document.
    workspace.run_rows(&[(json!({ "ensure_file": ["./inside.md", "x"] }), None)]);
    assert!(narrow.join("inside.md").is_file());
}

/// A lifecycle operand is itself a template, so an author reaches
/// biscuit-file's `{{NAME}}` placeholder through an interpolation literal:
/// `{{{NAME}}}` renders the text `{{NAME}}`, which the file reference expands
/// from the captured environment.
#[test]
fn an_environment_expanded_absolute_target_is_judged_as_that_absolute_path() {
    let workspace = Workspace::new();
    let env = HashMap::from([
        ("INSIDE_DIR".to_string(), workspace.ws.join("specs").to_string_lossy().into_owned()),
        ("OUTSIDE_DIR".to_string(), workspace.outside.to_string_lossy().into_owned()),
    ]);
    let workspace = workspace.configure_files(|files| files.with_env(env));

    workspace.run_rows(&[
        (json!({ "ensure_file": ["{{{INSIDE_DIR}}}/env/a.md", "inside"] }), None),
        (json!({ "ensure_file": ["{{{OUTSIDE_DIR}}}/env/b.md", "outside"] }), Some(OUTSIDE_ROOT)),
    ]);

    assert_eq!(std::fs::read_to_string(workspace.ws.join("specs/env/a.md")).unwrap(), "inside");
    assert!(!workspace.ws.join("prompts/specs").exists(), "never joined beside the document");
    assert!(!workspace.outside.join("env").exists(), "no parent was created outside");
}
