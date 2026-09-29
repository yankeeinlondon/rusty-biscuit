//! `claudine steer` behavior through its public seams.
//!
//! - Parser: command forms, conflicts, `--` messages, no consent bypass.
//! - Flow: a fake [`SteeringService`], scripted [`Interaction`] answers, and
//!   a captured [`Console`] drive validation, selection, consent,
//!   revalidation, receipts, and exit codes without a provider or a human.
//! - Rendering: the listing and the picker are rendered off-screen and
//!   inspected for labels, wrapping, and dim/strikethrough styling.
//! - With a daemon (`daemon-tests`): the real requester routes to an
//!   in-process owner, end to end.

use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use biscuit_tui::prelude::*;
use chrono::Utc;
use claudine::provider::Provider;
use claudine::steering::contract::{
    CancellationOutcome, InterruptionOutcome, MAX_MESSAGE_BYTES, SendOutcome, SteeringRequest, SteeringResult,
};
use claudine::steering::discovery::{
    DiscoveryError, DiscoveryFailed, DiscoveryGap, DiscoveryReport, ObservationSource, SessionListing,
};
use claudine::steering::eligibility::AvailabilitySummary;
use claudine::steering::identity::{ConversationGeneration, ExecutionId, ManagedTarget, ProcessStartIdentity, SteeringTargetId};
use claudine::steering::vocabulary::{DiscoveryMethod, ExecutionState, OperationIntent, SteeringAvailability};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;

use super::interact::{self, Interaction};
use super::render;
use super::service::{Delivery, SteeringService};
use super::*;

const SECRET_MESSAGE: &str = "  Recheck the failing test; token=ghp_0123456789abcdefghij0123 \n";
const REASON: &str = "steering is blocked for this launch profile: extensions can switch sessions unguarded";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn binding(n: u128, generation: u64) -> ManagedTarget {
    ManagedTarget {
        execution: ExecutionId::from_u128(n),
        wrapper: ProcessStartIdentity::new(100 + n as u32, "1700000000").unwrap(),
        generation: ConversationGeneration(generation),
        conversation: Some(format!("conversation-{generation}")),
    }
}

fn row(n: u128, state: ExecutionState, availability: SteeringAvailability, operation: Option<OperationIntent>) -> SessionListing {
    let reason = match availability {
        SteeringAvailability::NonInterrupting => None,
        SteeringAvailability::InterruptionRequired => Some("delivery requires interrupting the running turn first".into()),
        SteeringAvailability::Unavailable => Some(REASON.to_string()),
    };
    let target = binding(n, 1);
    SessionListing {
        id: target.id(),
        provider: Provider::Pi,
        name: Some(format!("session-{n}")),
        cwd: Some(format!("/work/project-{n}")),
        state,
        origins: vec![ObservationSource::Managed],
        launch_profile: Some("retained-rpc".into()),
        provider_version: Some("0.87.1".into()),
        availability: AvailabilitySummary { availability, operation, reason, setup_requirements: Vec::new() },
        observed_at: Utc::now(),
        session_key: None,
        binding: Some(target),
    }
}

fn steerable(n: u128) -> SessionListing {
    row(n, ExecutionState::Working, SteeringAvailability::NonInterrupting, Some(OperationIntent::SteerActiveTurn))
}

fn idle(n: u128) -> SessionListing {
    row(n, ExecutionState::Idle, SteeringAvailability::NonInterrupting, Some(OperationIntent::StartIdleTurn))
}

fn interrupting(n: u128) -> SessionListing {
    row(n, ExecutionState::Working, SteeringAvailability::InterruptionRequired, Some(OperationIntent::InterruptThenSubmit))
}

fn unavailable(n: u128) -> SessionListing {
    row(n, ExecutionState::Working, SteeringAvailability::Unavailable, None)
}

fn report(sessions: Vec<SessionListing>) -> DiscoveryReport {
    DiscoveryReport { sessions, errors: Vec::new(), gaps: Vec::new() }
}

/// What the fake provider establishes for each delivered request.
#[derive(Debug, Clone, Copy)]
enum Reply {
    Outcome(SendOutcome),
    Interrupted(InterruptionOutcome),
}

struct FakeService {
    /// Successive discovery answers; the last one repeats.
    discoveries: Mutex<VecDeque<Result<DiscoveryReport, DiscoveryFailed>>>,
    reply: Reply,
    discovered: AtomicUsize,
    delivered: Mutex<Vec<(SteeringRequest, Option<ManagedTarget>)>>,
}

impl FakeService {
    fn new(discoveries: Vec<Result<DiscoveryReport, DiscoveryFailed>>, reply: Reply) -> Self {
        Self {
            discoveries: Mutex::new(discoveries.into()),
            reply,
            discovered: AtomicUsize::new(0),
            delivered: Mutex::new(Vec::new()),
        }
    }

    fn listing(sessions: Vec<SessionListing>) -> Self {
        Self::new(vec![Ok(report(sessions))], Reply::Outcome(SendOutcome::Accepted))
    }

    fn replying(mut self, reply: Reply) -> Self {
        self.reply = reply;
        self
    }

    fn delivered(&self) -> Vec<(SteeringRequest, Option<ManagedTarget>)> {
        self.delivered.lock().unwrap().clone()
    }

    fn discovery_count(&self) -> usize {
        self.discovered.load(Ordering::SeqCst)
    }
}

impl SteeringService for FakeService {
    async fn discover(&self) -> Result<DiscoveryReport, DiscoveryFailed> {
        self.discovered.fetch_add(1, Ordering::SeqCst);
        let mut queue = self.discoveries.lock().unwrap();
        if queue.len() > 1 { queue.pop_front().unwrap() } else { queue.front().cloned().expect("a discovery answer") }
    }

    async fn deliver(&self, row: &SessionListing, request: &SteeringRequest) -> Delivery {
        self.delivered.lock().unwrap().push((request.clone(), row.binding.clone()));
        let result = match self.reply {
            Reply::Outcome(outcome) => SteeringResult::submitted(request, Some("rpc-steer"), outcome),
            Reply::Interrupted(phases) => SteeringResult::interrupted(request, "rpc-abort-submit", phases),
        };
        let detail = (!result.outcome.is_confirmed()).then(|| {
            claudine::secrets::Redactor::for_message(request.message.as_str()).redact(&format!("owner said: {}", request.message.as_str()))
        });
        Delivery { result, detail }
    }
}

/// Scripted human answers. An unscripted prompt fails the test: a flow that
/// must not ask never reaches it.
#[derive(Default)]
struct Scripted {
    /// Index into the rows offered, `None` to cancel.
    choices: VecDeque<io::Result<Option<usize>>>,
    consents: VecDeque<io::Result<bool>>,
    offered: Vec<Vec<SteeringTargetId>>,
    asked_consent_for: Vec<SteeringTargetId>,
}

impl Scripted {
    fn choosing(index: usize) -> Self {
        Self { choices: VecDeque::from([Ok(Some(index))]), ..Self::default() }
    }

    fn consenting(mut self, answers: impl IntoIterator<Item = io::Result<bool>>) -> Self {
        self.consents.extend(answers);
        self
    }
}

impl Interaction for Scripted {
    fn choose(&mut self, rows: &[SessionListing]) -> io::Result<Option<SteeringTargetId>> {
        self.offered.push(rows.iter().map(|row| row.id.clone()).collect());
        let choice = self.choices.pop_front().expect("the picker was not expected")?;
        Ok(choice.map(|index| rows[index].id.clone()))
    }

    fn confirm_interruption(&mut self, row: &SessionListing) -> io::Result<bool> {
        self.asked_consent_for.push(row.id.clone());
        self.consents.pop_front().expect("an interruption prompt was not expected")
    }
}

fn plain_terminal(width: u32) -> Terminal {
    let mut term = Terminal::new_optimistic(width);
    term.is_tty = false;
    term.color_depth = ColorDepth::None;
    term
}

struct Captured {
    out: String,
    err: String,
    interactive: bool,
}

impl Captured {
    fn interactive() -> Self {
        Self { out: String::new(), err: String::new(), interactive: true }
    }

    fn piped() -> Self {
        Self { interactive: false, ..Self::interactive() }
    }

    fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.out).unwrap_or_else(|error| panic!("stdout is one JSON document ({error}): {}", self.out))
    }

    fn all(&self) -> String {
        format!("{}\n{}", self.out, self.err)
    }
}

/// `text` with line wrapping and indentation collapsed, so a phrase can be
/// found wherever the terminal width broke it.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Console for Captured {
    fn data(&mut self, text: &str) {
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn status(&mut self, text: &str) {
        self.err.push_str(text);
        self.err.push('\n');
    }

    fn terminal(&self) -> Terminal {
        plain_terminal(100)
    }

    fn interactive(&self) -> bool {
        self.interactive
    }
}

fn send_args(message: &str, session: Option<&SteeringTargetId>, json: bool) -> SteerArgs {
    SteerArgs { message: Some(message.into()), list: false, session: session.map(ToString::to_string), json }
}

fn list_args(json: bool) -> SteerArgs {
    SteerArgs { message: None, list: true, session: None, json }
}

async fn run(args: SteerArgs, service: &FakeService, interaction: &mut Scripted, console: &mut Captured) -> i32 {
    execute(args, service, interaction, console).await
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(argv: &[&str]) -> Result<SteerArgs, clap::Error> {
    use clap::Parser;
    let cli = crate::args::Cli::try_parse_from(std::iter::once("claudine").chain(argv.iter().copied()))?;
    match cli.command {
        Some(crate::args::Commands::Steer(args)) => Ok(args),
        _ => panic!("`{argv:?}` did not parse as steer"),
    }
}

#[test]
fn the_specified_command_forms_parse() {
    let send = parse(&["steer", "Recheck the failing test."]).unwrap();
    assert_eq!((send.message.as_deref(), send.list, send.session, send.json), (Some("Recheck the failing test."), false, None, false));
    let list = parse(&["steer", "--list"]).unwrap();
    assert!(list.list && list.message.is_none());
    assert!(parse(&["steer", "--list", "--json"]).unwrap().json);
    let explicit = parse(&["steer", "--session", "managed:00000000-0000-0000-0000-000000000001", "--json", "go"]).unwrap();
    assert_eq!(explicit.session.as_deref(), Some("managed:00000000-0000-0000-0000-000000000001"));
    assert!(explicit.json);
}

#[test]
fn a_message_after_double_dash_keeps_its_exact_bytes() {
    for message in ["--help", "-v", "--provider cl", "  padded\ttext  ", "--list"] {
        let args = parse(&["steer", "--", message]).unwrap();
        assert_eq!(args.message.as_deref(), Some(message));
        assert!(!args.list);
    }
    // Argv normalization runs before clap; it must leave steer messages alone.
    let argv: Vec<std::ffi::OsString> =
        ["claudine", "steer", "--", "--provider", "cl"].iter().map(std::ffi::OsString::from).collect();
    assert_eq!(crate::argv::normalize_with_completion(argv.clone(), false), argv);
    let argv: Vec<std::ffi::OsString> =
        ["claudine", "steer", "use --provider cl, then --claude"].iter().map(std::ffi::OsString::from).collect();
    assert_eq!(crate::argv::normalize_with_completion(argv.clone(), false), argv);
}

#[test]
fn invalid_forms_are_usage_errors_and_no_consent_bypass_exists() {
    for argv in [
        vec!["steer"],
        vec!["steer", "--list", "message"],
        vec!["steer", "--list", "--session", "managed:00000000-0000-0000-0000-000000000001"],
        vec!["steer", "--yes", "message"],
        vec!["steer", "--interrupt", "message"],
        vec!["steer", "--force", "message"],
        vec!["steer", "one", "two"],
    ] {
        let error = parse(&argv).map(|_| ()).unwrap_err();
        assert_eq!(error.exit_code(), EXIT_USAGE, "{argv:?}");
    }
}

// ---------------------------------------------------------------------------
// Validation before dispatch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invalid_messages_and_ids_fail_before_discovery() {
    let cases: Vec<(String, Option<String>, &str)> = vec![
        ("   \n\t".into(), None, "empty"),
        ("a\0b".into(), None, "NUL"),
        ("x".repeat(MAX_MESSAGE_BYTES + 1), None, "limit"),
        ("é".repeat(MAX_MESSAGE_BYTES / 2 + 1), None, "limit"),
        ("go".into(), Some("managed:00000000".into()), "not a session ID"),
        ("go".into(), Some("MANAGED:00000000-0000-0000-0000-000000000001".into()), "not a session ID"),
        ("go".into(), Some("session-1".into()), "not a session ID"),
    ];
    for (message, session, expected) in cases {
        let service = FakeService::listing(vec![steerable(1)]);
        let mut console = Captured::interactive();
        let args = SteerArgs { message: Some(message), list: false, session, json: false };
        let code = run(args, &service, &mut Scripted::default(), &mut console).await;
        assert_eq!(code, EXIT_USAGE, "{expected}");
        assert!(flat(&console.err).contains(expected), "{expected}: {}", console.err);
        assert_eq!(service.discovery_count(), 0, "{expected}: nothing is discovered for invalid input");
        assert!(service.delivered().is_empty());
    }
}

#[tokio::test]
async fn a_message_at_the_limit_is_delivered_unchanged() {
    let message = "x".repeat(MAX_MESSAGE_BYTES);
    let service = FakeService::listing(vec![steerable(1)]);
    let id = steerable(1).id;
    let code = run(send_args(&message, Some(&id), false), &service, &mut Scripted::default(), &mut Captured::piped()).await;
    assert_eq!(code, EXIT_CONFIRMED);
    assert_eq!(service.delivered()[0].0.message.as_str(), message);
}

#[tokio::test]
async fn without_an_explicit_target_json_and_pipes_never_prompt() {
    let service = FakeService::listing(vec![steerable(1)]);
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, true), &service, &mut Scripted::default(), &mut console).await, EXIT_USAGE);
    assert!(flat(&console.err).contains("--session"), "{}", console.err);
    assert!(console.out.is_empty(), "no JSON document for a usage error");

    let mut console = Captured::piped();
    assert_eq!(run(send_args("go", None, false), &service, &mut Scripted::default(), &mut console).await, EXIT_USAGE);
    assert!(flat(&console.err).contains("claudine steer --list"), "the error says how to list and choose: {}", console.err);
    assert_eq!(service.discovery_count(), 0);
    assert!(service.delivered().is_empty(), "no target is ever inferred");
}

// ---------------------------------------------------------------------------
// Listing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_json_is_the_versioned_contract_with_unavailable_rows() {
    let secret = "daemon echoed token=ghp_0123456789abcdefghij0123";
    let mut listing = report(vec![steerable(1), unavailable(2), idle(3)]);
    listing.errors.push(DiscoveryError::new("codex", Box::new(io::Error::other(secret))));
    listing.gaps.push(DiscoveryGap { provider: Provider::Codex, discovery_id: "app-server-list", method: DiscoveryMethod::ProviderApi });
    let service = FakeService::new(vec![Ok(listing)], Reply::Outcome(SendOutcome::Accepted));
    let mut console = Captured::piped();
    let code = run(list_args(true), &service, &mut Scripted::default(), &mut console).await;
    assert_eq!(code, EXIT_CONFIRMED, "partial discovery still lists");

    let doc = console.json();
    let mut keys: Vec<&str> = doc.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["coverage_gaps", "discovery_errors", "observed_at", "schema_version", "sessions"]);
    assert_eq!(doc["schema_version"], 1);
    let sessions = doc["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 3, "unavailable rows are listed, not hidden");
    let row = &sessions[1];
    for key in [
        "id", "provider", "name", "cwd", "state", "origins", "launch_profile", "provider_version", "availability", "operation",
        "reason", "setup_requirements", "observed_at",
    ] {
        assert!(row.get(key).is_some(), "session field `{key}`");
    }
    assert_eq!(row["id"], "managed:00000000-0000-0000-0000-000000000002", "the exact ID `--session` accepts");
    assert_eq!(row["availability"], "unavailable");
    assert_eq!(row["reason"], REASON);
    assert!(row["operation"].is_null());
    assert_eq!(sessions[0]["operation"], "steer_active_turn");
    assert_eq!(sessions[2]["operation"], "start_idle_turn");
    assert!(sessions[0]["reason"].is_null(), "unknown values are explicit");
    assert_eq!(doc["discovery_errors"][0]["source"], "codex");
    assert!(!console.all().contains("ghp_0123456789abcdefghij0123"), "discovery errors are masked everywhere");
    assert_eq!(doc["coverage_gaps"][0]["provider"], "codex");
    assert!(flat(&console.err).contains("discovery from codex failed"), "partial failure is a warning on stderr");
    assert!(service.delivered().is_empty(), "listing sends nothing");
}

#[tokio::test]
async fn an_empty_listing_succeeds_and_total_failure_fails() {
    let service = FakeService::listing(Vec::new());
    let mut console = Captured::piped();
    assert_eq!(run(list_args(true), &service, &mut Scripted::default(), &mut console).await, EXIT_CONFIRMED);
    assert_eq!(console.json()["sessions"], serde_json::json!([]));

    let mut console = Captured::piped();
    assert_eq!(run(list_args(false), &service, &mut Scripted::default(), &mut console).await, EXIT_CONFIRMED);
    assert!(flat(&console.out).contains("No active sessions were found."));

    let failed = DiscoveryFailed { errors: vec![DiscoveryError::new("managed", Box::new(io::Error::other("unreachable")))] };
    let service = FakeService::new(vec![Err(failed)], Reply::Outcome(SendOutcome::Accepted));
    let mut console = Captured::piped();
    assert_eq!(run(list_args(true), &service, &mut Scripted::default(), &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(console.out.is_empty(), "no partial document on total failure");
    assert!(flat(&console.err).contains("session discovery failed for every source") && flat(&console.err).contains("unreachable"));
}

// ---------------------------------------------------------------------------
// Explicit sends
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_explicit_send_reaches_exactly_its_target_with_the_listed_operation() {
    let target = steerable(2);
    let service = FakeService::listing(vec![steerable(1), target.clone(), steerable(3)]);
    let mut console = Captured::piped();
    let code = run(send_args(SECRET_MESSAGE, Some(&target.id), true), &service, &mut Scripted::default(), &mut console).await;
    assert_eq!(code, EXIT_CONFIRMED);

    let delivered = service.delivered();
    assert_eq!(delivered.len(), 1);
    let (request, bound) = &delivered[0];
    assert_eq!(request.target, target.id);
    assert_eq!(bound.as_ref(), target.binding.as_ref(), "routed under the binding it was listed with");
    assert_eq!(request.operation, OperationIntent::SteerActiveTurn);
    assert_eq!(request.origin, claudine::steering::contract::SteeringOrigin::Manual);
    assert_eq!(request.message.as_str(), SECRET_MESSAGE, "original bytes, untrimmed");
    assert_eq!(request.consent, None);
    assert_eq!(service.discovery_count(), 1, "no human pause, so the owner's own check suffices");

    let doc = console.json();
    let mut keys: Vec<&str> = doc.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["detail", "interruption", "mechanism", "operation", "outcome", "receipt", "request_id", "schema_version", "target"]
    );
    assert_eq!(doc["target"], target.id.to_string());
    assert_eq!(doc["request_id"], request.id.to_string());
    assert_eq!((doc["outcome"].as_str(), doc["receipt"].as_str()), (Some("accepted"), Some("accepted")));
    assert_eq!(doc["mechanism"], "rpc-steer");
    assert!(doc["interruption"].is_null() && doc["detail"].is_null());
    assert!(!console.all().contains("ghp_0123456789abcdefghij0123"), "the message is never echoed");
}

/// Exit 0 exactly for provider confirmation; every other outcome is 1 and
/// keeps its own name.
#[tokio::test]
async fn exit_codes_follow_the_established_outcome() {
    let cases = [
        (SendOutcome::Accepted, EXIT_CONFIRMED, "accepted"),
        (SendOutcome::Queued, EXIT_CONFIRMED, "queued"),
        (SendOutcome::Delivered, EXIT_CONFIRMED, "delivered"),
        (SendOutcome::Refused, EXIT_NOT_CONFIRMED, "refused"),
        (SendOutcome::Held, EXIT_NOT_CONFIRMED, "held"),
        (SendOutcome::Unavailable, EXIT_NOT_CONFIRMED, "unavailable"),
        (SendOutcome::Busy, EXIT_NOT_CONFIRMED, "busy"),
        (SendOutcome::PartialInterruption, EXIT_NOT_CONFIRMED, "partial_interruption"),
        (SendOutcome::Unknown, EXIT_NOT_CONFIRMED, "unknown"),
    ];
    for (outcome, exit, name) in cases {
        let target = steerable(1);
        let service = FakeService::listing(vec![target.clone()]).replying(Reply::Outcome(outcome));
        let mut console = Captured::piped();
        assert_eq!(run(send_args(SECRET_MESSAGE, Some(&target.id), true), &service, &mut Scripted::default(), &mut console).await, exit, "{name}");
        let doc = console.json();
        assert_eq!(doc["outcome"], name);
        let receipt = if outcome.is_confirmed() { name } else { "unknown" };
        assert_eq!(doc["receipt"], receipt, "{name}: no receipt stronger than the outcome");
        assert_eq!(service.delivered().len(), 1, "{name}: sent once, never retried");
        if !outcome.is_confirmed() {
            let detail = doc["detail"].as_str().unwrap();
            assert!(!detail.contains("ghp_0123456789abcdefghij0123"), "{name}: an echoed detail is redacted: {detail}");
            assert!(detail.contains("****"), "{name}: {detail}");
        }
    }
}

#[tokio::test]
async fn receipts_say_only_what_was_established() {
    let text = |outcome| {
        let target = steerable(1);
        async move {
            let service = FakeService::listing(vec![target.clone()]).replying(Reply::Outcome(outcome));
            let mut console = Captured::piped();
            run(send_args("go", Some(&target.id), false), &service, &mut Scripted::default(), &mut console).await;
            flat(&console.out)
        }
    };
    let accepted = text(SendOutcome::Accepted).await;
    assert!(accepted.contains("accepted") && accepted.contains("does not mean the agent acted on it"), "{accepted}");
    let queued = text(SendOutcome::Queued).await;
    assert!(queued.contains("not in the conversation yet"), "{queued}");
    let held = text(SendOutcome::Held).await;
    assert!(held.contains("held (undelivered)") && held.contains("has not been delivered"), "{held}");
    assert!(held.contains("separate setup") && held.contains("never changes provider configuration"), "{held}");
    let unknown = text(SendOutcome::Unknown).await;
    assert!(unknown.contains("not retried") && unknown.contains("duplicate"), "{unknown}");
}

#[tokio::test]
async fn an_unlisted_or_unavailable_explicit_target_sends_nothing() {
    let stale: SteeringTargetId = "managed:00000000-0000-0000-0000-0000000000ff".parse().unwrap();
    let service = FakeService::listing(vec![steerable(1), unavailable(2)]);
    let mut console = Captured::piped();
    assert_eq!(run(send_args("go", Some(&stale), true), &service, &mut Scripted::default(), &mut console).await, EXIT_NOT_CONFIRMED);
    let doc = console.json();
    assert_eq!((doc["outcome"].as_str(), doc["target"].as_str()), (Some("unavailable"), Some(stale.to_string().as_str())));
    assert!(doc["detail"].as_str().unwrap().contains("not redirected"));

    let blocked = unavailable(2);
    let mut console = Captured::piped();
    assert_eq!(run(send_args("go", Some(&blocked.id), true), &service, &mut Scripted::default(), &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(console.json()["detail"].as_str().unwrap().contains(REASON), "the reason is reported");
    assert!(service.delivered().is_empty());
}

// ---------------------------------------------------------------------------
// Interactive selection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn one_eligible_session_still_needs_a_choice_and_unavailable_rows_are_offered_disabled() {
    let rows = vec![unavailable(1), steerable(2)];
    let service = FakeService::listing(rows.clone());
    let mut interaction = Scripted::choosing(1);
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut interaction, &mut console).await, EXIT_CONFIRMED);
    assert_eq!(interaction.offered, [vec![rows[0].id.clone(), rows[1].id.clone()]], "every row is shown");
    assert_eq!(service.delivered()[0].0.target, rows[1].id);
    assert!(flat(&console.err).contains(REASON), "the listing with reasons is shown before choosing");
    assert_eq!(service.discovery_count(), 2, "a picked session is looked up again after the human pause");
}

#[tokio::test]
async fn empty_states_and_cancellation_never_broadcast() {
    let service = FakeService::listing(Vec::new());
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut Scripted::default(), &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(flat(&console.err).contains("No active sessions were found.") && flat(&console.err).contains("Nothing was sent."));

    let unknown_state = row(3, ExecutionState::Unknown, SteeringAvailability::Unavailable, None);
    let service = FakeService::listing(vec![unavailable(1), unknown_state]);
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut Scripted::default(), &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(flat(&console.err).contains("None of these sessions can be steered"), "{}", console.err);
    assert!(flat(&console.err).contains("unknown"), "an unknown state is shown as unknown");

    let service = FakeService::listing(vec![steerable(1), steerable(2)]);
    let mut interaction = Scripted { choices: VecDeque::from([Ok(None)]), ..Scripted::default() };
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut interaction, &mut console).await, EXIT_CANCELLED);
    assert!(flat(&console.err).contains("Cancelled. Nothing was sent."));
    assert!(service.delivered().is_empty());
}

#[tokio::test]
async fn an_idle_target_is_told_it_starts_a_turn() {
    let target = idle(1);
    let service = FakeService::listing(vec![target.clone()]);
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut Scripted::choosing(0), &mut console).await, EXIT_CONFIRMED);
    assert_eq!(service.delivered()[0].0.operation, OperationIntent::StartIdleTurn);
    assert!(flat(&console.err).contains("starts a new turn"), "{}", console.err);
}

#[tokio::test]
async fn a_stale_selection_is_reported_and_never_redirected() {
    let chosen = steerable(1);
    let mut replaced = chosen.clone();
    replaced.binding = Some(binding(1, 2));
    let service = FakeService::new(
        vec![Ok(report(vec![chosen.clone(), steerable(2)])), Ok(report(vec![replaced, steerable(2)]))],
        Reply::Outcome(SendOutcome::Accepted),
    );
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut Scripted::choosing(0), &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(service.delivered().is_empty(), "neither the replaced conversation nor another session receives it");
    assert!(flat(&console.out).contains("not redirected"), "{}", console.out);

    // An ended session is the same outcome.
    let service = FakeService::new(
        vec![Ok(report(vec![chosen.clone()])), Ok(report(Vec::new()))],
        Reply::Outcome(SendOutcome::Accepted),
    );
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut Scripted::choosing(0), &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(service.delivered().is_empty());
}

#[tokio::test]
async fn a_changed_action_after_choosing_is_never_sent_silently() {
    // Steering became an idle start: a different action, so nothing is sent.
    let chosen = steerable(1);
    let mut now_idle = idle(1);
    now_idle.binding = chosen.binding.clone();
    let service = FakeService::new(vec![Ok(report(vec![chosen.clone()])), Ok(report(vec![now_idle]))], Reply::Outcome(SendOutcome::Accepted));
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut Scripted::choosing(0), &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(flat(&console.out).contains("changed since it was chosen"), "{}", console.out);
    assert!(service.delivered().is_empty());

    // Non-interrupting delivery was lost: the same explanation and choice
    // are required, and declining sends nothing.
    let mut now_interrupting = interrupting(1);
    now_interrupting.binding = chosen.binding.clone();
    let service = FakeService::new(
        vec![Ok(report(vec![chosen.clone()])), Ok(report(vec![now_interrupting.clone()]))],
        Reply::Outcome(SendOutcome::Accepted),
    );
    let mut interaction = Scripted::choosing(0).consenting([Ok(false)]);
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut interaction, &mut console).await, EXIT_CANCELLED);
    assert_eq!(interaction.asked_consent_for, std::slice::from_ref(&chosen.id));
    assert!(flat(&console.err).contains("can no longer be steered without interrupting"), "{}", console.err);
    assert!(service.delivered().is_empty());

    // Accepting sends the interruption, with consent bound to that action.
    let service = FakeService::new(
        vec![Ok(report(vec![chosen.clone()])), Ok(report(vec![now_interrupting]))],
        Reply::Outcome(SendOutcome::Queued),
    );
    let mut interaction = Scripted::choosing(0).consenting([Ok(true)]);
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut interaction, &mut console).await, EXIT_CONFIRMED);
    let request = &service.delivered()[0].0;
    assert_eq!(request.operation, OperationIntent::InterruptThenSubmit);
    assert!(request.may_interrupt());
    assert_eq!(service.discovery_count(), 3, "revalidated again after the consent pause");
}

#[tokio::test]
async fn a_picker_failure_sends_nothing() {
    let service = FakeService::listing(vec![steerable(1)]);
    let mut interaction = Scripted { choices: VecDeque::from([Err(io::Error::other("no tty"))]), ..Scripted::default() };
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", None, false), &service, &mut interaction, &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(flat(&console.err).contains("the session picker failed"));
    assert!(service.delivered().is_empty());
}

// ---------------------------------------------------------------------------
// Interruption consent
// ---------------------------------------------------------------------------

#[tokio::test]
async fn interruption_consent_is_explained_bound_and_revalidated() {
    let target = interrupting(1);
    let service = FakeService::listing(vec![target.clone()]).replying(Reply::Interrupted(InterruptionOutcome {
        cancellation: CancellationOutcome::Established,
        replacement: Some(SendOutcome::Unknown),
    }));
    let mut interaction = Scripted::default().consenting([Ok(true)]);
    let mut console = Captured::interactive();
    let code = run(send_args("go", Some(&target.id), false), &service, &mut interaction, &mut console).await;
    assert_eq!(code, EXIT_NOT_CONFIRMED, "a partial interruption is not a confirmation");

    assert_eq!(interaction.asked_consent_for, std::slice::from_ref(&target.id), "an explicit ID does not grant consent");
    let explanation = flat(&console.err);
    for effect in ["running turn is stopped first", "tool the agent is running", "kept, not cleared"] {
        assert!(explanation.contains(effect), "the explanation covers `{effect}`: {explanation}");
    }
    let request = &service.delivered()[0].0;
    assert!(request.may_interrupt());
    assert_eq!(request.consent.as_ref().map(|consent| (&consent.target, consent.operation)), Some((&target.id, OperationIntent::InterruptThenSubmit)));
    assert_eq!(service.discovery_count(), 2, "looked up again after consent, before cancellation");
    let receipt = flat(&console.out);
    assert!(receipt.contains("partial interruption"), "{receipt}");
    assert!(receipt.contains("the running turn was stopped; the replacement message was unknown"), "{receipt}");
    assert!(receipt.contains("not restarted"), "{receipt}");
}

#[tokio::test]
async fn interruption_phases_stay_separate_in_json() {
    let target = interrupting(1);
    let service = FakeService::listing(vec![target.clone()]).replying(Reply::Interrupted(InterruptionOutcome {
        cancellation: CancellationOutcome::Established,
        replacement: Some(SendOutcome::Accepted),
    }));
    // JSON never prompts, so the interruption cannot be consented and is
    // refused without performing either operation.
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", Some(&target.id), true), &service, &mut Scripted::default(), &mut console).await, EXIT_NOT_CONFIRMED);
    let doc = console.json();
    assert_eq!(doc["outcome"], "unavailable");
    assert_eq!(doc["operation"], "interrupt_then_submit");
    assert!(doc["interruption"].is_null(), "neither phase was attempted");
    assert!(doc["detail"].as_str().unwrap().contains("`--json` never asks for consent"));
    assert!(service.delivered().is_empty());
}

#[tokio::test]
async fn consent_is_never_inferred() {
    let target = interrupting(1);
    let service = FakeService::listing(vec![target.clone()]);

    // No terminal: refused, nothing asked or sent.
    let mut console = Captured::piped();
    assert_eq!(run(send_args("go", Some(&target.id), false), &service, &mut Scripted::default(), &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(flat(&console.out).contains("no terminal is available to ask for consent"), "{}", console.out);

    // Declined: user cancellation.
    let mut interaction = Scripted::default().consenting([Ok(false)]);
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", Some(&target.id), false), &service, &mut interaction, &mut console).await, EXIT_CANCELLED);
    assert!(flat(&console.err).contains("Not interrupted. Nothing was sent."));

    // A failed prompt is not consent.
    let mut interaction = Scripted::default().consenting([Err(io::Error::other("tty lost"))]);
    let mut console = Captured::interactive();
    assert_eq!(run(send_args("go", Some(&target.id), false), &service, &mut interaction, &mut console).await, EXIT_NOT_CONFIRMED);
    assert!(flat(&console.out).contains("consent was not given"), "{}", console.out);

    assert!(service.delivered().is_empty());
}

// ---------------------------------------------------------------------------
// Rendering (off-screen; nothing gains focus)
// ---------------------------------------------------------------------------

fn styled_terminal(width: u32) -> Terminal {
    Terminal::new_optimistic(width)
}

#[test]
fn the_listing_labels_every_row_and_styles_unavailable_ones() {
    let listing = report(vec![steerable(1), unavailable(2), interrupting(3)]);
    let table = render::SessionTable::new(&listing);

    let plain = table.render(&plain_terminal(140));
    assert!(!plain.contains('\x1b'), "plain output has no escapes");
    for header in ["Provider", "Session", "Directory", "State", "Steering"] {
        assert!(plain.contains(header), "{header}");
    }
    for label in ["non-interrupting", "unavailable", "interruption required"] {
        assert!(plain.contains(label), "plain output keeps the `{label}` label");
    }
    for n in 1..=3 {
        assert!(plain.contains(&binding(n, 1).id().to_string()), "full ID for row {n}");
    }
    assert!(plain.contains(REASON), "the reason is spelled out");
    assert!(plain.contains("adds the message to the running turn"));

    let styled = table.render(&styled_terminal(140));
    let line_of = |needle: &str| styled.lines().find(|line| line.contains(needle)).unwrap_or_else(|| panic!("{needle}")).to_string();
    let unavailable_row = line_of("session-2");
    assert!(unavailable_row.contains("\x1b[9m") || unavailable_row.contains(";9m") || unavailable_row.contains("[9;"), "struck through: {unavailable_row:?}");
    assert!(unavailable_row.contains("\x1b[2m") || unavailable_row.contains(";2m") || unavailable_row.contains("[2;"), "dim: {unavailable_row:?}");
    let available_row = line_of("session-1");
    assert!(!available_row.contains("\x1b[9m"), "selectable rows are not struck: {available_row:?}");
}

#[test]
fn a_narrow_terminal_wraps_details_instead_of_hiding_them() {
    let mut blocked = unavailable(2);
    blocked.availability.setup_requirements = vec!["start the provider with its retained control channel enabled".into()];
    let listing = report(vec![steerable(1), blocked]);
    let narrow = render::SessionTable::new(&listing).render(&plain_terminal(44));
    assert!(!narrow.contains("could not be rendered"), "a too-narrow table falls back to stacked rows:\n{narrow}");
    let text = flat(&narrow);
    assert!(text.contains(REASON), "the reason survives wrapping:\n{narrow}");
    assert!(text.contains("start the provider with its retained control channel enabled"), "{narrow}");
    assert!(text.contains("cannot change this open session"), "setup is for future launches:\n{narrow}");
    assert!(narrow.contains(&binding(2, 1).id().to_string()), "the full ID is never truncated:\n{narrow}");
    for label in ["pi", "session-1", "/work/project-1", "working", "non-interrupting", "unavailable"] {
        assert!(text.contains(label), "stacked rows keep `{label}`:\n{narrow}");
    }
    assert!(narrow.lines().all(|line| line.chars().count() <= 44 || line.contains("managed:")), "only an unbreakable ID may exceed the width:\n{narrow}");

    let styled = render::SessionTable::new(&listing).render(&styled_terminal(44));
    let summary = styled.lines().find(|line| line.contains("session-2")).expect("stacked summary");
    assert!(summary.contains("\x1b[9m") || summary.contains(";9m") || summary.contains("[9;"), "still struck through: {summary:?}");
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Simulated picker input: disabled rows are skipped by navigation and
/// Enter, and Esc cancels.
#[test]
fn the_picker_only_selects_available_rows() {
    let rows = vec![unavailable(1), steerable(2), unavailable(3), idle(4)];
    let chooser = ChooseOne::<String>::new();

    let mut state = interact::picker_state(&rows);
    assert!(state.options()[0].disabled && state.options()[2].disabled);
    assert!(!state.options()[1].disabled && !state.options()[3].disabled);
    assert!(state.options()[0].label.contains("unavailable"), "a disabled row still says why in words");
    assert_eq!(chooser.handle_event(&mut state, press(KeyCode::Enter)), EventOutcome::Submitted);
    assert_eq!(state.value().as_deref(), Some(rows[1].id.to_string().as_str()), "the first available row, never a disabled one");

    let mut state = interact::picker_state(&rows);
    chooser.handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(chooser.handle_event(&mut state, press(KeyCode::Enter)), EventOutcome::Submitted);
    assert_eq!(state.value().as_deref(), Some(rows[3].id.to_string().as_str()), "navigation skips the disabled row");

    let mut state = interact::picker_state(&rows);
    chooser.handle_event(&mut state, press(KeyCode::Up));
    chooser.handle_event(&mut state, press(KeyCode::Up));
    chooser.handle_event(&mut state, press(KeyCode::Esc));
    assert_eq!(state.value(), None, "Esc selects nothing, which the command treats as cancellation");
}

#[test]
fn the_picker_draws_unavailable_rows_dim_and_struck() {
    let theme = interact::picker_theme();
    assert!(theme.disabled_style.add_modifier.contains(Modifier::DIM | Modifier::CROSSED_OUT));

    let rows = vec![steerable(1), unavailable(2)];
    let mut state = interact::picker_state(&rows);
    let backend = ratatui::backend::TestBackend::new(120, 6);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| frame.render_stateful_widget(ChooseOne::<String>::new(), frame.area(), &mut state))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let row_with = |needle: &str| {
        (0..buffer.area.height)
            .find(|y| (0..buffer.area.width).map(|x| buffer[(x, *y)].symbol().to_string()).collect::<String>().contains(needle))
            .unwrap_or_else(|| panic!("`{needle}` is drawn"))
    };
    let modifier_at = |y: u16, needle: &str| {
        let line: String = (0..buffer.area.width).map(|x| buffer[(x, y)].symbol().to_string()).collect();
        let x = u16::try_from(line.find(needle).unwrap()).unwrap();
        buffer[(x, y)].modifier
    };
    let disabled = row_with("session-2");
    assert!(modifier_at(disabled, "session-2").contains(Modifier::CROSSED_OUT | Modifier::DIM));
    let enabled = row_with("session-1");
    assert!(!modifier_at(enabled, "session-1").contains(Modifier::CROSSED_OUT));
}

// ---------------------------------------------------------------------------
// With a daemon: the real requester and an in-process owner
// ---------------------------------------------------------------------------

#[cfg(feature = "daemon-tests")]
mod with_daemon {
    use std::sync::Arc;

    use claudine::steering::audit::{DeliveryReport, SteeringAuditLog};
    use claudine::steering::controller::{ControllerConfig, DeliveryDeadlines, DeliveryFuture, EligibilityFn, ExecutionFacts, SteeringExecutor};
    use claudine::steering::eligibility::{AutomaticEligibility, Eligibility, ManualEligibility, Route};
    use claudine::steering::vocabulary::{AdapterRef, CaseSupport, LaunchMode, SteeringMechanism};
    use rendezvous_core::local_endpoint::test_support::private_endpoint;

    use super::*;
    use crate::commands::steer::service::LocalService;
    use crate::steering::owner::ExecutionSteering;
    use crate::steering::tests::point_endpoint_at;
    use crate::steering::tests::with_daemon::{boot, listed};

    fn mechanism(id: &str) -> &'static SteeringMechanism {
        claudine::steering::facts(Provider::Pi).mechanisms.iter().find(|m| m.id == id).expect("researched mechanism")
    }

    /// Eligibility offering one researched Pi route, standing in for a
    /// reviewed grant (none exists while Pi is blocked).
    fn offering(id: &'static str, support: CaseSupport) -> EligibilityFn {
        Arc::new(move |_| {
            let route = Route { mechanism: mechanism(id), adapter: AdapterRef { id: "fixture", revision: 1 }, support };
            let availability = match support {
                CaseSupport::InterruptionRequired => SteeringAvailability::InterruptionRequired,
                _ => SteeringAvailability::NonInterrupting,
            };
            Eligibility {
                manual: ManualEligibility { availability, route: Some(route), blockers: Vec::new() },
                automatic: AutomaticEligibility::Eligible(route),
            }
        })
    }

    struct Fixture {
        delivered: Arc<Mutex<Vec<SteeringRequest>>>,
        reply: Reply,
    }

    impl SteeringExecutor for Fixture {
        fn deliver(&self, request: SteeringRequest, route: Route, _deadlines: DeliveryDeadlines) -> DeliveryFuture {
            self.delivered.lock().unwrap().push(request.clone());
            let reply = self.reply;
            Box::pin(async move {
                DeliveryReport::new(match reply {
                    Reply::Outcome(outcome) => SteeringResult::submitted(&request, Some(route.mechanism.id), outcome),
                    Reply::Interrupted(phases) => SteeringResult::interrupted(&request, route.mechanism.id, phases),
                })
            })
        }
    }

    fn owner(tmp: &tempfile::TempDir, eligibility: EligibilityFn, reply: Reply) -> (ExecutionSteering, Arc<Mutex<Vec<SteeringRequest>>>) {
        let delivered = Arc::new(Mutex::new(Vec::new()));
        let config = ControllerConfig {
            execution: ExecutionId::random(),
            wrapper: ProcessStartIdentity::new(std::process::id(), "fixture-start").unwrap(),
            facts: ExecutionFacts {
                provider: Provider::Pi,
                profile_id: Some("retained-rpc".into()),
                os: claudine::steering::host_os(),
                launch_mode: LaunchMode::NonInteractive,
                provider_version: Some("0.87.1".into()),
                cwd: Some("/work/project".into()),
                name: Some("fixture".into()),
            },
            initial_state: ExecutionState::Working,
            eligibility,
            audit: SteeringAuditLog::at(tmp.path()),
        };
        let executor = Arc::new(Fixture { delivered: Arc::clone(&delivered), reply });
        (ExecutionSteering::start_with(config, executor), delivered)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn list_then_send_by_id_reaches_the_owner_and_returns_its_receipt() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        let (steering, delivered) = owner(&tmp, offering("rpc-steer", CaseSupport::NonInterrupting), Reply::Outcome(SendOutcome::Queued));
        let (_other, other_delivered) = owner(&tmp, offering("rpc-steer", CaseSupport::NonInterrupting), Reply::Outcome(SendOutcome::Queued));
        let own_id = steering.controller().snapshot().target.id();
        listed(|row| row.id == own_id).await;

        let mut console = Captured::piped();
        assert_eq!(execute(list_args(true), &LocalService, &mut Scripted::default(), &mut console).await, EXIT_CONFIRMED);
        let doc = console.json();
        let row = doc["sessions"].as_array().unwrap().iter().find(|row| row["id"] == own_id.to_string()).expect("listed");
        assert_eq!((row["availability"].as_str(), row["operation"].as_str()), (Some("non_interrupting"), Some("steer_active_turn")));

        let id: SteeringTargetId = row["id"].as_str().unwrap().parse().unwrap();
        let mut console = Captured::piped();
        let code = execute(send_args(SECRET_MESSAGE, Some(&id), true), &LocalService, &mut Scripted::default(), &mut console).await;
        assert_eq!(code, EXIT_CONFIRMED);
        let doc = console.json();
        assert_eq!((doc["outcome"].as_str(), doc["receipt"].as_str(), doc["mechanism"].as_str()), (Some("queued"), Some("queued"), Some("rpc-steer")));
        let requests = delivered.lock().unwrap().clone();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].message.as_str(), SECRET_MESSAGE, "original bytes reach the owner");
        assert!(other_delivered.lock().unwrap().is_empty(), "no other session receives it");
        daemon.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_conversation_switch_after_listing_is_refused_by_the_owner() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        let (steering, delivered) = owner(&tmp, offering("rpc-steer", CaseSupport::NonInterrupting), Reply::Outcome(SendOutcome::Accepted));
        let own_id = steering.controller().snapshot().target.id();
        let listed_row = listed(|row| row.id == own_id).await;

        // The switch lands between listing and routing: the row's binding
        // is the old conversation, and the owner refuses it.
        steering.controller().set_conversation(Some("replacement".into()));
        let service = StaleRow(listed_row);
        let mut console = Captured::piped();
        let code = execute(send_args("go", Some(&own_id), true), &service, &mut Scripted::default(), &mut console).await;
        assert_eq!(code, EXIT_NOT_CONFIRMED);
        let doc = console.json();
        assert_eq!(doc["outcome"], "unavailable");
        assert!(doc["detail"].as_str().unwrap().contains("the provider conversation was replaced"), "{doc}");
        assert!(delivered.lock().unwrap().is_empty(), "never retargeted to the new conversation");
        daemon.shutdown().await.unwrap();
    }

    /// Discovery frozen at a listing taken before the switch; delivery is
    /// the real daemon route.
    struct StaleRow(SessionListing);

    impl SteeringService for StaleRow {
        async fn discover(&self) -> Result<DiscoveryReport, DiscoveryFailed> {
            Ok(report(vec![self.0.clone()]))
        }

        async fn deliver(&self, row: &SessionListing, request: &SteeringRequest) -> Delivery {
            LocalService.deliver(row, request).await
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_consented_interruption_reports_both_phases_and_json_cannot_consent() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        let phases = InterruptionOutcome { cancellation: CancellationOutcome::Established, replacement: Some(SendOutcome::Unknown) };
        let (steering, delivered) = owner(&tmp, offering("rpc-abort-submit", CaseSupport::InterruptionRequired), Reply::Interrupted(phases));
        let own_id = steering.controller().snapshot().target.id();
        listed(|row| row.id == own_id).await;

        let mut console = Captured::interactive();
        let code = execute(send_args("go", Some(&own_id), true), &LocalService, &mut Scripted::default(), &mut console).await;
        assert_eq!(code, EXIT_NOT_CONFIRMED);
        assert!(delivered.lock().unwrap().is_empty(), "JSON never consents, so nothing was interrupted");

        let mut interaction = Scripted::default().consenting([Ok(true)]);
        let mut console = Captured::interactive();
        let code = execute(send_args("go", Some(&own_id), false), &LocalService, &mut interaction, &mut console).await;
        assert_eq!(code, EXIT_NOT_CONFIRMED, "a partial interruption is exit 1");
        assert!(flat(&console.out).contains("partial interruption"), "{}", console.out);
        assert!(flat(&console.out).contains("the running turn was stopped"), "{}", console.out);
        let requests = delivered.lock().unwrap().clone();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].may_interrupt(), "consent crossed the daemon bound to its target and operation");
        daemon.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_blocked_production_owner_is_listed_unavailable_with_its_reason() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        // The build's real eligibility: Pi's managed RPC profile is blocked.
        let (steering, delivered) =
            owner(&tmp, Arc::new(claudine::steering::eligibility::evaluate), Reply::Outcome(SendOutcome::Accepted));
        let own_id = steering.controller().snapshot().target.id();
        listed(|row| row.id == own_id).await;

        let mut console = Captured::piped();
        let code = execute(send_args("go", Some(&own_id), true), &LocalService, &mut Scripted::default(), &mut console).await;
        assert_eq!(code, EXIT_NOT_CONFIRMED);
        let detail = console.json()["detail"].as_str().unwrap().to_string();
        assert!(detail.contains("steering is blocked for this launch profile"), "{detail}");
        assert!(delivered.lock().unwrap().is_empty());

        let mut console = Captured::piped();
        execute(list_args(false), &LocalService, &mut Scripted::default(), &mut console).await;
        assert!(flat(&console.out).contains("unavailable") && flat(&console.out).contains("steering is blocked"), "{}", console.out);
        daemon.shutdown().await.unwrap();
    }
}
