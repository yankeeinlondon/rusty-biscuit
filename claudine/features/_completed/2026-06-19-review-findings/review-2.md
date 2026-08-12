---
ready: false
implemented: true
agent: unknown/default
created: "2026-06-19T18:20:54"
---

# Review 2 — Comprehensive Review Remediation

## Findings

### Medium — guard trip details are not delivered to programmatic handlers

The new content-guard termination variants document that exit-expression
`pattern`/`scope` and runaway counters are carried into the failure payload
(`claudine/lib/src/stream/logs/opencode/reasoning.rs:107`). The implementation
does synthesize the right `StreamExecutionSummary.error_kind` values in
`apply_early_termination_to_summary`, and maps guard trips to
`ProcessTermination::Aborted` so they no longer route through timeout retry.
However, the detail is dropped before handlers run:

- `claudine/lib/src/harness/runtime.rs:27` builds every `AttemptOutcome` with
  `error_kind: None` and `guard_context: None`, ignoring the summary's
  `error_kind`.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:459` then calls
  `build_agent_failure_context(..., None, None)` even though `outcome` is
  already available.
- `claudine/lib/src/harness/handlers.rs:202` builds the programmatic handler
  JSON payload without `error_kind` or `guard_context`, and the handler
  environment also has no `CLAUDINE_ERROR_KIND` or equivalent.

That means a `handle:` script cannot distinguish `exit_expression`,
`runaway_repetition`, and ordinary non-zero agent failure except by parsing the
human message string. This breaks the documented payload contract and makes the
new `FailureContext.error_kind`, `FailureContext.guard_context`, and
`AttemptOutcome` fields effectively inert for their intended consumer.

Fix by preserving `summary.error_kind` in `build_attempt_outcome`, constructing
the appropriate `GuardContext` when applying an `EarlyTermination`, passing
`outcome.error_kind.clone()` / `outcome.guard_context.as_ref()` through
`build_agent_failure_context`, and adding those fields to the programmatic
handler JSON payload (plus env vars if that is part of the public handler
contract). Add a Level 1 regression test that executes a programmatic handler
against an `Aborted` outcome and asserts the payload contains the specific guard
kind and structured fields.

**Verification level:** Level 1 is appropriate. This is in-process
summary-to-handler payload plumbing with no terminal renderer or OS keyboard
dependency. Current Level 1 tests cover `Aborted -> AgentFailure`, but not that
the guard details reach the handler payload.

## Coverage Notes

The Review 1 gaps I checked are resolved: `paths[]` extraction now returns every
string path, both `statusCode` regexes have a non-digit/end boundary, and the
lifecycle ternary docs now match the implementation.

I did not find a requirement in this review-remediation scope that needs Level 2
or Level 3 terminal verification. The remaining gap is pure handler payload
behavior and should be covered at Level 1.

## Verification

Targeted Level 1 run passed:

```bash
cargo nextest run -p claudine -p claudine-cli \
  classify_aborted_returns_agent_failure \
  process_termination_aborted_serde_round_trips_snake_case \
  built_in_step_timeout_is_not_user_configured \
  claude_interactive_prompt_starting_with_dash_is_separated_with_end_of_options \
  --no-tests=pass
```

The run emitted one non-blocking Rust naming warning:
`vc_2_2_single_line_spam_trips_at_L1` should be snake_case.

## Production Readiness

Not ready. The retry-classification fix is in place, but the documented guard
payload still does not reach programmatic handlers, so the new guard termination
surface is incomplete.
