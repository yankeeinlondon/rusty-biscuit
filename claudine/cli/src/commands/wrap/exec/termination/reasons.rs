//! Provider-neutral termination reasons and the projection from
//! watchdog/detector inputs into the [`EarlyTermination`] the wait loop carries
//! and its resulting [`ProcessTermination`].
//!
//! [`ProcessTermination`]: claudine::harness::ProcessTermination

use claudine::runaway::Trip;
use claudine::stream::logs::EarlyTermination;

/// Reason for a watchdog-initiated termination.
///
/// The unified two-rule design has exactly two reasons. Stuck-subagent
/// detail is surfaced through [`WatchdogTermination::stuck_subagents`] for
/// the rendered error block, not through a distinct reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WatchdogTerminationReason {
    /// Wall-clock budget (`timeout`) elapsed since the child was spawned.
    Timeout,
    /// Stream-silence budget (`step_timeout`) elapsed since the last parent
    /// stream event. Stuck subagents (if any) are carried in the
    /// [`WatchdogTermination`] for diagnostic enrichment.
    StepTimeout,
}

/// Request sent by the watchdog ticker to the exec wait loop asking for
/// child-process termination.
///
/// Carries the reason, a human-readable message, and optional snapshots
/// of stuck subagents so the summary can be enriched with details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchdogTermination {
    pub(crate) reason: WatchdogTerminationReason,
    pub(crate) message: String,
    pub(crate) stuck_subagents: Vec<super::super::subagent_watchdog::ActiveSubagentSnapshot>,
}

/// Request sent when the wrapper has already received the successful final
/// response but the provider keeps its transport process alive.
///
/// The wait loop still terminates the child tree through the same signal /
/// Job Object path used for watchdog and content-guard termination, but the
/// resulting process outcome remains `Completed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletionTermination;

/// Convert a [`WatchdogTermination`] request from the watchdog ticker into
/// the lib-side [`EarlyTermination`] variant the wait loop carries until
/// the synthesized summary is built.
///
/// `Timeout` and `StepTimeout` are the only two reasons; `StepTimeout`
/// preserves the stuck-subagent snapshots as `outstanding` so the
/// rendered error block can name any subagents still in flight.
pub(crate) fn watchdog_request_to_early_termination(req: WatchdogTermination) -> EarlyTermination {
    match req.reason {
        WatchdogTerminationReason::Timeout => EarlyTermination::Timeout {
            message: req.message,
        },
        WatchdogTerminationReason::StepTimeout => EarlyTermination::StepTimeout {
            message: req.message,
            outstanding: req
                .stuck_subagents
                .iter()
                .map(|snap| snap.to_stuck_info())
                .collect(),
        },
    }
}

pub(crate) fn early_termination_process_outcome(
    early_termination: Option<&EarlyTermination>,
) -> claudine::harness::ProcessTermination {
    match early_termination {
        Some(EarlyTermination::Timeout { .. }) => claudine::harness::ProcessTermination::TimedOut,
        Some(EarlyTermination::StepTimeout { .. }) => {
            claudine::harness::ProcessTermination::TimedOut
        }
        Some(EarlyTermination::RateLimit { .. }) => {
            claudine::harness::ProcessTermination::Completed
        }
        // Content-guard trips (exit-expression / runaway-repetition /
        // runaway-volume) map to `Aborted` so `classify_failure` yields
        // `AgentFailure` — never `TimedOut` (which would route through the
        // lifecycle `failure` stack as a retryable timeout and reproduce the
        // runaway).
        //
        // The repeated-stream-error backstop is also a fail-fast abort: the
        // provider failed every retry, so a retryable timeout classification
        // would only reproduce the loop.
        //
        // The stalled-generation backstop fires for the same reason: retrying
        // a silently-dropped generation loop reproduces the stall, so it must
        // never route through `TimedOut` / `handle_timeout:`.
        Some(
            EarlyTermination::ExitExpression { .. }
            | EarlyTermination::RunawayRepetition { .. }
            | EarlyTermination::RunawayVolume { .. }
            | EarlyTermination::RepeatedStreamError { .. }
            | EarlyTermination::StalledGeneration { .. },
        ) => claudine::harness::ProcessTermination::Aborted,
        None => claudine::harness::ProcessTermination::Completed,
    }
}

/// Convert a lib-side detector [`Trip`] into the lib-side
/// [`EarlyTermination`] the wait loop carries on its termination channel.
///
/// This is the single bridge between the pure content detector
/// (`claudine::runaway`, Phase 2) and the termination-channel types — keeping
/// it here means the detector never imports [`EarlyTermination`]. Fields are
/// copied verbatim; the detector owns the structured detail, the termination
/// channel owns the routing.
///
/// The consumer is the live semantic sink's content detector
/// (`live_semantic_sink`), which sends the converted signal on the unified
/// early-termination channel the wait loop polls.
pub(crate) fn trip_to_early_termination(trip: Trip) -> EarlyTermination {
    match trip {
        Trip::ExitExpression { pattern, scope } => {
            EarlyTermination::ExitExpression { pattern, scope }
        }
        Trip::RunawayRepetition { cycle_len, repeats } => {
            EarlyTermination::RunawayRepetition { cycle_len, repeats }
        }
        Trip::RunawayVolume { lines, bytes } => EarlyTermination::RunawayVolume { lines, bytes },
    }
}
