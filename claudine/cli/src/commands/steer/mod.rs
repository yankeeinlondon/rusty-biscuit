//! `claudine steer` — send one message to one running agent session.
//!
//! ```text
//! claudine steer "message"                  # pick a session interactively
//! claudine steer --session <id> "message"   # exact target, no picker
//! claudine steer --list [--json]            # list sessions; sends nothing
//! ```
//!
//! The command never broadcasts and never infers a target: without
//! `--session` it needs a terminal to ask, even when only one session is
//! selectable. An explicit ID skips only the picker. A session that can be
//! reached only by interrupting it needs interactive consent every time, and
//! `--json` or a missing terminal refuses instead of asking. After a human
//! decision the session is looked up again, and a changed target or action
//! is reported, never followed.
//!
//! Exit codes: 0 when the provider confirmed acceptance, queueing, or
//! delivery (or a listing succeeded); 1 for every other outcome; 2 for
//! usage errors; 130 when the user cancelled before anything was sent.
//!
//! User guide: `claudine/docs/cli/steer.md`. Routing and receipts:
//! `claudine/docs/topics/steering-routing.md`.

mod interact;
mod render;
mod service;
#[cfg(test)]
mod tests;

use std::io::IsTerminal;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use clap::Args;
use claudine::steering::contract::{InterruptionConsent, SendOutcome, SteeringMessage, SteeringOrigin, SteeringRequest, SteeringResult};
use claudine::steering::discovery::{DiscoveryReport, SessionListing};
use claudine::steering::identity::{RequestId, SteeringTargetId};
use claudine::steering::vocabulary::{OperationIntent, SteeringAvailability};

use crate::log;
use crate::steering::render_chain;
use interact::{Interaction, TerminalInteraction};
use service::{Delivery, LocalService, SteeringService};

/// The provider confirmed acceptance, queueing, or delivery.
pub(crate) const EXIT_CONFIRMED: i32 = 0;
/// Refused, held, unavailable, busy, partial, unknown, or nothing to send to.
pub(crate) const EXIT_NOT_CONFIRMED: i32 = 1;
/// The invocation itself was invalid.
pub(crate) const EXIT_USAGE: i32 = 2;
/// The user cancelled before anything was sent.
pub(crate) const EXIT_CANCELLED: i32 = 130;

/// Arguments for `claudine steer`.
#[derive(Debug, Args)]
pub struct SteerArgs {
    /// The message, delivered byte for byte. Put `--` before a message that
    /// starts with `-`.
    #[arg(value_name = "MESSAGE", required_unless_present = "list")]
    pub message: Option<String>,

    /// List sessions, including unavailable ones with their reasons. Sends
    /// nothing.
    #[arg(long, conflicts_with_all = ["message", "session"])]
    pub list: bool,

    /// Send to this exact session ID from `--list`. Skips the picker, not
    /// the availability check or interruption consent.
    #[arg(long, value_name = "ID")]
    pub session: Option<String>,

    /// Print the typed result as JSON. Never prompts: a send needs
    /// `--session`, and one that requires interruption is refused.
    #[arg(long)]
    pub json: bool,
}

/// Where the command writes, and whether it may ask the user anything.
pub(crate) trait Console {
    /// Pipeable result: stdout.
    fn data(&mut self, text: &str);
    /// Status, warnings, and explanations: stderr.
    fn status(&mut self, text: &str);
    fn terminal(&self) -> Terminal;
    /// Whether a human can answer a prompt (stdin and stderr are terminals).
    fn interactive(&self) -> bool;
}

struct StdConsole;

impl Console for StdConsole {
    fn data(&mut self, text: &str) {
        log::data(text);
    }

    fn status(&mut self, text: &str) {
        log::message(text);
    }

    fn terminal(&self) -> Terminal {
        log::terminal()
    }

    fn interactive(&self) -> bool {
        std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
    }
}

/// Runs `claudine steer` and returns its exit code.
pub async fn run(args: SteerArgs) -> i32 {
    execute(args, &LocalService, &mut TerminalInteraction, &mut StdConsole).await
}

pub(crate) async fn execute(
    args: SteerArgs,
    service: &impl SteeringService,
    interaction: &mut impl Interaction,
    console: &mut impl Console,
) -> i32 {
    if args.list {
        return list(args.json, service, console).await;
    }
    let message = match SteeringMessage::new(args.message.unwrap_or_default()) {
        Ok(message) => message,
        Err(invalid) => return usage(console, &render_chain("the message cannot be sent", &invalid)),
    };
    let explicit = match args.session.as_deref().map(str::parse::<SteeringTargetId>) {
        Some(Ok(id)) => Some(id),
        Some(Err(malformed)) => {
            return usage(console, &render_chain("`--session` is not a session ID from `claudine steer --list`", &malformed));
        }
        None => None,
    };
    let mode = Mode { json: args.json, interactive: console.interactive() };
    if explicit.is_none() {
        if mode.json {
            return usage(console, "`--json` sends only to an explicit `--session <id>`; list IDs with `claudine steer --list --json`");
        }
        if !mode.interactive {
            return usage(
                console,
                "no terminal is available to choose a session; list sessions with `claudine steer --list` and pass `--session <id>`",
            );
        }
    }

    let report = match discover(service, console).await {
        Some(report) => report,
        None => return EXIT_NOT_CONFIRMED,
    };
    let mut send = Send { service, interaction, console, mode, message };
    let row = match &explicit {
        Some(id) => match report.sessions.iter().find(|row| row.id == *id) {
            Some(row) => row.clone(),
            None => {
                return send.unsent(
                    id,
                    OperationIntent::Unknown,
                    "no listed session has this ID; it may have ended or been replaced. Nothing was sent and it was not \
                     redirected. List sessions again with `claudine steer --list`",
                );
            }
        },
        None => match send.pick(&report) {
            Ok(row) => row,
            Err(code) => return code,
        },
    };
    // An explicit row comes from the discovery just run, with no human pause
    // since, so the owner's own check at submission is enough; a picked row
    // waited on a human and is looked up again first.
    send.deliver(row, explicit.is_some()).await
}

#[derive(Debug, Clone, Copy)]
struct Mode {
    json: bool,
    interactive: bool,
}

fn usage(console: &mut impl Console, problem: &str) -> i32 {
    let term = console.terminal();
    console.status(&Prose::new(format!("<red><bold>Error:</bold></red> {}", Prose::escape_text(problem))).render(&term));
    EXIT_USAGE
}

/// One discovery pass; partial errors are warned about, total failure is
/// reported and returns `None`.
async fn discover(service: &impl SteeringService, console: &mut impl Console) -> Option<DiscoveryReport> {
    let term = console.terminal();
    match service.discover().await {
        Ok(report) => {
            for error in &report.errors {
                console.status(
                    &Prose::new(format!(
                        "<orange><bold>warning:</bold></orange> discovery from {} failed: {}",
                        Prose::escape_text(&error.source),
                        Prose::escape_text(&error.message())
                    ))
                    .render(&term),
                );
            }
            Some(report)
        }
        Err(failed) => {
            let causes: Vec<String> =
                failed.errors.iter().map(|error| format!("{}: {}", error.source, error.message())).collect();
            console.status(
                &Prose::new(format!(
                    "<red><bold>Error:</bold></red> session discovery failed for every source ({}). Is the local \
                     Rendezvous daemon running?",
                    Prose::escape_text(&causes.join("; "))
                ))
                .render(&term),
            );
            None
        }
    }
}

async fn list(json: bool, service: &impl SteeringService, console: &mut impl Console) -> i32 {
    let Some(report) = discover(service, console).await else {
        return EXIT_NOT_CONFIRMED;
    };
    if json {
        match serde_json::to_string_pretty(&render::ListDocument::new(&report)) {
            Ok(text) => console.data(&text),
            Err(unencodable) => {
                return usage(console, &render_chain("the listing could not be encoded as JSON", &unencodable));
            }
        }
    } else {
        let term = console.terminal();
        console.data(&render::SessionTable::new(&report).render(&term));
    }
    EXIT_CONFIRMED
}

/// One send in progress.
struct Send<'a, S, I, C> {
    service: &'a S,
    interaction: &'a mut I,
    console: &'a mut C,
    mode: Mode,
    message: SteeringMessage,
}

impl<S: SteeringService, I: Interaction, C: Console> Send<'_, S, I, C> {
    fn status(&mut self, markup: &str) {
        let term = self.console.terminal();
        self.console.status(&Prose::new(markup).render(&term));
    }

    /// Shows the listing and asks for one selectable row.
    fn pick(&mut self, report: &DiscoveryReport) -> Result<SessionListing, i32> {
        let term = self.console.terminal();
        self.console.status(&render::SessionTable::new(report).render(&term));
        if report.sessions.is_empty() {
            self.status("Nothing was sent.");
            return Err(EXIT_NOT_CONFIRMED);
        }
        if !report.sessions.iter().any(SessionListing::is_selectable) {
            self.status("None of these sessions can be steered; the reasons are listed above. Nothing was sent.");
            return Err(EXIT_NOT_CONFIRMED);
        }
        match self.interaction.choose(&report.sessions) {
            Ok(Some(id)) => match report.sessions.iter().find(|row| row.id == id && row.is_selectable()) {
                Some(row) => Ok(row.clone()),
                // The picker disables unavailable rows; never trust it alone.
                None => Err(self.unsent(&id, OperationIntent::Unknown, "the chosen session cannot be steered; nothing was sent")),
            },
            Ok(None) => {
                self.status("Cancelled. Nothing was sent.");
                Err(EXIT_CANCELLED)
            }
            Err(prompt) => {
                let failure = render_chain("the session picker failed", &prompt);
                self.status(&format!("<red><bold>Error:</bold></red> {}. Nothing was sent.", Prose::escape_text(&failure)));
                Err(EXIT_NOT_CONFIRMED)
            }
        }
    }

    /// Consent (when required), revalidation, delivery, and the receipt.
    /// `fresh` is true when no human pause separates discovery from here.
    async fn deliver(&mut self, mut row: SessionListing, mut fresh: bool) -> i32 {
        let mut consented = false;
        // A second pass happens only when revalidation finds the session now
        // needs interruption; the same explanation and choice are required.
        for _ in 0..2 {
            match row.availability.availability {
                SteeringAvailability::Unavailable => {
                    let reason = row.availability.reason.clone().unwrap_or_else(|| "no usable steering route".into());
                    return self.unsent(&row.id, OperationIntent::Unknown, &format!("{reason}. Nothing was sent"));
                }
                SteeringAvailability::InterruptionRequired if !consented => match self.consent(&row) {
                    Ok(()) => {
                        consented = true;
                        fresh = false;
                    }
                    Err(code) => return code,
                },
                _ => {}
            }
            if fresh {
                return self.submit(&row, consented).await;
            }
            match self.revalidate(&row).await {
                Ok(current) if same_action(&row, &current) => return self.submit(&current, consented).await,
                Ok(current) if current.availability.availability == SteeringAvailability::InterruptionRequired => {
                    self.status(
                        "<yellow>This session can no longer be steered without interrupting it.</yellow> Nothing has \
                         been sent; decide again.",
                    );
                    row = current;
                    consented = false;
                }
                Ok(current) => {
                    let now = match current.availability.operation {
                        Some(operation) => format!("sending now {}", render::operation_effect(operation)),
                        None => format!(
                            "it is now unavailable: {}",
                            current.availability.reason.as_deref().unwrap_or("no usable steering route")
                        ),
                    };
                    return self.unsent(
                        &row.id,
                        row.availability.operation.unwrap_or(OperationIntent::Unknown),
                        &format!("the session changed since it was chosen ({now}). Nothing was sent; run the command again"),
                    );
                }
                Err(code) => return code,
            }
        }
        self.unsent(
            &row.id,
            row.availability.operation.unwrap_or(OperationIntent::Unknown),
            "the session kept changing while it was being confirmed. Nothing was sent; run the command again",
        )
    }

    /// Explains the interruption and asks. Never infers consent.
    fn consent(&mut self, row: &SessionListing) -> Result<(), i32> {
        let operation = row.availability.operation.unwrap_or(OperationIntent::InterruptThenSubmit);
        let refuse = |why: &str| {
            format!(
                "delivery to this session requires interrupting its running turn, and {why}. Nothing was sent or \
                 interrupted; run the command in a terminal to decide"
            )
        };
        if self.mode.json {
            return Err(self.unsent(&row.id, operation, &refuse("`--json` never asks for consent")));
        }
        if !self.mode.interactive {
            return Err(self.unsent(&row.id, operation, &refuse("no terminal is available to ask for consent")));
        }
        let term = self.console.terminal();
        self.console.status(&render::interruption_explanation(row, &term));
        match self.interaction.confirm_interruption(row) {
            Ok(true) => Ok(()),
            Ok(false) => {
                self.status("Not interrupted. Nothing was sent.");
                Err(EXIT_CANCELLED)
            }
            Err(prompt) => {
                let failure = render_chain("the consent prompt failed", &prompt);
                Err(self.unsent(&row.id, operation, &format!("{failure}; consent was not given. Nothing was sent or interrupted")))
            }
        }
    }

    /// The same session as discovered now, or the exit code for why not.
    async fn revalidate(&mut self, chosen: &SessionListing) -> Result<SessionListing, i32> {
        let operation = chosen.availability.operation.unwrap_or(OperationIntent::Unknown);
        let Some(report) = discover(self.service, self.console).await else {
            return Err(self.unsent(&chosen.id, operation, "the session could not be checked again before sending. Nothing was sent"));
        };
        match report.sessions.into_iter().find(|row| row.id == chosen.id) {
            Some(current) if current.binding == chosen.binding => Ok(current),
            _ => Err(self.unsent(
                &chosen.id,
                operation,
                "the chosen session ended or its conversation was replaced since it was listed. Nothing was sent, and \
                 the message was not redirected to another session",
            )),
        }
    }

    async fn submit(&mut self, row: &SessionListing, consented: bool) -> i32 {
        let Some(operation) = row.availability.operation else {
            return self.unsent(&row.id, OperationIntent::Unknown, "the session offers no steering operation. Nothing was sent");
        };
        let request = SteeringRequest {
            id: RequestId::random(),
            target: row.id.clone(),
            origin: SteeringOrigin::Manual,
            operation,
            message: self.message.clone(),
            consent: (consented && operation == OperationIntent::InterruptThenSubmit)
                .then(|| InterruptionConsent { target: row.id.clone(), operation }),
        };
        if operation == OperationIntent::StartIdleTurn && !self.mode.json {
            self.status("<dim>This session is idle, so the message starts a new turn.</dim>");
        }
        let delivery = self.service.deliver(row, &request).await;
        self.report(operation, &delivery)
    }

    /// Reports a request that was never handed to an owner.
    fn unsent(&mut self, target: &SteeringTargetId, operation: OperationIntent, why: &str) -> i32 {
        let request = SteeringRequest {
            id: RequestId::random(),
            target: target.clone(),
            origin: SteeringOrigin::Manual,
            operation,
            message: self.message.clone(),
            consent: None,
        };
        let delivery = Delivery {
            result: SteeringResult::submitted(&request, None, SendOutcome::Unavailable),
            detail: Some(claudine::secrets::Redactor::for_message(self.message.as_str()).redact(why)),
        };
        self.report(operation, &delivery)
    }

    fn report(&mut self, operation: OperationIntent, delivery: &Delivery) -> i32 {
        if self.mode.json {
            match serde_json::to_string_pretty(&render::SendDocument::new(operation, delivery)) {
                Ok(text) => self.console.data(&text),
                Err(unencodable) => {
                    return usage(self.console, &render_chain("the result could not be encoded as JSON", &unencodable));
                }
            }
        } else {
            let term = self.console.terminal();
            self.console.data(&render::receipt(delivery, &term));
        }
        if delivery.result.outcome.is_confirmed() { EXIT_CONFIRMED } else { EXIT_NOT_CONFIRMED }
    }
}

/// Whether `current` would perform exactly the action chosen from `chosen`.
fn same_action(chosen: &SessionListing, current: &SessionListing) -> bool {
    current.availability.availability == chosen.availability.availability
        && current.availability.operation == chosen.availability.operation
}
