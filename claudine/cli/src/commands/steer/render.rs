//! Terminal and JSON output for `claudine steer`.
//!
//! Every availability is spelled out in words (`non-interrupting`,
//! `interruption required`, `unavailable`), so a listing is understood
//! without color or strikethrough; styling only reinforces it. The JSON
//! shapes are the versioned script contract documented in
//! `claudine/docs/cli/steer.md`.

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Edges, Layout, Length, WordWrap};
use chrono::{DateTime, Utc};
use claudine::secrets::RedactedText;
use claudine::steering::contract::{CancellationOutcome, InterruptionOutcome, SendOutcome};
use claudine::steering::discovery::{DiscoveryError, DiscoveryGap, DiscoveryReport, ObservationSource, SessionListing};
use claudine::steering::identity::{RequestId, SteeringTargetId};
use claudine::steering::vocabulary::{ExecutionState, OperationIntent, ReceiptStrength, SteeringAvailability};
use serde::Serialize;

use super::service::Delivery;

/// Version of both JSON documents. Bump on any breaking change.
pub(crate) const SCHEMA_VERSION: u32 = 1;

/// `claudine steer --list --json`.
#[derive(Debug, Serialize)]
pub(crate) struct ListDocument<'a> {
    pub(crate) schema_version: u32,
    pub(crate) observed_at: DateTime<Utc>,
    pub(crate) sessions: &'a [SessionListing],
    pub(crate) discovery_errors: &'a [DiscoveryError],
    /// Researched native discovery methods this build does not implement:
    /// sessions they would find are missing from `sessions`.
    pub(crate) coverage_gaps: &'a [DiscoveryGap],
}

impl<'a> ListDocument<'a> {
    pub(crate) fn new(report: &'a DiscoveryReport) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            observed_at: Utc::now(),
            sessions: &report.sessions,
            discovery_errors: &report.errors,
            coverage_gaps: &report.gaps,
        }
    }
}

/// `claudine steer --session <id> --json "<message>"`.
#[derive(Debug, Serialize)]
pub(crate) struct SendDocument<'a> {
    pub(crate) schema_version: u32,
    pub(crate) request_id: RequestId,
    pub(crate) target: &'a SteeringTargetId,
    pub(crate) operation: OperationIntent,
    pub(crate) mechanism: Option<&'static str>,
    pub(crate) outcome: SendOutcome,
    pub(crate) receipt: ReceiptStrength,
    /// Present for an interrupt-then-submit request that was attempted.
    pub(crate) interruption: Option<InterruptionOutcome>,
    /// Why the outcome is not a confirmation; secret-masked.
    pub(crate) detail: Option<&'a RedactedText>,
}

impl<'a> SendDocument<'a> {
    pub(crate) fn new(operation: OperationIntent, delivery: &'a Delivery) -> Self {
        let result = &delivery.result;
        Self {
            schema_version: SCHEMA_VERSION,
            request_id: result.request_id,
            target: &result.target,
            operation,
            mechanism: result.mechanism,
            outcome: result.outcome,
            receipt: result.receipt,
            interruption: result.interruption,
            detail: delivery.detail.as_ref(),
        }
    }
}

pub(crate) fn availability_label(availability: SteeringAvailability) -> &'static str {
    match availability {
        SteeringAvailability::NonInterrupting => "non-interrupting",
        SteeringAvailability::InterruptionRequired => "interruption required",
        SteeringAvailability::Unavailable => "unavailable",
    }
}

fn state_label(state: ExecutionState) -> &'static str {
    match state {
        ExecutionState::Working => "working",
        ExecutionState::Idle => "idle",
        ExecutionState::Unknown => "unknown",
    }
}

/// What sending to a row would do, in words.
pub(crate) fn operation_effect(operation: OperationIntent) -> &'static str {
    match operation {
        OperationIntent::SteerActiveTurn => "adds the message to the running turn",
        OperationIntent::QueueFollowUp => "queues the message to be read after the current work",
        OperationIntent::StartIdleTurn => "the session is idle, so the message starts a new turn",
        OperationIntent::InterruptThenSubmit => "stops the running turn, then sends the message",
        OperationIntent::Unknown => "the delivery effect is unknown",
    }
}

/// A short session label: its name, else the start of its identity.
fn session_label(row: &SessionListing) -> String {
    if let Some(name) = row.name.as_deref().filter(|name| !name.trim().is_empty()) {
        return name.to_string();
    }
    match &row.id {
        SteeringTargetId::Managed { execution } => execution.to_string().chars().take(8).collect(),
        SteeringTargetId::Native { conversation, .. } => conversation.chars().take(12).collect(),
    }
}

/// One plain-text picker line. Unavailable rows keep their label, so the
/// reason for disabling one is legible without styling.
pub(crate) fn picker_label(index: usize, row: &SessionListing) -> String {
    format!(
        "#{} {} · {} · {} · {} · {}",
        index + 1,
        row.provider.as_slug(),
        session_label(row),
        row.cwd.as_deref().unwrap_or("unknown directory"),
        state_label(row.state),
        availability_label(row.availability.availability),
    )
}

/// The listing: a Provider/Session/Directory/State/Steering table, then
/// each row's full ID and wrapped details. Unavailable rows are dim and
/// struck through in the table; their details stay unstruck so the reason
/// reads easily.
#[derive(Debug)]
pub(crate) struct SessionTable {
    rows: Vec<SessionListing>,
    gaps: Vec<DiscoveryGap>,
    layout: Layout,
}

impl SessionTable {
    pub(crate) fn new(report: &DiscoveryReport) -> Self {
        Self { rows: report.sessions.clone(), gaps: report.gaps.clone(), layout: Layout::default() }
    }

    fn table(&self) -> Table {
        let mut table = Table::new().with_columns(vec![
            TableColumn::new("#"),
            TableColumn::new("Provider"),
            TableColumn::new("Session"),
            TableColumn::new("Directory"),
            TableColumn::new("State"),
            TableColumn::new("Steering"),
        ]);
        table.layout_mut().margin = Edges::x(Length::ch(1));
        for (index, row) in self.rows.iter().enumerate() {
            let selectable = row.is_selectable();
            table.add_row(vec![
                cell(&(index + 1).to_string(), selectable).into(),
                cell(row.provider.as_slug(), selectable).into(),
                cell(&session_label(row), selectable).into(),
                cell(row.cwd.as_deref().unwrap_or("unknown"), selectable).into(),
                cell(state_label(row.state), selectable).into(),
                cell(availability_label(row.availability.availability), selectable).into(),
            ]);
        }
        table
    }
}

/// A standalone line that wraps at the terminal width. (List items are
/// wrapped by their list; table cells by their column.)
fn line(markup: impl Into<String>) -> Prose {
    Prose::new(markup.into()).with_word_wrap(WordWrap::WrapProse(None, Some(2)))
}

fn cell(text: &str, selectable: bool) -> Prose {
    let text = Prose::escape_text(text);
    if selectable { line(text) } else { line(format!("<dim><strikethrough>{text}</strikethrough></dim>")) }
}

fn details(row: &SessionListing) -> Vec<RenderableTerminalContent> {
    let escape = |text: &str| Prose::escape_text(text).to_string();
    let mut items: Vec<RenderableTerminalContent> = Vec::new();
    let availability = &row.availability;
    if let Some(operation) = availability.operation {
        items.push(Prose::new(format!("Sending {}.", escape(operation_effect(operation)))).into());
    }
    if let Some(reason) = availability.reason.as_deref() {
        let label = if availability.availability == SteeringAvailability::Unavailable { "Why" } else { "Note" };
        items.push(Prose::new(format!("<bold>{label}:</bold> {}", escape(reason))).into());
    }
    for setup in &availability.setup_requirements {
        items.push(
            Prose::new(format!(
                "<bold>Setup for future launches:</bold> {} <dim>(setup cannot change this open session)</dim>",
                escape(setup)
            ))
            .into(),
        );
    }
    let origins: Vec<&str> = row
        .origins
        .iter()
        .map(|origin| match origin {
            ObservationSource::Managed => "Claudine wrapper",
            ObservationSource::Native => "provider",
        })
        .collect();
    items.push(
        Prose::new(format!(
            "<dim>Seen by {}; launch profile {}; provider version {}.</dim>",
            origins.join(" and "),
            escape(row.launch_profile.as_deref().unwrap_or("unknown")),
            escape(row.provider_version.as_deref().unwrap_or("unknown")),
        ))
        .into(),
    );
    items
}

impl TerminalRenderable for SessionTable {
    fn render(&self, term: &Terminal) -> String {
        let mut lines: Vec<String> = Vec::new();
        if self.rows.is_empty() {
            lines.push(line("No active sessions were found.").render(term));
        } else {
            let table = self.table();
            // Too narrow for six columns: each row gets the same facts as
            // one styled summary line instead of a table that cannot render.
            let tabular = table.plan_widths_for_terminal(term).is_ok();
            if tabular {
                lines.push(table.render(term));
            }
            for (index, row) in self.rows.iter().enumerate() {
                lines.push(String::new());
                if !tabular {
                    lines.push(cell(&picker_label(index, row), row.is_selectable()).render(term));
                }
                let id = Prose::escape_text(&row.id.to_string()).to_string();
                let heading = if tabular { format!("<bold>#{}</bold> {id}", index + 1) } else { format!("  {id}") };
                let heading = if row.is_selectable() { heading } else { format!("<dim>{heading}</dim>") };
                // Never wrapped: the ID is copied whole into `--session`.
                lines.push(Prose::new(heading).render(term));
                lines.push(UnorderedList::from(details(row)).render(term));
            }
        }
        if !self.gaps.is_empty() {
            lines.push(String::new());
            let providers: Vec<&str> = {
                let mut providers: Vec<&str> = self.gaps.iter().map(|gap| gap.provider.as_slug()).collect();
                providers.dedup();
                providers
            };
            lines.push(
                line(format!(
                    "<dim>Sessions launched outside Claudine are not listed for: {}. This build cannot discover them yet.</dim>",
                    providers.join(", ")
                ))
                .render(term),
            );
        }
        lines.join("\n")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

/// What the outcome establishes, in words. Only accepted, queued, and
/// delivered are confirmations, and none proves the agent acted on it.
pub(crate) fn outcome_explanation(outcome: SendOutcome) -> &'static str {
    match outcome {
        SendOutcome::Accepted => {
            "The provider accepted the message. It has not confirmed the message reached the conversation, \
             and acceptance does not mean the agent acted on it."
        }
        SendOutcome::Queued => {
            "The provider queued the message for delivery at its next boundary. It is not in the conversation yet."
        }
        SendOutcome::Delivered => {
            "The message was delivered into the conversation. This does not prove the agent acted on it."
        }
        SendOutcome::Held => {
            "The provider's inbound policy is holding the message; it has not been delivered. Changing that policy \
             is separate setup: `claudine steer` never changes provider configuration."
        }
        SendOutcome::Refused => "The request was refused. Nothing was delivered.",
        SendOutcome::Unavailable => "Nothing was sent: no usable steering route reached the session.",
        SendOutcome::Busy => {
            "Nothing was sent: the session's steering queue was full, or the request expired before it could be submitted."
        }
        SendOutcome::PartialInterruption => {
            "The running turn was stopped, but the replacement message was not confirmed. The old work is not restarted."
        }
        SendOutcome::Unknown => {
            "The message may have been submitted, but nothing confirmed it. It was not retried, to avoid a duplicate; \
             check the session before sending again."
        }
    }
}

fn outcome_word(outcome: SendOutcome) -> &'static str {
    match outcome {
        SendOutcome::Accepted => "accepted",
        SendOutcome::Queued => "queued",
        SendOutcome::Delivered => "delivered",
        SendOutcome::Refused => "refused",
        SendOutcome::Held => "held (undelivered)",
        SendOutcome::Unavailable => "not sent",
        SendOutcome::Busy => "not sent (busy)",
        SendOutcome::PartialInterruption => "partial interruption",
        SendOutcome::Unknown => "unknown",
    }
}

fn cancellation_words(cancellation: CancellationOutcome) -> &'static str {
    match cancellation {
        CancellationOutcome::Established => "the running turn was stopped",
        CancellationOutcome::Refused => "the provider refused to stop the running turn",
        CancellationOutcome::Unknown => "whether the running turn stopped could not be established",
    }
}

/// The human receipt for one delivery.
pub(crate) fn receipt(delivery: &Delivery, term: &Terminal) -> String {
    let result = &delivery.result;
    let color = if result.outcome.is_confirmed() { "green" } else { "yellow" };
    let mut heading = format!("<{color}><bold>{}</bold></{color}> · {}", outcome_word(result.outcome), result.target);
    if let Some(mechanism) = result.mechanism {
        heading.push_str(&format!(" · via `{mechanism}`"));
    }
    let mut items: Vec<RenderableTerminalContent> =
        vec![Prose::new(Prose::escape_text(outcome_explanation(result.outcome)).to_string()).into()];
    if let Some(phases) = result.interruption {
        let replacement = match phases.replacement {
            Some(outcome) => format!("the replacement message was {}", outcome_word(outcome)),
            None => "the replacement message was not sent".to_string(),
        };
        items.push(Prose::new(format!("Interruption: {}; {replacement}.", cancellation_words(phases.cancellation))).into());
    }
    if let Some(detail) = delivery.detail.as_ref() {
        items.push(Prose::new(format!("<bold>Detail:</bold> {}", Prose::escape_text(detail.as_str()))).into());
    }
    items.push(Prose::new(format!("<dim>Request {}</dim>", result.request_id)).into());
    [line(heading).render(term), UnorderedList::from(items).render(term)].join("\n")
}

/// What consenting to an interruption of `row` would do. Shown before the
/// consent prompt; nothing is interrupted unless the user then agrees.
pub(crate) fn interruption_explanation(row: &SessionListing, term: &Terminal) -> String {
    let heading = format!(
        "<bold>Steering this session requires interrupting it.</bold> {} session {} in {}:",
        row.provider.as_slug(),
        Prose::escape_text(&session_label(row)),
        Prose::escape_text(row.cwd.as_deref().unwrap_or("an unknown directory")),
    );
    let items: Vec<RenderableTerminalContent> = vec![
        Prose::new("The running turn is stopped first; your message is sent only after the stop is confirmed.").into(),
        Prose::new(
            "A tool the agent is running may be stopped with the turn. What happens to that tool's partial work \
             has not been established for this provider.",
        )
        .into(),
        Prose::new("Messages already waiting in the session are kept, not cleared, and may shape the replacement work.")
            .into(),
        Prose::new(
            "If the stop succeeds but your message is not confirmed, that is reported as a partial interruption; \
             the stopped work is not restarted.",
        )
        .into(),
    ];
    [line(heading).render(term), UnorderedList::from(items).render(term)].join("\n")
}
