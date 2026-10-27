//! A deterministic, non-agent host: a release publisher. It registers one
//! before-composition notification (`configure`), renames `composed` to
//! `ready`, and pairs `published` with `rejected`, where `published` may
//! downgrade only to `rejected`. Its stacks are evaluated with Darkmatter's
//! real expression engine over the state and `err` binding each request
//! carries; it changes the run only by replying.


use darkmatter::markdown::compose::EffectiveStateBuilder;
use darkmatter::markdown::compose::expression::{
    AuthoredMode, Availability, EvaluationSession, UnavailabilityReason, evaluate_prepared, prepare_value,
};
use serde_json::{Map, Value, json};

use super::engine::{
    ApprovalEpoch, CoreEvent, Directive, DocumentInput, EvaluationMode, EventDescriptor, EventOutcome, EventRequest,
    Finish, HostError, LifecycleError, OperationReport, Outcome, OutcomePair, Position, Preparation, Registry, Reply,
    RequestKind, Role, Run, SessionAvailability, ShellRefusal, StartError, Ticket, TranscriptEntry, err_view,
};

const CONFIGURE_HAS_NO_ERROR: &str = "publisher.configure-precedes-failure";
const PUBLISHED_HAS_NO_ERROR: &str = "publisher.published-has-no-error";
const EPOCH: ApprovalEpoch = ApprovalEpoch(7);

fn publisher_profile() -> Registry {
    let unavailable = |reason| Availability::Unavailable(UnavailabilityReason::new(reason));
    let mut registry = Registry::new("publisher");
    registry.rename(CoreEvent::Composed, "ready").expect("ready is free");
    for descriptor in [
        EventDescriptor::new(
            "configure",
            Position::BeforeComposition,
            Role::Notification,
            err_view("publisher.configure", unavailable(CONFIGURE_HAS_NO_ERROR)),
        ),
        EventDescriptor::new(
            "published",
            Position::AfterComposition,
            Role::TerminalOutcome,
            err_view("publisher.published", unavailable(PUBLISHED_HAS_NO_ERROR)),
        ),
        EventDescriptor::new("rejected", Position::AfterComposition, Role::TerminalOutcome, err_view("publisher.rejected", Availability::Available)),
    ] {
        registry.register(descriptor).expect("publisher events register");
    }
    registry
        .register_outcome_pair(OutcomePair::new("published", "rejected").downgrade("published", "rejected"))
        .expect("the publisher's outcome pair registers");
    registry
}

struct Publisher {
    verdict: Outcome,
    log: Vec<String>,
}

impl Publisher {
    fn new(verdict: Outcome) -> Self {
        Self { verdict, log: Vec::new() }
    }

    fn run(&mut self, document: DocumentInput) -> Run {
        let mut run = Run::start(&publisher_profile(), document).expect("publisher documents start");
        loop {
            let request = run.pending();
            let reply = match request.kind() {
                RequestKind::EvaluateEvent(event) => Reply::Event(self.evaluate(&run, request.ticket(), event)),
                RequestKind::Prepare { .. } => Reply::Prepared(Preparation::Ready { epoch: EPOCH }),
                RequestKind::PerformOperation { .. } => Reply::Operation(OperationReport::Completed {
                    outcome: self.verdict.clone(),
                    session: SessionAvailability::Unavailable,
                }),
                RequestKind::Done(_) => return run,
                other => panic!("the publisher never receives {other:?}"),
            };
            run.reply(request.ticket(), reply).expect("publisher replies are well-formed");
        }
    }

    /// Runs `set`, `info`, `shell`, and `error` actions in order; the first
    /// failure ends the stack.
    fn evaluate(&mut self, run: &Run, ticket: Ticket, event: &EventRequest) -> EventOutcome {
        let mut outcome = EventOutcome::default();
        let mut state = event.state.clone();
        let actions = event.stack.as_ref().and_then(Value::as_array).cloned().unwrap_or_default();
        for action in actions {
            let (verb, operand) = action.as_object().and_then(|action| action.iter().next()).expect("one verb per action");
            match verb.as_str() {
                "set" => {
                    for (key, template) in operand.as_object().expect("set takes a mapping") {
                        match render(template, &state, event) {
                            Ok(value) => {
                                state.insert(key.clone(), value.clone());
                                outcome = outcome.set(key, value);
                            }
                            Err(error) => return outcome.evaluation_error(error),
                        }
                    }
                }
                "info" => match render(operand, &state, event) {
                    Ok(value) => self.log.push(format!("{}: {}", event.name, value.as_str().unwrap_or_default())),
                    Err(error) => return outcome.evaluation_error(error),
                },
                "shell" => match run.shell_permission(&ticket) {
                    Ok(ApprovalEpoch(epoch)) => self.log.push(format!("{}: shell `{}` under epoch {epoch}", event.name, operand.as_str().unwrap_or_default())),
                    Err(refusal) => {
                        self.log.push(format!("{}: shell refused ({refusal:?})", event.name));
                        return outcome.failure(LifecycleError::ShellRefused { event: event.name.clone(), refusal });
                    }
                },
                "error" => {
                    let message = operand.as_str().unwrap_or_default();
                    return outcome.directive(Directive::Error(LifecycleError::Host(HostError::new("publisher.error", message))));
                }
                other => panic!("the publisher has no `{other}` action"),
            }
        }
        outcome
    }
}

fn render(template: &Value, state: &Map<String, Value>, event: &EventRequest) -> Result<Value, LifecycleError> {
    let document = EffectiveStateBuilder::new().with_frontmatter(state.clone()).build().expect("publisher state builds");
    let session = EvaluationSession::associate(event.globals.clone(), &document, event.runtime_bindings())
        .expect("a request's bindings match its view");
    let prepared = prepare_value(template, AuthoredMode::InterpolatedValue).expect("publisher templates parse");
    evaluate_prepared(&prepared, &session).map_err(|error| match error.unavailable_binding() {
        Some(read) => LifecycleError::Unavailable(Box::new(read.clone())),
        None => LifecycleError::Host(HostError::new("publisher.evaluation", &error.to_string())),
    })
}

fn events(run: &Run) -> Vec<String> {
    run.transcript_lines().into_iter().filter_map(|line| line.strip_prefix("event ").map(str::to_string)).collect()
}

#[test]
fn the_notification_fires_after_initialize_before_composition_and_its_set_reaches_later_events() {
    let mut publisher = Publisher::new(Outcome::Succeeded);
    let run = publisher.run(
        DocumentInput::new("release.md")
            .with("track", json!("stable"))
            .with("configure", json!([{ "set": { "channel": "{{ track }}-candidate" } }]))
            .with("ready", json!([{ "info": "publishing to {{ channel }}" }, { "shell": "git tag v1.2.0" }]))
            .with("published", json!([{ "info": "published to {{ channel }}" }])),
    );

    assert_eq!(
        run.transcript_lines(),
        [
            "event initialize",
            "event configure",
            "prepare initial",
            "shell unlocked",
            "event ready",
            "operation",
            "event published",
            "event finalize",
            "finish succeeded",
        ]
    );
    assert_eq!(
        publisher.log,
        [
            "ready: publishing to stable-candidate",
            "ready: shell `git tag v1.2.0` under epoch 7",
            "published: published to stable-candidate",
        ]
    );
    assert_eq!(run.state().get("channel"), Some(&json!("stable-candidate")));
    assert_eq!(run.terminal(), Some("published"));
}

#[test]
fn a_failed_completion_fires_the_failed_alternative_with_the_typed_failure_as_err() {
    let refusal = HostError::new("publisher.registry-refused", "version 1.2.0 already exists");
    let mut publisher = Publisher::new(Outcome::Failed(refusal.clone()));
    let run = publisher.run(
        DocumentInput::new("release.md")
            .with("rejected", json!([{ "info": "{{ err.code }}: {{ err.msg }}" }]))
            .with("finalize", json!([{ "info": "closed after {{ err.code }}" }])),
    );

    assert_eq!(events(&run), ["initialize", "configure", "ready", "rejected", "finalize"]);
    assert_eq!(
        publisher.log,
        [
            "rejected: publisher.registry-refused: version 1.2.0 already exists",
            "finalize: closed after publisher.registry-refused",
        ]
    );
    assert_eq!(run.selected_error(), Some(&LifecycleError::Host(refusal)));
    assert_eq!((run.terminal(), run.finish()), (Some("rejected"), Some(Finish::Failed)));
}

#[test]
fn an_explicit_error_in_published_downgrades_only_to_the_registered_target() {
    let mut publisher = Publisher::new(Outcome::Succeeded);
    let run = publisher.run(
        DocumentInput::new("release.md")
            .with("published", json!([{ "info": "announced" }, { "error": "changelog missing" }]))
            .with("rejected", json!([{ "info": "rejected: {{ err.msg }}" }])),
    );

    assert_eq!(events(&run), ["initialize", "configure", "ready", "published", "rejected", "finalize"]);
    // The communication `published` already made is not undone.
    assert_eq!(publisher.log, ["published: announced", "rejected: rejected: changelog missing"]);
    assert_eq!((run.terminal(), run.finish()), (Some("rejected"), Some(Finish::Failed)));

    // `rejected` registered no downgrade: its explicit error selects no
    // further terminal event, only the owed `finalize`.
    let mut publisher = Publisher::new(Outcome::Failed(HostError::new("publisher.registry-refused", "refused")));
    let run = publisher.run(DocumentInput::new("release.md").with("rejected", json!([{ "error": "still refused" }])));
    assert_eq!(events(&run), ["initialize", "configure", "ready", "rejected", "finalize"]);
    assert_eq!(run.terminal(), Some("rejected"));
}

#[test]
fn reading_an_unavailable_err_fails_with_the_views_reason_never_the_frontmatter_value() {
    let mut publisher = Publisher::new(Outcome::Succeeded);
    let run = publisher.run(
        DocumentInput::new("release.md")
            .with("err", json!("a frontmatter value named err"))
            .with("configure", json!([{ "info": "configuring after {{ err }}" }]))
            .with("rejected", json!([{ "info": "{{ err.code }}" }])),
    );

    let Some(LifecycleError::Unavailable(read)) = run.selected_error() else {
        panic!("expected an unavailable-binding error, got {:?}", run.selected_error());
    };
    assert_eq!((read.root.as_str(), read.reason.code(), read.scope.as_str()), ("err", CONFIGURE_HAS_NO_ERROR, "publisher.configure"));
    // The failure is caught like any setup failure, as cleanup only.
    assert!(run.transcript().contains(&TranscriptEntry::Event {
        name: "rejected".to_string(),
        mode: EvaluationMode::FatalEvaluationCleanup
    }));
    assert_eq!(events(&run), ["initialize", "configure", "rejected", "finalize"]);
    assert_eq!(publisher.log, [format!("rejected: {CONFIGURE_HAS_NO_ERROR}")]);
}

#[test]
fn initialize_refuses_a_shell_and_early_cleanup_stays_locked() {
    let mut publisher = Publisher::new(Outcome::Succeeded);
    let run = publisher.run(
        DocumentInput::new("release.md")
            .with("initialize", json!([{ "shell": "rm -rf build" }]))
            .with("finalize", json!([{ "shell": "git clean -fd" }])),
    );

    assert_eq!(events(&run), ["initialize", "rejected", "finalize"]);
    assert_eq!(
        run.selected_error(),
        Some(&LifecycleError::ShellRefused { event: "initialize".to_string(), refusal: ShellRefusal::InitializeForbidsShell })
    );
    assert_eq!(publisher.log, ["initialize: shell refused (InitializeForbidsShell)", "finalize: shell refused (Locked)"]);
    assert!(!run.transcript_lines().contains(&"shell unlocked".to_string()));

    // A before-composition notification may declare `shell`, but shells
    // unlock only on entering `ready`.
    let mut publisher = Publisher::new(Outcome::Succeeded);
    let run = publisher.run(DocumentInput::new("release.md").with("configure", json!([{ "shell": "make dist" }])));
    assert_eq!(publisher.log, ["configure: shell refused (Locked)"]);
    assert_eq!(events(&run), ["initialize", "configure", "rejected", "finalize"]);
}

#[test]
fn the_rename_is_honored_in_parsing_and_diagnostics() {
    let error = Run::start(&publisher_profile(), DocumentInput::new("release.md").with("composed", json!([]))).unwrap_err();
    assert_eq!(error, StartError::RenamedEvent { authored: "composed".to_string(), replacement: "ready".to_string() });

    // Another host's event names are data to the publisher.
    let run = Run::start(&publisher_profile(), DocumentInput::new("release.md").with("start", json!([{ "say": "hi" }]))).unwrap();
    assert_eq!(run.state().get("start"), Some(&json!([{ "say": "hi" }])));
}
