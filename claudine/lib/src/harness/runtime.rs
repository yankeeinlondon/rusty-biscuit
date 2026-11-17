//! Runtime helpers for the harness execution loop.

use crate::harness::model::{
    AttemptOutcome, FailureEvent, GuardContext, OutputStatus, ProcessTermination,
};
use crate::stream::summary::StreamExecutionSummary;
use biscuit_terminal::discovery::eval::strip_ansi_codes;
use darkmatter::lifecycle::envelope::concise_message_with_suffix;
pub use darkmatter::lifecycle::envelope::concise_message;
use tracing::info_span;

/// Map `ProcessTermination` and `AttemptOutcome` to a `FailureEvent`.
///
/// ## Notes
///
/// A completed attempt is a success only when the provider both exited 0 **and**
/// its stream parser judged the session clean. Exit code 0 alone proves nothing:
/// a provider can force-stop its own sub-agents, finish the parent turn, and
/// exit 0, and Claude Code did exactly that in the 2026-08-31 incident. So a
/// `Completed` attempt is an [`FailureEvent::AgentFailure`] when *either*
/// `exit_code != 0` *or* `is_error` is set. This is the general
/// provider-agnostic semantic-error contract, not a Claude special case — any
/// parser that truthfully reports `is_error` now routes through failure, which
/// may surface pre-existing failures that used to look successful.
///
/// `ProcessTermination::Aborted` (a claudine content-guard trip —
/// exit-expression, runaway-repetition, or volume-cap) deliberately maps
/// to [`FailureEvent::AgentFailure`] rather than [`FailureEvent::Timeout`]:
/// the timeout-retry path would re-run the provider and reproduce the
/// runaway. It also deliberately does **not** map to `None` the way
/// [`ProcessTermination::Interrupted`] does, because a guard trip is a
/// genuine failure the operator's lifecycle recovery must observe —
/// suppressing it would silently swallow a runaway kill.
pub fn classify_failure(outcome: &AttemptOutcome) -> Option<FailureEvent> {
    let _span = info_span!(
        "harness_classify_failure",
        termination = %outcome.termination,
        exit_code = outcome.exit_code,
        is_error = outcome.is_error,
        attempt = outcome.attempt,
    )
    .entered();
    match outcome.termination {
        ProcessTermination::TimedOut => Some(FailureEvent::Timeout),
        ProcessTermination::Interrupted => None, // User canceled, no recovery
        ProcessTermination::LaunchFailed => Some(FailureEvent::AgentFailure),
        ProcessTermination::Aborted => Some(FailureEvent::AgentFailure),
        ProcessTermination::Completed => {
            if outcome.exit_code != 0 || outcome.is_error {
                Some(FailureEvent::AgentFailure)
            } else {
                None // Success, no failure event
            }
        }
    }
}

pub fn build_attempt_outcome(
    attempt: u32,
    summary: &StreamExecutionSummary,
    termination: ProcessTermination,
) -> AttemptOutcome {
    let _span = info_span!(
        "harness_attempt_outcome",
        attempt,
        termination = %termination,
        exit_code = summary.exit_code,
        is_error = summary.is_error,
        has_session_id = summary.session_id.is_some(),
    )
    .entered();
    AttemptOutcome {
        attempt,
        session_id: summary.session_id.clone(),
        final_response: summary.assistant_text.clone(),
        exit_code: summary.exit_code,
        // The parser's semantic verdict is authoritative even when the
        // process exited 0; dropping it here is what let a run with
        // force-stopped sub-agents fire the success lifecycle stack.
        is_error: summary.is_error,
        termination,
        stderr_text: summary.stderr_text.clone(),
        // Preserve the synthesized per-guard label so the failure-handler
        // payload can read it. The summary carries no structured guard
        // detail or configured timeout durations, so `guard_context` and
        // `timeout_secs` stay `None` here; the wrapper attempt path sets
        // them directly.
        error_kind: summary.error_kind.clone(),
        guard_context: None,
        error_message: summary.error_message.clone(),
        timeout_secs: None,
        output_status: OutputStatus::default(),
    }
}

/// Build the operator-facing message for a failed attempt.
///
/// Prefers the most contextual source available:
///
/// 1. `error_message` — the provider's own error text, or the guard/timeout
///    prose the wrapper synthesizes at trip time on the structured-stream
///    path (which overwrites any stale provider text, so on `Aborted` this
///    is never a leftover stream error);
/// 2. structured `guard_context` — capture-path guard trips carry no
///    stream summary, so no `error_message`;
/// 3. timeout phrasing from `error_kind` + `timeout_secs` — capture and
///    interactive paths synthesize no timeout message;
/// 4. the last useful stderr line;
/// 5. a termination label, falling back to the generic exit-code message so
///    the operator is never left without any signal.
///
/// The message becomes the lifecycle `err.msg`: the complete selected
/// explanation, escape-free and folded onto one line, never shortened. An
/// explicit `error_message` is kept whole (every line), while stderr
/// contributes only its last useful line; the stderr log itself is never
/// appended. An `error_message` that normalizes to nothing (only escapes or
/// whitespace) falls through to the next source. The `(attempt N)` suffix
/// is appended only when `attempt > 1` — at attempt 1 it would imply a retry
/// that may never happen.
pub fn failure_message(outcome: &AttemptOutcome, attempt: u32) -> String {
    let base = base_failure_message(outcome);
    let suffix = if attempt > 1 {
        format!(" (attempt {attempt})")
    } else {
        String::new()
    };
    concise_message_with_suffix(&base, &suffix)
}

fn base_failure_message(outcome: &AttemptOutcome) -> String {
    if let Some(message) = outcome
        .error_message
        .as_deref()
        .map(concise_message)
        .filter(|message| !message.is_empty())
    {
        return message;
    }
    if outcome.termination == ProcessTermination::Aborted
        && let Some(message) = outcome.guard_context.as_ref().and_then(guard_message)
    {
        return message;
    }
    if outcome.termination == ProcessTermination::TimedOut {
        return timeout_message(outcome);
    }
    if let Some(line) = outcome.stderr_text.as_deref().and_then(last_useful_line) {
        return line;
    }
    match outcome.termination {
        ProcessTermination::LaunchFailed => "failed to launch provider process".to_string(),
        ProcessTermination::Aborted => "aborted by content guard".to_string(),
        _ => format!("agent exited with error code {}", outcome.exit_code),
    }
}

/// Render a guard trip from its structured context, mirroring the message
/// vocabulary of the wrapper's `EarlyTermination` renderers. Only the
/// cluster relevant to the trip is populated, so the first matching
/// cluster wins; `None` when no recognizable cluster is present.
fn guard_message(context: &GuardContext) -> Option<String> {
    if let Some(pattern) = &context.pattern {
        return Some(match context.scope.as_deref() {
            Some(scope) if !scope.is_empty() => {
                format!("exit expression matched ({scope}): {pattern}")
            }
            _ => format!("exit expression matched: {pattern}"),
        });
    }
    if let (Some(cycle_len), Some(repeats)) = (context.cycle_len, context.repeats) {
        return Some(format!(
            "runaway repetition detected (cycle length {cycle_len}, {repeats} repeats)"
        ));
    }
    if let (Some(lines), Some(bytes)) = (context.lines, context.bytes) {
        return Some(format!(
            "output volume cap exceeded ({lines} lines, {bytes} bytes)"
        ));
    }
    if let (Some(generations), Some(stall_ms)) =
        (context.generation_count, context.stall_duration_ms)
    {
        return Some(format!(
            "stalled generation ({generations} attempts without progress, {} silence)",
            format_secs(stall_ms / 1000)
        ));
    }
    None
}

fn timeout_message(outcome: &AttemptOutcome) -> String {
    let configured = outcome.timeout_secs.map(format_secs);
    match (outcome.error_kind.as_deref(), configured) {
        (Some("step_timeout"), Some(duration)) => {
            format!("step timeout (no output for {duration})")
        }
        (Some("step_timeout"), None) => "step timeout (no stream output)".to_string(),
        (_, Some(duration)) => format!("provider timed out (wall-clock limit {duration})"),
        (_, None) => "provider timed out".to_string(),
    }
}

fn format_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        let minutes = secs / 60;
        let rem = secs % 60;
        if rem == 0 {
            format!("{minutes}m")
        } else {
            format!("{minutes}m{rem}s")
        }
    } else {
        let hours = secs / 3_600;
        let minutes = (secs % 3_600) / 60;
        if minutes == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h{minutes}m")
        }
    }
}

/// The last non-empty line of escape-stripped stderr; providers print the
/// fatal line last.
fn last_useful_line(text: &str) -> Option<String> {
    let stripped = strip_ansi_codes(text);
    stripped
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests;
