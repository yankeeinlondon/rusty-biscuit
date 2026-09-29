//! Automatic repetition help for one agent execution.
//!
//! The live semantic sink hands every [`RepetitionSignal`] from its content
//! detector to [`AutomaticHelp`]. A warning claims one of the execution's
//! opportunities and, when the execution's own controller offers an
//! automatic (non-interrupting) route, submits the helper message to it on
//! the Tokio runtime; otherwise it prints a notice that nothing can be sent.
//! Nothing here blocks the stream reader, and nothing here touches the
//! detector: warnings, sends, and replies never reset repetition evidence or
//! timeout clocks, and the existing hard stop keeps its schedule.
//!
//! A hard stop wins: [`AutomaticHelp::hard_stop`] refuses further warnings,
//! abandons a send in flight, and stops the controller, so nothing reaches
//! the provider after Claudine decided to terminate it.
//!
//! Topic: `claudine/docs/topics/automatic-steering.md`.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use claudine::runaway::RepetitionSignal;
use claudine::steering::automatic::{HELPER_MESSAGE, OPPORTUNITIES_PER_EXECUTION, OpportunityBudget};
use claudine::steering::contract::{SteeringMessage, SteeringOrigin, SteeringRequest};
use claudine::steering::controller::{ControllerReply, SteeringController, Submission};
use claudine::steering::identity::RequestId;

/// Why automatic help has no route when the execution has no controller.
pub(crate) const NO_OWNER_REASON: &str = "this execution has no steering owner";

/// How a notice should be presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoticeLevel {
    /// The provider confirmed the warning.
    Sent,
    /// Nothing could be sent, or the send was not confirmed.
    Warning,
}

/// Prints one notice on STDERR.
pub(crate) type NoticeFn = Arc<dyn Fn(NoticeLevel, &str) + Send + Sync>;

/// Automatic help for one execution; created only when it is enabled.
pub(crate) struct AutomaticHelp {
    controller: Option<SteeringController>,
    runtime: Option<tokio::runtime::Handle>,
    budget: OpportunityBudget,
    notices: Notices,
    delivery: Option<tokio::task::JoinHandle<()>>,
    stopped: bool,
}

/// Notice output, deduplicated by text so a repeated failure cannot flood
/// STDERR.
#[derive(Clone)]
struct Notices {
    emit: NoticeFn,
    shown: Arc<Mutex<HashSet<String>>>,
}

impl Notices {
    fn show(&self, level: NoticeLevel, text: String) {
        let first = self.shown.lock().unwrap_or_else(|poison| poison.into_inner()).insert(text.clone());
        if first {
            (self.emit)(level, &text);
        }
    }
}

impl AutomaticHelp {
    /// Help for the execution `controller` owns, sending on the current Tokio
    /// runtime. Without a controller or a runtime every warning is reported
    /// as unavailable.
    pub(crate) fn new(controller: Option<SteeringController>, notice: NoticeFn) -> Self {
        Self {
            controller,
            runtime: tokio::runtime::Handle::try_current().ok(),
            budget: OpportunityBudget::default(),
            notices: Notices { emit: notice, shown: Arc::default() },
            delivery: None,
            stopped: false,
        }
    }

    /// Handles one detector signal without blocking.
    pub(crate) fn on_signal(&mut self, signal: &RepetitionSignal) {
        let RepetitionSignal::Warning { cycle_len, repeats, stop_limit } = *signal else {
            tracing::debug!(target: "claudine::steering", ?signal, "repetition episode recovered");
            return;
        };
        if self.stopped {
            return;
        }
        let Some(opportunity) = self.budget.claim() else {
            return;
        };
        let (Some(controller), Some(runtime)) = (&self.controller, &self.runtime) else {
            self.unavailable(NO_OWNER_REASON, stop_limit);
            return;
        };
        let route = match controller.automatic_route() {
            Ok(route) => route,
            Err(reason) => {
                self.unavailable(&reason, stop_limit);
                return;
            }
        };
        let message = match SteeringMessage::new(helper_message(cycle_len, repeats)) {
            Ok(message) => message,
            Err(error) => {
                self.unavailable(&crate::steering::render_chain("the helper message is invalid", &error), stop_limit);
                return;
            }
        };
        let request = SteeringRequest {
            id: RequestId::random(),
            target: controller.snapshot().target.id(),
            origin: SteeringOrigin::Automatic,
            operation: route.mechanism.operation_intent,
            message,
            consent: None,
        };
        let submission = Submission { request, expected: None, opportunity: Some(opportunity) };
        let controller = controller.clone();
        let notices = self.notices.clone();
        let ordinal = self.budget.used();
        self.delivery = Some(runtime.spawn(async move {
            let reply = controller.submit(submission).await;
            report(&notices, &reply, ordinal, stop_limit);
        }));
    }

    /// A content guard tripped: no further warning is sent, a send in
    /// flight is abandoned, and the controller stops.
    pub(crate) fn hard_stop(&mut self) {
        self.stopped = true;
        if let Some(delivery) = self.delivery.take() {
            delivery.abort();
        }
        if let Some(controller) = &self.controller {
            controller.shutdown();
        }
    }

    fn unavailable(&self, reason: &str, stop_limit: usize) {
        tracing::info!(target: "claudine::steering", reason, "automatic steering unavailable");
        self.notices.show(
            NoticeLevel::Warning,
            format!(
                "Claudine detected repeated output but cannot send automatic steering to this session: {reason}. \
                 It will keep enforcing the repetition limit of {stop_limit}."
            ),
        );
    }
}

/// Notices rendered as status lines on a wrapped run's STDERR.
pub(crate) fn stderr_notices(output: Arc<crate::commands::wrap::stream_io::StreamOutput>) -> NoticeFn {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::components::status::{Status, StatusState};

    let term = crate::log::terminal();
    Arc::new(move |level, text| {
        let state = match level {
            NoticeLevel::Sent => StatusState::Info,
            NoticeLevel::Warning => StatusState::Warning,
        };
        output.emit_stderr_line(&Status::new(text).state(state).render(&term));
    })
}

/// The helper message plus the observed counts. It never echoes output.
pub(crate) fn helper_message(cycle_len: usize, repeats: usize) -> String {
    let block = if cycle_len == 1 { "line".to_string() } else { format!("{cycle_len}-line block") };
    format!("{HELPER_MESSAGE}\n\n(Observed: the same {block} {repeats} times in a row.)")
}

fn report(notices: &Notices, reply: &ControllerReply, ordinal: usize, stop_limit: usize) {
    let outcome = crate::commands::steer::render::outcome_word(reply.result.outcome);
    if reply.result.outcome.is_confirmed() {
        notices.show(
            NoticeLevel::Sent,
            format!(
                "Claudine detected repeated output and sent the agent a warning ({outcome}; {ordinal} of \
                 {OPPORTUNITIES_PER_EXECUTION}). The repetition limit of {stop_limit} still applies."
            ),
        );
        return;
    }
    let detail = reply.detail.as_ref().map(|detail| format!(": {}", detail.as_str())).unwrap_or_default();
    notices.show(
        NoticeLevel::Warning,
        format!(
            "Claudine's automatic repetition warning was not confirmed ({outcome}{detail}). \
             It will keep enforcing the repetition limit of {stop_limit}."
        ),
    );
}

#[cfg(test)]
pub(crate) mod tests;
