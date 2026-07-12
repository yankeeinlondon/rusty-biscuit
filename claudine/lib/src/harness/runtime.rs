//! Runtime helpers for the harness execution loop.

use crate::harness::model::{AttemptOutcome, FailureEvent, GuardContext, ProcessTermination};
use crate::stream::summary::StreamExecutionSummary;
use biscuit_terminal::discovery::eval::strip_ansi_codes;
use tracing::info_span;

/// Map `ProcessTermination` and `AttemptOutcome` to a `FailureEvent`.
///
/// ## Notes
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
        attempt = outcome.attempt,
    )
    .entered();
    match outcome.termination {
        ProcessTermination::TimedOut => Some(FailureEvent::Timeout),
        ProcessTermination::Interrupted => None, // User canceled, no recovery
        ProcessTermination::LaunchFailed => Some(FailureEvent::AgentFailure),
        ProcessTermination::Aborted => Some(FailureEvent::AgentFailure),
        ProcessTermination::Completed => {
            if outcome.exit_code != 0 {
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
        has_session_id = summary.session_id.is_some(),
    )
    .entered();
    AttemptOutcome {
        attempt,
        session_id: summary.session_id.clone(),
        final_response: summary.assistant_text.clone(),
        exit_code: summary.exit_code,
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
    }
}

/// Maximum rendered length of a failure message, in characters.
///
/// The message becomes the lifecycle `err.msg`, which feeds TTS, outbound
/// messaging routes, and the stderr banner — it is the headline, not the
/// archive. The full text stays available in `stderr_text` and the stream
/// summary.
const FAILURE_MESSAGE_MAX_CHARS: usize = 240;

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
/// Provider-derived text (sources 1 and 4) is clamped to a single
/// escape-stripped line of at most [`FAILURE_MESSAGE_MAX_CHARS`] characters.
/// The `(attempt N)` suffix is appended only when `attempt > 1` — at
/// attempt 1 it would imply a retry that may never happen.
pub fn failure_message(outcome: &AttemptOutcome, attempt: u32) -> String {
    let base = base_failure_message(outcome);
    if attempt > 1 {
        format!("{base} (attempt {attempt})")
    } else {
        base
    }
}

fn base_failure_message(outcome: &AttemptOutcome) -> String {
    if let Some(line) = outcome
        .error_message
        .as_deref()
        .and_then(|text| headline(text, LinePick::First))
    {
        return line;
    }
    if outcome.termination == ProcessTermination::Aborted
        && let Some(message) = outcome.guard_context.as_ref().and_then(guard_message)
    {
        return message;
    }
    if outcome.termination == ProcessTermination::TimedOut {
        return timeout_message(outcome);
    }
    if let Some(line) = outcome
        .stderr_text
        .as_deref()
        .and_then(|text| headline(text, LinePick::Last))
    {
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

enum LinePick {
    First,
    Last,
}

/// Reduce provider-derived text to a single sanitized headline: strip
/// escapes, pick the first (structured error message) or last (stderr —
/// providers print the fatal line last) non-empty line, and clamp to
/// [`FAILURE_MESSAGE_MAX_CHARS`].
fn headline(text: &str, pick: LinePick) -> Option<String> {
    let stripped = strip_ansi_codes(text);
    let mut lines = stripped.lines().map(str::trim).filter(|line| !line.is_empty());
    let line = match pick {
        LinePick::First => lines.next(),
        LinePick::Last => lines.next_back(),
    }?;
    Some(clamp_chars(line))
}

fn clamp_chars(line: &str) -> String {
    let mut chars = line.chars();
    let clamped: String = chars.by_ref().take(FAILURE_MESSAGE_MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{clamped}…")
    } else {
        clamped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(termination: ProcessTermination) -> AttemptOutcome {
        AttemptOutcome {
            attempt: 1,
            session_id: None,
            final_response: String::new(),
            exit_code: 1,
            termination,
            stderr_text: None,
            error_kind: None,
            guard_context: None,
            error_message: None,
            timeout_secs: None,
        }
    }

    #[test]
    fn provider_error_message_is_the_message() {
        let mut o = outcome(ProcessTermination::Completed);
        o.error_message = Some("Too many requests".into());
        assert_eq!(failure_message(&o, 1), "Too many requests");
    }

    #[test]
    fn error_message_first_line_stripped_and_clamped() {
        let mut o = outcome(ProcessTermination::Completed);
        let long_tail = "x".repeat(300);
        o.error_message = Some(format!("\x1b[31mBilling error\x1b[0m\n{long_tail}"));
        assert_eq!(failure_message(&o, 1), "Billing error");

        o.error_message = Some(long_tail);
        let rendered = failure_message(&o, 1);
        assert_eq!(rendered.chars().count(), FAILURE_MESSAGE_MAX_CHARS + 1);
        assert!(rendered.ends_with('…'));
    }

    #[test]
    fn synthesized_error_message_outranks_guard_context() {
        let mut o = outcome(ProcessTermination::Aborted);
        o.error_message =
            Some("exit expression matched: STOPWIRE; terminated to stop the loop".into());
        o.guard_context = Some(GuardContext {
            pattern: Some("STOPWIRE".into()),
            ..GuardContext::default()
        });
        assert_eq!(
            failure_message(&o, 1),
            "exit expression matched: STOPWIRE; terminated to stop the loop"
        );
    }

    #[test]
    fn guard_context_exit_expression() {
        let mut o = outcome(ProcessTermination::Aborted);
        o.guard_context = Some(GuardContext {
            pattern: Some("STOPWIRE".into()),
            scope: Some("opencode/kimi".into()),
            ..GuardContext::default()
        });
        assert_eq!(
            failure_message(&o, 1),
            "exit expression matched (opencode/kimi): STOPWIRE"
        );
    }

    #[test]
    fn guard_context_repetition_volume_stall() {
        let mut o = outcome(ProcessTermination::Aborted);
        o.guard_context = Some(GuardContext {
            cycle_len: Some(4),
            repeats: Some(35),
            ..GuardContext::default()
        });
        assert_eq!(
            failure_message(&o, 1),
            "runaway repetition detected (cycle length 4, 35 repeats)"
        );

        o.guard_context = Some(GuardContext {
            lines: Some(52_000),
            bytes: Some(1024),
            ..GuardContext::default()
        });
        assert_eq!(
            failure_message(&o, 1),
            "output volume cap exceeded (52000 lines, 1024 bytes)"
        );

        o.guard_context = Some(GuardContext {
            generation_count: Some(5),
            stall_duration_ms: Some(600_000),
            ..GuardContext::default()
        });
        assert_eq!(
            failure_message(&o, 1),
            "stalled generation (5 attempts without progress, 10m silence)"
        );
    }

    #[test]
    fn aborted_without_context_falls_back_to_label() {
        let o = outcome(ProcessTermination::Aborted);
        assert_eq!(failure_message(&o, 1), "aborted by content guard");
    }

    #[test]
    fn timeout_phrasing_by_error_kind_and_duration() {
        let mut o = outcome(ProcessTermination::TimedOut);
        o.error_kind = Some("step_timeout".into());
        o.timeout_secs = Some(1_800);
        assert_eq!(failure_message(&o, 1), "step timeout (no output for 30m)");

        o.timeout_secs = None;
        assert_eq!(failure_message(&o, 1), "step timeout (no stream output)");

        o.error_kind = None;
        o.timeout_secs = Some(7_200);
        assert_eq!(
            failure_message(&o, 1),
            "provider timed out (wall-clock limit 2h)"
        );

        o.timeout_secs = None;
        assert_eq!(failure_message(&o, 1), "provider timed out");
    }

    #[test]
    fn stderr_last_nonempty_line_wins_over_fallback() {
        let mut o = outcome(ProcessTermination::Completed);
        o.exit_code = 99;
        o.stderr_text = Some("warming up...\nError: invalid API key\n\n".into());
        assert_eq!(failure_message(&o, 1), "Error: invalid API key");
    }

    #[test]
    fn launch_failed_and_exit_code_fallbacks() {
        let o = outcome(ProcessTermination::LaunchFailed);
        assert_eq!(failure_message(&o, 1), "failed to launch provider process");

        let mut o = outcome(ProcessTermination::Completed);
        o.exit_code = 99;
        assert_eq!(failure_message(&o, 1), "agent exited with error code 99");
    }

    #[test]
    fn attempt_suffix_only_after_first_attempt() {
        let mut o = outcome(ProcessTermination::Completed);
        o.error_message = Some("Too many requests".into());
        assert_eq!(failure_message(&o, 1), "Too many requests");
        assert_eq!(failure_message(&o, 2), "Too many requests (attempt 2)");

        o.error_message = None;
        o.exit_code = 7;
        assert_eq!(
            failure_message(&o, 3),
            "agent exited with error code 7 (attempt 3)"
        );
    }
}
