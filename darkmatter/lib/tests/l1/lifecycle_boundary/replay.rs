//! The Stage 1 gate: every production trace recorded by
//! `claudine/cli/tests/l1/lifecycle_traces.rs` replayed through the prototype
//! engine by a scripted Claudine-profile host.
//!
//! The host answers each request with facts only: an event's outcome
//! (mutations, an evaluation error, a directive), preparation readiness or a
//! failure with its stage and entry, an operation's outcome and session, and a
//! recovery's admission, exhaustion, refusal, adoption, or abort. It also writes
//! the trace lines production's authored actions and fake services write:
//! `event`, `err` (rendered from the request's `err` binding), `state`, `shell`
//! (only when [`Run::shell_permission`] grants it), `work`, and `task`. Each
//! scenario's documents are its `.md` fixtures transcribed into the small stack
//! vocabulary [`Host::step`] interprets; its facts are the trace's `# given:`
//! lines. Claudine's own concerns stay here: retry/resume budgets, sessions,
//! proxy target resolution and parsing, and container scheduling (one [`Run`]
//! per subject, sequence order, `fail_fast` cancellation through
//! [`Run::cancel`]).

use std::collections::BTreeSet;
use std::fs;

use serde_json::{Map, Value, json};

use super::engine::{
    ApprovalEpoch, Directive, DocumentInput, Entry, EventOutcome, EventRequest, FailureStage, Finish, HostError,
    HostFailure, LifecycleError, OperationReport, Outcome, Preparation, RecoveryKind, RecoveryReply, RecoveryRequest,
    Registry, Reply, RequestKind, Run, SessionAvailability, Ticket,
};
use super::profiles::claudine_profile;

/// Claudine's refusal of a lifecycle shell before `start`; it is raised like
/// an evaluation error.
const SHELL_LOCKED: &str =
    "shell commands are forbidden during initialize and before preflight completes; move the command to start or a later event";

/// The code a failed sequence task or group reports to its parent subject.
const COMPOSITION_FAILED: &str = "composition.failed";

/// One lifecycle subject: a document, step, group, leg, or nested sequence.
struct Subject {
    prefix: &'static str,
    document: DocumentInput,
    /// The code every attempt's preflight fails with.
    preflight: Option<&'static str>,
    operation: Operation,
}

impl Subject {
    fn new(prefix: &'static str, document: DocumentInput) -> Self {
        Self { prefix, document, preflight: None, operation: Operation::Agent }
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
    /// One provider launch per attempt.
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
}

impl World {
    fn new(root: Subject) -> Self {
        Self { root, provider_exits: &[0], targets: Vec::new() }
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
    /// The entry of the latest preparation or operation request, which a
    /// failure must name.
    entry: Entry,
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
    registry: Registry,
    lines: Vec<String>,
    launches: usize,
    /// The entry document's `model` on disk; `set_frontmatter` rewrites it.
    disk_model: Option<Value>,
    race: Option<Race<'w>>,
}

/// One event's evaluation: the request plus the stack's local view of state.
struct Evaluation<'a> {
    run: &'a Run,
    ticket: &'a Ticket,
    prefix: &'a str,
    event: &'a EventRequest,
    state: Map<String, Value>,
    outcome: EventOutcome,
}

impl Evaluation<'_> {
    /// Ends the stack with an evaluation error.
    fn raise(&mut self, error: LifecycleError) -> bool {
        self.outcome.evaluation_error = Some(error);
        false
    }

    /// Ends the stack with a directive.
    fn direct(&mut self, directive: Directive) -> bool {
        self.outcome.directive = Some(directive);
        false
    }
}

fn uncoded(message: &str) -> LifecycleError {
    LifecycleError::Host(HostError::new("", message))
}

/// The fixtures' `err.code || 'uncoded: ' + err.msg`.
fn render(error: &LifecycleError) -> String {
    match error {
        LifecycleError::Host(error) if error.code.is_empty() => format!("uncoded: {}", error.message),
        other => other.code().to_string(),
    }
}

fn render_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn answer(run: &mut Run, reply: Reply) {
    run.answer(reply).unwrap_or_else(|error| panic!("{error:?} after {:?}", run.transcript_lines()));
}

impl<'w> Host<'w> {
    fn new(world: &'w World) -> Self {
        Self {
            world,
            registry: claudine_profile(),
            lines: Vec::new(),
            launches: 0,
            disk_model: world.root.document.frontmatter.get("model").cloned(),
            race: None,
        }
    }

    /// Drives one subject's run to its finish.
    fn drive(&mut self, subject: &'w Subject) -> Finish {
        let mut run = Run::start(&self.registry, subject.document.clone()).expect("fixture documents start");
        let mut cx = RunContext { documents: vec![subject], entry: Entry::Initial, budget: None, launched_model: None };
        loop {
            let request = run.pending();
            let ticket = request.ticket();
            let current: &'w Subject = cx.documents[ticket.document() as usize - 1];
            let reply = match request.kind() {
                RequestKind::Done(finish) => return *finish,
                RequestKind::EvaluateEvent(event) => Reply::Event(self.evaluate(&run, &ticket, current.prefix, event)),
                RequestKind::Prepare { entry } => {
                    cx.entry = *entry;
                    Reply::Prepared(match current.preflight {
                        Some(code) => Preparation::Failed(HostFailure {
                            stage: FailureStage::Preflight,
                            entry: *entry,
                            error: HostError::new(code, code),
                        }),
                        None => Preparation::Ready { epoch: ApprovalEpoch(u64::from(ticket.document())) },
                    })
                }
                RequestKind::PerformOperation { entry } => {
                    cx.entry = *entry;
                    if let Operation::Agent = current.operation {
                        Reply::Operation(self.launch(*entry, &mut cx))
                    } else {
                        answer(&mut run, Reply::Operation(OperationReport::Started { session: SessionAvailability::Unavailable }));
                        match self.perform(&current.operation) {
                            Some(outcome) => {
                                Reply::Operation(OperationReport::Completed { outcome, session: SessionAvailability::Unavailable })
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

    fn evaluate(&mut self, run: &Run, ticket: &Ticket, prefix: &str, event: &EventRequest) -> EventOutcome {
        let mut eval = Evaluation { run, ticket, prefix, event, state: event.state.clone(), outcome: EventOutcome::default() };
        match (event.name.as_str(), &event.stack) {
            // Claudine's `loop` key holds the gate's configuration and stack.
            ("loop", Some(config)) => {
                let steps = config.get("gate").and_then(Value::as_array).cloned().unwrap_or_default();
                if self.steps(&mut eval, &steps) {
                    let max = config.get("max").and_then(Value::as_u64).unwrap_or(1);
                    let satisfied = config.get("until") == Some(&json!("true"));
                    if !satisfied && u64::from(ticket.iteration()) < max {
                        eval.outcome.directive = Some(Directive::Iterate);
                    }
                }
            }
            (_, Some(Value::Array(steps))) => {
                self.steps(&mut eval, steps);
            }
            _ => {}
        }
        eval.outcome
    }

    /// Runs `steps` in order; `false` when one ended the stack.
    fn steps(&mut self, eval: &mut Evaluation<'_>, steps: &[Value]) -> bool {
        steps.iter().all(|step| self.step(eval, step))
    }

    /// One authored action. The vocabulary mirrors the fixture documents:
    /// `"event"`/`"err"` are their two `append_line`s, `"loop.count"` the
    /// loop-state line, `{"state": key}` a state line, and `{"frontmatter":
    /// file}` a guard reading a missing file.
    fn step(&mut self, eval: &mut Evaluation<'_>, step: &Value) -> bool {
        let name = format!("{}{}", eval.prefix, eval.event.name);
        if let Some(verb) = step.as_str() {
            match verb {
                "event" => self.lines.push(format!("event {name}")),
                "err" => match eval.event.read_err() {
                    Ok(error) => self.lines.push(format!("err {}", error.map_or_else(|| "none".to_string(), render))),
                    Err(unavailable) => return eval.raise(unavailable),
                },
                "loop.count" => self.lines.push(format!("state loop.count={}", eval.ticket.iteration())),
                "shell" => match eval.run.shell_permission(eval.ticket) {
                    Ok(_) => self.lines.push(format!("shell {name}")),
                    Err(_) => return eval.raise(uncoded(SHELL_LOCKED)),
                },
                "skip" => return eval.direct(Directive::Skip),
                "break" => return eval.direct(Directive::Break),
                other => panic!("unknown step {other}"),
            }
            return true;
        }
        let action = step.as_object().expect("a step is a verb or a one-key action");
        if let Some(condition) = action.get("when") {
            let holds = match condition {
                Value::String(read) if read == "err" => matches!(eval.event.read_err(), Ok(Some(_))),
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
                    eval.outcome.mutations.push((key.clone(), value.clone()));
                }
            }
            "state" => {
                let key = text();
                let value = eval.state.get(key).map_or_else(|| "null".to_string(), render_value);
                self.lines.push(format!("state {key}={value}"));
            }
            "set_frontmatter" => self.disk_model = Some(operand.clone()),
            "frontmatter" => return eval.raise(uncoded(&format!("invalid file path: {}", text()))),
            "error" => return eval.direct(Directive::Error(uncoded(text()))),
            "retry" => return eval.direct(Directive::Retry { max_attempts: attempts() }),
            "resume" => return eval.direct(Directive::Resume { max_attempts: attempts() }),
            "proxy" => return eval.direct(Directive::Proxy { target: text().to_string() }),
            other => panic!("unknown action {other}"),
        }
        true
    }

    fn launch(&mut self, entry: Entry, cx: &mut RunContext<'w>) -> OperationReport {
        if entry == Entry::Resume && cx.launched_model != self.disk_model {
            return OperationReport::NotStarted(HostFailure {
                stage: FailureStage::ResumeIncompatible,
                entry,
                error: HostError::new(COMPOSITION_FAILED, "the session's launch identity changed"),
            });
        }
        let exits = self.world.provider_exits;
        let status = exits[self.launches.min(exits.len() - 1)];
        self.launches += 1;
        cx.launched_model = self.disk_model.clone();
        let mode = if entry == Entry::Resume { "resume" } else { "fresh" };
        self.lines.push(format!("work {mode} {status}"));
        let outcome = if status == 0 {
            Outcome::Succeeded
        } else {
            Outcome::Failed(HostError::new("provider.exited", "the provider exited without a result"))
        };
        OperationReport::Completed { outcome, session: SessionAvailability::Available }
    }

    fn recover(&mut self, recovery: &RecoveryRequest, ticket: &Ticket, cx: &mut RunContext<'w>) -> RecoveryReply {
        let failure = |stage, code| HostFailure { stage, entry: cx.entry, error: HostError::new(code, code) };
        match &recovery.directive {
            Directive::Retry { max_attempts } | Directive::Resume { max_attempts } => {
                let budget = *cx.budget.get_or_insert(ticket.attempt() + max_attempts);
                if ticket.attempt() >= budget {
                    RecoveryReply::Exhausted
                } else if recovery.kind == RecoveryKind::Resume && recovery.session == SessionAvailability::Unavailable {
                    RecoveryReply::Aborted(failure(FailureStage::RecoveryRefused, "composition.lifecycle_resume_without_session"))
                } else {
                    RecoveryReply::Admitted
                }
            }
            Directive::Proxy { target } => match self.world.targets.iter().find(|(path, _)| path == target) {
                None => RecoveryReply::Refused(failure(FailureStage::HandoffRefused, "composition.invalid_file_reference")),
                Some((_, target)) if target.initialize_runs_shell() => {
                    RecoveryReply::Aborted(failure(FailureStage::Setup, "composition.lifecycle_parse"))
                }
                Some((_, target)) => {
                    cx.documents.push(target);
                    cx.budget = None;
                    RecoveryReply::Adopted(target.document.clone())
                }
            },
            other => unreachable!("{other:?} is not a recovery"),
        }
    }

    /// A container's work; `None` when the subject must be cancelled.
    fn perform(&mut self, operation: &'w Operation) -> Option<Outcome> {
        let failed = || Outcome::Failed(HostError::new(COMPOSITION_FAILED, "a task failed"));
        match operation {
            Operation::Agent => unreachable!("an agent subject launches its provider"),
            Operation::Tasks(tasks) => {
                for task in tasks {
                    if !self.task(task)? {
                        return Some(failed());
                    }
                }
                Some(Outcome::Succeeded)
            }
            Operation::Race { leg, sibling, fail_fast } => {
                let cancel = *fail_fast && leg.document.frontmatter.contains_key("cancelled");
                self.race = Some(Race { sibling, cancel });
                let leg = self.drive(leg);
                assert!(self.race.is_none(), "the leg's waiting shell let the sibling run");
                Some(if sibling.succeeds && leg == Finish::Succeeded { Outcome::Succeeded } else { failed() })
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

fn replay(id: &str, trace: &str, world: World) {
    assert!(trace.lines().any(|line| line == format!("# scenario: {id}")), "{id}.trace names its scenario");
    let expected: Vec<&str> = trace.lines().map(|line| line.trim_end_matches('\r')).filter(|line| !line.starts_with('#')).collect();
    let mut host = Host::new(&world);
    let finish = host.drive(&world.root);
    host.lines.push(format!("exit {}", exit_code(finish)));
    assert_eq!(host.lines, expected, "replay of {id}");
}

/// The trace scenarios, one `#[test]` per row; each row's `include_str!`
/// stays inside its test so a fixture edit schedules exactly that test.
macro_rules! replay_traces {
    ($(#[test] fn $name:ident() { $trace:expr })*) => {
        fn scenarios() -> BTreeSet<String> {
            [$(stringify!($name)),*].into_iter().map(scenario_id).collect()
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
    #[test] fn sequence_nested_group() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/sequence-nested-group.trace") }
    #[test] fn shell_locked_blocked() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/shell-locked-blocked.trace") }
    #[test] fn shell_relocked_after_proxy() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/shell-relocked-after-proxy.trace") }
    #[test] fn shell_unlocked_blocked_after_start() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/shell-unlocked-blocked-after-start.trace") }
    #[test] fn skip() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/skip.trace") }
    #[test] fn success() { include_str!("../../../../../claudine/cli/tests/fixtures/lifecycle_traces/success.trace") }
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

/// Each scenario's documents (its `.md` fixtures) and scripted host facts
/// (its `# given:` lines).
mod worlds {
    use serde_json::{Value, json};

    use super::super::engine::DocumentInput;
    use super::{COMPOSITION_FAILED, Operation, Shell, Subject, Task, World};

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
}
