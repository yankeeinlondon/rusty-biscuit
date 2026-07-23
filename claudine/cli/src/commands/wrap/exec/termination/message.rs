//! Human-facing early-termination message rendering.
//!
//! Every [`EarlyTermination`] variant carries a single canonical message
//! string. [`early_termination_message`] returns it for the inline stderr
//! `Warning` line, and [`apply_early_termination_to_summary`] writes the same
//! string into `summary.error_message` — the two surfaces share these
//! renderers so they never drift apart.
//!
//! [`apply_early_termination_to_summary`]: super::apply_early_termination_to_summary

use std::time::Duration;

use claudine::stream::logs::{EarlyTermination, StalledGenerationContext};

/// Render the user-facing message for an [`EarlyTermination`] trip.
///
/// Returns `None` only when the variant carries no inline message (none
/// today — every variant has one). Used both by
/// [`apply_early_termination_to_summary`] (to populate `summary.error_message`)
/// and by the spawn-side post-wait match (to emit a styled `Warning` line on
/// stderr) so the two surfaces never drift apart.
///
/// [`apply_early_termination_to_summary`]: super::apply_early_termination_to_summary
pub(crate) fn early_termination_message(termination: &EarlyTermination) -> Option<String> {
    match termination {
        EarlyTermination::RateLimit { message, .. } => Some(message.clone()),
        EarlyTermination::Timeout { message } => Some(message.clone()),
        EarlyTermination::StepTimeout { message, .. } => Some(message.clone()),
        EarlyTermination::ExitExpression { pattern, scope } => {
            Some(render_exit_expression_message(pattern, scope.as_deref()))
        }
        EarlyTermination::RunawayRepetition { cycle_len, repeats } => {
            Some(render_runaway_repetition_message(*cycle_len, *repeats))
        }
        EarlyTermination::RunawayVolume { lines, bytes } => {
            Some(render_runaway_volume_message(*lines, *bytes))
        }
        EarlyTermination::RepeatedStreamError { count } => {
            Some(render_repeated_stream_error_message(*count))
        }
        EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            context,
        } => Some(render_stalled_generation_message(
            *generation_count,
            *stall_duration,
            context,
        )),
    }
}

/// Render the error message for an [`EarlyTermination::ExitExpression`] trip,
/// naming the matched pattern and (when present) its scope.
pub(super) fn render_exit_expression_message(pattern: &str, scope: Option<&str>) -> String {
    match scope {
        Some(scope) if !scope.is_empty() => {
            format!("exit expression matched ({scope}): {pattern}")
        }
        _ => format!("exit expression matched: {pattern}"),
    }
}

/// Render the error message for an [`EarlyTermination::RunawayRepetition`]
/// trip, naming the detected cycle length and observed repeat count.
pub(super) fn render_runaway_repetition_message(cycle_len: usize, repeats: usize) -> String {
    format!(
        "runaway repetition detected (cycle length {cycle_len}, {repeats} repeats); \
         terminated to stop the loop"
    )
}

/// Render the error message for an [`EarlyTermination::RunawayVolume`] trip,
/// naming the line and byte counters at the moment of breach.
pub(super) fn render_runaway_volume_message(lines: u64, bytes: u64) -> String {
    format!(
        "output volume cap exceeded ({lines} lines, {bytes} bytes); \
         terminated to bound the runaway"
    )
}

/// Render the error message for an [`EarlyTermination::RepeatedStreamError`]
/// trip, naming the consecutive-failure count at the moment of breach.
pub(super) fn render_repeated_stream_error_message(count: u32) -> String {
    format!(
        "provider stream failed {count} times with no progress; \
         terminated to stop the retry loop"
    )
}

/// Render the error message for an [`EarlyTermination::StalledGeneration`]
/// trip, naming the generation-attempt count, the elapsed progress silence,
/// and any available safe OpenCode context (session id, step, agent, provider
/// id, model id, mode). Never includes prompt text or tool payloads.
pub(super) fn render_stalled_generation_message(
    generation_count: u32,
    stall_duration: Duration,
    context: &StalledGenerationContext,
) -> String {
    let seconds = stall_duration.as_secs();
    let mut message = format!(
        "provider attempted {generation_count} generations over {seconds}s with no progress; \
         terminated to stop the stalled-generation loop"
    );

    let mut details: Vec<String> = Vec::new();
    if let Some(session_id) = &context.session_id {
        details.push(format!("session={session_id}"));
    }
    if let Some(step) = context.step {
        details.push(format!("step={step}"));
    }
    if let Some(agent) = &context.agent {
        details.push(format!("agent={agent}"));
    }
    if let Some(provider_id) = &context.provider_id {
        details.push(format!("provider={provider_id}"));
    }
    if let Some(model_id) = &context.model_id {
        details.push(format!("model={model_id}"));
    }
    if let Some(mode) = &context.mode {
        details.push(format!("mode={mode}"));
    }
    if !details.is_empty() {
        message.push_str(&format!(" ({})", details.join(", ")));
    }

    message
}
