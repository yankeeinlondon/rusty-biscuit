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
/// The whole rendered message — every cascade branch (provider, guard,
/// timeout, stderr, and the fallbacks) plus any `(attempt N)` suffix — is
/// passed through one final hygiene stage: escapes stripped, collapsed to a
/// single line, and clamped so the returned string is always ≤
/// [`FAILURE_MESSAGE_MAX_CHARS`] characters. The suffix is budget-reserved,
/// so it survives truncation rather than being cut off. The `(attempt N)`
/// suffix is appended only when `attempt > 1` — at attempt 1 it would imply
/// a retry that may never happen.
pub fn failure_message(outcome: &AttemptOutcome, attempt: u32) -> String {
    let base = base_failure_message(outcome);
    let suffix = if attempt > 1 {
        format!(" (attempt {attempt})")
    } else {
        String::new()
    };
    sanitize_message(&base, &suffix)
}

/// Reduce arbitrary error text to a concise, notification-safe line.
///
/// The same hygiene [`failure_message`] applies to a provider attempt failure,
/// exposed for the other producer of notification-facing text: the
/// [`DiagnosticSnapshot`] message projected from a typed error's `Display`.
/// One implementation means a typed error and a provider failure cannot reach
/// TTS, messaging, or a desktop notification under different rules.
///
/// [`DiagnosticSnapshot`]: crate::diagnostics::DiagnosticSnapshot
pub fn concise_message(text: &str) -> String {
    sanitize_message(text, "")
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

/// Reduce provider-derived text to its headline line: strip escapes, then
/// pick the first (structured error message) or last (stderr — providers
/// print the fatal line last) non-empty line. Length clamping is deferred to
/// [`sanitize_message`], the single final pass shared by every cascade
/// branch, so provider text is never double-ellipsized.
fn headline(text: &str, pick: LinePick) -> Option<String> {
    let stripped = strip_ansi_codes(text);
    let mut lines = stripped.lines().map(str::trim).filter(|line| !line.is_empty());
    let line = match pick {
        LinePick::First => lines.next(),
        LinePick::Last => lines.next_back(),
    }?;
    Some(line.to_string())
}

/// Final hygiene pass over the complete failure message. Runs over *every*
/// cascade branch — including guard-context and timeout text that is
/// interpolated from user-configured `pattern`/`scope` — so no unsanitized
/// input can reach the lifecycle `err.msg` (TTS, messaging routes, desktop
/// notifications, stderr banner).
///
/// Strips escapes, collapses any surviving newline/control characters to a
/// single space-joined line, and clamps so that the returned `base + suffix`
/// is always ≤ [`FAILURE_MESSAGE_MAX_CHARS`]. The suffix budget is reserved
/// first (and the trailing `…` counts against the cap), so the `(attempt N)`
/// suffix always survives intact.
fn sanitize_message(base: &str, suffix: &str) -> String {
    let single_line = collapse_to_line(&strip_ansi_codes(base));
    let budget = FAILURE_MESSAGE_MAX_CHARS.saturating_sub(suffix.chars().count());
    format!("{}{suffix}", clamp_chars(&single_line, budget))
}

/// Fold a (possibly multi-line) string to a single line: split on any control
/// character (newlines, tabs, stray escape bytes), trim each segment, and
/// rejoin with a single space.
fn collapse_to_line(text: &str) -> String {
    text.split(|c: char| c.is_control())
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Clamp `line` to at most `budget` characters, reserving one character for
/// the trailing `…` when truncation occurs so the returned string never
/// exceeds `budget`.
fn clamp_chars(line: &str, budget: usize) -> String {
    if line.chars().count() <= budget {
        return line.to_string();
    }
    let clamped: String = line.chars().take(budget.saturating_sub(1)).collect();
    format!("{clamped}…")
}

#[cfg(test)]
mod tests;
