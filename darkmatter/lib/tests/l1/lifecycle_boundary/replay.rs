//! The Stage 1 gate, now run against production: every trace recorded by
//! `claudine/cli/tests/l1/lifecycle_traces.rs` replayed through
//! `darkmatter::lifecycle::run::Run` by a scripted Claudine host.
//!
//! The host answers each request with facts only: an event's
//! `LifecycleEventOutcome` (an evaluation error, a control) and, for `loop`,
//! its iteration verdict; preparation readiness or a failure with its stage and
//! entry; an operation's verdict and session; and a recovery's budget ceiling,
//! refusal, adoption, or abort. It also writes the trace lines production's
//! authored actions and fake services write: `event`, `err` (rendered from the
//! request's selected error), `state`, `shell` (only when
//! [`Run::shell_permission`] grants it), `work`, and `task`. Each scenario's
//! documents are its `.md` fixtures transcribed into the small stack
//! vocabulary [`Host::step`] interprets; its facts are the trace's `# given:`
//! lines. Claudine's own concerns stay here: document state and `set`,
//! retry/resume budgets, proxy target resolution and parsing, `loop`
//! `until`/`max`, and container scheduling (one [`Run`] per subject, sequence
//! order, `fail_fast` cancellation through [`Run::cancel`]).

use std::collections::BTreeSet;
use std::error::Error as StdError;
use std::fs;
use std::sync::Arc;

use darkmatter::lifecycle::LifecycleSignal;
use darkmatter::lifecycle::actions::{LifecyclePath, RetryBackoff};
use darkmatter::lifecycle::policy::SubjectKind;
use darkmatter::lifecycle::envelope::{DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, DiagnosticCatalog, DiagnosticSnapshot};
use darkmatter::lifecycle::executor::{ActionLocation, LifecycleEventOutcome, StackControl};
use darkmatter::lifecycle::run::{
    ApprovalEpoch, AuthoredEvents, CompletionVerdict, EntryReason, EventRequest, FailureStage, Finish, HostFailure,
    OperationReport, Preparation, RecoveryReply, RecoveryRequest, Reply, RequestKind, Run, RunError, RunSettings,
    SessionAvailability, Ticket,
};
use darkmatter::lifecycle::LifecycleErrorInfo;
use indexmap::IndexMap;
use serde_json::{Map, Value, json};

/// Claudine's refusal of a lifecycle shell before `start`; it is raised like
/// an evaluation error.
const SHELL_LOCKED: &str =
    "shell commands are forbidden during initialize and before preflight completes; move the command to start or a later event";

/// The code a failed sequence task or group reports to its parent subject.
const COMPOSITION_FAILED: &str = "composition.failed";

/// Claudine's cancellation cause, read as `err` by a cancelled subject.
const COMPOSITION_CANCELLED: &str = "composition.cancelled";

/// Claudine's cancellation cause for a Ctrl+C, read as `err` by an
/// interrupted root's cleanup events.
const PROVIDER_INTERRUPTED: &str = "provider.interrupted";

/// A provider status that stands for a launch interrupted by Ctrl+C.
const INTERRUPTED: i32 = 130;

/// Scenarios whose recorded trace `Run` does not reproduce yet. Each one's
/// test asserts that the replay still differs from production, and
/// [`known_divergences_still_diverge`] fails once any of them matches, so the
/// list cannot go stale: remove a scenario here when `Run` is fixed.
const KNOWN_DIVERGENT: &[&str] = &[];

/// The fixtures' explicit `error(...)` failures are uncoded.
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

/// A frontmatter document: its lifecycle keys and its data.
#[derive(Clone, Debug)]
pub struct DocumentInput {
    frontmatter: Map<String, Value>,
}

impl DocumentInput {
    pub fn new(_label: &str) -> Self {
        Self { frontmatter: Map::new() }
    }

    #[must_use]
    pub fn with(mut self, key: &str, value: Value) -> Self {
        self.frontmatter.insert(key.to_string(), value);
        self
    }

    fn signal(key: &str) -> Option<LifecycleSignal> {
        LifecycleSignal::ALL.into_iter().find(|signal| signal.property_name() == key)
    }

    /// The events this document authors a non-empty stack for.
    fn authored(&self) -> AuthoredEvents {
        AuthoredEvents::of(self.frontmatter.iter().filter_map(|(key, value)| {
            let empty = value.as_array().is_some_and(Vec::is_empty);
            Self::signal(key).filter(|_| !empty)
        }))
    }

    /// The document's data: every key that is not a lifecycle event.
    fn data(&self) -> Map<String, Value> {
        self.frontmatter.iter().filter(|(key, _)| Self::signal(key).is_none()).map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    fn stack(&self, signal: LifecycleSignal) -> Option<&Value> {
        self.frontmatter.get(signal.property_name())
    }
}

/// One lifecycle subject: a document, step, group, leg, or nested sequence.
struct Subject {
    prefix: &'static str,
    /// The subject kind its run is told, a document unless set.
    kind: SubjectKind,
    document: DocumentInput,
    /// The code every attempt's preflight fails with.
    preflight: Option<&'static str>,
    operation: Operation,
}

impl Subject {
    fn new(prefix: &'static str, document: DocumentInput) -> Self {
        Self { prefix, kind: SubjectKind::Document, document, preflight: None, operation: Operation::Agent }
    }

    fn kind(mut self, kind: SubjectKind) -> Self {
        self.kind = kind;
        self
    }

    fn preflight(mut self, code: &'static str) -> Self {
        self.preflight = Some(code);
        self
    }

    fn operation(mut self, operation: Operation) -> Self {
        self.operation = operation;
        self
    }

    /// Claudine's target parse rejects a `shell` in `initialize`.
    fn initialize_runs_shell(&self) -> bool {
        self.document
            .frontmatter
            .get("initialize")
            .and_then(Value::as_array)
            .is_some_and(|steps| steps.contains(&json!("shell")))
    }
}

enum Operation {
    /// One provider launch per attempt; a launch whose scripted status is
    /// [`INTERRUPTED`] is cancelled while it runs.
    Agent,
    /// Tasks in order; the first failed task fails the subject.
    Tasks(Vec<Task>),
    /// A concurrent group: the leg runs until it waits on a shell, then the
    /// sibling runs; a failed sibling cancels the leg only under `fail_fast`
    /// and only when the leg declares `cancelled`.
    Race { leg: Box<Subject>, sibling: Shell, fail_fast: bool },
}

enum Task {
    Shell(Shell),
    Subject(Subject),
    /// A shell that blocks until the enclosing race's sibling has run.
    Wait { started: &'static str, completed: &'static str },
    /// A shell that writes its line and then interrupts the whole run.
    Interrupt(&'static str),
}

struct Shell {
    line: &'static str,
    succeeds: bool,
}

struct World {
    root: Subject,
    /// Exit status of each provider launch in order; the last one repeats.
    provider_exits: &'static [i32],
    /// Proxy targets that resolve; any other target is missing.
    targets: Vec<(&'static str, Subject)>,
    /// The code of the error a cancellation reports.
    cancellation: &'static str,
    /// The root run's selected error at its finish, as [`RunError::label`]
    /// spells it, when the scenario checks it: production's reported error
    /// is in the trace's `# given:` lines, not in its event lines.
    reported: Option<&'static str>,
}

impl World {
    fn new(root: Subject) -> Self {
        Self { root, provider_exits: &[0], targets: Vec::new(), cancellation: COMPOSITION_CANCELLED, reported: None }
    }

    /// A Ctrl+C interrupts the run rather than a failed sibling.
    fn interrupted(mut self) -> Self {
        self.cancellation = PROVIDER_INTERRUPTED;
        self
    }

    fn reports(mut self, label: &'static str) -> Self {
        self.reported = Some(label);
        self
    }

    fn exits(mut self, exits: &'static [i32]) -> Self {
        self.provider_exits = exits;
        self
    }

    fn target(mut self, path: &'static str, subject: Subject) -> Self {
        self.targets.push((path, subject));
        self
    }
}

/// Per-run host bookkeeping. None of it is engine accounting.
struct RunContext<'w> {
    /// The run's documents by ticket document number (1-based).
    documents: Vec<&'w Subject>,
    /// The current document's data, which `set` writes.
    state: Map<String, Value>,
    /// The entry of the latest preparation or operation request, which a
    /// failure must name.
    entry: EntryReason,
    /// Claudine's recovery ceiling, fixed by a document's first recovery.
    budget: Option<u32>,
    launched_model: Option<Value>,
}

struct Race<'w> {
    sibling: &'w Shell,
    cancel: bool,
}

struct Host<'w> {
    world: &'w World,
    lines: Vec<String>,
    launches: usize,
    /// The entry document's `model` on disk; `set_frontmatter` rewrites it.
    disk_model: Option<Value>,
    race: Option<Race<'w>>,
    /// The selected error of the run that finished last; after the root's
    /// drive, the root's.
    reported: Option<String>,
}

/// One event's evaluation: the request plus the stack's local view of state.
struct Evaluation<'a> {
    run: &'a Run,
    ticket: &'a Ticket,
    prefix: &'a str,
    event: &'a EventRequest,
    state: Map<String, Value>,
    outcome: LifecycleEventOutcome,
}

impl Evaluation<'_> {
    fn name(&self) -> &'static str {
        self.event.signal.property_name()
    }

    /// Ends the stack with an evaluation error.
    fn raise(&mut self, error: LifecycleErrorInfo) -> bool {
        self.outcome.evaluation_error = Some(error);
        false
    }

    /// Ends the stack with a control.
    fn direct(&mut self, control: StackControl) -> bool {
        self.outcome.control = Some(control);
        false
    }
}

fn snapshot(code: &str, message: &str) -> DiagnosticSnapshot {
    DiagnosticSnapshot {
        schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: code.split('.').next().unwrap_or(code).to_string(),
        code: code.to_string(),
        disposition: "correctable".to_string(),
        origin: "author".to_string(),
        severity: "error".to_string(),
        detail: json!({}),
        message: message.to_string(),
        cause: None,
        frontmatter_excerpt: None,
    }
}

/// A failure Claudine's catalog classifies under `code`.
fn coded(code: &str, message: &str) -> LifecycleErrorInfo {
    LifecycleErrorInfo::from_snapshot("Host", code.to_string(), snapshot(code, message))
}

fn uncoded(message: &str) -> LifecycleErrorInfo {
    LifecycleErrorInfo::from_action_failure_with(&NoCodes, "host", message)
}

/// A reached template shell whose placeholder reads `null`; Claudine reports
/// it with the generic composition code.
fn missing_value(key: &str) -> LifecycleErrorInfo {
    coded(COMPOSITION_FAILED, &format!("the command reads `{key}`, which is missing"))
}

fn settings(cancellation: &str) -> RunSettings {
    RunSettings::new(Arc::new(NoCodes)).with_cancellation_error(coded(cancellation, "the run was cancelled"))
}

/// Expression truthiness for the values the fixtures guard on.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::String(text) => !text.is_empty(),
        _ => true,
    }
}

/// The fixtures' `err.code || 'uncoded: ' + err.msg`. A refusal the run
/// raised itself reads as Claudine's projection of it
/// (`CompositionError::from_lifecycle_refusal`), whose code is
/// [`COMPOSITION_FAILED`].
fn render(error: &RunError) -> String {
    match error {
        RunError::Reported(error) => match &error.snapshot {
            Some(snapshot) => snapshot.code.clone(),
            None => format!("uncoded: {}", error.msg),
        },
        RunError::SetupRecoveryUnsupported { .. } | RunError::Aborted(_) => COMPOSITION_FAILED.to_string(),
    }
}

fn render_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        // `{{ key }}` renders a null as empty text.
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn answer(run: &mut Run, reply: Reply) {
    run.answer(reply).unwrap_or_else(|error| panic!("{error:?} after {:?}", run.transcript_lines()));
}

fn retry(max_attempts: u32) -> StackControl {
    StackControl::Retry { max_attempts, backoff: RetryBackoff::Fixed, delay: "0s".to_string(), overlay: IndexMap::new() }
}

fn resume(max_attempts: u32) -> StackControl {
    StackControl::Resume { message: String::new(), max_attempts }
}

fn proxy(signal: LifecycleSignal, target: &str) -> StackControl {
    StackControl::Proxy {
        target: target.to_string(),
        overlay: IndexMap::new(),
        location: ActionLocation::new(signal, LifecyclePath::root(signal.property_name())),
    }
}

impl<'w> Host<'w> {
    fn new(world: &'w World) -> Self {
        Self {
            world,
            lines: Vec::new(),
            launches: 0,
            disk_model: world.root.document.frontmatter.get("model").cloned(),
            race: None,
            reported: None,
        }
    }

    /// Drives one subject's run to its finish.
    fn drive(&mut self, subject: &'w Subject) -> Finish {
        let mut run = Run::start(settings(self.world.cancellation).with_subject(subject.kind), subject.document.authored());
        let mut cx = RunContext {
            documents: vec![subject],
            state: subject.document.data(),
            entry: EntryReason::Initial,
            budget: None,
            launched_model: None,
        };
        loop {
            let request = run.pending();
            let ticket = request.ticket();
            let current: &'w Subject = cx.documents[ticket.document() as usize - 1];
            let reply = match request.kind() {
                RequestKind::Done(finish) => {
                    self.reported = run.selected_error().map(|error| error.label().to_string());
                    return *finish;
                }
                RequestKind::EvaluateEvent(event) => {
                    let (outcome, iterate) = self.evaluate(&run, &ticket, current, event, &mut cx.state);
                    if event.signal == LifecycleSignal::Loop {
                        Reply::Gate { outcome, iterate }
                    } else {
                        Reply::Event(outcome)
                    }
                }
                RequestKind::Prepare { entry } => {
                    cx.entry = *entry;
                    Reply::Prepared(match current.preflight {
                        Some(code) => Preparation::Failed(HostFailure {
                            stage: FailureStage::Preflight,
                            entry: *entry,
                            error: coded(code, code),
                        }),
                        None => Preparation::Ready { epoch: ApprovalEpoch(u64::from(ticket.document())) },
                    })
                }
                RequestKind::PerformOperation { entry } => {
                    cx.entry = *entry;
                    if let Operation::Agent = current.operation {
                        match self.launch(*entry, &mut cx) {
                            Some(report) => Reply::Operation(report),
                            None => {
                                answer(&mut run, Reply::Operation(OperationReport::Started { session: SessionAvailability::Available }));
                                run.cancel().expect("a subject awaiting its work can be cancelled");
                                continue;
                            }
                        }
                    } else {
                        answer(&mut run, Reply::Operation(OperationReport::Started { session: SessionAvailability::Unavailable }));
                        match self.perform(&current.operation) {
                            Some(verdict) => {
                                Reply::Operation(OperationReport::Completed { verdict, session: SessionAvailability::Unavailable })
                            }
                            None => {
                                run.cancel().expect("a subject awaiting its work can be cancelled");
                                continue;
                            }
                        }
                    }
                }
                RequestKind::AwaitCompletion => unreachable!("the host completes a started operation in place"),
                RequestKind::Recover(recovery) => Reply::Recovery(self.recover(recovery, &ticket, &mut cx)),
            };
            answer(&mut run, reply);
        }
    }

    /// The event's outcome, and for `loop` whether another iteration runs.
    fn evaluate(
        &mut self,
        run: &Run,
        ticket: &Ticket,
        subject: &Subject,
        event: &EventRequest,
        state: &mut Map<String, Value>,
    ) -> (LifecycleEventOutcome, bool) {
        let mut eval = Evaluation {
            run,
            ticket,
            prefix: subject.prefix,
            event,
            state: state.clone(),
            outcome: LifecycleEventOutcome::default(),
        };
        let mut iterate = false;
        match (event.signal, subject.document.stack(event.signal)) {
            // Claudine's `loop` key holds the gate's configuration and stack.
            (LifecycleSignal::Loop, Some(config)) => {
                let steps = config.get("gate").and_then(Value::as_array).cloned().unwrap_or_default();
                if self.steps(&mut eval, &steps) {
                    let max = config.get("max").and_then(Value::as_u64).unwrap_or(1);
                    let satisfied = config.get("until") == Some(&json!("true"));
                    iterate = !satisfied && u64::from(ticket.iteration()) < max;
                }
            }
            (_, Some(Value::Array(steps))) => {
                self.steps(&mut eval, steps);
            }
            _ => {}
        }
        *state = eval.state;
        (eval.outcome, iterate)
    }

    /// Runs `steps` in order; `false` when one ended the stack.
    fn steps(&mut self, eval: &mut Evaluation<'_>, steps: &[Value]) -> bool {
        steps.iter().all(|step| self.step(eval, step))
    }

    /// One authored action. The vocabulary mirrors the fixture documents:
    /// `"event"`/`"err"` are their two `append_line`s, `"loop.count"` the
    /// loop-state line, `{"state": key}` a state line, `{"frontmatter":
    /// file}` a guard reading a missing file, and `{"template_shell": key}` a
    /// shell approved only as its template, which reads `key` when it starts.
    fn step(&mut self, eval: &mut Evaluation<'_>, step: &Value) -> bool {
        let name = format!("{}{}", eval.prefix, eval.name());
        if let Some(verb) = step.as_str() {
            match verb {
                "event" => self.lines.push(format!("event {name}")),
                "err" => self.lines.push(format!("err {}", eval.event.error.as_ref().map_or_else(|| "none".to_string(), render))),
                "loop.count" => self.lines.push(format!("state loop.count={}", eval.ticket.iteration())),
                "shell" => match eval.run.shell_permission(eval.ticket) {
                    Ok(_) => self.lines.push(format!("shell {name}")),
                    Err(_) => return eval.raise(uncoded(SHELL_LOCKED)),
                },
                "skip" => return eval.direct(StackControl::Skip),
                "break" => return eval.direct(StackControl::Break { reason: None, code: 0 }),
                other => panic!("unknown step {other}"),
            }
            return true;
        }
        let action = step.as_object().expect("a step is a verb or a one-key action");
        if let Some(condition) = action.get("when") {
            let holds = match condition {
                Value::String(read) if read == "err" => eval.event.error.is_some(),
                Value::String(key) => eval.state.get(key).is_some_and(truthy),
                Value::Object(test) => test.get("loop.count").and_then(Value::as_u64) == Some(u64::from(eval.ticket.iteration())),
                other => panic!("unknown condition {other}"),
            };
            let then = action.get("then").and_then(Value::as_array).cloned().unwrap_or_default();
            return !holds || self.steps(eval, &then);
        }
        let (verb, operand) = action.iter().next().expect("a one-key action");
        let text = || operand.as_str().expect("a string operand");
        let attempts = || u32::try_from(operand.as_u64().expect("an attempt count")).expect("a small attempt count");
        match verb.as_str() {
            "set" => {
                for (key, value) in operand.as_object().expect("a `set` map") {
                    eval.state.insert(key.clone(), value.clone());
                }
            }
            "state" => {
                let key = text();
                let value = eval.state.get(key).map_or_else(|| "null".to_string(), render_value);
                self.lines.push(format!("state {key}={value}"));
            }
            "template_shell" => {
                let key = text();
                if eval.run.shell_permission(eval.ticket).is_err() {
                    return eval.raise(uncoded(SHELL_LOCKED));
                }
                // The host fills the template: a placeholder that reads `null`
                // when the action starts is a missing value, and nothing runs.
                match eval.state.get(key).filter(|value| !value.is_null()) {
                    Some(value) => self.lines.push(format!("shell {} {}", eval.name(), render_value(value))),
                    None => return eval.raise(missing_value(key)),
                }
            }
            "set_frontmatter" => self.disk_model = Some(operand.clone()),
            "frontmatter" => return eval.raise(uncoded(&format!("invalid file path: {}", text()))),
            "error" => return eval.direct(StackControl::Error { reason: Some(text().to_string()) }),
            "retry" => return eval.direct(retry(attempts())),
            "resume" => return eval.direct(resume(attempts())),
            "proxy" => {
                let control = proxy(eval.event.signal, text());
                return eval.direct(control);
            }
            "defer" => return eval.direct(StackControl::Defer { delay: text().to_string(), reason: None }),
            other => panic!("unknown action {other}"),
        }
        true
    }

    /// The launch's report, or `None` when it was interrupted while running.
    fn launch(&mut self, entry: EntryReason, cx: &mut RunContext<'w>) -> Option<OperationReport> {
        if entry == EntryReason::Resume && cx.launched_model != self.disk_model {
            return Some(OperationReport::NotStarted(HostFailure {
                stage: FailureStage::ResumeIncompatible,
                entry,
                error: coded(COMPOSITION_FAILED, "the session's launch identity changed"),
            }));
        }
        let exits = self.world.provider_exits;
        let status = exits[self.launches.min(exits.len() - 1)];
        self.launches += 1;
        cx.launched_model = self.disk_model.clone();
        let mode = if entry == EntryReason::Resume { "resume" } else { "fresh" };
        self.lines.push(format!("work {mode} {status}"));
        if status == INTERRUPTED {
            return None;
        }
        let verdict = if status == 0 {
            CompletionVerdict::Succeeded
        } else {
            CompletionVerdict::Failed(coded("provider.exited", "the provider exited without a result"))
        };
        Some(OperationReport::Completed { verdict, session: SessionAvailability::Available })
    }

    fn recover(&mut self, recovery: &RecoveryRequest, ticket: &Ticket, cx: &mut RunContext<'w>) -> RecoveryReply {
        let failure = |stage, code| HostFailure { stage, entry: cx.entry, error: coded(code, code) };
        match &recovery.control {
            StackControl::Retry { max_attempts, .. } | StackControl::Resume { max_attempts, .. } => {
                RecoveryReply::Budget { ceiling: *cx.budget.get_or_insert(ticket.attempt() + max_attempts) }
            }
            StackControl::Proxy { target, .. } => match self.world.targets.iter().find(|(path, _)| path == target) {
                None => RecoveryReply::Refused(failure(FailureStage::HandoffRefused, "composition.invalid_file_reference")),
                Some((_, target)) if target.initialize_runs_shell() => {
                    RecoveryReply::Aborted(failure(FailureStage::Setup, "composition.lifecycle_parse"))
                }
                Some((_, target)) => {
                    cx.documents.push(target);
                    cx.state = target.document.data();
                    cx.budget = None;
                    RecoveryReply::Adopted(target.document.authored())
                }
            },
            other => unreachable!("{other:?} is not a recovery"),
        }
    }

    /// A container's work; `None` when the subject must be cancelled.
    fn perform(&mut self, operation: &'w Operation) -> Option<CompletionVerdict> {
        let failed = || CompletionVerdict::Failed(coded(COMPOSITION_FAILED, "a task failed"));
        match operation {
            Operation::Agent => unreachable!("an agent subject launches its provider"),
            Operation::Tasks(tasks) => {
                for task in tasks {
                    if !self.task(task)? {
                        return Some(failed());
                    }
                }
                Some(CompletionVerdict::Succeeded)
            }
            Operation::Race { leg, sibling, fail_fast } => {
                let cancel = *fail_fast && leg.document.frontmatter.contains_key("cancelled");
                self.race = Some(Race { sibling, cancel });
                let leg = self.drive(leg);
                assert!(self.race.is_none(), "the leg's waiting shell let the sibling run");
                Some(if sibling.succeeds && leg == Finish::Succeeded { CompletionVerdict::Succeeded } else { failed() })
            }
        }
    }

    /// `Some(succeeded)`, or `None` when the task was cancelled.
    fn task(&mut self, task: &'w Task) -> Option<bool> {
        match task {
            Task::Shell(shell) => Some(self.shell_task(shell)),
            Task::Subject(subject) => match self.drive(subject) {
                Finish::Succeeded | Finish::Skipped => Some(true),
                Finish::Failed => Some(false),
                Finish::Cancelled => None,
            },
            Task::Wait { started, completed } => {
                self.lines.push((*started).to_string());
                let race = self.race.take().expect("a waiting shell runs inside a race");
                if !self.shell_task(race.sibling) && race.cancel {
                    return None;
                }
                self.lines.push((*completed).to_string());
                Some(true)
            }
            Task::Interrupt(line) => {
                self.lines.push((*line).to_string());
                None
            }
        }
    }

    fn shell_task(&mut self, shell: &Shell) -> bool {
        self.lines.push(shell.line.to_string());
        shell.succeeds
    }
}

fn exit_code(finish: Finish) -> i32 {
    match finish {
        Finish::Succeeded | Finish::Skipped => 0,
        Finish::Failed => 1,
        Finish::Cancelled => 130,
    }
}

fn scenario_id(test_name: &str) -> String {
    test_name.replace('_', "-")
}

/// The trace's lines and the replay's, each ending in `exit`; when the world
/// checks the reported error, a `reported <label>` line precedes `exit` in
/// both.
fn replay_lines(id: &str, trace: &str, world: &World) -> (Vec<String>, Vec<String>) {
    assert!(trace.lines().any(|line| line == format!("# scenario: {id}")), "{id}.trace names its scenario");
    let mut expected: Vec<String> =
        trace.lines().map(|line| line.trim_end_matches('\r')).filter(|line| !line.starts_with('#')).map(str::to_owned).collect();
    let mut host = Host::new(world);
    let finish = host.drive(&world.root);
    if let Some(label) = world.reported {
        let exit = expected.pop().expect("a trace ends with its exit line");
        expected.extend([format!("reported {label}"), exit]);
        host.lines.push(format!("reported {}", host.reported.as_deref().unwrap_or("none")));
    }
    host.lines.push(format!("exit {}", exit_code(finish)));
    (expected, host.lines)
}

fn replay(id: &str, trace: &str, world: World) {
    let (expected, replayed) = replay_lines(id, trace, &world);
    if KNOWN_DIVERGENT.contains(&id) {
        assert_ne!(replayed, expected, "replay of {id} now matches production: remove it from KNOWN_DIVERGENT");
    } else {
        assert_eq!(replayed, expected, "replay of {id}");
    }
}

/// The trace scenarios, one `#[test]` per row; each row's `include_str!`
/// stays inside its test so a fixture edit schedules exactly that test.
macro_rules! replay_traces {
    ($(#[test] fn $name:ident() { $trace:expr })*) => {
        fn scenarios() -> BTreeSet<String> {
            [$(stringify!($name)),*].into_iter().map(scenario_id).collect()
        }

        /// Every scenario's id, trace, and world.
        fn cases() -> Vec<(String, &'static str, fn() -> World)> {
            vec![$((scenario_id(stringify!($name)), $trace, worlds::$name as fn() -> World)),*]
        }

        $(
            #[test]
            fn $name() {
                replay(&scenario_id(stringify!($name)), $trace, worlds::$name());
            }
        )*
    };
}

replay_traces! {
    #[test] fn blocked_preflight() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/blocked-preflight.trace") }
    #[test] fn blocked_refused_handoff() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/blocked-refused-handoff.trace") }
    #[test] fn cancelled_not_opted_in() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/cancelled-not-opted-in.trace") }
    #[test] fn cancelled_with_handler() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/cancelled-with-handler.trace") }
    #[test] fn cancelled_without_handler() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/cancelled-without-handler.trace") }
    #[test] fn catch_blocked() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/catch-blocked.trace") }
    #[test] fn catch_failure() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/catch-failure.trace") }
    #[test] fn catch_finalize() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/catch-finalize.trace") }
    #[test] fn catch_initialize() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/catch-initialize.trace") }
    #[test] fn catch_loop() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/catch-loop.trace") }
    #[test] fn catch_start() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/catch-start.trace") }
    #[test] fn catch_success() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/catch-success.trace") }
    #[test] fn downgrade_blocked() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/downgrade-blocked.trace") }
    #[test] fn downgrade_success() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/downgrade-success.trace") }
    #[test] fn finalize_error() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/finalize-error.trace") }
    #[test] fn loop_break() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/loop-break.trace") }
    #[test] fn proxy_adopted() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/proxy-adopted.trace") }
    #[test] fn proxy_target_bootstrap_failed() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/proxy-target-bootstrap-failed.trace") }
    #[test] fn resume_admitted() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/resume-admitted.trace") }
    #[test] fn resume_incompatible() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/resume-incompatible.trace") }
    #[test] fn resume_no_session() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/resume-no-session.trace") }
    #[test] fn retry_admitted() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/retry-admitted.trace") }
    #[test] fn retry_exhausted() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/retry-exhausted.trace") }
    #[test] fn retry_in_finalize() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/retry-in-finalize.trace") }
    #[test] fn retry_in_initialize() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize.trace") }
    #[test] fn retry_in_preflight_blocked() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/retry-in-preflight-blocked.trace") }
    #[test] fn retry_in_start() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/retry-in-start.trace") }
    #[test] fn sequence_nested_group() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/sequence-nested-group.trace") }
    #[test] fn shell_locked_blocked() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/shell-locked-blocked.trace") }
    #[test] fn shell_relocked_after_proxy() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/shell-relocked-after-proxy.trace") }
    #[test] fn shell_unlocked_blocked_after_start() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/shell-unlocked-blocked-after-start.trace") }
    #[test] fn guarded_shell_null() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-null.trace") }
    #[test] fn guarded_shell_present() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-present.trace") }
    #[test] fn skip() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/skip.trace") }
    #[test] fn success() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/success.trace") }
    #[test] fn cancelled_root_document() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-document.trace") }
    #[test] fn cancelled_root_document_unauthored() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-document-unauthored.trace") }
    #[test] fn cancelled_root_sequence() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/cancelled-root-sequence.trace") }
    #[test] fn defer_in_preflight_blocked() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/defer-in-preflight-blocked.trace") }
    #[test] fn proxy_hop_limit() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/proxy-hop-limit.trace") }
    #[test] fn proxy_hop_limit_after_launch() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/proxy-hop-limit-after-launch.trace") }
    #[test] fn proxy_in_initialize() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/proxy-in-initialize.trace") }
    #[test] fn resume_in_initialize() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/resume-in-initialize.trace") }
    #[test] fn resume_in_preflight_blocked() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/resume-in-preflight-blocked.trace") }
    #[test] fn retry_in_initialize_group_leg() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize-group-leg.trace") }
    #[test] fn retry_in_initialize_root_sequence() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize-root-sequence.trace") }
}

#[test]
fn every_recorded_trace_has_a_replay() {
    let fixtures = biscuit_test_harness::manifest_dir!().join("../../claudine/cli/tests/fixtures/lifecycle_traces");
    let recorded: BTreeSet<String> = fs::read_dir(&fixtures)
        .unwrap_or_else(|error| panic!("{}: {error}", fixtures.display()))
        .map(|entry| entry.expect("a readable fixture entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "trace"))
        .map(|path| path.file_stem().expect("a named trace").to_string_lossy().into_owned())
        .collect();
    assert_eq!(scenarios(), recorded);
}

/// Fails once `Run` reproduces a scenario listed in [`KNOWN_DIVERGENT`], or
/// when the list names a scenario that does not exist.
#[test]
fn known_divergences_still_diverge() {
    let cases = cases();
    for id in KNOWN_DIVERGENT {
        let (_, trace, world) = cases.iter().find(|(case, _, _)| case == id).unwrap_or_else(|| panic!("{id} is not a replayed scenario"));
        let (expected, replayed) = replay_lines(id, trace, &world());
        assert_ne!(replayed, expected, "replay of {id} now matches production: remove it from KNOWN_DIVERGENT");
    }
}

/// Each scenario's documents (its `.md` fixtures) and scripted host facts
/// (its `# given:` lines).
mod worlds {
    use serde_json::{Value, json};

    use super::{COMPOSITION_FAILED, DocumentInput, INTERRUPTED, Operation, Shell, Subject, SubjectKind, Task, World};

    /// The stacks every fixture authors: each event writes its name, and the
    /// error-reading events write `err`.
    fn document(label: &str) -> DocumentInput {
        DocumentInput::new(label)
            .with("initialize", json!(["event"]))
            .with("start", json!(["event"]))
            .with("success", json!(["event"]))
            .with("blocked", json!(["event", "err"]))
            .with("failure", json!(["event", "err"]))
            .with("finalize", json!(["event", "err"]))
    }

    fn entry(document: DocumentInput) -> Subject {
        Subject::new("", document)
    }

    fn target(document: DocumentInput) -> Subject {
        Subject::new("target:", document)
    }

    fn container(document: DocumentInput) -> DocumentInput {
        document.with("cancelled", json!(["event", "err"]))
    }

    fn shell(line: &'static str, succeeds: bool) -> Shell {
        Shell { line, succeeds }
    }

    fn frontmatter_guard(steps: Value, file: &str) -> Value {
        let mut steps = steps;
        steps.as_array_mut().expect("a stack").push(json!({ "frontmatter": file }));
        steps
    }

    /// The start shell is not approved, so the shell audit denies it.
    fn shell_denied(document: DocumentInput) -> Subject {
        entry(document.with("start", json!(["event", "shell"]))).preflight("composition.shell_approval")
    }

    pub(super) fn blocked_preflight() -> World {
        World::new(shell_denied(document("blocked-preflight.md")))
    }

    pub(super) fn blocked_refused_handoff() -> World {
        World::new(entry(
            document("blocked-refused-handoff.md").with("start", json!(["event", { "proxy": "./blocked-refused-handoff.missing.md" }])),
        ))
    }

    /// `seq` runs the `fail_fast` group `checks`, whose leg `opted` waits on
    /// a shell while its sibling `bad` fails, then `later`.
    fn fail_fast_sequence(label: &str, opted: Subject) -> World {
        let checks = Subject::new("checks:", container(document("checks"))).operation(Operation::Race {
            leg: Box::new(opted),
            sibling: shell("task bad", false),
            fail_fast: true,
        });
        World::new(
            Subject::new("seq:", container(document(label)))
                .operation(Operation::Tasks(vec![Task::Subject(checks), Task::Shell(shell("task later", true))])),
        )
    }

    fn waiting(started: &'static str, completed: &'static str) -> Operation {
        Operation::Tasks(vec![Task::Wait { started, completed }])
    }

    pub(super) fn cancelled_not_opted_in() -> World {
        fail_fast_sequence("cancelled-not-opted-in.md", Subject::new("opted:", document("opted")).operation(waiting("task opted waiting", "task opted completed")))
    }

    pub(super) fn cancelled_with_handler() -> World {
        fail_fast_sequence("cancelled-with-handler.md", Subject::new("opted:", container(document("opted"))).operation(waiting("task opted waiting", "task opted completed")))
    }

    pub(super) fn cancelled_without_handler() -> World {
        let inner = Subject::new("inner:", document("inner")).operation(waiting("task inner waiting", "task inner completed"));
        let opted = Subject::new("opted:", document("opted").with("cancelled", json!([])))
            .operation(Operation::Tasks(vec![Task::Subject(inner)]));
        fail_fast_sequence("cancelled-without-handler.md", opted)
    }

    pub(super) fn catch_blocked() -> World {
        let document = document("catch-blocked.md");
        let blocked = frontmatter_guard(document.frontmatter["blocked"].clone(), "nope-blocked.md");
        World::new(shell_denied(document.with("blocked", blocked)))
    }

    pub(super) fn catch_failure() -> World {
        let document = document("catch-failure.md");
        let failure = frontmatter_guard(document.frontmatter["failure"].clone(), "nope-failure.md");
        World::new(entry(document.with("failure", failure))).exits(&[1])
    }

    pub(super) fn catch_finalize() -> World {
        let document = document("catch-finalize.md");
        let finalize = frontmatter_guard(document.frontmatter["finalize"].clone(), "nope-finalize.md");
        World::new(entry(document.with("finalize", finalize)))
    }

    pub(super) fn catch_initialize() -> World {
        World::new(entry(document("catch-initialize.md").with("initialize", json!(["event", { "frontmatter": "nope-initialize.md" }]))))
    }

    pub(super) fn catch_loop() -> World {
        World::new(entry(
            document("catch-loop.md")
                .with("loop", json!({ "until": "false", "max": 2, "gate": ["event", { "frontmatter": "nope-loop.md" }] }))
                .with("start", json!(["event", "loop.count"])),
        ))
    }

    pub(super) fn catch_start() -> World {
        World::new(entry(document("catch-start.md").with("start", json!(["event", { "frontmatter": "nope-start.md" }]))))
    }

    pub(super) fn catch_success() -> World {
        World::new(entry(document("catch-success.md").with("success", json!(["event", { "frontmatter": "nope-success.md" }]))))
    }

    pub(super) fn downgrade_blocked() -> World {
        World::new(shell_denied(document("downgrade-blocked.md").with("blocked", json!(["event", "err", { "error": "blocked downgraded" }]))))
    }

    pub(super) fn downgrade_success() -> World {
        World::new(entry(document("downgrade-success.md").with("success", json!(["event", { "error": "success downgraded" }]))))
    }

    pub(super) fn finalize_error() -> World {
        World::new(entry(document("finalize-error.md").with("finalize", json!(["event", "err", { "error": "finalize raised" }]))))
    }

    pub(super) fn loop_break() -> World {
        World::new(entry(
            document("loop-break.md")
                .with("loop", json!({ "until": "false", "max": 3 }))
                .with("start", json!(["event", "loop.count"]))
                .with("success", json!(["event", { "when": { "loop.count": 2 }, "then": ["break"] }])),
        ))
    }

    pub(super) fn proxy_adopted() -> World {
        World::new(entry(document("proxy-adopted.md").with("success", json!(["event", { "proxy": "./proxy-adopted.target.md" }]))))
            .target("./proxy-adopted.target.md", target(document("proxy-adopted.target.md")))
    }

    pub(super) fn proxy_target_bootstrap_failed() -> World {
        let path = "./proxy-target-bootstrap-failed.target.md";
        World::new(entry(document("proxy-target-bootstrap-failed.md").with("success", json!(["event", { "proxy": path }]))))
            .target(path, target(document(path).with("initialize", json!(["event", "shell"]))))
    }

    pub(super) fn resume_admitted() -> World {
        World::new(entry(document("resume-admitted.md").with("failure", json!(["event", "err", { "resume": 1 }])))).exits(&[1, 0])
    }

    /// `failure` rewrites the model on disk before resuming, so the resumed
    /// launch's identity no longer matches the session's.
    pub(super) fn resume_incompatible() -> World {
        World::new(entry(
            document("resume-incompatible.md")
                .with("model", json!("claude-model-one"))
                .with("failure", json!(["event", "err", { "set_frontmatter": "claude-model-two" }, { "resume": 1 }])),
        ))
        .exits(&[1])
    }

    pub(super) fn resume_no_session() -> World {
        World::new(entry(document("resume-no-session.md").with("start", json!(["event", { "resume": 1 }]))))
    }

    fn retry_on_failure(label: &str) -> Subject {
        entry(document(label).with("failure", json!(["event", "err", { "retry": 1 }])))
    }

    pub(super) fn retry_admitted() -> World {
        World::new(retry_on_failure("retry-admitted.md")).exits(&[1, 0])
    }

    pub(super) fn retry_exhausted() -> World {
        World::new(retry_on_failure("retry-exhausted.md")).exits(&[1])
    }

    pub(super) fn retry_in_finalize() -> World {
        World::new(entry(
            document("retry-in-finalize.md").with("finalize", json!(["event", "err", { "when": "err", "then": [{ "retry": 1 }] }])),
        ))
        .exits(&[1, 0])
    }

    /// `max_parallel: 1` runs the group's legs in declaration order.
    pub(super) fn retry_in_initialize() -> World {
        World::new(entry(document("retry-in-initialize.md").with("initialize", json!(["event", { "retry": 1 }]))))
    }

    pub(super) fn retry_in_preflight_blocked() -> World {
        World::new(shell_denied(document("retry-in-preflight-blocked.md").with("blocked", json!(["event", "err", { "retry": 1 }]))))
    }

    pub(super) fn retry_in_start() -> World {
        World::new(entry(document("retry-in-start.md").with("start", json!(["event", { "retry": 1 }]))))
    }

    pub(super) fn sequence_nested_group() -> World {
        let leg = |prefix: &'static str, line: &'static str| {
            Task::Subject(Subject::new(prefix, document(prefix)).operation(Operation::Tasks(vec![Task::Shell(shell(line, true))])))
        };
        let checks = Subject::new("checks:", document("checks")).operation(Operation::Tasks(vec![
            leg("leg-a:", "task leg-a"),
            leg("leg-b:", "task leg-b"),
        ]));
        World::new(Subject::new("seq:", document("sequence-nested-group.md")).operation(Operation::Tasks(vec![
            Task::Subject(Subject::new("step:", document("sequence-nested-group.step.md"))),
            Task::Subject(checks),
            Task::Shell(shell("task last", true)),
        ])))
    }

    /// Shells in `blocked` and `finalize`, with the start shell approved.
    fn shells_everywhere(document: DocumentInput) -> DocumentInput {
        document
            .with("blocked", json!(["event", "err", "shell"]))
            .with("finalize", json!(["event", "err", "shell"]))
    }

    pub(super) fn shell_locked_blocked() -> World {
        World::new(
            entry(shells_everywhere(document("shell-locked-blocked.md")).with("start", json!(["event", "shell"])))
                .preflight(COMPOSITION_FAILED),
        )
    }

    pub(super) fn shell_relocked_after_proxy() -> World {
        let path = "./shell-relocked-after-proxy.target.md";
        World::new(entry(document("shell-relocked-after-proxy.md").with("start", json!(["event", "shell", { "proxy": path }]))))
            .target(path, target(shells_everywhere(document(path)).with("start", json!(["event", "shell"]))).preflight(COMPOSITION_FAILED))
    }

    pub(super) fn shell_unlocked_blocked_after_start() -> World {
        World::new(entry(
            shells_everywhere(document("shell-unlocked-blocked-after-start.md"))
                .with("start", json!(["event", "shell", { "proxy": "./shell-unlocked-blocked-after-start.missing.md" }])),
        ))
    }

    pub(super) fn guarded_shell_present() -> World {
        World::new(entry(
            document("guarded-shell-present.md")
                .with("note", json!("hello"))
                .with("start", json!(["event", { "when": "note", "then": [{ "template_shell": "note" }] }])),
        ))
    }

    pub(super) fn guarded_shell_null() -> World {
        World::new(entry(
            document("guarded-shell-null.md").with("note", json!("hello")).with(
                "start",
                json!(["event", { "when": "note", "then": [{ "set": { "note": null } }, { "state": "note" }, { "template_shell": "note" }] }]),
            ),
        ))
    }

    pub(super) fn skip() -> World {
        World::new(entry(document("skip.md").with("initialize", json!(["event", "skip"]))))
    }

    pub(super) fn success() -> World {
        World::new(entry(
            document("success.md")
                .with("counter", json!(0))
                .with("initialize", json!(["event", { "set": { "counter": 1 } }, { "state": "counter" }]))
                .with("start", json!(["event", "shell"]))
                .with("success", json!(["event", { "set": { "counter": 2 } }, { "state": "counter" }]))
                .with("finalize", json!(["event", "err", { "state": "counter" }, "shell"])),
        ))
    }

    pub(super) fn resume_in_preflight_blocked() -> World {
        World::new(shell_denied(document("resume-in-preflight-blocked.md").with("blocked", json!(["event", "err", { "resume": 1 }]))))
            .reports("resume-without-session")
    }

    pub(super) fn defer_in_preflight_blocked() -> World {
        World::new(shell_denied(document("defer-in-preflight-blocked.md").with("blocked", json!(["event", "err", { "defer": "1h" }]))))
            .reports("defer-unsupported")
    }

    pub(super) fn resume_in_initialize() -> World {
        World::new(entry(document("resume-in-initialize.md").with("initialize", json!(["event", { "resume": 1 }]))))
            .reports("resume-without-session")
    }

    pub(super) fn proxy_in_initialize() -> World {
        let path = "./proxy-in-initialize.target.md";
        World::new(entry(document("proxy-in-initialize.md").with("initialize", json!(["event", { "proxy": path }]))))
            .target(path, target(document(path)))
    }

    pub(super) fn cancelled_root_document() -> World {
        World::new(entry(container(document("cancelled-root-document.md")))).exits(&[INTERRUPTED]).interrupted()
    }

    pub(super) fn cancelled_root_document_unauthored() -> World {
        World::new(entry(document("cancelled-root-document-unauthored.md"))).exits(&[INTERRUPTED]).interrupted()
    }

    pub(super) fn cancelled_root_sequence() -> World {
        World::new(Subject::new("seq:", container(document("cancelled-root-sequence.md"))).operation(Operation::Tasks(vec![
            Task::Interrupt("task interrupt"),
            Task::Shell(shell("task later", true)),
        ])))
        .interrupted()
    }

    /// The entry and every hop document proxy to the next hop from `start`;
    /// `after_launch` names the one hop that proxies from `success` instead.
    /// As in the trace test, `MAX_PROXY_HOPS` (16) hop documents exist.
    fn proxy_chain(id: &'static str, after_launch: Option<usize>) -> World {
        const HOP_DOCUMENTS: usize = 16;
        let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
        let hop_path = |hop: usize| leak(format!("./{id}.hop-{hop:02}.md"));
        let mut world = World::new(entry(document(id).with("start", json!(["event", { "proxy": hop_path(1) }]))));
        for hop in 1..=HOP_DOCUMENTS {
            let next = hop_path(hop + 1);
            let hop_document = document(hop_path(hop))
                .with("proxy_before_launch", json!(after_launch != Some(hop)))
                .with("start", json!(["event", { "when": "proxy_before_launch", "then": [{ "proxy": next }] }]))
                .with("success", json!(["event", { "proxy": next }]));
            world = world.target(hop_path(hop), Subject::new(leak(format!("hop-{hop:02}:")), hop_document));
        }
        world
    }

    pub(super) fn proxy_hop_limit() -> World {
        proxy_chain("proxy-hop-limit", None).reports("proxy-budget-exhausted")
    }

    /// Hop 15, the last document production adopts, launches before it
    /// proxies.
    pub(super) fn proxy_hop_limit_after_launch() -> World {
        proxy_chain("proxy-hop-limit-after-launch", Some(15)).reports("proxy-budget-exhausted")
    }

    pub(super) fn retry_in_initialize_root_sequence() -> World {
        World::new(
            Subject::new("seq:", container(document("retry-in-initialize-root-sequence.md")).with("initialize", json!(["event", { "retry": 1 }])))
                .operation(Operation::Tasks(vec![Task::Shell(shell("task only", true))])),
        )
        .reports("setup-recovery-unsupported")
    }

    pub(super) fn retry_in_initialize_group_leg() -> World {
        // The leg is a nested sequence (a task with its own `sequence:`).
        let leg = Subject::new("leg:", container(document("leg")).with("initialize", json!(["event", { "retry": 1 }])))
            .kind(SubjectKind::Sequence)
            .operation(Operation::Tasks(vec![Task::Shell(shell("task leg", true))]));
        let checks = Subject::new("checks:", container(document("checks"))).operation(Operation::Tasks(vec![Task::Subject(leg)]));
        World::new(Subject::new("seq:", container(document("retry-in-initialize-group-leg.md"))).operation(Operation::Tasks(vec![
            Task::Subject(checks),
            Task::Shell(shell("task later", true)),
        ])))
    }
}
