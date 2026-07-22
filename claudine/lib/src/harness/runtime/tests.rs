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
