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

    /// One cascade branch, described by the outcome it needs and the headline
    /// it must render. `build` is a non-capturing closure so each row is a
    /// self-contained `AttemptOutcome` recipe (mirrors the field-update style
    /// of the focused tests below).
    struct Case {
        name: &'static str,
        build: fn() -> AttemptOutcome,
        expected: &'static str,
    }

    /// Drive every cascade branch through `failure_message` and assert both the
    /// bare attempt-1 headline and the attempt-2 `… (attempt 2)` form, so the
    /// uniform suffix policy is demonstrated on *every* source in one place.
    #[test]
    fn cascade_and_suffix_matrix() {
        let cases: &[Case] = &[
            // Source 1: provider error_message, first non-empty line.
            Case {
                name: "provider error_message (rate limit)",
                build: || {
                    let mut o = outcome(ProcessTermination::Completed);
                    o.error_message = Some("Too many requests".into());
                    o
                },
                expected: "Too many requests",
            },
            // Source 2: guard context.
            Case {
                name: "guard exit-expression with scope",
                build: || {
                    let mut o = outcome(ProcessTermination::Aborted);
                    o.guard_context = Some(GuardContext {
                        pattern: Some("STOPWIRE".into()),
                        scope: Some("opencode/kimi".into()),
                        ..GuardContext::default()
                    });
                    o
                },
                expected: "exit expression matched (opencode/kimi): STOPWIRE",
            },
            Case {
                name: "guard exit-expression without scope",
                build: || {
                    let mut o = outcome(ProcessTermination::Aborted);
                    o.guard_context = Some(GuardContext {
                        pattern: Some("STOPWIRE".into()),
                        ..GuardContext::default()
                    });
                    o
                },
                expected: "exit expression matched: STOPWIRE",
            },
            Case {
                name: "guard runaway repetition",
                build: || {
                    let mut o = outcome(ProcessTermination::Aborted);
                    o.guard_context = Some(GuardContext {
                        cycle_len: Some(4),
                        repeats: Some(35),
                        ..GuardContext::default()
                    });
                    o
                },
                expected: "runaway repetition detected (cycle length 4, 35 repeats)",
            },
            Case {
                name: "guard volume cap",
                build: || {
                    let mut o = outcome(ProcessTermination::Aborted);
                    o.guard_context = Some(GuardContext {
                        lines: Some(52_000),
                        bytes: Some(1024),
                        ..GuardContext::default()
                    });
                    o
                },
                expected: "output volume cap exceeded (52000 lines, 1024 bytes)",
            },
            Case {
                name: "guard stalled generation",
                build: || {
                    let mut o = outcome(ProcessTermination::Aborted);
                    o.guard_context = Some(GuardContext {
                        generation_count: Some(5),
                        stall_duration_ms: Some(600_000),
                        ..GuardContext::default()
                    });
                    o
                },
                expected: "stalled generation (5 attempts without progress, 10m silence)",
            },
            // Source 3: timeout phrasing.
            Case {
                name: "step timeout with configured duration",
                build: || {
                    let mut o = outcome(ProcessTermination::TimedOut);
                    o.error_kind = Some("step_timeout".into());
                    o.timeout_secs = Some(1_800);
                    o
                },
                expected: "step timeout (no output for 30m)",
            },
            Case {
                name: "wall-clock timeout with configured duration",
                build: || {
                    let mut o = outcome(ProcessTermination::TimedOut);
                    o.timeout_secs = Some(7_200);
                    o
                },
                expected: "provider timed out (wall-clock limit 2h)",
            },
            // Source 4: stderr last non-empty line wins over the fallback label.
            Case {
                name: "stderr last line over exit-code fallback",
                build: || {
                    let mut o = outcome(ProcessTermination::Completed);
                    o.exit_code = 99;
                    o.stderr_text = Some("warming up...\nError: invalid API key\n\n".into());
                    o
                },
                expected: "Error: invalid API key",
            },
            // Source 5: termination-label fallbacks.
            Case {
                name: "launch-failure label",
                build: || outcome(ProcessTermination::LaunchFailed),
                expected: "failed to launch provider process",
            },
            Case {
                name: "context-less abort label",
                build: || outcome(ProcessTermination::Aborted),
                expected: "aborted by content guard",
            },
            Case {
                name: "generic exit-code fallback",
                build: || {
                    let mut o = outcome(ProcessTermination::Completed);
                    o.exit_code = 99;
                    o
                },
                expected: "agent exited with error code 99",
            },
        ];

        for case in cases {
            let o = (case.build)();
            assert_eq!(
                failure_message(&o, 1),
                case.expected,
                "attempt-1 headline for `{}`",
                case.name
            );
            assert_eq!(
                failure_message(&o, 2),
                format!("{} (attempt 2)", case.expected),
                "attempt-2 suffix for `{}`",
                case.name
            );
        }
    }

    #[test]
    fn error_message_first_line_stripped_and_clamped() {
        let mut o = outcome(ProcessTermination::Completed);
        let long_tail = "x".repeat(300);
        o.error_message = Some(format!("\x1b[31mBilling error\x1b[0m\n{long_tail}"));
        assert_eq!(failure_message(&o, 1), "Billing error");

        o.error_message = Some(long_tail);
        let rendered = failure_message(&o, 1);
        // The `…` counts against the cap now, so the whole string fits within
        // FAILURE_MESSAGE_MAX_CHARS rather than spilling one char past it.
        assert_eq!(rendered.chars().count(), FAILURE_MESSAGE_MAX_CHARS);
        assert!(rendered.ends_with('…'));
    }

    #[test]
    fn guard_context_pattern_newline_collapsed_to_single_line() {
        let mut o = outcome(ProcessTermination::Aborted);
        o.guard_context = Some(GuardContext {
            pattern: Some("STOP\nHERE".into()),
            ..GuardContext::default()
        });
        let rendered = failure_message(&o, 1);
        assert!(!rendered.contains('\n'));
        assert_eq!(rendered, "exit expression matched: STOP HERE");
    }

    #[test]
    fn guard_context_pattern_escapes_stripped() {
        let mut o = outcome(ProcessTermination::Aborted);
        o.guard_context = Some(GuardContext {
            // SGR around the pattern, and an OSC hyperlink in the scope.
            pattern: Some("\x1b[31mDANGER\x1b[0m".into()),
            scope: Some("\x1b]8;;http://x\x07opencode\x1b]8;;\x07".into()),
            ..GuardContext::default()
        });
        let rendered = failure_message(&o, 1);
        assert!(!rendered.contains('\x1b'));
        assert!(rendered.contains("DANGER"));
    }

    #[test]
    fn guard_context_pattern_over_cap_is_clamped() {
        let mut o = outcome(ProcessTermination::Aborted);
        o.guard_context = Some(GuardContext {
            pattern: Some("x".repeat(300)),
            ..GuardContext::default()
        });
        let rendered = failure_message(&o, 1);
        assert!(rendered.chars().count() <= FAILURE_MESSAGE_MAX_CHARS);
        assert!(rendered.ends_with('…'));
    }

    #[test]
    fn oversized_message_with_suffix_respects_cap_and_keeps_suffix() {
        let mut o = outcome(ProcessTermination::Completed);
        o.error_message = Some("x".repeat(300));
        let rendered = failure_message(&o, 2);
        assert!(rendered.chars().count() <= FAILURE_MESSAGE_MAX_CHARS);
        assert!(rendered.ends_with("(attempt 2)"));
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
    fn provider_error_message_outranks_stderr() {
        // Source 1 beats source 4: a present provider message wins even when
        // stderr also carries a plausible fatal line.
        let mut o = outcome(ProcessTermination::Completed);
        o.exit_code = 99;
        o.error_message = Some("Too many requests".into());
        o.stderr_text = Some("Error: invalid API key".into());
        assert_eq!(failure_message(&o, 1), "Too many requests");
    }

    #[test]
    fn timeout_phrasing_without_configured_duration() {
        // The two no-`timeout_secs` timeout branches the matrix does not cover.
        let mut o = outcome(ProcessTermination::TimedOut);
        o.error_kind = Some("step_timeout".into());
        assert_eq!(failure_message(&o, 1), "step timeout (no stream output)");

        o.error_kind = None;
        assert_eq!(failure_message(&o, 1), "provider timed out");
    }

    #[test]
    fn stderr_last_line_ansi_stripped() {
        // Source 4 is a distinct line-pick branch (LAST meaningful line) and
        // must be escape-stripped like the provider branch.
        let mut o = outcome(ProcessTermination::Completed);
        o.exit_code = 99;
        o.stderr_text = Some("warming up...\n\x1b[31mError: invalid API key\x1b[0m".into());
        let rendered = failure_message(&o, 1);
        assert!(!rendered.contains('\x1b'));
        assert_eq!(rendered, "Error: invalid API key");
    }

    #[test]
    fn stderr_oversized_last_line_clamped_to_single_line() {
        // Single-line + 240-char final length on a non-provider branch.
        let mut o = outcome(ProcessTermination::Completed);
        o.exit_code = 99;
        o.stderr_text = Some(format!("warming up...\n{}", "x".repeat(300)));
        let rendered = failure_message(&o, 1);
        assert_eq!(rendered.lines().count(), 1);
        assert!(rendered.chars().count() <= FAILURE_MESSAGE_MAX_CHARS);
        assert!(rendered.ends_with('…'));
    }
}
